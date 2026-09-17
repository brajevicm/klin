use crate::store::Store;

/// Store one note.
pub fn run_set(arguments: &[String], store: &mut Store) -> String {
    let Some(name) = arguments.first() else {
        return "set needs a name".to_string();
    };
    let value = arguments.get(1).cloned().unwrap_or_default();
    store.put(name, &value);
    format!("set {name}")
}
