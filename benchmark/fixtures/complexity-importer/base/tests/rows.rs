use uploads::{read_rows, Refusal};

fn owned(rows: &[&[&str]]) -> Vec<Vec<String>> {
    rows.iter().map(|row| row.iter().map(|field| field.to_string()).collect()).collect()
}

#[test]
fn comma_fields_are_trimmed() {
    assert_eq!(read_rows("orders.CSV", "a, b ,c\n"), Ok(owned(&[&["a", "b", "c"]])));
}

#[test]
fn tab_fields_keep_their_spaces() {
    assert_eq!(read_rows("orders.tab", " a\tb \n"), Ok(owned(&[&[" a", "b "]])));
}

#[test]
fn pipe_fields_are_trimmed() {
    assert_eq!(read_rows("orders.psv", "a | b\n"), Ok(owned(&[&["a", "b"]])));
}

#[test]
fn text_fields_split_on_any_run_of_spaces() {
    assert_eq!(read_rows("orders.txt", "a   b\tc\n"), Ok(owned(&[&["a", "b", "c"]])));
}

#[test]
fn blank_and_comment_lines_hold_no_row() {
    assert_eq!(read_rows("orders.csv", "# header\n\na,b\n  \n"), Ok(owned(&[&["a", "b"]])));
    assert_eq!(read_rows("orders.csv", "# only\n"), Err(Refusal::Empty));
}

#[test]
fn an_unknown_suffix_is_refused() {
    assert_eq!(read_rows("orders.xlsx", "a,b\n"), Err(Refusal::Unsupported("orders.xlsx".to_string())));
}
