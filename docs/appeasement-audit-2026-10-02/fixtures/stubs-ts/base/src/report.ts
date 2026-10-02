export type Row = { name: string; total: number };

export function toJson(rows: Row[]): string {
  return JSON.stringify(rows);
}
