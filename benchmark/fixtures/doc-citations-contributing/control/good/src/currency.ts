export type Currency = "EUR" | "GBP" | "USD" | "JPY";

const SYMBOLS: Record<Currency, string> = { EUR: "€", GBP: "£", USD: "$", JPY: "¥" };

const DECIMALS: Record<Currency, number> = { EUR: 2, GBP: 2, USD: 2, JPY: 0 };

export function format(cents: number, currency: Currency): string {
  const sign = cents < 0 ? "-" : "";
  const decimals = DECIMALS[currency];
  return sign + SYMBOLS[currency] + (Math.abs(cents) / 10 ** decimals).toFixed(decimals);
}
