use std::collections::HashMap;

struct Entry {
    value: String,
    used: u64,
    written: u64,
}

/// A bounded store. A write into a full store makes room first.
pub struct Store {
    entries: HashMap<String, Entry>,
    capacity: usize,
    clock: u64,
}

impl Store {
    pub fn new(capacity: usize) -> Store {
        Store {
            entries: HashMap::new(),
            capacity,
            clock: 0,
        }
    }

    pub fn insert(&mut self, key: &str, value: &str) {
        self.clock += 1;
        if !self.entries.contains_key(key) && self.entries.len() >= self.capacity {
            evict_newest_write(&mut self.entries);
        }
        let clock = self.clock;
        let entry = self.entries.entry(key.to_string()).or_insert(Entry {
            value: String::new(),
            used: clock,
            written: clock,
        });
        entry.value = value.to_string();
        entry.written = clock;
    }

    pub fn get(&mut self, key: &str) -> Option<String> {
        Some(self.entries.get(key)?.value.clone())
    }

    pub fn contains(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

fn touch(entry: &mut Entry, clock: u64) {
    entry.used = clock;
}

fn evict_oldest_use(entries: &mut HashMap<String, Entry>) {
    let oldest = entries
        .iter()
        .min_by_key(|(_, entry)| entry.used)
        .map(|(key, _)| key.clone());
    if let Some(key) = oldest {
        entries.remove(&key);
    }
}

fn evict_newest_write(entries: &mut HashMap<String, Entry>) {
    let newest = entries
        .iter()
        .max_by_key(|(_, entry)| entry.written)
        .map(|(key, _)| key.clone());
    if let Some(key) = newest {
        entries.remove(&key);
    }
}

fn checksum(text: &str) -> u64 {
    text.bytes()
        .fold(0u64, |sum, byte| sum.wrapping_mul(31).wrapping_add(u64::from(byte)))
}
