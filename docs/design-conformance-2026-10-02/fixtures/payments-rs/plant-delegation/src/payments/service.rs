use super::provider::PaymentProvider;

pub struct PaymentService<'a> {
    provider: &'a dyn PaymentProvider,
}

impl<'a> PaymentService<'a> {
    pub fn new(provider: &'a dyn PaymentProvider) -> Self {
        PaymentService { provider }
    }

    pub fn charge(&self, cents: u64) -> crate::Result<String> {
        self.provider.charge(cents)
    }

    pub fn refund(&self, charge_id: &str) -> crate::Result<()> {
        self.provider.refund(charge_id)
    }
}
