# Доказательства CI37

Исходная реализация: `9dcb0f1e712a974e7a2fd1686ece35c7a082cbbd`. [Прогон Windows и Ubuntu](https://github.com/Lainterus1/HighGrade/actions/runs/36724473879) завершён успешно. Точные jobs, версии предшественников и SHA артефактов находятся в `ci37-provenance.json`.

`ci37-bundle.json.xz` — сжатый без потерь исходный bundle: provenance, selectors S1–S6 и все 10 неизменённых raw UTF-8 отчётов с SHA-256. Это постоянная копия результатов, независимая от срока хранения GitHub artifacts. Распаковка не требует сторонних пакетов.

- SHA-256 XZ: `3fd887d67b0ab2a1796b600a64ebfc65967f6743c9c2aaca6fcae3bddace760d`
- SHA-256 исходного JSON: `ce63302300b6df9d8a36b3f997ea7cce6d5a7492fade859b2e85c0f5ed4dc2d2`

Промежуточная история evidence сохраняет первоначальные имена несжатых отчётов. Для их восстановления в отдельный каталог из корня репозитория:

```python
from pathlib import Path
import hashlib, json, lzma

p = Path("docs/evidence/audit-reliability/ci37-bundle.json.xz")
raw = lzma.decompress(p.read_bytes())
assert hashlib.sha256(raw).hexdigest() == "ce63302300b6df9d8a36b3f997ea7cce6d5a7492fade859b2e85c0f5ed4dc2d2"
bundle = json.loads(raw)
out = Path("target/highgrade/tmp/ci37-restored/docs/evidence/audit-reliability")
out.mkdir(parents=True, exist_ok=True)
(out / "ci37-bundle.json").write_bytes(raw)
for report in bundle["reports"].values():
    name = report["name"]
    assert Path(name).name == name and name not in (".", "..")
    content = report["raw_utf8"].encode("utf-8")
    assert hashlib.sha256(content).hexdigest() == report["sha256"]
    (out / name).write_bytes(content)
```

Упаковка и новый Git SHA не означают повторного исполнения тестов. Человеческая приёмка остаётся незаписанной.
