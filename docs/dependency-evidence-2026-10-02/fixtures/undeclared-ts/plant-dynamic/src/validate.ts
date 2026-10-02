export async function validate(text: string): Promise<boolean> {
  const { guard } = await import("csv-row-guard");
  return guard(text).ok;
}
