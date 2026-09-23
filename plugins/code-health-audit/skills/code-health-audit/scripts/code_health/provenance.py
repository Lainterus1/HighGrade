"""Coverage evidence tied to an observed run, never to JSON creation time."""
from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from .core import canonical_json, sha256_file, sha256_text
from .git_scope import discover_python_files
from .validation import validate, schema_for


def source_manifest(root: Path, policy: dict[str, Any]) -> dict[str, str]:
    return {path: sha256_file(root / path) for path in discover_python_files(root, policy)}


def validate_provenance(root: Path, artifact: Path, provenance: Path | None,
                        policy: dict[str, Any]) -> str | None:
    if provenance is None:
        return "coverage needs a successful capture_coverage.py run and --coverage-provenance"
    try:
        payload = json.loads(provenance.read_text(encoding="utf-8"))
        if not isinstance(payload, dict) or payload.get("schema_version") != "1.0.0":
            return "unsupported coverage provenance"
        if payload.get("status") != "VERIFIED" or payload.get("exit_code") != 0:
            return "coverage run did not finish successfully"
        validate(payload,schema_for('coverage-provenance'))
        if payload.get("artifact_sha256") != sha256_file(artifact):
            return "coverage JSON changed after the observed run"
        current = source_manifest(root, policy)
        if payload.get("sources_before") != current or payload.get("sources_after") != current:
            return "coverage source state differs from current audit scope"
        if payload.get("policy_sha256") != sha256_text(canonical_json(policy)):
            return "coverage was captured with a different effective policy"
        config = payload.get("config")
        if config is not None:
            if not isinstance(config, dict) or not isinstance(config.get("path"), str):
                return "invalid coverage configuration identity"
            config_path = (root / config["path"]).resolve()
            if root.resolve() not in config_path.parents or not config_path.is_file():
                return "coverage configuration must still exist inside the repository"
            if sha256_file(config_path) != config.get("sha256"):
                return "coverage configuration changed after measurement"
        # Auto-discovered configs can affect a run even without --rcfile.
        for name, digest in payload.get("auto_configs", {}).items():
            if name not in {".coveragerc", "pyproject.toml", "setup.cfg", "tox.ini"}:
                return "invalid automatic configuration path"
            path = root / name
            if (sha256_file(path) if path.is_file() else None) != digest:
                return "automatic coverage configuration changed after measurement"
        data_path = provenance.parent / ".coverage"
        if not data_path.is_file() or sha256_file(data_path) != payload.get("data_sha256"):
            return "coverage data changed or is missing"
        from coverage import CoverageData
        data = CoverageData(basename=str(data_path))
        data.read()
        nonce = payload.get("run_context")
        if not isinstance(nonce, str) or not nonce.startswith("code-health-audit:"):
            return "coverage run context is invalid"
        if nonce not in data.measured_contexts():
            return "fresh run context is absent from coverage data"
    except (OSError, ValueError, TypeError, AttributeError, ImportError) as exc:
        return f"invalid coverage provenance: {exc}"
    return None
