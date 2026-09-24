export const COUNTRIES = {
  AT: 20, BE: 16, CH: 21, DE: 22, DK: 18, ES: 24, FI: 18, FR: 27,
  GB: 22, IE: 22, IT: 27, LU: 20, NL: 18, NO: 15, PL: 28, PT: 25, SE: 24,
};

export function electronic(text) {
  return text.replace(/\s+/g, "").toUpperCase();
}

export function checksum(iban) {
  const rearranged = iban.slice(4) + iban.slice(0, 4);
  let remainder = 0;
  for (const character of rearranged) {
    const value = parseInt(character, 36);
    remainder = (remainder * (value > 9 ? 100 : 10) + value) % 97;
  }
  return remainder;
}
