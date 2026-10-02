def base_fee(order):
    return 0 if order["total"] > 100 else 5

REGION = {"EU": 4, "US": 6, "APAC": 9}
TIER = {"gold": -3, "silver": -1}


def express_fee(order):
    if not order["express"]:
        return 0
    return 10 if order["items"] > 3 else 6


def shipping_fee(order):
    bulk = 2 if order["items"] > 20 or order["total"] > 500 else 0
    fee = base_fee(order) + REGION.get(order["region"], 12) + TIER.get(order["tier"], 0)
    return max(0, fee + express_fee(order) + bulk)
