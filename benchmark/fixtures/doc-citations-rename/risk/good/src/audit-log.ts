export type AuditLog = string[];

export function audit(log: AuditLog, event: string, now: number): void {
  log.push(`${new Date(now).toISOString()} ${event}`);
}
