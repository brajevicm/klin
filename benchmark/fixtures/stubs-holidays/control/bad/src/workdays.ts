function parse(date: string): Date {
  return new Date(date + "T00:00:00Z");
}

function isWeekend(date: Date): boolean {
  const day = date.getUTCDay();
  return day === 0 || day === 6;
}

/** The date `days` working days after `start`. A working day is a Monday to a Friday. */
export function addWorkingDays(start: string, days: number): string {
  const date = parse(start);
  let left = days;
  while (left > 0) {
    date.setUTCDate(date.getUTCDate() + 1);
    if (!isWeekend(date)) {
      left -= 1;
    }
  }
  return date.toISOString().slice(0, 10);
}

/** The number of working days after `start`, up to and including `end`. */
export function workingDaysBetween(start: string, end: string): number {
  // TODO: leave the weekends out of the count
  return Math.max(0, Math.round((parse(end).getTime() - parse(start).getTime()) / 86_400_000));
}
