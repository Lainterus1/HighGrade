from __future__ import annotations

import re
import os
import stat
import subprocess
import zipfile
from pathlib import Path
from typing import Any

from .core import normalize_relative, sha256_bytes, sha256_file, matches_path
from .processes import run_bounded
from .validation import safe_relative


ACCEPTANCE_PATH = ".code-health-audit/accepted-findings.json"
MANIFEST_NAMES = {
    "Pipfile",
    "Pipfile.lock",
    "pdm.lock",
    "poetry.lock",
    "pyproject.toml",
    "requirements.in",
    "requirements.txt",
    "setup.cfg",
    "setup.py",
    "uv.lock",
}


class GitScopeError(RuntimeError):
    pass


def _run(
    command: list[str],
    *,
    cwd: Path,
    check: bool = True,
    text: bool = False,
) -> subprocess.CompletedProcess[Any]:
    environment = os.environ.copy()
    environment['GIT_OPTIONAL_LOCKS'] = '0'
    if command[0] == 'git':
        if command[1] == 'diff':
            command = command[:2] + ['--no-ext-diff', '--no-textconv'] + command[2:]
        command = ['git', '-c', 'core.fsmonitor=false'] + command[1:]
    result = run_bounded(
        command, env=environment,
        cwd=cwd,
        capture_output=True,
        check=False,
        text=text,
        encoding="utf-8" if text else None,
        errors="replace" if text else None,
        timeout=30,
    )
    if check and result.returncode != 0:
        stderr = result.stderr.strip() if text else result.stderr.decode("utf-8", "replace").strip()
        raise GitScopeError(f"{' '.join(command)} failed ({result.returncode}): {stderr}")
    return result


def is_git_repository(root: Path) -> bool:
    try:
        result = _run(["git", "rev-parse", "--is-inside-work-tree"], cwd=root, check=False, text=True)
    except FileNotFoundError:
        return False
    return result.returncode == 0 and result.stdout.strip() == "true"


def _excluded(relative: str, excluded_directories: set[str]) -> bool:
    parts = normalize_relative(relative).split("/")
    return any(part in excluded_directories for part in parts[:-1])


def _is_link(path: Path) -> bool:
    info = path.lstat()
    return path.is_symlink() or bool(getattr(info, 'st_file_attributes', 0) & stat.FILE_ATTRIBUTE_REPARSE_POINT)


def repository_files(root: Path, exclusions: set[str] | None = None) -> list[str]:
    if is_git_repository(root):
        result = _run(['git', 'ls-files', '-co', '--exclude-standard', '-z'], cwd=root)
        return sorted({safe_relative(normalize_relative(raw.decode('utf-8', 'surrogateescape')))
                       for raw in result.stdout.split(b'\0') if raw})
    if exclusions is None:
        exclusions = {'.git', '.hg', '__pycache__', '.venv', 'venv', 'node_modules', '.pytest_cache', '.mypy_cache', '.ruff_cache', '.tox', 'build', 'dist'}
    exclusions = exclusions | {'.git', '.hg'}
    found = []
    for directory, dirs, names in os.walk(root, followlinks=False):
        for name in list(dirs):
            path = Path(directory) / name
            if name in exclusions or _is_link(path):
                dirs.remove(name)
                found.append(path.relative_to(root).as_posix())
        found.extend((Path(directory) / name).relative_to(root).as_posix() for name in names)
    return sorted(found)


def _inside(root: Path, relative: str) -> Path:
    path = root / safe_relative(relative)
    resolved = path.resolve()
    if resolved != root.resolve() and root.resolve() not in resolved.parents:
        raise GitScopeError(f'path escapes repository: {relative}')
    return path


def effective_source_roots(root: Path, policy: dict[str, Any]) -> list[str]:
    roots = [path for path in policy.get('source_roots', ['.', 'src']) if _inside(root, path).is_dir()]
    return sorted(roots, key=lambda path: (-len((root / path).resolve().parts), path))


