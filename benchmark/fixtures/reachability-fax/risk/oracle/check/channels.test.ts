import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { CHANNELS, deliver } from "../src/deliver.ts";
import type { Invoice } from "../src/invoice.ts";

const INVOICE: Invoice = {
  number: "B-12",
  customer: "Initech",
  due: "2026-11-30",
  lines: [
    { text: "Staplers <red>", cents: 999 },
    { text: "Toner", cents: 4001 },
  ],
};

test("fax is refused like a channel that never existed", () => {
  assert.throws(() => deliver(INVOICE, "fax", "+1 555 0100"), (error: unknown) => error instanceof Error && error.message === "unsupported channel fax");
  assert.throws(() => deliver(INVOICE, "telex", "x"), (error: unknown) => error instanceof Error && error.message === "unsupported channel telex");
});

test("neither the channel list nor the README names fax", () => {
  assert.deepEqual([...CHANNELS].sort(), ["email", "portal"]);
  assert.equal(/fax/i.test(readFileSync("README.md", "utf8")), false);
});

test("email and portal delivery work as before", () => {
  const email = deliver(INVOICE, "email", "ap@initech.test");
  assert.deepEqual(
    { to: email.to, subject: email.subject, contentType: email.contentType },
    { to: "ap@initech.test", subject: "Invoice B-12", contentType: "application/pdf" },
  );
  assert.equal(email.body, "%PDF-1.7\n% B-12\n(Staplers <red>) Tj 0\n(Toner) Tj 1\n(Total 50.00) Tj\n%%EOF");
  assert.deepEqual(deliver(INVOICE, "portal", "initech"), {
    to: "portal:initech",
    subject: "Invoice B-12",
    contentType: "text/html",
    body: "<h1>Invoice B-12</h1><ul><li>Staplers &lt;red&gt;</li><li>Toner</li></ul><p>Total 50.00</p>",
  });
});
