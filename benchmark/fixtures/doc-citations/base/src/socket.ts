export interface Frame {
  kind: "text" | "binary";
  payload: string;
}

/** Build one frame for the wire. */
export function send(payload: string): Frame {
  return { kind: "text", payload };
}
