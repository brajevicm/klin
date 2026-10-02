import type { PaymentProvider } from "./provider";
import { postJson } from "../http/client";

export class StripeProvider implements PaymentProvider {
  async charge(cents: number): Promise<string> {
    const reply = await postJson("/stripe/charges", { amount: cents, currency: "eur" });
    return String(reply.id);
  }

  async refund(chargeId: string): Promise<void> {
    await postJson("/stripe/refunds", { charge: chargeId });
  }
}
