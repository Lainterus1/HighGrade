import unittest

from codec import parse_version


class CodecTests(unittest.TestCase):
    def test_v1(self):
        """HG-0001-S2: сообщение v1 возвращает текст."""
        self.assertEqual(parse_version("v1:hello"), "hello")

    def test_v2(self):
        """HG-0001-S1: сообщение v2 возвращает текст."""
        self.assertEqual(parse_version("v2:текст"), "текст")

    def test_unknown(self):
        """HG-0001-S3: сообщение v3 отклоняется."""
        with self.assertRaises(ValueError):
            parse_version("v3:hello")
