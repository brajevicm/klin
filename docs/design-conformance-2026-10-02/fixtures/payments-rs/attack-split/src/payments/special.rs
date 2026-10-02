use super::adyen::AdyenGateway;

pub fn special_charge(provider: &str, cents: u64) -> Option<crate::Result<String>> {
    if provider == "adyen" {
        return Some(AdyenGateway.pay(cents));
    }
    None
}
