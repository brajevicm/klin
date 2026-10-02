export type Row = { name: string; total: number };

export function toJson(rows: Row[]): string {
  return JSON.stringify(rows);
}

export function toCsv(rows: Row[]): string {
  const lines = rows.map((row) => `${row.name},${row.total}`);
  return ["name,total", ...lines].join("\n");
}
