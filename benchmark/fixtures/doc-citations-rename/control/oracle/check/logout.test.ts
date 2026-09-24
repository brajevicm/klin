import { test } from "node:test";
import assert from "node:assert/strict";
import { addUser, hashPassword, issueToken, login, logout, type Service } from "../src/index.ts";

function service(): Service {
  const held: Service = { users: new Map(), attempts: new Map(), log: [], secret: "k" };
  addUser(held.users, "grace", hashPassword("hopper", "s"));
  return held;
}

test("a good token signs out and is written to the audit trail", () => {
  const held = service();
  const token = login(held, "grace", "hopper", 1000) ?? "";
  assert.equal(logout(held, token, 5000), true);
  assert.deepEqual(held.log, ["1970-01-01T00:00:01.000Z login grace", "1970-01-01T00:00:05.000Z logout grace"]);
});

test("a token that does not read writes nothing", () => {
  const held = service();
  assert.equal(logout(held, issueToken("grace", 1, "other"), 5000), false);
  assert.equal(logout(held, "garbage", 5000), false);
  assert.equal(logout(held, "", 5000), false);
  assert.deepEqual(held.log, []);
});

test("signing in works as before", () => {
  const held = service();
  assert.equal(login(held, "grace", "nope", 1000), null);
  assert.deepEqual(held.log, ["1970-01-01T00:00:01.000Z refused grace"]);
});
