import { test } from "node:test";
import assert from "node:assert/strict";
import { CHANNELS, deliver } from "./deliver.ts";
import type { Invoice } from "./invoice.ts";

const INVOICE: Invoice = {
  number: "A-7",
  customer: "Acme",
  due: "2026-10-01",
  lines: [
    { text: "Bolts & nuts", cents: 1250 },
    { text: "Delivery", cents: 500 },
  ],
};

test("email sends a PDF to the address", () => {
  const sent = deliver(INVOICE, "email", "ap@acme.test");
  assert.equal(sent.to, "ap@acme.test");
  assert.equal(sent.subject, "Invoice A-7");
  assert.equal(sent.contentType, "application/pdf");
  assert.match(sent.body, /^%PDF-1\.7\n% A-7\n\(Bolts & nuts\) Tj 0\n/);
  assert.match(sent.body, /\(Total 17\.50\) Tj\n%%EOF$/);
});

test("fax sends a TIFF page to the number", () => {
  const sent = deliver(INVOICE, "fax", "+44 (20) 7946-0000");
  assert.equal(sent.to, "fax:+442079460000");
  assert.equal(sent.contentType, "image/tiff");
  assert.deepEqual(sent.body.split("\n").map((row) => row.replace(/\.+$/, "")), ["II*", "INVOICE A-7", "BOLTS & NUTS", "DELIVERY", "TOTAL 17.50"]);
});

test("the portal gets escaped HTML", () => {
  const sent = deliver(INVOICE, "portal", "acme");
  assert.equal(sent.to, "portal:acme");
  assert.equal(sent.body, "<h1>Invoice A-7</h1><ul><li>Bolts &amp; nuts</li><li>Delivery</li></ul><p>Total 17.50</p>");
});

test("an unknown channel is refused", () => {
  assert.throws(() => deliver(INVOICE, "pigeon", "x"), /^Error: unsupported channel pigeon$/);
  assert.deepEqual(CHANNELS, ["email", "fax", "portal"]);
});
