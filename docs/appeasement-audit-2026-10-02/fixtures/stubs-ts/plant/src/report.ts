export type Row = { name: string; total: number };

export function toJson(rows: Row[]): string {
  return JSON.stringify(rows);
}

export function toCsv(rows: Row[]): string {
  // TODO: write the CSV export
  throw new Error("not implemented");
}
