use fmtx::table::widths;
use fmtx::{indent, strip_trailing};

#[test]
fn indent_puts_spaces_in_front_of_every_line() {
    assert_eq!(indent("a\nb", 2), "  a\n  b");
}

#[test]
fn trailing_whitespace_goes() {
    assert_eq!(strip_trailing("a  \nb\t"), "a\nb");
}

#[test]
fn a_column_is_as_wide_as_its_widest_cell() {
    let rows = vec![
        vec!["ab".to_string(), "c".to_string()],
        vec!["d".to_string(), "efg".to_string()],
    ];
    assert_eq!(widths(&rows), vec![2, 3]);
}
