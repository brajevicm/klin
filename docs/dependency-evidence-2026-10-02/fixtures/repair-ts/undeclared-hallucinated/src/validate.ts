import { guard } from "csv-row-guard";

export function validate(rows: unknown[]): void {
  const result = guard(rows, { inferSchema: true });
  if (!result.ok) {
    throw new Error(`invalid row: ${result.errors[0]}`);
  }
}
