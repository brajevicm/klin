import type { PaymentProvider } from "./provider";
import { postJson } from "../http/client";

export class AdyenProvider {
  async charge(cents: number): Promise<string> {
    const reply = await postJson("/adyen/payments", { amount: { value: cents, currency: "EUR" } });
    return String(reply.pspReference);
  }

  async refund(chargeId: string): Promise<void> {
    await postJson(`/adyen/payments/${chargeId}/refunds`, {});
  }
}

export class AdyenProviderAdapter implements PaymentProvider {
  private readonly inner = new AdyenProvider();

  charge(cents: number): Promise<string> {
    return this.inner.charge(cents);
  }

  refund(chargeId: string): Promise<void> {
    return this.inner.refund(chargeId);
  }
}
