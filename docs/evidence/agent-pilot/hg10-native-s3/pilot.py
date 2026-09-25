"""Local, reversible native-unittest probe for synthetic HG-0010-S7/S8."""

import difflib
import hashlib
import json
import re
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
OUT = Path(__file__).resolve().parent
TEST = ROOT / "test_codec.py"
TRACKED = (
    "AGENTS.md",
    "README.md",
    "codec.py",
    "test_codec.py",
    "specs/changes/HG-0001/spec.json",
    "specs/changes/HG-0001/results.json",
)
COMMAND = [sys.executable, "-B", "-m", "unittest", "-v", "test_codec"]
JOURNAL = OUT / "actions.jsonl"


def now():
    return datetime.now(timezone.utc).isoformat()


def sha(data):
    return hashlib.sha256(data).hexdigest()


def file_sha(path):
    return sha(path.read_bytes())


def log(action, **details):
    record = {"at": now(), "action": action, **details}
    with JOURNAL.open("a", encoding="utf-8", newline="\n") as stream:
        stream.write(json.dumps(record, ensure_ascii=False, sort_keys=True) + "\n")


def run(label):
    log("native_unittest_started", label=label, command=COMMAND, cwd=str(ROOT))
    completed = subprocess.run(
        COMMAND, cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
        check=False,
    )
    report = OUT / f"{label}-unittest.txt"
    report.write_bytes(completed.stdout)
    result = {
        "command": COMMAND,
        "cwd": str(ROOT),
        "exit_code": completed.returncode,
        "report": str(report.relative_to(ROOT)).replace("\\", "/"),
        "report_sha256": sha(completed.stdout),
        "test_codec_sha256": file_sha(TEST),
        "codec_sha256": file_sha(ROOT / "codec.py"),
        "finished_at": now(),
    }
    log("native_unittest_finished", label=label, **result)
    return result, completed.stdout.decode("utf-8", errors="replace")


def require(condition, message):
    if not condition:
        log("verification_failed", message=message)
        raise RuntimeError(message)


