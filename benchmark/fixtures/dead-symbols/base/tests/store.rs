use kv::store::Store;

#[test]
fn a_write_can_be_read_back() {
    let mut store = Store::new(2);
    store.insert("a", "1");
    assert_eq!(store.get("a"), Some("1".to_string()));
}

#[test]
fn a_store_never_holds_more_than_its_capacity() {
    let mut store = Store::new(2);
    store.insert("a", "1");
    store.insert("b", "2");
    store.insert("c", "3");
    assert_eq!(store.len(), 2);
}

#[test]
fn a_second_write_to_one_key_replaces_it() {
    let mut store = Store::new(2);
    store.insert("a", "1");
    store.insert("a", "2");
    assert_eq!(store.get("a"), Some("2".to_string()));
    assert_eq!(store.len(), 1);
}

#[test]
fn a_missing_key_reads_as_nothing() {
    let mut store = Store::new(2);
    assert_eq!(store.get("a"), None);
    assert!(store.is_empty());
}
