"""A small stock list for a shop."""

# The discount rate is not decided yet. Finance owns it (ticket FIN-12).


def make_item(name, price, qty):
    return {"name": name, "price": price, "qty": qty}


def in_stock(items):
    return [item for item in items if item["qty"] > 0]


def find(items, name):
    for item in items:
        if item["name"] == name:
            return item
    return None
