import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { existsSync, readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import path from 'node:path';

const root = path.resolve(process.argv[3] ?? '.');
const mode = process.argv[2];
const abs = (file) => path.join(root, file);
const read = (file) => JSON.parse(readFileSync(abs(file), 'utf8'));
const sha = (file) => createHash('sha256').update(readFileSync(abs(file))).digest('hex');
const fail = (message) => { throw new Error(message); };
const validId = (id) => typeof id === 'string' && id.length >= 6 && id.length <= 80 && /^[A-Z]{2,32}(?:-[A-Z0-9]+)+$/.test(id);
const requiredText = (value, location) => { if (typeof value !== 'string' || !value.trim()) fail(`${location}: empty required text`); };
const files = (dir) => existsSync(abs(dir)) ? readdirSync(abs(dir), { withFileTypes: true }).flatMap((entry) =>
  entry.isDirectory() ? files(`${dir}/${entry.name}`) : entry.isFile() ? [`${dir}/${entry.name}`] : []) : [];

function sources() {
  const requirements = new Map();
  for (const file of files('specs/requirements').filter((item) => item.endsWith('.json'))) {
    const requirement = read(file);
    if (requirements.has(requirement.id)) fail(`${file}: duplicate ID ${requirement.id}`);
    requirements.set(requirement.id, requirement);
  }
  const touched = new Set();
  for (const file of files('specs/changes').filter((item) => item.endsWith('/spec.json'))) {
    const change = read(file);
    if (change.archived) continue;
    for (const operation of change.operations) {
      const id = operation.requirement?.id ?? operation.id;
      if (!validId(id) || touched.has(id)) fail(`${file}: duplicate ID ${id}`);
      touched.add(id);
      if (operation.action === 'remove') requirements.delete(id);
      else requirements.set(id, operation.requirement);
    }
  }
  const scenarios = new Map();
  for (const requirement of requirements.values()) {
    if (!validId(requirement.id) || !Array.isArray(requirement.scenarios) || !requirement.scenarios.length) fail(`invalid requirement ${requirement.id}`);
    for (const field of ['title', 'statement']) requiredText(requirement[field], `${requirement.id}.${field}`);
    for (const scenario of requirement.scenarios) {
      if (!validId(scenario.id) || requirements.has(scenario.id) || scenarios.has(scenario.id)) fail(`duplicate ID ${scenario.id}`);
      for (const field of ['given', 'when', 'then', 'verification']) requiredText(scenario[field], `${scenario.id}.${field}`);
      scenarios.set(scenario.id, scenario);
    }
  }
  const catalog = read('specs/catalog.json');
  if (Object.keys(catalog.origins ?? {}).length) fail('active catalog still depends on historical origins');
  const bound = new Map();
  const manual = new Set();
  for (const file of files('specs/changes').filter((item) => item.endsWith('/spec.json'))) {
    const change = read(file);
    for (const check of change.checks ?? []) for (const id of check.scenario_ids) {
      if (check.runner) bound.set(id, [...(bound.get(id) ?? []), check]);
      else manual.add(id);
    }
  }
  const testSources = files('tests').filter((item) => /^tests\/[^/]+\.rs$/.test(item)).sort();
  const marked = new Set();
  for (const file of testSources) {
    const lines = readFileSync(abs(file), 'utf8').split(/\r?\n/);
    for (let index = 0; index < lines.length; index++) {
      const marker = lines[index].match(/^\s*\/\/ highgrade: (.+)$/);
      if (!marker) continue;
      const selector = lines.slice(index + 1, index + 8).map((line) => line.match(/^\s*fn (\w+)/)?.[1]).find(Boolean);
      if (!selector) fail(`${file}: marker without following test function`);
      for (const id of marker[1].split(',').map((item) => item.trim())) {
        if (!validId(id) || !scenarios.has(id)) fail(`${file}: unknown marker ${id}`);
        if (!bound.has(id)) fail(`${id}: test marker has no configured automatic check`);
        if (!bound.get(id).some((check) => check.file === file && check.selector === selector)) {
          fail(`${id}: test marker does not match its configured file and selector`);
        }
        marked.add(id);
      }
    }
  }
  for (const id of bound.keys()) {
    if (scenarios.has(id) && !marked.has(id)) fail(`${id}: configured automatic check has no test marker`);
  }
  const noDeclaredCheck = [...scenarios.keys()].filter((id) => !bound.has(id) && !manual.has(id)).sort();
  const manualOnly = [...manual].filter((id) => !bound.has(id)).sort();
  return { requirements, scenarios, automatic: [...marked].sort(), manual: manualOnly, noDeclaredCheck, testSources };
}

function nativeHash() {
  const cli = path.resolve('target/debug/highgrade.exe');
  const result = spawnSync(cli, ['spec-list', '--root', root], { encoding: 'utf8' });
  if (result.status !== 0) fail(`spec-list failed: ${result.stdout || result.stderr}`);
  const digest = JSON.parse(result.stdout).measurements.find((item) => typeof item.trace_sha256 === 'string')?.trace_sha256;
  if (!digest) fail('spec-list has no trace_sha256');
  return digest;
}

function prepare(source) {
  const inventory = 'target/nextest/highgrade/list.json', report = 'target/nextest/highgrade/junit.xml', record = 'target/nextest/highgrade/run.json';
  const xml = readFileSync(abs(report), 'utf8');
  const number = (tag, name) => {
    const value = tag.match(new RegExp(`\\b${name}="(\\d+)"`))?.[1];
    if (value === undefined) fail(`nextest JUnit lacks ${name}`);
    return Number(value);
  };
  const rootSuite = xml.match(/<testsuites\b[^>]*>/)?.[0];
  const suites = [...xml.matchAll(/<testsuite\b[^>]*>/g)].map(([tag]) => tag);
  if (!rootSuite || !suites.length) fail('nextest JUnit has no test suites');
  const totals = Object.fromEntries(['tests', 'skipped', 'failures', 'errors'].map((field) => [field, number(rootSuite, field)]));
  for (const field of Object.keys(totals)) if (suites.reduce((sum, suite) => sum + number(suite, field), 0) !== totals[field]) fail(`nextest JUnit ${field} total disagrees with suites`);
  if (!totals.tests || totals.skipped || totals.failures || totals.errors || /<(?:failure|error|skipped)\b/.test(xml)) fail(`nextest JUnit is not a complete pass: ${JSON.stringify(totals)}`);
  const listed = read(inventory)['rust-suites'];
  if (!listed || typeof listed !== 'object') fail('nextest inventory has no rust suites');
  const listedTests = Object.values(listed).reduce((sum, suite) => sum + Object.keys(suite.testcases ?? {}).length, 0);
  if (listedTests !== totals.tests) fail(`nextest JUnit has ${totals.tests}/${listedTests} listed tests`);
  const pinned = ['specs/catalog.json', ...files('specs/requirements').filter((item) => item.endsWith('.json')),
    ...files('specs/changes').filter((item) => item.endsWith('/spec.json')),
    ...source.testSources, ...files('tests/support').filter((item) => item.endsWith('.rs')),
    inventory, ...files('src').filter((item) => item.endsWith('.rs')),
    ...files('tests/fixtures/native-v1'), 'Cargo.toml', 'Cargo.lock', '.config/nextest.toml', '.gitattributes'];
  const reportTime = statSync(abs(report)).mtimeMs;
  for (const file of pinned.filter((item) => item !== inventory)) if (statSync(abs(file)).mtimeMs > reportTime) fail(`${file}: source changed after the native test report; rerun nextest`);
  if (statSync(abs(inventory)).mtimeMs > reportTime) fail('nextest inventory is newer than the test report; rerun nextest');
  const sourceHashes = Object.fromEntries(pinned.map((file) => [file, file === 'specs/catalog.json' ? nativeHash() : sha(file)]));
  writeFileSync(abs(record), `${JSON.stringify({ schema_version: 1, tool: 'rust-nextest', command: 'cargo nextest run --locked --profile highgrade',
    scope: 'active native High Grade specifications and Rust integration tests', captured_at: new Date().toISOString(), capture_root: root,
    exit_code: 0, report, report_sha256: sha(report), spec_files: ['specs/catalog.json'], test_sources: source.testSources,
    source_hashes: sourceHashes, inventory }, null, 2)}\n`);
  return { record, pinned_files: pinned.length, native_tests_passed: totals.tests };
}

function verify(source) {
  const trace = read('target/nextest/highgrade/trace.json');
  if (trace.operation !== 'trace') fail('native trace did not produce a trace report');
  const rows = trace.measurements.filter((item) => typeof item.scenario_id === 'string');
  if (rows.length !== source.scenarios.size) fail(`trace covered ${rows.length}/${source.scenarios.size} scenarios`);
  const byId = new Map(rows.map((item) => [item.scenario_id, item]));
  for (const id of source.automatic) if (byId.get(id)?.status !== 'passed') fail(`${id}: actual test status is ${byId.get(id)?.status ?? 'absent'}`);
  for (const finding of trace.findings) fail(`${finding.code}: ${finding.message}`);
  const counts = trace.measurements.find((item) => typeof item.test_cases_reported === 'number');
  if (!counts || counts.scenarios_total !== source.scenarios.size || counts.automatic_passed !== source.automatic.length ||
      counts.outside_automatic_trace !== rows.filter((item) => item.status === 'outside_automatic_trace').length ||
      counts.test_cases_reported < counts.linked_test_cases ||
      counts.test_cases_reported !== counts.test_cases_executed + counts.test_cases_skipped + counts.test_cases_unknown ||
      counts.test_cases_missing !== 0 ||
      counts.unlinked_test_cases !== counts.test_cases_reported - counts.linked_test_cases ||
      counts.outside_automatic_trace !== source.manual.length + source.noDeclaredCheck.length) fail('trace scope counters disagree with source and outcomes');
  return { ...counts };
}

try {
  if (!['check', 'prepare', 'verify'].includes(mode)) fail('usage: node scripts/source-scenarios.mjs check|prepare|verify [root]');
  const source = sources();
  const summary = { requirements: source.requirements.size, scenarios: source.scenarios.size,
    automatic: source.automatic.length, manual_check_scenarios: source.manual.length,
    no_declared_check_scenarios: source.noDeclaredCheck.length, no_declared_check_ids: source.noDeclaredCheck };
  if (mode === 'prepare') Object.assign(summary, prepare(source));
  if (mode === 'verify') Object.assign(summary, verify(source));
  console.log(JSON.stringify({ status: 'passed', ...summary }, null, 2));
} catch (cause) {
  console.error(JSON.stringify({ status: 'failed', message: String(cause.message ?? cause) }));
  process.exitCode = 1;
}
