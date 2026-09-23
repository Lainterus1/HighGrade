from __future__ import annotations

import json
import math
import re
from datetime import date
from pathlib import Path
from typing import Any

from .core import normalize_relative, sha256_file
from .git_scope import (
    ACCEPTANCE_PATH,
    acceptance_is_clean,
    is_git_repository,
    read_file_at_revision,
)


def _entry_problem(entry: Any) -> str | None:
    from .validation import validate, schema_for, safe_relative
    try:
        schema = schema_for('accepted-findings')
        validate(entry, schema['properties']['entries']['items'])
        safe_relative(entry['path'])
        for guard in entry.get('guard_files', []):
            safe_relative(guard['path'])
        if entry.get('review_after'):
            date.fromisoformat(entry['review_after'])
    except (ValueError, TypeError) as exc:
        return str(exc)
    return None


def _parse_registry(raw: bytes, source: str) -> tuple[list[dict[str, Any]], list[dict[str, str]]]:
    errors: list[dict[str, str]] = []
    try:
        payload = json.loads(raw.decode("utf-8"))
    except (UnicodeError, json.JSONDecodeError) as exc:
        return [], [{"code": "ACCEPTANCE_INVALID", "message": f"{source}: {exc}"}]
    if (
        not isinstance(payload, dict)
        or set(payload) != {"schema_version", "entries"}
        or payload.get("schema_version") != "1.0.0"
        or not isinstance(payload.get("entries"), list)
    ):
        return [], [
            {
                "code": "ACCEPTANCE_INVALID",
                "message": f"{source}: expected schema_version 1.0.0 and an entries array",
            }
        ]
    valid: list[dict[str, Any]] = []
    seen: set[str] = set()
    for index, entry in enumerate(payload["entries"]):
        problem = _entry_problem(entry)
        if problem:
            errors.append(
                {
                    "code": "ACCEPTANCE_ENTRY_INVALID",
                    "message": f"{source}: entry {index}: {problem}",
                }
            )
            continue
        finding_id = str(entry["finding_id"])
        if finding_id in seen:
            errors.append(
                {
                    "code": "ACCEPTANCE_DUPLICATE_ID",
                    "message": f"{source}: duplicate finding_id {finding_id}",
                }
            )
            continue
        seen.add(finding_id)
        normalized = dict(entry)
        normalized["path"] = normalize_relative(entry["path"])
        valid.append(normalized)
    return valid, errors


def load_trusted_acceptance(
    root: Path,
    mode: str,
    merge_base: str | None,
    acceptance_changed: bool,
) -> dict[str, Any]:
    errors: list[dict[str, str]] = []
    source = "none"
    entries: list[dict[str, Any]] = []
    trusted = True
    if mode == "diff":
        if merge_base:
            raw = read_file_at_revision(root, merge_base, ACCEPTANCE_PATH)
            if raw is not None:
                source = f"{merge_base}:{ACCEPTANCE_PATH}"
                parsed, parse_errors = _parse_registry(raw, source)
                errors.extend(parse_errors)
                if parse_errors:
                    trusted = False
                else:
                    entries = parsed
        if acceptance_changed:
            errors.append(
                {
                    "code": "ACCEPTANCE_CHANGE_UNTRUSTED",
                    "message": "current diff changes accepted-findings.json; base registry remains the only trusted input",
                }
            )
    else:
        path = root / ACCEPTANCE_PATH
        if path.is_file():
            if is_git_repository(root) and acceptance_is_clean(root):
                source = ACCEPTANCE_PATH
                parsed, parse_errors = _parse_registry(path.read_bytes(), source)
                errors.extend(parse_errors)
                if parse_errors:
                    trusted = False
                else:
                    entries = parsed
            else:
                trusted = False
                errors.append(
                    {
                        "code": "ACCEPTANCE_CURRENT_UNTRUSTED",
                        "message": "full mode ignores an uncommitted or non-versioned acceptance registry",
                    }
                )
    if any(
        error["code"]
        in {"ACCEPTANCE_INVALID", "ACCEPTANCE_ENTRY_INVALID", "ACCEPTANCE_DUPLICATE_ID"}
        for error in errors
    ):
        trusted = False
    return {"entries": entries, "errors": errors, "trusted": trusted, "source": source}


