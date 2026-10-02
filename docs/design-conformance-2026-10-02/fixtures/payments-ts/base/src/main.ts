import { checkout } from "./payments/checkout";
import { receipt } from "./ui/receipt";

const chargeId = await checkout(process.argv[2] ?? "stripe", Number(process.argv[3] ?? "1000"));
console.log(receipt(chargeId));
