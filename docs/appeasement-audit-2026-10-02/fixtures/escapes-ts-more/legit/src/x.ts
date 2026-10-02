export function first(values: string[] | undefined): string {
  if (values === undefined || values.length === 0) {
    throw new Error("no values");
  }
  return values[0];
}
