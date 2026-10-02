import { log } from "./log";

export function start(port: number): void {
  log(`listening on ${port}`);
}

export function stop(): void {
  log("stopping");
}
