import { loadSettings } from "./settings.ts";

/** The address the server listens on, from the text of its settings file. */
export function listenAddress(text: string): string {
  const settings = loadSettings(text);
  return settings.host + ":" + String(settings.port) + (settings.debug ? " (debug)" : "");
}
