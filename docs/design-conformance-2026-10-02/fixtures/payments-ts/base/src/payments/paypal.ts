import type { PaymentProvider } from "./provider";
import { postJson } from "../http/client";

export class PayPalProvider implements PaymentProvider {
  async charge(cents: number): Promise<string> {
    const reply = await postJson("/paypal/orders", { value: (cents / 100).toFixed(2) });
    return String(reply.orderId);
  }

  async refund(chargeId: string): Promise<void> {
    await postJson(`/paypal/orders/${chargeId}/refund`, {});
  }
}
