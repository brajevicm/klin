use crate::store::Store;

/// Write every note as one line of `name=value`.
pub fn run_export(_arguments: &[String], store: &Store) -> String {
    store
        .names()
        .iter()
        .map(|name| format!("{name}={}", store.get(name).unwrap_or_default()))
        .collect::<Vec<_>>()
        .join("\n")
}
