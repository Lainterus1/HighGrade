// A required installed run must fail at collection time instead of reporting a skip.
import {createHash} from 'node:crypto';
import {readFileSync, readdirSync, lstatSync} from 'node:fs';
import {spawnSync} from 'node:child_process';
import path from 'node:path';

const inputs = ['HIGHGRADE_UI_CANDIDATE', 'HIGHGRADE_UI_SOURCE_SHA', 'HIGHGRADE_UI_VERSION',
  'HIGHGRADE_OLD_SOURCE', 'HIGHGRADE_OLD_EXE', 'HIGHGRADE_OLD_SOURCE_SHA', 'HIGHGRADE_OLD_VERSION'];
export function installedConfig(env = process.env) {
  if (env.HIGHGRADE_UI_REQUIRED && !['true', 'false', '1', '0'].includes(env.HIGHGRADE_UI_REQUIRED)) {
    throw Error('HIGHGRADE_UI_REQUIRED must be true or false');
  }
  const required = ['true', '1'].includes(env.HIGHGRADE_UI_REQUIRED) || ['true', '1'].includes(env.CI);
  if (!required && !inputs.some(key => env[key])) return undefined;
  const missing = inputs.filter(key => !env[key]?.trim());
  if (missing.length) throw Error(`Installed UI fixture is required; missing ${missing.join(', ')}`);
  for (const key of ['HIGHGRADE_UI_SOURCE_SHA', 'HIGHGRADE_OLD_SOURCE_SHA']) {
    if (!/^[0-9a-f]{40}$/.test(env[key])) throw Error(`${key} must be an exact Git SHA`);
  }
  for (const key of ['HIGHGRADE_UI_VERSION', 'HIGHGRADE_OLD_VERSION']) {
    if (!/^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/.test(env[key])) throw Error(`${key} must be an exact CLI version`);
  }
  if (env.HIGHGRADE_UI_SOURCE_SHA === env.HIGHGRADE_OLD_SOURCE_SHA || env.HIGHGRADE_UI_VERSION === env.HIGHGRADE_OLD_VERSION) {
    throw Error('Installed UI requires distinct current and previous source revisions and versions');
  }
  return Object.fromEntries(inputs.map(key => [key, env[key]]));
}

export function verifyArtifact(directory, sourceSha, version, binaryName) {
  directory = path.resolve(directory);
  const readJson = file => JSON.parse(readFileSync(path.join(directory, file), 'utf8'));
  const record = readJson('candidate.json');
  const manifest = readJson('kit/manifest.json');
  if (record.owner !== 'highgrade-build-release-v1' || record.source_sha !== sourceSha || manifest.cli_version !== version) {
    throw Error('Installed UI artifact source/version does not match the required fixture');
  }
  const files = {};
  function walk(dir) {
    for (const name of readdirSync(dir)) {
      const file = path.join(dir, name), stat = lstatSync(file);
      if (stat.isSymbolicLink()) throw Error(`Linked installed fixture file: ${file}`);
      if (stat.isDirectory()) walk(file);
      else if (stat.isFile()) {
        const relative = path.relative(directory, file).split(path.sep).join('/');
        if (relative !== 'candidate.json') files[relative] = createHash('sha256').update(readFileSync(file)).digest('hex');
      } else throw Error(`Unsupported installed fixture file: ${file}`);
    }
  }
  walk(directory);
  if (!record.files || JSON.stringify(Object.entries(files).sort()) !== JSON.stringify(Object.entries(record.files).sort())) {
    throw Error('Installed UI artifact file fingerprints do not match candidate.json');
  }
  const executable = path.join(directory, binaryName);
  const result = spawnSync(executable, ['--version'], {encoding: 'utf8', windowsHide: true, timeout: 10000});
  let report;
  try { report = JSON.parse(result.stdout); } catch { throw Error(`Installed UI executable probe failed: ${result.error ?? result.stdout}`); }
  if (result.status !== 0 || report.operation !== 'version' || report.status !== 'passed' || report.measurements?.[0]?.version !== version) {
    throw Error('Installed UI executable version does not match the required fixture');
  }
  return manifest;
}
