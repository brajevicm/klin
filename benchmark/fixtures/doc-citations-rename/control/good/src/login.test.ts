import { test } from "node:test";
import assert from "node:assert/strict";
import { addUser, hashPassword, login, logout, readToken, type Service } from "./index.ts";

function service(): Service {
  const held: Service = { users: new Map(), attempts: new Map(), log: [], secret: "s3cret" };
  addUser(held.users, "ada", hashPassword("lovelace", "salt1"));
  return held;
}

test("a right password gives a token that reads back as the user", () => {
  const held = service();
  const token = login(held, "ada", "lovelace", 0);
  assert.ok(token);
  assert.equal(readToken(token, "s3cret"), "ada");
  assert.equal(readToken(token, "other"), null);
  assert.deepEqual(held.log, ["1970-01-01T00:00:00.000Z login ada"]);
});

test("a wrong password or an unknown user gives nothing", () => {
  const held = service();
  assert.equal(login(held, "ada", "babbage", 0), null);
  assert.equal(login(held, "bob", "x", 0), null);
});

test("a sixth attempt within a minute is limited", () => {
  const held = service();
  for (let tried = 0; tried < 5; tried += 1) {
    login(held, "ada", "wrong", tried);
  }
  assert.equal(login(held, "ada", "lovelace", 10), null);
  assert.ok(login(held, "ada", "lovelace", 60_010));
});

test("a signed-in user signs out once the token reads", () => {
  const held = service();
  const token = login(held, "ada", "lovelace", 0) ?? "";
  assert.equal(logout(held, token, 1000), true);
  assert.equal(logout(held, "nonsense", 1000), false);
  assert.equal(held.log.at(-1), "1970-01-01T00:00:01.000Z logout ada");
});
