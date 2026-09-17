use crate::store::Store;

/// Name every note, one per line.
pub fn run_list(_arguments: &[String], store: &Store) -> String {
    store.names().join("\n")
}
