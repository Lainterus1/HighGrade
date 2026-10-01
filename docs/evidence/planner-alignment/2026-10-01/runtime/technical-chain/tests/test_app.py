import unittest
from app import normalize_items
class Contract(unittest.TestCase):
    def test_normalization(self):
        self.assertEqual(normalize_items([" A ", "", " B ", "   ", "A"]), ["A", "B", "A"])
    def test_empty(self):
        self.assertEqual(normalize_items([]), [])
