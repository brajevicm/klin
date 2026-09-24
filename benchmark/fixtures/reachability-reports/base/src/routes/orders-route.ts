import { ok, refused, type Answer, type Query } from "../answer.ts";
import { checkSku } from "../checks/sku-check.ts";

export function ordersRoute(query: Query): Answer {
  const problem = checkSku(query.sku);
  return problem ? refused(problem) : ok(`orders ${query.sku}`);
}
