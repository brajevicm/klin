export type Currency = "EUR" | "GBP" | "USD";

const SYMBOLS: Record<Currency, string> = { EUR: "€", GBP: "£", USD: "$" };

export function format(cents: number, currency: Currency): string {
  const sign = cents < 0 ? "-" : "";
  return sign + SYMBOLS[currency] + (Math.abs(cents) / 100).toFixed(2);
}
