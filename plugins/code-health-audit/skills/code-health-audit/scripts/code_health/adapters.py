from __future__ import annotations

import importlib.metadata
import io
import tokenize
import json
import math
import os
import re
import subprocess
import sys
import tempfile
from datetime import datetime
from pathlib import Path
from typing import Any

from .core import (
    PYTHON_AST_NESTING_ANALYZER,
    canonical_json,
    make_measurement,
    normalize_relative,
    relative_path,
    sha256_file,
    sha256_text,
    source_kind,
    symbol_at,
)
from .provenance import validate_provenance
from .processes import run_bounded


def _version(package: str) -> str | None:
    try:
        return importlib.metadata.version(package)
    except importlib.metadata.PackageNotFoundError:
        return None


def _tool(name: str, expected: str, actual: str | None, status: str, reason: str = "") -> dict[str, Any]:
    return {
        "name": name,
        "expected_version": expected,
        "actual_version": actual,
        "status": status,
        "reason": reason,
    }


def _version_status(name: str, expected: str) -> tuple[str | None, str, str]:
    actual = _version(name)
    if actual is None:
        return None, "NOT_RUN", "package is not installed in the analyzer Python environment"
    if actual != expected:
        return actual, "VERSION_MISMATCH", f"expected {expected}, found {actual}"
    return actual, "OK", ""


def _finite_number(value: Any) -> bool:
    return not isinstance(value, bool) and isinstance(value, (int, float)) and math.isfinite(value)


def _coverage_file_problem(data: Any) -> str | None:
    if not isinstance(data, dict):
        return "file evidence must be an object"
    line_fields = ("executed_lines", "missing_lines", "excluded_lines")
    branch_fields = ("executed_branches", "missing_branches")
    for field in line_fields:
        values = data.get(field)
        if not isinstance(values, list) or any(
            isinstance(value, bool) or not isinstance(value, int) or value < 1 for value in values
        ):
            return f"{field} must be an array of positive line numbers"
    for field in branch_fields:
        values = data.get(field)
        if not isinstance(values, list) or any(
            not isinstance(value, list)
            or len(value) != 2
            or any(isinstance(line, bool) or not isinstance(line, int) for line in value)
            for value in values
        ):
            return f"{field} must be an array of integer line pairs"
    summary = data.get("summary")
    if not isinstance(summary, dict):
        return "summary must be an object"
    integer_fields = ("covered_lines", "num_statements", "num_branches", "covered_branches")
    for field in integer_fields:
        value = summary.get(field)
        if isinstance(value, bool) or not isinstance(value, int) or value < 0:
            return f"summary.{field} must be a non-negative integer"
    for field in ("percent_statements_covered", "percent_branches_covered"):
        value = summary.get(field)
        if not _finite_number(value) or not 0 <= value <= 100:
            return f"summary.{field} must be a finite percentage"
    executed = data["executed_lines"]
    missing = data["missing_lines"]
    if set(executed) & set(missing):
        return "executed_lines and missing_lines must not overlap"
    if summary["covered_lines"] != len(set(executed)):
        return "summary.covered_lines does not match executed_lines"
    if summary["num_statements"] != len(set(executed) | set(missing)):
        return "summary.num_statements does not match line evidence"
    if summary["covered_branches"] != len(data["executed_branches"]):
        return "summary.covered_branches does not match executed_branches"
    if summary["num_branches"] != len(data["executed_branches"]) + len(data["missing_branches"]):
        return "summary.num_branches does not match branch evidence"
    expected_lines = (
        100.0
        if summary["num_statements"] == 0
        else summary["covered_lines"] / summary["num_statements"] * 100
    )
    if not math.isclose(
        summary["percent_statements_covered"], expected_lines, rel_tol=0, abs_tol=1e-9
    ):
        return "summary.percent_statements_covered does not match line counts"
    expected_branches = (
        100.0
        if summary["num_branches"] == 0
        else summary["covered_branches"] / summary["num_branches"] * 100
    )
    if not math.isclose(
        summary["percent_branches_covered"], expected_branches, rel_tol=0, abs_tol=1e-9
    ):
        return "summary.percent_branches_covered does not match branch counts"
    return None


