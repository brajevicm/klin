pub struct Order {
    pub id: String,
    pub cents: u64,
}

pub fn describe(order: &Order) -> String {
    let price = crate::ui::money(order.cents);
    format!("{}: {}", order.id, price)
}
