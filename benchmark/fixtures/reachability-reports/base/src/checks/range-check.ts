const DAY = /^\d{4}-\d{2}-\d{2}$/;

function isDay(value: string | undefined): value is string {
  return value !== undefined && DAY.test(value) && !Number.isNaN(Date.parse(value));
}

export function checkRange(from: string | undefined, to: string | undefined): string | null {
  if (!isDay(from) || !isDay(to)) {
    return "from and to must be days like 2026-01-31";
  }
  return from <= to ? null : "from must not be after to";
}
