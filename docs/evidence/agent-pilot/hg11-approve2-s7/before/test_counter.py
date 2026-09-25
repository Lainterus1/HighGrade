import unittest

from counter import count_orders


class CounterTests(unittest.TestCase):
    def test_empty(self):
        self.assertEqual(count_orders([]), 0)

    def test_two(self):
        self.assertEqual(count_orders(["a", "b"]), 2)

    def test_unique_order_ids(self):
        """HG-0001-S1: повторный ID обозначает тот же заказ."""
        self.assertEqual(count_orders(["a", "b", "a", "a"]), 2)
        self.assertEqual(count_orders(["a", "a"]), 1)
