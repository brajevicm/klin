/// The number of terminal columns `text` takes.
pub fn width(text: &str) -> usize {
    text.chars().count()
}

/// The rows as a table: each column as wide as its widest cell, two spaces between columns,
/// and no space at the end of a line. Every line ends with a newline.
pub fn render(rows: &[Vec<&str>]) -> String {
    let columns = rows.iter().map(Vec::len).max().unwrap_or(0);
    let widths: Vec<usize> = (0..columns)
        .map(|at| rows.iter().filter_map(|row| row.get(at)).map(|cell| width(cell)).max().unwrap_or(0))
        .collect();
    let mut table = String::new();
    for row in rows {
        let mut line = String::new();
        for (at, cell) in row.iter().enumerate() {
            if at > 0 {
                line.push_str("  ");
            }
            line.push_str(cell);
            line.push_str(&" ".repeat(widths[at] - width(cell)));
        }
        table.push_str(line.trim_end());
        table.push('\n');
    }
    table
}
