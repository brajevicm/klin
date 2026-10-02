export interface Row {
  name: string;
  amount: number;
}

export function toJson(rows: Row[]): string {
  return JSON.stringify(rows);
}

export function toCsv(rows: Row[]): string {
  return [...headerLines("csv"), ...rows.map((row) => `${row.name},${row.amount}`)].join("\n");
}

export function headerLines(format: "csv" | "json"): string[] {
  return format === "csv" ? ["name,amount"] : [];
}

export function footerLines(): string[] {
  return [];
}
