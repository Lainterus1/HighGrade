import { createHash } from 'node:crypto';
import { existsSync, readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import path from 'node:path';

const root = path.resolve(process.argv[3] ?? '.');
const mode = process.argv[2];
const relative = (file) => path.relative(root, file).replaceAll('\\', '/');
const absolute = (file) => path.join(root, file);
const sha256 = (file) => createHash('sha256').update(readFileSync(absolute(file))).digest('hex');
const error = (message) => { throw new Error(message); };
const validId = (id) => id.length >= 6 && id.length <= 80
  && /^[A-Z]{2,32}(?:-[A-Z0-9]+)+$/.test(id);

function activeSources() {
  const specRoot = absolute('openspec/specs');
  const specFiles = readdirSync(specRoot, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => path.join(specRoot, entry.name, 'spec.md'))
    .map(relative);
  const changeRoot = absolute('openspec/changes');
  if (existsSync(changeRoot)) {
    for (const change of readdirSync(changeRoot, { withFileTypes: true })) {
      if (!change.isDirectory() || change.name === 'archive') continue;
      const deltas = path.join(changeRoot, change.name, 'specs');
      if (!existsSync(deltas)) continue;
      for (const capability of readdirSync(deltas, { withFileTypes: true })) {
        if (!capability.isDirectory()) continue;
        const file = path.join(deltas, capability.name, 'spec.md');
        if (existsSync(file)) specFiles.push(relative(file));
      }
    }
  }
  specFiles.sort();
  const replaced = new Map();
  for (const file of specFiles.filter((item) => item.startsWith('openspec/changes/'))) {
    const capability = file.match(/^openspec\/changes\/[^/]+\/specs\/([^/]+)\/spec\.md$/)?.[1];
    if (!capability) continue;
    const base = `openspec/specs/${capability}/spec.md`;
    if (!specFiles.includes(base)) continue;
    let section = '';
    for (const line of readFileSync(absolute(file), 'utf8').split(/\r?\n/)) {
      if (line.startsWith('## ')) section = line;
      if (section !== '## MODIFIED Requirements' || !line.startsWith('### Requirement:')) continue;
      const id = line.match(/^### Requirement: \[([^\]]+)\]/)?.[1];
      if (!id) error(`${file}: invalid modified requirement heading: ${line}`);
      if (!replaced.has(base)) replaced.set(base, new Set());
      if (replaced.get(base).has(id)) error(`${file}: duplicate modified requirement ${id}`);
      replaced.get(base).add(id);
    }
  }
  const testRoot = absolute('tests');
  const testSources = readdirSync(testRoot, { withFileTypes: true })
    .filter((entry) => entry.isFile() && entry.name.endsWith('.rs'))
    .map((entry) => relative(path.join(testRoot, entry.name))).sort();
  const ids = new Map();
  const scenarios = new Map();
  let requirements = 0;
  for (const file of specFiles) {
    const lines = readFileSync(absolute(file), 'utf8').split(/\r?\n/);
    let current = null;
    let skip = false;
    const finish = () => {
      if (current && current.methods.length !== 1) {
        error(`${file}: ${current.id} needs exactly one verification method`);
      }
      if (current) scenarios.set(current.id, { file, method: current.methods[0] });
    };
    for (const line of lines) {
      if (line.startsWith('### Requirement:')) {
        finish(); current = null;
        const match = line.match(/^### Requirement: \[([^\]]+)\] \S/);
        if (!match || !validId(match[1])) error(`${file}: invalid requirement heading: ${line}`);
        skip = replaced.get(file)?.has(match[1]) ?? false;
        if (skip) continue;
        if (ids.has(match[1])) error(`${file}: duplicate ID ${match[1]} in ${ids.get(match[1])}`);
        ids.set(match[1], file);
        requirements++;
      } else if (line.startsWith('#### Scenario:')) {
        if (skip) continue;
        finish();
        const match = line.match(/^#### Scenario: \[([^\]]+)\] \S/);
        if (!match || !validId(match[1])) error(`${file}: invalid scenario heading: ${line}`);
        if (ids.has(match[1])) error(`${file}: duplicate ID ${match[1]} in ${ids.get(match[1])}`);
        ids.set(match[1], file);
        current = { id: match[1], methods: [] };
      } else if (!skip && current && line.startsWith('**Проверка:**')) {
        const match = line.match(/^\*\*Проверка:\*\* `(rust-nextest|ручная)`$/);
        if (!match) error(`${file}: invalid verification method for ${current.id}: ${line}`);
        current.methods.push(match[1]);
      }
    }
    finish();
  }

  const marked = new Set();
  for (const file of testSources) {
    for (const line of readFileSync(absolute(file), 'utf8').split(/\r?\n/)) {
      const match = line.match(/^\s*\/\/ highgrade: (.+)$/);
      if (!match) continue;
      for (const id of match[1].split(',').map((part) => part.trim())) {
        if (!validId(id) || !scenarios.has(id)) error(`${file}: unknown marker ${id}`);
        if (scenarios.get(id).method !== 'rust-nextest') error(`${file}: manual scenario marked as automatic: ${id}`);
        marked.add(id);
      }
    }
  }
  const automatic = [...scenarios].filter(([, item]) => item.method === 'rust-nextest').map(([id]) => id);
  const manual = [...scenarios].filter(([, item]) => item.method === 'ручная').map(([id]) => id);
  for (const id of automatic) if (!marked.has(id)) error(`${id}: automated scenario has no test marker`);
  return { specFiles, testSources, requirements, scenarios, automatic, manual };
}

function prepare(sources) {
  const inventory = 'target/nextest/highgrade/list.json';
  const report = 'target/nextest/highgrade/junit.xml';
  const record = 'target/nextest/highgrade/run.json';
  const xml = readFileSync(absolute(report), 'utf8');
  const numeric = (tag, name) => {
    const value = tag.match(new RegExp(`\\b${name}="(\\d+)"`))?.[1];
    if (value === undefined) error(`nextest JUnit lacks ${name}`);
    return Number(value);
  };
  const rootSuite = xml.match(/<testsuites\b[^>]*>/)?.[0];
  const suites = [...xml.matchAll(/<testsuite\b[^>]*>/g)].map(([tag]) => tag);
  if (!rootSuite || suites.length === 0) error('nextest JUnit has no test suites');
  const fields = ['tests', 'skipped', 'failures', 'errors'];
  const totals = Object.fromEntries(fields.map((field) => [field, numeric(rootSuite, field)]));
  for (const field of fields) {
    if (suites.reduce((sum, suite) => sum + numeric(suite, field), 0) !== totals[field]) {
      error(`nextest JUnit ${field} total disagrees with suites`);
    }
  }
  if (totals.tests === 0 || totals.skipped !== 0 || totals.failures !== 0 || totals.errors !== 0
      || /<(?:failure|error|skipped)\b/.test(xml)) {
    error(`nextest JUnit is not a complete pass: ${JSON.stringify(totals)}`);
  }
  const listed = JSON.parse(readFileSync(absolute(inventory), 'utf8'))['rust-suites'];
  if (!listed || typeof listed !== 'object') error('nextest inventory has no rust suites');
  const listedTests = Object.values(listed).reduce((sum, suite) => sum + Object.keys(suite.testcases ?? {}).length, 0);
  if (listedTests !== totals.tests) error(`nextest JUnit has ${totals.tests}/${listedTests} listed tests`);
  const pinned = [
    ...sources.specFiles, ...sources.testSources, inventory,
    ...readdirSync(absolute('src')).filter((name) => name.endsWith('.rs')).map((name) => `src/${name}`),
    'Cargo.toml', 'Cargo.lock', '.config/nextest.toml', 'openspec/config.yaml',
  ];
  const reportTime = statSync(absolute(report)).mtimeMs;
  for (const file of pinned.filter((item) => item !== inventory)) {
    if (statSync(absolute(file)).mtimeMs > reportTime) {
      error(`${file}: source changed after the native test report; rerun nextest`);
    }
  }
  if (statSync(absolute(inventory)).mtimeMs > reportTime) {
    error('nextest inventory is newer than the test report; rerun nextest');
  }
  const sourceHashes = Object.fromEntries(pinned.map((file) => [file, sha256(file)]));
  const run = {
    schema_version: 1,
    tool: 'rust-nextest',
    command: 'cargo nextest run --locked --profile highgrade',
    scope: 'all active High Grade specifications and Rust integration tests',
    captured_at: new Date().toISOString(),
    capture_root: root,
    exit_code: 0,
    report,
    report_sha256: sha256(report),
    spec_files: sources.specFiles,
    test_sources: sources.testSources,
    source_hashes: sourceHashes,
    inventory,
  };
  writeFileSync(absolute(record), `${JSON.stringify(run, null, 2)}\n`);
  return { record, pinned_files: pinned.length, native_tests_passed: totals.tests };
}

function verify(sources) {
  const trace = JSON.parse(readFileSync(absolute('target/nextest/highgrade/trace.json'), 'utf8'));
  if (trace.operation !== 'trace') error('native trace did not produce a trace report');
  const rows = trace.measurements.filter((item) => typeof item.scenario_id === 'string');
  if (rows.length !== sources.scenarios.size) error(`trace covered ${rows.length}/${sources.scenarios.size} scenarios`);
  const byId = new Map(rows.map((item) => [item.scenario_id, item]));
  for (const id of sources.automatic) {
    if (byId.get(id)?.status !== 'passed') error(`${id}: actual test status is ${byId.get(id)?.status ?? 'absent'}`);
  }
  for (const id of sources.manual) {
    const row = byId.get(id);
    if (row?.status !== 'unlinked' || row.tests?.length !== 0) {
      error(`${id}: manual scenario was represented as an automated result`);
    }
  }
  for (const finding of trace.findings) {
    if (finding.code !== 'ScenarioUnconfirmed') error(`${finding.code}: ${finding.message}`);
    const id = String(finding.message).split(':')[0];
    if (!sources.manual.includes(id)) error(`${id}: unexpected scenario finding`);
  }
  return { status: 'passed', automatic_passed: sources.automatic.length, manual_not_automated: sources.manual.length };
}

try {
  if (!['check', 'prepare', 'verify'].includes(mode)) error('usage: node scripts/source-scenarios.mjs check|prepare|verify [root]');
  const sources = activeSources();
  const summary = {
    requirements: sources.requirements,
    scenarios: sources.scenarios.size,
    automatic: sources.automatic.length,
    manual: sources.manual.length,
  };
  if (mode === 'prepare') Object.assign(summary, prepare(sources));
  if (mode === 'verify') Object.assign(summary, verify(sources));
  console.log(JSON.stringify({ status: 'passed', ...summary }, null, 2));
} catch (cause) {
  console.error(JSON.stringify({ status: 'failed', message: String(cause.message ?? cause) }));
  process.exitCode = 1;
}
