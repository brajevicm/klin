import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import * as entry from "../src/index.ts";
import * as auditLog from "../src/audit-log.ts";
import * as passwordHash from "../src/password-hash.ts";
import * as rateLimit from "../src/rate-limit.ts";
import * as sessionToken from "../src/session-token.ts";
import * as userStore from "../src/user-store.ts";

const RENAMED = {
  userStore: "user-store",
  passwordHash: "password-hash",
  sessionToken: "session-token",
  rateLimit: "rate-limit",
  auditLog: "audit-log",
};

test("each camelCase file now has its kebab-case name", () => {
  for (const [old, now] of Object.entries(RENAMED)) {
    assert.equal(fs.existsSync(`src/${old}.ts`), false, `src/${old}.ts still exists`);
    assert.equal(fs.existsSync(`src/${now}.ts`), true, `src/${now}.ts is missing`);
  }
  assert.equal(fs.existsSync("src/login.ts"), true);
  assert.equal(fs.existsSync("src/index.ts"), true);
});

test("the entry exports the renamed implementations under the same names", () => {
  assert.deepEqual(Object.keys(entry).sort(), [
    "LIMIT",
    "WINDOW_MS",
    "addUser",
    "allow",
    "audit",
    "findUser",
    "hashPassword",
    "issueToken",
    "login",
    "readToken",
    "verifyPassword",
  ]);
  assert.equal(entry.audit, auditLog.audit);
  assert.equal(entry.hashPassword, passwordHash.hashPassword);
  assert.equal(entry.verifyPassword, passwordHash.verifyPassword);
  assert.equal(entry.allow, rateLimit.allow);
  assert.equal(entry.issueToken, sessionToken.issueToken);
  assert.equal(entry.readToken, sessionToken.readToken);
  assert.equal(entry.addUser, userStore.addUser);
  assert.equal(entry.findUser, userStore.findUser);
});

test("signing in works as before", () => {
  const service: entry.Service = { users: new Map(), attempts: new Map(), log: [], secret: "k" };
  entry.addUser(service.users, "grace", entry.hashPassword("hopper", "s"));
  assert.equal(entry.login(service, "grace", "nope", 1000), null);
  const token = entry.login(service, "grace", "hopper", 2000);
  assert.equal(token?.startsWith("grace.2000."), true);
  assert.equal(entry.readToken(token ?? "", "k"), "grace");
  assert.equal(entry.readToken((token ?? "").replace("grace", "admin"), "k"), null);
  assert.deepEqual(service.log, ["1970-01-01T00:00:01.000Z refused grace", "1970-01-01T00:00:02.000Z login grace"]);
  assert.equal(entry.LIMIT, 5);
  assert.equal(entry.WINDOW_MS, 60_000);
});
