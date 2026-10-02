export interface Row {
  name: string;
  amount: number;
}

export function toJson(rows: Row[]): string {
  return JSON.stringify(rows);
}

export class UnsupportedFormat extends Error {}

export function toCsv(rows: Row[]): string {
  throw new UnsupportedFormat("csv");
}
