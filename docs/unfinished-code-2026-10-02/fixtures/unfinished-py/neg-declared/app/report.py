import json


def to_json(rows):
    return json.dumps(rows)


def to_csv(rows):
    return "\n".join(f"{row['name']},{row['amount']}" for row in rows)


from abc import ABC, abstractmethod
from typing import Protocol, overload


class Sink(Protocol):
    def lines(self) -> list:
        ...


class BaseSink(ABC):
    @abstractmethod
    def lines(self) -> list:
        ...


@overload
def scale(value: int) -> int: ...
@overload
def scale(value: float) -> float: ...
def scale(value):
    return value * 2
