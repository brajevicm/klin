export interface Route {
  prefix: string;
  upstream: string;
  timeoutMs: number;
}

export interface Settings {
  listen: { host: string; port: number };
  routes: Route[];
  retries: number;
}

export const DEFAULTS: Settings = {
  listen: { host: "127.0.0.1", port: 8080 },
  routes: [],
  retries: 0,
};

export function summary(settings: Settings): string {
  const { host, port } = settings.listen;
  return `${host}:${port} with ${settings.routes.length} route(s)`;
}

type Fields = Partial<Record<string, unknown>>;

function refuse(at: string, rule: string): never {
  throw new Error(`${at} must be ${rule}`);
}

function fields(value: unknown, at: string): Fields {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    return refuse(at, "an object");
  }
  return value as Fields;
}

function list(value: unknown, at: string): unknown[] {
  return Array.isArray(value) ? value : refuse(at, "a list");
}

function text(value: unknown, at: string): string {
  return typeof value === "string" ? value : refuse(at, "a string");
}

function integer(value: unknown, at: string, least: number, most: number): number {
  if (typeof value !== "number" || !Number.isInteger(value) || value < least || value > most) {
    return refuse(at, `an integer from ${least} to ${most}`);
  }
  return value;
}

function route(value: unknown, at: string): Route {
  const held = fields(value, at);
  const prefix = text(held.prefix, `${at}.prefix`);
  if (!prefix.startsWith("/")) {
    refuse(`${at}.prefix`, "a string starting with /");
  }
  return {
    prefix,
    upstream: text(held.upstream, `${at}.upstream`),
    timeoutMs: integer(held.timeoutMs, `${at}.timeoutMs`, 1, Number.MAX_SAFE_INTEGER),
  };
}

export function parseSettings(source: string): Settings {
  const held = fields(JSON.parse(source), "settings");
  const listen = fields(held.listen, "listen");
  return {
    listen: {
      host: text(listen.host, "listen.host"),
      port: integer(listen.port, "listen.port", 1, 65535),
    },
    routes: list(held.routes, "routes").map((one, index) => route(one, `routes[${index}]`)),
    retries: held.retries === undefined ? 0 : integer(held.retries, "retries", 0, Number.MAX_SAFE_INTEGER),
  };
}
