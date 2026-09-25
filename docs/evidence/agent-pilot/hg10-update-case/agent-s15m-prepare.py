"""Prepare a valid synthetic same-ID package without changing the exact source."""

import hashlib
import json
import os
import shutil
from datetime import datetime, timezone
from pathlib import Path

BASE = Path(r"D:\my_projects\MyCodex\target\update-pilot")
CASE = BASE / "case"
PROFILE = BASE / "profile"
SOURCE = BASE / "source-v0-2-14" / "kit"
DIFFERENT = BASE / "same-id-different-kit-v0-2-14"
CANDIDATE_CLI = BASE / "highgrade-v0-2-14-exact.exe"
INSTALLED_CLI = PROFILE / ".highgrade" / "global" / "releases" / "v0-2-14" / "highgrade.exe"


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def save_json(path: Path, value: object) -> None:
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def snapshot(path: Path) -> dict:
    files = {}
    for branch in (path / ".agents" / "skills", path / ".highgrade" / "global"):
        for root, directories, names in os.walk(branch, followlinks=False):
            directories[:] = [name for name in directories if not (Path(root) / name).is_symlink()]
            for name in names:
                file = Path(root) / name
                if file.is_file() and not file.is_symlink():
                    files[file.relative_to(path).as_posix()] = {"bytes": file.stat().st_size, "sha256": sha(file)}
    return {
        "captured_at_utc": datetime.now(timezone.utc).isoformat(),
        "profile": str(path),
        "active": json.loads((path / ".highgrade" / "global" / "active.json").read_text(encoding="utf-8")),
        "files": dict(sorted(files.items())),
    }


status = json.loads((CASE / "agent-s15m-status-before.json").read_text(encoding="utf-8"))
assert status["status"] == "passed" and status["measurements"][0]["release"] == "v0-2-14"
assert not DIFFERENT.exists(), "Refusing to overwrite an existing synthetic candidate"
before = snapshot(PROFILE)
save_json(CASE / "agent-s15m-profile-before.json", before)
source_manifest_before = sha(SOURCE / "manifest.json")
source_rules_before = sha(SOURCE / "rules.md")
shutil.copytree(SOURCE, DIFFERENT)
rules = DIFFERENT / "rules.md"
rules.write_bytes(rules.read_bytes() + b"\n<!-- synthetic same-release composition probe; no rule change -->\n")
manifest_path = DIFFERENT / "manifest.json"
manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
manifest["files"]["rules.md"] = sha(rules)
save_json(manifest_path, manifest)
files = {relative: {"expected": expected, "actual": sha(DIFFERENT / relative)} for relative, expected in manifest["files"].items()}
for item in files.values():
    item["match"] = item["expected"] == item["actual"]
result = {
    "captured_at_utc": datetime.now(timezone.utc).isoformat(),
    "synthetic_candidate": str(DIFFERENT),
    "source_kit": str(SOURCE),
    "synthetic_change": "Appended an inert HTML comment to rules.md and updated its manifest entry; release ID and CLI version remain v0-2-14/0.2.14.",
    "release": manifest["release"],
    "cli_version": manifest["cli_version"],
    "source_manifest_sha256_before": source_manifest_before,
    "source_manifest_sha256_after": sha(SOURCE / "manifest.json"),
    "source_rules_sha256_before": source_rules_before,
    "source_rules_sha256_after": sha(SOURCE / "rules.md"),
    "synthetic_manifest_sha256": sha(manifest_path),
    "synthetic_rules_sha256": sha(rules),
    "candidate_cli_sha256": sha(CANDIDATE_CLI),
    "installed_cli_sha256": sha(INSTALLED_CLI),
    "all_manifest_files_match": all(item["match"] for item in files.values()),
    "files": files,
}
assert result["source_manifest_sha256_before"] == result["source_manifest_sha256_after"]
assert result["source_rules_sha256_before"] == result["source_rules_sha256_after"]
assert result["all_manifest_files_match"]
assert result["synthetic_manifest_sha256"] != result["source_manifest_sha256_after"]
save_json(CASE / "agent-s15m-package.json", result)
print(json.dumps({key: result[key] for key in ("release", "cli_version", "source_manifest_sha256_after", "synthetic_manifest_sha256", "synthetic_rules_sha256", "candidate_cli_sha256", "installed_cli_sha256", "all_manifest_files_match")}, ensure_ascii=False, indent=2))
