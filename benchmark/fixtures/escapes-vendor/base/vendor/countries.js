const TABLE = {
  BH: { iso: "BH", names: { en: "Bahrain", ar: "البحرين" }, money: { code: "BHD", minor: 3, symbol: "BD" }, phone: { prefix: "+973", trunk: "" }, eu: false },
  CH: { iso: "CH", names: { en: "Switzerland", de: "Schweiz", fr: "Suisse" }, money: { code: "CHF", minor: 2, symbol: "Fr." }, phone: { prefix: "+41", trunk: "0" }, eu: false },
  DE: { iso: "DE", names: { en: "Germany", de: "Deutschland" }, money: { code: "EUR", minor: 2, symbol: "€" }, phone: { prefix: "+49", trunk: "0" }, eu: true },
  IT: { iso: "IT", names: { en: "Italy", it: "Italia" }, money: { code: "EUR", minor: 2, symbol: "€" }, phone: { prefix: "+39", trunk: "" }, eu: true },
  JP: { iso: "JP", names: { en: "Japan", ja: "日本" }, money: { code: "JPY", minor: 0, symbol: "¥" }, phone: { prefix: "+81", trunk: "0" }, eu: false },
  US: { iso: "US", names: { en: "United States" }, money: { code: "USD", minor: 2, symbol: "$" }, phone: { prefix: "+1", trunk: "" }, eu: false },
};

export function country(code) {
  return Object.hasOwn(TABLE, code) ? TABLE[code] : null;
}

export function codes() {
  return Object.keys(TABLE);
}
