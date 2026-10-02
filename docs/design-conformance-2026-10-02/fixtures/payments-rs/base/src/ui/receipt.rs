use super::labels::status_label;
use crate::db::orders::order_total;
use crate::money::format::format_money;

pub fn receipt(charge_id: &str) -> String {
    match order_total(charge_id) {
        Some(total) => format!("{charge_id}: {} {}", format_money(total as i64, "eur"), status_label("paid")),
        None => format!("{charge_id}: {}", status_label("pending")),
    }
}
