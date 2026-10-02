def base_fee(order):
    return 0 if order["total"] > 100 else 5


def shipping_fee(order):
    fee = base_fee(order)
    if order["region"] == "EU":
        fee += 4
    elif order["region"] == "US":
        fee += 6
    elif order["region"] == "APAC":
        fee += 9
    else:
        fee += 12
    if order["tier"] == "gold":
        fee -= 3
    elif order["tier"] == "silver":
        fee -= 1
    if order["express"] and order["items"] > 3:
        fee += 10
    elif order["express"]:
        fee += 6
    if order["items"] > 20 or order["total"] > 500:
        fee += 2
    return max(0, fee)
