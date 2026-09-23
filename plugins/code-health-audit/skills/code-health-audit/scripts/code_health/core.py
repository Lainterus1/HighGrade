from __future__ import annotations

import ast
import copy
import hashlib
import fnmatch
import json
from pathlib import Path
from typing import Any


SEVERITY = {"OK": 0, "WATCH": 1, "REFACTOR": 2, "CRITICAL": 3}
PYTHON_AST_NESTING_ANALYZER = {
    "name": "code-health-audit-python-ast",
    "version": "1.1.0",
}


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_text(value: str) -> str:
    return sha256_bytes(value.encode("utf-8"))


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def canonical_json(value: Any) -> str:
    return json.dumps(value, ensure_ascii=False, separators=(",", ":"), sort_keys=True)


def content_digest(report: dict[str, Any]) -> str:
    stable = copy.deepcopy(report)
    audit = stable.get("audit", {})
    audit.pop("root", None)
    audit.pop("generated_at", None)
    audit.pop("content_digest", None)
    stable.pop("artifacts", None)
    stable.get("runtime", {}).pop("executable", None)
    return sha256_text(canonical_json(stable))


def normalize_relative(path: str | Path) -> str:
    return str(path).replace("\\", "/").removeprefix("./")


def relative_path(root: Path, path: str | Path) -> str:
    candidate = Path(path)
    if not candidate.is_absolute():
        return normalize_relative(candidate)
    try:
        return normalize_relative(candidate.resolve().relative_to(root.resolve()))
    except ValueError:
        return normalize_relative(candidate)


def matches_path(path: str, patterns: list[str]) -> bool:
    path = normalize_relative(path)
    return any(pattern == '.' or path == pattern or path.startswith(pattern.rstrip('/') + '/')
               or fnmatch.fnmatchcase(path, pattern) for pattern in patterns)


def source_kind(path: str, policy: dict[str, Any] | None = None) -> str:
    if policy is not None and "test_paths" in policy:
        return "test" if matches_path(path, policy["test_paths"]) else "production"
    normalized = normalize_relative(path).lower()
    parts = normalized.split("/")
    name = parts[-1]
    if "tests" in parts or "test" in parts or name.startswith("test_") or name.endswith("_test.py"):
        return "test"
    return "production"


def classify(metric: str, value: float, policy: dict[str, Any]) -> tuple[str, dict[str, Any]]:
    rule = policy["metrics"][metric]
    direction = rule["direction"]
    if direction == "high":
        if value >= rule["critical"]:
            result = "CRITICAL"
        elif value >= rule["refactor"]:
            result = "REFACTOR"
        elif value >= rule["watch"]:
            result = "WATCH"
        else:
            result = "OK"
    elif direction == "low":
        if value < rule["critical"]:
            result = "CRITICAL"
        elif value < rule["refactor"]:
            result = "REFACTOR"
        elif value < rule["watch"]:
            result = "WATCH"
        else:
            result = "OK"
    else:
        raise ValueError(f"Unsupported metric direction: {direction}")
    thresholds = {key: rule[key] for key in ("direction", "watch", "refactor", "critical")}
    return result, thresholds


def make_measurement(
    *,
    metric: str,
    value: float,
    path: str,
    symbol: str,
    start_line: int | None,
    end_line: int | None,
    analyzer: dict[str, str],
    evidence_fingerprint: str,
    evidence: dict[str, Any],
    policy: dict[str, Any],
) -> dict[str, Any]:
    normalized_path = normalize_relative(path)
    classification, thresholds = classify(metric, value, policy)
    identity = sha256_text(canonical_json([metric, normalized_path, symbol]))[:20]
    return {
        "measurement_id": f"{metric}:{identity}",
        "metric": metric,
        "classification": classification,
        "value": round(float(value), 6),
        "unit": policy["metrics"][metric]["unit"],
        "path": normalized_path,
        "symbol": symbol,
        "start_line": start_line,
        "end_line": end_line,
        "source_kind": source_kind(normalized_path, policy),
        "policy_sha256": sha256_text(canonical_json(policy)),
        "analyzer": analyzer,
        "thresholds": thresholds,
        "evidence_fingerprint": evidence_fingerprint,
        "evidence": evidence,
    }


def worst_classification(values: list[str]) -> str:
    return max(values or ["OK"], key=lambda item: SEVERITY[item])


def deep_merge(base: dict[str, Any], override: dict[str, Any]) -> dict[str, Any]:
    result = copy.deepcopy(base)
    for key, value in override.items():
        if isinstance(value, dict) and isinstance(result.get(key), dict):
            result[key] = deep_merge(result[key], value)
        else:
            result[key] = copy.deepcopy(value)
    return result


