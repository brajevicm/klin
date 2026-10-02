pub fn list_command(args: &[&str]) -> String {
    format!("list {}", args.join(" "))
}
