import datetime
import hashlib
import json
import pathlib
import shutil
import subprocess
import tempfile

repo = pathlib.Path(__file__).resolve().parents[1]
candidate = repo / "target/debug/highgrade.exe"
output = repo / "docs/evidence/spec-catalog/isolated-pilot-refresh.json"
operations = []

subprocess.run(["cargo", "build", "--locked"], cwd=repo, check=True, timeout=300)
candidate_sha256 = hashlib.sha256(candidate.read_bytes()).hexdigest()


def call(exe, command, *args):
    process = subprocess.run(
        [str(exe), command, *map(str, args)],
        cwd=project,
        capture_output=True,
        text=True,
        encoding="utf-8",
        timeout=120,
    )
    try:
        result = json.loads(process.stdout)
    except json.JSONDecodeError as error:
        raise RuntimeError(f"{command}: invalid JSON: {process.stdout[:500]}") from error
    operations.append({"operation": command, "status": result.get("status")})
    if process.returncode != 0 or result.get("status") != "passed":
        raise RuntimeError(f"{command}: {process.returncode} {result.get('findings')}")
    return result


def store_hash(result):
    return next(item["store_sha256"] for item in result["measurements"] if "store_sha256" in item)


with tempfile.TemporaryDirectory(prefix="hg-spec-catalog-") as work_name:
    work = pathlib.Path(work_name).resolve()
    profile = work / "profile"
    project = work / "project"
    source = work / "source-kit"
    profile.mkdir()
    project.mkdir()
    assert not project.is_relative_to(repo.resolve())
    shutil.copytree(repo / "kit", source)
    installation = call(candidate, "global-install", "--profile", profile, "--source", source, "--candidate-exe", candidate)
    release = installation["measurements"][0]["release"]
    installed = profile / ".highgrade/global/releases" / release / "highgrade.exe"
    status = call(installed, "global-status", "--profile", profile)
    assert status["measurements"][0]["release"] == release
    doctor = subprocess.run([str(installed), "doctor", "--root", str(project)], cwd=project,
                            capture_output=True, text=True, encoding="utf-8", timeout=120)
    candidate_version = json.loads(doctor.stdout)["measurements"][0]["highgrade_version"]
    call(installed, "spec-init", "--root", project)
    first = call(installed, "spec-list", "--root", project)
    call(installed, "spec-new", "--root", project, "--title", "Проверка автономного каталога", "--expected", store_hash(first))
    read = call(installed, "spec-read", "--root", project, "--id", "HG-0001")
    change = next(item["change"] for item in read["measurements"] if "change" in item)
    example = json.loads((repo / "docs/examples/spec-catalog/spec.json").read_text(encoding="utf-8"))
    assert any("а" <= char <= "я" or "А" <= char <= "Я" for char in example["goal"])
    change["goal"] = "Каталог читается установленной Поставкой без исходников"
    change["rationale"] = "Проверка автономности сохранённых спецификаций"
    change["scope"] = "Изолированный временный проект"
    change["tasks"] = [{"id": "HG-0001-T1", "description": "Проверить чтение каталога", "done": True}]
    change["operations"] = [{"action": "add", "requirement": {
        "id": "HG-0001-R1", "title": "Чтение каталога",
        "statement": "Установленная Поставка читает собственный каталог проекта",
        "scenarios": [{"id": "HG-0001-S1", "given": "Создано нативное изменение",
                       "when": "Запущен spec-list", "then": "Изменение найдено по ID",
                       "verification": "Сравнить фактический ответ spec-list с HG-0001"}]}}]
    change["checks"] = [{"id": "HG-0001-C1", "scenario_ids": ["HG-0001-S1"],
                         "runner": None, "file": "", "selector": "",
                         "preparation": "Создать изменение через установленный CLI",
                         "action": "Запустить spec-list", "observation": "HG-0001 присутствует",
                         "inputs": ["probe.txt"]}]
    edit = project / "edit.json"
    edit.write_text(json.dumps(change, ensure_ascii=False), encoding="utf-8")
    call(installed, "spec-save", "--root", project, "--id", "HG-0001", "--expected", store_hash(read), "--input", "edit.json")
    saved = json.loads((project / "specs/changes/HG-0001/spec.json").read_text(encoding="utf-8"))
    assert set(example) == set(saved)
    call(installed, "spec-validate", "--root", project, "--id", "HG-0001")
    listed = call(installed, "spec-list", "--root", project)
    summary = next(item for item in listed["measurements"] if "changes" in item)
    assert any(item["id"] == "HG-0001" for item in summary["changes"])
    (project / "probe.txt").write_text("Создание HG-0001 и проверка spec-list", encoding="utf-8")
    (project / "list-report.json").write_text(json.dumps(listed, ensure_ascii=False), encoding="utf-8")
    evidence = {"scenario": "HG-0001-S1", "method": "manual",
                "command": "installed highgrade.exe spec-list --root project",
                "captured_at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
                "outcome": "passed", "observation": "Фактический spec-list содержит HG-0001",
                "inputs": ["probe.txt"], "report": "list-report.json"}
    (project / "evidence.json").write_text(json.dumps(evidence, ensure_ascii=False), encoding="utf-8")
    current = call(installed, "spec-read", "--root", project, "--id", "HG-0001")
    call(installed, "spec-evidence", "--root", project, "--id", "HG-0001", "--expected", store_hash(current), "--input", "evidence.json")
    current = call(installed, "spec-read", "--root", project, "--id", "HG-0001")
    call(installed, "spec-review", "--root", project, "--id", "HG-0001", "--expected", store_hash(current),
         "--verdict", "go", "--reviewer", "synthetic-cli-pilot", "--conclusion", "Mechanical lifecycle only")
    call(installed, "spec-check", "--root", project, "--id", "HG-0001")
    current = call(installed, "spec-read", "--root", project, "--id", "HG-0001")
    call(installed, "spec-integrate", "--root", project, "--id", "HG-0001", "--expected", store_hash(current))
    call(installed, "spec-read", "--root", project, "--requirement", "HG-0001-R1")

    service = project / ".highgrade/local"
    service.mkdir(parents=True)
    (service / "state.txt").write_text("temporary service config", encoding="utf-8")
    (service / "state.txt").unlink()
    assert service.parent.resolve().is_relative_to(project.resolve())
    shutil.rmtree(service.parent)
    assert source.is_relative_to(work) and source != work
    shutil.rmtree(source)
    assert not source.exists() and not (project / ".highgrade").exists()
    final_list = call(installed, "spec-list", "--root", project)
    final_summary = next(item for item in final_list["measurements"] if "requirements" in item)
    assert "HG-0001-R1" in final_summary["requirements"]
    call(installed, "spec-read", "--root", project, "--requirement", "HG-0001-R1")
    call(installed, "spec-check", "--root", project, "--id", "HG-0001")
    report = {"status": "passed", "captured_at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
              "command": "python scripts/verify-spec-catalog-pilot.py",
              "build_command": "cargo build --locked", "candidate_sha256": candidate_sha256,
              "candidate_version": candidate_version, "source_copy_removed": True,
              "project_outside_repository": True,
              "project_service_removed_before_recheck": True,
              "requirement_readable_after_recheck": True,
              "russian_example_matches_change_fields": True,
              "human_acceptance": "pending", "operations": operations,
              "limitations": ["Synthetic CLI lifecycle only; no agent behavior or human acceptance verified"]}
    output.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"status": "passed", "operations": len(operations), "report": str(output)}, ensure_ascii=False))
