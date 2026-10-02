use crate::http::transport::send;

pub fn refund_all(charge_ids: &[String]) -> crate::Result<()> {
    for charge_id in charge_ids {
        send("POST", "/refunds", &format!("{{\"charge\":\"{charge_id}\"}}"), 0)?;
    }
    Ok(())
}
