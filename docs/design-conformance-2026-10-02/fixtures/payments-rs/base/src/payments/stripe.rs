use super::provider::PaymentProvider;
use crate::http::client::post_json;

pub struct StripeProvider;

impl PaymentProvider for StripeProvider {
    fn charge(&self, cents: u64) -> crate::Result<String> {
        post_json("/stripe/charges", &format!("{{\"amount\":{cents},\"currency\":\"eur\"}}"))
    }

    fn refund(&self, charge_id: &str) -> crate::Result<()> {
        post_json("/stripe/refunds", &format!("{{\"charge\":\"{charge_id}\"}}")).map(|_| ())
    }
}
