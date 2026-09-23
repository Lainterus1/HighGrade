#!/usr/bin/env python3
"""Run an explicitly selected project test entrypoint and attest fresh coverage."""
from __future__ import annotations

import argparse
import json
import os
import math
import subprocess
import sys
import uuid
from pathlib import Path

from code_health.validation import load_policy, validate, schema_for
from code_health.core import canonical_json, deep_merge, sha256_file, sha256_text
from code_health.git_scope import snapshot_repository, compare_snapshots
from code_health.provenance import source_manifest
from code_health.processes import run_bounded


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", required=True, type=Path)
    parser.add_argument("--output-dir", required=True, type=Path, help="New, empty directory outside the repository")
    parser.add_argument("--python", default=sys.executable, help="Project Python with pinned Coverage.py and project dependencies")
    parser.add_argument("--rcfile", type=Path, help="Existing project coverage configuration")
    parser.add_argument("--policy", type=Path)
    parser.add_argument("--timeout", type=float, default=300)
    target = parser.add_mutually_exclusive_group(required=True)
    target.add_argument("--module", help="Existing test module, e.g. pytest or unittest")
    target.add_argument("--script", help="Existing test script relative to root")
    parser.add_argument("test_args", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    root, output = args.root.resolve(), args.output_dir.resolve()
    if not root.is_dir() or root == output or root in output.parents:
        parser.error("root must exist; output must be outside it")
    if not math.isfinite(args.timeout) or args.timeout <= 0 or args.timeout > 3600:
        parser.error("timeout must be in (0, 3600]")
    if output.exists() and any(output.iterdir()):
        parser.error("output directory must be empty; never reuse coverage data")
    output.mkdir(parents=True, exist_ok=True)
    data = output / ".coverage"
    artifact = output / "coverage.json"
    provenance = output / "coverage-provenance.json"
    policy = load_policy(args.policy)
    run_context = "code-health-audit:" + uuid.uuid4().hex
    command = [args.python, "-B", "-m", "coverage", "run", "--branch", f"--data-file={data}", f"--context={run_context}"]
    config = None
    config_args = []
    if args.rcfile:
        path = args.rcfile.resolve()
        if root not in path.parents or not path.is_file():
            parser.error("rcfile must be an existing file inside root")
        config = {"path": path.relative_to(root).as_posix(), "sha256": sha256_file(path)}
        config_args = [f"--rcfile={path}"]
        command += config_args
    if args.module:
        command += ["-m", args.module]
    else:
        path = (root / args.script).resolve()
        if root not in path.parents or not path.is_file():
            parser.error("script must be an existing file inside root")
        command += [str(path)]
    command += args.test_args[1:] if args.test_args[:1] == ["--"] else args.test_args
    environment = os.environ.copy()
    environment.update(PYTHONDONTWRITEBYTECODE="1", COVERAGE_FILE=str(data))
    # Explicit config selection; inherited configuration must not change evidence silently.
    environment.pop("COVERAGE_RCFILE", None)
    environment.pop("COVERAGE_PROCESS_START", None)
    temp = output / "scratch"
    temp.mkdir()
    environment.update(TEMP=str(temp), TMP=str(temp), TMPDIR=str(temp))
    environment["PYTHONPYCACHEPREFIX"] = str(temp / "pycache")
    result = {"schema_version": "1.0.0", "status": "FAILED", "exit_code": None,
              "command": command, "run_context": run_context, "config": config,
              "auto_configs": {name: sha256_file(root / name) if (root / name).is_file() else None
                               for name in (".coveragerc", "pyproject.toml", "setup.cfg", "tox.ini")},
              "policy_sha256": sha256_text(canonical_json(policy))}
    before = None
    try:
        result["sources_before"] = source_manifest(root, policy)
        before = snapshot_repository(root, list(result["sources_before"]), policy)
        with (output / "test-output.log").open("w", encoding="utf-8") as log:
            completed = run_bounded(command, cwd=root, env=environment, stdout=log,
                                       stderr=subprocess.STDOUT, timeout=args.timeout, check=False)
        result["exit_code"] = completed.returncode
        if completed.returncode:
            raise ValueError(f"test command exited {completed.returncode}")
        if not data.is_file():
            raise ValueError("fresh coverage data was not produced; parallel/custom data routing is unsupported")
        export = run_bounded([args.python, "-B", "-m", "coverage", "json", *config_args,
                                 f"--data-file={data}", "-o", str(artifact)], cwd=root,
                                env=environment, capture_output=True, timeout=60, check=False)
        if export.returncode:
            raise ValueError(f"coverage JSON export exited {export.returncode}: {export.stderr.decode('utf-8', 'replace')[:300]}")
        # Inspect data in the selected project interpreter, not an unrelated environment.
        check_code = "import json,sys,platform,coverage; from coverage import CoverageData; d=CoverageData(basename=sys.argv[1]); d.read(); print(json.dumps({'contexts':sorted(d.measured_contexts()),'runtime':{'python':platform.python_version(),'implementation':platform.python_implementation(),'system':platform.system(),'machine':platform.machine(),'coverage':coverage.__version__}}))"
        check = run_bounded([args.python, "-B", "-c", check_code, str(data)], cwd=output,
                               env=environment, capture_output=True, text=True, timeout=30, check=True)
        observed = json.loads(check.stdout)
        result["project_runtime"] = observed["runtime"]
        if run_context not in observed["contexts"]:
            raise ValueError("fresh run context missing; test tooling replaced the coverage session")
        result.update(artifact_sha256=sha256_file(artifact), data_sha256=sha256_file(data), status="VERIFIED")
    except Exception as exc:
        result.update(status="FAILED", error=f"{type(exc).__name__}: {exc}")
    finally:
        try:
            result["sources_after"] = source_manifest(root, policy)
            result["changed_paths"] = compare_snapshots(before, snapshot_repository(root, list(result["sources_after"]), policy)) if before else ["<snapshot-unavailable>"]
            if result["sources_after"] != result.get("sources_before") or result["changed_paths"]:
                result.update(status="FAILED", error="READ_ONLY_BREACH: files changed during coverage capture")
        except Exception as exc:
            result.update(status="FAILED", error=f"final snapshot failed: {exc}")
        if result['status'] == 'VERIFIED':
            try:
                validate(result,schema_for('coverage-provenance'))
            except ValueError as exc:
                result.update(status='FAILED',error=f'invalid capture evidence: {exc}')
        provenance.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"status": result["status"], "provenance": str(provenance), "coverage": str(artifact), "error": result.get("error")}))
    return 0 if result["status"] == "VERIFIED" else 2


if __name__ == "__main__":
    raise SystemExit(main())
