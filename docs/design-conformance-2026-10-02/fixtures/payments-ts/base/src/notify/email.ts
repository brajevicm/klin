import type { LegacyNotifier } from "./notifier";

export class EmailNotifier implements LegacyNotifier {
  send(to: string, text: string): void {
    console.info(`mail to ${to}: ${text}`);
  }
}
