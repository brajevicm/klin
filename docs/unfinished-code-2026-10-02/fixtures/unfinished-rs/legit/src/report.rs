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

pub fn to_csv(rows: &[Row]) -> String {
    let quote = |field: &str| format!("\"{}\"", field.replace('"', "\"\""));
    let mut lines = vec!["name,amount".to_string()];
    lines.extend(rows.iter().map(|row| format!("{},{}", quote(&row.name), row.amount)));
    lines.join("\n")
}
