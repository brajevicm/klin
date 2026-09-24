use tabulate::{render, width};

#[test]
fn a_chinese_character_takes_two_columns() {
    assert_eq!(width("梨"), 2);
    assert_eq!(width("苹果"), 4);
}

#[test]
fn kana_take_two_columns() {
    assert_eq!(width("なし"), 4);
    assert_eq!(width("リンゴ"), 6);
}

#[test]
fn a_hangul_syllable_takes_two_columns() {
    assert_eq!(width("사과"), 4);
}

#[test]
fn a_fullwidth_letter_takes_two_columns() {
    assert_eq!(width("ＡＢ"), 4);
}

#[test]
fn a_combining_mark_takes_no_column() {
    assert_eq!(width("cafe\u{301}"), 4);
    assert_eq!(width("n\u{303}u"), 2);
}

#[test]
fn mixed_rows_line_up() {
    let rows = vec![vec!["梨", "4"], vec!["pears", "12"], vec!["cafe\u{301}", "1"]];
    assert_eq!(render(&rows), "梨     4\npears  12\ncafe\u{301}   1\n");
}

#[test]
fn plain_text_is_unchanged() {
    assert_eq!(width("pears"), 5);
    let rows = vec![vec!["pears", "4"], vec!["figs", "12"]];
    assert_eq!(render(&rows), "pears  4\nfigs   12\n");
}
