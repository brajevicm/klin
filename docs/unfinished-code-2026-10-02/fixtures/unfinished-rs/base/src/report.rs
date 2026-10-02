pub struct Row {
    pub name: String,
    pub amount: i64,
}

pub fn to_json(rows: &[Row]) -> String {
    let items: Vec<String> = rows
        .iter()
        .map(|row| format!("{{\"name\":\"{}\",\"amount\":{}}}", row.name, row.amount))
        .collect();
    format!("[{}]", items.join(","))
}
