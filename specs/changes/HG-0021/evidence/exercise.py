"""Bounded, reproducible HG-0021 workflow exercise on a synthetic fixture."""

import datetime
import hashlib
import json
import pathlib
import subprocess
import sys


EVIDENCE = pathlib.Path(__file__).resolve().parent
ROOT = EVIDENCE.parents[3]
PILOT = ROOT / "target" / "highgrade" / "tmp" / "HG-0021" / "pilot2"
FIXTURE = json.loads((EVIDENCE / "fixture.json").read_text(encoding="utf-8"))["files"]
EVENTS = []


def emit(kind, **fields):
    EVENTS.append({"at": datetime.datetime.now(datetime.timezone.utc).isoformat(), "kind": kind, **fields})


def preflight(cwd, mode):
    cwd = pathlib.Path(cwd).resolve()
    if not cwd.is_relative_to(PILOT.resolve()):
        return "reject: cwd outside isolated pilot"
    if mode != "persistent":
        return "reject: mode cannot retain required Goal"
    if not (cwd / "TASK.md").is_file() or not (cwd / "DEVELOPMENT.md").is_file():
        return "reject: required inputs missing"
    return "accept"


def run(name, args):
    p = subprocess.run(args, cwd=PILOT, capture_output=True, text=True, encoding="utf-8")
    emit("command", name=name, command=args, cwd=str(PILOT), exit_code=p.returncode,
         stdout=p.stdout, stderr=p.stderr)
    return p.returncode


def write_report(path, data):
    path.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def main():
    assert PILOT.resolve().is_relative_to((ROOT / "target").resolve())
    assert not PILOT.exists(), "pilot2 must be a fresh workspace"
    for name, record in FIXTURE.items():
        path = PILOT / name
        assert path.resolve().is_relative_to(PILOT.resolve())
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(record["content"].replace("\n", "\r\n").encode("utf-8"))
        assert hashlib.sha256(path.read_bytes()).hexdigest() == record["sha256"]
    emit("setup", cwd=str(PILOT), file_count=len(FIXTURE), fixture="fixture.json")

    cases = [
        ("wrong_cwd", ROOT, "persistent"),
        ("ephemeral_mode", PILOT, "ephemeral"),
        ("valid", PILOT, "persistent"),
    ]
    decisions = []
    for name, cwd, mode in cases:
        result = preflight(cwd, mode)
        decisions.append({"case": name, "cwd": str(cwd), "mode": mode, "result": result})
        emit("preflight", **decisions[-1])
    assert [d["result"] for d in decisions] == [
        "reject: cwd outside isolated pilot", "reject: mode cannot retain required Goal", "accept"
    ]

    toy = PILOT / "toy.py"
    original = toy.read_bytes()
    toy.write_bytes(original.replace(b"return a + b", b"return a - b"))
    emit("controlled_fault", file="toy.py", change="addition to subtraction")
    focused = [sys.executable, "-m", "unittest", "tests.test_toy.ToyTests.test_add", "-v"]
    assert run("focus_expected_failure", focused) != 0
    toy.write_bytes(original)
    emit("repair", file="toy.py", restored_sha256=hashlib.sha256(toy.read_bytes()).hexdigest())
    for cache in (PILOT / "__pycache__").glob("toy.*.pyc"):
        cache.unlink()
        emit("cache_invalidation", file=str(cache.relative_to(PILOT)))
    assert run("focus_retry", focused) == 0
    assert run("full_barrier", [sys.executable, "-m", "unittest", "discover", "-s", "tests", "-v"]) == 0
    assert run("unique_schema", [sys.executable, "scripts/check_schema.py"]) == 0
    emit("skip_duplicate", command="python scripts/run_full.py", reason="same unittest discovery as full_barrier")

    for name, record in FIXTURE.items():
        assert hashlib.sha256((PILOT / name).read_bytes()).hexdigest() == record["sha256"]
    raw = EVIDENCE / "raw-pilot-actions.jsonl"
    raw.write_text("".join(json.dumps(e, ensure_ascii=False) + "\n" for e in EVENTS), encoding="utf-8")
    raw_sha = hashlib.sha256(raw.read_bytes()).hexdigest()
    command_events = [e for e in EVENTS if e["kind"] == "command"]
    write_report(EVIDENCE / "scenario-selection.json", {
        "fixture": "fixture.json", "raw_actions": raw.name, "raw_actions_sha256": raw_sha,
        "selected": [
            {"id": "P-S1", "outcome": "integer addition", "examples": ["2+3=5", "4+1=5"], "observed_in": "full_barrier"},
            {"id": "P-S2", "outcome": "non-integer rejected", "examples": ["add_int('2',3) raises TypeError"], "observed_in": "full_barrier"},
        ],
        "full_barrier_exit_code": next(e["exit_code"] for e in command_events if e["name"] == "full_barrier"),
        "limit": "Synthetic fixture demonstrates one execution, not future agent reliability.",
    })
    write_report(EVIDENCE / "check-selection.json", {
        "fixture": "fixture.json", "raw_actions": raw.name, "raw_actions_sha256": raw_sha,
        "command_outcomes": [{"name": e["name"], "exit_code": e["exit_code"]} for e in command_events],
        "failure_handling": "Controlled subtraction fault failed focus; restored input passed focus retry and final full barrier.",
        "unique_coverage": "schema check passed", "duplicate_wrapper": "skipped because it runs the same full unittest discovery",
    })
    write_report(EVIDENCE / "agent-preflight.json", {
        "driver": "current Codex agent session", "fixture": "fixture.json", "raw_actions": raw.name,
        "raw_actions_sha256": raw_sha, "preflight_cases": decisions,
        "actual_command_cwd": str(PILOT),
        "summary": "Two invalid preparations rejected before test commands; valid isolated run completed, including expected fault and repair.",
        "separate_cli_attempt": "Earlier separate CLI session yielded no events and was stopped; it is not used as success evidence.",
    })
    print(json.dumps({"status": "passed", "raw_actions": str(raw), "commands": len(command_events),
                      "preflight_rejected": 2, "cwd": str(PILOT)}))


if __name__ == "__main__":
    main()
