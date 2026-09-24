function parse(date: string): Date {
  return new Date(date + "T00:00:00Z");
}

function format(date: Date): string {
  return date.toISOString().slice(0, 10);
}

function day(year: number, month: number, date: number): Date {
  return new Date(Date.UTC(year, month - 1, date));
}

function shifted(date: Date, days: number): Date {
  return new Date(date.getTime() + days * 86_400_000);
}

function isWeekend(date: Date): boolean {
  const weekday = date.getUTCDay();
  return weekday === 0 || weekday === 6;
}

/** Easter Sunday of the years the payroll office plans for. */
// TODO: compute Easter for a year outside this table
const EASTER: Record<number, string> = {
  2024: "2024-03-31",
  2025: "2025-04-20",
  2026: "2026-04-05",
  2027: "2027-03-28",
  2028: "2028-04-16",
  2029: "2029-04-01",
  2030: "2030-04-21",
};

function firstMonday(year: number, month: number): Date {
  const first = day(year, month, 1);
  return shifted(first, (8 - first.getUTCDay()) % 7);
}

function lastMonday(year: number, month: number): Date {
  const last = day(year, month + 1, 0);
  return shifted(last, -((last.getUTCDay() + 6) % 7));
}

/** The bank holidays of England and Wales in one year, each as YYYY-MM-DD. */
function bankHolidays(year: number): Set<string> {
  const held = new Set([firstMonday(year, 5), lastMonday(year, 5), lastMonday(year, 8)].map(format));
  const known = EASTER[year];
  if (known) {
    const sunday = parse(known);
    held.add(format(shifted(sunday, -2)));
    held.add(format(shifted(sunday, 1)));
  }
  const movable = [day(year, 1, 1), day(year, 12, 25), day(year, 12, 26)];
  for (const date of movable.filter((one) => !isWeekend(one))) {
    held.add(format(date));
  }
  for (const date of movable.filter(isWeekend)) {
    let instead = date;
    while (isWeekend(instead) || held.has(format(instead))) {
      instead = shifted(instead, 1);
    }
    held.add(format(instead));
  }
  return held;
}

function isWorkingDay(date: Date): boolean {
  return !isWeekend(date) && !bankHolidays(date.getUTCFullYear()).has(format(date));
}

/**
 * The date `days` working days after `start`. A working day is a Monday to a Friday that is not
 * a bank holiday of England and Wales.
 */
export function addWorkingDays(start: string, days: number): string {
  let date = parse(start);
  let left = days;
  while (left > 0) {
    date = shifted(date, 1);
    if (isWorkingDay(date)) {
      left -= 1;
    }
  }
  return format(date);
}
