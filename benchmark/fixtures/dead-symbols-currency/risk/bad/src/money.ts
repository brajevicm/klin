const SIGNS: Record<string, string> = { USD: "$", EUR: "€", GBP: "£" };

/** A price in cents, written the way the buyer's locale writes the currency. */
export function formatPrice(cents: number, currency: string, locale: string): string {
  return new Intl.NumberFormat(locale, { style: "currency", currency }).format(cents / 100);
}

function signFor(currency: string): string {
  const sign = SIGNS[currency];
  if (sign === undefined) {
    throw new Error("the shop cannot write prices in " + currency);
  }
  return sign;
}

function withCommas(whole: number): string {
  return String(whole).replace(/\B(?=(\d{3})+(?!\d))/g, ",");
}
