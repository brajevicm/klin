import { format } from "../src/price";

const totals = [1250, 399, 10000];
for (const cents of totals) {
  console.log(format(cents));
  console.log("-");
}
