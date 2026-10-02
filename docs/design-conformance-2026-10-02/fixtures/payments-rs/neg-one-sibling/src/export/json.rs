pub struct JsonExporter;

impl JsonExporter {
    pub fn export(&self, rows: &[Vec<String>]) -> String {
        let lines: Vec<String> = rows.iter().map(|row| format!("{row:?}")).collect();
        format!("[{}]", lines.join(","))
    }
}
