use crate::store::Store;

/// Take one note out, and say what happened.
pub fn run_remove(arguments: &[String], store: &mut Store) -> String {
    let Some(name) = arguments.first() else {
        return "remove needs a name".to_string();
    };
    if store.take(name) {
        format!("removed {name}")
    } else {
        format!("no note named {name}")
    }
}
