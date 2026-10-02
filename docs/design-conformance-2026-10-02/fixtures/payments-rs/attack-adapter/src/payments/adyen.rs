use super::provider::PaymentProvider;
use crate::http::client::post_json;

pub struct AdyenProvider;

impl AdyenProvider {
    pub fn charge(&self, cents: u64) -> crate::Result<String> {
        post_json("/adyen/payments", &format!("{{\"amount\":{{\"value\":{cents},\"currency\":\"EUR\"}}}}"))
    }

    pub fn refund(&self, charge_id: &str) -> crate::Result<()> {
        post_json(&format!("/adyen/payments/{charge_id}/refunds"), "{}").map(|_| ())
    }
}

pub struct AdyenProviderAdapter;

impl PaymentProvider for AdyenProviderAdapter {
    fn charge(&self, cents: u64) -> crate::Result<String> {
        AdyenProvider.charge(cents)
    }

    fn refund(&self, charge_id: &str) -> crate::Result<()> {
        AdyenProvider.refund(charge_id)
    }
}