def main():
    setup = json.loads((OUT / "setup.json").read_text(encoding="utf-8"))
    expected = setup["before_sha256"]
    require(set(expected) == set(TRACKED), "setup tracked-file set differs")
    before = {name: file_sha(ROOT / name) for name in TRACKED}
    snapshots = {name: file_sha(OUT / "before" / name) for name in TRACKED}
    require(before == expected == snapshots, "current inputs or before snapshot differ from setup")
    original = TEST.read_bytes()
    log("initial_state_verified", sha256=before)

    native, native_text = run("s7-native")
    require(native["exit_code"] == 0, "native unittest exit is not zero")
    require("Ran 3 tests" in native_text and re.search(r"(?m)^OK\s*$", native_text),
            "native unittest summary is not clean OK")
    for test_name, scenario_id in (
        ("test_v2", "HG-0001-S1"),
        ("test_v1", "HG-0001-S2"),
        ("test_unknown", "HG-0001-S3"),
    ):
        pattern = rf"(?s){test_name}.*?{scenario_id}.*?\.\.\. ok"
        require(re.search(pattern, native_text) is not None,
                f"native report lacks executed {test_name}/{scenario_id}")
    require(file_sha(TEST) == before["test_codec.py"], "test file changed during native run")
    require(file_sha(ROOT / "codec.py") == before["codec.py"], "source changed during native run")
    log("s7_native_evidence_verified", linked_scenarios=["HG-0001-S1", "HG-0001-S2", "HG-0001-S3"])

    needle = b"    def test_v2(self):"
    require(original.count(needle) == 1, "test_v2 anchor is not unique")
    newline = b"\r\n" if b"\r\n" in original else b"\n"
    annotation = b'    @unittest.skip("HG-0010-S8 synthetic skip probe")' + newline
    temporary = original.replace(needle, annotation + needle, 1)
    temporary_hash = sha(temporary)
    temporary_diff = "".join(difflib.unified_diff(
        original.decode("utf-8").splitlines(keepends=True),
        temporary.decode("utf-8").splitlines(keepends=True),
        fromfile="before/test_codec.py", tofile="temporary/test_codec.py",
    ))
    (OUT / "temporary-test.diff").write_text(temporary_diff, encoding="utf-8", newline="\n")

    skipped = None
    try:
        TEST.write_bytes(temporary)
        require(file_sha(TEST) == temporary_hash, "temporary skip edit differs")
        log("skip_injected", scenario="HG-0001-S1", test="test_v2",
            before_sha256=before["test_codec.py"], temporary_sha256=temporary_hash,
            diff="evidence/hg10-native/temporary-test.diff")
        skipped, skipped_text = run("s8-skipped")
        require(skipped["exit_code"] == 0, "skipped suite exit is not zero")
        require("Ran 3 tests" in skipped_text and "OK (skipped=1)" in skipped_text,
                "skipped suite summary differs")
        require(re.search(r"(?s)test_v2.*?HG-0001-S1.*?\.\.\. skipped", skipped_text) is not None,
                "HG-0001-S1/test_v2 is not explicitly skipped")
        for test_name, scenario_id in (("test_v1", "HG-0001-S2"),
                                       ("test_unknown", "HG-0001-S3")):
            require(re.search(rf"(?s){test_name}.*?{scenario_id}.*?\.\.\. ok", skipped_text)
                    is not None, f"{scenario_id} is not observed as passed")
        require(file_sha(TEST) == temporary_hash, "test file changed during skipped run")
        require(file_sha(ROOT / "codec.py") == before["codec.py"], "source changed during skipped run")
        log("s8_gap_verified", suite_exit_code=0, skipped_scenario="HG-0001-S1",
            scenario_status="unknown", reason="test_v2 was skipped")
    finally:
        current = file_sha(TEST)
        if current == temporary_hash:
            TEST.write_bytes(original)
            log("test_file_restored", restored_sha256=file_sha(TEST))
        elif current == before["test_codec.py"]:
            log("test_file_already_restored", restored_sha256=current)
        else:
            log("restore_blocked_by_unexpected_change", current_sha256=current)
            raise RuntimeError("test file changed unexpectedly; refusing to overwrite it")

    after = {name: file_sha(ROOT / name) for name in TRACKED}
    final_diff = "".join(difflib.unified_diff(
        original.decode("utf-8").splitlines(keepends=True),
        TEST.read_text(encoding="utf-8").splitlines(keepends=True),
        fromfile="before/test_codec.py", tofile="after/test_codec.py",
    ))
    (OUT / "final-vs-before.diff").write_text(final_diff, encoding="utf-8", newline="\n")
    require(after == before and final_diff == "", "final inputs differ from before")
    log("final_state_verified", sha256=after, diff_empty=True)

    manifest = {
        "captured_at": now(),
        "synthetic_root": str(ROOT),
        "python_executable": sys.executable,
        "python_version": sys.version,
        "inputs_sha256": before,
        "final_sha256": after,
        "final_diff_empty": True,
        "HG-0010-S7": {
            "native_unittest": native,
            "linked_scenarios": ["HG-0001-S1", "HG-0001-S2", "HG-0001-S3"],
            "observation": "All three named tests executed and passed in the raw native report.",
            "trace_status": "unknown",
            "trace_limit": "Native unittest output is not a supported trace run-record; trace was not run and no trace PASS is claimed.",
        },
        "HG-0010-S8": {
            "native_unittest": skipped,
            "skipped_scenario": "HG-0001-S1",
            "scenario_status": "unknown",
            "reason": "test_v2 was skipped; suite exit code 0 does not verify HG-0001-S1.",
            "temporary_test_sha256": temporary_hash,
            "restored_test_sha256": after["test_codec.py"],
        },
        "files": {
            "journal": "evidence/hg10-native/actions.jsonl",
            "temporary_diff": "evidence/hg10-native/temporary-test.diff",
            "final_diff": "evidence/hg10-native/final-vs-before.diff",
        },
    }
    (OUT / "run-manifest.json").write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n",
    )
    print(json.dumps({"native_exit": native["exit_code"],
                      "skipped_exit": skipped["exit_code"],
                      "skipped_scenario": "HG-0001-S1",
                      "scenario_status": "unknown", "restored": after == before}))


if __name__ == "__main__":
    main()
