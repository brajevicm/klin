use tabulate::{render, width};

#[test]
fn plain_text_takes_one_column_a_character() {
    assert_eq!(width("pears"), 5);
    assert_eq!(width(""), 0);
}

#[test]
fn a_column_is_as_wide_as_its_widest_cell() {
    let rows = vec![vec!["pears", "4"], vec!["figs", "12"]];
    assert_eq!(render(&rows), "pears  4\nfigs   12\n");
}

#[test]
fn a_short_row_ends_early() {
    let rows = vec![vec!["a", "b", "c"], vec!["dd"]];
    assert_eq!(render(&rows), "a   b  c\ndd\n");
}

#[test]
fn wide_characters_take_two_columns() {
    assert_eq!(width("梨なし사과"), 10);
}

#[test]
fn a_combining_mark_takes_no_column() {
    assert_eq!(width("cafe\u{301}"), 4);
}
