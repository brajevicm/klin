export interface PaymentProvider {
  charge(cents: number): Promise<string>;
  refund(chargeId: string): Promise<void>;
}
