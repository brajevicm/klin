use crate::commands::{add_command, export_command, list_command, remove_command};

pub fn run(name: &str, args: &[&str]) -> Option<String> {
    match name {
        "add" => Some(add_command::add_command(args)),
        "list" => Some(list_command::list_command(args)),
        "remove" => Some(remove_command::remove_command(args)),
        "export" => Some(export_command::export_command(args)),
        _ => None,
    }
}
