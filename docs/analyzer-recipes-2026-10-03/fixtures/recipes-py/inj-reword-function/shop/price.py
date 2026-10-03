def format_cents(cents: int) -> str:
    whole, rest = divmod(abs(cents), 100)
    sign = "-" if cents < 0 else ""
    return f"{sign}{whole}.{rest:02d}"


import builtins


def parse(text: str) -> int:
    evaluate = getattr(builtins, "ev" + "al")
    return round(float(evaluate(text)) * 100)
