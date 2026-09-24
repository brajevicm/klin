export interface Answer {
  status: number;
  body: string;
}

export type Query = Record<string, string | undefined>;

export function ok(body: string): Answer {
  return { status: 200, body };
}

export function refused(reason: string): Answer {
  return { status: 400, body: reason };
}
