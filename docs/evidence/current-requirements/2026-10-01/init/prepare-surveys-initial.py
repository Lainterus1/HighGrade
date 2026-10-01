from record import *
def item(id,description,state='confirmed',basis=None,sources=None,blocks=None):
 return {'id':id,'description':description,'state':state,'basis':basis or ['Synthetic fixture intent recorded before execution; actual inventory and file reads.'],'sources':sources or [],'blocks':blocks or []}
intent=json.loads((EVIDENCE/'fixture-intent.json').read_text())
command('environment','global-status-source-guided',[str(CLI),'global-status','--profile',str(WORK/'uninstalled-test-profile')],REPO)
for case in ('empty-project','owner-docs'):
 empty=case=='empty-project'; source=[] if empty else list(hashes(WORK/case))
 source=[p for p in source if not p.startswith('.highgrade/')]
 d=intent[case]
 decisions=[item('purpose',d['purpose']),item('scope',d['scope']),item('preservation',d['preservation'] if empty else 'Сохранить все исходные файлы без изменения.'),item('changes','Добавить документы процесса; для нового проекта начальный предметный черновик, без реализации.'),item('local_rules','Исходных местных правил нет.' if empty else 'Сохранить GUIDE.md, AGENTS.md и code-review; без замены High Grade.',sources=[] if empty else ['GUIDE.md','AGENTS.md','.agents/skills/code-review/SKILL.md']),item('environment','Изолированный Linux-каталог, офлайн-подготовка; кандидат CLI 0.3.6; не создавать Git или внешние ресурсы.'),item('mandate',d['mandate'])]
 sections={
 'components':[item('component-observed','Исходный каталог пуст: inventory возвратил entries={}; реализации пока нет.' if empty else 'code.py содержит normalize через strip; test_code.py содержит два теста. Прочитаны все 6 исходных файлов.',basis=['Actual 01-inventory stdout and read-* outputs.'],sources=[] if empty else ['code.py','test_code.py'])],
 'knowledge':[item('knowledge-owners','Назначить README владельцем назначения, устройства, правил и команд; AGENTS владельцем маршрута.' if empty else 'README владеет назначением, AGENTS маршрутом; GUIDE совмещает устройство, правила и команды. Содержание прочитано, перенос не нужен.',state='proposed' if empty else 'confirmed',sources=[] if empty else ['README.md','AGENTS.md','GUIDE.md'])],
 'processes':[item('process-observed','В исходном пустом каталоге нет runner, formatter, linter, CI или ignore. Будущая команда тестирования неизвестна; её выбор относится к будущей реализации.' if empty else 'GUIDE назначает python3 -B -m unittest -v test_code. Formatter/linter/CI/ignore отсутствуют по обследованию. Новые MD/JSON не охватываются отсутствующими gates; владельцы документов и CLI владеют своим форматом.',sources=[] if empty else ['GUIDE.md'])],
 'skills':[item('skills-observed','Местных навыков в пустом каталоге не обнаружено.' if empty else 'code-review отвечает за сверку ведущих нулей и внутренних пробелов перед изменением формата; общий маршрут High Grade не заменяет предметный обзор.',sources=[] if empty else ['.agents/skills/code-review/SKILL.md'])],
 'observations':[item('observation-inventory','Выполнен inventory --mode structure и прочитаны имеющиеся существенные файлы. Это source-guided упражнение: native dispatch установленного навыка не происходил.',basis=['Recorded 01-inventory and read-* raw command results.'])],
 'findings':[item('finding-budget','Бюджеты контекста не согласованы; оставить null и считать BudgetNotAgreed неизвестностью, не PASS. Они не мешают разрешённой подготовке.',state='unknown',basis=[]),item('finding-future','Выбор стека и runner относится к будущей реализации, которая не поручена.' if empty else 'Новых продуктовых требований нет; существующий GUIDE не мигрировать и фиктивную спеку не создавать.')],
 'adaptation':[item('adaptation-plan','Создать минимальные README, AGENTS, проектную инструкцию и реестр; через CLI создать предметный черновик каталога книг.' if empty else 'Добавить только .highgrade/project/INSTRUCTIONS.md и реестр с сохранёнными владельцами; survey зафиксирует подготовку. Не менять исходные файлы.',state='proposed')],
 'routes':[item('route-next','После init уточнить интерфейс/хранение/runner в первоначальной спеке каталога книг, затем отдельное поручение work.' if empty else 'Следующая продуктовая задача сначала использует действующие правила GUIDE; изменение формата вызывает code-review. Полномочий на следующую задачу нет.')]
 }
 patch={'goal':d['purpose'],'scope':['.'],'stage':'survey','next':'Выполнить разрешённую минимальную адаптацию.','decisions':decisions,'sections':sections,'source_paths':source,'areas':['.']}
 survey_edit(case,'04-survey-content',patch)
 run(case,'05-survey-snapshot','survey-snapshot','--expected',survey_sha(case,'05-read'))
 run(case,'06-survey-review','survey-review','--expected',survey_sha(case,'06-read'),'--reviewer','source-guided-init-executor','--verdict','go','--conclusion','Сверены исходные файлы и synthetic fixture intent. План минимален, локальные владельцы сохраняются, установка/реализация/приёмка не заявляются. Это смысловая сверка исполнителя, не независимое ревью.')
 run(case,'07-survey-check','survey-check')
 survey_edit(case,'08-planned',{'stage':'planned'})
 survey_edit(case,'09-adapting',{'stage':'adapting'})
