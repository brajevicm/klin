use fmtx::center;

#[test]
fn a_line_sits_in_the_middle_of_the_field() {
    assert_eq!(center("ab", 6), "  ab");
    assert_eq!(center("abc", 7), "  abc");
}

#[test]
fn an_odd_remainder_goes_on_the_right() {
    assert_eq!(center("ab", 5), " ab");
    assert_eq!(center("a", 4), " a");
}

#[test]
fn a_line_as_wide_as_the_field_is_unchanged() {
    assert_eq!(center("abcd", 4), "abcd");
    assert_eq!(center("abcde", 4), "abcde");
}

#[test]
fn every_line_is_centred_on_its_own() {
    assert_eq!(center("ab\nabcd", 6), "  ab\n abcd");
}
