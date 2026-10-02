export function removeCommand(args: string[]): string {
  return `remove ${args.join(" ")}`;
}
