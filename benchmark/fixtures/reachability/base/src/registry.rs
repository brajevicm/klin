use crate::commands::{add_command::run_add, list_command::run_list, remove_command::run_remove};
use crate::store::Store;

/// Answer one command line.
pub fn dispatch(arguments: &[String], store: &mut Store) -> String {
    let Some((command, rest)) = arguments.split_first() else {
        return usage();
    };
    match command.as_str() {
        "add" => run_add(rest, store),
        "remove" => run_remove(rest, store),
        "list" => run_list(rest, store),
        _ => usage(),
    }
}

fn usage() -> String {
    "usage: notes [add|remove|list]".to_string()
}
