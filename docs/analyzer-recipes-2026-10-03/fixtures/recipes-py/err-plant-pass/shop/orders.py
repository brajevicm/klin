import json
import logging
import sqlite3
from dataclasses import dataclass

log = logging.getLogger(__name__)


class NotFound(Exception):
    pass


@dataclass(frozen=True)
class Item:
    sku: str
    cents: int
    quantity: int


@dataclass(frozen=True)
class Order:
    id: int
    items: list[Item]


def total(order: Order) -> int:
    result = 0
    for item in order.items:
        result += item.cents * item.quantity
    return result


def load(db: sqlite3.Connection, order_id: int) -> Order:
    row = db.execute("SELECT items FROM orders WHERE id = ?", (order_id,)).fetchone()
    if row is None:
        raise NotFound(f"order {order_id}")
    return Order(order_id, [Item(**item) for item in json.loads(row[0])])


def total_of(db: sqlite3.Connection, order_id: int) -> int:
    try:
        return total(load(db, order_id))
    except Exception:
        pass
    return 0
