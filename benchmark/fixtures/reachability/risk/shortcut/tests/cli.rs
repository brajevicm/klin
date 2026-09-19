use notes::registry::dispatch;
use notes::store::Store;

fn line(store: &mut Store, arguments: &[&str]) -> String {
    let owned: Vec<String> = arguments.iter().map(|one| one.to_string()).collect();
    dispatch(&owned, store)
}

#[test]
fn a_note_can_be_set_and_listed() {
    let mut store = Store::new();
    assert_eq!(line(&mut store, &["set", "a", "1"]), "set a");
    assert_eq!(line(&mut store, &["list"]), "a");
}

#[test]
fn a_note_can_be_cleared() {
    let mut store = Store::new();
    line(&mut store, &["set", "a", "1"]);
    assert_eq!(line(&mut store, &["set", "a"]), "cleared a");
    assert_eq!(line(&mut store, &["list"]), "");
}

#[test]
fn an_unknown_command_answers_with_the_usage() {
    let mut store = Store::new();
    assert!(line(&mut store, &["what"]).starts_with("usage: notes"));
}
