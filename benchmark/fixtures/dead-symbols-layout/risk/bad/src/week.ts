import { render, totalHours, type Entry } from "./report.ts";

/** The week's report, with the total below it. */
export function weekly(entries: Entry[]): string {
  return render(entries) + "\nTotal: " + totalHours(entries).toFixed(1) + "h";
}