def _fail_coverage(result: dict[str, Any], code: str, reason: str) -> dict[str, Any]:
    result["measurements"] = []
    result["tool"]["status"] = "FAILED"
    result["tool"]["reason"] = reason
    result["errors"].append({"code": code, "message": reason})
    return result


def run_lizard(
    root: Path,
    files: list[str],
    ast_index: dict[str, dict[str, Any]],
    policy: dict[str, Any],
    expected_version: str,
) -> dict[str, Any]:
    actual, status, reason = _version_status("lizard", expected_version)
    result: dict[str, Any] = {
        "measurements": [],
        "errors": [],
        "tool": _tool("lizard", expected_version, actual, status, reason),
        "file_nloc": {},
    }
    if status != "OK":
        return result
    try:
        import lizard

        analyzed = list(
            lizard.analyze(
                [str(root / relative) for relative in files],
                lans=["python"],
                use_gitignore=False,
            )
        )
    except Exception as exc:  # analyzer boundary
        result["tool"]["status"] = "FAILED"
        result["tool"]["reason"] = str(exc)
        result["errors"].append({"code": "LIZARD_FAILED", "message": str(exc)})
        return result

    analyzer = {"name": "lizard", "version": actual or "unknown"}
    for file_info in analyzed:
        relative = relative_path(root, file_info.filename)
        file_fingerprint = ast_index.get(relative, {}).get("file_fingerprint", sha256_text(relative))
        file_nloc = float(file_info.nloc)
        result["file_nloc"][relative] = file_nloc
        result["measurements"].append(
            make_measurement(
                metric="file_nloc",
                value=file_nloc,
                path=relative,
                symbol="<module>",
                start_line=1,
                end_line=None,
                analyzer=analyzer,
                evidence_fingerprint=file_fingerprint,
                evidence={"nloc": int(file_nloc)},
                policy=policy,
            )
        )
        if relative.endswith(".pyi"):
            continue
        for function in file_info.function_list:
            symbol = symbol_at(ast_index, relative, function.start_line, function.name)
            if "nesting_depth" not in symbol:
                result["tool"]["status"] = "PARTIAL"
                result["errors"].append({"code": "LIZARD_SYMBOL_UNRESOLVED", "message": f"{relative}:{function.start_line}: AST symbol unavailable"})
                continue
            common = {
                "path": relative,
                "symbol": symbol["symbol"],
                "start_line": int(function.start_line),
                "end_line": int(function.end_line),
                "evidence_fingerprint": symbol["fingerprint"],
                "policy": policy,
            }
            result["measurements"].extend(
                [
                    make_measurement(
                        metric="cyclomatic_complexity",
                        value=float(function.cyclomatic_complexity),
                        analyzer=analyzer,
                        evidence={"lizard_field": "cyclomatic_complexity"},
                        **common,
                    ),
                    make_measurement(
                        metric="function_nloc",
                        value=float(function.nloc),
                        analyzer=analyzer,
                        evidence={"lizard_field": "nloc"},
                        **common,
                    ),
                    make_measurement(
                        metric="nesting_depth",
                        value=float(symbol["nesting_depth"]),
                        analyzer=PYTHON_AST_NESTING_ANALYZER,
                        evidence={
                            "algorithm": "python_ast_control_flow_depth",
                            "algorithm_version": PYTHON_AST_NESTING_ANALYZER["version"],
                            "top_level_control_depth": 1,
                            "elif_adds_depth": False,
                        },
                        **common,
                    ),
                ]
            )
    return result


def run_complexipy(
    root: Path,
    files: list[str],
    ast_index: dict[str, dict[str, Any]],
    policy: dict[str, Any],
    expected_version: str,
) -> dict[str, Any]:
    actual, status, reason = _version_status("complexipy", expected_version)
    result: dict[str, Any] = {
        "measurements": [],
        "errors": [],
        "tool": _tool("complexipy", expected_version, actual, status, reason),
    }
    if status != "OK":
        return result
    try:
        import complexipy
    except Exception as exc:
        result["tool"]["status"] = "FAILED"
        result["tool"]["reason"] = str(exc)
        result["errors"].append({"code": "COMPLEXIPY_IMPORT_FAILED", "message": str(exc)})
        return result

    analyzer = {"name": "complexipy", "version": actual or "unknown"}
    failures = 0
    for relative in files:
        if not relative.endswith(".py"):
            continue
        try:
            file_result = complexipy.file_complexity(str(root / relative), no_ignore=True)
        except Exception as exc:  # analyzer boundary
            failures += 1
            result["errors"].append(
                {"code": "COMPLEXIPY_FILE_FAILED", "message": f"{relative}: {exc}"}
            )
            continue
        for function in file_result.functions:
            symbol = symbol_at(ast_index, relative, int(function.line_start), function.name)
            result["measurements"].append(
                make_measurement(
                    metric="cognitive_complexity",
                    value=float(function.complexity),
                    path=relative,
                    symbol=symbol["symbol"],
                    start_line=int(function.line_start),
                    end_line=int(function.line_end),
                    analyzer=analyzer,
                    evidence_fingerprint=symbol["fingerprint"],
                    evidence={"complexipy_no_ignore": True},
                    policy=policy,
                )
            )
    if failures:
        result["tool"]["status"] = "PARTIAL"
        result["tool"]["reason"] = f"{failures} file(s) failed"
    return result


