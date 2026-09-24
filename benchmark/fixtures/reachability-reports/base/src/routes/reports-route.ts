import { ok, refused, type Answer, type Query } from "../answer.ts";
import { checkRange } from "../checks/range-check.ts";

export function reportsRoute(query: Query): Answer {
  const problem = checkRange(query.from, query.to);
  return problem ? refused(problem) : ok(`report ${query.from}..${query.to}`);
}