class _ControlFlowNestingVisitor(ast.NodeVisitor):
    """Measure statement-level control-flow nesting inside one function.

    A top-level control statement has depth 1. ``elif``/``else`` branches stay
    at the depth of their owning ``if``; exception handlers and ``finally``
    stay at the depth of their owning ``try``. Nested functions and classes
    are measured independently by ``_SymbolVisitor``.
    """

    def __init__(self) -> None:
        self.depth = 0
        self.max_depth = 0

    def _visit_blocks(self, *blocks: list[ast.stmt]) -> None:
        self.depth += 1
        self.max_depth = max(self.max_depth, self.depth)
        for block in blocks:
            for statement in block:
                self.visit(statement)
        self.depth -= 1

    def visit_If(self, node: ast.If) -> None:
        self.depth += 1
        self.max_depth = max(self.max_depth, self.depth)
        for statement in node.body:
            self.visit(statement)

        alternate = node.orelse
        while (len(alternate) == 1 and isinstance(alternate[0], ast.If)
               and alternate[0].col_offset == node.col_offset):
            chained = alternate[0]
            for statement in chained.body:
                self.visit(statement)
            alternate = chained.orelse
        for statement in alternate:
            self.visit(statement)
        self.depth -= 1

    def visit_For(self, node: ast.For) -> None:
        self._visit_blocks(node.body, node.orelse)

    def visit_AsyncFor(self, node: ast.AsyncFor) -> None:
        self._visit_blocks(node.body, node.orelse)

    def visit_While(self, node: ast.While) -> None:
        self._visit_blocks(node.body, node.orelse)

    def visit_Try(self, node: ast.Try) -> None:
        self._visit_blocks(
            node.body,
            *(handler.body for handler in node.handlers),
            node.orelse,
            node.finalbody,
        )

    def visit_TryStar(self, node: ast.TryStar) -> None:
        self.visit_Try(node)

    def visit_With(self, node: ast.With) -> None:
        self._visit_blocks(node.body)

    def visit_AsyncWith(self, node: ast.AsyncWith) -> None:
        self._visit_blocks(node.body)

    def visit_Match(self, node: ast.Match) -> None:
        self._visit_blocks(*(case.body for case in node.cases))

    def visit_FunctionDef(self, node: ast.FunctionDef) -> None:
        return

    def visit_AsyncFunctionDef(self, node: ast.AsyncFunctionDef) -> None:
        return

    def visit_ClassDef(self, node: ast.ClassDef) -> None:
        return

    def visit_Lambda(self, node: ast.Lambda) -> None:
        return


def _function_nesting_depth(node: ast.FunctionDef | ast.AsyncFunctionDef) -> int:
    visitor = _ControlFlowNestingVisitor()
    for statement in node.body:
        visitor.visit(statement)
    return visitor.max_depth


class _SymbolVisitor(ast.NodeVisitor):
    def __init__(self) -> None:
        self.stack: list[str] = []
        self.symbols: list[dict[str, Any]] = []
        self.occurrences: dict[str, int] = {}

    def visit_ClassDef(self, node: ast.ClassDef) -> None:
        self.stack.append(node.name)
        self.generic_visit(node)
        self.stack.pop()

    def visit_FunctionDef(self, node: ast.FunctionDef) -> None:
        self._record(node)

    def visit_AsyncFunctionDef(self, node: ast.AsyncFunctionDef) -> None:
        self._record(node)

    def _record(self, node: ast.FunctionDef | ast.AsyncFunctionDef) -> None:
        qualified = ".".join([*self.stack, node.name])
        self.occurrences[qualified] = self.occurrences.get(qualified, 0) + 1
        if self.occurrences[qualified] > 1:
            qualified += f"#{self.occurrences[qualified]}"
        fingerprint = sha256_text(ast.dump(node, annotate_fields=True, include_attributes=False))
        self.symbols.append(
            {
                "symbol": qualified,
                "start_line": node.lineno,
                "end_line": node.end_lineno or node.lineno,
                "fingerprint": fingerprint,
                "nesting_depth": _function_nesting_depth(node),
            }
        )
        self.stack.append(node.name)
        self.generic_visit(node)
        self.stack.pop()


def build_ast_index(root: Path, files: list[str]) -> tuple[dict[str, dict[str, Any]], list[dict[str, str]]]:
    index: dict[str, dict[str, Any]] = {}
    errors: list[dict[str, str]] = []
    for relative in files:
        path = root / relative
        try:
            source = path.read_text(encoding="utf-8")
            tree = ast.parse(source, filename=relative)
        except (OSError, UnicodeError, SyntaxError) as exc:
            errors.append({"code": "AST_PARSE_FAILED", "message": f"{relative}: {exc}"})
            continue
        visitor = _SymbolVisitor()
        visitor.visit(tree)
        file_fingerprint = sha256_text(ast.dump(tree, annotate_fields=True, include_attributes=False))
        index[relative] = {"symbols": visitor.symbols, "file_fingerprint": file_fingerprint}
    return index, errors


def symbol_at(index: dict[str, dict[str, Any]], path: str, start_line: int | None, fallback: str) -> dict[str, Any]:
    entry = index.get(normalize_relative(path), {})
    symbols = entry.get("symbols", [])
    if start_line is not None:
        exact = [item for item in symbols if item["start_line"] == start_line]
        if exact:
            return exact[0]
        containing = [
            item
            for item in symbols
            if item["start_line"] <= start_line <= item["end_line"]
        ]
        if containing:
            return min(containing, key=lambda item: item["end_line"] - item["start_line"])
    return {
        "symbol": fallback,
        "start_line": start_line,
        "end_line": start_line,
        "fingerprint": entry.get("file_fingerprint", sha256_text(normalize_relative(path))),
    }
