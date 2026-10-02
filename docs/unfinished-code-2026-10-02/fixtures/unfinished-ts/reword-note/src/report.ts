export interface Row {
  name: string;
  amount: number;
}

export function toJson(rows: Row[]): string {
  return JSON.stringify(rows);
}

export function toCsv(rows: Row[]): string {
  // Note: CSV export lands in a later release.
  throw new UnsupportedFormat("csv");
}

export class UnsupportedFormat extends Error {}
