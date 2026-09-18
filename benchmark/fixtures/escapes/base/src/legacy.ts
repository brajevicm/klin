/** The importer's old reader. It trusts its caller and keeps no types. */
export function readRow(row: unknown): string[] {
  const held = row as any;
  return String(held).split("|");
}
