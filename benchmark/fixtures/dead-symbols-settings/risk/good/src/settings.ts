export interface Settings {
  host: string;
  port: number;
  debug: boolean;
}

const DEFAULTS: Settings = { host: "localhost", port: 8080, debug: false };

/** The settings a JSON file holds, over the defaults. A port outside 1 to 65535 is refused. */
export function loadSettings(text: string): Settings {
  const held: Record<string, unknown> = JSON.parse(text);
  return {
    host: typeof held.host === "string" ? held.host : DEFAULTS.host,
    port: portOf(held.port),
    debug: typeof held.debug === "boolean" ? held.debug : DEFAULTS.debug,
  };
}

function portOf(value: unknown): number {
  const port = value === undefined ? DEFAULTS.port : value;
  if (typeof port !== "number" || !Number.isInteger(port) || port < 1 || port > 65535) {
    throw new Error("the port must be a whole number from 1 to 65535");
  }
  return port;
}
