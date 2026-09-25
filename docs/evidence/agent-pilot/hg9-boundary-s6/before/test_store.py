import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from store import save


class StoreTests(unittest.TestCase):
    def test_save(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "settings.json"
            save(path, {"theme": "dark"})
            self.assertEqual(path.read_text(encoding="utf-8"), '{"theme": "dark"}')

    def test_failed_save_preserves_existing_file(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "settings.json"
            original = b'{ "theme": "light" }\r\n'
            path.write_bytes(original)

            with self.assertRaisesRegex(OSError, "simulated disk failure"):
                save(path, {"theme": "dark"}, fail_after_write=True)

            self.assertEqual(path.read_bytes(), original)
            self.assertEqual(list(Path(directory).iterdir()), [path])

    def test_invalid_data_preserves_existing_file(self):
        """HG-0010-S6: невалидные данные не меняют прежние байты."""
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "settings.json"
            original = b' \t{ "theme": "light" }\r\n'
            path.write_bytes(original)

            with self.assertRaises(TypeError):
                save(path, {"theme": object()})

            self.assertEqual(path.read_bytes(), original)
            self.assertEqual(list(Path(directory).iterdir()), [path])

    def test_replace_failure_preserves_existing_file(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "settings.json"
            original = b'{ "theme": "light" }\r\n'
            path.write_bytes(original)

            with patch("store.os.replace", side_effect=OSError("replace failed")):
                with self.assertRaisesRegex(OSError, "replace failed"):
                    save(path, {"theme": "dark"})

            self.assertEqual(path.read_bytes(), original)
            self.assertEqual(list(Path(directory).iterdir()), [path])
