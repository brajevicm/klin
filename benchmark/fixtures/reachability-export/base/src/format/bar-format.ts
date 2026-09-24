export function bar(label: string, count: number, widest: number): string {
  return `${label.padEnd(widest)}  ${"#".repeat(count)} ${count}`;
}
