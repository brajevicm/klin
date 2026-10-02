pub fn remove_command(args: &[&str]) -> String {
    format!("remove {}", args.join(" "))
}
