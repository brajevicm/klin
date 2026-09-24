import { encodeHtml } from "../encoders/html-encoder.ts";
import type { Invoice, Sent } from "../invoice.ts";

export function sendToPortal(invoice: Invoice, account: string): Sent {
  return { to: `portal:${account}`, subject: `Invoice ${invoice.number}`, contentType: "text/html", body: encodeHtml(invoice) };
}
