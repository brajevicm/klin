pub mod report;
pub mod rows;

pub fn main(path: &str) -> String {
    report::to_json(&rows::load_rows(path))
}

pub fn main_csv(path: &str) -> String {
    let rows = rows::load_rows(path);
    report::to_csv(&rows)
}
