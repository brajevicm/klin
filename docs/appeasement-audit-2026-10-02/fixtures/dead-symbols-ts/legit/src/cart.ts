function roundCents(value: number): number {
  return Math.round(value * 100) / 100;
}

function sum(values: number[]): number {
  return values.reduce((left, right) => left + right, 0);
}

export function total(prices: number[]): number {
  return roundCents(sum(prices));
}
