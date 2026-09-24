use uploads::{read_rows, Refusal};

fn owned(rows: &[&[&str]]) -> Vec<Vec<String>> {
    rows.iter().map(|row| row.iter().map(|field| field.to_string()).collect()).collect()
}

#[test]
fn semicolon_fields_are_trimmed_and_keep_a_decimal_comma() {
    assert_eq!(read_rows("buchung.ssv", "1,50 ; Kaffee\n"), Ok(owned(&[&["1,50", "Kaffee"]])));
}

#[test]
fn the_new_suffix_is_read_in_any_case() {
    assert_eq!(read_rows("BUCHUNG.SSV", "a;b\n"), Ok(owned(&[&["a", "b"]])));
}

#[test]
fn semicolon_files_skip_blank_and_comment_lines() {
    assert_eq!(read_rows("buchung.ssv", "# Kopf\n\na;b\n"), Ok(owned(&[&["a", "b"]])));
    assert_eq!(read_rows("buchung.ssv", "\n# nur\n"), Err(Refusal::Empty));
}

#[test]
fn the_suffixes_that_were_already_there_are_unchanged() {
    assert_eq!(read_rows("a.csv", "a, b ,c\n"), Ok(owned(&[&["a", "b", "c"]])));
    assert_eq!(read_rows("a.tsv", " a\tb \n"), Ok(owned(&[&[" a", "b "]])));
    assert_eq!(read_rows("a.tab", "a\tb\n"), Ok(owned(&[&["a", "b"]])));
    assert_eq!(read_rows("a.psv", "a | b\n"), Ok(owned(&[&["a", "b"]])));
    assert_eq!(read_rows("a.txt", "a   b\tc\n"), Ok(owned(&[&["a", "b", "c"]])));
    assert_eq!(read_rows("a.xlsx", "a\n"), Err(Refusal::Unsupported("a.xlsx".to_string())));
}
