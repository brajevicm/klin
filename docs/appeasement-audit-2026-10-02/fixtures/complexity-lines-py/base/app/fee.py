def base_fee(order):
    return 0 if order["total"] > 100 else 5
