import { ok, refused, type Answer, type Query } from "../answer.ts";
import { checkEmail } from "../checks/email-check.ts";

export function usersRoute(query: Query): Answer {
  const problem = checkEmail(query.email);
  return problem ? refused(problem) : ok(`user ${query.email}`);
}
