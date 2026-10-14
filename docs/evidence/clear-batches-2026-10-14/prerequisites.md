# HG-0076-S5: дополнительные причинные входы

Проверенное поведение и исходники не изменились после verification.md. Для будущей freshness C1 дополнительно включает src/main.rs, src/lib.rs, src/ui.rs, src/ui/bundle.rs, build.rs и ui/dist/highgrade-ui.json: они участвуют в проверке candidate CLI/version/embedded UI перед установкой fixture.

Новый адресный C1 прошёл: specs/changes/HG-0076/evidence/HG-0076-C1-1791971268314611804.log (1 PASS, 37 filtered). Все 18 before/after/current pins совпали. Эти шесть входов также уже подтверждены исходным sealed Rust snapshot; повтор общего набора не требовался.

verification.md и его прежний original C1 сохранены без редактирования. Дополнение не разрешает доставку или новую приёмку.
