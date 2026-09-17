use crate::store::Store;

/// Name every note, one per line, or as a JSON array.
pub fn run_list(arguments: &[String], store: &Store) -> String {
    let names = store.names();
    if arguments.first().map(String::as_str) == Some("--json") {
        let quoted: Vec<String> = names.iter().map(|name| quote(name)).collect();
        return format!("[{}]", quoted.join(","));
    }
    names.join("\n")
}

fn quote(name: &str) -> String {
    let escaped = name.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}
