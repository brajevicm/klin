use ledger::format_cents;

#[test]
fn cents_render_with_two_places() {
    assert_eq!(format_cents(1234), "12.34");
    assert_eq!(format_cents(-5), "-0.05");
}
