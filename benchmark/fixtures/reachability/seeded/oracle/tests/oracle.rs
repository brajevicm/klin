use notes::registry::dispatch;
use notes::store::Store;

fn line(store: &mut Store, arguments: &[&str]) -> String {
    let owned: Vec<String> = arguments.iter().map(|one| one.to_string()).collect();
    dispatch(&owned, store)
}

#[test]
fn show_answers_with_the_value_a_note_holds() {
    let mut store = Store::new();
    line(&mut store, &["add", "a", "1"]);
    line(&mut store, &["add", "b", "two words"]);
    assert_eq!(line(&mut store, &["show", "a"]), "1");
    assert_eq!(line(&mut store, &["show", "b"]), "two words");
}

#[test]
fn show_answers_for_a_name_the_keeper_does_not_hold() {
    let mut store = Store::new();
    assert_eq!(line(&mut store, &["show", "a"]), "no note named a");
    assert_eq!(line(&mut store, &["show"]), "show needs a name");
}

#[test]
fn the_existing_commands_still_answer() {
    let mut store = Store::new();
    assert_eq!(line(&mut store, &["add", "a", "1"]), "added a");
    assert_eq!(line(&mut store, &["list"]), "a");
    assert_eq!(line(&mut store, &["remove", "a"]), "removed a");
    assert!(line(&mut store, &["what"]).starts_with("usage: notes"));
}
