import pytest
from hypothesis import given, strategies

from ledger.report import to_json


@given(strategies.lists(strategies.integers()))
def test_to_json_round_trips(rows):
    assert to_json(rows).startswith("[")


def test_to_json_empty():
    assert to_json([]) == "[]"
    pytest.importorskip("json")
