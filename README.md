# High Grade

[Исходники на GitHub](https://github.com/Lainterus1/HighGrade). High Grade — переносимый набор навыков Codex и инструментов для работы над разными проектами. Общие навыки и Rust CLI устанавливаются один раз у пользователя; каждый проект хранит только собственную инструкцию, документы, OpenSpec, тесты и допустимые профильные дополнения. High Grade не ведёт центральный реестр проектов.

**Состояние:** исходная поставка v0-2-8 содержит обновлённый маршрут сценариев; v0-2-7 [опубликована ранее](docs/releases/v0-2-7.md). `approve` и `push` разделены. Текущую установленную версию проверяет `global-status`. Пилоты целевых проектов отложены. Действующий маршрут — в [глобальном контракте](docs/workflow/GLOBAL.md).

## Состав

| Навык | Задача |
|---|---|
| `highgrade-init` | Аудит/опрос и подключение нового или существующего проекта |
| `highgrade-task` | Проработка задачи с вариантами решений и согласованием результата |
| `highgrade-spec` | Спецификация согласованного изменения в OpenSpec |
| `highgrade-work` | Выполнение с проверками, документами, архивом и ревью |
| `highgrade-clear` | Дополнительный обзор и актуализация состояния проекта |
| `highgrade-approve` | Принятие задачи: локальный коммит и назначенная проектом местная активация |
| `highgrade-push` | Отправка выбранного диапазона готовых коммитов и проверка удалённого результата |
| `highgrade-update` | Переключение общей установки на выбранный пакет |

CLI проверяет готовность (`doctor`), показывает обязательную структуру до реестра (`inspect --bootstrap`), проверяет канонические пути, ссылки и бюджеты (`inspect`), связывает OpenSpec-сценарии с результатами поддерживаемых тестов (`trace`) и управляет глобальной установкой. Он не запускает все тесты проекта и не заменяет их смысловую оценку.

## Установка из исходников

Нужны Windows и Rust с Cargo. Из этого каталога выполни:

```powershell
cargo build --release --locked
& '.\target\release\highgrade.exe' global-install --profile "$env:USERPROFILE" --source (Join-Path $PWD 'kit') --candidate-exe (Join-Path $PWD 'target\release\highgrade.exe')
& '.\target\release\highgrade.exe' global-status --profile "$env:USERPROFILE"
```

После установки открой новый или существующий проект в Codex и вызови `$highgrade-init`, указав его корень. Init сначала готовит аудит, вопросы и план адаптации. Общие навыки не копируются в проект. Подробности и обновление — в [командах](kit/references/cli.md). Настройка проекта находится в `.highgrade/project/INSTRUCTIONS.md`; местные документы остаются у своих владельцев. Новое изменение поведения после согласования задачи оформляется в OpenSpec, затем выполняется штатными инструментами проекта. Для `highgrade-spec` в целевом проекте нужен доступный OpenSpec; его версию и путь фиксирует проектная инструкция. Без OpenSpec спецификация не считается проверенной.

## Разработка High Grade

[AGENTS](AGENTS.md) задаёт работу в этом исходном проекте. [ARCHITECTURE](docs/ARCHITECTURE.md) показывает фактическое устройство, [ENGINEERING](docs/ENGINEERING.md) — правила изменений, [DEVELOPMENT](docs/DEVELOPMENT.md) — команды OpenSpec и Rust. [QUALITY](docs/workflow/QUALITY.md) и [CONTEXT](docs/workflow/CONTEXT.md) задают правила доказательств и документов; [SPECIFICATION](docs/workflow/SPECIFICATION.md) хранит историю решений. История исследования BDD — в [research](docs/research/bdd-workflow.md); архив — в [archive](docs/archive/README.md). Исходники SVG Vectorizer и Code Health Audit поддерживаются отдельно под [plugins](plugins/README.md) и не обязательны для основного маршрута.


## Лицензия

Собственные исходники High Grade распространяются по [MIT](LICENSE). Происхождение импортированных плагинов и границы сторонних зависимостей описаны в [NOTICE](NOTICE.md).
