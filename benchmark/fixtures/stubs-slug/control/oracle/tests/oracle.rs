use permalink::{short_slug, slug};

#[test]
fn a_slug_is_cut_where_a_dash_stands() {
    assert_eq!(short_slug("The Spring Issue", 12), "the-spring");
    assert_eq!(short_slug("The Spring Issue", 10), "the-spring");
}

#[test]
fn a_short_slug_is_unchanged() {
    assert_eq!(short_slug("The Spring Issue", 40), "the-spring-issue");
}

#[test]
fn a_long_first_word_is_cut_at_the_limit() {
    assert_eq!(short_slug("Extraordinarily long", 5), "extra");
}

#[test]
fn the_slug_that_was_already_there_is_unchanged() {
    assert_eq!(slug("  Hello, World!  "), "hello-world");
}
