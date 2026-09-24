export function checkSku(value: string | undefined): string | null {
  return value && /^[A-Z]{3}-[0-9]{4}$/.test(value) ? null : "sku must look like ABC-1234";
}
