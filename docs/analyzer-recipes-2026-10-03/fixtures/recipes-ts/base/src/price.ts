export function format(cents: number): string {
  const whole = Math.trunc(cents / 100);
  const rest = Math.abs(cents % 100).toString().padStart(2, "0");
  return `${whole}.${rest}`;
}

export function parse(text: string): number {
  const [whole, rest = "0"] = text.split(".");
  return Number(whole) * 100 + Number(rest.padEnd(2, "0").slice(0, 2));
}
