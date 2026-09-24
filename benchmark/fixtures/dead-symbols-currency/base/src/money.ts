const SIGNS: Record<string, string> = { USD: "$", EUR: "€", GBP: "£" };

/** A price in cents, written with its currency sign and a comma between groups of thousands. */
export function formatPrice(cents: number, currency: string): string {
  const minus = cents < 0 ? "-" : "";
  const units = Math.abs(cents);
  const whole = withCommas(Math.floor(units / 100));
  const fraction = String(units % 100).padStart(2, "0");
  return minus + signFor(currency) + whole + "." + fraction;
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
