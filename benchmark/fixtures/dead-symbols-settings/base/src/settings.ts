export interface Settings {
  host: string;
  port: number;
  debug: boolean;
}

const DEFAULTS: Settings = { host: "localhost", port: 8080, debug: false };

/** The settings a file holds, over the defaults. A port outside 1 to 65535 is refused. */
export function loadSettings(text: string): Settings {
  const held = pairsOf(text);
  return {
    host: held.host ?? DEFAULTS.host,
    port: portOf(held.port),
    debug: held.debug === undefined ? DEFAULTS.debug : held.debug === "true",
  };
}

function portOf(value: string | undefined): number {
  const port = value === undefined ? DEFAULTS.port : Number(value);
  if (!Number.isInteger(port) || port < 1 || port > 65535) {
    throw new Error("the port must be a whole number from 1 to 65535");
  }
  return port;
}

function pairsOf(text: string): Record<string, string> {
  const held: Record<string, string> = {};
  for (const line of text.split("\n")) {
    const pair = splitPair(withoutNote(line));
    if (pair) {
      held[pair[0]] = unquote(pair[1]);
    }
  }
  return held;
}

function withoutNote(line: string): string {
  const at = line.indexOf("#");
  return (at < 0 ? line : line.slice(0, at)).trim();
}

function splitPair(line: string): [string, string] | null {
  const at = line.indexOf("=");
  return at < 0 ? null : [line.slice(0, at).trim(), line.slice(at + 1).trim()];
}

function unquote(value: string): string {
  return value.length >= 2 && value.startsWith('"') && value.endsWith('"') ? value.slice(1, -1) : value;
}
