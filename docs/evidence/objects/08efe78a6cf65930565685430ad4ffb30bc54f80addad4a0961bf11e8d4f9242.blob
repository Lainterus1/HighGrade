from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock


SCRIPT_DIR = Path(__file__).resolve().parents[1]
if str(SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIR))

from code_health.acceptance import _parse_registry, apply_acceptance
from code_health.adapters import run_coverage, run_lizard, run_pylint
from code_health.core import build_ast_index, classify, content_digest, make_measurement, sha256_file
from code_health.git_scope import acceptance_is_clean


POLICY = json.loads((SCRIPT_DIR / "data" / "default-policy.json").read_text(encoding="utf-8"))


class ClassificationTests(unittest.TestCase):
    def test_high_metric_boundaries(self) -> None:
        self.assertEqual(classify("nesting_depth", 3, POLICY)[0], "OK")
        self.assertEqual(classify("nesting_depth", 4, POLICY)[0], "WATCH")
        self.assertEqual(classify("nesting_depth", 5, POLICY)[0], "REFACTOR")
        self.assertEqual(classify("nesting_depth", 7, POLICY)[0], "CRITICAL")

    def test_low_metric_boundaries(self) -> None:
        self.assertEqual(classify("line_coverage", 80, POLICY)[0], "OK")
        self.assertEqual(classify("line_coverage", 79.99, POLICY)[0], "WATCH")
        self.assertEqual(classify("line_coverage", 69.99, POLICY)[0], "REFACTOR")
        self.assertEqual(classify("line_coverage", 49.99, POLICY)[0], "CRITICAL")


class FingerprintTests(unittest.TestCase):
    def test_formatting_and_comments_do_not_change_ast_fingerprint(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / "module.py"
            path.write_text("def decide(x):\n    # reason\n    return x + 1\n", encoding="utf-8")
            first, first_errors = build_ast_index(root, ["module.py"])
            path.write_text("def decide( x ):\n\n    return (x + 1)  # same behavior\n", encoding="utf-8")
            second, second_errors = build_ast_index(root, ["module.py"])
            self.assertEqual(first_errors, [])
            self.assertEqual(second_errors, [])
            self.assertEqual(
                first["module.py"]["symbols"][0]["fingerprint"],
                second["module.py"]["symbols"][0]["fingerprint"],
            )

    def test_semantic_change_reopens_fingerprint(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / "module.py"
            path.write_text("def decide(x):\n    return x + 1\n", encoding="utf-8")
            first, _ = build_ast_index(root, ["module.py"])
            path.write_text("def decide(x):\n    return x + 2\n", encoding="utf-8")
            second, _ = build_ast_index(root, ["module.py"])
            self.assertNotEqual(
                first["module.py"]["symbols"][0]["fingerprint"],
                second["module.py"]["symbols"][0]["fingerprint"],
            )


class PythonNestingTests(unittest.TestCase):
    def _depths(self, source: str) -> dict[str, int]:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "module.py").write_text(source, encoding="utf-8")
            index, errors = build_ast_index(root, ["module.py"])
        self.assertEqual(errors, [])
        return {
            symbol["symbol"]: symbol["nesting_depth"]
            for symbol in index["module.py"]["symbols"]
        }

    def test_sequential_branches_do_not_accumulate_depth(self) -> None:
        depths = self._depths(
            "def flat(value):\n"
            "    if value == 1:\n"
            "        return 1\n"
            "    if value == 2:\n"
            "        return 2\n"
            "    if value == 3:\n"
            "        return 3\n"
            "    return 0\n"
        )
        self.assertEqual(depths["flat"], 1)

    def test_elif_stays_flat_while_nested_blocks_increase_depth(self) -> None:
        depths = self._depths(
            "def decide(items):\n"
            "    if items is None:\n"
            "        return None\n"
            "    elif items:\n"
            "        for item in items:\n"
            "            try:\n"
            "                if item:\n"
            "                    return item\n"
            "            except ValueError:\n"
            "                pass\n"
            "    return None\n"
        )
        self.assertEqual(depths["decide"], 4)

    def test_nested_functions_are_measured_independently(self) -> None:
        depths = self._depths(
            "def outer(value):\n"
            "    if value:\n"
            "        def inner(other):\n"
            "            while other:\n"
            "                if other > 1:\n"
            "                    return other\n"
            "        return inner(value)\n"
            "    return None\n"
        )
        self.assertEqual(depths["outer"], 1)
        self.assertEqual(depths["outer.inner"], 2)

    @mock.patch("code_health.adapters._version_status", return_value=("1.24.0", "OK", ""))
    def test_lizard_adapter_ignores_mutable_ns_and_is_order_invariant(
        self, _version: mock.Mock
    ) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "a.py").write_text(
                "def flat(value):\n    if value:\n        return value\n    return None\n",
                encoding="utf-8",
            )
            (root / "b.py").write_text(
                "def nested(values):\n    for value in values:\n        if value:\n            return value\n    return None\n",
                encoding="utf-8",
            )
            ast_index, errors = build_ast_index(root, ["a.py", "b.py"])
            self.assertEqual(errors, [])

            mutable_ns = {"value": 0}

            def analyze(paths: list[str], **_kwargs: object) -> list[SimpleNamespace]:
                infos: list[SimpleNamespace] = []
                for path in paths:
                    mutable_ns["value"] += 100
                    infos.append(
                        SimpleNamespace(
                            filename=path,
                            nloc=5,
                            function_list=[
                                SimpleNamespace(
                                    name="ignored-fallback",
                                    start_line=1,
                                    end_line=5,
                                    nloc=5,
                                    cyclomatic_complexity=2,
                                    max_nested_structures=mutable_ns["value"],
                                )
                            ],
                        )
                    )
                return infos

            fake_lizard = SimpleNamespace(
                analyze=analyze,
                get_extensions=lambda _names: [object()],
            )
            with mock.patch.dict(sys.modules, {"lizard": fake_lizard}):
                first = run_lizard(root, ["a.py", "b.py"], ast_index, POLICY, "1.24.0")
                second = run_lizard(root, ["b.py", "a.py"], ast_index, POLICY, "1.24.0")

        def nesting(result: dict) -> dict[tuple[str, str], tuple[float, str]]:
            return {
                (item["path"], item["symbol"]): (item["value"], item["analyzer"]["name"])
                for item in result["measurements"]
                if item["metric"] == "nesting_depth"
            }

        expected = {
            ("a.py", "flat"): (1.0, "code-health-audit-python-ast"),
            ("b.py", "nested"): (2.0, "code-health-audit-python-ast"),
        }
        self.assertEqual(nesting(first), expected)
        self.assertEqual(nesting(second), expected)


