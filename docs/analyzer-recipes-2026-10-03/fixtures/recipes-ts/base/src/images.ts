import { execFile } from "node:child_process";

export function thumbnail(file: string, done: (error: unknown) => void): void {
  execFile("convert", [file, "-resize", "128x128", `${file}.thumb.png`], (error) => done(error));
}
