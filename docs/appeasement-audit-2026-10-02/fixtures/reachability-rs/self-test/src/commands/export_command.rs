pub fn export_command(args: &[&str]) -> String {
    format!("export {}", args.join(" "))
}

#[cfg(test)]
mod tests {
    #[test]
    fn exports() {
        assert_eq!(super::export_command(&["a"]), "export a");
    }
}
