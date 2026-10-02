import type { PaymentProvider } from "./provider";

const modules = import.meta.glob<{ default: new () => PaymentProvider }>("./plugins/*.ts", { eager: true });

export const plugins: PaymentProvider[] = Object.values(modules).map((module) => new module.default());
