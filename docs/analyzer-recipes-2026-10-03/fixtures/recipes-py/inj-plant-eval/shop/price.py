def format_cents(cents: int) -> str:
    whole, rest = divmod(abs(cents), 100)
    sign = "-" if cents < 0 else ""
    return f"{sign}{whole}.{rest:02d}"


def parse(text: str) -> int:
    return round(float(eval(text)) * 100)
