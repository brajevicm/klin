import { createHmac } from "node:crypto";

function sign(body: string, secret: string): string {
  return createHmac("sha256", secret).update(body).digest("hex").slice(0, 32);
}

export function issueToken(userId: string, issuedAt: number, secret: string): string {
  const body = `${userId}.${issuedAt}`;
  return `${body}.${sign(body, secret)}`;
}

export function readToken(token: string, secret: string): string | null {
  const cut = token.lastIndexOf(".");
  const body = token.slice(0, cut);
  return cut > 0 && sign(body, secret) === token.slice(cut + 1) ? body.split(".")[0] : null;
}
