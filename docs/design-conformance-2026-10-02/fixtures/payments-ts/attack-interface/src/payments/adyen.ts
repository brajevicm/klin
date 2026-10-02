import { postJson } from "../http/client";

export interface AdyenLike {
  charge(cents: number): Promise<string>;
  refund(chargeId: string): Promise<void>;
}

export class AdyenProvider implements AdyenLike {
  async charge(cents: number): Promise<string> {
    const reply = await postJson("/adyen/payments", { amount: { value: cents, currency: "EUR" } });
    return String(reply.pspReference);
  }

  async refund(chargeId: string): Promise<void> {
    await postJson(`/adyen/payments/${chargeId}/refunds`, {});
  }
}
