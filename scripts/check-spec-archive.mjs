import { createHash } from 'node:crypto';
import { readFileSync, readdirSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const inside = (rel) => {
  const file = path.resolve(root, rel);
  if (!file.startsWith(`${root}${path.sep}`)) throw new Error(`outside project: ${rel}`);
  return file;
};
const read = (rel) => readFileSync(inside(rel));
const json = (rel) => JSON.parse(read(rel).toString('utf8'));
const hash = (rel) => createHash('sha256').update(read(rel)).digest('hex');

try {
  const old = json('docs/archive/legacy-openspec/migration.json');
  for (const row of old.source_files) {
    if (hash(row.archive) !== row.sha256) throw new Error(`OpenSpec source changed: ${row.archive}`);
  }
  const prefix = 'docs/archive/legacy-native-specs/2026-09-25/';
  const files = json(`${prefix}file-hashes.json`);
  for (const [rel, expected] of Object.entries(files)) {
    if (hash(`${prefix}specs/${rel}`) !== expected) throw new Error(`native snapshot changed: ${rel}`);
  }
  const mapping = json(`${prefix}mapping.json`);
  if (Object.keys(mapping.requirements).length !== 36 || Object.keys(mapping.scenarios).length !== 68) {
    throw new Error('canonical ID map incomplete');
  }
  const archivedRequirements = readdirSync(inside(`${prefix}specs/requirements`))
    .filter((name) => name.endsWith('.json'))
    .map((name) => json(`${prefix}specs/requirements/${name}`));
  const activeRequirements = readdirSync(inside('specs/requirements'))
    .filter((name) => name.endsWith('.json'))
    .map((name) => json(`specs/requirements/${name}`));
  const oldRequirementIds = new Set(archivedRequirements.map((item) => item.id));
  const oldScenarioIds = new Set(archivedRequirements.flatMap((item) => item.scenarios.map((scenario) => scenario.id)));
  const activeRequirementIds = new Set(activeRequirements.map((item) => item.id));
  const activeScenarioIds = new Set(activeRequirements.flatMap((item) => item.scenarios.map((scenario) => scenario.id)));
  const retiredRequirements = new Set(mapping.retired_requirements);
  const retiredScenarios = new Set(mapping.retired_scenarios);
  for (const oldId of oldRequirementIds) {
    const target = mapping.requirements[oldId] ?? oldId;
    if (retiredRequirements.has(oldId) === activeRequirementIds.has(target)) throw new Error(`requirement fate: ${oldId}`);
  }
  for (const oldId of oldScenarioIds) {
    const target = mapping.scenarios[oldId] ?? oldId;
    const owner = archivedRequirements.find((item) => item.scenarios.some((scenario) => scenario.id === oldId)).id;
    const retired = retiredRequirements.has(owner) || retiredScenarios.has(oldId);
    if (retired === activeScenarioIds.has(target)) throw new Error(`scenario fate: ${oldId}`);
  }
  for (const oldId of Object.keys(mapping.requirements)) if (!oldRequirementIds.has(oldId)) throw new Error(`unknown old requirement: ${oldId}`);
  for (const oldId of Object.keys(mapping.scenarios)) if (!oldScenarioIds.has(oldId)) throw new Error(`unknown old scenario: ${oldId}`);
  if (activeRequirementIds.size !== activeRequirements.length || activeScenarioIds.size !== activeRequirements.flatMap((item) => item.scenarios).length) {
    throw new Error('duplicate canonical ID');
  }
  console.log(JSON.stringify({ status: 'passed', openspec_files: old.source_files.length, native_files: Object.keys(files).length, mapped_requirements: 36, mapped_scenarios: 68 }));
} catch (error) {
  console.error(JSON.stringify({ status: 'failed', message: String(error.message ?? error) }));
  process.exitCode = 1;
}
