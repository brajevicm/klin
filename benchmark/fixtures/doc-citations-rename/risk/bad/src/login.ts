import { audit, type AuditLog } from "./audit-log.ts";
import { verifyPassword } from "./password-hash.ts";
import { allow, type Attempts } from "./rate-limit.ts";
import { issueToken } from "./session-token.ts";
import { findUser, type UserStore } from "./user-store.ts";

export interface Service {
  users: UserStore;
  attempts: Attempts;
  log: AuditLog;
  secret: string;
}

export function login(service: Service, id: string, password: string, now: number): string | null {
  if (!allow(service.attempts, id, now)) {
    audit(service.log, `limited ${id}`, now);
    return null;
  }
  const user = findUser(service.users, id);
  if (!user || !verifyPassword(password, user.hash)) {
    audit(service.log, `refused ${id}`, now);
    return null;
  }
  audit(service.log, `login ${id}`, now);
  return issueToken(id, now, service.secret);
}
