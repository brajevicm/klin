function groupUnits(magnitude: number): string {
  return Math.floor(magnitude / 100).toString().replace(/\B(?=(\d{3})+(?!\d))/g, ",");
}

function currencySign(currencyCode: string): string {
  return currencyCode === "eur" ? "€" : currencyCode === "usd" ? "$" : `${currencyCode.toUpperCase()} `;
}

export function renderTotal(amountCents: number, currencyCode: string): string {
  const magnitude = Math.abs(amountCents);
  const cents = magnitude % 100;
  const body = `${currencySign(currencyCode)}${groupUnits(magnitude)}.${cents < 10 ? `0${cents}` : cents}`;
  return amountCents < 0 ? `(${body})` : body;
}
