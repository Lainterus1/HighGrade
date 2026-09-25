"""Verify the bounded synthetic update pilot package and profile snapshots."""

import difflib
import hashlib
import json
import os
import sys
from datetime import datetime, timezone
from pathlib import Path

BASE = Path(r"D:\my_projects\MyCodex\target\update-pilot")
CASE = BASE / "case"
SETUP = json.loads((CASE / "setup.json").read_text(encoding="utf-8"))


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write_json(path: Path, value: object) -> None:
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def snapshot(profile: Path, label: str) -> dict:
    files = {}
    for branch in (profile / ".agents" / "skills", profile / ".highgrade" / "global"):
        for root, directories, names in os.walk(branch, followlinks=False):
            directories[:] = [name for name in directories if not (Path(root) / name).is_symlink()]
            for name in names:
                path = Path(root) / name
                if path.is_symlink() or not path.is_file():
                    continue
                relative = path.relative_to(profile).as_posix()
                files[relative] = {"bytes": path.stat().st_size, "sha256": sha(path)}
    active_path = profile / ".highgrade" / "global" / "active.json"
    active = json.loads(active_path.read_text(encoding="utf-8")) if active_path.exists() else None
    report = {
        "captured_at_utc": datetime.now(timezone.utc).isoformat(),
        "profile": str(profile),
        "active": active,
        "files": dict(sorted(files.items())),
    }
    write_json(CASE / f"agent-{label}-snapshot.json", report)
    return report


if __name__ == "__main__":
    mode = sys.argv[1]
    if mode == "preflight":
        candidate_kit = Path(SETUP["paths"]["candidate_kit"])
        manifest_path = candidate_kit / "manifest.json"
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        package = {
            "source_sha_from_setup": SETUP["git_commits"]["new"],
            "git_archive_manifest_verified_by_setup": SETUP["git_archive_manifest_verified"],
            "manifest_sha256_expected": SETUP["sha256"]["candidate_manifest"],
            "manifest_sha256_actual": sha(manifest_path),
            "manifest_metadata": {"schema_version": manifest.get("schema_version"), "release": manifest.get("release"), "cli_version": manifest.get("cli_version")},
            "files": {},
        }
        for relative, expected in manifest["files"].items():
            path = candidate_kit / relative
            actual = sha(path) if path.is_file() else None
            package["files"][relative] = {"expected": expected, "actual": actual, "match": actual == expected}
        package["all_manifest_files_match"] = all(item["match"] for item in package["files"].values())
        for key in ("old_cli", "candidate_cli"):
            package[key] = {"expected": SETUP["sha256"][key], "actual": sha(Path(SETUP["paths"][key]))}
            package[key]["match"] = package[key]["expected"] == package[key]["actual"]
        old_manifest = BASE / "source-v0-2-13" / "kit" / "manifest.json"
        package["old_manifest"] = {"expected": SETUP["sha256"]["old_manifest"], "actual": sha(old_manifest)}
        package["old_manifest"]["match"] = package["old_manifest"]["expected"] == package["old_manifest"]["actual"]
        old_rules = (BASE / "source-v0-2-13" / "kit" / "rules.md").read_text(encoding="utf-8").splitlines(keepends=True)
        new_rules = (candidate_kit / "rules.md").read_text(encoding="utf-8").splitlines(keepends=True)
        old_update = (BASE / "source-v0-2-13" / "kit" / "procedures" / "update.md").read_text(encoding="utf-8").splitlines(keepends=True)
        new_update = (candidate_kit / "procedures" / "update.md").read_text(encoding="utf-8").splitlines(keepends=True)
        diff = list(difflib.unified_diff(old_rules, new_rules, fromfile="v0-2-13/rules.md", tofile="v0-2-14/rules.md"))
        diff += list(difflib.unified_diff(old_update, new_update, fromfile="v0-2-13/procedures/update.md", tofile="v0-2-14/procedures/update.md"))
        (CASE / "agent-rules-update.diff").write_text("".join(diff), encoding="utf-8")
        profile = Path(SETUP["paths"]["good_profile"])
        stable_skills = {}
        for name in ("approve", "clear", "init", "push", "spec", "task"):
            old = profile / ".agents" / "skills" / f"highgrade-{name}" / "SKILL.md"
            new = candidate_kit / "skills" / f"highgrade-{name}" / "SKILL.md"
            stable_skills[name] = {"installed_sha256": sha(old), "candidate_sha256": sha(new), "match": sha(old) == sha(new)}
        package["stable_router_skills"] = stable_skills
        package["all_stable_router_skills_match"] = all(item["match"] for item in stable_skills.values())
        write_json(CASE / "agent-preflight.json", package)
        snapshot(profile, "good-before")
        snapshot(Path(SETUP["paths"]["fail_profile"]), "fail-before")
        print(json.dumps({
            "candidate_manifest_match": package["manifest_sha256_expected"] == package["manifest_sha256_actual"],
            "all_manifest_files_match": package["all_manifest_files_match"],
            "old_cli_match": package["old_cli"]["match"],
            "candidate_cli_match": package["candidate_cli"]["match"],
            "old_manifest_match": package["old_manifest"]["match"],
            "stable_router_skills_match": package["all_stable_router_skills_match"],
        }, ensure_ascii=False, indent=2))
    elif mode in ("good-after", "fail-after", "repeat-after"):
        profile = Path(SETUP["paths"]["fail_profile"] if mode == "fail-after" else SETUP["paths"]["good_profile"])
        result = snapshot(profile, mode)
        print(json.dumps({"label": mode, "active": result["active"], "file_count": len(result["files"])}, ensure_ascii=False, indent=2))
    else:
        raise SystemExit(f"unknown mode: {mode}")
