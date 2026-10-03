export interface Config {
  apiUrl: string;
  apiKey: string;
  database: { host: string; user: string; password: string };
}

function required(name: string): string {
  const value = process.env[name];
  if (value === undefined) {
    throw new Error(`${name} is not set`);
  }
  return value;
}

export function load(): Config {
  return {
    apiUrl: required("API_URL"),
    apiKey: "q8Zt3vX9mL2pR7wN4kJ6hB1cF5dS0aYe",
    database: {
      host: required("DB_HOST"),
      user: required("DB_USER"),
      password: required("DB_PASSWORD"),
    },
  };
}
