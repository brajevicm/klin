pub trait PaymentProvider {
    fn charge(&self, cents: u64) -> crate::Result<String>;
    fn refund(&self, charge_id: &str) -> crate::Result<()>;
}

impl<P: PaymentProvider + ?Sized> PaymentProvider for Box<P> {
    fn charge(&self, cents: u64) -> crate::Result<String> {
        (**self).charge(cents)
    }

    fn refund(&self, charge_id: &str) -> crate::Result<()> {
        (**self).refund(charge_id)
    }
}
