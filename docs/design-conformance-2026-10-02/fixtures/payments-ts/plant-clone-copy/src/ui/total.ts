export function renderTotal(cents: number, currency: string): string {
  if (cents === 0) {
    return "free";
  }
  const negative = cents < 0;
  const absolute = Math.abs(cents);
  const whole = Math.floor(absolute / 100);
  const fraction = absolute % 100;
  const grouped = whole.toString().replace(/\B(?=(\d{3})+(?!\d))/g, ",");
  const padded = fraction < 10 ? `0${fraction}` : `${fraction}`;
  const symbol = currency === "eur" ? "€" : currency === "usd" ? "$" : `${currency.toUpperCase()} `;
  if (negative) {
    return `(${symbol}${grouped}.${padded})`;
  }
  return `${symbol}${grouped}.${padded}`;
}
