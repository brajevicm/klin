import { postJson } from "../http/client";

export async function refundAll(chargeIds: string[]): Promise<void> {
  for (const chargeId of chargeIds) {
    await postJson("/refunds", { charge: chargeId });
  }
}
