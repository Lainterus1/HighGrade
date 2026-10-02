"""Explicit content-addressed evidence export and owned scratch lifecycle (stdlib only)."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import platform
import re
import shutil
import stat
import subprocess
import sys
import uuid

BASE = 'docs/evidence/objects'
RUNS = 'target/highgrade/runs'


def canonical(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':')).encode()


def digest(data):
    return hashlib.sha256(data).hexdigest()


def safe(root, relative):
    parts = PurePosixPath(relative).parts
    if not parts or PurePosixPath(relative).is_absolute() or any(p in ('.', '..') for p in parts) or '\\' in relative or ':' in relative:
        raise ValueError('Unsafe path: ' + relative)
    path = root
    for part in parts:
        path = path / part
        if path.exists() or path.is_symlink():
            info = path.lstat()
            if stat.S_ISLNK(info.st_mode) or getattr(info, 'st_file_attributes', 0) & 0x400:
                raise ValueError('Linked path: ' + relative)
    return path


def put(path, data):
    path.parent.mkdir(parents=True, exist_ok=True)
    try:
        with path.open('xb') as stream:
            stream.write(data)
    except FileExistsError:
        if path.read_bytes() != data:
            raise ValueError('Conflicting stored evidence: ' + str(path))


def export(root, files, metadata):
    """Copy an explicit selection, validate every stored byte, never remove sources."""
    root = Path(root).resolve(strict=True)
    if not files or len(set(files)) != len(files):
        raise ValueError('Select nonempty unique files')
    entries = []
    # Validate all sources and destination ancestry before writing anything.
    payloads = []
    for name in sorted(files):
        source = safe(root, name)
        if not source.is_file():
            raise ValueError('Missing report: ' + name)
        data = source.read_bytes()
        sha = digest(data)
        destination = f'{BASE}/{sha}.blob'
        safe(root, destination)
        entries.append({'source': name, 'path': destination, 'sha256': sha, 'bytes': len(data)})
        payloads.append((destination, data))
    manifest = {'schema': 1, 'metadata': metadata, 'files': entries}
    data = canonical(manifest)
    name = f'{BASE}/{digest(data)}.json'
    safe(root, name)
    for destination, content in payloads:
        put(safe(root, destination), content)
    put(safe(root, name), data)
    verify(root, name)
    return name


def verify(root, manifest):
    root = Path(root).resolve(strict=True)
    if not re.fullmatch(re.escape(BASE) + r'/[a-f0-9]{64}\.json', manifest):
        raise ValueError('Unknown manifest location')
    data = safe(root, manifest).read_bytes()
    if digest(data) != Path(manifest).stem:
        raise ValueError('Manifest hash mismatch')
    value = json.loads(data)
    if value.get('schema') != 1 or not value.get('files'):
        raise ValueError('Invalid evidence manifest')
    for item in value['files']:
        sha = item['sha256']
        if not re.fullmatch('[a-f0-9]{64}', sha) or item['path'] != f'{BASE}/{sha}.blob':
            raise ValueError('Invalid evidence object')
        content = safe(root, item['path']).read_bytes()
        if len(content) != item['bytes'] or digest(content) != sha:
            raise ValueError('Evidence hash mismatch: ' + item['path'])
    return value


def alive(pid):
    if not isinstance(pid, int) or pid < 1:
        return False
    if os.name == 'nt':
        # Query only: os.kill(pid, 0) is not a portable Windows liveness probe.
        result = subprocess.run(['tasklist', '/FI', f'PID eq {pid}', '/FO', 'CSV', '/NH'],
                                capture_output=True, text=True, check=True)
        return f'"{pid}"' in result.stdout
    try:
        os.kill(pid, 0)
        return True
    except ProcessLookupError:
        return False
    except PermissionError:
        return True


def state_write(path, state):
    # State is mutable only inside the registered scratch run.
    temp = path.with_name('.state-' + uuid.uuid4().hex)
    with temp.open('xb') as stream:
        stream.write(canonical(state))
    os.replace(temp, path)


def begin(root, name, owner=None):
    root = Path(root).resolve(strict=True)
    if not re.fullmatch('[a-z0-9][a-z0-9-]{0,63}', name):
        raise ValueError('Invalid run name')
    path = safe(root, f'{RUNS}/{name}-{uuid.uuid4().hex}')
    path.mkdir(parents=True)
    state_write(path / 'run.json', {'schema': 1, 'state': 'active', 'owner_pid': owner or os.getpid()})
    return path.relative_to(root).as_posix()


def finish(root, run, files):
    root = Path(root).resolve(strict=True)
    if not re.fullmatch(re.escape(RUNS) + r'/[a-z0-9-]+', run):
        raise ValueError('Unknown run')
    path = safe(root, run)
    state = json.loads(safe(root, run + '/run.json').read_bytes())
    if state.get('schema') != 1 or state.get('state') != 'awaiting_export':
        raise ValueError('Run is not finished')
    if alive(state.get('owner_pid')):
        raise ValueError('Run owner active')
    selected = []
    for name in files:
        source = safe(root, run + '/' + name)
        source.relative_to(path)
        if source.name == 'run.json':
            raise ValueError('Run state is not evidence')
        selected.append(source.relative_to(root).as_posix())
    manifest = export(root, selected, state['result'])
    state.update(state='sealed', bundle=manifest, owner_pid=None)
    state_write(path / 'run.json', state)
    return manifest


def durable_reference(root, relative):
    """Conservative guard: export never silently rewrites existing consumers."""
    directory='specs'
    location=safe(root,'specs-location.json')
    if location.exists():
        directory=json.loads(location.read_bytes())['directory']
    for folder in dict.fromkeys([directory,'.highgrade/specs','docs']):
        base=safe(root,folder)
        if not base.exists(): continue
        for path in base.rglob('*'):
            rel=path.relative_to(root).as_posix()
            # Object manifests retain origin labels, not live scratch references.
            if rel.startswith(BASE+'/') or path.suffix not in ('.json','.md','.txt','.log'):
                continue
            safe(root,rel)
            if not path.is_file():continue
            content=path.read_text(encoding='utf-8')
            if path.suffix=='.json':
                try:
                    content=json.dumps(json.loads(content),ensure_ascii=False)
                except json.JSONDecodeError:
                    if folder != 'docs': raise
                    # Historical diagnostic output may be empty/non-JSON.
                    # Inspect its literal links without treating it as a catalog.
            normalized=content.replace(chr(92),'/')
            while '//' in normalized: normalized=normalized.replace('//','/')
            if relative in normalized:
                return rel
    return None


def sweep(root, apply=False, blocked=None):
    """Only sealed registered runs; unknown/active/unexported runs always survive."""
    root = Path(root).resolve(strict=True)
    base = safe(root, RUNS)
    result = {'removed': [], 'eligible': [], 'retained': []}
    if not base.exists():
        return result
    for path in sorted(base.iterdir()):
        rel = path.relative_to(root).as_posix()
        try:
            safe(root, rel)
            state = json.loads(safe(root, rel + '/run.json').read_bytes())
            if state.get('schema') != 1:
                raise ValueError('unknown state')
            if alive(state.get('owner_pid')):
                raise ValueError('active owner')
            if state.get('state') != 'sealed':
                raise ValueError('unexported or interrupted run')
            if state.get('discard') is not True:
                verify(root, state['bundle'])
            reference = durable_reference(root, rel)
            if reference:
                raise ValueError('live reference: ' + reference)
            # Never erase files added or modified after sealing. Extras present at
            # sealing are explicitly disposable; capture the entire inventory below.
            inventory = tree_hashes(root, rel)
            if inventory != state.get('inventory'):
                raise ValueError('scratch changed since sealing')
            if blocked:
                raise ValueError(blocked)
            result['eligible'].append(rel)
            if apply:
                shutil.rmtree(path)
                result['removed'].append(rel)
        except (OSError, ValueError, KeyError, TypeError) as error:
            result['retained'].append({'path': rel, 'reason': str(error)})
    return result


def tree_hashes(root, relative):
    path = safe(root, relative)
    result = {}
    for entry in path.rglob('*'):
        rel = entry.relative_to(root).as_posix()
        safe(root, rel)
        if entry.is_dir():
            result[entry.relative_to(path).as_posix()] = {'type': 'directory'}
        elif entry.is_file() and entry != path / 'run.json':
            result[entry.relative_to(path).as_posix()] = {'type': 'file', 'sha256': digest(entry.read_bytes())}
        elif not entry.is_dir() and entry != path / 'run.json':
            raise ValueError('Unsupported scratch entry')
    return result


def seal(root, run, files):
    root = Path(root).resolve(strict=True)
    inventory = tree_hashes(root, run)
    manifest = finish(root, run, files)
    path = safe(root, run + '/run.json')
    state = json.loads(path.read_bytes())
    state['inventory'] = inventory
    state_write(path, state)
    return manifest


def discard(root, run):
    """Explicit review decision: this finished run has no evidence to retain."""
    root=Path(root).resolve(strict=True)
    if not re.fullmatch(re.escape(RUNS)+r'/[a-z0-9-]+',run):
        raise ValueError('Unknown run')
    path=safe(root,run+'/run.json')
    state=json.loads(path.read_bytes())
    if state.get('schema')!=1 or state.get('state')!='awaiting_export' or alive(state.get('owner_pid')):
        raise ValueError('Run is not finished or owner active')
    state.update(state='sealed',discard=True,owner_pid=None,inventory=tree_hashes(root,run))
    state_write(path,state)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[1])
    sub = parser.add_subparsers(dest='operation', required=True)
    p = sub.add_parser('export'); p.add_argument('--file', action='append', required=True); p.add_argument('--metadata', required=True)
    p = sub.add_parser('show'); p.add_argument('--manifest', required=True)
    p = sub.add_parser('finish'); p.add_argument('--run', required=True)
    choice=p.add_mutually_exclusive_group(required=True); choice.add_argument('--file', action='append'); choice.add_argument('--discard', action='store_true')
    p = sub.add_parser('run'); p.add_argument('--name', required=True); p.add_argument('command', nargs=argparse.REMAINDER)
    args = parser.parse_args(); root = args.root.resolve(strict=True)
    if args.operation == 'export':
        value = {'manifest': export(root, args.file, json.loads(safe(root, args.metadata).read_bytes()))}
    elif args.operation == 'show':
        manifest = verify(root, args.manifest)
        value = {'manifest': args.manifest, 'metadata': manifest['metadata'], 'files': len(manifest['files']), 'integrity': 'current'}
    elif args.operation == 'finish':
        if args.discard:
            discard(root,args.run); value={'discarded':args.run}
        else:
            value = {'manifest': seal(root, args.run, args.file)}
        # The regular maintenance entrypoint supplies process/recovery guards.
        subprocess.run([sys.executable, str(Path(__file__).with_name('target-maintenance.py')), '--root', str(root), '--sweep', '--apply'], check=True)
    else:
        command = args.command[1:] if args.command[:1] == ['--'] else args.command
        if not command: parser.error('command required')
        subprocess.run([sys.executable, str(Path(__file__).with_name('target-maintenance.py')), '--root', str(root), '--sweep', '--apply'], check=True)
        run = begin(root, args.name)
        state_path = safe(root, run + '/run.json')
        with safe(root, run + '/run.log').open('wb') as log:
            job = windows_job() if os.name == 'nt' else None
            process = subprocess.Popen(command, cwd=root, env={**os.environ, 'HIGHGRADE_RUN_DIR': str(root / run)}, stdout=log, stderr=subprocess.STDOUT, start_new_session=os.name != 'nt')
            process.wait()
            descendants = job() > 1 if job else group_alive(process.pid)
        state = json.loads(state_path.read_bytes())
        state.update(state='interrupted_descendants' if descendants else 'awaiting_export', result={'command': command, 'platform': platform.platform(), 'captured_at': datetime.now(timezone.utc).isoformat(), 'exit_code': process.returncode, 'outcome': 'unknown', 'observation': 'Process exit only; inspect selected source for scenario results'})
        state_write(state_path, state)
        value = {'run': run, 'exit_code': process.returncode, 'next': 'descendant processes remain; preserve scratch' if descendants else 'review outputs, then finish with explicitly selected files'}
    print(json.dumps(value, ensure_ascii=True))
    if args.operation == 'run': return process.returncode
    return 0


def group_alive(pid):
    try:
        os.killpg(pid, 0)
        return True
    except ProcessLookupError:
        return False
    except PermissionError:
        return True


def windows_job():
    """Contain wrapper and descendants; OS kills remaining children on wrapper exit.

    Assignment precedes child creation. If nesting is unavailable, refuse to run.
    Keep the job handle open until process exit, never close it from Python.
    """
    import ctypes as c
    from ctypes import wintypes as w
    class Basic(c.Structure):
        _fields_ = [('process_time', c.c_longlong), ('job_time', c.c_longlong),
                    ('flags', w.DWORD), ('minimum', c.c_size_t), ('maximum', c.c_size_t),
                    ('active_limit', w.DWORD), ('affinity', c.c_size_t),
                    ('priority', w.DWORD), ('scheduling', w.DWORD)]
    class IO(c.Structure):
        _fields_ = [(name, c.c_ulonglong) for name in ('read_ops','write_ops','other_ops','read_bytes','write_bytes','other_bytes')]
    class Limits(c.Structure):
        _fields_ = [('basic', Basic), ('io', IO), ('process_memory',c.c_size_t),
                    ('job_memory',c.c_size_t), ('peak_process',c.c_size_t), ('peak_job',c.c_size_t)]
    class Accounting(c.Structure):
        _fields_ = [(name,c.c_longlong) for name in ('user','kernel','period_user','period_kernel')] + [(name,w.DWORD) for name in ('faults','total','active','terminated')]
    k = c.WinDLL('kernel32', use_last_error=True)
    k.CreateJobObjectW.argtypes = [c.c_void_p,w.LPCWSTR]; k.CreateJobObjectW.restype = w.HANDLE
    k.SetInformationJobObject.argtypes = [w.HANDLE,c.c_int,c.c_void_p,w.DWORD]
    k.AssignProcessToJobObject.argtypes = [w.HANDLE,w.HANDLE]
    k.GetCurrentProcess.restype = w.HANDLE
    k.QueryInformationJobObject.argtypes = [w.HANDLE,c.c_int,c.c_void_p,w.DWORD,c.c_void_p]
    handle = k.CreateJobObjectW(None,None)
    limits=Limits(); limits.basic.flags=0x2000
    if not handle or not k.SetInformationJobObject(handle,9,c.byref(limits),c.sizeof(limits)) or not k.AssignProcessToJobObject(handle,k.GetCurrentProcess()):
        raise OSError(c.get_last_error(),'Cannot contain managed command in Windows Job')
    def active():
        info=Accounting()
        if not k.QueryInformationJobObject(handle,1,c.byref(info),c.sizeof(info),None):
            raise OSError(c.get_last_error(),'Cannot query managed command Job')
        return info.active
    return active


if __name__ == '__main__':
    try: sys.exit(main())
    except (OSError, ValueError, KeyError, TypeError) as error:
        sys.exit('Evidence storage refused: ' + str(error))
