use permalink::{short_slug, slug};

#[test]
fn words_are_lowercased_and_joined_by_one_dash() {
    assert_eq!(slug("The Spring Issue"), "the-spring-issue");
}

#[test]
fn punctuation_and_spaces_at_the_ends_go() {
    assert_eq!(slug("  Hello, World!  "), "hello-world");
}

#[test]
fn digits_stay() {
    assert_eq!(slug("Top 10 of 2026"), "top-10-of-2026");
}

#[test]
fn a_long_slug_is_cut_where_a_dash_stands() {
    assert_eq!(short_slug("The Spring Issue", 12), "the-spring");
    assert_eq!(short_slug("Extraordinarily long", 5), "extra");
}