def discover_scope(root: Path, policy: dict[str, Any]) -> tuple[list[str], list[dict[str, str]]]:
    included, excluded = [], []
    for relative in repository_files(root, set(policy['excluded_directories'])):
        if (root / relative).is_dir():
            excluded.append({'path': relative, 'reason': 'directory pruned from traversal'})
            continue
        if Path(relative).suffix not in {'.py', '.pyi'}:
            continue
        reason = None
        if relative.endswith('.pyi') and not policy.get('include_pyi_loc'):
            reason = 'stub analysis disabled'
        elif _excluded(relative, set(policy['excluded_directories'])):
            reason = 'excluded directory'
        elif matches_path(relative, policy.get('excluded_paths', [])):
            reason = 'excluded path'
        elif not matches_path(relative, policy.get('include_paths', ['.'])):
            reason = 'outside include paths'
        if reason:
            excluded.append({'path': relative, 'reason': reason})
        elif _inside(root, relative).is_file():
            included.append(relative)
    return included, excluded


def discover_python_files(root: Path, policy: dict[str, Any]) -> list[str]:
    return discover_scope(root, policy)[0]


def resolve_main_diff(root: Path, base_name: str, python_files: list[str]) -> dict[str, Any]:
    if not is_git_repository(root):
        raise GitScopeError("diff mode requires a Git repository")
    top = _run(['git', 'rev-parse', '--show-toplevel'], cwd=root, text=True).stdout.strip()
    if Path(top).resolve() != root.resolve():
        raise GitScopeError('diff root must be the Git worktree root')
    candidates = []
    if base_name == "main":
        candidates.extend(["refs/remotes/origin/main", "refs/heads/main"])
    else:
        candidates.extend([base_name, f"refs/remotes/origin/{base_name}", f"refs/heads/{base_name}"])
    resolved_ref = None
    resolved_sha = None
    for candidate in candidates:
        result = _run(["git", "rev-parse", "--verify", f"{candidate}^{{commit}}"], cwd=root, check=False, text=True)
        if result.returncode == 0:
            resolved_ref = candidate
            resolved_sha = result.stdout.strip()
            break
    if not resolved_ref or not resolved_sha:
        raise GitScopeError(f"no local base found for {base_name}")
    head_sha = _run(["git", "rev-parse", "HEAD"], cwd=root, text=True).stdout.strip()
    merge_base = _run(["git", "merge-base", "HEAD", resolved_sha], cwd=root, text=True).stdout.strip()

    changed_name_result = _run(
        ["git", "diff", "--name-only", "-z", merge_base, "--", "*.py"],
        cwd=root,
    )
    changed_files = {
        normalize_relative(raw.decode("utf-8", "surrogateescape"))
        for raw in changed_name_result.stdout.split(b"\0")
        if raw
    }
    untracked_result = _run(
        ["git", "ls-files", "--others", "--exclude-standard", "-z", "--", "*.py"],
        cwd=root,
    )
    untracked = {
        normalize_relative(raw.decode("utf-8", "surrogateescape"))
        for raw in untracked_result.stdout.split(b"\0")
        if raw
    }
    changed_files.update(untracked)

    available = set(python_files)
    changed_lines: dict[str, list[int]] = {}
    hunk_pattern = re.compile(r"^@@ -\d+(?:,\d+)? \+(\d+)(?:,(\d+))? @@")
    for relative in sorted(changed_files):
        if relative not in available:
            continue
        path = root / relative
        if relative in untracked:
            count = len(path.read_text(encoding="utf-8").splitlines())
            changed_lines[relative] = list(range(1, count + 1))
            continue
        patch = _run(
            ["git", "diff", "--unified=0", "--no-color", merge_base, "--", relative],
            cwd=root,
            text=True,
        ).stdout
        lines: set[int] = set()
        for line in patch.splitlines():
            match = hunk_pattern.match(line)
            if not match:
                continue
            start = int(match.group(1))
            count = int(match.group(2) or "1")
            lines.update(range(start, start + count))
        changed_lines[relative] = sorted(lines)

    acceptance_diff = _run(
        ["git", "diff", "--name-only", "-z", merge_base, "--", ACCEPTANCE_PATH],
        cwd=root,
    ).stdout
    acceptance_untracked = _run(
        ["git", "ls-files", "--others", "--exclude-standard", "-z", "--", ACCEPTANCE_PATH],
        cwd=root,
    ).stdout

    diff_bytes = _run(["git", "diff", "--binary", merge_base, "--"], cwd=root).stdout
    digest_parts = [diff_bytes]
    for relative in sorted(untracked):
        digest_parts.append(relative.encode("utf-8"))
        digest_parts.append((root / relative).read_bytes())

    return {
        "base_ref": resolved_ref,
        "base_sha": resolved_sha,
        "merge_base": merge_base,
        "head_sha": head_sha,
        "changed_lines": changed_lines,
        "changed_files": sorted(changed_files),
        "untracked_files": sorted(untracked),
        "acceptance_changed": bool(acceptance_diff or acceptance_untracked),
        "worktree_diff_sha256": sha256_bytes(b"\0".join(digest_parts)),
    }