def _guard_reason(root: Path, entry: dict[str, Any]) -> str | None:
    for guard in entry.get("guard_files", []):
        relative = normalize_relative(guard.get("path", ""))
        expected = guard.get("sha256")
        path = (root / relative).resolve()
        if root.resolve() not in path.parents:
            return f"GUARD_OUTSIDE_ROOT:{relative}"
        if not path.is_file():
            return f"GUARD_MISSING:{relative}"
        if sha256_file(path) != expected:
            return f"GUARD_CHANGED:{relative}"
    return None


def apply_acceptance(
    root: Path,
    measurements: list[dict[str, Any]],
    entries: list[dict[str, Any]],
    policy_version: str,
) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    entry_map = {entry["finding_id"]: entry for entry in entries}
    matched: set[str] = set()
    findings: list[dict[str, Any]] = []
    acceptance_results: list[dict[str, Any]] = []
    for measurement in measurements:
        if measurement["classification"] == "OK":
            continue
        finding = dict(measurement)
        finding["finding_id"] = measurement["measurement_id"]
        finding["active"] = True
        finding["disposition"] = "ACTIVE"
        entry = entry_map.get(finding["finding_id"])
        if entry is None:
            findings.append(finding)
            continue
        matched.add(finding["finding_id"])
        reopen_reason = None
        if (
            entry.get("metric") != finding["metric"]
            or normalize_relative(entry.get("path", "")) != finding["path"]
            or entry.get("symbol") != finding["symbol"]
        ):
            reopen_reason = "IDENTITY_CHANGED"
        elif entry.get("policy_version") != policy_version:
            reopen_reason = "POLICY_CHANGED"
        elif not entry.get("policy_sha256") or entry.get("policy_sha256") != finding.get("policy_sha256"):
            reopen_reason = "POLICY_CHANGED"
        elif finding.get('runtime_sha256') and entry.get('runtime_sha256') != finding['runtime_sha256']:
            reopen_reason = 'RUNTIME_CHANGED'
        elif entry.get("analyzer") != finding.get("analyzer"):
            reopen_reason = "ANALYZER_CHANGED"
        elif entry.get("evidence_fingerprint") != finding.get("evidence_fingerprint"):
            reopen_reason = "EVIDENCE_CHANGED"
        elif ((finding['thresholds']['direction'] == 'high' and finding['value'] > entry['ceiling'])
              or (finding['thresholds']['direction'] == 'low' and finding['value'] < entry['ceiling'])):
            reopen_reason = "REGRESSION"
        else:
            review_after = entry.get("review_after")
            if review_after:
                try:
                    if date.fromisoformat(review_after) < date.today():
                        reopen_reason = "REVIEW_EXPIRED"
                except ValueError:
                    reopen_reason = "REVIEW_DATE_INVALID"
        if reopen_reason is None:
            reopen_reason = _guard_reason(root, entry)

        if reopen_reason:
            finding["disposition"] = "REOPENED"
            finding["reopen_reason"] = reopen_reason
            acceptance_results.append(
                {
                    "finding_id": finding["finding_id"],
                    "status": "REOPENED",
                    "reason": reopen_reason,
                }
            )
        else:
            finding["active"] = False
            finding["disposition"] = "ACCEPTED"
            if ((finding['thresholds']['direction'] == 'high' and finding['value'] < entry['accepted_value'])
                or (finding['thresholds']['direction'] == 'low' and finding['value'] > entry['accepted_value'])):
                finding["baseline_can_tighten"] = True
            acceptance_results.append(
                {
                    "finding_id": finding["finding_id"],
                    "status": "ACCEPTED",
                    "reason": entry.get("reason"),
                    "baseline_can_tighten": bool(finding.get("baseline_can_tighten")),
                }
            )
        findings.append(finding)

    for entry in entries:
        if entry["finding_id"] not in matched:
            acceptance_results.append(
                {
                    "finding_id": entry["finding_id"],
                    "status": "ORPHANED",
                    "reason": "no matching non-OK measurement in this audit scope",
                }
            )
    return findings, sorted(acceptance_results, key=lambda item: (item["finding_id"], item["status"]))
