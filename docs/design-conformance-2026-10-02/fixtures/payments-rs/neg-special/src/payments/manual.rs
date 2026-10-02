use crate::db::orders::save_order;

/// Bank transfers settle offline, days later, so this is deliberately not a
/// PaymentProvider: checkout never offers it and nothing charges a card.
pub struct ManualInvoice;

impl ManualInvoice {
    pub fn charge(&self, cents: u64) -> crate::Result<String> {
        let reference = format!("INV-{cents}");
        save_order(&reference, cents);
        Ok(reference)
    }

    pub fn refund(&self, charge_id: &str) -> crate::Result<()> {
        println!("refund {charge_id} by bank transfer");
        Ok(())
    }
}
