from record import *
def item(id,description,state='confirmed',basis=None,sources=None):
 return {'id':id,'description':description,'state':state,'basis':basis or ['Observed files, source-guided execution raw reports and predeclared fixture intent.'],'sources':sources or [],'blocks':[]}
for case in ('empty-project','owner-docs'):
 empty=case=='empty-project';root=WORK/case
 current=run(case,'28-read-final-content','survey-read','--view','editable',allowed=(0,))['result']['value']
 sections=current['sections']
 sections['components']=[item('component-current','Созданы README, AGENTS, проектная инструкция, реестр и предметный черновик HG-0001. Продуктового кода нет.' if empty else 'Исходные code.py, test_code.py, README, GUIDE, AGENTS и code-review сохранены; добавлены только документы адаптации и survey.',sources=['README.md','AGENTS.md','.highgrade/project/INSTRUCTIONS.md','.highgrade/project/documents.json']+(['specs/changes/HG-0001/spec.json'] if empty else ['GUIDE.md','code.py','test_code.py','.agents/skills/code-review/SKILL.md']))]
 sections['knowledge']=[item('knowledge-current','README совмещает четыре роли, AGENTS — агентский маршрут; такое минимальное распределение создано как предложение.' if empty else 'Существующие владельцы README/AGENTS/GUIDE и местный предметный навык сохранены без изменения.',sources=['README.md','AGENTS.md']+([] if empty else ['GUIDE.md']))]
 sections['observations']=[item('observation-execution','Фактические survey-* команды выполнились через CLI 0.3.6. Первые абсолютные input-пути были отвергнуты UnsafePath; supported root-relative inputs после чтения схемы сохранены успешно. Это source-guided исполнение, не автоматическое обнаружение установленного навыка.'),item('observation-inspect','inspect exit 2, только BudgetNotAgreed: 3 находки.' if empty else 'inspect exit 2, только BudgetNotAgreed: 4 находки.',basis=['Actual 25-inspect stdout.']),item('observation-doctor','doctor exit 0, findings=[]; область schema-only, не смысловая сертификация.',basis=['Actual 26-doctor stdout.']),item('observation-specific','spec-validate HG-0001 exit 1 OpenQuestions; сценарий книги не выполнялся, черновик не готов к интеграции.' if empty else 'python3 -B -m unittest -v test_code exit 0: test_inner_spaces и test_leading_zeroes оба ok, всего 2 теста.',basis=['Actual 24-spec-validate stdout.' if empty else 'Actual 17-domain-tests stderr and exit.'])]
 sections['adaptation']=[item('adaptation-written','Предложение местной инструкции и реестр созданы; прочитаны по смыслу. Новые знания записаны предметно, а не оставлены полями шаблона. Автор не принимал новую инструкцию.',sources=['.highgrade/project/INSTRUCTIONS.md','.highgrade/project/documents.json'])]
 sections['routes']=[item('route-next','Первый следующий шаг: task/spec уточняет вопросы существующего HG-0001 об интерфейсе, хранении и runner. Реализация требует отдельного поручения.' if empty else 'Первый следующий шаг: автор рассматривает предложение адаптации. Новых требований и спек нет; следующая продуктовая задача использует GUIDE и code-review без массового переноса.')]
 paths=[p for p in hashes(root) if not p.startswith('.highgrade/local/') and p!='.highgrade/project/survey.json']
 survey_edit(case,'29-final-content',{'sections':sections,'source_paths':paths,'next':'Уточнить вопросы HG-0001 перед отдельным поручением work.' if empty else 'Рассмотреть предложение адаптации; новых продуктовых задач не поручено.'})
 run(case,'30-final-snapshot','survey-snapshot','--expected',survey_sha(case,'30-read'),allowed=(0,))
 run(case,'31-final-review','survey-review','--expected',survey_sha(case,'31-read'),'--reviewer','source-guided-init-executor','--verdict','go','--conclusion','Сверены созданные файлы и реальные отчёты. Подготовка по поручению выполнена; неизвестные бюджеты и будущие вопросы не названы PASS. Инструкция остаётся предложением, продукт не реализован/не принят. Это сверка исполнителя; независимое ревью доказательства выполняется отдельно.',allowed=(0,))
 run(case,'32-final-check-before-complete','survey-check')
 survey_edit(case,'33-completed',{'stage':'completed','next':''})
 run(case,'34-final-check','survey-check')
 run(case,'35-final-survey','survey-read','--view','full',allowed=(0,))
 snapshot(case,'after')
 command(case,'36-diff',['diff','-ruN',str(EVIDENCE/case/'before'),str(EVIDENCE/case/'after')],REPO,allowed=(0,1))
 b=json.loads((EVIDENCE/case/'before-sha256.json').read_text());a=hashes(root)
 observed={'before_files':len(b),'after_files':len(a),'original_missing':[p for p in b if p not in a],'original_modified':[p for p in b if a.get(p)!=b[p]],'created':sorted(set(a)-set(b))}
 write(EVIDENCE/case,'preservation-observation.json',json.dumps(observed,ensure_ascii=False,indent=2)+'\n')
 print(case,observed)
before=json.loads((EVIDENCE/'source-before.json').read_text())
after={'captured_at':now(),'files':{p:sha(REPO/p) for p in before['files']},'cli_sha256':sha(CLI),'outside_sentinel_sha256':sha(WORK/'outside-sentinel.txt')}
after['changed_sources']=[p for p,s in before['files'].items() if after['files'][p]!=s]
after['cli_unchanged']=before['cli']['sha256']==after['cli_sha256'];after['outside_sentinel_unchanged']=before['outside_sentinel_sha256']==after['outside_sentinel_sha256']
write(EVIDENCE,'source-after.json',json.dumps(after,ensure_ascii=False,indent=2)+'\n')
print('source-after',after)
