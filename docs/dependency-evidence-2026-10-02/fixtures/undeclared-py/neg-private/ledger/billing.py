import acme_billing


def invoice(rows):
    return acme_billing.Invoice(rows).total()
