import { render, totalHours, type Entry, type Layout } from "./report.ts";

/** The week's report, with the total below it. */
export function weekly(entries: Entry[], layout: Layout): string {
  return render(entries, layout) + "\nTotal: " + totalHours(entries).toFixed(1) + "h";
}
