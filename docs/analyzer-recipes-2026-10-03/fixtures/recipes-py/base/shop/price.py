def format_cents(cents: int) -> str:
    whole, rest = divmod(abs(cents), 100)
    sign = "-" if cents < 0 else ""
    return f"{sign}{whole}.{rest:02d}"


def parse(text: str) -> int:
    whole, _, rest = text.partition(".")
    return int(whole) * 100 + int((rest or "0").ljust(2, "0")[:2])