def _module_map(files: list[str]) -> dict[str, str]:
    # Resolve analyzer names only to unique, explicitly scoped existing paths.
    # Different source roots can produce the same import name; never guess then.
    candidates: dict[str, set[str]] = {}
    for relative in files:
        if not relative.endswith(".py"):
            continue
        parts = relative[:-3].split("/")
        if parts[-1] == "__init__":
            parts = parts[:-1]
        for start in range(len(parts)):
            candidates.setdefault(".".join(parts[start:]), set()).add(relative)
    return {name: next(iter(paths)) for name, paths in candidates.items() if len(paths) == 1}


def _duplicate_locations(message: str, modules: dict[str, str]) -> list[dict[str, Any]]:
    locations: list[dict[str, Any]] = []
    pattern = re.compile(r"^==([^:]+):\[(\d+):(\d+)\]$")
    for line in message.splitlines():
        match = pattern.match(line.strip())
        if not match:
            continue
        module, start, end = match.groups()
        if module not in modules or int(end) <= int(start):
            return []
        locations.append({"module": module, "path": modules[module], "start": int(start) + 1, "end": int(end)})
    return sorted(locations, key=lambda item: (item["path"], item["start"], item["end"]))


def _required_suppression(comment: str) -> bool:
    text = comment.lower()
    if re.search(r"#\s*pylint:\s*skip-file\b", text):
        return True
    pylint = re.search(r"#\s*pylint:\s*disable(?:-next)?\s*=\s*([^;#]+)", text)
    if pylint:
        rules = set(re.split(r"[\s,]+", pylint.group(1).strip()))
        return bool(rules & {"all", "r", "refactor", "r0801", "r0401", "duplicate-code", "cyclic-import", "similarities", "imports"})
    return bool(re.search(r"#\s*(?:lizard\s+forgives|complexipy:\s*ignore|noqa:\s*complexipy)", text))


def _scan_suppressions(root: Path, files: list[str], ast_index: dict[str, dict[str, Any]], policy: dict[str, Any]) -> list[dict[str, Any]]:
    analyzer = {"name": "code-health-audit-policy", "version": policy["policy_version"]}
    measurements = []
    for relative in files:
        if not relative.endswith(".py"):
            continue
        occurrences: dict[str, int] = {}
        try:
            with tokenize.open(root / relative) as handle:
                tokens = list(tokenize.generate_tokens(io.StringIO(handle.read()).readline))
        except (OSError, UnicodeError, SyntaxError, tokenize.TokenError):
            continue
        for token in tokens:
            if token.type != tokenize.COMMENT or not _required_suppression(token.string):
                continue
            number = token.start[0]
            symbol = symbol_at(ast_index, relative, number, "<module>")
            fingerprint = sha256_text(token.string.strip())
            identity = symbol["symbol"] + ":" + fingerprint[:12]
            occurrences[identity] = occurrences.get(identity, 0) + 1
            measurements.append(make_measurement(
                metric="suppression", value=1, path=relative,
                symbol=f"{symbol['symbol']}::<suppression:{fingerprint[:12]}:{occurrences[identity]}>",
                start_line=number, end_line=number, analyzer=analyzer,
                evidence_fingerprint=fingerprint, evidence={"directive": token.string.strip()}, policy=policy))
    return measurements


