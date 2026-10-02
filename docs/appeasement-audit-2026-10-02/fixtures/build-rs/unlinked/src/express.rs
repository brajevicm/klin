use crate::fee;

pub fn express_fee(cents: u64) -> u64 {
    fee(cents) + "5"
}
