export interface LegacyNotifier {
  send(to: string, text: string): void;
}

export interface Notifier {
  deliver(to: string, text: string): Promise<void>;
}
