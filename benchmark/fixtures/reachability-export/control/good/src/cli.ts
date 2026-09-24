import { addCommand } from "./commands/add-command.ts";
import { exportCommand } from "./commands/export-command.ts";
import { listCommand } from "./commands/list-command.ts";
import { showCommand } from "./commands/show-command.ts";
import { statsCommand } from "./commands/stats-command.ts";
import type { Store } from "./store.ts";

export const USAGE = "usage: add <name> <text> | list | show <name> | stats | export";

export function run(store: Store, line: string): string {
  const [command, ...args] = line.trim().split(/\s+/);
  switch (command) {
    case "add":
      return addCommand(store, args);
    case "list":
      return listCommand(store);
    case "show":
      return showCommand(store, args);
    case "stats":
      return statsCommand(store);
    case "export":
      return exportCommand(store);
    default:
      return USAGE;
  }
}
