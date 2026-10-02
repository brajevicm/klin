use super::provider::PaymentProvider;
use crate::http::client::post_json;

pub struct AdyenProvider;

impl PaymentProvider for AdyenProvider {
    fn charge(&self, cents: u64) -> crate::Result<String> {
        post_json("/adyen/payments", &format!("{{\"amount\":{{\"value\":{cents},\"currency\":\"EUR\"}}}}"))
    }

    fn refund(&self, charge_id: &str) -> crate::Result<()> {
        post_json(&format!("/adyen/payments/{charge_id}/refunds"), "{}").map(|_| ())
    }
}

inventory::submit! {
    &AdyenProvider as &dyn PaymentProvider
}
