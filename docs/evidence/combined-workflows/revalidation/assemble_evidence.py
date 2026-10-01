#!/usr/bin/env python3
"""Assemble a proposed batch from completed observations; never writes a catalog."""
from pathlib import Path
import json,hashlib,datetime,shlex
R=Path(__file__).resolve().parents[4];E=Path(__file__).resolve().parent;P=str(E.relative_to(R))
def load(n):return json.loads((E/n).read_text())
def paths(prefix):return {str(p.relative_to(R)) for p in (R/prefix).rglob('*') if p.is_file() and '__pycache__' not in p.parts}
def sha(p):return hashlib.sha256((R/p).read_bytes()).hexdigest()
old=load('prior-evidence-index.json');before=load('inputs-before.json');native=load('native-observations.json');direct=load('direct-observations.json');at=datetime.datetime.now(datetime.timezone.utc).isoformat()
kit={p for p in before['files'] if p.startswith('kit/')};cli={p for p in before['files'] if p.startswith('src/') or p in ['Cargo.toml','Cargo.lock','build.rs','rust-toolchain.toml']}
common={P+'/inputs-before.json',P+'/inputs-after.json',P+'/normative-review.md',P+'/focused-python.log',P+'/render-check.log',P+'/native-observations.json',P+'/native/attempt.json',P+'/native/completion.json',P+'/native/list.json',P+'/native/junit.xml'}
norm63={'HG-0031-S1','HG-0048-S2'}|{f'HG-0063-S{i}' for i in range(1,8)}
pilot={'HG-0011-S1':['solo'],'HG-0011-S2':['activation-failure'],'HG-0011-S3':['solo'],'HG-0011-S4':['solo','foreign-staged'],'HG-0011-S5':['multi-push'],'HG-0011-S6':['multi-push'],'HG-0011-S7':['unknown-push'],'HG-0011-S8':['autodeploy-push'],'HG-0011-S11':['solo'],'HG-0011-S12':['activation-only']}
observations={
 'HG-0062-S1':'Текущая нормативная ветвь recovery совпадает с status/recover и выполненными recovery-контрактами; конфликт и иная ошибка не разрешают произвольную запись.',
 'HG-0062-S2':'Description и правило Update обещают восстановление после сбоя, не постоянный rollback; фактический status обязателен.',
 'HG-0062-S3':'Три свежих regression теста генератора подтвердили условный проектный вход только Update и единый источник body/metadata; render --check прошёл.',
 'HG-0062-S4':'Правило по фактическому формату/полям согласовано с текущим v5/shared-inputs справочником; старая CLI не может писать новый формат, миграция отдельная.',
 'HG-0062-S5':'Нормативная unknown/exit2/validation/decision/applied ветвь совпадает с текущим кодом; combined testcase подтвердил preview без переключения, отказ неверного/изменённого digest и apply исходного.',
 'HG-0011-S1':'Реально создан task-only commit в solo без навязанной активации/release/push.',
 'HG-0011-S2':'Реальный commit сохранён; apply игрушечной активации вернул 23, status подтвердил old-build; успех восстановления не выдуман.',
 'HG-0011-S3':'Сверены оба входных SHA-256, полный staged diff и итог commit; счётчик успешного toy теста остался 1.',
 'HG-0011-S4':'Коммит содержит только app.py/evidence/work.json; чужие unstaged/untracked байты и отдельный чужой staged diff сохранены.',
 'HG-0011-S5':'Проверены источник задач, фактический диапазон и remote, выполнен настоящий push в локальный bare; toy тест не перезапускался.',
 'HG-0011-S6':'Оба коммита alpha/beta проверены и отправлены в local origin/delivery; remote SHA совпадает с HEAD.',
 'HG-0011-S7':'Неизвестный mystery.txt связан с конкретным коммитом, отправка остановлена, local remote не изменён.',
 'HG-0011-S8':'Объявленный RUNBOOK production autodeploy раскрыт, разрешения на эффект нет; зависимый push не выполнялся и remote не изменён.',
 'HG-0011-S11':'Сохранён краткий русский ответ solo с фактическим commit SHA, проверками и сохранёнными посторонними изменениями, без выдуманной активации/приёмки.',
 'HG-0011-S12':'Точное уже закоммиченное содержимое активировано игрушечной командой git-show-exact-sha; status SHA совпал. Ответ называет локально активный неопубликованный результат.',
 'HG-0004-S3':'Настоящая установленная CLI создала/нашла русский каталог, два check совпали, validate passed; после перемещения служебной конфигурации spec-read passed и все catalog bytes прежние. Check честно failed из-за незавершённого синтетического change, не объявлен ready.'}
unknown={
 'HG-0020-S6':'Нет настоящего Goal API; состояние другого незавершённого Goal и разрешение конфликта не исполнялись.',
 'HG-0020-S7':'Нет настоящего Goal API; дополнительно сохранён конфликт plan-only между Planner (не создавать) и HG-0020-R3/S7 (создать Goal). Нормативное ожидание не исправлялось.',
 'HG-0020-S8':'Свежий technical-only опыт подтвердил сохранность и отсутствие commit/activation/push/acceptance, но не воспроизвёл завершённую зависимую цепочку с планом Planner. Полного нового наблюдения нет; прежний PASS не перепривязан.'}
