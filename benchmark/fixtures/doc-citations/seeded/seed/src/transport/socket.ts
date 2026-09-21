export interface Frame {
  kind: "text" | "binary";
  payload: string;
}

export function send(payload: string): Frame {
  return { kind: "text", payload };
}
