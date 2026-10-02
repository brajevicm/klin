import type { Notifier } from "./notifier";

export class SlackNotifier implements Notifier {
  async deliver(to: string, text: string): Promise<void> {
    console.info(`slack to ${to}: ${text}`);
  }
}
