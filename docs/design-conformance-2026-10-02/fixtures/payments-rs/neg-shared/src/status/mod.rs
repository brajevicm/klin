pub fn status_label(status: &str) -> &'static str {
    if status == "paid" {
        "Paid"
    } else {
        "Pending"
    }
}
