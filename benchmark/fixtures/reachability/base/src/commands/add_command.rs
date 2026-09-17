use crate::store::Store;

/// Store one note, and say what happened.
pub fn run_add(arguments: &[String], store: &mut Store) -> String {
    let Some(name) = arguments.first() else {
        return "add needs a name".to_string();
    };
    let value = arguments.get(1).cloned().unwrap_or_default();
    store.put(name, &value);
    format!("added {name}")
}
