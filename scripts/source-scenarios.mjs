import { createHash, randomUUID } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { existsSync, readdirSync, readFileSync, mkdirSync, renameSync, rmSync, writeFileSync } from 'node:fs';
import path from 'node:path';

const root = path.resolve(process.argv[3] ?? '.');
const mode = process.argv[2];
const abs = (file) => path.join(root, file);
const read = (file) => JSON.parse(readFileSync(abs(file), 'utf8'));
const readHashed = (file) => {
  const bytes = readFileSync(abs(file));
  return { value: JSON.parse(bytes.toString('utf8')), sha256: createHash('sha256').update(bytes).digest('hex') };
};
const sha = (file) => createHash('sha256').update(readFileSync(abs(file))).digest('hex');
const digest = (value) => createHash('sha256').update(JSON.stringify(canonical(value))).digest('hex');
const canonical = (value) => Array.isArray(value) ? value.map(canonical) : value && typeof value === 'object'
  ? Object.fromEntries(Object.keys(value).sort().map((key) => [key, canonical(value[key])])) : value;
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
  const checks = [];
  for (const file of files('specs/changes').filter((item) => item.endsWith('/spec.json'))) {
    const change = read(file);
    for (const check of change.checks ?? []) {
      checks.push(check);
      for (const id of check.scenario_ids) {
        if (check.runner) bound.set(id, [...(bound.get(id) ?? []), check]);
        else manual.add(id);
      }
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
        if (bound.has(id) && !bound.get(id).some((check) => check.file === file && check.selector === selector)) {
          fail(`${id}: test marker does not match its configured file and selector`);
        }
        marked.add(id);
      }
    }
  }
  for (const id of bound.keys()) {
    if (scenarios.has(id) && !marked.has(id)) fail(`${id}: configured automatic check has no test marker`);
  }
  const noDeclaredCheck = [...scenarios.keys()].filter((id) => !bound.has(id) && !manual.has(id) && !marked.has(id)).sort();
  const manualOnly = [...manual].filter((id) => scenarios.has(id) && !bound.has(id) && !marked.has(id)).sort();
  const scenarioSha256 = digest({ requirements: [...requirements].sort(([a], [b]) => a.localeCompare(b)),
    checks: checks.sort((a, b) => a.id.localeCompare(b.id)), automatic: [...marked].sort(), manual: manualOnly, noDeclaredCheck });
  return { requirements, scenarios, automatic: [...marked].sort(), manual: manualOnly, noDeclaredCheck, testSources, scenarioSha256 };
}

function nativeHash() {
  const cli = path.resolve(`target/debug/highgrade${process.platform === 'win32' ? '.exe' : ''}`);
  const result = spawnSync(cli, ['spec-list', '--root', root], { encoding: 'utf8' });
  if (result.status !== 0) fail(`spec-list failed: ${result.stdout || result.stderr}`);
  const digest = JSON.parse(result.stdout).measurements.find((item) => typeof item.trace_sha256 === 'string')?.trace_sha256;
  if (!digest) fail('spec-list has no trace_sha256');
  return digest;
}

const inventory = 'target/nextest/highgrade/list.json';
const nativeReport = 'target/nextest/highgrade/junit.xml';
const record = 'target/nextest/highgrade/run.json';
const attempt = 'target/nextest/highgrade/attempt.json';
const completion = 'target/nextest/highgrade/completion.json';
const attemptId = process.argv[4];
const validAttemptId = (value) => typeof value === 'string' && /^[a-f0-9]{32}$/.test(value);
const isSpec = (file) => file === 'specs/catalog.json' || file.startsWith('specs/requirements/') || file.startsWith('specs/changes/');
const save = (file, value) => {
  const temporary = `${abs(file)}.${process.pid}.${randomUUID()}.tmp`;
  try {
    writeFileSync(temporary, `${JSON.stringify(value, null, 2)}\n`);
    renameSync(temporary, abs(file));
  } finally {
    rmSync(temporary, { force: true });
  }
};

