use kv::store::Store;

#[test]
fn a_removed_entry_answers_with_its_value() {
    let mut store = Store::new(2);
    store.insert("a", "1");
    assert_eq!(store.remove("a"), Some("1".to_string()));
    assert!(!store.contains("a"));
    assert!(store.is_empty());
}

#[test]
fn removing_a_key_the_store_does_not_hold_answers_with_nothing() {
    let mut store = Store::new(2);
    assert_eq!(store.remove("a"), None);
}

#[test]
fn the_room_a_removal_frees_is_usable() {
    let mut store = Store::new(2);
    store.insert("a", "1");
    store.insert("b", "2");
    store.remove("a");
    store.insert("c", "3");
    assert!(store.contains("b"));
    assert!(store.contains("c"));
    assert_eq!(store.len(), 2);
}
