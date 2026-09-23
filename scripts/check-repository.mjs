import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const errors = [];
const observations = [];
const args = process.argv.slice(2);
if (args.some(arg => arg !== '--verify-import')) {
  console.error('Usage: node scripts/check-repository.mjs [--verify-import]');
  process.exit(2);
}
function local(relative) {
  const resolved = path.resolve(root, relative);
  const rel = path.relative(root, resolved);
  if (rel.startsWith('..' + path.sep) || rel === '..' || path.isAbsolute(rel)) throw new Error(`Path outside root: ${relative}`);
  return resolved;
}
function readJson(relative) { return JSON.parse(fs.readFileSync(local(relative), 'utf8').replace(/^\uFEFF/, '')); }
try {
  const docs = readJson('.highgrade/project/documents.json');
  const ids = docs.documents.map(doc => doc.id);
  for (const id of ['readme', 'agents', 'architecture', 'engineering', 'development']) {
    if (ids.filter(value => value === id).length !== 1) errors.push(`Required document role missing or duplicated: ${id}`);
  }
  for (const doc of docs.documents) {
    const bytes = fs.readFileSync(local(doc.path));
    observations.push({path: doc.path, bytes: bytes.length, lines: bytes.toString('utf8').split(/\r?\n/).length, budget: 'not-assessed'});
  }
  for (const name of ['svg-vectorizer', 'code-health-audit']) {
    const manifest = readJson(`plugins/${name}/.codex-plugin/plugin.json`);
    if (manifest.name !== name) errors.push(`Plugin name mismatch: ${name}`);
    const skill = name === 'svg-vectorizer' ? 'vectorize' : name;
    fs.accessSync(local(`plugins/${name}/skills/${skill}/SKILL.md`));
  }
  let verified = 0;
  if (args.includes('--verify-import')) {
    const imported = readJson('docs/imports/plugins-import.json');
    for (const plugin of imported.plugins) for (const file of plugin.files) {
      const data = fs.readFileSync(local(`${plugin.relative_destination}/${file.path}`));
      if (crypto.createHash('sha256').update(data).digest('hex') !== file.sha256) errors.push(`Changed since import: ${plugin.name}/${file.path}`);
      verified++;
    }
  }
  console.log(JSON.stringify({scope:'repository-structure',status:errors.length?'FAILED':'PASSED',observations,import_files_verified:verified,errors,limitations:['No functional plugin tests executed','No semantic documentation review','Document budgets are not assessed here; run highgrade inspect']},null,2));
  process.exitCode = errors.length ? 1 : 0;
} catch (error) {
  console.error(JSON.stringify({status:'INCOMPLETE',error:error.message}));
  process.exitCode=2;
}

