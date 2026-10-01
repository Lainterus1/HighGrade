import {test} from 'node:test';
import assert from 'node:assert/strict';
import {installedConfig, verifyArtifact} from '../installed-config.mjs';
import {mkdtempSync, mkdirSync, writeFileSync, rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import path from 'node:path';

const complete = {HIGHGRADE_UI_REQUIRED: 'true', HIGHGRADE_UI_CANDIDATE: '/candidate',
  HIGHGRADE_UI_SOURCE_SHA: 'a'.repeat(40), HIGHGRADE_UI_VERSION: '0.3.8',
  HIGHGRADE_OLD_SOURCE: '/old/kit', HIGHGRADE_OLD_EXE: '/old/highgrade',
  HIGHGRADE_OLD_SOURCE_SHA: 'b'.repeat(40), HIGHGRADE_OLD_VERSION: '0.3.5'};

test('an unconfigured optional local installed test may skip', () => {
  assert.equal(installedConfig({}), undefined);
});
test('required and CI modes fail instead of skipping, even if explicitly disabled in CI', () => {
  for (const env of [{HIGHGRADE_UI_REQUIRED: 'true'}, {CI: 'true'}, {CI: 'true', HIGHGRADE_UI_REQUIRED: 'false'}]) {
    assert.throws(() => installedConfig(env), /missing HIGHGRADE_UI_CANDIDATE/);
  }
});
test('partial inputs always fail and every required input is checked', () => {
  assert.throws(() => installedConfig({HIGHGRADE_UI_CANDIDATE: '/fixture'}), /missing/);
  for (const key of Object.keys(complete).filter(key => key !== 'HIGHGRADE_UI_REQUIRED')) {
    const env = {...complete}; delete env[key];
    assert.throws(() => installedConfig(env), new RegExp(`missing ${key}`));
  }
});
test('fixture identities must be exact and represent two distinct versions', () => {
  assert.equal(installedConfig(complete).HIGHGRADE_UI_VERSION, '0.3.8');
  assert.throws(() => installedConfig({...complete, HIGHGRADE_UI_SOURCE_SHA: 'HEAD'}), /exact Git SHA/);
  assert.throws(() => installedConfig({...complete, HIGHGRADE_OLD_VERSION: 'latest'}), /exact CLI version/);
  assert.throws(() => installedConfig({...complete, HIGHGRADE_OLD_VERSION: '0.3.8'}), /distinct/);
  assert.throws(() => installedConfig({...complete, HIGHGRADE_OLD_SOURCE_SHA: 'a'.repeat(40)}), /distinct/);
  assert.throws(() => installedConfig({...complete, HIGHGRADE_UI_REQUIRED: 'maybe'}), /true or false/);
});
test('wrong source/version and modified fixture bytes are rejected before running binaries', () => {
  const dir = mkdtempSync(path.join(tmpdir(), 'hg-installed-config-'));
  try {
    mkdirSync(path.join(dir, 'kit'));
    writeFileSync(path.join(dir, 'kit/manifest.json'), JSON.stringify({cli_version: '0.3.8'}));
    writeFileSync(path.join(dir, 'candidate.json'), JSON.stringify({owner: 'highgrade-build-release-v1', source_sha: 'a'.repeat(40), files: {}}));
    assert.throws(() => verifyArtifact(dir, 'b'.repeat(40), '0.3.8', 'highgrade'), /source\/version/);
    assert.throws(() => verifyArtifact(dir, 'a'.repeat(40), '0.3.5', 'highgrade'), /source\/version/);
    assert.throws(() => verifyArtifact(dir, 'a'.repeat(40), '0.3.8', 'highgrade'), /fingerprints/);
  } finally { rmSync(dir, {recursive: true}); }
});
