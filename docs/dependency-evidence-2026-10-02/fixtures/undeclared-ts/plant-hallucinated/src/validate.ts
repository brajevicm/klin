import { guard } from "csv-row-guard";

export function validate(text: string): boolean {
  return guard(text).ok;
}
