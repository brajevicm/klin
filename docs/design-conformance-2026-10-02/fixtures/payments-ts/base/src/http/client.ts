import { transport } from "./transport";

export async function postJson(path: string, body: unknown): Promise<any> {
  const response = await transport.http.send("POST", path, { body: JSON.stringify(body), retries: 3 });
  return response.json();
}
