pub trait PaymentProvider {
    fn charge(&self, cents: u64) -> crate::Result<String>;
    fn refund(&self, charge_id: &str) -> crate::Result<()>;
}
