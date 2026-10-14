"""Inspect or prune the project-owned scratch area and selected Cargo caches."""

import argparse
import json
import os
from pathlib import Path
import shutil
import stat
import subprocess


BUDGET_MIB = 600
ROOT_ENTRIES = {'debug', 'release', 'nextest', 'tmp', 'highgrade', 'CACHEDIR.TAG', '.rustc_info.json'}
PROJECT_ENTRIES = {'build-cache', 'candidate', 'candidate.previous', 'reports', 'tools', 'work', 'tmp', 'runs', 'ui-playwright'}


def linked(path):
    info = path.lstat()
    return stat.S_ISLNK(info.st_mode) or bool(getattr(info, 'st_file_attributes', 0) & 0x400)


def inventory(root):
    """Measure payload once per inode; expose pathname and allocated sizes too.

    ``mib`` is the portable, hardlink-deduplicated payload used by the budget.
    ``logical_mib`` counts every pathname. ``physical_mib`` includes directory
    allocation when the platform exposes st_blocks, and is otherwise unknown.
    Areas are measured independently: a hardlink spanning two areas contributes
    to both area subtotals, but only once to the overall total.
    """
    def usage():
        return {'unique': 0, 'logical': 0, 'physical': 0, 'physical_known': True, 'seen': set()}

    def account(measurement, path, info, is_file):
        if is_file:
            measurement['logical'] += info.st_size
        # A zero/unavailable inode must not collapse unrelated files.
        identity = (info.st_dev, info.st_ino) if info.st_ino else path
        if identity in measurement['seen']:
            return
        measurement['seen'].add(identity)
        if is_file:
            measurement['unique'] += info.st_size
        blocks = getattr(info, 'st_blocks', None)
        if blocks is None:
            measurement['physical_known'] = False
        else:
            measurement['physical'] += blocks * 512

    def summarize(measurement):
        return {
            'mib': round(measurement['unique'] / 1024 / 1024, 2),
            'logical_mib': round(measurement['logical'] / 1024 / 1024, 2),
            'physical_mib': round(measurement['physical'] / 1024 / 1024, 2)
            if measurement['physical_known'] else None,
            'hardlink_duplicates_mib': round(
                (measurement['logical'] - measurement['unique']) / 1024 / 1024, 2),
        }

    target = root / 'target'
    total = usage()
    if not target.exists() and not target.is_symlink():
        return {**summarize(total), 'areas': {}, 'unknown_root': [],
                'unknown_highgrade': [], 'links': [], 'scratch': []}
    if linked(target) or not target.is_dir():
        raise ValueError('target is linked or is not a directory')
    account(total, target, target.stat(), False)
    areas = {}
    links = []
    pending = [target]
    while pending:
        directory = pending.pop()
        with os.scandir(directory) as entries:
            for entry in entries:
                path = Path(entry.path)
                if linked(path):
                    links.append(path.relative_to(root).as_posix())
                    continue
                relative = path.relative_to(target).parts
                area = '/'.join(relative[:2] if relative[0] == 'highgrade' else relative[:1])
                measurement = areas.setdefault(area, usage())
                # DirEntry.stat reports zero inode/device identities on Windows;
                # a fresh path stat supplies the actual hardlink identity.
                info = path.stat(follow_symlinks=False)
                if entry.is_dir(follow_symlinks=False):
                    account(total, path, info, False)
                    account(measurement, path, info, False)
                    pending.append(path)
                elif entry.is_file(follow_symlinks=False):
                    account(total, path, info, True)
                    account(measurement, path, info, True)
                else:
                    links.append(path.relative_to(root).as_posix())
    project = target / 'highgrade'
    if project.exists() and (linked(project) or not project.is_dir()):
        links.append('target/highgrade')
    scratch = project / 'tmp'
    if scratch.exists() and (linked(scratch) or not scratch.is_dir()):
        links.append('target/highgrade/tmp')
    return {
        **summarize(total),
        'areas': {name: summarize(value) for name, value in sorted(areas.items())},
        'unknown_root': sorted(p.name for p in target.iterdir() if p.name not in ROOT_ENTRIES),
        'unknown_highgrade': sorted(p.name for p in project.iterdir() if p.name not in PROJECT_ENTRIES)
        if project.is_dir() and not linked(project) else [],
        'links': sorted(set(links)),
        'scratch': sorted(p.name for p in scratch.iterdir())
        if scratch.is_dir() and not linked(scratch) else [],
    }


def active_builds():
    if os.name == 'nt':
        command = ['powershell', '-NoProfile', '-Command',
                   "$ErrorActionPreference='Stop'; Get-Process | "
                   "Where-Object { $_.ProcessName -in @('cargo','rustc','cargo-nextest','highgrade','node') } | "
                   'Select-Object -ExpandProperty ProcessName']
    else:
        command = ['ps', '-eo', 'comm=']
    result = subprocess.run(command, capture_output=True, text=True, check=True)
    names = {line.strip().lower().removesuffix('.exe') for line in result.stdout.splitlines()}
    return sorted(names & {'cargo', 'rustc', 'cargo-nextest', 'highgrade', 'node'})


