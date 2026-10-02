from app.report import to_csv
from tests.fakes import empty_rows


def test_empty():
    assert to_csv(empty_rows()) == ""
