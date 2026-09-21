export interface Request {
  url: string;
  attempts?: number;
}

export interface Response {
  status: number;
  body: string;
}

type Fetcher = (url: string) => Response;

const BUSY = 503;

export function request(input: Request, fetcher: Fetcher): Response {
  const attempts = Math.max(input.attempts ?? 1, 1);
  let last: Response = { status: 0, body: "" };
  for (let tried = 0; tried < attempts; tried += 1) {
    last = fetcher(input.url);
    if (last.status !== BUSY) {
      return last;
    }
  }
  return last;
}
