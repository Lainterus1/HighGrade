import json

from order_model import Order


def load_orders(path) -> list[Order]:
    with open(path, encoding="utf-8") as source:
        return [
            Order(
                id=order["id"],
                prices=tuple(order["prices"]),
                discount_percent=order["discount_percent"],
            )
            for order in json.load(source)
        ]
