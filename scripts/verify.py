"""Portable verification entry points shared by Windows and Linux CI."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import uuid

ROOT = Path(__file__).resolve().parents[1]
CLI = ROOT / 'target/debug' / ('highgrade.exe' if os.name == 'nt' else 'highgrade')
REPORTS = ROOT / 'target/nextest/highgrade'
MIN_BUILD_FREE_BYTES = 1024 * 1024 * 1024


def storage_preflight():
    """Refuse a known-low-space build before replacing the last test reports.

    This is a conservative headroom floor, not a bound on Cargo's peak usage.
    Cargo metadata resolves target-dir configuration without compiling sources.
    """
    metadata = subprocess.run(
        ['cargo', 'metadata', '--locked', '--offline', '--no-deps', '--format-version', '1'],
        cwd=ROOT, capture_output=True, text=True, encoding='utf-8', check=True)
    target = Path(json.loads(metadata.stdout)['target_directory'])
    measurements = []
    for destination in dict.fromkeys((ROOT, target)):
        existing = destination
        while not existing.exists():
            if existing == existing.parent:
                raise RuntimeError(f'Cannot inspect build filesystem: {destination}')
            existing = existing.parent
        free = shutil.disk_usage(existing).free
        measurements.append({'destination': str(destination), 'free_bytes': free,
                             'minimum_free_bytes': MIN_BUILD_FREE_BYTES})
    result = {'storage_preflight': measurements}
    print(json.dumps(result, ensure_ascii=True))
    if any(item['free_bytes'] < MIN_BUILD_FREE_BYTES for item in measurements):
        raise RuntimeError(
            'Insufficient build headroom: at least 1 GiB free is required before tests. '
            'Inspect python scripts/target-maintenance.py and preview an explicit '
            'owned cache/scratch selection; no files were removed. '
            'Other worktrees have separate caches. Free space can still change during a build.')
    return result


def run(args, output=None, codes=(0,)):
    result = subprocess.run([str(arg) for arg in args], cwd=ROOT,
                            stdout=subprocess.PIPE if output else None)
    if output:
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_bytes(result.stdout)
    if result.returncode not in codes:
        raise SystemExit(result.returncode or 1)


def report(*args):
    result = subprocess.run([str(CLI), *args, '--root', str(ROOT)], cwd=ROOT,
                            capture_output=True, text=True, encoding='utf-8')
    if result.returncode not in (0, 2):
        raise RuntimeError(result.stdout + result.stderr)
    return json.loads(result.stdout)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('step', choices=['tests', 'specs', 'scenarios', 'inspect'])
    step = parser.parse_args().step
    if step == 'tests':
        storage_preflight()
        run([sys.executable, ROOT / 'scripts/target-maintenance.py', '--sweep', '--apply'])
        attempt_id = uuid.uuid4().hex
        run(['node', 'scripts/source-scenarios.mjs', 'capture', ROOT, attempt_id])
        run(['cargo', 'nextest', 'list', '--locked', '--message-format', 'json'], REPORTS / 'list.json')
        run(['cargo', 'nextest', 'run', '--locked', '--profile', 'highgrade'])
        run(['node', 'scripts/source-scenarios.mjs', 'complete', ROOT, attempt_id])
    elif step == 'specs':
        catalog = report('spec-list')
        if catalog['status'] != 'passed':
            raise RuntimeError(catalog)
        for change in catalog['measurements'][0]['changes']:
            if change['technical'] != 'abandoned':
                validation = report('spec-validate', '--id', change['id'])
                if validation['status'] != 'passed':
                    raise RuntimeError(validation)
    elif step == 'scenarios':
        run(['node', 'scripts/source-scenarios.mjs', 'check'])
        run(['node', 'scripts/source-scenarios.mjs', 'prepare'])
        run([CLI, 'trace', '--root', ROOT, '--record', (REPORTS / 'run.json').relative_to(ROOT).as_posix()],
            REPORTS / 'trace.json', codes=(0, 2))
        run(['node', 'scripts/source-scenarios.mjs', 'verify'])
    else:
        result = report('inspect')
        if any(f['status'] in ('failed', 'unknown') for f in result['findings']):
            raise RuntimeError(result)
        print(json.dumps(result, ensure_ascii=True))


if __name__ == '__main__':
    main()
