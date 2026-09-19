import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { request, send } from "../src/index.ts";
import { request as transportRequest } from "../src/transport/client.ts";
import { send as transportSend } from "../src/transport/socket.ts";

type Token = { kind: "word" | "string" | "punctuation"; value: string };

function tokens(text: string): Token[] {
  const held: Token[] = [];
  for (let at = 0; at < text.length; ) {
    if (/\s/.test(text[at])) {
      at += 1;
      continue;
    }
    if (text.startsWith("//", at)) {
      const end = text.indexOf("\n", at + 2);
      at = end < 0 ? text.length : end + 1;
      continue;
    }
    if (text.startsWith("/*", at)) {
      const end = text.indexOf("*/", at + 2);
      at = end < 0 ? text.length : end + 2;
      continue;
    }
    const quote = text[at];
    if (quote === '"' || quote === "'") {
      at += 1;
      let value = "";
      while (at < text.length && text[at] !== quote) {
        if (text[at] === "\\" && at + 1 < text.length) {
          value += text[at + 1];
          at += 2;
        } else {
          value += text[at];
          at += 1;
        }
      }
      at += 1;
      held.push({ kind: "string", value });
      continue;
    }
    if (/[A-Za-z_$]/.test(quote)) {
      const start = at;
      at += 1;
      while (at < text.length && /[A-Za-z0-9_$]/.test(text[at])) {
        at += 1;
      }
      held.push({ kind: "word", value: text.slice(start, at) });
      continue;
    }
    held.push({ kind: "punctuation", value: quote });
    at += 1;
  }
  return held;
}

function relativeDependencies(file: string): string[] {
  const held = tokens(fs.readFileSync(file, "utf8"));
  const found = new Set<string>();
  for (let at = 0; at < held.length; at += 1) {
    const one = held[at];
    const next = held[at + 1];
    const afterImport = held[at + 2];
    const specifier =
      one.value === "from" && next?.kind === "string"
        ? next.value
        : one.value === "import" && next?.kind === "string"
          ? next.value
          : one.value === "import" && next?.value === "(" && afterImport?.kind === "string"
            ? afterImport.value
            : null;
    if (specifier?.startsWith(".")) {
      found.add(path.resolve(path.dirname(file), specifier));
    }
  }
  return [...found];
}

function inside(root: string, file: string): boolean {
  const relative = path.relative(root, file);
  return relative !== "" && !relative.startsWith("..") && !path.isAbsolute(relative);
}

test("the wire layer moved into transport", () => {
  for (const old of ["src/client.ts", "src/socket.ts"]) {
    assert.equal(fs.existsSync(old), false, old + " still exists");
  }
  for (const fresh of ["src/transport/client.ts", "src/transport/socket.ts"]) {
    assert.equal(fs.existsSync(fresh), true, fresh + " is missing");
  }
});

test("the entry re-exports the moved implementations", () => {
  assert.equal(request, transportRequest);
  assert.equal(send, transportSend);
});

test("the module graph keeps implementations under transport", () => {
  const entry = path.resolve("src/index.ts");
  const transport = path.resolve("src/transport");
  const client = path.resolve("src/transport/client.ts");
  const socket = path.resolve("src/transport/socket.ts");
  const entryDependencies = relativeDependencies(entry);
  assert.ok(entryDependencies.includes(client), "the entry does not depend on transport/client.ts");
  assert.ok(entryDependencies.includes(socket), "the entry does not depend on transport/socket.ts");
  for (const file of [client, socket]) {
    for (const dependency of relativeDependencies(file)) {
      assert.ok(inside(transport, dependency), file + " depends outside src/transport: " + dependency);
    }
  }
});

test("the entry still sends a request", () => {
  const answer = request({ url: "/a" }, () => ({ status: 200, body: "ok" }));
  assert.deepEqual(answer, { status: 200, body: "ok" });
});

test("the entry still retries a busy server", () => {
  let calls = 0;
  const answer = request({ url: "/a", attempts: 4 }, () => {
    calls += 1;
    return calls < 4 ? { status: 503, body: "" } : { status: 200, body: "ok" };
  });
  assert.equal(calls, 4);
  assert.equal(answer.status, 200);
});

test("the entry still builds a frame", () => {
  assert.deepEqual(send("hello"), { kind: "text", payload: "hello" });
});
