pub fn add_command(args: &[&str]) -> String {
    format!("add {}", args.join(" "))
}
