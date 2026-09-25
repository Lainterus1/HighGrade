"""Build an exact Git revision with one Cargo cache and one owned candidate.

Project tooling only; never installs or publishes the delivery.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import stat
import subprocess
import tarfile
import tempfile


def safe_path(root, path):
    """Reject escapes and Windows junctions as well as ordinary symlinks."""
    root = Path(root).absolute()
    path = Path(path).absolute()
    path.relative_to(root)
    for part in [path, *path.parents]:
        if part.exists() or part.is_symlink():
            info = part.lstat()
            if stat.S_ISLNK(info.st_mode) or getattr(info, 'st_file_attributes', 0) & 0x400:
                raise ValueError(f'Linked path is forbidden: {part}')
    return path


def files(root):
    result = {}
    for path in root.rglob('*'):
        safe_path(root, path)
        if path.is_file():
            result[path.relative_to(root).as_posix()] = hashlib.sha256(path.read_bytes()).hexdigest()
        elif not path.is_dir():
            raise ValueError(f'Unsupported file: {path}')
    return result


def check_tree(root, directory):
    """Check before Cargo can write through any nested cache junction."""
    safe_path(root, directory)
    pending = [directory]
    while pending:
        for path in pending.pop().iterdir():
            safe_path(root, path)
            if path.is_dir():
                pending.append(path)


def verify_candidate(root, candidate):
    safe_path(root, candidate)
    if not candidate.exists():
        return
    actual = files(candidate)
    manifest = candidate / 'candidate.json'
    if not manifest.is_file():
        raise ValueError('Unowned candidate; preserve and move it manually')
    record = json.loads(manifest.read_text(encoding='utf-8'))
    actual.pop('candidate.json')
    if record.get('owner') != 'highgrade-build-release-v1' or record.get('files') != actual:
        raise ValueError('Candidate contains changed or foreign files; preserved')
    # Empty foreign directories are also data, not ours to delete.
    expected_dirs = {str(p) for name in actual for p in Path(name).parents if str(p) != '.'}
    actual_dirs = {str(p.relative_to(candidate)) for p in candidate.rglob('*') if p.is_dir()}
    if actual_dirs != expected_dirs:
        raise ValueError('Candidate contains foreign directories; preserved')


def extract(archive, destination):
    with tarfile.open(archive) as source:
        for member in source.getmembers():
            path = destination / member.name
            if Path(member.name).is_absolute() or '..' in Path(member.name).parts:
                raise ValueError('Unsafe Git archive path')
            safe_path(destination, path)
            if member.isdir():
                path.mkdir(parents=True, exist_ok=True)
            elif member.isfile():
                path.parent.mkdir(parents=True, exist_ok=True)
                with source.extractfile(member) as stream, path.open('wb') as output:
                    shutil.copyfileobj(stream, output)
            else:
                raise ValueError(f'Unsupported Git archive member: {member.name}')


def build(root, revision):
    root = Path(root).absolute()
    target = safe_path(root, root / 'target')
    target.mkdir(exist_ok=True)
    project = safe_path(root, target / 'highgrade')
    project.mkdir(exist_ok=True)
    work = safe_path(root, project / 'work')
    reports = safe_path(root, project / 'reports')
    work.mkdir(exist_ok=True)
    reports.mkdir(exist_ok=True)
    candidate = project / 'candidate'
    previous = project / 'candidate.previous'
    # Shared across candidate builds, separate from ordinary developer builds.
    cache = project / 'build-cache'
    lock = safe_path(root, work / 'release-build.lock')
    # One build/replace at a time. Never clear a stale lock automatically.
    with lock.open('x'):
        pass
    try:
        check_tree(root, target)
        if previous.exists():
            raise ValueError('Recovery candidate exists: target/highgrade/candidate.previous; preserved')
        verify_candidate(root, candidate)
        log_path = safe_path(root, reports / 'release-build.log')
        with log_path.open('w', encoding='utf-8') as log:
            try:
                sha = subprocess.check_output(
                    ['git', 'rev-parse', '--verify', '--end-of-options', revision + '^{commit}'],
                    cwd=root, text=True, stderr=subprocess.STDOUT).strip()
                log.write(f'source_sha={sha}\ncargo_target_dir={cache}\n')
                log.flush()
                with tempfile.TemporaryDirectory(prefix='release-', dir=work) as temporary:
                    temp = Path(temporary)
                    archive = temp / 'source.tar'
                    subprocess.run(['git', 'archive', '--format=tar', '-o', str(archive), sha],
                                   cwd=root, check=True, stdout=log, stderr=log)
                    source = temp / 'source'
                    source.mkdir()
                    extract(archive, source)
                    environment = os.environ.copy()
                    environment['CARGO_TARGET_DIR'] = str(cache)
                    build = subprocess.run(
                        ['cargo', 'build', '--release', '--locked',
                         '--message-format=json-render-diagnostics', '--manifest-path',
                         str(source / 'Cargo.toml')],
                        cwd=source, env=environment, stdout=subprocess.PIPE,
                        stderr=subprocess.STDOUT, text=True, encoding='utf-8',
                        errors='replace')
                    log.write(build.stdout)
                    log.flush()
                    build.check_returncode()
                    artifacts = []
                    for line in build.stdout.splitlines():
                        try:
                            message = json.loads(line)
                        except json.JSONDecodeError:
                            continue
                        artifact_target = message.get('target') or {}
                        if (message.get('reason') == 'compiler-artifact'
                                and artifact_target.get('name') == 'highgrade'
                                and 'bin' in artifact_target.get('kind', [])
                                and message.get('executable')):
                            artifacts.append(safe_path(cache, message['executable']))
                    if len(artifacts) != 1 or not artifacts[0].is_file():
                        raise ValueError('Cargo did not report one highgrade executable')
                    staged = temp / 'candidate'
                    staged.mkdir()
                    executable = 'highgrade.exe' if os.name == 'nt' else 'highgrade'
                    shutil.copy2(artifacts[0], staged / executable)
                    shutil.copytree(source / 'kit', staged / 'kit')
                    record = {'owner': 'highgrade-build-release-v1', 'source_sha': sha,
                              'cargo_target_dir': str(cache), 'files': files(staged)}
                    (staged / 'candidate.json').write_text(
                        json.dumps(record, indent=2) + '\n', encoding='utf-8')
                    verify_candidate(root, candidate)
                    if candidate.exists():
                        candidate.rename(previous)
                    try:
                        staged.rename(candidate)
                    except OSError:
                        if previous.exists():
                            previous.rename(candidate)
                        raise
                    if previous.exists():
                        verify_candidate(root, previous)
                        shutil.rmtree(previous)
                return record
            except Exception as error:
                log.write(f'FAILED: {error}\n')
                raise
    finally:
        lock.unlink()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('revision', help='Git commit or ref; resolved to an exact SHA')
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    try:
        record = build(root, args.revision)
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        parser.exit(1, f'Build failed: {error}\nSee target/highgrade/reports/release-build.log when created.\n')
    print(json.dumps({'source_sha': record['source_sha'], 'candidate': 'target/highgrade/candidate'}))


if __name__ == '__main__':
    main()
