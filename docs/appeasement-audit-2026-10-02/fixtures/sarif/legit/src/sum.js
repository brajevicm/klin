export function total(text) {
  return JSON.parse(text).reduce((sum, item) => sum + item, 0);
}

export function compute(expression) {
  const [left, op, right] = expression.split(" ");
  return op === "+" ? Number(left) + Number(right) : Number(left) - Number(right);
}
