import { spawnSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';

const project = path.resolve('.');
const cli = path.join(project, 'target/debug/highgrade.exe');
const checker = path.join(project, 'scripts/source-scenarios.mjs');
const fixtureRoot = mkdtempSync(path.join(os.tmpdir(), 'highgrade-scale-'));
const results = [];
const fail = (message) => { throw new Error(message); };
const write = (root, name, content) => {
  const target = path.join(root, name);
  mkdirSync(path.dirname(target), { recursive: true });
  writeFileSync(target, content);
};
const budget = (current) => ({
  unit: 'bytes', agreed: true, min: 1, baseline: current, current,
  ceiling: Math.max(current, 120000), history: [], review_trigger: 'after the fixture run',
});
const command = (file, args) => spawnSync(file, args, { encoding: 'utf8', maxBuffer: 8 * 1024 * 1024 });

try {
  for (const count of [100, 200]) {
    const root = path.join(fixtureRoot, String(count));
    const core = [
      ['README.md', 'purpose-navigation', '# Fixture\n[AGENTS](AGENTS.md) [ARCHITECTURE](docs/ARCHITECTURE.md) [ENGINEERING](docs/ENGINEERING.md) [DEVELOPMENT](docs/DEVELOPMENT.md)\n'],
      ['AGENTS.md', 'agent-rules', '# Agent\n[INSTRUCTIONS](.highgrade/project/INSTRUCTIONS.md)\n'],
      ['docs/ARCHITECTURE.md', 'current-architecture', '# Architecture\n'],
      ['docs/ENGINEERING.md', 'engineering-rules', '# Engineering\n'],
      ['docs/DEVELOPMENT.md', 'commands-procedures', '# Development\n'],
    ];
    for (const [file, , content] of core) write(root, file, content);
    write(root, '.highgrade/project/INSTRUCTIONS.md', '# Project\n[documents](documents.json)\n');
    write(root, 'tests/empty.rs', '');
    const extra = [];
    for (let index = 1; index <= count; index++) {
      const id = String(index).padStart(3, '0');
      const file = `specs/requirements/HG-SCALE-R${id}.json`;
      const filler = 'A'.repeat(500);
      write(root, file, `${JSON.stringify({ id: `HG-SCALE-R${id}`, title: `Feature ${id}`, statement: filler,
        scenarios: [{ id: `HG-SCALE-S${id}`, given: 'Fixture', when: `feature ${id} is inspected`,
          then: 'its route is selected alone', verification: 'manual fixture observation' }] })}\n`);
      extra.push({ path: file, role: `feature-${id}`, active: true, loading: 'task', scope: [`features/${id}`] });
    }
    const documents = core.map(([file, role], index) => ({
      id: `core-${index}`, path: file, role, loading: 'entry', scope: [], budget: budget(5000),
    }));
    write(root, '.highgrade/project/documents.json', `${JSON.stringify({
      schema_version: 1, documents, additional_sources: extra, route_budget: budget(96000),
    })}\n`);
    write(root, 'specs/catalog.json', `${JSON.stringify({ schema_version: 3, next_number: 1, retired_ids: [], origins: {} })}\n`);
    const check = command(process.execPath, [checker, 'check', root]);
    if (check.status !== 0) fail(`source check ${count}: ${check.stderr}`);
    const checkResult = JSON.parse(check.stdout);
    if (checkResult.requirements !== count || checkResult.scenarios !== count) fail(`source count ${count}`);
    const selected = command(cli, ['inspect', '--root', root, '--scope', 'features/001']);
    const full = command(cli, ['inspect', '--root', root]);
    if (selected.status === null || full.status === null || !selected.stdout || !full.stdout) fail(`inspect ${count}: selected=${selected.status} ${selected.stderr}; full=${full.status} ${full.stderr}`);
    const selectedReport = JSON.parse(selected.stdout);
    const fullReport = JSON.parse(full.stdout);
    if (selectedReport.findings.some((item) => item.status === 'failed')) fail(`selected fixture invalid: ${JSON.stringify(selectedReport.findings.filter((item) => item.status === 'failed').slice(0, 5))}`);
    if (fullReport.findings.some((item) => item.status === 'failed')) fail(`full fixture invalid: ${JSON.stringify(fullReport.findings.filter((item) => item.status === 'failed').slice(0, 5))}`);
    const route = (report) => report.measurements.find((item) => Array.isArray(item.route));
    const selectedRoute = route(selectedReport);
    const fullRoute = route(fullReport);
    if (!selectedRoute || !fullRoute) fail(`missing route ${count}`);
    if (selectedRoute.route.filter((item) => item.startsWith('specs/requirements/')).length !== 1) fail(`scope over-selected ${count}`);
    if (fullRoute.route.filter((item) => item.startsWith('specs/requirements/')).length !== count) fail(`full route omitted specs ${count}`);
    if (selectedRoute.route_bytes >= 96000) fail(`selected route exceeded ceiling ${count}`);
    if (count === 200 && !fullReport.findings.some((item) => item.code === 'BudgetExceeded' && item.location === 'context-route')) fail('200-spec full route did not warn');
    write(root, 'tests/empty.rs', '// highgrade: HG-SCALE-S003\n#[test]\nfn apparent_test() {}\n');
    const orphanCheck = command(process.execPath, [checker, 'check', root]);
    if (orphanCheck.status === 0 || !orphanCheck.stderr.includes('test marker has no configured automatic check')) fail(`orphan marker not rejected ${count}`);
    write(root, 'specs/changes/HG-SCALE-CHECK/spec.json', JSON.stringify({ archived: true, operations: [], checks: [{ runner: 'rust-contracts', scenario_ids: ['HG-SCALE-S003'], file: 'tests/empty.rs', selector: 'different_test' }] }));
    const wrongSelector = command(process.execPath, [checker, 'check', root]);
    if (wrongSelector.status === 0 || !wrongSelector.stderr.includes('test marker does not match its configured file and selector')) fail(`wrong selector not rejected ${count}`);
    write(root, 'specs/changes/HG-SCALE-CHECK/spec.json', JSON.stringify({ archived: true, operations: [], checks: [{ runner: 'rust-contracts', scenario_ids: ['HG-SCALE-S003'], file: 'tests/empty.rs', selector: 'apparent_test' }] }));
    write(root, 'tests/empty.rs', '// highgrade: HG-SCALE-S003\n#[test]\nfn apparent_test() {}\n// highgrade: HG-SCALE-S003\n#[test]\nfn unrelated_test() {}\n');
    const extraMarker = command(process.execPath, [checker, 'check', root]);
    if (extraMarker.status === 0 || !extraMarker.stderr.includes('test marker does not match its configured file and selector')) fail(`extra wrong marker not rejected ${count}`);
    write(root, 'tests/empty.rs', '#[test]\nfn apparent_test() {}\n');
    const missingMarker = command(process.execPath, [checker, 'check', root]);
    if (missingMarker.status === 0 || !missingMarker.stderr.includes('configured automatic check has no test marker')) fail(`missing marker not rejected ${count}`);
    rmSync(path.join(root, 'specs/changes/HG-SCALE-CHECK/spec.json'));
    write(root, 'tests/empty.rs', '');
    const duplicate = 'specs/requirements/HG-SCALE-R002.json';
    const source = readFileSync(path.join(root, duplicate), 'utf8');
    write(root, duplicate, source.replace('HG-SCALE-S002', 'HG-SCALE-S001'));
    const duplicateCheck = command(process.execPath, [checker, 'check', root]);
    if (duplicateCheck.status === 0 || !duplicateCheck.stderr.includes('duplicate ID')) fail(`duplicate ID not rejected ${count}`);
    results.push({ specs: count, selected_bytes: selectedRoute.route_bytes, full_bytes: fullRoute.route_bytes, duplicate_rejected: true, orphan_marker_rejected: true, wrong_selector_rejected: true, extra_marker_rejected: true, missing_marker_rejected: true });
  }
  console.log(JSON.stringify({ status: 'passed', results }, null, 2));
} catch (cause) {
  console.error(JSON.stringify({ status: 'failed', message: String(cause.message ?? cause) }));
  process.exitCode = 1;
} finally {
  const resolved = path.resolve(fixtureRoot);
  if (resolved.startsWith(path.resolve(os.tmpdir()) + path.sep) && path.basename(resolved).startsWith('highgrade-scale-')) {
    rmSync(resolved, { recursive: true, force: true });
  }
}
