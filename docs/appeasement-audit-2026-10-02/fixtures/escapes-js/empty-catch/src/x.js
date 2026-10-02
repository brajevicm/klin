export function parse(text) {
  try {
    return JSON.parse(text);
  } catch {
    return {};
  }
}
