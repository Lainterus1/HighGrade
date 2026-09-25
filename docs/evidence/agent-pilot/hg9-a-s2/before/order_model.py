from dataclasses import dataclass


@dataclass(frozen=True, slots=True)
class Order:
    id: str
    prices: tuple[int, ...]
    discount_percent: int
