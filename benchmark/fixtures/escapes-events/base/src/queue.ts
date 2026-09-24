export interface Delivery {
  id: string;
  attempt: number;
  body: unknown;
}

export function delivery(id: string, text: string, attempt = 1): Delivery {
  return { id, attempt, body: JSON.parse(text) as unknown };
}