def run_pylint(
    root: Path,
    files: list[str],
    ast_index: dict[str, dict[str, Any]],
    file_nloc: dict[str, float],
    policy: dict[str, Any],
    expected_version: str,
) -> dict[str, Any]:
    actual, status, reason = _version_status("pylint", expected_version)
    result: dict[str, Any] = {
        "measurements": [],
        "errors": [],
        "tool": _tool("pylint", expected_version, actual, status, reason),
        "cycle_signatures": [],
    }
    if status != "OK":
        return result
    targets = sorted(relative for relative in files if relative.endswith(".py"))
    if not targets:
        result["tool"]["status"] = "NOT_RUN"
        result["tool"]["reason"] = "no Python files"
        return result
    with tempfile.TemporaryDirectory(prefix="code-health-pylint-") as temp_dir:
        rcfile = Path(temp_dir) / "pylintrc"
        rcfile.write_text("[MAIN]\n", encoding="utf-8")
        pylint_args = [
            *targets,
            "--recursive=n",
            "--disable=all",
            "--enable=duplicate-code,cyclic-import,locally-disabled",
            "--output-format=json2",
            "--reports=n",
            "--score=n",
            "--persistent=n",
            "--jobs=1",
            "--min-similarity-lines=5",
            "--ignore-comments=y",
            "--ignore-docstrings=y",
            "--ignore-imports=y",
            "--ignore-signatures=y",
            f"--rcfile={rcfile}",
        ]
        arguments = Path(temp_dir) / "arguments.json"
        from .git_scope import effective_source_roots
        source_roots = [str(root / path) for path in effective_source_roots(root, policy)]
        if source_roots:
            pylint_args.append('--source-roots=' + ','.join(source_roots))
        arguments.write_text(json.dumps({'args': pylint_args, 'source_roots': source_roots}), encoding="utf-8")
        command = [sys.executable, "-I", str(Path(__file__).resolve().parents[1] / "pylint_entry.py"), str(arguments)]
        environment = os.environ.copy()
        environment["PYTHONDONTWRITEBYTECODE"] = "1"
        environment["PYTHONPYCACHEPREFIX"] = str(Path(temp_dir) / "pycache")
        completed = run_bounded(
            command,
            cwd=root,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            env=environment,
            timeout=policy.get("subprocess_timeout_seconds", 60),
            check=False,
        )
    try:
        payload = json.loads(completed.stdout)
    except json.JSONDecodeError as exc:
        result["tool"]["status"] = "FAILED"
        result["tool"]["reason"] = f"invalid JSON2 output: {exc}"
        result["errors"].append(
            {
                "code": "PYLINT_OUTPUT_INVALID",
                "message": f"{exc}; stderr={completed.stderr.strip()[:500]}",
            }
        )
        return result
    if not isinstance(payload, dict) or not isinstance(payload.get("messages"), list):
        result["tool"]["status"] = "FAILED"
        result["tool"]["reason"] = "JSON2 output requires an object with a messages array"
        result["errors"].append(
            {"code": "PYLINT_OUTPUT_INVALID", "message": result["tool"]["reason"]}
        )
        return result
    messages = payload["messages"]
    if any(not isinstance(message, dict) for message in messages):
        result["tool"]["status"] = "FAILED"
        result["tool"]["reason"] = "JSON2 messages must be objects"
        result["errors"].append(
            {"code": "PYLINT_OUTPUT_INVALID", "message": result["tool"]["reason"]}
        )
        return result
    fatal_messages = [
        message
        for message in messages
        if message.get("type") in {"fatal", "error"}
        or str(message.get("messageId", "")).startswith(("F", "E"))
    ]
    if completed.returncode & (1 | 2 | 32) or fatal_messages:
        details = "; ".join(
            f"{message.get('messageId', 'unknown')}: {message.get('message', '')}"
            for message in fatal_messages[:5]
        )
        if not details:
            details = completed.stderr.strip()[:500] or f"exit code {completed.returncode}"
        result["tool"]["status"] = "FAILED"
        result["tool"]["reason"] = details
        result["errors"].append({"code": "PYLINT_FATAL", "message": details})
        return result

    modules = _module_map(files)
    from pylint.checkers.symilar import stripped_lines
    eligible = {}
    try:
        for path in targets:
            with tokenize.open(root / path) as handle:
                lines = handle.readlines()
            eligible[path] = {int(line.line_number) + 1 for line in stripped_lines(lines, True, True, True, True)}
    except Exception as exc:
        result["tool"].update(status="FAILED", reason=str(exc))
        result["errors"].append({"code": "PYLINT_LINE_EVIDENCE_FAILED", "message": str(exc)})
        return result
    analyzer = {"name": "pylint", "version": actual or "unknown"}
    duplicate_line_sets: dict[str, set[int]] = {}
    duplicate_groups = 0
    cycle_measurements = 0
    for message in messages:
        message_id = message.get("messageId")
        if message_id == "R0801":
            locations = _duplicate_locations(message.get("message", ""), modules)
            if not locations:
                result["errors"].append(
                    {"code": "PYLINT_DUPLICATE_RANGE_MISSING", "message": message.get("message", "")}
                )
                result["tool"]["status"] = "PARTIAL"
                continue
            duplicate_groups += 1
            for location in locations:
                location["lines"] = sorted(eligible[location["path"]] & set(range(location["start"], location["end"] + 1)))
            signature = sha256_text(canonical_json(locations))
            first = locations[0]
            block_lines = max(len(location["lines"]) for location in locations)
            for location in locations:
                duplicate_line_sets.setdefault(location["path"], set()).update(location["lines"])
            result["measurements"].append(
                make_measurement(
                    metric="duplication_block_lines",
                    value=block_lines,
                    path=first["path"],
                    symbol=f"duplicate:{signature[:12]}",
                    start_line=first["start"],
                    end_line=first["end"],
                    analyzer=analyzer,
                    evidence_fingerprint=signature,
                    evidence={"locations": locations},
                    policy=policy,
                )
            )
        elif message_id == "R0401":
            match = re.search(r"Cyclic import \((.+)\)", message.get("message", ""))
            if not match:
                result["errors"].append(
                    {"code": "PYLINT_CYCLE_PARSE_FAILED", "message": message.get("message", "")}
                )
                result["tool"]["status"] = "PARTIAL"
                continue
            cycle = [item.strip() for item in match.group(1).split("->") if item.strip()]
            normalized_cycle = sorted(set(cycle))
            if not normalized_cycle or any(module not in modules for module in normalized_cycle):
                result["tool"]["status"] = "PARTIAL"
                result["errors"].append({"code": "PYLINT_CYCLE_PATH_UNRESOLVED", "message": message.get("message", "")})
                continue
            signature = sha256_text(canonical_json(normalized_cycle))
            result["cycle_signatures"].append(signature)
            cycle_measurements += 1
            first_path = modules[normalized_cycle[0]]
            evidence_fingerprint = sha256_text(canonical_json({"modules": normalized_cycle}))
            result["measurements"].append(
                make_measurement(
                    metric="dependency_cycle",
                    value=len(normalized_cycle),
                    path=first_path,
                    symbol="cycle:" + "->".join(normalized_cycle),
                    start_line=None,
                    end_line=None,
                    analyzer=analyzer,
                    evidence_fingerprint=evidence_fingerprint,
                    evidence={"modules": normalized_cycle, "paths": sorted({modules[m] for m in normalized_cycle}), "cycle_signature": signature},
                    policy=policy,
                )
            )

    total_nloc = sum(len(lines) for lines in eligible.values())
    duplicated_lines = sum(len(lines) for lines in duplicate_line_sets.values())
    duplication_percent = (duplicated_lines / total_nloc * 100) if total_nloc else 0.0
    duplicate_fingerprint = sha256_text(
        canonical_json({path: sorted(lines) for path, lines in sorted(duplicate_line_sets.items())})
    )
    result["measurements"].append(
        make_measurement(
            metric="duplication_percent",
            value=duplication_percent,
            path=".",
            symbol="<repository>",
            start_line=None,
            end_line=None,
            analyzer=analyzer,
            evidence_fingerprint=duplicate_fingerprint,
            evidence={
                "duplicate_groups": duplicate_groups,
                "duplicated_lines": duplicated_lines,
                "eligible_lines_count": total_nloc,
                "eligible_lines": {path: sorted(lines) for path, lines in sorted(eligible.items())},
                "duplicated_line_sets": {path: sorted(lines) for path, lines in sorted(duplicate_line_sets.items())},
                "algorithm": "pylint_significant_lines_v2",
            },
            policy=policy,
        )
    )
    if cycle_measurements == 0:
        result["measurements"].append(
            make_measurement(
                metric="dependency_cycle",
                value=0,
                path=".",
                symbol="<repository>",
                start_line=None,
                end_line=None,
                analyzer=analyzer,
                evidence_fingerprint=sha256_text("no-cycles"),
                evidence={"modules": [], "cycle_signature": None},
                policy=policy,
            )
        )

    suppressions = _scan_suppressions(root, files, ast_index, policy)
    if suppressions:
        result["measurements"].extend(suppressions)
        result["tool"]["status"] = "PARTIAL"
        result["tool"]["reason"] = "required analyzer suppression directives were found"
        result["errors"].append(
            {
                "code": "REQUIRED_SUPPRESSION_FOUND",
                "message": f"{len(suppressions)} required analyzer suppression directive(s) found",
            }
        )
    if result["tool"]["status"] != "OK":
        # Suppressed or partially parsed global analysis cannot certify absent findings.
        result["measurements"] = [m for m in result["measurements"] if m["metric"] == "suppression"]
        result["cycle_signatures"] = []
    return result


