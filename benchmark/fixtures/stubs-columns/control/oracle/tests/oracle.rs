use tabulate::render;

#[test]
fn a_column_of_numbers_lines_up_on_the_right() {
    let rows = vec![vec!["pears", "4"], vec!["figs", "12"]];
    assert_eq!(render(&rows), "pears   4\nfigs   12\n");
}

#[test]
fn a_column_with_a_word_lines_up_on_the_left() {
    let rows = vec![vec!["a", "1"], vec!["b", "10"], vec!["c", "x"]];
    assert_eq!(render(&rows), "a  1\nb  10\nc  x\n");
}

#[test]
fn a_first_column_of_numbers_is_padded_in_front() {
    let rows = vec![vec!["1", "a"], vec!["22", "b"], vec!["333"]];
    assert_eq!(render(&rows), "  1  a\n 22  b\n333\n");
}
