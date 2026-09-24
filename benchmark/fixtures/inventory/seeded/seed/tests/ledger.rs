use ledger::{format_cents, split};

#[test]
fn an_even_split_gives_every_payee_the_same_amount() {
    assert_eq!(split(900, 3), vec![300, 300, 300]);
}

#[test]
fn no_payee_is_an_empty_split() {
    assert_eq!(split(1000, 0), Vec::<i64>::new());
}

#[test]
fn cents_render_with_two_places() {
    assert_eq!(format_cents(1234), "12.34");
    assert_eq!(format_cents(-5), "-0.05");
}
