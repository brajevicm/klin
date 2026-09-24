import { createHash, timingSafeEqual } from "node:crypto";

function digest(salt: string, password: string): string {
  return createHash("sha256").update(salt + ":" + password).digest("hex");
}

export function hashPassword(password: string, salt: string): string {
  return salt + ":" + digest(salt, password);
}

export function verifyPassword(password: string, hash: string): boolean {
  const [salt, held] = hash.split(":");
  const fresh = digest(salt, password);
  return held.length === fresh.length && timingSafeEqual(Buffer.from(held), Buffer.from(fresh));
}
