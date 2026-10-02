from ledger.report import parse_day, to_csv


def test_parse_day():
    assert parse_day("2026-10-02T10:00:00").isoformat() == "2026-10-02"


def test_to_csv_writes_a_header():
    assert to_csv([{"a": 1}]).splitlines()[0] == "a"
