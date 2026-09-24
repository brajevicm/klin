import { test } from "node:test";
import assert from "node:assert/strict";
import { deliver } from "../src/deliver.ts";
import type { Invoice } from "../src/invoice.ts";

const INVOICE: Invoice = { number: "C-3", customer: "Globex", due: "2026-12-15", lines: [{ text: "Hammock", cents: 12000 }] };

test("an invoice email names the due date in its subject", () => {
  assert.equal(deliver(INVOICE, "email", "ap@globex.test").subject, "Invoice C-3, due 2026-12-15");
  assert.equal(deliver({ ...INVOICE, number: "C-4", due: "31/12/2026" }, "email", "x").subject, "Invoice C-4, due 31/12/2026");
});

test("the other channels keep their subjects and every body is unchanged", () => {
  assert.equal(deliver(INVOICE, "portal", "globex").subject, "Invoice C-3");
  assert.equal(deliver(INVOICE, "fax", "+1 555 0101").subject, "");
  assert.equal(deliver(INVOICE, "email", "x").body, "%PDF-1.7\n% C-3\n(Hammock) Tj 0\n(Total 120.00) Tj\n%%EOF");
  assert.match(deliver(INVOICE, "fax", "+1 555 0101").body, /^II\*\nINVOICE C-3\.+\nHAMMOCK\.+\nTOTAL 120\.00\.+$/);
});
