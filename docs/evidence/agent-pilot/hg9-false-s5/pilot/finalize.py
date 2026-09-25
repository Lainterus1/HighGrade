from __future__ import annotations

import difflib
import hashlib
import json
import re
from pathlib import Path

root = Path(__file__).resolve().parents[2]
evidence = Path(__file__).resolve().parent
before = evidence / "before"


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


files = sorted(
    p for p in root.rglob("*")
    if p.is_file() and (root / "evidence") not in p.parents and ".git" not in p.parts
)
rows = []
diff = []
for path in files:
    rel = path.relative_to(root)
    prior = before / rel
    row = {"path": rel.as_posix(), "after_sha256": sha(path)}
    if prior.is_file():
        row["before_sha256"] = sha(prior)
        row["unchanged"] = row["before_sha256"] == row["after_sha256"]
    else:
        row["before_sha256"] = None
        row["unchanged"] = False
    rows.append(row)
    if row["unchanged"]:
        continue
    old = prior.read_text(encoding="utf-8").splitlines(keepends=True) if prior.is_file() else []
    new = path.read_text(encoding="utf-8").splitlines(keepends=True)
    diff.extend(difflib.unified_diff(old, new, fromfile=f"before/{rel.as_posix()}", tofile=f"after/{rel.as_posix()}"))

(evidence / "after.json").write_text(json.dumps(rows, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
(evidence / "changes.diff").write_text("".join(diff), encoding="utf-8")

link_rows = []
for path in files:
    if path.suffix.lower() != ".md" or "legacy" in path.parts:
        continue
    text = path.read_text(encoding="utf-8")
    for target in re.findall(r"(?<!!)\[[^]]+\]\(([^)]+)\)", text):
        target_path = target.split("#", 1)[0]
        if not target_path or "://" in target_path:
            continue
        destination = (path.parent / target_path).resolve()
        link_rows.append({
            "source": path.relative_to(root).as_posix(),
            "target": target,
            "inside_project": destination.is_relative_to(root),
            "exists": destination.is_file(),
        })
(evidence / "links.json").write_text(json.dumps(link_rows, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
print(json.dumps({"files": len(rows), "changed": sum(not r["unchanged"] for r in rows), "links": len(link_rows), "broken_links": [r for r in link_rows if not r["inside_project"] or not r["exists"]]}, ensure_ascii=False))
