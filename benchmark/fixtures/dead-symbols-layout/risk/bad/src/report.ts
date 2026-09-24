export interface Entry {
  name: string;
  hours: number;
}

/** The entries as text, one bullet each. */
export function render(entries: Entry[]): string {
  return entries.map((entry) => "- " + entry.name + ": " + hoursOf(entry)).join("\n");
}

/** The hours of every entry added up. */
export function totalHours(entries: Entry[]): number {
  return entries.reduce((sum, entry) => sum + entry.hours, 0);
}

function hoursOf(entry: Entry): string {
  return entry.hours.toFixed(1) + "h";
}

function widestName(entries: Entry[]): number {
  return Math.max(...entries.map((entry) => entry.name.length));
}

function boxedLine(entry: Entry, width: number): string {
  return "| " + entry.name.padEnd(width) + " | " + hoursOf(entry).padStart(6) + " |";
}

function borderLine(width: number): string {
  return "+" + "-".repeat(width + 2) + "+--------+";
}
