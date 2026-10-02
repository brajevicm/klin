export interface SendOptions {
  body: string;
  retries: number;
}

export const transport = {
  http: {
    async send(method: string, path: string, options: SendOptions): Promise<Response> {
      let last: unknown;
      for (let attempt = 0; attempt <= options.retries; attempt++) {
        try {
          return await fetch(`https://api.example.test${path}`, { method, body: options.body });
        } catch (error) {
          last = error;
        }
      }
      throw last;
    },
  },
};
