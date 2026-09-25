import json
import tempfile
import unittest
from dataclasses import FrozenInstanceError
from pathlib import Path

from order_loader import load_orders
from order_model import Order
from order_report import report


class OrderReportTests(unittest.TestCase):
    def test_fixture(self):
        path = Path(__file__).with_name("orders.json")
        self.assertEqual(report(path), [
            {"id": "A", "quote": 270, "receipt": 270},
            {"id": "B", "quote": 89, "receipt": 89},
        ])

    def test_loader_returns_immutable_orders(self):
        path = Path(__file__).with_name("orders.json")
        orders = load_orders(path)
        self.assertEqual(orders[0], Order("A", (100, 200), 10))
        self.assertIsInstance(orders[0].prices, tuple)
        with self.assertRaises(FrozenInstanceError):
            orders[0].discount_percent = 20
        with self.assertRaises(TypeError):
            orders[0].prices[0] = 1

    def test_report_preserves_integer_edges(self):
        orders = [
            {"id": "empty", "prices": [], "discount_percent": 15},
            {"id": "floor", "prices": [99], "discount_percent": 10},
            {"id": "negative", "prices": [-1], "discount_percent": 10},
        ]
        with tempfile.TemporaryDirectory(dir=Path(__file__).parent) as directory:
            path = Path(directory) / "orders.json"
            path.write_text(json.dumps(orders), encoding="utf-8")
            self.assertEqual(report(path), [
                {"id": "empty", "quote": 0, "receipt": 0},
                {"id": "floor", "quote": 89, "receipt": 89},
                {"id": "negative", "quote": -1, "receipt": -1},
            ])
