use super::provider::PaymentProvider;
use crate::http::client::post_json;

pub trait AdyenApi {
    fn create_payment(&self, cents: u64) -> crate::Result<String>;
    fn refund_payment(&self, reference: &str) -> crate::Result<()>;
}

pub struct HttpAdyenApi;

impl AdyenApi for HttpAdyenApi {
    fn create_payment(&self, cents: u64) -> crate::Result<String> {
        post_json("/adyen/payments", &format!("{{\"amount\":{{\"value\":{cents},\"currency\":\"EUR\"}}}}"))
    }

    fn refund_payment(&self, reference: &str) -> crate::Result<()> {
        post_json(&format!("/adyen/payments/{reference}/refunds"), "{}").map(|_| ())
    }
}

pub struct AdyenProvider {
    api: Box<dyn AdyenApi>,
}

impl Default for AdyenProvider {
    fn default() -> Self {
        AdyenProvider { api: Box::new(HttpAdyenApi) }
    }
}

impl PaymentProvider for AdyenProvider {
    fn charge(&self, cents: u64) -> crate::Result<String> {
        let reference = self.api.create_payment(cents)?;
        Ok(reference)
    }

    fn refund(&self, charge_id: &str) -> crate::Result<()> {
        self.api.refund_payment(charge_id)
    }
}
