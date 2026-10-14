"""Process boundary for analyzers, including native-extension failures and hangs."""
from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
from typing import Any
from .processes import run_bounded


def run_adapter(name: str, root: Path, files: list[str], ast_index: dict,
                policy: dict, expected_version: str, **extra: Any) -> dict:
    failure = {"tool": {"name": name, "expected_version": expected_version,
                        "actual_version": None, "status": "FAILED", "reason": ""},
               "measurements": [], "errors": [], "file_nloc": {}, "cycle_signatures": []}
    try:
        with tempfile.TemporaryDirectory(prefix="code-health-worker-") as directory:
            request, result = Path(directory) / "request.json", Path(directory) / "result.json"
            request.write_text(json.dumps({"name": name, "root": str(root), "files": files,
                                          "ast_index": ast_index, "policy": policy,
                                          "expected_version": expected_version, "extra": extra}), encoding="utf-8")
            env = os.environ.copy()
            env["PYTHONDONTWRITEBYTECODE"] = "1"
            completed = run_bounded([sys.executable, "-B", str(Path(__file__).resolve().parents[1] / "analyzer_worker.py"), str(request), str(result)],
                                       cwd=directory, env=env, capture_output=True,
                                       timeout=policy.get("analyzer_timeout_seconds", 120), check=False)
            if completed.returncode or not result.is_file():
                raise RuntimeError(f"analyzer process exited {completed.returncode}: {completed.stderr.decode('utf-8', 'replace')[-400:]}")
            payload = json.loads(result.read_text(encoding="utf-8"))
            if not isinstance(payload, dict) or not isinstance(payload.get("measurements"), list) or not isinstance(payload.get("tool"), dict):
                raise ValueError("invalid analyzer response")
            return payload
    except Exception as exc:
        failure["tool"]["reason"] = f"{type(exc).__name__}: {exc}"
        failure["errors"] = [{"code": name.upper() + "_BOUNDARY_FAILED", "message": failure["tool"]["reason"]}]
        return failure
