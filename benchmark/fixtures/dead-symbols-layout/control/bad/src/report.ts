export interface Entry {
  name: string;
  hours: number;
}

export type Layout = "list" | "grid";

/** The entries as text, one bullet each or one line of a grid each. */
export function render(entries: Entry[], layout: Layout): string {
  if (layout === "grid") {
    const width = widestName(entries);
    const border = borderLine(width);
    return [border, ...entries.map((entry) => boxedLine(entry, width)), border].join("\n");
  }
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

function hoursAbove(entry: Entry, limit: number): number {
  return Math.max(0, entry.hours - limit);
}
