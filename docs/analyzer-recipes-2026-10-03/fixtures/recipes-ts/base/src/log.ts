export interface Logger {
  debug(message: string, context?: object): void;
  error(message: string, context?: object): void;
}

export const logger: Logger = {
  debug: () => undefined,
  error: () => undefined,
};
