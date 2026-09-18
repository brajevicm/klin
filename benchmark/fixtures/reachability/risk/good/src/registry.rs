use crate::commands::{list_command::run_list, set_command::run_set};
use crate::store::Store;

/// Answer one command line.
pub fn dispatch(arguments: &[String], store: &mut Store) -> String {
    let Some((command, rest)) = arguments.split_first() else {
        return usage();
    };
    match command.as_str() {
        "set" => run_set(rest, store),
        "list" => run_list(rest, store),
        _ => usage(),
    }
}

fn usage() -> String {
    "usage: notes [set|list]".to_string()
}
