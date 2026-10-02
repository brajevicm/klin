use super::exporter::Exporter;

pub struct CsvExporter;

impl Exporter for CsvExporter {
    fn export(&self, rows: &[Vec<String>]) -> String {
        rows.iter().map(|row| row.join(",")).collect::<Vec<_>>().join("\n")
    }
}
