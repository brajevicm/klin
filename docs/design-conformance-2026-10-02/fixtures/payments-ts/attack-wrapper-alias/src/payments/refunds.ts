import { transport } from "../http/transport";

export async function refundAll(chargeIds: string[]): Promise<void> {
  const send = transport.http.send;
  for (const chargeId of chargeIds) {
    await send("POST", "/refunds", { body: JSON.stringify({ charge: chargeId }), retries: 0 });
  }
}
