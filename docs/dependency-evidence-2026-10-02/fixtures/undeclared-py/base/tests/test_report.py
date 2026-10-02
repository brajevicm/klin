import pytest

from ledger.report import parse_day


def test_parse_day():
    assert parse_day("2026-10-02T10:00:00").isoformat() == "2026-10-02"


def test_parse_day_rejects_text():
    with pytest.raises(ValueError):
        parse_day("soon")
