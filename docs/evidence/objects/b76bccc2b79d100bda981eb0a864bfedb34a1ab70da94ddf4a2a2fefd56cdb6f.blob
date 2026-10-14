"""Internal process entrypoint; use audit.py for public CLI."""
import json
from pathlib import Path
import sys
from code_health import adapters

request = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
name = request["name"]
root = Path(request["root"])
common = dict(root=root, files=request["files"], ast_index=request["ast_index"], policy=request["policy"], expected_version=request["expected_version"])
extra = request["extra"]
if name == "coverage":
    for key in ("artifact", "provenance"):
        extra[key] = Path(extra[key]) if extra.get(key) else None
result = getattr(adapters, "run_" + name)(**common, **extra)
Path(sys.argv[2]).write_text(json.dumps(result, ensure_ascii=False), encoding="utf-8")