function inputHashes(source) {
  const pinned = ['specs/catalog.json', ...files('specs/requirements').filter((item) => item.endsWith('.json')),
    ...files('specs/changes').filter((item) => item.endsWith('/spec.json')),
    ...source.testSources, ...files('tests/support').filter((item) => item.endsWith('.rs')),
    ...files('src').filter((item) => /\.(rs|html|js)$/.test(item)), 'build.rs', ...files('ui/dist'),
    ...files('kit'), ...files('bundle'), ...files('tests/fixtures'),
    'Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml', '.config/nextest.toml', '.gitattributes',
    'scripts/source-scenarios.mjs', 'scripts/verify.py'];
  return Object.fromEntries(pinned.sort().map((file) => [file, sha(file)]));
}

function beginAttempt() {
  mkdirSync(abs('target/nextest/highgrade'), { recursive: true });
  // Invalidate first, even if source discovery, listing or the runner is interrupted.
  rmSync(abs(attempt), { force: true });
  save(attempt, { schema_version: 1, state: 'running', attempt_id: attemptId, capture_root: root });
  for (const file of [completion, inventory, nativeReport, record, 'target/nextest/highgrade/trace.json']) rmSync(abs(file), { force: true });
}

function capture(source) {
  save(attempt, { schema_version: 1, state: 'running', attempt_id: attemptId, capture_root: root,
    captured_at: new Date().toISOString(), scenario_sha256: source.scenarioSha256, source_hashes: inputHashes(source) });
  return { attempt, state: 'running' };
}

function checkedAttempt(source, state) {
  if (!existsSync(abs(attempt))) fail('native test attempt is missing; rerun python scripts/verify.py tests');
  const captured = readHashed(attempt);
  const previous = captured.value;
  const manifestHashes = { [attempt]: captured.sha256 };
  if (previous.schema_version !== 1 || previous.state !== 'running' || !validAttemptId(previous.attempt_id) ||
      typeof previous.capture_root !== 'string' || !path.isAbsolute(previous.capture_root) ||
      (state === 'running' && (previous.capture_root !== root || previous.attempt_id !== attemptId)) ||
      !previous.source_hashes || !previous.captured_at) fail('native test attempt is incomplete or superseded; rerun python scripts/verify.py tests');
  let completed;
  if (state === 'completed') {
    if (!existsSync(abs(completion))) fail('native test attempt is incomplete; rerun python scripts/verify.py tests');
    const stamp = readHashed(completion);
    completed = stamp.value;
    manifestHashes[completion] = stamp.sha256;
    if (completed.schema_version !== 1 || completed.state !== 'completed' || completed.attempt_id !== previous.attempt_id) {
      fail('native test attempt is incomplete or superseded; rerun python scripts/verify.py tests');
    }
  }
  const current = inputHashes(source);
  const nonSpec = (hashes) => Object.fromEntries(Object.entries(hashes).filter(([file]) => !isSpec(file)));
  if (digest(nonSpec(previous.source_hashes)) !== digest(nonSpec(current)) || previous.scenario_sha256 !== source.scenarioSha256) {
    fail('specification or test inputs changed since capture; rerun nextest with python scripts/verify.py tests');
  }
  return { previous, current, completed, manifestHashes };
}

function passedReport() {
  const xml = readFileSync(abs(nativeReport), 'utf8');
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
  return totals.tests;
}

function complete(source) {
  const { previous } = checkedAttempt(source, 'running');
  const tests = passedReport();
  // Never replace the capture: a newer concurrent capture must not inherit this success.
  save(completion, { schema_version: 1, state: 'completed', attempt_id: previous.attempt_id,
    report_sha256: sha(nativeReport), inventory_sha256: sha(inventory) });
  return { attempt, state: 'completed', native_tests_passed: tests };
}

