use notes::registry::dispatch;
use notes::store::Store;

fn line(store: &mut Store, arguments: &[&str]) -> String {
    let owned: Vec<String> = arguments.iter().map(|one| one.to_string()).collect();
    dispatch(&owned, store)
}

#[test]
fn an_empty_keeper_answers_with_an_empty_array() {
    let mut store = Store::new();
    assert_eq!(line(&mut store, &["list", "--json"]), "[]");
}

#[test]
fn every_name_is_a_string_in_the_array() {
    let mut store = Store::new();
    line(&mut store, &["add", "b", "2"]);
    line(&mut store, &["add", "a", "1"]);
    assert_eq!(line(&mut store, &["list", "--json"]), "[\"a\",\"b\"]");
}

#[test]
fn a_quote_and_a_backslash_are_escaped() {
    let mut store = Store::new();
    line(&mut store, &["add", "a\"b", "1"]);
    line(&mut store, &["add", "c\\d", "2"]);
    assert_eq!(line(&mut store, &["list", "--json"]), "[\"a\\\"b\",\"c\\\\d\"]");
}

#[test]
fn the_plain_list_is_unchanged() {
    let mut store = Store::new();
    line(&mut store, &["add", "a", "1"]);
    line(&mut store, &["add", "b", "2"]);
    assert_eq!(line(&mut store, &["list"]), "a\nb");
}
