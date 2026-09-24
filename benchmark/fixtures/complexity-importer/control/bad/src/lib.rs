/// Why an upload holds no rows.
#[derive(Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The file's name ends in no suffix this crate reads.
    Unsupported(String),
    /// Every line of the file was blank or a comment.
    Empty,
}

/// The rows of one uploaded file, each a list of its fields, read by the file's name.
pub fn read_rows(name: &str, text: &str) -> Result<Vec<Vec<String>>, Refusal> {
    let lower = name.to_ascii_lowercase();
    let mut rows = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() || line.starts_with('#') || line.starts_with("//") {
            continue;
        }
        let fields: Vec<String> = if lower.ends_with(".csv") {
            line.split(',').map(|field| field.trim().to_string()).collect()
        } else if lower.ends_with(".tsv") || lower.ends_with(".tab") {
            line.split('\t').map(|field| field.to_string()).collect()
        } else if lower.ends_with(".psv") {
            line.split('|').map(|field| field.trim().to_string()).collect()
        } else if lower.ends_with(".txt") {
            line.split_whitespace().map(|field| field.to_string()).collect()
        } else {
            return Err(Refusal::Unsupported(name.to_string()));
        };
        rows.push(fields);
    }
    if rows.is_empty() {
        return Err(Refusal::Empty);
    }
    Ok(rows)
}
