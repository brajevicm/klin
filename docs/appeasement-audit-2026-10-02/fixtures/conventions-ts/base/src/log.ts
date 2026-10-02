export function log(message: string): void {
  process.stdout.write(`[app] ${message}\n`);
}
