#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import sys
import tempfile
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

SCRIPT_DIR = Path(__file__).resolve().parent
if str(SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIR))

from code_health import REPORT_SCHEMA_VERSION
from code_health.acceptance import apply_acceptance, load_trusted_acceptance
from code_health.adapters import run_complexipy, run_coverage, run_lizard, run_pylint
from code_health.execution import run_adapter
from code_health.core import (
    SEVERITY,
    build_ast_index,
    canonical_json,
    content_digest,
    deep_merge,
    make_measurement,
    sha256_file,
    sha256_text,
    worst_classification,
)
from code_health.git_scope import (
    ACCEPTANCE_PATH,
    GitScopeError,
    compare_snapshots,
    discover_python_files,
    is_git_repository,
    materialize_archive,
    resolve_main_diff,
    snapshot_repository,
)
from code_health.validation import load_policy, validate_report
from code_health.runtime import runtime_evidence, preflight
from code_health.git_scope import discover_scope, effective_source_roots
from code_health.render import build_hotspots, build_summary, render_markdown, verdict


DATA_DIR = SCRIPT_DIR / "data"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Deterministic read-only Python code-health audit"
    )
    parser.add_argument("--preflight", action="store_true", help="Check interpreter, tool versions, scope and local diff base; do not run tests/analyzers")
    parser.add_argument("--root", required=True, type=Path, help="Python repository root")
    parser.add_argument("--mode", required=True, choices=("full", "diff"))
    parser.add_argument("--base", default="main", help="Local diff base name (default: main)")
    parser.add_argument("--coverage-json", type=Path, help="Fresh Coverage.py JSON artifact")
    parser.add_argument("--coverage-provenance", type=Path, help="Provenance from capture_coverage.py")
    parser.add_argument("--policy", type=Path, help="JSON policy override")
    parser.add_argument("--output-dir", type=Path, help="Output directory outside the audited root")
    return parser.parse_args()


def load_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def ensure_output(root: Path, requested: Path | None) -> Path:
    if requested is None:
        return Path(tempfile.mkdtemp(prefix="code-health-audit-"))
    output = requested.resolve()
    root_resolved = root.resolve()
    if output == root_resolved or root_resolved in output.parents:
        raise ValueError("output directory must be outside the audited repository")
    output.mkdir(parents=True, exist_ok=True)
    return output


def _intersects(measurement: dict[str, Any], changed_lines: dict[str, list[int]]) -> bool:
    path = measurement["path"]
    changed = set(changed_lines.get(path, []))
    if not changed:
        return False
    start = measurement.get("start_line")
    end = measurement.get("end_line")
    if start is None:
        return True
    end = end if end is not None else start
    return any(start <= line <= end for line in changed)


def filter_diff_measurements(
    measurements: list[dict[str, Any]], changed_lines: dict[str, list[int]],
    base_cycle_signatures: set[str], policy: dict[str, Any], pylint_version: str,
    pylint_complete: bool = False, base_complete: bool = False,
) -> list[dict[str, Any]]:
    scoped = []
    for measurement in measurements:
        metric = measurement["metric"]
        if metric in {"cognitive_complexity", "cyclomatic_complexity", "function_nloc", "nesting_depth", "suppression"}:
            if _intersects(measurement, changed_lines):
                scoped.append(measurement)
        elif metric == "file_nloc":
            if measurement["path"] in changed_lines:
                scoped.append(measurement)
        elif metric == "duplication_block_lines" and pylint_complete:
            locations = measurement.get("evidence", {}).get("locations", [])
            if any(set(changed_lines.get(loc["path"], [])) & set(loc.get("lines", [])) for loc in locations):
                scoped.append(measurement)
        elif metric == "dependency_cycle" and pylint_complete and base_complete:
            signature = measurement.get("evidence", {}).get("cycle_signature")
            if signature and signature not in base_cycle_signatures:
                scoped.append(measurement)
        elif metric in {"line_coverage", "branch_coverage"}:
            scoped.append(measurement)
    aggregate = next((m for m in measurements if m["metric"] == "duplication_percent"), None)
    analyzer = {"name": "pylint", "version": pylint_version}
    if pylint_complete and aggregate is not None:
        evidence = aggregate["evidence"]
        eligible = {(path, line) for path, lines in evidence["eligible_lines"].items()
                    for line in set(lines) & set(changed_lines.get(path, []))}
        duplicates = {(path, line) for path, lines in evidence["duplicated_line_sets"].items() for line in lines} & eligible
        scoped.append(make_measurement(
            metric="duplication_percent", value=len(duplicates) / len(eligible) * 100 if eligible else 0,
            path=".", symbol="<diff>", start_line=None, end_line=None, analyzer=analyzer,
            evidence_fingerprint=sha256_text(canonical_json(sorted(duplicates))),
            evidence={"duplicated_changed_lines": len(duplicates), "eligible_changed_lines": len(eligible), "algorithm": "pylint_significant_lines_v2"}, policy=policy))
    if pylint_complete and base_complete and not any(m["metric"] == "dependency_cycle" for m in scoped):
        scoped.append(make_measurement(
            metric="dependency_cycle", value=0, path=".", symbol="<diff>", start_line=None, end_line=None,
            analyzer=analyzer, evidence_fingerprint=sha256_text("no-new-cycles"),
            evidence={"modules": [], "paths": [], "cycle_signature": None, "relative_to_base": True}, policy=policy))
    return scoped


