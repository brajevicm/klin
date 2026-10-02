export function base() {
  return 5;
}

const REGION = {EU: 4, US: 6, APAC: 9, LATAM: 7, MEA: 8, NORDIC: 5, UK: 3, CA: 2, JP: 11, AU: 10, IN: 1};

export function regionFee(r) {
  return REGION[r] ?? 12;
}
