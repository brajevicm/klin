use app::double;

#[test]
fn doubles_zero() {
    assert_eq!(double(0), 0);
}

#[test]
fn doubles_one() {
    assert_eq!(double(1), 2);
}

#[test]
fn doubles_ten() {
    assert_eq!(double(10), 20);
}
