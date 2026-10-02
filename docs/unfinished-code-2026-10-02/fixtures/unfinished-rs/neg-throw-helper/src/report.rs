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

fn invalid(row: &Row) -> ! {
    panic!("row {} has no amount", row.name)
}

pub fn to_csv(rows: &[Row]) -> String {
    rows.iter()
        .map(|row| if row.amount < 0 { invalid(row) } else { format!("{},{}", row.name, row.amount) })
        .collect::<Vec<_>>()
        .join("\n")
}
