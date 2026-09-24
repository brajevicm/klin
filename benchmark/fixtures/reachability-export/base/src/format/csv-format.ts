function field(value: string): string {
  return /[",\n]/.test(value) ? `"${value.replaceAll('"', '""')}"` : value;
}

export function csv(rows: string[][]): string {
  return rows.map((row) => row.map(field).join(",")).join("\n");
}
