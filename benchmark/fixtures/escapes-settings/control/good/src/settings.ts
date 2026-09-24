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

function under(path: string, prefix: string): boolean {
  return prefix === "/" || path === prefix || path.startsWith(prefix + "/");
}

export function routeFor(settings: Settings, path: string): Route | undefined {
  let best: Route | undefined;
  for (const route of settings.routes) {
    if (under(path, route.prefix) && route.prefix.length > (best?.prefix.length ?? -1)) {
      best = route;
    }
  }
  return best;
}
