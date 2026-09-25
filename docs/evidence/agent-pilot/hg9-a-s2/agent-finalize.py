"""Create bounded evidence for the synthetic s2 init pilot."""

import difflib
import hashlib
import json
import re
from datetime import datetime, timezone
from pathlib import Path
from urllib.parse import unquote

ROOT = Path(r"D:\my_projects\MyCodex\target\agent-pilot\s2")
EVIDENCE = ROOT / "evidence" / "hg9-a"
CLI = Path(r"D:\my_projects\MyCodex\target\agent-pilot\s7\.pilot-profile\.highgrade\global\releases\v0-2-14\highgrade.exe")

SOURCE_PATHS = [
    "AGENTS.md", "README.md", "PLAN.md", "discount.py", "quote.py", "receipt.py",
    "order_model.py", "order_loader.py", "order_report.py", "orders.json",
    "test_totals.py", "test_order_report.py", "test-results.txt",
    "docs/project-guide.md", "docs/test-strategy.md", "docs/ARCHITECTURE.md",
    "docs/ENGINEERING.md", "docs/DEVELOPMENT.md",
    ".highgrade/project/INSTRUCTIONS.md", ".highgrade/project/documents.json",
    "specs/README.md", "specs/catalog.json", "specs/runners.json", "specs/tags.json",
]
MARKDOWN_PATHS = [
    "AGENTS.md", "README.md", "PLAN.md", "docs/project-guide.md", "docs/test-strategy.md",
    "docs/ARCHITECTURE.md", "docs/ENGINEERING.md", "docs/DEVELOPMENT.md",
    ".highgrade/project/INSTRUCTIONS.md", "specs/README.md",
]
UNCHANGED_PATHS = [
    "PLAN.md", "discount.py", "quote.py", "receipt.py", "order_model.py",
    "order_loader.py", "order_report.py", "orders.json", "test_totals.py",
    "test_order_report.py", "test-results.txt",
]


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def save_json(path: Path, value: object) -> None:
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


links = []
for relative in MARKDOWN_PATHS:
    source = ROOT / relative
    for line_no, line in enumerate(source.read_text(encoding="utf-8").splitlines(), 1):
        for match in re.finditer(r"(?<!!)\[[^\]]+\]\(([^)]+)\)", line):
            target_text = match.group(1)
            if "://" in target_text or target_text.startswith("#"):
                continue
            path_part, _, anchor = target_text.partition("#")
            target = (source.parent / unquote(path_part)).resolve()
            inside = target == ROOT or ROOT in target.parents
            links.append({
                "from": relative, "line": line_no, "target": target_text,
                "inside_s2": inside, "target_exists": inside and target.is_file(),
                "anchor": anchor or None,
            })
save_json(EVIDENCE / "agent-links.json", {
    "checked_at_utc": datetime.now(timezone.utc).isoformat(),
    "link_occurrences": len(links),
    "all_targets_exist": all(item["target_exists"] for item in links),
    "links": links,
    "limitation": "Проверено существование локальных целевых файлов; якоря не интерпретировались.",
})

diff = []
for relative in SOURCE_PATHS:
    before_path = EVIDENCE / "before" / relative
    after_path = ROOT / relative
    before = before_path.read_text(encoding="utf-8").splitlines(keepends=True) if before_path.exists() else []
    after = after_path.read_text(encoding="utf-8").splitlines(keepends=True)
    if before != after:
        diff.extend(difflib.unified_diff(before, after, fromfile="before/" + relative, tofile="after/" + relative))
(EVIDENCE / "agent-final.diff").write_text("".join(diff), encoding="utf-8")

setup = json.loads((EVIDENCE / "setup.json").read_text(encoding="utf-8"))
after_hashes = {relative: sha(ROOT / relative) for relative in SOURCE_PATHS}
evidence_files = [
    "agent-audit-plan.md", "agent-bootstrap.json", "agent-check-commands.json",
    "agent-check-commands.ps1", "agent-doctor-after.json", "agent-doctor-before.json",
    "agent-final.diff", "agent-final-cli.json", "agent-final-cli.ps1",
    "agent-doctor-final.json", "agent-inspect-final.json",
    "agent-global-status.json", "agent-initial-commands.json",
    "agent-initial-commands.ps1", "agent-input-verification.json", "agent-inspect-after.json",
    "agent-inventory.json", "agent-links.json", "agent-spec-init.json", "agent-tests.json",
    "agent-tests.txt", "agent-finalize.py", "agent-journal.md",
]
manifest = {
    "schema_version": 1,
    "captured_at_utc": datetime.now(timezone.utc).isoformat(),
    "synthetic_root": str(ROOT),
    "installed_cli_sha256": sha(CLI),
    "before_sha256": setup["before_sha256"],
    "after_sha256": after_hashes,
    "unchanged_paths": UNCHANGED_PATHS,
    "all_unchanged_paths_match_input": all(after_hashes[path] == setup["before_sha256"][path] for path in UNCHANGED_PATHS),
    "evidence_sha256": {name: sha(EVIDENCE / name) for name in evidence_files},
}
save_json(EVIDENCE / "agent-sha-manifest.json", manifest)

doctor = json.loads((EVIDENCE / "agent-doctor-final.json").read_text(encoding="utf-8"))
inspect = json.loads((EVIDENCE / "agent-inspect-final.json").read_text(encoding="utf-8"))
tests = json.loads((EVIDENCE / "agent-tests.json").read_text(encoding="utf-8"))
verification = {
    "source_hashes_match_manifest": all(sha(ROOT / path) == expected for path, expected in after_hashes.items()),
    "all_unchanged_paths_match_input": manifest["all_unchanged_paths_match_input"],
    "all_local_link_targets_exist": all(item["target_exists"] for item in links),
    "doctor_status": doctor["status"],
    "doctor_semantic_readiness": next((m.get("semantic_readiness") for m in doctor["measurements"] if "semantic_readiness" in m), None),
    "inspect_status": inspect["status"],
    "inspect_findings": [{"code": finding["code"], "location": finding["location"]} for finding in inspect["findings"]],
    "required_links": next((m["required_links"] for m in inspect["measurements"] if "required_links" in m), []),
    "test_exit_code": tests["exit_code"],
}
save_json(EVIDENCE / "agent-final-verification.json", verification)
print(json.dumps(verification, ensure_ascii=False, indent=2))
