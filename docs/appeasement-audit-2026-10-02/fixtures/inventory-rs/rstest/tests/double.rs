use app::double;

#[test]
fn doubles_zero() {
    assert_eq!(double(0), 0);
}

#[rstest::rstest]
fn doubles_one() {
    assert_eq!(double(1), 2);
}
