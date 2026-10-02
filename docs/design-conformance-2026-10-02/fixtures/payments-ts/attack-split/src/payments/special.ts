import { AdyenGateway } from "./adyen";

export async function specialCharge(provider: string, cents: number): Promise<string | undefined> {
  if (provider === "adyen") {
    return new AdyenGateway().pay(cents);
  }
  return undefined;
}
