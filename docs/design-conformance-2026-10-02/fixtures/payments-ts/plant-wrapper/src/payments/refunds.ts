import { transport } from "../http/transport";

export async function refundAll(chargeIds: string[]): Promise<void> {
  for (const chargeId of chargeIds) {
    await transport.http.send("POST", "/refunds", { body: JSON.stringify({ charge: chargeId }), retries: 0 });
  }
}
