use super::registry::providers;
use crate::db::orders::save_order;
use super::service::PaymentService;

pub fn checkout(provider: &str, cents: u64) -> crate::Result<String> {
    let providers = providers();
    let (_, chosen) = providers
        .iter()
        .find(|(name, _)| *name == provider)
        .ok_or_else(|| format!("unknown payment provider: {provider}"))?;
    let charge_id = PaymentService::new(chosen.as_ref()).charge(cents)?;
    save_order(&charge_id, cents);
    Ok(charge_id)
}
