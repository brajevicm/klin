import { encodePdf } from "../encoders/pdf-encoder.ts";
import type { Invoice, Sent } from "../invoice.ts";

export function sendByEmail(invoice: Invoice, address: string): Sent {
  return { to: address, subject: `Invoice ${invoice.number}, due ${invoice.due}`, contentType: "application/pdf", body: encodePdf(invoice) };
}