class AcceptanceTests(unittest.TestCase):
    def _measurement(self, fingerprint: str = "ast-v1", value: float = 5) -> dict:
        return make_measurement(
            metric="nesting_depth",
            value=value,
            path="pkg/module.py",
            symbol="decide",
            start_line=1,
            end_line=20,
            analyzer={"name": "lizard", "version": "1.24.0"},
            evidence_fingerprint=fingerprint,
            evidence={"reason": "measured"},
            policy=POLICY,
        )

    def _entry(self, measurement: dict) -> dict:
        return {
            "finding_id": measurement["measurement_id"],
            "metric": measurement["metric"],
            "path": measurement["path"],
            "symbol": measurement["symbol"],
            "accepted_value": measurement["value"],
            "ceiling": measurement["value"],
            "reason": "The nesting mirrors an externally defined decision table.",
            "evidence_fingerprint": measurement["evidence_fingerprint"],
            "policy_version": POLICY["policy_version"],
            "policy_sha256": measurement["policy_sha256"],
            "analyzer": measurement["analyzer"],
        }

    def test_unchanged_justified_finding_is_inactive(self) -> None:
        measurement = self._measurement()
        findings, results = apply_acceptance(
            Path.cwd(), [measurement], [self._entry(measurement)], POLICY["policy_version"]
        )
        self.assertFalse(findings[0]["active"])
        self.assertEqual(findings[0]["disposition"], "ACCEPTED")
        self.assertEqual(results[0]["status"], "ACCEPTED")

    def test_changed_code_reopens_accepted_finding(self) -> None:
        accepted = self._measurement()
        changed = self._measurement(fingerprint="ast-v2")
        findings, results = apply_acceptance(
            Path.cwd(), [changed], [self._entry(accepted)], POLICY["policy_version"]
        )
        self.assertTrue(findings[0]["active"])
        self.assertEqual(findings[0]["reopen_reason"], "EVIDENCE_CHANGED")
        self.assertEqual(results[0]["status"], "REOPENED")

    def test_metric_regression_reopens_accepted_finding(self) -> None:
        accepted = self._measurement(value=5)
        regressed = self._measurement(value=7)
        findings, _ = apply_acceptance(
            Path.cwd(), [regressed], [self._entry(accepted)], POLICY["policy_version"]
        )
        self.assertEqual(findings[0]["reopen_reason"], "REGRESSION")

    def test_improvement_suggests_tighter_baseline(self) -> None:
        accepted = self._measurement(value=5)
        improved = self._measurement(value=4)
        findings, _ = apply_acceptance(
            Path.cwd(), [improved], [self._entry(accepted)], POLICY["policy_version"]
        )
        self.assertFalse(findings[0]["active"])
        self.assertTrue(findings[0]["baseline_can_tighten"])

    def test_policy_change_reopens_accepted_finding(self) -> None:
        measurement = self._measurement()
        entry = self._entry(measurement)
        entry["policy_version"] = "0.9.0"
        findings, _ = apply_acceptance(
            Path.cwd(), [measurement], [entry], POLICY["policy_version"]
        )
        self.assertEqual(findings[0]["reopen_reason"], "POLICY_CHANGED")

    def test_analyzer_change_reopens_accepted_finding(self) -> None:
        measurement = self._measurement()
        entry = self._entry(measurement)
        entry["analyzer"] = {"name": "lizard", "version": "1.23.0"}
        findings, _ = apply_acceptance(
            Path.cwd(), [measurement], [entry], POLICY["policy_version"]
        )
        self.assertEqual(findings[0]["reopen_reason"], "ANALYZER_CHANGED")

    def test_guard_change_reopens_accepted_finding(self) -> None:
        measurement = self._measurement()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            guard = root / "tests" / "test_contract.py"
            guard.parent.mkdir()
            guard.write_text("EXPECTED = 1\n", encoding="utf-8")
            entry = self._entry(measurement)
            entry["guard_files"] = [
                {"path": "tests/test_contract.py", "sha256": sha256_file(guard)}
            ]
            guard.write_text("EXPECTED = 2\n", encoding="utf-8")
            findings, _ = apply_acceptance(
                root, [measurement], [entry], POLICY["policy_version"]
            )
        self.assertEqual(findings[0]["reopen_reason"], "GUARD_CHANGED:tests/test_contract.py")

    def test_malformed_registry_root_fails_closed(self) -> None:
        entries, errors = _parse_registry(b"[]", "fixture")
        self.assertEqual(entries, [])
        self.assertEqual(errors[0]["code"], "ACCEPTANCE_INVALID")

    def test_non_numeric_ceiling_is_rejected(self) -> None:
        measurement = self._measurement()
        entry = self._entry(measurement)
        entry["ceiling"] = "not-a-number"
        payload = json.dumps({"schema_version": "1.0.0", "entries": [entry]}).encode()
        entries, errors = _parse_registry(payload, "fixture")
        self.assertEqual(entries, [])
        self.assertEqual(errors[0]["code"], "ACCEPTANCE_ENTRY_INVALID")


