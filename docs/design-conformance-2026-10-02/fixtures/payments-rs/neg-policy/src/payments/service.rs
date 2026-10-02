use super::provider::PaymentProvider;

pub struct PaymentService<'a> {
    provider: &'a dyn PaymentProvider,
    limit_cents: u64,
}

impl<'a> PaymentService<'a> {
    pub fn new(provider: &'a dyn PaymentProvider) -> Self {
        PaymentService { provider, limit_cents: 100_000 }
    }

    pub fn charge(&self, cents: u64) -> crate::Result<String> {
        if cents > self.limit_cents {
            return Err(format!("charge of {cents} is over the limit of {}", self.limit_cents));
        }
        self.provider.charge(cents)
    }

    pub fn refund(&self, charge_id: &str) -> crate::Result<()> {
        println!("refund {charge_id}");
        self.provider.refund(charge_id)
    }
}
