mod harness;

use harness::Tree;
use serde_json::Value;

const COMMANDS: &str = r#"{"reachability":[{"name":"commands","roots":["src/commands"],"pattern":"*_command.rs","languages":["rust"]}]}"#;
const HANDLERS: &str = r#"{"reachability":[{"name":"handlers","roots":["web/handlers"],"pattern":"*Handler.ts","languages":["typescript"]}]}"#;

fn config(tree: &Tree) -> Value {
    let Ok(text) = std::fs::read_to_string(tree.path("klin.json")) else {
        panic!("no klin.json was written")
    };
    match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(why) => panic!("klin.json is not JSON: {why}\n{text}"),
    }
}

/// Three commands the base holds, each declared once under its own name and each called from
/// `src/main.rs`, which is the smallest cohort a family may be derived from.
fn three_reached_commands() -> Tree {
    let tree = Tree::new();
    for name in ["alpha", "beta", "gamma"] {
        tree.write(
            &format!("src/commands/{name}_command.rs"),
            &format!("pub fn run_{name}() {{}}\n"),
        );
    }
    tree.write(
        "src/main.rs",
        "fn main() { run_alpha(); run_beta(); run_gamma(); }\n",
    );
    tree
}

#[test]
fn a_new_command_file_nothing_references_fails_as_new() {
    let tree = Tree::new();
    tree.write("klin.json", COMMANDS);
    tree.write("src/commands/alpha_command.rs", "pub fn run_alpha() {}\n");

    let run = tree.run(&["reachability"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("1 new unreached file(s)"), "{}", run.out);
    assert!(run.says("src/commands/alpha_command.rs"), "{}", run.out);
}

#[test]
fn a_command_another_file_references_passes() {
    let tree = Tree::new();
    tree.write("klin.json", COMMANDS);
    tree.write("src/commands/alpha_command.rs", "pub fn run_alpha() {}\n");
    tree.write("src/main.rs", "fn main() { run_alpha(); }\n");

    let run = tree.run(&["reachability"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("1 file(s) judged, 0 unreached"), "{}", run.out);
}

#[test]
fn losing_the_last_external_reference_is_worsened() {
    let tree = Tree::new();
    tree.write("klin.json", COMMANDS);
    tree.write("src/commands/alpha_command.rs", "pub fn run_alpha() {}\n");
    tree.write("src/main.rs", "fn main() { run_alpha(); }\n");
    tree.base();
    tree.write("src/main.rs", "fn main() {}\n");

    let run = tree.run(&["reachability"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("got worse"), "{}", run.out);
    assert!(run.says("src/commands/alpha_command.rs"), "{}", run.out);
}

#[test]
fn an_unreached_file_the_base_already_held_is_a_note() {
    let tree = Tree::new();
    tree.write("klin.json", COMMANDS);
    tree.write("src/commands/alpha_command.rs", "pub fn run_alpha() {}\n");
    tree.base();

    let run = tree.run(&["reachability"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("NOTE: 1 unreached file(s) the base already held:"),
        "{}",
        run.out
    );
}

#[test]
fn a_reference_from_the_same_file_does_not_reach_it() {
    let tree = Tree::new();
    tree.write("klin.json", COMMANDS);
    tree.write(
        "src/commands/alpha_command.rs",
        "pub fn run_alpha() {}\nfn again() { run_alpha(); }\n",
    );

    let run = tree.run(&["reachability"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("1 new unreached file(s)"), "{}", run.out);
}

#[test]
fn one_ambiguous_reference_reaches_every_file_that_declares_the_name() {
    let tree = Tree::new();
    tree.write("klin.json", COMMANDS);
    tree.write("src/commands/alpha_command.rs", "pub fn run() {}\n");
    tree.write("src/commands/beta_command.rs", "pub fn run() {}\n");
    tree.write("src/main.rs", "fn main() { run(); }\n");

    let run = tree.run(&["reachability"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("2 file(s) judged, 0 unreached"), "{}", run.out);
}

#[test]
fn a_file_with_only_entry_points_or_methods_is_measured_and_not_judged() {
    let tree = Tree::new();
    tree.write("klin.json", COMMANDS);
    tree.write("src/commands/alpha_command.rs", "#[test]\nfn works() {}\n");
    tree.write(
        "src/commands/beta_command.rs",
        "impl super::Beta {\n    pub fn go(&self) {}\n}\n",
    );

    let run = tree.run(&["reachability"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("0 file(s) judged, 0 unreached, 2 measured with no eligible declaration"),
        "{}",
        run.out
    );
    assert!(run.says("2 measured"), "{}", run.out);
}

#[test]
fn a_method_name_reference_alone_does_not_reach_a_file() {
    let tree = Tree::new();
    tree.write("klin.json", COMMANDS);
    tree.write(
        "src/commands/alpha_command.rs",
        "pub struct Alpha;\nimpl Alpha {\n    pub fn go(&self) {}\n}\n",
    );
    tree.write("src/main.rs", "fn main() { held.go(); }\n");

    let run = tree.run(&["reachability"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("1 new unreached file(s)"), "{}", run.out);
}

#[test]
fn the_remedy_names_a_proven_sibling_and_not_one_reached_by_ambiguity() {
    let tree = Tree::new();
    tree.write("klin.json", COMMANDS);
    tree.write("src/commands/alpha_command.rs", "pub fn shared() {}\n");
    tree.write("src/commands/beta_command.rs", "pub fn run_beta() {}\n");
    tree.write("src/commands/gamma_command.rs", "pub fn run_gamma() {}\n");
    tree.write("src/other.rs", "pub fn shared() {}\n");
    tree.write("src/main.rs", "fn main() { shared(); run_beta(); }\n");

    let run = tree.run(&["reachability"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("src/commands/gamma_command.rs:0  unreached, a reached sibling is src/commands/beta_command.rs"),
        "{}",
        run.out
    );
    assert!(
        run.says("Wire this file into the application"),
        "{}",
        run.out
    );
}

#[test]
fn a_new_typescript_handler_fails_and_an_exported_one_another_file_imports_passes() {
    let tree = Tree::new();
    tree.write("klin.json", HANDLERS);
    tree.write(
        "web/handlers/LoginHandler.ts",
        "export function login() {}\n",
    );
    tree.write(
        "web/handlers/LogoutHandler.ts",
        "export function logout() {}\n",
    );
    tree.write("web/app.ts", "export const app = login();\n");

    let run = tree.run(&["reachability"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("1 new unreached file(s)"), "{}", run.out);
    assert!(run.says("web/handlers/LogoutHandler.ts"), "{}", run.out);
    assert!(!run.says("LoginHandler.ts:0"), "{}", run.out);
}

#[test]
fn a_typescript_handler_that_loses_its_reference_is_worsened_and_a_held_one_is_a_note() {
    let tree = Tree::new();
    tree.write("klin.json", HANDLERS);
    tree.write(
        "web/handlers/LoginHandler.ts",
        "export function login() {}\n",
    );
    tree.write("web/handlers/OldHandler.ts", "export function old() {}\n");
    tree.write("web/app.ts", "export const app = login();\n");
    tree.base();
    tree.write("web/app.ts", "export const app = 1;\n");

    let run = tree.run(&["reachability"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("got worse"), "{}", run.out);
    assert!(run.says("web/handlers/LoginHandler.ts"), "{}", run.out);
    assert!(
        run.says("NOTE: 1 unreached file(s) the base already held:"),
        "{}",
        run.out
    );
}

#[test]
fn tsx_is_judged_as_typescript_and_a_method_only_class_body_is_not_judged() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{"reachability":[{"name":"views","roots":["web"],"pattern":"*View.tsx","languages":["typescript"]}]}"#,
    );
    tree.write(
        "web/AdminView.tsx",
        "export const AdminView = () => <p>hi</p>;\n",
    );
    tree.write("web/index.ts", "export const app = AdminView;\n");
    tree.write(
        "web/OtherView.tsx",
        "export const OtherView = () => <p>no</p>;\n",
    );

    let run = tree.run(&["reachability"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("web/OtherView.tsx"), "{}", run.out);
    assert!(!run.says("web/AdminView.tsx:0"), "{}", run.out);
}

#[test]
fn one_ambiguous_typescript_reference_reaches_every_handler_that_declares_the_name() {
    let tree = Tree::new();
    tree.write("klin.json", HANDLERS);
    tree.write(
        "web/handlers/LoginHandler.ts",
        "export function handle() {}\n",
    );
    tree.write(
        "web/handlers/LogoutHandler.ts",
        "export function handle() {}\n",
    );
    tree.write("web/app.ts", "export const app = handle();\n");

    let run = tree.run(&["reachability"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("2 file(s) judged, 0 unreached"), "{}", run.out);
}

#[test]
fn a_test_directory_under_a_family_root_stays_in_the_cohort_it_must_prove() {
    let tree = three_reached_commands();
    tree.write(
        "src/commands/__tests__/delta_command.rs",
        "pub fn run_delta() {}\n",
    );
    tree.base();

    needs_a_section(&tree);
}

#[test]
fn a_file_two_families_match_is_judged_once_and_an_accepted_path_holds_it() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{"reachability":[
            {"name":"commands","roots":["src/commands"],"pattern":"*_command.rs"},
            {"name":"alphas","roots":["src"],"pattern":"alpha_*.rs"}]}"#,
    );
    tree.write("src/commands/alpha_command.rs", "pub fn run_alpha() {}\n");

    let run = tree.run(&["reachability"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("1 new unreached file(s)"), "{}", run.out);
    assert_eq!(
        run.out.matches("alpha_command.rs:0").count(),
        1,
        "{}",
        run.out
    );

    tree.write(
        "klin.json",
        r#"{"accepted":[{"gate":"reachability","file":"src/commands/alpha_command.rs","text":"file","unreached":1}],
            "reachability":[
            {"name":"commands","roots":["src/commands"],"pattern":"*_command.rs"},
            {"name":"alphas","roots":["src"],"pattern":"alpha_*.rs"}]}"#,
    );
    let run = tree.run(&["reachability"]);
    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn a_deleted_unreached_file_is_no_reachability_finding() {
    let tree = Tree::new();
    tree.write("klin.json", COMMANDS);
    tree.write("src/commands/alpha_command.rs", "pub fn run_alpha() {}\n");
    tree.base();
    tree.remove("src/commands/alpha_command.rs");

    let run = tree.run(&["reachability"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("0 file(s) judged"), "{}", run.out);
}

#[test]
fn a_file_the_grammar_rejects_keeps_the_unparsed_rule() {
    let tree = Tree::new();
    tree.write("klin.json", COMMANDS);
    tree.write("src/commands/alpha_command.rs", "pub fn broken( {\n");

    let run = tree.run(&["reachability"]);

    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("1 unreadable"), "{}", run.out);
}

#[test]
fn changed_mode_still_resolves_against_unchanged_callers() {
    let tree = Tree::new();
    tree.write("klin.json", COMMANDS);
    tree.write("src/commands/alpha_command.rs", "pub fn run_alpha() {}\n");
    tree.write("src/main.rs", "fn main() { run_alpha(); }\n");
    tree.base();
    tree.write(
        "src/commands/alpha_command.rs",
        "pub fn run_alpha() {}\npub fn also() {}\n",
    );

    let run = tree.run(&["gate", "--changed", "--gate", "reachability"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("ok    reachability"), "{}", run.out);
}

#[test]
fn a_family_the_base_proves_is_derived_and_judges_a_new_working_tree_member() {
    let tree = three_reached_commands();
    tree.base();
    tree.write("src/commands/delta_command.rs", "pub fn run_delta() {}\n");

    let run = tree.run(&["gate", "--gate", "reachability"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("derived: reachability src/commands/*_command.rs"),
        "{}",
        run.out
    );
    assert!(run.says("src/commands/delta_command.rs"), "{}", run.out);
}

fn needs_a_section(tree: &Tree) {
    let run = tree.run(&["gate", "--list"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("reachability — needs a section a person writes"),
        "{}",
        run.out
    );
    assert!(!run.says("derived: reachability"), "{}", run.out);
}

#[test]
fn two_members_derive_no_family_and_the_working_tree_cannot_add_the_third() {
    let tree = Tree::new();
    tree.write("src/commands/alpha_command.rs", "pub fn run_alpha() {}\n");
    tree.write("src/commands/beta_command.rs", "pub fn run_beta() {}\n");
    tree.write(
        "src/main.rs",
        "fn main() { run_alpha(); run_beta(); run_gamma(); }\n",
    );
    tree.base();
    tree.write("src/commands/gamma_command.rs", "pub fn run_gamma() {}\n");

    needs_a_section(&tree);
}

#[test]
fn one_unreached_member_derives_no_broad_family() {
    let tree = three_reached_commands();
    tree.write("src/main.rs", "fn main() { run_alpha(); run_beta(); }\n");
    tree.base();

    needs_a_section(&tree);
}

#[test]
fn a_member_reached_only_through_ambiguity_derives_no_family() {
    let tree = three_reached_commands();
    tree.write("src/commands/gamma_command.rs", "pub fn run_alpha() {}\n");
    tree.base();

    needs_a_section(&tree);
}

#[test]
fn a_member_with_no_eligible_declaration_derives_no_family() {
    let tree = three_reached_commands();
    tree.write("src/commands/gamma_command.rs", "#[test]\nfn works() {}\n");
    tree.base();

    needs_a_section(&tree);
}

#[test]
fn a_member_the_grammar_rejects_derives_no_family() {
    let tree = three_reached_commands();
    tree.write("src/commands/gamma_command.rs", "pub fn run_gamma( {\n");
    tree.base();

    needs_a_section(&tree);
}

#[test]
fn a_test_root_and_a_whole_extension_derive_no_family() {
    let tree = Tree::new();
    for name in ["alpha", "beta", "gamma"] {
        tree.write(
            &format!("tests/{name}_test.rs"),
            &format!("pub fn check_{name}() {{}}\n"),
        );
        tree.write(
            &format!("src/{name}.rs"),
            &format!("pub fn {name}() {{}}\n"),
        );
    }
    tree.write(
        "src/main.rs",
        "fn main() { alpha(); beta(); gamma(); check_alpha(); check_beta(); check_gamma(); }\n",
    );
    tree.base();

    needs_a_section(&tree);
}

#[test]
fn the_broadest_safe_candidate_wins_and_a_narrow_one_survives_an_unsafe_broad_one() {
    let tree = Tree::new();
    for name in ["create", "delete", "update"] {
        tree.write(
            &format!("src/commands/{name}_user_command.rs"),
            &format!("pub fn {name}_user() {{}}\n"),
        );
    }
    tree.write(
        "src/main.rs",
        "fn main() { create_user(); delete_user(); update_user(); }\n",
    );
    tree.base();

    let run = tree.run(&["gate", "--list"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("derived: reachability src/commands/*_command.rs,"),
        "{}",
        run.out
    );
    assert!(!run.says("*_user_command.rs"), "{}", run.out);

    tree.write("src/commands/list_command.rs", "pub fn list() {}\n");
    tree.base();

    let run = tree.run(&["gate", "--list"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("derived: reachability src/commands/*_user_command.rs,"),
        "{}",
        run.out
    );
    assert!(!run.says("src/commands/*_command.rs"), "{}", run.out);
}

#[test]
fn a_pinned_array_and_false_both_suppress_inference() {
    let tree = three_reached_commands();
    tree.base();
    tree.write(
        "klin.json",
        r#"{"reachability":[{"name":"mine","roots":["src"],"pattern":"*_command.rs"}]}"#,
    );

    let run = tree.run(&["gate", "--list"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("reachability — runs"), "{}", run.out);
    assert!(run.says("pinned: reachability mine"), "{}", run.out);
    assert!(!run.says("derived: reachability"), "{}", run.out);

    tree.write("klin.json", r#"{"reachability": false}"#);
    let run = tree.run(&["gate", "--list"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("reachability — excluded"), "{}", run.out);
}

#[test]
fn typescript_and_tsx_derive_separate_concrete_families_in_one_order() {
    let tree = Tree::new();
    for name in ["Login", "Logout", "Reset"] {
        tree.write(
            &format!("web/handlers/{name}Handler.ts"),
            &format!("export function {name}() {{}}\n"),
        );
        tree.write(
            &format!("web/handlers/{name}Handler.tsx"),
            &format!("export const {name}View = () => <p>{name}</p>;\n"),
        );
    }
    tree.write(
        "web/app.ts",
        "export const app = [Login, Logout, Reset, LoginView, LogoutView, ResetView];\n",
    );
    tree.base();

    let run = tree.run(&["gate", "--list"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("derived: reachability web/handlers/*Handler.ts, web/handlers/*Handler.tsx,"),
        "{}",
        run.out
    );
    assert!(!run.says("*Handler.ts*"), "{}", run.out);
}

#[test]
fn init_pins_the_derived_family_and_add_leaves_an_explicit_section_alone() {
    let tree = three_reached_commands();
    tree.base();

    let run = tree.run(&["init"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let pinned = config(&tree);
    assert_eq!(
        pinned["reachability"],
        serde_json::json!([{
            "name": "src/commands/*_command.rs",
            "roots": ["src/commands"],
            "pattern": "*_command.rs",
            "languages": ["rust"]
        }]),
        "{pinned}"
    );

    tree.write("klin.json", r#"{"reachability": false}"#);
    let run = tree.run(&["init", "--add"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(config(&tree)["reachability"], Value::Bool(false));
}

#[test]
fn init_force_pairs_named_entries_by_name_and_not_by_position() {
    let tree = three_reached_commands();
    tree.base();
    tree.write(
        "klin.json",
        r#"{"reachability":[
            {"name":"gone","roots":["src/old"],"pattern":"*_old.rs","exclude":["src/old/skip_old.rs"]},
            {"name":"src/commands/*_command.rs","roots":["src/commands"],"pattern":"*_command.rs",
             "languages":["rust"],"exclude":["src/commands/skip_command.rs"]}]}"#,
    );

    let run = tree.run(&["init", "--force"]);

    assert_eq!(run.code, 0, "{}", run.out);
    let pinned = config(&tree);
    assert_eq!(
        pinned["reachability"],
        serde_json::json!([{
            "name": "src/commands/*_command.rs",
            "roots": ["src/commands"],
            "pattern": "*_command.rs",
            "languages": ["rust"],
            "exclude": ["src/commands/skip_command.rs"]
        }]),
        "{pinned}"
    );
}
