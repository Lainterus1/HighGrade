from __future__ import annotations

from collections import Counter
from typing import Any

from .core import SEVERITY, sha256_text, worst_classification


def build_hotspots(findings: list[dict[str, Any]]) -> list[dict[str, Any]]:
    grouped: dict[tuple[str, str], list[dict[str, Any]]] = {}
    for finding in findings:
        if not finding.get("active"):
            continue
        grouped.setdefault((finding["path"], finding["symbol"]), []).append(finding)
    hotspots: list[dict[str, Any]] = []
    for (path, symbol), items in grouped.items():
        classification = worst_classification([item["classification"] for item in items])
        related_files: set[str] = set()
        for item in items:
            for location in item.get("evidence", {}).get("locations", []):
                related_files.add(location["path"])
            related_files.update(item.get("evidence", {}).get("paths", []))
        hotspot_id = "hotspot:" + sha256_text(f"{path}\0{symbol}")[:20]
        hotspots.append(
            {
                "hotspot_id": hotspot_id,
                "classification": classification,
                "path": path,
                "symbol": symbol,
                "start_line": min(
                    (item["start_line"] for item in items if item.get("start_line") is not None),
                    default=None,
                ),
                "end_line": max(
                    (item["end_line"] for item in items if item.get("end_line") is not None),
                    default=None,
                ),
                "finding_ids": sorted(item["finding_id"] for item in items),
                "metrics": sorted({item["metric"] for item in items}),
                "related_files": sorted(related_files - {path}),
            }
        )
    return sorted(
        hotspots,
        key=lambda item: (-SEVERITY[item["classification"]], item["path"], item["symbol"]),
    )


def build_summary(
    measurements: list[dict[str, Any]],
    findings: list[dict[str, Any]],
    hotspots: list[dict[str, Any]],
    accepted_results: list[dict[str, Any]],
) -> dict[str, Any]:
    classification_counts = Counter(item["classification"] for item in measurements)
    active_counts = Counter(
        item["classification"] for item in findings if item.get("active")
    )
    metric_counts: dict[str, dict[str, int]] = {}
    for measurement in measurements:
        metric = measurement["metric"]
        metric_counts.setdefault(metric, {key: 0 for key in SEVERITY})
        metric_counts[metric][measurement["classification"]] += 1
    return {
        "classification_counts": {key: classification_counts.get(key, 0) for key in SEVERITY},
        "active_finding_counts": {key: active_counts.get(key, 0) for key in SEVERITY},
        "active_hotspots": len(hotspots),
        "accepted_count": sum(item["status"] == "ACCEPTED" for item in accepted_results),
        "reopened_count": sum(item["status"] == "REOPENED" for item in accepted_results),
        "orphaned_acceptance_count": sum(item["status"] == "ORPHANED" for item in accepted_results),
        "metrics": {key: metric_counts[key] for key in sorted(metric_counts)},
    }


def verdict(completeness: str, health: str) -> str:
    if completeness == "FAILED":
        return "FAILED"
    if completeness != "COMPLETE":
        return "INCOMPLETE"
    return {
        "OK": "HEALTHY",
        "WATCH": "ATTENTION",
        "REFACTOR": "ACTION_REQUIRED",
        "CRITICAL": "CRITICAL",
    }[health]


def render_markdown(report: dict[str, Any]) -> str:
    audit = report["audit"]
    summary = report["summary"]
    lines = [
        "# Code health audit",
        "",
        f"- Mode: `{audit['mode']}`",
        f"- Completeness: `{audit['completeness']}`",
        f"- Health: `{audit['health']}`",
        f"- Verdict: `{audit['verdict']}`",
        f"- Content digest: `{audit['content_digest']}`",
        f"- Active hotspots: `{summary['active_hotspots']}`",
        f"- Accepted unchanged: `{summary['accepted_count']}`",
        f"- Reopened: `{summary['reopened_count']}`",
        "",
        "## Active findings",
        "",
    ]
    active = [finding for finding in report["findings"] if finding.get("active")]
    if not active:
        lines.append("No active findings.")
    else:
        lines.append("| Class | Metric | Value | Subject |")
        lines.append("|---|---|---:|---|")
        for finding in sorted(
            active,
            key=lambda item: (
                -SEVERITY[item["classification"]],
                item["path"],
                item["symbol"],
                item["metric"],
            ),
        ):
            subject = f"{finding['path']}::{finding['symbol']}"
            lines.append(
                f"| {finding['classification']} | {finding['metric']} | {finding['value']} {finding['unit']} | `{subject}` |"
            )
    lines.extend(["", "## Evidence completeness", ""])
    for tool in report["toolchain"]:
        reason = f" — {tool['reason']}" if tool.get("reason") else ""
        lines.append(
            f"- `{tool['name']}` {tool.get('actual_version') or 'not available'}: `{tool['status']}`{reason}"
        )
    if report["errors"]:
        lines.extend(["", "## Errors and policy events", ""])
        for error in report["errors"]:
            lines.append(f"- `{error['code']}` — {error['message']}")
    lines.append("")
    return "\n".join(lines)
