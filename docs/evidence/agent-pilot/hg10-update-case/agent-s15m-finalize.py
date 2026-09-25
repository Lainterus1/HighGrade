"""Verify the same-ID mismatch experiment did not modify the profile."""

import hashlib
import json
import os
from datetime import datetime, timezone
from pathlib import Path

BASE = Path(r"D:\my_projects\MyCodex\target\update-pilot")
CASE = BASE / "case"
PROFILE = BASE / "profile"
SOURCE = BASE / "source-v0-2-14" / "kit"
DIFFERENT = BASE / "same-id-different-kit-v0-2-14"


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read(name: str) -> dict:
    return json.loads((CASE / name).read_text(encoding="utf-8"))


def save(name: str, value: dict) -> None:
    (CASE / name).write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


files = {}
for branch in (PROFILE / ".agents" / "skills", PROFILE / ".highgrade" / "global"):
    for root, directories, names in os.walk(branch, followlinks=False):
        directories[:] = [name for name in directories if not (Path(root) / name).is_symlink()]
        for name in names:
            path = Path(root) / name
            if path.is_file() and not path.is_symlink():
                files[path.relative_to(PROFILE).as_posix()] = {"bytes": path.stat().st_size, "sha256": sha(path)}
after = {
    "captured_at_utc": datetime.now(timezone.utc).isoformat(),
    "profile": str(PROFILE),
    "active": json.loads((PROFILE / ".highgrade" / "global" / "active.json").read_text(encoding="utf-8")),
    "files": dict(sorted(files.items())),
}
save("agent-s15m-profile-after.json", after)
before = read("agent-s15m-profile-before.json")
decision = read("agent-s15m-decision.json")
package = read("agent-s15m-package.json")
commands = read("agent-s15m-commands.json")
manifest = json.loads((DIFFERENT / "manifest.json").read_text(encoding="utf-8"))
all_package_files_match = all(sha(DIFFERENT / path) == expected for path, expected in manifest["files"].items())
report = {
    "captured_at_utc": datetime.now(timezone.utc).isoformat(),
    "profile_files_before": len(before["files"]),
    "profile_files_after": len(after["files"]),
    "profile_files_identical": before["files"] == after["files"],
    "active_before": before["active"],
    "active_after": after["active"],
    "source_manifest_unchanged": sha(SOURCE / "manifest.json") == package["source_manifest_sha256_before"],
    "source_rules_unchanged": sha(SOURCE / "rules.md") == package["source_rules_sha256_before"],
    "synthetic_package_valid": all_package_files_match and sha(DIFFERENT / "manifest.json") == package["synthetic_manifest_sha256"],
    "same_release_error": decision["preview_error"] == "GlobalSameRelease",
    "active_status_passed": decision["active_status"] == "passed" and decision["active_release"] == "v0-2-14",
    "manifest_mismatch": not decision["manifest_match"],
    "cli_match": decision["cli_match"],
    "apply_invoked": decision["apply_invoked"] or any("--apply" in item["arguments"] for item in commands),
}
assert report["profile_files_identical"]
assert report["source_manifest_unchanged"] and report["source_rules_unchanged"]
assert report["synthetic_package_valid"]
assert report["same_release_error"] and report["active_status_passed"]
assert report["manifest_mismatch"] and report["cli_match"] and not report["apply_invoked"]
save("agent-s15m-final-verification.json", report)
print(json.dumps(report, ensure_ascii=False, indent=2))
