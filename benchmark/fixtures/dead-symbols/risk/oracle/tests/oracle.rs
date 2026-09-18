use kv::store::Store;

#[test]
fn the_entry_written_longest_ago_goes_first() {
    let mut store = Store::new(2);
    store.insert("a", "1");
    store.insert("b", "2");
    store.insert("c", "3");
    assert!(!store.contains("a"));
    assert!(store.contains("b"));
    assert!(store.contains("c"));
}

#[test]
fn reading_an_entry_does_not_save_it() {
    let mut store = Store::new(2);
    store.insert("a", "1");
    store.insert("b", "2");
    assert_eq!(store.get("a"), Some("1".to_string()));
    store.insert("c", "3");
    assert!(!store.contains("a"), "a read must not move an entry in the order");
}

#[test]
fn replacing_a_value_keeps_the_key_where_it_was() {
    let mut store = Store::new(2);
    store.insert("a", "1");
    store.insert("b", "2");
    store.insert("a", "9");
    store.insert("c", "3");
    assert!(!store.contains("a"));
    assert_eq!(store.len(), 2);
}

#[test]
fn a_store_never_holds_more_than_its_capacity() {
    let mut store = Store::new(3);
    for key in ["a", "b", "c", "d", "e"] {
        store.insert(key, key);
    }
    assert_eq!(store.len(), 3);
}
