def base_fee(order):
    return 0 if order["total"] > 100 else 5


INVOICE_FIELDS = (
    "id",
    "name",
    "email",
    "street",
    "city",
    "zip",
    "country",
    "phone",
    "tier",
    "region",
    "total",
    "items",
    "express",
    "note",
    "coupon",
    "channel",
    "currency",
    "vat",
    "discount",
    "seller",
    "warehouse",
    "carrier",
    "batch",
    "lot",
    "gift",
)


def invoice_rows(order):
    return [(field, order[field]) for field in INVOICE_FIELDS]
