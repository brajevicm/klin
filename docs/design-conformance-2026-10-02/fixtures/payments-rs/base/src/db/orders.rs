use std::collections::HashMap;
use std::sync::Mutex;

static ORDERS: Mutex<Option<HashMap<String, u64>>> = Mutex::new(None);

pub fn save_order(charge_id: &str, cents: u64) {
    let mut orders = ORDERS.lock().unwrap();
    orders.get_or_insert_with(HashMap::new).insert(charge_id.to_string(), cents);
}

pub fn order_total(charge_id: &str) -> Option<u64> {
    ORDERS.lock().unwrap().as_ref().and_then(|orders| orders.get(charge_id).copied())
}