def run_coverage(
    root: Path,
    files: list[str],
    changed_lines: dict[str, list[int]],
    mode: str,
    artifact: Path | None,
    ast_index: dict[str, dict[str, Any]],
    policy: dict[str, Any],
    expected_version: str,
    provenance: Path | None = None,
) -> dict[str, Any]:
    result: dict[str, Any] = {
        "measurements": [],
        "errors": [],
        "tool": _tool("coverage", expected_version, None, "NOT_RUN", "coverage JSON was not supplied"),
        "artifact_sha256": None,
    }
    if artifact is None:
        return result
    artifact = artifact.resolve()
    if not artifact.is_file():
        result["tool"]["reason"] = f"coverage JSON does not exist: {artifact}"
        result["errors"].append({"code": "COVERAGE_NOT_FOUND", "message": result["tool"]["reason"]})
        return result
    try:
        payload = json.loads(artifact.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as exc:
        return _fail_coverage(result, "COVERAGE_INVALID", str(exc))
    if not isinstance(payload, dict):
        return _fail_coverage(result, "COVERAGE_INVALID", "coverage JSON root must be an object")
    meta = payload.get("meta")
    coverage_payload_files = payload.get("files")
    if not isinstance(meta, dict) or not isinstance(coverage_payload_files, dict):
        return _fail_coverage(
            result, "COVERAGE_INVALID", "coverage JSON requires meta and files objects"
        )
    if meta.get("format") != 3 or not isinstance(meta.get("branch_coverage"), bool):
        return _fail_coverage(
            result,
            "COVERAGE_INVALID",
            "coverage metadata must use JSON format 3 and declare branch_coverage",
        )
    actual = str(meta.get("version", "unknown"))
    result["tool"]["actual_version"] = actual
    if actual != expected_version:
        result["tool"]["status"] = "VERSION_MISMATCH"
        result["tool"]["reason"] = f"expected {expected_version}, found {actual}"
        result["errors"].append({"code": "COVERAGE_VERSION_MISMATCH", "message": result["tool"]["reason"]})
        return result
    result["artifact_sha256"] = sha256_file(artifact)

    coverage_files = {
        relative_path(root, path): data for path, data in coverage_payload_files.items()
    }
    analyzer = {"name": "coverage", "version": actual}
    matched: list[tuple[str, dict[str, Any]]] = []
    missing: list[str] = []
    for relative in files:
        if source_kind(relative, policy) != "production" or not relative.endswith(".py"):
            continue
        data = coverage_files.get(relative)
        if data is None:
            missing.append(relative)
            continue
        problem = _coverage_file_problem(data)
        if problem:
            return _fail_coverage(
                result, "COVERAGE_INVALID", f"{relative}: {problem}"
            )
        matched.append((relative, data))

    if missing:
        result["errors"].append(
            {
                "code": "COVERAGE_FILES_MISSING",
                "message": "coverage has no evidence for: " + ", ".join(missing[:20]),
            }
        )
        result["tool"]["status"] = "PARTIAL"
        result["tool"]["reason"] = f"{len(missing)} production file(s) missing from coverage"
    else:
        result["tool"]["status"] = "OK"
        result["tool"]["reason"] = ""
    if meta["branch_coverage"] is not True:
        result["errors"].append(
            {
                "code": "COVERAGE_BRANCH_NOT_MEASURED",
                "message": "coverage artifact was generated without branch measurement",
            }
        )
        result["tool"]["status"] = "PARTIAL"
        result["tool"]["reason"] = "branch coverage was not measured"

    problem = validate_provenance(root, artifact, provenance, policy)
    if problem:
        result["tool"]["status"] = "UNVERIFIED"
        result["tool"]["reason"] = problem
        result["errors"].append({"code": "COVERAGE_PROVENANCE_UNVERIFIED", "message": problem})
        return result

    result["project_runtime"] = json.loads(provenance.read_text(encoding="utf-8"))["project_runtime"]
    total_statements = 0
    total_covered = 0
    total_branches = 0
    total_covered_branches = 0
    for relative, data in matched:
        executed = set(int(line) for line in data.get("executed_lines", []))
        missing_lines_set = set(int(line) for line in data.get("missing_lines", []))
        if mode == "diff":
            changed = set(changed_lines.get(relative, []))
            executable = executed | missing_lines_set
            scoped_executable = executable & changed
            covered = executed & scoped_executable
            statements = len(scoped_executable)
            covered_count = len(covered)
            percent = 100.0 if statements == 0 else covered_count / statements * 100
            evidence = {
                "changed_executable_lines": sorted(scoped_executable),
                "covered_changed_lines": sorted(covered),
            }
        else:
            summary = data.get("summary", {})
            statements = int(summary.get("num_statements", 0))
            covered_count = int(summary.get("covered_lines", 0))
            percent = 100.0 if statements == 0 else covered_count / statements * 100
            evidence = {
                "executed_lines": sorted(executed),
                "missing_lines": sorted(missing_lines_set),
            }
        total_statements += statements
        total_covered += covered_count
        fingerprint = ast_index.get(relative, {}).get("file_fingerprint", sha256_text(relative))
        result["measurements"].append(
            make_measurement(
                metric="line_coverage",
                value=percent,
                path=relative,
                symbol="<module>",
                start_line=1,
                end_line=None,
                analyzer=analyzer,
                evidence_fingerprint=sha256_text(canonical_json({"ast": fingerprint, **evidence})),
                evidence=evidence,
                policy=policy,
            )
        )
        summary = data.get("summary", {})
        branches = int(summary.get("num_branches", 0))
        if mode == "full" and branches:
            covered_branches = int(summary.get("covered_branches", 0))
            branch_percent = covered_branches / branches * 100
            total_branches += branches
            total_covered_branches += covered_branches
            result["measurements"].append(
                make_measurement(
                    metric="branch_coverage",
                    value=branch_percent,
                    path=relative,
                    symbol="<module>",
                    start_line=1,
                    end_line=None,
                    analyzer=analyzer,
                    evidence_fingerprint=sha256_text(
                        canonical_json(
                            {
                                "executed_branches": data.get("executed_branches", []),
                                "missing_branches": data.get("missing_branches", []),
                            }
                        )
                    ),
                    evidence={
                        "covered_branches": covered_branches,
                        "num_branches": branches,
                    },
                    policy=policy,
                )
            )

    if matched and not missing:
        total_percent = 100.0 if total_statements == 0 else total_covered / total_statements * 100
        result["measurements"].append(
            make_measurement(
                metric="line_coverage",
                value=total_percent,
                path=".",
                symbol="<repository>",
                start_line=None,
                end_line=None,
                analyzer=analyzer,
                evidence_fingerprint=sha256_text(
                    canonical_json({"covered": total_covered, "statements": total_statements})
                ),
                evidence={"covered_lines": total_covered, "num_statements": total_statements},
                policy=policy,
            )
        )
        if mode == "full" and total_branches:
            result["measurements"].append(
                make_measurement(
                    metric="branch_coverage",
                    value=total_covered_branches / total_branches * 100,
                    path=".",
                    symbol="<repository>",
                    start_line=None,
                    end_line=None,
                    analyzer=analyzer,
                    evidence_fingerprint=sha256_text(
                        canonical_json({"covered": total_covered_branches, "branches": total_branches})
                    ),
                    evidence={"covered_branches": total_covered_branches, "num_branches": total_branches},
                    policy=policy,
                )
            )
    return result
