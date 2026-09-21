use crate::store::Store;

pub fn run_set(arguments: &[String], store: &mut Store) -> String {
    let Some(name) = arguments.first() else {
        return "set needs a name".to_string();
    };
    match arguments.get(1) {
        Some(value) => {
            store.put(name, value);
            format!("set {name}")
        }
        None if store.take(name) => format!("cleared {name}"),
        None => format!("no note named {name}"),
    }
}
