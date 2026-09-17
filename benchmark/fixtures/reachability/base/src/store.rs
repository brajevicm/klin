use std::collections::BTreeMap;

/// The notes, by name.
#[derive(Default)]
pub struct Store {
    notes: BTreeMap<String, String>,
}

impl Store {
    pub fn new() -> Store {
        Store::default()
    }

    pub fn put(&mut self, name: &str, value: &str) {
        self.notes.insert(name.to_string(), value.to_string());
    }

    pub fn take(&mut self, name: &str) -> bool {
        self.notes.remove(name).is_some()
    }

    pub fn names(&self) -> Vec<&str> {
        self.notes.keys().map(String::as_str).collect()
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        self.notes.get(name).map(String::as_str)
    }
}
