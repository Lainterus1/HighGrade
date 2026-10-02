# Windows: исправление тестового запуска

[CI e021c3e](https://github.com/Lainterus1/HighGrade/actions/runs/36969851656) выявил неверное quoting в Rust-вызове cmd.exe: путь обрезался на пробеле, до входа в запускатель. 151 passed, 1 failed, 67 not run. [Неизменённый журнал](../objects/746cbc496f07516d3bf00342a9bbce864c0c1b1c207268e58072ebdf4ccdb0a8.json).

Тест использует cmd /D /V:OFF /S /C с raw_arg и двумя парами кавычек; Windows-путь с пробелами, кириллицей и & ! % сохранён, assertions не ослаблены. Основание: [CommandExt::raw_arg](https://doc.rust-lang.org/std/os/windows/process/trait.CommandExt.html#tymethod.raw_arg). Производственный код не изменён. Успех исправления на Windows должен подтвердить новый exact-SHA CI.

После исправления локально повторены 217 Rust-контрактов: все passed. [Исходный комплект](../objects/21c38cbbb036fec56d385706a443d800ec74aabbdb32af8eab51ce467034386d.json). Независимое адресное ревью catalog_read_review: GO кандидата; проверка Windows остаётся за новым CI.
