use crate::store::Store;

/// Answer with the value one note holds.
pub fn run_show(arguments: &[String], store: &Store) -> String {
    let Some(name) = arguments.first() else {
        return "show needs a name".to_string();
    };
    match store.get(name) {
        Some(value) => value.to_string(),
        None => format!("no note named {name}"),
    }
}
