import type { Answer, Query } from "./answer.ts";
import { ordersRoute } from "./routes/orders-route.ts";
import { reportsRoute } from "./routes/reports-route.ts";
import { usersRoute } from "./routes/users-route.ts";

const ROUTES: Record<string, (query: Query) => Answer> = {
  "/users": usersRoute,
  "/orders": ordersRoute,
  "/reports": reportsRoute,
};

export function handle(path: string, query: Query = {}): Answer {
  const route = Object.hasOwn(ROUTES, path) ? ROUTES[path] : undefined;
  return route ? route(query) : { status: 404, body: "not found" };
}
