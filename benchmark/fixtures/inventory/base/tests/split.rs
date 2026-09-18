use ledger::{format_cents, split, total};

#[test]
fn an_even_split_gives_every_payee_the_same_amount() {
    assert_eq!(split(900, 3), vec![300, 300, 300]);
}

#[test]
fn remainder_goes_to_the_first_payee() {
    assert_eq!(split(1000, 3), vec![334, 333, 333]);
}

#[test]
fn a_split_keeps_the_whole_amount() {
    assert_eq!(total(&split(1000, 3)), 1000);
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
