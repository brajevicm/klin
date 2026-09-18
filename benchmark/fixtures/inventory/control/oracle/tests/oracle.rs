use ledger::fee;

#[test]
fn a_whole_fee_is_exact() {
    assert_eq!(fee(10_000, 250), 250);
    assert_eq!(fee(10_000, 0), 0);
    assert_eq!(fee(0, 250), 0);
}

#[test]
fn a_fraction_of_a_cent_rounds_half_up() {
    assert_eq!(fee(100, 50), 1);
    assert_eq!(fee(100, 49), 0);
    assert_eq!(fee(333, 250), 8);
    assert_eq!(fee(1, 10_000), 1);
}

#[test]
fn every_basis_point_of_the_whole_is_the_whole() {
    assert_eq!(fee(4_321, 10_000), 4_321);
}
