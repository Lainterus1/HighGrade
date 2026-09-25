import unittest

from total import total


class TotalTests(unittest.TestCase):
    def test_existing_total(self):
        self.assertEqual(total([10.0, 5.0], 0.2), 12.0)
