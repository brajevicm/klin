/// Code point ranges a terminal shows two columns wide: the East Asian wide and fullwidth blocks.
const WIDE: [(u32, u32); 19] = [
    (0x1100, 0x115F),
    (0x231A, 0x231B),
    (0x2329, 0x232A),
    (0x2E80, 0x303E),
    (0x3041, 0x33FF),
    (0x3400, 0x4DBF),
    (0x4E00, 0x9FFF),
    (0xA000, 0xA4CF),
    (0xA960, 0xA97F),
    (0xAC00, 0xD7A3),
    (0xF900, 0xFAFF),
    (0xFE10, 0xFE19),
    (0xFE30, 0xFE6F),
    (0xFF00, 0xFF60),
    (0xFFE0, 0xFFE6),
    (0x1F300, 0x1F64F),
    (0x1F900, 0x1F9FF),
    (0x20000, 0x2FFFD),
    (0x30000, 0x3FFFD),
];

/// Code point ranges a terminal shows as no column: combining marks, joiners and variation selectors.
const ZERO: [(u32, u32); 8] = [
    (0x0300, 0x036F),
    (0x0483, 0x0489),
    (0x1AB0, 0x1AFF),
    (0x1DC0, 0x1DFF),
    (0x200B, 0x200F),
    (0x20D0, 0x20FF),
    (0xFE00, 0xFE0F),
    (0xFE20, 0xFE2F),
];

fn within(ranges: &[(u32, u32)], point: u32) -> bool {
    ranges.iter().any(|&(first, last)| (first..=last).contains(&point))
}

fn columns(character: char) -> usize {
    let point = u32::from(character);
    if within(&ZERO, point) {
        0
    } else if within(&WIDE, point) {
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
