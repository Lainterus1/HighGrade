# Установка и обновление

Собери интерфейс (`npm ci` и `npm run build` из `ui/`), затем CLI командой `cargo build --release --locked` из корня. Передавай абсолютные пути. `--profile` обозначает домашний каталог пользователя, а не целевой проект. Команды установки и обновления не сканируют соседние проекты или весь домашний каталог.

```powershell
& '<исходник>/target/release/highgrade.exe' global-install --profile '<домашний-каталог>' --source '<исходник>/kit' --candidate-exe '<исходник>/target/release/highgrade.exe'
& '<домашний-каталог>/.highgrade/global/releases/<active>/highgrade.exe' global-status --profile '<домашний-каталог>'
& '<домашний-каталог>/.highgrade/global/releases/<active>/highgrade.exe' doctor --root '<проект>'
& '<исходник>/target/release/highgrade.exe' inspect --bootstrap --root '<проект>'
& '<домашний-каталог>/.highgrade/global/releases/<active>/highgrade.exe' inspect --root '<проект>' --registry '.highgrade/project/documents.json'
```

Установка создаёт общие маршрутизирующие файлы в `<домашний-каталог>/.agents/skills/highgrade-*/SKILL.md`, а неизменяемую поставку — в `<домашний-каталог>/.highgrade/global/releases/<release>/`. Один активный указатель переключается после проверки хешей и версии. Чужой навык с тем же именем — конфликт. Общие навыки и CLI в проект не копируются. Init после аудита и в разрешённой области готовит `.highgrade/project/INSTRUCTIONS.md` и короткую ссылку из AGENTS.

Обновление всей пользовательской поставки выполняется явно. Сначала preview, затем смысловое сравнение правил, затем применение с тем же `candidate_sha256`. CLI проверяет рабочую команду `doctor` до переключения и после установки. Отпечаток связывает манифест со всеми материалами и байтами исполняемого файла. Принадлежащие активному журналу SKILL.md обновляются вместе с выпуском; пользовательские изменения отвергают переключение. Основная инструкция включена в SKILL.md, процедуры служат единственным редактируемым источником. Незавершённое обновление навыков обозначается GlobalSkillRecoveryRequired; явный `global-recover --profile PATH` проверяет журнал и восстанавливает прежние байты при старом указателе либо подтверждает новый активный выпуск. Сторонние изменения останавливают восстановление. Текущий локальный маршрут — `highgrade-commit`, внешний — `highgrade-push`. При обновлении прежние журналируемые `highgrade-approve` либо `highgrade-deploy` заменяются без постоянных алиасов: новые навыки готовятся до переключения, старый собственный маршрут убирается после проверки. Изменённый/чужой Approve останавливает миграцию; восстановление сверяет принадлежность и байты. Содержимое принятых проектных инструкций автоматически не переписывается: на следующем входе агент сверяет старое имя и недостающие поля маршрута, сохраняет местные правила и согласует нужное дополнение. При ошибке postcheck CLI пытается восстановить прежний активный выпуск; после сбоя фактическое состояние проверяют по [процедуре Update](../../procedures/update.md#повтор-и-ошибки). После успеха собственные файлы старых выпусков удаляются, чужие сохраняются с предупреждением. После прерванного переключения установи фактическое состояние через global-status; ошибка диагностики не подтверждает исправность активной поставки. Неактивный остаток не даёт полномочий. Проектные инструкции проверяются при следующей работе с конкретным проектом.

```powershell
& '<кандидат>/target/release/highgrade.exe' global-update --profile '<домашний-каталог>' --source '<кандидат>/kit' --candidate-exe '<кандидат>/target/release/highgrade.exe'
& '<кандидат>/target/release/highgrade.exe' global-update --profile '<домашний-каталог>' --source '<кандидат>/kit' --candidate-exe '<кандидат>/target/release/highgrade.exe' --apply true --candidate-sha256 '<отпечаток-из-preview>'
```

`doctor`, `inspect`, `inventory` и `trace` работают на чтение с `--root <проект>`. `doctor` обнаруживает путь CLI, но не запускает его и не сверяет версию; закреплённую версию проверяют командой проекта. Штатные тесты запускаются командами проекта. Отсутствующая инструкция или неизвестный инструмент остаются состоянием unknown, а не успешной проверкой. Старые проектные `install`/`update` сохранены только для исторического кандидата как библиотечный контракт и явно названные команды `legacy-install`/`legacy-update`; новые проекты используют глобальный маршрут.



Preview global-update дополнительно возвращает `validation=passed`, `decision=required`, `applied=false`.

## Linux (Bash)

Соберите CLI штатным Cargo с закреплённым toolchain. Профиль — домашний каталог пользователя; команды получают абсолютный путь кандидата. Для готового кандидата:

```bash
hgCandidate="$PWD/target/highgrade/candidate"
"$hgCandidate/highgrade" global-install --profile "$HOME" --source "$hgCandidate/kit" --candidate-exe "$hgCandidate/highgrade"
hgRelease=$(python -c 'import json,pathlib; print(json.loads((pathlib.Path.home()/".highgrade/global/active.json").read_text())["release"])')
hgExe="$HOME/.highgrade/global/releases/$hgRelease/highgrade"
"$hgExe" global-status --profile "$HOME"
"$hgExe" doctor --root "$PWD"
"$hgCandidate/highgrade" global-update --profile "$HOME" --source "$hgCandidate/kit" --candidate-exe "$hgCandidate/highgrade"
# После сверки preview повторите ту же команду с:
# --apply true --candidate-sha256 '<отпечаток-из-preview>'
```

CLI устанавливается с правами 0700. `InstalledExecutableNotExecutable` означает потерю права исполнения; status не исправляет файлы. Перед восстановлением прав сверяйте принадлежность и хеш бинарника с журналом. Ограничение noexec или отказ доступа не обходите: используйте разрешённый исполняемый профиль. Соседние навыки и их ссылки не изменяются. Recovery восстанавливает прерванную транзакцию; произвольный откат выпуска этой командой не предоставляется.
