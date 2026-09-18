//! The report writer's column layout.

/// The width of the widest cell in each column.
// HACK: a cell holding a tab is measured as one character.
pub fn widths(rows: &[Vec<String>]) -> Vec<usize> {
    let mut found: Vec<usize> = Vec::new();
    for row in rows {
        for (column, cell) in row.iter().enumerate() {
            if column >= found.len() {
                found.push(0);
            }
            found[column] = found[column].max(cell.chars().count());
        }
    }
    found
}
