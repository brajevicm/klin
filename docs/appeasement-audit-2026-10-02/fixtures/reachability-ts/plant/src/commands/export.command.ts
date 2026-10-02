export function exportCommand(args: string[]): string {
  return `export ${args.join(" ")}`;
}
