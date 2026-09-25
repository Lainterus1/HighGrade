import unittest

from quote import quote_total
from receipt import receipt_total


class TotalsTests(unittest.TestCase):
    def test_quote(self):
        self.assertEqual(quote_total([100, 200], 10), 270)

    def test_receipt(self):
        self.assertEqual(receipt_total([100, 200], 10), 270)

    def test_rounding(self):
        self.assertEqual(quote_total([99], 10), 89)
        self.assertEqual(receipt_total([99], 10), 89)

    def test_existing_integer_edges(self):
        cases = [
            ([], 15, 0),
            ([1], 1, 0),
            ([-1], 10, -1),
            ([100], -20, 120),
            ([100], 150, -50),
        ]
        for total in (quote_total, receipt_total):
            for prices, discount_percent, expected in cases:
                with self.subTest(total=total.__name__, prices=prices, discount_percent=discount_percent):
                    self.assertEqual(total(prices, discount_percent), expected)
