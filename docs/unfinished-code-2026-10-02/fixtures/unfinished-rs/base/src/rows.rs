use crate::report::Row;

pub fn load_rows(path: &str) -> Vec<Row> {
    std::fs::read_to_string(path)
        .map(|text| {
            text.lines()
                .filter_map(|line| line.split_once(','))
                .map(|(name, amount)| Row {
                    name: name.to_string(),
                    amount: amount.trim().parse().unwrap_or(0),
                })
                .collect()
        })
        .unwrap_or_default()
}
