pub fn export_command(args: &[&str]) -> String {
    format!("export {}", args.join(" "))
}
