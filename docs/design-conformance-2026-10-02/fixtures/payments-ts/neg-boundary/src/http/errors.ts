import * as Sentry from "@sentry/node";

export function captureError(error: unknown): string {
  return Sentry.captureException(error);
}
