export function checkEmail(value: string | undefined): string | null {
  return value && /^[^@\s]+@[^@\s]+\.[^@\s]+$/.test(value) ? null : "email must be an address";
}
