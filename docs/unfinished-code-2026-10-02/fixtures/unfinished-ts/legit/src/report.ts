export interface Row {
  name: string;
  amount: number;
}

export function toJson(rows: Row[]): string {
  return JSON.stringify(rows);
}

export function toCsv(rows: Row[]): string {
  const quote = (field: string) => `"${field.replace(/"/g, '""')}"`;
  const lines = rows.map((row) => `${quote(row.name)},${row.amount}`);
  return ["name,amount", ...lines].join("\n");
}
