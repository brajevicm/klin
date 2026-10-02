export type Row = { name: string; total: number };

export class UnsupportedFormat extends Error {}

export function toJson(rows: Row[]): string {
  return JSON.stringify(rows);
}

export function toCsv(rows: Row[]): string {
  // Note: the CSV export follows in a later release.
  throw new UnsupportedFormat("csv");
}
