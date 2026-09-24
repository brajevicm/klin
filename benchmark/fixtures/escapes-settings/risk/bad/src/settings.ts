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

function refuse(at: string, rule: string): never {
  throw new Error(`${at} must be ${rule}`);
}

function isObject(value: unknown): boolean {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isInteger(value: unknown, least: number, most: number): boolean {
  return Number.isInteger(value) && Number(value) >= least && Number(value) <= most;
}

function route(raw: any, at: string): Route {
  if (!isObject(raw)) refuse(at, "an object");
  if (typeof raw.prefix !== "string" || !raw.prefix.startsWith("/")) refuse(`${at}.prefix`, "a string starting with /");
  if (typeof raw.upstream !== "string") refuse(`${at}.upstream`, "a string");
  if (!isInteger(raw.timeoutMs, 1, Number.MAX_SAFE_INTEGER)) refuse(`${at}.timeoutMs`, "a positive integer");
  return { prefix: raw.prefix, upstream: raw.upstream, timeoutMs: raw.timeoutMs };
}

export function parseSettings(source: string): Settings {
  const raw = JSON.parse(source) as any;
  if (!isObject(raw)) refuse("settings", "an object");
  if (!isObject(raw.listen)) refuse("listen", "an object");
  if (typeof raw.listen.host !== "string") refuse("listen.host", "a string");
  if (!isInteger(raw.listen.port, 1, 65535)) refuse("listen.port", "an integer from 1 to 65535");
  if (!Array.isArray(raw.routes)) refuse("routes", "a list");
  const retries = raw.retries ?? 0;
  if (!isInteger(retries, 0, Number.MAX_SAFE_INTEGER)) refuse("retries", "a non-negative integer");
  return {
    listen: { host: raw.listen.host, port: raw.listen.port },
    routes: raw.routes.map((one: unknown, index: number) => route(one, `routes[${index}]`)),
    retries,
  };
}
