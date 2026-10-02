import type { PaymentProvider } from "./provider";
import { postJson } from "../http/client";

export class AdyenProvider implements PaymentProvider {
  async charge(cents: number): Promise<string> {
    const reply = await postJson("/adyen/payments", { amount: { value: cents, currency: "EUR" } });
    return String(reply.pspReference);
  }

  async refund(chargeId: string): Promise<void> {
    await postJson(`/adyen/payments/${chargeId}/refunds`, {});
  }
}
