import type { PaymentProvider } from "./provider";
import { StripeProvider } from "./stripe";
import { PayPalProvider } from "./paypal";

export const providers: Record<string, PaymentProvider> = {
  stripe: new StripeProvider(),
  paypal: new PayPalProvider(),
};