def materialize_archive(root: Path, revision: str, destination: Path) -> Path:
    destination.mkdir(parents=True, exist_ok=True)
    archive = destination / "base.zip"
    extracted = destination / "base"
    _run(["git", "archive", "--format=zip", f"--output={archive}", revision], cwd=root)
    extracted.mkdir(parents=True, exist_ok=True)
    root_resolved = extracted.resolve()
    with zipfile.ZipFile(archive) as bundle:
        for member in bundle.infolist():
            target = (extracted / member.filename).resolve()
            if target != root_resolved and root_resolved not in target.parents:
                raise GitScopeError(f"unsafe archive member: {member.filename}")
        bundle.extractall(extracted)
    return extracted


def read_file_at_revision(root: Path, revision: str, relative: str) -> bytes | None:
    result = _run(["git", "show", f"{revision}:{relative}"], cwd=root, check=False)
    if result.returncode != 0:
        return None
    return result.stdout


def acceptance_is_clean(root: Path) -> bool:
    if not is_git_repository(root):
        return False
    tracked = _run(
        ["git", "ls-files", "--error-unmatch", "--", ACCEPTANCE_PATH],
        cwd=root,
        check=False,
    )
    if tracked.returncode != 0:
        return False
    head = _run(
        ["git", "rev-parse", "--verify", f"HEAD:{ACCEPTANCE_PATH}"],
        cwd=root,
        check=False,
        text=True,
    )
    index = _run(['git', 'rev-parse', '--verify', f':{ACCEPTANCE_PATH}'], cwd=root, check=False, text=True)
    committed = read_file_at_revision(root, 'HEAD', ACCEPTANCE_PATH)
    current = _inside(root, ACCEPTANCE_PATH).read_bytes()
    # Permit Git's ordinary LF/CRLF checkout conversion without executing clean filters.
    same_content = committed is not None and committed.replace(b'\r\n', b'\n') == current.replace(b'\r\n', b'\n')
    return (
        index.returncode == 0
        and index.stdout.strip() == head.stdout.strip()
        and head.returncode == 0
        and same_content
    )


def snapshot_repository(root: Path, python_files: list[str], policy: dict[str, Any] | None = None) -> dict[str, Any]:
    # Rediscover both times: source arguments cannot hide newly created files.
    files = set(repository_files(root, set(policy['excluded_directories']) if policy is not None else None)) | set(python_files)
    hashes = {}
    for relative in sorted(files):
        path = root / relative
        if not path.exists() and not path.is_symlink():
            continue
        if _is_link(path):
            hashes[relative] = sha256_bytes(('link:' + os.readlink(path)).encode('utf-8'))
        elif _inside(root, relative).is_file():
            hashes[relative] = sha256_file(path)
    status = index = None
    if is_git_repository(root):
        status = _run(['git', 'status', '--porcelain=v1', '-z'], cwd=root).stdout.hex()
        index = sha256_bytes(_run(['git', 'ls-files', '--stage', '-z'], cwd=root).stdout)
    return {'hashes': hashes, 'git_status_hex': status, 'index_sha256': index}


def compare_snapshots(before: dict[str, Any], after: dict[str, Any]) -> list[str]:
    changed = [
        path
        for path in sorted(set(before["hashes"]) | set(after["hashes"]))
        if before["hashes"].get(path) != after["hashes"].get(path)
    ]
    if before.get("git_status_hex") != after.get("git_status_hex"):
        changed.append("<git-status>")
    if before.get('index_sha256') != after.get('index_sha256'):
        changed.append('<git-index>')
    return changed
