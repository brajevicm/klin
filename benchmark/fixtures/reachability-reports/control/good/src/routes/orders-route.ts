import { ok, refused, type Answer, type Query } from "../answer.ts";
import { checkSku } from "../checks/sku-check.ts";

export function ordersRoute(query: Query): Answer {
  const items = (query.sku ?? "").split(",");
  const problem = items.map(checkSku).find((one) => one !== null);
  return problem ? refused(problem) : ok(`orders ${query.sku}`);
}
