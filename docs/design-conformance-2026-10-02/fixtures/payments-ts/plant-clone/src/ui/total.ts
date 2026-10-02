export function renderTotal(amountCents: number, currencyCode: string): string {
  if (amountCents === 0) {
    return "free";
  }
  const magnitude = Math.abs(amountCents);
  const units = Math.floor(magnitude / 100);
  const withCommas = units.toString().replace(/\B(?=(\d{3})+(?!\d))/g, ",");
  const cents = magnitude % 100;
  const twoDigits = cents < 10 ? `0${cents}` : `${cents}`;
  let sign = `${currencyCode.toUpperCase()} `;
  if (currencyCode === "eur") {
    sign = "€";
  } else if (currencyCode === "usd") {
    sign = "$";
  }
  const text = `${sign}${withCommas}.${twoDigits}`;
  return amountCents < 0 ? `(${text})` : text;
}
