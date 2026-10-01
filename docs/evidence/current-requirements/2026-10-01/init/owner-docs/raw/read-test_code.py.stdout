import unittest
from code import normalize

class CodeTests(unittest.TestCase):
    def test_leading_zeroes(self):
        self.assertEqual(normalize(" 0012 "), "0012")
    def test_inner_spaces(self):
        self.assertEqual(normalize(" 00 12 "), "00 12")
