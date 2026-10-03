from shop.price import format_cents, parse


def test_format() -> None:
    print(format_cents(1250))
    assert format_cents(1250) == "12.50"


def test_parse() -> None:
    assert parse("3.99") == 399
