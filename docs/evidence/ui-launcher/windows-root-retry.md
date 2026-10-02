# Windows: аргумент корня проекта

В [CI 366ac86](https://github.com/Lainterus1/HighGrade/actions/runs/36970538688) вызов .cmd уже состоялся, но тест прочитал только первую строку многострочного JSON и скрыл диагностику CLI. [Неизменённый журнал](../objects/811d158bbb829a935e1f4e49d67ca997cd53748d116206703c132d883871fb78.json). Исход failed, 151 passed/1 failed/67 not run.

Корень проекта теперь передаётся через Join-Path с точкой, без завершающего backslash в native-аргументе. Это устраняет известную особенность [Windows PowerShell](https://github.com/PowerShell/PowerShell/issues/7400), сохраняя корень диска и каталог. Причина прежнего JSON-ответа ещё не доказана напрямую. Reader теста читает полный JSON и показывает ответ ошибки; сложный путь и assertions сохранены. Независимое ревью catalog_read_review: GO кандидата.

[Локальный повтор](../objects/b72a1460785c8d3102a3d5a34c3a61849df5275aed19fc3acc9731c161e300d3.json): 217 Rust passed. Windows должен подтвердить новый exact-SHA CI; установка не переключалась.
