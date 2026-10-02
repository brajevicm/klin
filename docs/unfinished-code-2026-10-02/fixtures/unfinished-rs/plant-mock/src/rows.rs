use crate::report::Row;

pub fn load_rows(path: &str) -> Vec<Row> {
    let _ = path;
    mock_rows()
}

fn mock_rows() -> Vec<Row> {
    vec![Row { name: "alpha".to_string(), amount: 1 }]
}
