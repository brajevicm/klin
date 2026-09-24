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
