function sum(values: number[]): number {
  return values.reduce((left, right) => left + right, 0);
}

export function total(prices: number[]): number {
  return sum(prices);
}
