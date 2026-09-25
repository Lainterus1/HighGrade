from order_loader import load_orders
from quote import quote_total
from receipt import receipt_total


def report(path):
    rows = []
    for order in load_orders(path):
        rows.append({
            "id": order.id,
            "quote": quote_total(order.prices, order.discount_percent),
            "receipt": receipt_total(order.prices, order.discount_percent),
        })
    return rows