class TrustedRegistryTests(unittest.TestCase):
    def _git(self, root: Path, *args: str) -> None:
        subprocess.run(
            ["git", *args], cwd=root, check=True, capture_output=True, text=True
        )

    def test_ignored_untracked_registry_is_not_clean(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self._git(root, "init", "-b", "main")
            (root / ".gitignore").write_text(
                ".code-health-audit/accepted-findings.json\n", encoding="utf-8"
            )
            registry = root / ".code-health-audit" / "accepted-findings.json"
            registry.parent.mkdir()
            registry.write_text('{"schema_version":"1.0.0","entries":[]}\n', encoding="utf-8")
            self.assertFalse(acceptance_is_clean(root))

    def test_tracked_head_registry_is_clean(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self._git(root, "init", "-b", "main")
            self._git(root, "config", "user.name", "Fixture")
            self._git(root, "config", "user.email", "fixture@example.invalid")
            registry = root / ".code-health-audit" / "accepted-findings.json"
            registry.parent.mkdir()
            registry.write_text('{"schema_version":"1.0.0","entries":[]}\n', encoding="utf-8")
            self._git(root, "add", "--", ".code-health-audit/accepted-findings.json")
            self._git(root, "commit", "-m", "registry")
            self.assertTrue(acceptance_is_clean(root))


class AdapterFailureTests(unittest.TestCase):
    def test_malformed_coverage_file_fails_without_measurements(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            module = root / "pkg" / "a.py"
            module.parent.mkdir()
            module.write_text("VALUE = 1\n", encoding="utf-8")
            artifact = root.parent / f"{root.name}-coverage.json"
            self.addCleanup(artifact.unlink, missing_ok=True)
            artifact.write_text(
                json.dumps(
                    {
                        "meta": {
                            "format": 3,
                            "version": "7.15.4",
                            "timestamp": "2999-01-01T00:00:00",
                            "branch_coverage": True,
                        },
                        "files": {"pkg/a.py": {}},
                    }
                ),
                encoding="utf-8",
            )
            ast_index, _ = build_ast_index(root, ["pkg/a.py"])
            result = run_coverage(
                root,
                ["pkg/a.py"],
                {},
                "full",
                artifact,
                ast_index,
                POLICY,
                "7.15.4",
            )
        self.assertEqual(result["tool"]["status"], "FAILED")
        self.assertEqual(result["measurements"], [])
        self.assertEqual(result["errors"][0]["code"], "COVERAGE_INVALID")

    def test_forged_coverage_percentage_fails_without_measurements(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            module = root / "pkg" / "a.py"
            module.parent.mkdir()
            module.write_text("VALUE = 1\n", encoding="utf-8")
            artifact = root.parent / f"{root.name}-forged-coverage.json"
            self.addCleanup(artifact.unlink, missing_ok=True)
            artifact.write_text(
                json.dumps(
                    {
                        "meta": {
                            "format": 3,
                            "version": "7.15.4",
                            "timestamp": "2999-01-01T00:00:00",
                            "branch_coverage": True,
                        },
                        "files": {
                            "pkg/a.py": {
                                "executed_lines": [],
                                "missing_lines": [1],
                                "excluded_lines": [],
                                "executed_branches": [],
                                "missing_branches": [],
                                "summary": {
                                    "covered_lines": 0,
                                    "num_statements": 1,
                                    "percent_statements_covered": 100.0,
                                    "covered_branches": 0,
                                    "num_branches": 0,
                                    "percent_branches_covered": 100.0,
                                },
                            }
                        },
                    }
                ),
                encoding="utf-8",
            )
            ast_index, _ = build_ast_index(root, ["pkg/a.py"])
            result = run_coverage(
                root,
                ["pkg/a.py"],
                {},
                "full",
                artifact,
                ast_index,
                POLICY,
                "7.15.4",
            )
        self.assertEqual(result["tool"]["status"], "FAILED")
        self.assertEqual(result["measurements"], [])
        self.assertIn("does not match line counts", result["tool"]["reason"])

    @mock.patch("code_health.adapters._version_status", return_value=("4.0.7", "OK", ""))
    @mock.patch("code_health.adapters.run_bounded")
    def test_pylint_fatal_output_fails_without_ok_measurements(
        self, run: mock.Mock, _version: mock.Mock
    ) -> None:
        run.return_value = SimpleNamespace(
            stdout=json.dumps(
                {
                    "messages": [
                        {
                            "type": "fatal",
                            "messageId": "F0001",
                            "message": "No module named missing.py",
                        }
                    ]
                }
            ),
            stderr="",
            returncode=1,
        )
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "a.py").write_text("VALUE = 1\n", encoding="utf-8")
            result = run_pylint(root, ["a.py"], {}, {"a.py": 1}, POLICY, "4.0.7")
        self.assertEqual(result["tool"]["status"], "FAILED")
        self.assertEqual(result["measurements"], [])
        self.assertEqual(result["errors"][0]["code"], "PYLINT_FATAL")


class DigestTests(unittest.TestCase):
    def test_environment_paths_and_artifacts_do_not_affect_digest(self) -> None:
        left = {
            "audit": {"root": "C:/one", "generated_at": "now", "content_digest": "old"},
            "artifacts": {"json": "C:/one/report.json"},
            "summary": {"health": "WATCH"},
        }
        right = {
            "audit": {"root": "D:/two", "generated_at": "later", "content_digest": "new"},
            "artifacts": {"json": "D:/two/report.json"},
            "summary": {"health": "WATCH"},
        }
        self.assertEqual(content_digest(left), content_digest(right))


if __name__ == "__main__":
    unittest.main()
