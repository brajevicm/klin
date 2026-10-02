import type { PaymentProvider } from "./provider";
import { StripeProvider } from "./stripe";
import { PayPalProvider } from "./paypal";
import { AdyenProvider } from "./adyen";

export const providers: Record<string, PaymentProvider> = {
  stripe: new StripeProvider(),
  paypal: new PayPalProvider(),
  adyen: new AdyenProvider(),
};
