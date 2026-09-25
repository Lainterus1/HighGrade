"""Compare exact file snapshots for the synthetic update pilot."""

import json
from pathlib import Path

CASE = Path(r"D:\my_projects\MyCodex\target\update-pilot\case")


def read(name: str) -> dict:
    return json.loads((CASE / f"agent-{name}-snapshot.json").read_text(encoding="utf-8"))


def compare(left_name: str, right_name: str) -> dict:
    left = read(left_name)
    right = read(right_name)
    names = set(left["files"]) | set(right["files"])
    added = sorted(name for name in names if name not in left["files"])
    removed = sorted(name for name in names if name not in right["files"])
    changed = sorted(name for name in names if name in left["files"] and name in right["files"] and left["files"][name] != right["files"][name])
    return {
        "from": left_name,
        "to": right_name,
        "from_active": left["active"],
        "to_active": right["active"],
        "added": added,
        "removed": removed,
        "changed": changed,
        "identical_files": not (added or removed or changed),
    }


result = {
    "fail_before_to_after": compare("fail-before", "fail-after"),
    "good_before_to_after": compare("good-before", "good-after"),
    "good_after_to_repeat_after": compare("good-after", "repeat-after"),
}
(CASE / "agent-snapshot-diff.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
print(json.dumps({key: {"identical_files": value["identical_files"], "added_count": len(value["added"]), "removed_count": len(value["removed"]), "changed": value["changed"], "from_release": value["from_active"]["release"], "to_release": value["to_active"]["release"]} for key, value in result.items()}, ensure_ascii=False, indent=2))
