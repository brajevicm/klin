export function listCommand(args: string[]): string {
  return `list ${args.join(" ")}`;
}
