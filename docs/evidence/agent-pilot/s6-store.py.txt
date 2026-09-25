import json
import os
import tempfile
from pathlib import Path


def save(path, data, fail_after_write=False):
    path = Path(path)
    contents = json.dumps(data, ensure_ascii=False)
    temporary_path = None
    try:
        with tempfile.NamedTemporaryFile(
            mode="w",
            encoding="utf-8",
            dir=path.parent,
            prefix=f".{path.name}.",
            suffix=".tmp",
            delete=False,
        ) as output:
            temporary_path = Path(output.name)
            output.write(contents)
            if fail_after_write:
                raise OSError("simulated disk failure")

        os.replace(temporary_path, path)
        temporary_path = None
    finally:
        if temporary_path is not None:
            temporary_path.unlink(missing_ok=True)
