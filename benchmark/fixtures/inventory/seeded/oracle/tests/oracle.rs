use ledger::{split, total};

#[test]
fn the_remainder_goes_to_the_earliest_payees() {
    assert_eq!(split(1000, 3), vec![334, 333, 333]);
    assert_eq!(split(1001, 3), vec![334, 334, 333]);
    assert_eq!(split(100, 4), vec![25, 25, 25, 25]);
    assert_eq!(split(7, 2), vec![4, 3]);
}

#[test]
fn a_split_keeps_the_whole_amount_whatever_the_remainder() {
    for total_cents in 0..64 {
        for payees in 1..6 {
            assert_eq!(total(&split(total_cents, payees)), total_cents);
        }
    }
}

#[test]
fn two_payees_never_differ_by_more_than_one_cent() {
    for total_cents in 0..64 {
        for payees in 1..6 {
            let amounts = split(total_cents, payees);
            let low = amounts.iter().min().copied().unwrap_or(0);
            let high = amounts.iter().max().copied().unwrap_or(0);
            assert!(high - low <= 1, "{total_cents} over {payees} spread too far");
        }
    }
}

#[test]
fn no_payee_is_an_empty_split() {
    assert_eq!(split(1000, 0), Vec::<i64>::new());
}
