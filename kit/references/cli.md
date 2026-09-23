# Команды глобальной поставки High Grade

Собери выбранный исходный каталог командой `cargo build --release --locked`. Первый выпуск распространяется исходниками для Windows. Передавай абсолютные пути. `--profile` обозначает домашний каталог пользователя, например `$env:USERPROFILE`, а не целевой проект. Команды установки и обновления не сканируют соседние проекты или весь домашний каталог.

```powershell
& '<исходник>/target/release/highgrade.exe' global-install --profile '<домашний-каталог>' --source '<исходник>/kit' --candidate-exe '<исходник>/target/release/highgrade.exe'
& '<домашний-каталог>/.highgrade/global/releases/<active>/highgrade.exe' global-status --profile '<домашний-каталог>'
& '<домашний-каталог>/.highgrade/global/releases/<active>/highgrade.exe' doctor --root '<проект>'
& '<домашний-каталог>/.highgrade/global/releases/<active>/highgrade.exe' inspect --root '<проект>' --registry '.highgrade/project/documents.json'
```

Установка создаёт общие маршрутизирующие файлы в `<домашний-каталог>/.agents/skills/highgrade-*/SKILL.md` и неизменяемую поставку в `<домашний-каталог>/.highgrade/global/releases/<release>/`. Один активный указатель переключается после проверки хешей и версии. Чужой навык с тем же именем — конфликт. Общие навыки и CLI в проект не копируются. Init после аудита и в разрешённой области готовит `.highgrade/project/INSTRUCTIONS.md` и короткую ссылку из AGENTS.

Обновление всей пользовательской поставки выполняется явно. Сначала preview, затем смысловое сравнение правил, затем применение с тем же `candidate_sha256`. CLI проверяет рабочую команду `doctor` до переключения и после установки. Отпечаток связывает манифест со всеми материалами и байтами исполняемого файла. Файлы SKILL.md маршрутизаторов в этой версии должны оставаться побайтно одинаковыми; их несовместимое изменение требует отдельной миграции. Проектные инструкции проверяются при следующей работе с конкретным проектом.

```powershell
& '<установленный-или-собранный>/highgrade.exe' global-update --profile '<домашний-каталог>' --source '<кандидат>/kit' --candidate-exe '<кандидат>/target/release/highgrade.exe'
& '<установленный-или-собранный>/highgrade.exe' global-update --profile '<домашний-каталог>' --source '<кандидат>/kit' --candidate-exe '<кандидат>/target/release/highgrade.exe' --apply true --candidate-sha256 '<отпечаток-из-preview>'
& '<установленный-или-собранный>/highgrade.exe' global-update --profile '<домашний-каталог>' --rollback '<прежняя-версия>'
```

`doctor`, `inspect`, `inventory` и `trace` работают на чтение с `--root <проект>`. Штатные тесты запускаются командами проекта. Отсутствующая инструкция или неизвестный инструмент остаются состоянием unknown, а не успешной проверкой. Старые проектные `install`/`update` сохранены только для исторического кандидата как библиотечный контракт и явно названные команды `legacy-install`/`legacy-update`; новые проекты используют глобальный маршрут.
