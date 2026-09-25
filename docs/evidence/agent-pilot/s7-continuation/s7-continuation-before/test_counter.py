import unittest

from counter import count_orders


class CounterTests(unittest.TestCase):
    def test_empty(self):
        self.assertEqual(count_orders([]), 0)

    def test_two(self):
        self.assertEqual(count_orders(["a", "b"]), 2)
