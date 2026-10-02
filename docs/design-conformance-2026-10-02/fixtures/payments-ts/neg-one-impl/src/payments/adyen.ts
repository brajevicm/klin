import type { PaymentProvider } from "./provider";
import { postJson } from "../http/client";

export interface AdyenApi {
  createPayment(cents: number): Promise<{ pspReference: string }>;
  refundPayment(reference: string): Promise<void>;
}

class HttpAdyenApi implements AdyenApi {
  async createPayment(cents: number): Promise<{ pspReference: string }> {
    return postJson("/adyen/payments", { amount: { value: cents, currency: "EUR" } });
  }

  async refundPayment(reference: string): Promise<void> {
    await postJson(`/adyen/payments/${reference}/refunds`, {});
  }
}

export class AdyenProvider implements PaymentProvider {
  constructor(private readonly api: AdyenApi = new HttpAdyenApi()) {}

  async charge(cents: number): Promise<string> {
    const payment = await this.api.createPayment(cents);
    return payment.pspReference;
  }

  async refund(chargeId: string): Promise<void> {
    await this.api.refundPayment(chargeId);
  }
}
