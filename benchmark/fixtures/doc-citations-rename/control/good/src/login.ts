import { audit, type AuditLog } from "./auditLog.ts";
import { verifyPassword } from "./passwordHash.ts";
import { allow, type Attempts } from "./rateLimit.ts";
import { issueToken, readToken } from "./sessionToken.ts";
import { findUser, type UserStore } from "./userStore.ts";

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

export function logout(service: Service, token: string, now: number): boolean {
  const id = readToken(token, service.secret);
  if (id === null) {
    return false;
  }
  audit(service.log, `logout ${id}`, now);
  return true;
}
