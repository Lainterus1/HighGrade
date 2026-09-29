# Логотип HighGrade — наблюдение трассировки

2026-09-27, HG-0054-S3. Источник: выбранная пользователем правая колонка `docs/proposals/specification-ui/logo-finalists-reference.png`; входные кадры сохранены в ui/src/assets/brand.

Фактическая команда для каждого из mark-source.png и wordmark-source.png: Python установленного SVG Vectorizer → `scripts/vectorize.py INPUT -o OUTPUT --preset logo --colors 3 --verbose`.

Оба запуска вернули код 0 и `preset=logo; colors=3; scale=1; VTracer -> SVGO -> XML/resvg validation`. Исходные выходы: mark.svg 10178 bytes, wordmark.svg 12863 bytes. После трассировки нормализованы только цвета и сформированы монохромные копии, alpha-mask сохранены. Ручной реконструкции paths и встроенных растров нет.

Визуальная проверка конечных размеров и XML конечных файлов выполняется в браузерном наборе HG-0054; исходная проверка плагина не подменяет её. Это наблюдение инструмента, не человеческая приёмка дизайна.
