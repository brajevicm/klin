use fmtx::wrap;

#[test]
fn a_paragraph_breaks_at_spaces() {
    assert_eq!(wrap("one two three", 7), vec!["one two", "three"]);
    assert_eq!(wrap("a b c", 10), vec!["a b c"]);
    assert_eq!(wrap("", 5), Vec::<String>::new());
}

#[test]
fn no_line_runs_past_the_width() {
    let text = "the quick brown fox jumps over the lazy dog";
    for width in 4..20 {
        for line in wrap(text, width) {
            assert!(line.chars().count() <= width, "{line:?} is wider than {width}");
        }
    }
}

#[test]
fn a_long_word_breaks_with_a_hyphen() {
    assert_eq!(wrap("antidisestablishment", 6), vec!["antid-", "isest-", "ablis-", "hment"]);
    assert_eq!(wrap("abcdef", 6), vec!["abcdef"]);
}
