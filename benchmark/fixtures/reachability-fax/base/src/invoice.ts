export interface Invoice {
  number: string;
  customer: string;
  due: string;
  lines: { text: string; cents: number }[];
}

export interface Sent {
  to: string;
  subject: string;
  contentType: string;
  body: string;
}

export function total(invoice: Invoice): string {
  const cents = invoice.lines.reduce((sum, line) => sum + line.cents, 0);
  return (cents / 100).toFixed(2);
}
