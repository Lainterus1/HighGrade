"""Actual runtime identity and preflight; never installs or imports project code."""
from __future__ import annotations
from functools import lru_cache
import importlib.metadata
import json
import platform
import shutil
import sys
from pathlib import Path
from .core import canonical_json, sha256_text, sha256_file

SCRIPTS = Path(__file__).resolve().parents[1]


def package_version(name):
    try:
        return importlib.metadata.version(name)
    except importlib.metadata.PackageNotFoundError:
        return None


@lru_cache(maxsize=1)
def runtime_evidence():
    toolchain = json.loads((SCRIPTS/'data/toolchain.json').read_text(encoding='utf-8'))
    packages = {name: package_version(name) for name in sorted({**toolchain['tools'], **toolchain['dependencies']})}
    identity = {'python': platform.python_version(), 'implementation': platform.python_implementation(),
                'system': platform.system(), 'machine': platform.machine(), 'packages': packages,
                'runner_sha256': sha256_text(canonical_json({path.relative_to(SCRIPTS).as_posix():sha256_file(path)
                    for path in sorted(SCRIPTS.rglob('*.py')) if 'tests' not in path.relative_to(SCRIPTS).parts}))}
    problems = []
    tested = any(all(identity[key] == value for key,value in runtime.items()) for runtime in toolchain['tested_runtimes'])
    if not tested:
        problems.append('This Python/platform combination has not been validated for this release')
    for name, expected in {**toolchain['tools'], **toolchain['dependencies']}.items():
        if packages[name] != expected:
            problems.append(f'{name}: expected {expected}, found {packages[name] or "not installed"}')
    return {**identity, 'sha256':sha256_text(canonical_json(identity)), 'executable':sys.executable,
            'validated':not problems, 'problems':problems}


def preflight(root, mode, base, policy):
    from .git_scope import discover_scope, effective_source_roots, is_git_repository, resolve_main_diff
    errors = []
    runtime = runtime_evidence()
    files, excluded, roots, diff = [], [], [], None
    git = shutil.which('git')
    try:
        if not root.is_dir():
            raise ValueError('repository root does not exist')
        files, excluded = discover_scope(root, policy)
        roots = effective_source_roots(root, policy)
        if not files:
            errors.append('No Python files in the selected scope')
        if mode == 'diff':
            if not git or not is_git_repository(root):
                raise ValueError('diff requires Git and a repository with a local base')
            resolved = resolve_main_diff(root,base,files)
            diff = {key:resolved[key] for key in ('base_ref','base_sha','merge_base','head_sha')}
    except Exception as exc:
        errors.append(f'{type(exc).__name__}: {exc}')
    return {'ready':not errors and runtime['validated'], 'root':str(root),'mode':mode,
            'runtime':runtime,'git':git,'base':diff,'policy_sha256':sha256_text(canonical_json(policy)),
            'scope':{'python_files':files,'excluded_files':excluded,'source_roots':roots,
                     'include_paths':policy['include_paths'],'excluded_paths':policy['excluded_paths'],
                     'test_paths':policy['test_paths'],'watch_hotspot_limit':policy['watch_hotspot_limit']},
            'errors':errors,'coverage':'Not checked by preflight; an observed capture is required for a complete audit'}
