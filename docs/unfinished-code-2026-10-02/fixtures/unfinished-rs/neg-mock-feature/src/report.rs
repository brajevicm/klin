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
    rows.iter()
        .map(|row| format!("{},{}", row.name, row.amount))
        .collect::<Vec<_>>()
        .join("\n")
}

pub struct MockResponse {
    pub status: u16,
    pub body: String,
}

pub fn create_mock_response(status: u16, body: &str) -> MockResponse {
    MockResponse { status, body: body.to_string() }
}
