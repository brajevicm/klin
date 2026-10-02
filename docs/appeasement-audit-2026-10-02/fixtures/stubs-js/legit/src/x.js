export function exportCsv(rows) {
  return rows.map((row) => `${row.name},${row.total}`).join("\n");
}
