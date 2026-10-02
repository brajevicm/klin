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

pub trait Sink {
    fn lines(&self) -> Vec<String> {
        Vec::new()
    }
}

pub struct NullSink;

impl Sink for NullSink {
    fn lines(&self) -> Vec<String> {
        vec![]
    }
}

impl Default for Row {
    fn default() -> Self {
        Row { name: String::new(), amount: 0 }
    }
}
