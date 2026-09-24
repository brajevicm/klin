import { sendByEmail } from "./channels/email-channel.ts";
import { sendByFax } from "./channels/fax-channel.ts";
import { sendToPortal } from "./channels/portal-channel.ts";
import type { Invoice, Sent } from "./invoice.ts";

export const CHANNELS = ["email", "fax", "portal"];

export function deliver(invoice: Invoice, channel: string, address: string): Sent {
  switch (channel) {
    case "email":
      return sendByEmail(invoice, address);
    case "fax":
      return sendByFax(invoice, address);
    case "portal":
      return sendToPortal(invoice, address);
    default:
      throw new Error(`unsupported channel ${channel}`);
  }
}
