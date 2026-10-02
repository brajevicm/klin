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

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_rows() -> Vec<Row> {
        Vec::new()
    }

    #[test]
    fn empty() {
        assert_eq!(to_csv(&fake_rows()), "");
    }
}
