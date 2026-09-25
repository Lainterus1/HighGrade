from collections.abc import Sequence


def discounted_total(prices: Sequence[int], discount_percent: int) -> int:
    subtotal = sum(prices)
    return subtotal * (100 - discount_percent) // 100
