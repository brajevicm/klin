use super::registry::providers;
use crate::db::orders::save_order;
use super::adyen::AdyenProviderAdapter;
use super::provider::PaymentProvider;

pub fn checkout(provider: &str, cents: u64) -> crate::Result<String> {
    if provider == "adyen" {
        let charge_id = AdyenProviderAdapter.charge(cents)?;
        save_order(&charge_id, cents);
        return Ok(charge_id);
    }
    let providers = providers();
    let (_, chosen) = providers
        .iter()
        .find(|(name, _)| *name == provider)
        .ok_or_else(|| format!("unknown payment provider: {provider}"))?;
    let charge_id = chosen.charge(cents)?;
    save_order(&charge_id, cents);
    Ok(charge_id)
}
