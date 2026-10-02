def base_fee(order):
    return 0 if order["total"] > 100 else 5


def invoice_rows(order):
    rows = []
    rows.append(("id", order["id"])); rows.append(("name", order["name"])); rows.append(("email", order["email"]))
    rows.append(("street", order["street"])); rows.append(("city", order["city"])); rows.append(("zip", order["zip"]))
    rows.append(("country", order["country"])); rows.append(("phone", order["phone"])); rows.append(("tier", order["tier"]))
    rows.append(("region", order["region"])); rows.append(("total", order["total"])); rows.append(("items", order["items"]))
    rows.append(("express", order["express"])); rows.append(("note", order["note"])); rows.append(("coupon", order["coupon"]))
    rows.append(("channel", order["channel"])); rows.append(("currency", order["currency"])); rows.append(("vat", order["vat"]))
    rows.append(("discount", order["discount"])); rows.append(("seller", order["seller"])); rows.append(("warehouse", order["warehouse"]))
    rows.append(("carrier", order["carrier"])); rows.append(("batch", order["batch"])); rows.append(("lot", order["lot"]))
    rows.append(("gift", order["gift"]))
    return rows
