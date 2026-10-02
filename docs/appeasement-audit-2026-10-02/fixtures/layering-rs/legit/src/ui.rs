use crate::domain::Order;

pub fn money(cents: u64) -> String {
    format!("€{}.{:02}", cents / 100, cents % 100)
}

pub fn summary(order: &Order) -> String {
    money(order.cents)
}

pub fn describe(order: &Order) -> String {
    format!("{}: {}", order.id, money(order.cents))
}
