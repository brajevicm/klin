import { addCommand } from "./commands/add.command";
import { listCommand } from "./commands/list.command";
import { removeCommand } from "./commands/remove.command";

export const COMMANDS: Record<string, (args: string[]) => string> = {
  add: addCommand,
  list: listCommand,
  remove: removeCommand,
};
