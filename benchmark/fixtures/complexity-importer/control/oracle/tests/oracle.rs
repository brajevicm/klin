use uploads::{columns, read_rows, Refusal};

#[test]
fn the_widest_row_gives_the_column_count() {
    assert_eq!(columns("a.csv", "a,b\nc,d,e\nf\n"), Ok(3));
}

#[test]
fn a_refused_file_is_refused_the_same_way() {
    assert_eq!(columns("a.xlsx", "a\n"), Err(Refusal::Unsupported("a.xlsx".to_string())));
    assert_eq!(columns("a.csv", "# only\n"), Err(Refusal::Empty));
}

#[test]
fn the_rows_that_were_already_there_are_unchanged() {
    assert_eq!(read_rows("a.txt", "a   b\n").map(|rows| rows.len()), Ok(1));
    assert_eq!(read_rows("a.tsv", " a\tb \n"), Ok(vec![vec![" a".to_string(), "b ".to_string()]]));
}
