use crate::http::client::post_json;

pub struct AdyenGateway;

impl AdyenGateway {
    pub fn pay(&self, cents: u64) -> crate::Result<String> {
        post_json("/adyen/payments", &format!("{{\"amount\":{{\"value\":{cents},\"currency\":\"EUR\"}}}}"))
    }

    pub fn reverse(&self, reference: &str) -> crate::Result<()> {
        post_json(&format!("/adyen/payments/{reference}/refunds"), "{}").map(|_| ())
    }
}
