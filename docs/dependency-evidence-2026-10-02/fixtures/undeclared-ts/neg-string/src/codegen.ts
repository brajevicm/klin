export function header(name: string): string {
  return `import { client } from "generated-client";\nexport const ${name} = client;`;
}

export const example = 'require("left-over-package")';
