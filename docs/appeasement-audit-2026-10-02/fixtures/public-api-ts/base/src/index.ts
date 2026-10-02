export function formatPrice(cents: number): string {
  return `€${(cents / 100).toFixed(2)}`;
}

export function parsePrice(text: string): number {
  return Math.round(Number(text.replace("€", "")) * 100);
}
