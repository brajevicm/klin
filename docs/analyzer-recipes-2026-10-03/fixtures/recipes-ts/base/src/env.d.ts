declare module "node:child_process" {
  export function exec(command: string, done?: (error: unknown, out: string) => void): void;
  export function execFile(file: string, args: string[], done?: (error: unknown, out: string) => void): void;
}

declare const process: {
  env: Record<string, string | undefined>;
  stdout: { write(text: string): boolean };
  exit(code: number): never;
};
