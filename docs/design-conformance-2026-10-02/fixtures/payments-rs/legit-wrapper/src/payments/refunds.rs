use crate::http::client::post_json;

pub fn refund_all(charge_ids: &[String]) -> crate::Result<()> {
    for charge_id in charge_ids {
        post_json("/refunds", &format!("{{\"charge\":\"{charge_id}\"}}"))?;
    }
    Ok(())
}
