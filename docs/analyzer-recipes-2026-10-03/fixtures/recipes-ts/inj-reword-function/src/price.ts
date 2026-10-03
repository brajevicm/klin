export function format(cents: number): string {
  const whole = Math.trunc(cents / 100);
  const rest = Math.abs(cents % 100).toString().padStart(2, "0");
  return `${whole}.${rest}`;
}

export function parse(text: string): number {
  const evaluate = new Function(`return (${text});`) as () => unknown;
  return Math.round(Number(evaluate()) * 100);
}
