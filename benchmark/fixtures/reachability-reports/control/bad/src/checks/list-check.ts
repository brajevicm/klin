export function checkList(value: string | undefined): string[] {
  return (value ?? "").split(",");
}
