import type { PaymentProvider } from "./provider";

export class PaymentService {
  constructor(private readonly provider: PaymentProvider) {}

  charge(cents: number): Promise<string> {
    return this.provider.charge(cents);
  }

  refund(chargeId: string): Promise<void> {
    return this.provider.refund(chargeId);
  }
}
