pub struct Order {
    pub id: String,
    pub cents: u64,
}

pub fn describe(order: &Order) -> String {
    format!("{}: {}", order.id, crate::ui::money(order.cents))
}
