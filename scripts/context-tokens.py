"""Measure selected instruction routes; this is not full-session billing."""
import argparse
import hashlib
import importlib.metadata
import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
TOOL = ROOT / 'target/highgrade/tools/tokenizer'
if TOOL.is_dir():
    sys.path.insert(0, str(TOOL))

ENTRY = ['README.md', 'AGENTS.md', '.highgrade/project/INSTRUCTIONS.md', 'kit/rules.md']
ROUTES = {
    'simple-edit': ENTRY,
    'work': ENTRY + ['kit/skills/highgrade-work/SKILL.md', 'docs/DEVELOPMENT.md', 'docs/ENGINEERING.md', 'kit/references/documentation.md', 'kit/references/tools/verification.md'],
    'planner': ENTRY + ['kit/skills/highgrade-planner/SKILL.md', 'kit/skills/highgrade-task/SKILL.md'],
    'update': ['kit/rules.md', 'kit/skills/highgrade-update/SKILL.md', 'kit/references/tools/installation.md', 'kit/references/tools/diagnostics.md'],
}


def measure(ref=None, source=ROOT):
    import tiktoken
    encoding = tiktoken.get_encoding('o200k_base')
    files = {}
    for relative in sorted({p for paths in ROUTES.values() for p in paths}):
        data = (subprocess.check_output(['git', 'show', f'{ref}:{relative}'], cwd=ROOT)
                if ref else (source / relative).read_bytes())
        files[relative] = {'sha256': hashlib.sha256(data).hexdigest(), 'bytes': len(data),
                           'tokens': len(encoding.encode(data.decode('utf-8-sig'), disallowed_special=()))}
    return {'tokenizer': {'package': 'tiktoken', 'version': importlib.metadata.version('tiktoken'),
                          'encoding': encoding.name}, 'revision': ref or 'working-tree',
            'scope': 'Sum of independently encoded file texts; no messages, tool output or hidden context.',
            'files': files, 'routes': {name: {'paths': paths, 'bytes': sum(files[p]['bytes'] for p in paths),
                                              'tokens': sum(files[p]['tokens'] for p in paths)}
                                      for name, paths in ROUTES.items()}}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--ref')
    parser.add_argument('--source', type=Path, default=ROOT,
                        help='Saved source tree for a reproducible intermediate measurement')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    if args.ref and args.source != ROOT:
        parser.error('--ref and --source cannot be combined')
    result = measure(args.ref, args.source)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    print(json.dumps({name: value['tokens'] for name, value in result['routes'].items()}))


if __name__ == '__main__':
    main()
