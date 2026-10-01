from record import *
root=WORK/'empty-project'
write(root,'README.md','# Локальный каталог книг\n\nСтатус: подготовка нового проекта, приложение ещё не реализовано. Исходная цель synthetic fixture owner: один человек добавляет название и автора прочитанной книги и видит список; первая версия без сети и аккаунтов.\n\n## Устройство\n\nСейчас существуют только документы процесса и первоначальная спека. Стек, интерфейс и постоянное хранение пока неизвестны; они будут уточнены до реализации. Отсутствие кода не означает неприменимость будущих компонентов.\n\n## Правила\n\nПервая версия локальная, один пользователь, без внешних интеграций. Предметные требования и задачи принадлежат specs/. Новые продуктовые решения не выводятся из шаблона High Grade.\n\n## Команды и доказательства\n\nШтатная команда приложения и тестов неизвестна: выбрать стек и runner в первой спеке до work. Команды formatter/linter/CI также неизвестны; для текущей документации нет таких gates. Не создавать fake check или приложение ради подключения. Markdown принадлежит автору документов, JSON каталога/survey — CLI High Grade. Обязательные отчёты будущей работы хранить с исходниками проверенной редакции в локальном пакете с SHA-256; точный срок передачи будущего продукта уточнить перед такой передачей.\n\n## Навигация\n\n[Агентский маршрут](AGENTS.md), [устройство](README.md#устройство), [правила](README.md#правила), [команды](README.md#команды-и-доказательства), [предложенная адаптация](.highgrade/project/INSTRUCTIONS.md).\n')
write(root,'AGENTS.md','# Маршрут каталога книг\n\nЧитай [README](README.md) и [предложенную проектную инструкцию](.highgrade/project/INSTRUCTIONS.md). Новые требования идут в spec, реализация определённого контракта — в work. Сейчас разрешён только init с предметным черновиком; приложение не реализовывать. Не создавать Goal, Git, remote, commit, push, release или deploy без отдельного поручения. Сохраняй посторонние файлы. Местных предметных навыков пока нет.\n')
common='''---
highgrade_project_schema: 1
highgrade_spec_format: native-v1
---
# Предложенная местная инструкция

Статус: содержательная адаптация для изолированного упражнения. Это предложение; автор не принимал её как действующие правила и не принимал продукт. Разрешение на подготовку задано synthetic fixture intent до начала упражнения, не выводится из CLI go.

Язык: русский, определён входным поручением фикстуры. Бюджеты: неизвестно, решения автора нет; [реестр](documents.json) хранит null. Это не PASS бюджета и не препятствие независимой подготовке.

'''
write(root,'.highgrade/project/INSTRUCTIONS.md',common+'''## Владельцы и состояние

[README](../../README.md) совмещает назначение, текущее устройство, инженерные правила и команды; [AGENTS](../../AGENTS.md) владеет маршрутом. Обоснование: проект пуст, разделение создало бы заглушки. Обследование: [survey.json](survey.json); завершение относится только к подготовке. Реализации нет.

## Каталог и продолжение

Новые требования в specs/, CLI spec-* владеет записью. Подготовить первоначальную спеку каталога книг; после init уточнить тот же назначенный CLI ID, его интерфейс, хранение и runner. Затем отдельное поручение реализации. Успех spec-validate не означает готовности продукта. Местных навыков нет по исходному inventory; придуманные обязательные роли не вводятся.

## Проверки и полномочия

Команда тестирования/formatter/linter/CI неизвестна; зависит будущая реализация. Для этой подготовки проверить структуру через inspect, совместимость doctor и предметный черновик через spec-validate; команды реально запускаются, исходы записываются отдельно. Доказательства и неизвестный срок передачи будущего результата принадлежат README. Содержательное независимое ревью результата упражнения выполняет координатор отдельно от survey-review исполнителя.

Git не применим к текущей локальной подготовке: создание не поручено. Commit и push не применимы без Git. Способ неизменяемой идентификации будущего noGit-продукта пока неизвестен; определить до финальной передачи Work, не выдавать список файлов за build ID. Remote, ветки/PR/base, squash/rebase, CI exact-SHA и последствия push неприменимы к текущему поручению; при появлении Git требуется отдельное решение. Локальная активация неприменима: исполняемый продукт не создаётся. Приёмка результата, merge, tag/release, deploy и внешняя публикация требуют самостоятельного основания; ни подготовка, ни команда проверки их не разрешают.
''')
root=WORK/'owner-docs'
write(root,'.highgrade/project/INSTRUCTIONS.md',common+'''## Сохранённые владельцы

[README](../../README.md) владеет назначением, [AGENTS](../../AGENTS.md) маршрутом, [GUIDE](../../GUIDE.md) совмещает устройство, предметные/инженерные правила и команды. Существующие файлы сохраняются байт-в-байт; переноса и массовой формализации нет. [survey](survey.json) хранит обследование, не второй backlog.

Продукт: локальный нормализатор кодов поставки без сети. Проверенные источники: code.py, test_code.py и GUIDE.md. Ведущие нули и внутренние пробелы сохраняются; код остаётся строкой, API normalize и имя модуля не меняются. Владелец этих требований — GUIDE, эта ссылка не создаёт второй редактируемый контракт.

## Навыки и требования

Местный [.agents/skills/code-review](../../.agents/skills/code-review/SKILL.md) продолжает отдельный предметный обзор перед изменением формата кода; вход GUIDE и diff, выход замечания о сохранности идентификаторов. Общий High Grade не заменяет его. Новых продуктовых требований нет, начальная спека не создаётся ради отчёта. Прежний GUIDE остаётся действующим; будущие новые изменения могут использовать specs/ после отдельного задания без автоматического переноса.

## Проверки и сопровождение

Определено по GUIDE: python3 -B -m unittest -v test_code, без сторонних пакетов. Среда Python проверяется запуском. Formatter/linter/CI отсутствуют; создавать их ради High Grade не требуется. Markdown принадлежит существующим владельцам, JSON survey/каталога — CLI. Точный локальный пакет и сырой отчёт хранит маршрут из GUIDE до следующей принятой редакции; внешняя передача сейчас не требуется. Pre-submit Git неприменим; финальная будущая передача noGit проверяет неизменяемый архив SHA-256 и отчёты согласно GUIDE. Человеческая приёмка отдельна.

Определено по обследованию и GUIDE: Git, remote, активации и deploy нет. Commit/push, PR/base, squash/rebase, exact-SHA CI неприменимы к текущей подготовке. Их появление потребует отдельной адаптации и полномочий; команда проверки не разрешает создание Git, публикацию или деплой. Полномочий на merge, tag/release или внешние действия нет. Последующее изменение продукта выполняется только по отдельному поручению. Следующий шаг: рассмотреть это предложение адаптации; продуктовых спек и поручения work нет.
''')
for case in ('empty-project','owner-docs'):
 empty=case=='empty-project'
 docs=[{'id':'readme','path':'README.md','roles':['purpose-navigation','current-architecture','engineering-rules','commands-procedures'] if empty else ['purpose-navigation'],'loading':'entry','scope':['specs'] if empty else [],'budget':None},{'id':'agents','path':'AGENTS.md','role':'agent-rules','loading':'entry','scope':[],'budget':None}]
 if not empty: docs.append({'id':'guide','path':'GUIDE.md','roles':['current-architecture','engineering-rules','commands-procedures'],'loading':'task','scope':['code.py','test_code.py'],'budget':None})
 additional=[{'path':'.highgrade/project/INSTRUCTIONS.md','role':'project-adaptation','active':True,'loading':'entry','scope':['.highgrade/project/documents.json','.highgrade/project/survey.json']}]
 if not empty: additional.append({'path':'.agents/skills/code-review/SKILL.md','role':'local-domain-review','active':True,'loading':'task','scope':[]})
 write(WORK/case,'.highgrade/project/documents.json',json.dumps({'schema_version':1,'documents':docs,'additional_sources':additional,'route_budget':None},ensure_ascii=False,indent=2)+'\n')
run('empty-project','16-spec-init','spec-init',allowed=(0,))
a=run('empty-project','17-spec-list','spec-list',allowed=(0,))
run('empty-project','18-spec-new','spec-new','--title','Локальное добавление и просмотр прочитанных книг','--expected',a['result']['store_sha256'],allowed=(0,))
run('empty-project','19-spec-read','spec-read','--id','HG-0001','--view','editable',allowed=(0,))
command('owner-docs','16-python-version',['python3','--version'],WORK/'owner-docs',allowed=(0,))
command('owner-docs','17-domain-tests',['python3','-B','-m','unittest','-v','test_code'],WORK/'owner-docs',allowed=(0,))
