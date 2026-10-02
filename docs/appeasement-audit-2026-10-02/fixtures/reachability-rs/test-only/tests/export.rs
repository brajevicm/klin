use app::commands::export_command::export_command;

#[test]
fn exports() {
    assert_eq!(export_command(&["a"]), "export a");
}
