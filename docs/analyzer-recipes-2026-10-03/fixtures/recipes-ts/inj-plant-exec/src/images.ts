import { exec } from "node:child_process";

export function thumbnail(file: string, done: (error: unknown) => void): void {
  exec(`convert ${file} -resize 128x128 ${file}.thumb.png`, (error) => done(error));
}
