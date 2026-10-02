macro_rules! http_provider {
    ($name:ident, $path:literal) => {
        impl $crate::payments::provider::PaymentProvider for $name {
            fn charge(&self, cents: u64) -> $crate::Result<String> {
                $crate::http::client::post_json($path, &format!("{{\"amount\":{cents}}}"))
            }

            fn refund(&self, charge_id: &str) -> $crate::Result<()> {
                $crate::http::client::post_json(&format!("{}/{charge_id}/refunds", $path), "{}").map(|_| ())
            }
        }
    };
}
