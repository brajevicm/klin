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
