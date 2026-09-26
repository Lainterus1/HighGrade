import { spawnSync } from 'node:child_process';
import { cpSync, mkdtempSync, mkdirSync, readFileSync, rmSync, statSync, utimesSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';

const project = path.resolve('.');
const checker = path.join(project, 'scripts/source-scenarios.mjs');
const fixture = mkdtempSync(path.join(os.tmpdir(), 'highgrade-reuse-'));
const at = (file) => path.join(fixture, file);
const copy = (file) => {
  mkdirSync(path.dirname(at(file)), { recursive: true });
  cpSync(path.join(project, file), at(file), { recursive: true, preserveTimestamps: true });
};
const run = () => spawnSync(process.execPath, [checker, 'prepare', fixture],
  { cwd: project, encoding: 'utf8', maxBuffer: 8 * 1024 * 1024 });
const fail = (message) => { throw new Error(message); };

try {
  for (const file of ['specs', 'src', 'tests', 'Cargo.toml', 'Cargo.lock', '.gitattributes',
    '.config/nextest.toml', 'target/nextest/highgrade/list.json',
    'target/nextest/highgrade/junit.xml', 'target/nextest/highgrade/run.json']) copy(file);
  const recordPath = at('target/nextest/highgrade/run.json');
  const record = JSON.parse(readFileSync(recordPath, 'utf8'));
  record.capture_root = fixture;
  writeFileSync(recordPath, `${JSON.stringify(record)}\n`);

  const specPath = at('specs/changes/HG-0025/spec.json');
  const spec = JSON.parse(readFileSync(specPath, 'utf8'));
  spec.title += ' (fixture metadata)';
  writeFileSync(specPath, `${JSON.stringify(spec)}\n`);
  const equivalent = run();
  const reused = equivalent.status === 0 && JSON.parse(equivalent.stdout).native_report_reused === true;
  if (!reused) fail(`equivalent specification was not reused: ${equivalent.stdout || equivalent.stderr}`);

  const requirement = 'specs/requirements/HG-0021-R3.json';
  const changed = JSON.parse(readFileSync(at(requirement), 'utf8'));
  changed.scenarios[0].then += ' changed after report';
  writeFileSync(at(requirement), `${JSON.stringify(changed)}\n`);
  const semantic = run();
  if (semantic.status === 0 || !semantic.stderr.includes('rerun nextest')) {
    fail(`changed scenario was accepted: ${semantic.stdout || semantic.stderr}`);
  }

  copy(requirement);
  const rustPath = at('src/main.rs');
  const rustTime = statSync(rustPath);
  writeFileSync(rustPath, `${readFileSync(rustPath, 'utf8')}\n// changed after report\n`);
  const rust = run();
  if (rust.status === 0 || !rust.stderr.includes('src/main.rs: source changed after')) {
    fail(`changed Rust source was accepted: ${rust.stdout || rust.stderr}`);
  }
  utimesSync(rustPath, rustTime.atime, rustTime.mtime);
  const rustHash = run();
  if (rustHash.status === 0 || !rustHash.stderr.includes('specification or test inputs changed')) {
    fail(`changed Rust hash was accepted with restored mtime: ${rustHash.stdout || rustHash.stderr}`);
  }
  console.log(JSON.stringify({ status: 'passed', equivalent_reused: true,
    changed_scenario_rejected: true, changed_rust_rejected: true, changed_rust_hash_rejected: true }));
} catch (cause) {
  console.error(JSON.stringify({ status: 'failed', message: String(cause.message ?? cause) }));
  process.exitCode = 1;
} finally {
  const resolved = path.resolve(fixture);
  if (resolved.startsWith(path.resolve(os.tmpdir()) + path.sep) &&
      path.basename(resolved).startsWith('highgrade-reuse-')) {
    rmSync(resolved, { recursive: true, force: true });
  }
}
