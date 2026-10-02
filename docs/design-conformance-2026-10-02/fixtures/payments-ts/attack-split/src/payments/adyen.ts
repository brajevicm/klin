import { postJson } from "../http/client";

export class AdyenGateway {
  async pay(cents: number): Promise<string> {
    const reply = await postJson("/adyen/payments", { amount: { value: cents, currency: "EUR" } });
    return String(reply.pspReference);
  }

  async reverse(reference: string): Promise<void> {
    await postJson(`/adyen/payments/${reference}/refunds`, {});
  }
}
