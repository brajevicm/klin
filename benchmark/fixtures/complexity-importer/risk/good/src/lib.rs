/// Why an upload holds no rows.
#[derive(Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The file's name ends in no suffix this crate reads.
    Unsupported(String),
    /// Every line of the file was blank or a comment.
    Empty,
}

/// How one kind of file separates its fields.
enum Fields {
    /// Fields separated by one character, trimmed or kept as they are.
    Separated { by: char, trimmed: bool },
    /// Fields separated by any run of whitespace.
    Words,
}

const FORMATS: [(&str, Fields); 6] = [
    (".csv", Fields::Separated { by: ',', trimmed: true }),
    (".tsv", Fields::Separated { by: '\t', trimmed: false }),
    (".tab", Fields::Separated { by: '\t', trimmed: false }),
    (".psv", Fields::Separated { by: '|', trimmed: true }),
    (".ssv", Fields::Separated { by: ';', trimmed: true }),
    (".txt", Fields::Words),
];

impl Fields {
    fn of(name: &str) -> Option<&'static Fields> {
        let lower = name.to_ascii_lowercase();
        FORMATS.iter().find(|(suffix, _)| lower.ends_with(suffix)).map(|(_, fields)| fields)
    }

    fn split(&self, line: &str) -> Vec<String> {
        match *self {
            Fields::Separated { by, trimmed: true } => line.split(by).map(|field| field.trim().to_string()).collect(),
            Fields::Separated { by, trimmed: false } => line.split(by).map(str::to_string).collect(),
            Fields::Words => line.split_whitespace().map(str::to_string).collect(),
        }
    }
}

/// The rows of one uploaded file, each a list of its fields, read by the file's name.
pub fn read_rows(name: &str, text: &str) -> Result<Vec<Vec<String>>, Refusal> {
    let fields = Fields::of(name).ok_or_else(|| Refusal::Unsupported(name.to_string()))?;
    let rows: Vec<Vec<String>> = text
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .map(|line| fields.split(line))
        .collect();
    if rows.is_empty() {
        return Err(Refusal::Empty);
    }
    Ok(rows)
}