batch=[];plan={'created_at':at,'counts':{},'input_manifests':{},'scenarios':[],'scope_policy':'Сохраняется каждый прежний объявленный путь; добавлены текущие релевантные источники и новые реальные отчёты. Старые наблюдения не служат новым исходом. Before-source capture и output artifacts разделены по роли. Не включены временные binary/profile/context копии: их источник и точные хеши зафиксированы.'}
for num in ['0062','0063']:
 count={}
 for sid,prior in old[num].items():
  inp=set(prior['input_paths'])|common|kit|{f'specs/changes/HG-{num}/spec.json'}
  method='manual';report=P+'/normative-review.md';command='Новая смысловая сверка текущих процедур, исходников и таблицы; python -m unittest scripts.tests.test_render_skills scripts.tests.test_workflow_contract -v; python scripts/render-skills.py --check';outcome='passed'
  observation=observations.get(sid,'Новая смысловая сверка соответствующей строки normative-review.md подтвердила текущие разрешённые переходы и обязательные барьеры; не runtime-пилот и не человеческая приёмка.')
  if sid in native['scenarios']:
   method='native_report';report=P+'/native/junit.xml';command='python scripts/verify.py tests (завершённый объединённый прогон); python '+P+'/verify_observations.py (сверка source/completion/JUnit и selectors)';inp|=cli|paths('tests')|{'.config/nextest.toml','.gitattributes'}
   observation='В объединённом Linux JUnit действительно passed: '+', '.join(t['test'] for t in native['scenarios'][sid])+'. Все 440 source capture хешей, completion/report/inventory проверены. Windows file-lock не исполнялся.'
  elif sid in pilot:
   report=P+'/direct-observations.json';command='Реальные audit-команды из pilots/CASE/after/actions.jsonl; python '+P+'/verify_observations.py';inp|=cli|{P+'/fixture_driver.py',P+'/pilot-inputs-before.json',P+'/observer-inputs-before.json',P+'/verify_observations.py',P+'/installed-status.json',P+'/install-fixture.json',P+'/direct-observations.json',P+'/observation-verification.log',P+'/context-provenance.json'}
   actual_commands=[]
   for case in pilot[sid]:
    inp|=paths(P+'/pilots/'+case)
    for line in (E/'pilots'/case/'after/actions.jsonl').read_text().splitlines():
     action=json.loads(line)
     actual_commands.append('cd '+shlex.quote('target/highgrade/tmp/combined-revalidation-20261001/'+case)+' && '+shlex.join(['python3','tools/audit.py',*action['argv']]))
   command='; '.join(actual_commands)+'; python '+P+'/verify_observations.py'
   observation+=' Предел: один осведомлённый продолжающий агент, supplied installed context; это не независимый или clean-context пилот, не hosted CI и не пользовательская установка.'
  elif sid=='HG-0004-S3':
   method='native_report';report=P+'/catalog-survival/summary.json';command='python '+P+'/catalog_exercise.py; python '+P+'/verify_observations.py';inp|=cli|paths(P+'/catalog-survival')|{P+'/catalog_exercise.py',P+'/catalog-inputs-before.json',P+'/catalog-exercise.log',P+'/install-fixture.json',P+'/installed-status.json',P+'/context-provenance.json'}
  elif sid in unknown:
   outcome='unknown';observation=unknown[sid];report=P+'/unverified.json';command='Проверка доступных инструментов Goal и границ текущих наблюдений; без вызова настоящего Goal';inp|={'specs/requirements/HG-0020-R1.json','specs/requirements/HG-0020-R3.json',P+'/unverified.json'}
   if sid=='HG-0020-S8':inp|=paths(P+'/pilots/technical-only')|{P+'/direct-observations.json'}
  evidence={'scenario':sid,'method':method,'command':command,'captured_at':at,'outcome':outcome,'observation':observation,'inputs':sorted(inp),'report':report}
  batch.append({'id':'HG-'+num,'evidence':evidence});count[outcome]=count.get(outcome,0)+1
  key=hashlib.sha256(json.dumps(sorted(inp),ensure_ascii=False,separators=(',',':')).encode()).hexdigest();plan['input_manifests'][key]={'input_paths':sorted(inp),'files':{p:sha(p) for p in sorted(inp)}};plan['scenarios'].append({'id':'HG-'+num,**{k:v for k,v in evidence.items() if k!='inputs'},'input_manifest':key,'preserved_prior_input_count':len(prior['input_paths']),'new_source_inputs_captured_before':True,'outputs_attached_after_observation':True})
 plan['counts']['HG-'+num]=count
for n,v in [('scenario-input-plan.json',plan)]: (E/n).write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n')
# CLI input is disposable transport after parent records it; retained plan avoids duplicated arrays.
target=R/'target/highgrade/tmp/combined-revalidation-evidence-batch.json';target.write_text(json.dumps(batch,ensure_ascii=False,separators=(',',':'))+'\n')
print(json.dumps({'counts':plan['counts'],'batch':str(target),'batch_bytes':target.stat().st_size,'scenario_count':len(batch)},ensure_ascii=False))
