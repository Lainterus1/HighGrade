"""Record the bounded synthetic invalid-input test without changing project data."""

import difflib
import hashlib
import json
import re
import sys
from datetime import datetime, timezone
from pathlib import Path


OUT = Path(__file__).resolve().parent
ROOT = OUT.parents[1]
SETUP = json.loads((OUT / "setup.json").read_text(encoding="utf-8"))
CHANGED = {"test_store.py", "docs/ENGINEERING.md", "docs/DEVELOPMENT.md"}


def sha(data):
    return hashlib.sha256(data).hexdigest()


before = {}
after = {}
diffs = []
for name, expected in SETUP["before_sha256"].items():
    old = (OUT / "before" / name).read_bytes()
    new = (ROOT / name).read_bytes()
    before[name] = sha(old)
    after[name] = sha(new)
    if before[name] != expected:
        raise RuntimeError(f"Before snapshot mismatch: {name}")
    if name not in CHANGED and after[name] != expected:
        raise RuntimeError(f"Unexpected project change: {name}")
    if old != new:
        diffs.extend(difflib.unified_diff(
            old.decode("utf-8").splitlines(keepends=True),
            new.decode("utf-8").splitlines(keepends=True),
            fromfile=f"before/{name}", tofile=f"after/{name}",
        ))

if {name for name in before if before[name] != after[name]} != CHANGED:
    raise RuntimeError("Changed file set differs from the test and its documentation")

targeted = (OUT / "targeted-unittest.txt").read_bytes()
full = (OUT / "full-unittest.txt").read_bytes()
targeted_text = targeted.decode("utf-8", errors="replace")
full_text = full.decode("utf-8", errors="replace")
for report, count in ((targeted_text, 1), (full_text, 4)):
    if f"Ran {count} test" not in report or not re.search(r"(?m)^OK\s*$", report):
        raise RuntimeError(f"Native report does not show {count} successful tests")
    if not re.search(r"(?s)test_invalid_data_preserves_existing_file.*?HG-0010-S6.*?\.\.\. ok", report):
        raise RuntimeError("The invalid-input test is not shown as executed")

(OUT / "changes.diff").write_text("".join(diffs), encoding="utf-8", newline="\n")
manifest = {
    "captured_at": datetime.now(timezone.utc).isoformat(),
    "synthetic_root": str(ROOT),
    "before_sha256": before,
    "after_sha256": after,
    "changed_paths": sorted(CHANGED),
    "store_py_unchanged": before["store.py"] == after["store.py"],
    "additional_dependency_sha256": {
        "notes/mechanism.txt": sha((ROOT / "notes/mechanism.txt").read_bytes()),
    },
    "python_executable": sys.executable,
    "python_version": sys.version,
    "native_checks": [
        {
            "command": "python -B -m unittest -v test_store.StoreTests.test_invalid_data_preserves_existing_file",
            "observed_exit_code": 0,
            "report": "evidence/hg10-s6-invalid-input/targeted-unittest.txt",
            "report_sha256": sha(targeted),
        },
        {
            "command": "python -B -m unittest -v",
            "observed_exit_code": 0,
            "report": "evidence/hg10-s6-invalid-input/full-unittest.txt",
            "report_sha256": sha(full),
        },
    ],
    "formal_pass_assigned": False,
    "commit_or_publish": False,
}
(OUT / "sha-manifest.json").write_text(
    json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n",
)
print(json.dumps({"changed_paths": sorted(CHANGED), "store_unchanged": True,
                  "targeted_tests": 1, "full_tests": 4}))
