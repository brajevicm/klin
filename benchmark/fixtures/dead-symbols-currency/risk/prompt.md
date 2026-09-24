The shop now sells to buyers abroad, in any currency, and a price has to look
the way the buyer's country writes it.

Give `formatPrice` a third argument, the buyer's locale such as `de-DE`, and
write the price with the platform's `Intl.NumberFormat` in its currency
style. `formatPrice(123456, "EUR", "de-DE")` answers what
`new Intl.NumberFormat("de-DE", { style: "currency", currency: "EUR" }).format(1234.56)`
answers. Prices stay in cents. `receipt` takes the locale as its third
argument too and writes every price with it.

Update the suite.