def maintain(root, apply=False, caches=(), scratch=(), reports=()):
    root = Path(root).resolve(strict=True)
    before = inventory(root)
    selected = tuple(dict.fromkeys(scratch))
    caches = tuple(dict.fromkeys(caches))
    reports = tuple(dict.fromkeys(reports))
    if any(name not in ('debug', 'release', 'build-cache') for name in caches):
        raise ValueError('Unknown cache selection')
    for name in selected:
        if name in ('.', '..') or '/' in name or '\\' in name or ':' in name or name not in before['scratch']:
            raise ValueError(f'Unknown or unsafe scratch selection: {name}')
    report_dir = root / 'target/highgrade/reports'
    for name in reports:
        if (name in ('', '.', '..') or any(c in name for c in '/\\:')
                or not (report_dir / name).is_file()):
            raise ValueError(f'Unknown or unsafe report selection: {name}')
    commands = []
    for cache in caches:
        directory = root / ('target/highgrade/build-cache' if cache == 'build-cache' else 'target')
        directory.resolve().relative_to(root)
        for path in (directory, directory / ('debug' if cache == 'debug' else 'release')):
            if path.exists() and not path.is_dir():
                raise ValueError(f'Cache destination is not a directory: {path}')
        commands.append(['cargo', 'clean', '--target-dir', str(directory),
                         *(['--profile', 'test'] if cache == 'debug' else ['--release'])])
    report = {'before': before, 'budget_mib': BUDGET_MIB,
              'planned': [f'target/highgrade/tmp/{name}' for name in selected]
              + [f'target/highgrade/reports/{name}' for name in reports],
              'cargo_commands': commands}
    if not apply:
        report['status'] = 'preview'
        return report
    if before['links']:
        raise ValueError(f'Linked or unsupported paths; nothing removed: {before["links"]}')
    running = active_builds()
    if running:
        raise ValueError(f'Build processes active; nothing removed: {running}')
    project = root / 'target' / 'highgrade'
    work = project / 'work'
    if (project / 'candidate.previous').exists() or (work.is_dir() and any(work.iterdir())):
        raise ValueError('Candidate recovery or build work exists; nothing removed')
    scratch = project / 'tmp'
    # Resolve every destination before any mutation; inventory already refused links.
    for child in [*(scratch / name for name in selected), *(report_dir / name for name in reports)]:
        child.resolve(strict=True).relative_to(root / 'target')
    if scratch.is_dir():
        for name in selected:
            child = scratch / name
            if linked(child):
                raise ValueError(f'Linked scratch path; nothing removed: {child}')
        for name in selected:
            child = scratch / name
            if child.is_dir():
                shutil.rmtree(child)
            else:
                child.unlink()
    for name in reports:
        (report_dir / name).unlink()
    for command in commands:
        subprocess.run(command, cwd=root, check=True)
    report['after'] = inventory(root)
    report['status'] = 'applied'
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parent.parent)
    parser.add_argument('--apply', action='store_true', help='remove only listed scratch and selected caches')
    parser.add_argument('--scratch', action='append', default=[], metavar='NAME',
                        help='select one direct child of target/highgrade/tmp for removal')
    parser.add_argument('--cache', choices=('debug', 'release', 'build-cache'), action='append', default=[])
    parser.add_argument('--report', action='append', default=[], metavar='NAME',
                        help='select one direct file of target/highgrade/reports')
    parser.add_argument('--sweep', action='store_true', help='collect only sealed managed runs; preserve active or unexported runs')
    parser.add_argument('--check', action='store_true', help='exit 2 on unknown paths or budget excess')
    args = parser.parse_args()
    try:
        if args.sweep:
            if args.cache or args.scratch or args.report:
                raise ValueError('Sweep cannot be mixed with explicit selections')
            from evidence_store import sweep
            root = args.root.resolve(strict=True)
            before = inventory(root)
            running = active_builds() if args.apply else []
            recovery = (root / 'target/highgrade/candidate.previous').exists() or (root / 'target/highgrade/work').is_dir() and any((root / 'target/highgrade/work').iterdir())
            blocked = 'active build/UI processes: ' + ','.join(running) if running else 'candidate recovery/work exists' if recovery else 'linked paths exist' if before['links'] else None
            report = {'before': before, 'collection': sweep(root, args.apply, blocked), 'after': inventory(root), 'status': 'applied' if args.apply else 'preview'}
        else:
            report = maintain(args.root, args.apply, tuple(args.cache), tuple(args.scratch), tuple(args.report))
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        parser.exit(1, f'Target maintenance refused: {error}\n')
    if args.sweep:
        collection = report['collection']
        print(json.dumps({'status': report['status'], 'before_mib': report['before']['mib'],
                          'after_mib': report['after']['mib'],
                          **{key + '_count': len(value) for key, value in collection.items()},
                          **{key: value[:10] for key, value in collection.items()},
                          'detail': 'first 10 per group; plain invocation shows inventory'}, ensure_ascii=True))
    else:
        print(json.dumps(report, ensure_ascii=False, indent=2))
    state = report.get('after', report['before'])
    if args.check and (state['unknown_root'] or state['unknown_highgrade'] or state['links'] or state['scratch']
                       or state['mib'] > BUDGET_MIB):
        raise SystemExit(2)


if __name__ == '__main__':
    main()
