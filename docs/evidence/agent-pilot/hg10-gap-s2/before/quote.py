from collections.abc import Sequence

from discount import discounted_total


def quote_total(prices: Sequence[int], discount_percent: int) -> int:
    return discounted_total(prices, discount_percent)
