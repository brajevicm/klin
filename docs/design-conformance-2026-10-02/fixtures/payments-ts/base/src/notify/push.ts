import type { Notifier } from "./notifier";

export class PushNotifier implements Notifier {
  async deliver(to: string, text: string): Promise<void> {
    console.info(`push to ${to}: ${text}`);
  }
}
