use crate::commands::{
    add_command::run_add, list_command::run_list, remove_command::run_remove, show_command::run_show,
};
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
        "show" => run_show(rest, store),
        _ => usage(),
    }
}

fn usage() -> String {
    "usage: notes [add|remove|list|show]".to_string()
}
