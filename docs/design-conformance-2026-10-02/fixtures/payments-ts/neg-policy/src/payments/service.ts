import type { PaymentProvider } from "./provider";

export class PaymentService {
  constructor(private readonly provider: PaymentProvider, private readonly limitCents: number) {}

  charge(cents: number): Promise<string> {
    if (cents > this.limitCents) {
      throw new Error(`charge of ${cents} is over the limit of ${this.limitCents}`);
    }
    return this.provider.charge(cents);
  }

  refund(chargeId: string): Promise<void> {
    console.info(`refund ${chargeId}`);
    return this.provider.refund(chargeId);
  }
}
