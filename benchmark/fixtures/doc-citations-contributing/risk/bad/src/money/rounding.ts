export function halfEven(value: number): number {
  const floor = Math.floor(value);
  const rest = value - floor;
  if (Math.abs(rest - 0.5) > 1e-9) {
    return Math.round(value);
  }
  return floor % 2 === 0 ? floor : floor + 1;
}
