/// Code point ranges a terminal shows two columns wide.
// TODO: the rest of the East Asian wide ranges, such as emoji and the CJK extension planes
const WIDE: [(u32, u32); 5] = [
    (0x3041, 0x30FF),
    (0x4E00, 0x9FFF),
    (0xAC00, 0xD7A3),
    (0xFF01, 0xFF60),
    (0xFFE0, 0xFFE6),
];

fn columns(character: char) -> usize {
    let point = u32::from(character);
    if (0x0300..=0x036F).contains(&point) {
        0
    } else if WIDE.iter().any(|&(first, last)| (first..=last).contains(&point)) {
        2
    } else {
        1
    }
}

/// The number of terminal columns `text` takes.
pub fn width(text: &str) -> usize {
    text.chars().map(columns).sum()
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