def sort_measurements(measurements: list[dict[str, Any]]) -> list[dict[str, Any]]:
    return sorted(
        measurements,
        key=lambda item: (
            item["path"],
            item["symbol"],
            item["metric"],
            item["measurement_id"],
        ),
    )


def run_audit(args: argparse.Namespace) -> int:
    root = args.root.resolve()
    if not root.is_dir():
        print(json.dumps({"error": f"repository root does not exist: {root}"}), file=sys.stderr)
        return 2
    try:
        output_dir = ensure_output(root, args.output_dir)
    except ValueError as exc:
        print(json.dumps({"error": str(exc)}), file=sys.stderr)
        return 2

    policy = load_policy(args.policy)
    runtime = runtime_evidence()
    toolchain = load_json(DATA_DIR / "toolchain.json")
    expected_tools = toolchain["tools"]
    policy_sha = sha256_text(canonical_json(policy))

    errors: list[dict[str, str]] = [{"code":"RUNTIME_UNVERIFIED", "message":message} for message in runtime["problems"]]
    tool_results: list[dict[str, Any]] = []
    measurements: list[dict[str, Any]] = []
    files, excluded_files = discover_scope(root, policy)
    before = snapshot_repository(root, files, policy)
    source_digest = sha256_text(
        canonical_json({relative: sha256_file(root / relative) for relative in files})
    )

    diff: dict[str, Any] = {
        "base_ref": None,
        "base_sha": None,
        "merge_base": None,
        "head_sha": None,
        "changed_lines": {},
        "changed_files": [],
        "untracked_files": [],
        "acceptance_changed": False,
        "worktree_diff_sha256": source_digest,
    }
    fatal = False
    if args.mode == "diff":
        try:
            diff = resolve_main_diff(root, args.base, files)
        except GitScopeError as exc:
            errors.append({"code": "NO_MAIN_BASE", "message": str(exc)})
            fatal = True

    ast_index, ast_errors = build_ast_index(root, files)
    errors.extend(ast_errors)

    lizard_result = run_adapter(
        "lizard", root, files, ast_index, policy, expected_tools["lizard"]
    )
    complexipy_result = run_adapter(
        "complexipy", root, files, ast_index, policy, expected_tools["complexipy"]
    )
    pylint_result = run_adapter(
        "pylint", root, files, ast_index, policy, expected_tools["pylint"],
        file_nloc=lizard_result.get("file_nloc", {}),
    )
    coverage_result = run_adapter(
        "coverage", root, files, ast_index, policy, expected_tools["coverage"],
        changed_lines=diff["changed_lines"], mode=args.mode,
        artifact=str(args.coverage_json.resolve()) if args.coverage_json else None,
        provenance=str(args.coverage_provenance.resolve()) if args.coverage_provenance else None,
    )
    for adapter in (lizard_result, complexipy_result, pylint_result, coverage_result):
        tool_results.append(adapter["tool"])
        errors.extend(adapter["errors"])
        measurements.extend(adapter["measurements"])

    base_cycle_signatures: set[str] = set()
    base_pylint_complete = False
    if args.mode == "diff" and not fatal:
        try:
            with tempfile.TemporaryDirectory(prefix="code-health-base-") as temp_dir:
                base_root = materialize_archive(root, diff["merge_base"], Path(temp_dir))
                base_files = discover_python_files(base_root, policy)
                base_ast, base_ast_errors = build_ast_index(base_root, base_files)
                if base_ast_errors:
                    errors.extend(
                        {"code": "BASE_" + item["code"], "message": item["message"]}
                        for item in base_ast_errors
                    )
                base_pylint = run_adapter(
                    "pylint", base_root, base_files, base_ast, policy, expected_tools["pylint"],
                    file_nloc={},
                )
                base_cycle_signatures = set(base_pylint["cycle_signatures"])
                base_pylint_complete = (base_pylint["tool"]["status"] == "OK" or not base_files) and not base_ast_errors
                if not base_pylint_complete:
                    errors.append(
                        {
                            "code": "BASE_PYLINT_FAILED",
                            "message": base_pylint["tool"].get("reason", "base Pylint failed"),
                        }
                    )
        except (GitScopeError, OSError) as exc:
            errors.append({"code": "BASE_ANALYSIS_FAILED", "message": str(exc)})
            fatal = True

    if args.mode == "diff":
        measurements = filter_diff_measurements(
            measurements,
            diff["changed_lines"],
            base_cycle_signatures,
            policy,
            expected_tools["pylint"],
            pylint_complete=pylint_result["tool"]["status"] == "OK",
            base_complete=base_pylint_complete and not fatal,
        )

    acceptance = load_trusted_acceptance(
        root,
        args.mode,
        diff.get("merge_base"),
        diff.get("acceptance_changed", False),
    )
    errors.extend(acceptance["errors"])
    if any(
        item["code"] in {"ACCEPTANCE_CHANGE_UNTRUSTED", "ACCEPTANCE_CURRENT_UNTRUSTED"}
        for item in acceptance["errors"]
    ):
        measurements.append(
            make_measurement(
                metric="suppression",
                value=1,
                path=ACCEPTANCE_PATH,
                symbol="<acceptance-registry>",
                start_line=1,
                end_line=1,
                analyzer={"name": "code-health-audit-policy", "version": policy["policy_version"]},
                evidence_fingerprint=sha256_text(
                    canonical_json([item["code"] for item in acceptance["errors"]])
                ),
                evidence={"trusted_source": acceptance["source"]},
                policy=policy,
            )
        )

    coverage_runtime = coverage_result.get('project_runtime')
    runtime_sha = sha256_text(canonical_json({'analyzers':runtime['sha256'],'coverage_project':coverage_runtime}))
    for measurement in measurements:
        measurement['runtime_sha256'] = runtime_sha
    measurements = sort_measurements(measurements)
    findings, accepted_results = apply_acceptance(
        root, measurements, acceptance["entries"], policy["policy_version"]
    )
    findings = sorted(
        findings,
        key=lambda item: (
            item["path"],
            item["symbol"],
            item["metric"],
            item["finding_id"],
        ),
    )
    hotspots = build_hotspots(findings)

    after = snapshot_repository(root, files, policy)
    breaches = compare_snapshots(before, after)
    if breaches:
        errors.append(
            {
                "code": "READ_ONLY_BREACH",
                "message": "audit changed repository evidence: " + ", ".join(breaches),
            }
        )
        fatal = True

    tool_results = sorted(tool_results, key=lambda item: item["name"])
    incomplete = any(tool["status"] != "OK" for tool in tool_results)
    incomplete = incomplete or bool(ast_errors) or not runtime["validated"]
    incomplete = incomplete or any(
        item["code"]
        in {
            "ACCEPTANCE_INVALID",
            "ACCEPTANCE_ENTRY_INVALID",
            "ACCEPTANCE_DUPLICATE_ID",
            "BASE_ANALYSIS_FAILED",
            "BASE_PYLINT_FAILED",
        }
        for item in errors
    )
    completeness = "FAILED" if fatal else ("INCOMPLETE" if incomplete else "COMPLETE")
    health = worst_classification(
        [item["classification"] for item in findings if item.get("active")]
    )
    summary = build_summary(measurements, findings, hotspots, accepted_results)

    report: dict[str, Any] = {
        "schema_version": REPORT_SCHEMA_VERSION,
        "audit": {
            "mode": args.mode,
            "root": str(root),
            "generated_at": datetime.now(timezone.utc).isoformat(),
            "completeness": completeness,
            "health": health,
            "verdict": verdict(completeness, health),
            "policy_version": policy["policy_version"],
            "policy_sha256": policy_sha,
            "content_digest": "",
            "source_sha256": source_digest,
            "coverage_sha256": coverage_result.get("artifact_sha256"),
            "read_only_verified": not breaches,
        },
        "toolchain": tool_results,
        "runtime": runtime,
        "coverage_project_runtime": coverage_runtime,
        "scope": {
            "python_files": files,
            "excluded_files": excluded_files,
            "source_roots": effective_source_roots(root, policy),
            "test_paths": policy["test_paths"],
            "include_paths": policy["include_paths"],
            "excluded_paths": policy["excluded_paths"],
            "excluded_directories": policy["excluded_directories"],
            "watch_hotspot_limit": policy["watch_hotspot_limit"],
            "diff_branch_coverage": "NOT_MEASURED" if args.mode == "diff" else "NOT_APPLICABLE",
            "base_ref": diff.get("base_ref"),
            "base_sha": diff.get("base_sha"),
            "merge_base": diff.get("merge_base"),
            "head_sha": diff.get("head_sha"),
            "worktree_diff_sha256": diff.get("worktree_diff_sha256"),
            "changed_files": diff.get("changed_files", []),
            "untracked_files": diff.get("untracked_files", []),
            "changed_lines": {
                path: lines for path, lines in sorted(diff.get("changed_lines", {}).items())
            },
            "acceptance_source": acceptance["source"],
            "acceptance_trusted": acceptance["trusted"],
        },
        "summary": summary,
        "measurements": measurements,
        "findings": findings,
        "hotspots": hotspots,
        "accepted_findings": accepted_results,
        "errors": sorted(errors, key=lambda item: (item["code"], item["message"])),
        "artifacts": {},
    }
    report["audit"]["content_digest"] = content_digest(report)
    validate_report(report)

    json_path = output_dir / "code-health-audit.json"
    markdown_path = output_dir / "code-health-audit.md"
    report["artifacts"] = {
        "json": str(json_path),
        "markdown": str(markdown_path),
    }
    json_path.write_text(
        json.dumps(report, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    markdown_path.write_text(render_markdown(report), encoding="utf-8")
    print(
        json.dumps(
            {
                "json": str(json_path),
                "markdown": str(markdown_path),
                "completeness": completeness,
                "health": health,
                "verdict": report["audit"]["verdict"],
                "content_digest": report["audit"]["content_digest"],
            },
            ensure_ascii=False,
            sort_keys=True,
        )
    )
    if any(item["code"] == "READ_ONLY_BREACH" for item in errors):
        return 3
    return 0 if completeness == "COMPLETE" else 2


def main() -> int:
    args = parse_args()
    root = args.root.resolve()
    if args.preflight:
        try:
            result = preflight(root,args.mode,args.base,load_policy(args.policy))
        except Exception as exc:
            result = {'ready':False,'errors':[f'{type(exc).__name__}: {exc}']}
        print(json.dumps(result,ensure_ascii=False,indent=2))
        return 0 if result['ready'] else 2
    before = None
    policy = None
    files: list[str] = []
    try:
        args.output_dir = ensure_output(root, args.output_dir)
        policy = load_policy(args.policy)
        files = discover_python_files(root, policy)
        before = snapshot_repository(root, files, policy)
        return run_audit(args)
    except Exception as exc:
        errors = [{"code": "AUDIT_FAILED", "message": f"{type(exc).__name__}: {exc}"}]
        verified = False
        try:
            after = snapshot_repository(root, files, policy)
            breaches = compare_snapshots(before, after) if before is not None else ["<initial-snapshot-unavailable>"]
            verified = not breaches
            if before is not None and breaches:
                errors.append({"code": "READ_ONLY_BREACH", "message": ", ".join(breaches)})
        except Exception as snapshot_exc:
            errors.append({"code": "SNAPSHOT_FAILED", "message": str(snapshot_exc)})
        # Even invalid input/configuration or infrastructure failures leave diagnostics,
        # provided an allowed output directory can be created.
        try:
            output = ensure_output(root, args.output_dir)
            policy = policy or load_policy()
            report = {"schema_version": REPORT_SCHEMA_VERSION,
                      "audit": {"root": str(root), "mode": args.mode,
                                "generated_at": datetime.now(timezone.utc).isoformat(),
                                "completeness": "FAILED", "health": "OK", "verdict": "FAILED",
                                "policy_version": policy["policy_version"],
                                "policy_sha256": sha256_text(canonical_json(policy)),
                                "read_only_verified": verified},
                      "scope": {"python_files": files, "changed_lines": {}},
                      "toolchain": [], "measurements": [], "findings": [], "hotspots": [],
                      "accepted_findings": [], "errors": errors,
                      "summary": build_summary([], [], [], [])}
            report["audit"]["content_digest"] = content_digest(report)
            validate_report(report)
            (output / "code-health-audit.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
            (output / "code-health-audit.md").write_text(render_markdown(report), encoding="utf-8")
            print(json.dumps({"verdict": "FAILED", "output": str(output), "errors": errors}, ensure_ascii=False))
        except Exception as output_exc:
            print(json.dumps({"verdict": "FAILED", "errors": errors, "output_error": str(output_exc)}, ensure_ascii=False), file=sys.stderr)
        return 3 if any(e["code"] == "READ_ONLY_BREACH" for e in errors) else 2


if __name__ == "__main__":
    raise SystemExit(main())
