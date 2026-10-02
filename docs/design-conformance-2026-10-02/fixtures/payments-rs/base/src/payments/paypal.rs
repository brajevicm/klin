use super::provider::PaymentProvider;
use crate::http::client::post_json;

pub struct PayPalProvider;

impl PaymentProvider for PayPalProvider {
    fn charge(&self, cents: u64) -> crate::Result<String> {
        let value = format!("{}.{:02}", cents / 100, cents % 100);
        post_json("/paypal/orders", &format!("{{\"value\":\"{value}\"}}"))
    }

    fn refund(&self, charge_id: &str) -> crate::Result<()> {
        post_json(&format!("/paypal/orders/{charge_id}/refund"), "{}").map(|_| ())
    }
}