function recordContext(source) {
  const { previous, current, completed, manifestHashes } = checkedAttempt(source, 'completed');
  const reportHash = sha(nativeReport), inventoryHash = sha(inventory);
  if (completed.report_sha256 !== reportHash || completed.inventory_sha256 !== inventoryHash) {
    fail('native report or inventory changed after completion; rerun nextest with python scripts/verify.py tests');
  }
  const tests = passedReport();
  const sourceHashes = { ...current, ...manifestHashes, [inventory]: inventoryHash, 'specs/catalog.json': nativeHash() };
  const data = { schema_version: 1, tool: 'rust-nextest', command: 'cargo nextest run --locked --profile highgrade',
    scope: 'active native High Grade specifications and Rust integration tests', captured_at: previous.captured_at, capture_root: previous.capture_root,
    exit_code: 0, report: nativeReport, report_sha256: reportHash, spec_files: ['specs/catalog.json'], test_sources: source.testSources,
    source_hashes: sourceHashes, inventory, scenario_sha256: source.scenarioSha256 };
  return { previous, current, tests, data };
}

function assertRecordIdentity(data) {
  const expected = { ...data.source_hashes, [nativeReport]: data.report_sha256 };
  // The capture is read last: capture invalidates it before clearing other reports.
  for (const file of [nativeReport, inventory, completion, attempt]) {
    if (sha(file) !== expected[file]) fail('native verification attempt or report was superseded; rerun scenarios');
  }
}

function currentPublication(expected) {
  const current = recordContext(sources());
  const published = readHashed(record);
  if (digest(current.data) !== digest(expected) || digest(published.value) !== digest(expected)) {
    fail('prepared native run was changed or superseded; rerun scenarios');
  }
  assertRecordIdentity(expected);
  return published;
}

function prepare(source) {
  const { previous, current, tests, data } = recordContext(source);
  const specHashes = (hashes) => Object.fromEntries(Object.entries(hashes).filter(([file]) => isSpec(file)));
  const changedSpec = digest(specHashes(previous.source_hashes)) !== digest(specHashes(current));
  save(record, data);
  // Publication is not a lock: later trace/verify also validate these manifest inputs.
  currentPublication(data);
  return { record, pinned_files: Object.keys(data.source_hashes).length, native_tests_passed: tests,
    native_report_reused: changedSpec, native_report_reason: changedSpec ? 'equivalent_specification' : 'fresh_report' };
}

function verify(source) {
  const { data } = recordContext(source);
  const published = currentPublication(data);
  const tracePath = 'target/nextest/highgrade/trace.json';
  const traceInput = readHashed(tracePath);
  const trace = traceInput.value;
  if (trace.operation !== 'trace') fail('native trace did not produce a trace report');
  const provenance = trace.measurements.filter((item) => typeof item.tool === 'string');
  if (provenance.length !== 1 || ['tool', 'report', 'report_sha256', 'command', 'scope', 'captured_at'].some((field) => provenance[0][field] !== data[field])) {
    fail('native trace does not match the prepared run and report');
  }
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
  // Recheck after consuming trace, including source freshness. This is an observation
  // boundary, not a promise that concurrent writers cannot change files afterward.
  if (currentPublication(data).sha256 !== published.sha256 || sha(tracePath) !== traceInput.sha256) {
    fail('native verification inputs changed while verifying; rerun scenarios');
  }
  assertRecordIdentity(data);
  return { ...counts };
}

try {
  if (!['check', 'capture', 'complete', 'prepare', 'verify'].includes(mode)) fail('usage: node scripts/source-scenarios.mjs check|capture|complete|prepare|verify [root] [attempt-id]');
  if (['capture', 'complete'].includes(mode) && !validAttemptId(attemptId)) fail('native test attempt ID required; use python scripts/verify.py tests');
  if (mode === 'capture') beginAttempt();
  const source = sources();
  const summary = { requirements: source.requirements.size, scenarios: source.scenarios.size,
    automatic: source.automatic.length, manual_check_scenarios: source.manual.length,
    no_declared_check_scenarios: source.noDeclaredCheck.length, no_declared_check_ids: source.noDeclaredCheck };
  if (mode === 'capture') Object.assign(summary, capture(source));
  if (mode === 'complete') Object.assign(summary, complete(source));
  if (mode === 'prepare') Object.assign(summary, prepare(source));
  if (mode === 'verify') Object.assign(summary, verify(source));
  console.log(JSON.stringify({ status: 'passed', ...summary }, null, 2));
} catch (cause) {
  console.error(JSON.stringify({ status: 'failed', message: String(cause.message ?? cause) }));
  process.exitCode = 1;
}
