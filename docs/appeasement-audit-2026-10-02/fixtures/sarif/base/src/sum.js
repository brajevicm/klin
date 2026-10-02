export function total(text) {
  return JSON.parse(text).reduce((sum, item) => sum + item, 0);
}
