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
