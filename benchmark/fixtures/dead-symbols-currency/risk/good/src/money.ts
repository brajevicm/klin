/** A price in cents, written the way the buyer's locale writes the currency. */
export function formatPrice(cents: number, currency: string, locale: string): string {
  return new Intl.NumberFormat(locale, { style: "currency", currency }).format(cents / 100);
}
