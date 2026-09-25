"""Verify and hash the bounded update pilot evidence."""

import hashlib
import json
import re
from datetime import datetime, timezone
from pathlib import Path

BASE = Path(r"D:\my_projects\MyCodex\target\update-pilot")
CASE = BASE / "case"


def read(name: str) -> dict:
    return json.loads((CASE / name).read_text(encoding="utf-8"))


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


setup = read("setup.json")
preflight = read("agent-preflight.json")
preview = read("agent-good-preview.json")
apply = read("agent-good-apply.json")
good_status = read("agent-good-status-after.json")
fail_preview = read("agent-fail-preview.json")
fail_status = read("agent-fail-status-after.json")
repeat_preview = read("agent-repeat-preview.json")
repeat_match = read("agent-repeat-comparison.json")
snapshot = read("agent-snapshot-diff.json")
fingerprint = preview["measurements"][0]["candidate_sha256"]
apply_record = read("agent-good-apply-command.json")
repeat_record = read("agent-repeat-command.json")
report = {
    "captured_at_utc": datetime.now(timezone.utc).isoformat(),
    "scope": str(BASE),
    "source_sha_from_setup": setup["git_commits"]["new"],
    "package_bytes_verified": preflight["all_manifest_files_match"] and preflight["manifest_sha256_expected"] == preflight["manifest_sha256_actual"] and preflight["candidate_cli"]["match"] and preflight["all_stable_router_skills_match"],
    "fail_error": fail_preview["findings"][0]["message"],
    "fail_old_status_confirmed": fail_status["status"] == "passed" and fail_status["measurements"][0]["release"] == "v0-2-13",
    "fail_files_identical": snapshot["fail_before_to_after"]["identical_files"],
    "preview_fingerprint": fingerprint,
    "apply_fingerprint": apply["measurements"][0]["candidate_sha256"],
    "same_preview_apply_fingerprint": fingerprint == apply["measurements"][0]["candidate_sha256"] and fingerprint == apply_record["arguments"][-1],
    "good_status_confirmed": apply["status"] == "passed" and good_status["status"] == "passed" and good_status["measurements"][0]["release"] == "v0-2-14",
    "repeat_error": repeat_preview["findings"][0]["message"],
    "repeat_manifest_and_cli_match": repeat_match["manifest_match"] and repeat_match["cli_match"],
    "repeat_apply_invoked": repeat_match["apply_invoked"],
    "repeat_files_identical": snapshot["good_after_to_repeat_after"]["identical_files"],
    "good_removed_count": len(snapshot["good_before_to_after"]["removed"]),
    "good_added_count": len(snapshot["good_before_to_after"]["added"]),
}
assert report["package_bytes_verified"]
assert report["fail_error"] == "GlobalManifestHashMismatch: rules.md"
assert report["fail_old_status_confirmed"] and report["fail_files_identical"]
assert report["same_preview_apply_fingerprint"] and report["good_status_confirmed"]
assert report["repeat_error"] == "GlobalSameRelease"
assert report["repeat_manifest_and_cli_match"] and not report["repeat_apply_invoked"] and report["repeat_files_identical"]
assert "--apply" not in repeat_record["arguments"]
(CASE / "agent-final-verification.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")

links = []
for match in re.finditer(r"\[[^\]]+\]\(([^)]+)\)", (CASE / "agent-journal.md").read_text(encoding="utf-8")):
    target = match.group(1)
    links.append({"path": target, "exists": (CASE / target).is_file()})
assert all(item["exists"] for item in links)
evidence = {path.name: sha(path) for path in CASE.iterdir() if path.is_file() and path.name.startswith("agent-") and path.name not in {"agent-evidence-sha256.json", "agent-finalize.py"}}
(CASE / "agent-evidence-sha256.json").write_text(json.dumps({"captured_at_utc": datetime.now(timezone.utc).isoformat(), "links_checked": len(links), "all_links_exist": True, "files": dict(sorted(evidence.items()))}, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
print(json.dumps(report, ensure_ascii=False, indent=2))
