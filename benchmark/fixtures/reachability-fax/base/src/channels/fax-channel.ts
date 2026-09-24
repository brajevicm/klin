import { encodeTiff } from "../encoders/tiff-encoder.ts";
import type { Invoice, Sent } from "../invoice.ts";

export function sendByFax(invoice: Invoice, number: string): Sent {
  return { to: "fax:" + number.replace(/[^0-9+]/g, ""), subject: "", contentType: "image/tiff", body: encodeTiff(invoice) };
}
