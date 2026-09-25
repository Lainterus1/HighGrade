# Локальный пилот work → highgrade-approve

Результат: в синтетическом `s7` восстановлена рабочая ссылка из `docs/overview.md` на `docs/DEVELOPMENT.md`; по отдельному решению в `scope-extension.json` добавлена проверенная команда `python -B -m unittest discover -v`. Один обычный локальный коммит: `ac230cc3b99f9495f9accf75ca303ebfe60ef015`; его единственный путь — `docs/overview.md`.

Исходный `HEAD` (`2ac78d78b24c3d6ae30213f29fd581b403b67ebe`) уже содержал корректную ссылку. Исправление грязного рабочего файла само по себе не давало staged diff; коммит содержит только добавленную команду проверки. Это отделяет исходный объём разрешения от расширения координатора. Пустой коммит не создавался.

## Хронология команд и результаты

1. Прочитаны `setup.json`, `before/`, `before-status.txt`, `before-head.txt`, `AGENTS.md`, проектная инструкция и процедуры установленной Поставки. `git status --porcelain=v1`, `git diff -- docs/overview.md` и `git show HEAD:docs/overview.md` установили исходную грязную ссылку и уже правильный `HEAD`.
2. CLI установленной Поставки: `highgrade.exe global-status --profile <s7>/.pilot-profile` и `highgrade.exe doctor --root <s7>`; оба кода выхода `0`, статусы `passed`. Сырые ответы: [global-status.json](global-status.json), [doctor.json](doctor.json).
3. Заменена только `DEVELOPMENT-old.md` на `DEVELOPMENT.md`. Относительные ссылки проверены, `python -B -m unittest discover` завершился с кодом `0`, три теста `OK`: [unittest.txt](unittest.txt). `git diff -- docs/overview.md` после этой правки был пустым: [work-diff.patch](work-diff.patch). Запрошено и получено отдельное [расширение объёма](scope-extension.json).
4. Добавлена строка «Быстрая проверка поведения» с командой `python -B -m unittest discover -v`. Команда выполнена в точности: код выхода `0`, три теста `ok`, общий `OK`: [unittest-verbose.txt](unittest-verbose.txt). Проверены все четыре ссылки и текст команды: [link-check-final.txt](link-check-final.txt).
5. `git diff --check -- docs/overview.md` вернул `0`; [work-final.diff](work-final.diff) содержит единственную добавленную строку с командой. Перед approve сохранён [work-снимок](work-snapshot.json) с SHA-256 принятого файла и значимых зависимостей.
6. `git add -- docs/overview.md`; `git diff --cached --name-only` показал ровно `docs/overview.md`. Полный [staged diff](staged.diff) проверен, `git diff --cached --check` вернул `0`. Файл и значимые зависимости совпали с work-снимком: [approve-precommit.json](approve-precommit.json). Успешный набор тестов на approve не повторялся.
7. `git commit -m "docs: добавить команду проверки в обзор"` вернул `0`: [сырой вывод](commit-output.txt). `git rev-parse HEAD` дал `ac230cc3b99f9495f9accf75ca303ebfe60ef015`; `git diff-tree --no-commit-id --name-only -r HEAD` подтвердил один путь. [Diff коммита](commit.diff), [итоговый status](after-status.txt), [снимок до/work/после](after-snapshot.json).

SHA-256 `docs/overview.md`: до `7c2e3ed7e32dba02025a664c78c71004a947d89fd795f6206b9228b0c64892ad`; после work и коммита `6fa900a4a001ae019b5963144ea15be93e7a9860b2bea5b9f584d2a2e5055c33`. Все остальные пути из `setup.json` и посторонний Git status совпали с исходным состоянием; индекс после коммита пуст. Локальная активация проектом не назначена. Push, глобальное переключение и пользовательская приёмка не выполнялись.
