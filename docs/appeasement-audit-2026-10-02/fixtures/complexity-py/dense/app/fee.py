def base_fee(order):
    return 0 if order["total"] > 100 else 5

REGION = {"EU": 4, "US": 6, "APAC": 9}
TIER = {"gold": -3, "silver": -1}
EXPRESS = [[0, 0], [6, 10]]


def shipping_fee(order):
    express = EXPRESS[int(order["express"])][int(order["items"] > 3)]
    bulk = 2 * int(order["items"] > 20 or order["total"] > 500)
    return max(0, base_fee(order) + REGION.get(order["region"], 12) + TIER.get(order["tier"], 0) + express + bulk)
