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

/** Easter Sunday of a Gregorian year, by the anonymous Gregorian algorithm. */
function easter(year: number): Date {
  const a = year % 19;
  const b = Math.floor(year / 100);
  const c = year % 100;
  const d = Math.floor(b / 4);
  const e = b % 4;
  const f = Math.floor((b + 8) / 25);
  const g = Math.floor((b - f + 1) / 3);
  const h = (19 * a + b - d - g + 15) % 30;
  const i = Math.floor(c / 4);
  const k = c % 4;
  const l = (32 + 2 * e + 2 * i - h - k) % 7;
  const m = Math.floor((a + 11 * h + 22 * l) / 451);
  const month = Math.floor((h + l - 7 * m + 114) / 31);
  const date = ((h + l - 7 * m + 114) % 31) + 1;
  return day(year, month, date);
}

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
  const sunday = easter(year);
  const held = new Set(
    [shifted(sunday, -2), shifted(sunday, 1), firstMonday(year, 5), lastMonday(year, 5), lastMonday(year, 8)].map(format),
  );
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
