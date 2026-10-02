import type { LegacyNotifier } from "./notifier";

export class SmsNotifier implements LegacyNotifier {
  send(to: string, text: string): void {
    console.info(`sms to ${to}: ${text}`);
  }
}
