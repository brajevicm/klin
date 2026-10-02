pub trait Exporter {
    fn export(&self, rows: &[Vec<String>]) -> String;
}
