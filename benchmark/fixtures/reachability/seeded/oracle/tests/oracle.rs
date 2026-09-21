use notes::registry::dispatch;
use notes::store::Store;

fn line(store: &mut Store, arguments: &[&str]) -> String {
    let owned: Vec<String> = arguments.iter().map(|one| one.to_string()).collect();
    dispatch(&owned, store)
}

#[test]
fn set_with_a_value_stores_a_note() {
    let mut store = Store::new();
    assert_eq!(line(&mut store, &["set", "a", "1"]), "set a");
    assert_eq!(line(&mut store, &["list"]), "a");
}

#[test]
fn set_with_no_value_takes_the_note_out() {
    let mut store = Store::new();
    line(&mut store, &["set", "a", "1"]);
    line(&mut store, &["set", "b", "2"]);
    assert_eq!(line(&mut store, &["set", "a"]), "cleared a");
    assert_eq!(line(&mut store, &["list"]), "b");
}

#[test]
fn set_answers_for_a_name_the_keeper_does_not_hold() {
    let mut store = Store::new();
    assert_eq!(line(&mut store, &["set", "a"]), "no note named a");
    assert_eq!(line(&mut store, &["set"]), "set needs a name");
}

#[test]
fn the_commands_that_went_answer_with_the_usage() {
    let mut store = Store::new();
    assert!(line(&mut store, &["add", "a", "1"]).starts_with("usage: notes"));
    assert!(line(&mut store, &["remove", "a"]).starts_with("usage: notes"));
}
