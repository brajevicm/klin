mod harness;

use harness::Tree;
use serde_json::Value;

const A_PROMPT: &str = r#"{"hook_event_name": "UserPromptSubmit"}"#;
const A_STOP: &str = r#"{"hook_event_name": "Stop", "stop_hook_active": false}"#;

fn findings(run: &harness::Run) -> Vec<String> {
    run.out
        .lines()
        .filter(|line| line.contains("src/commands/"))
        .map(|line| line.trim().to_string())
        .collect()
}

fn config(tree: &Tree) -> Value {
    let text = std::fs::read_to_string(tree.path("klin.json"))
        .unwrap_or_else(|why| panic!("klin.json could not be read: {why}"));
    serde_json::from_str(&text).unwrap_or_else(|why| panic!("klin.json is not JSON: {why}\n{text}"))
}

fn three_reached_commands() -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", r#"{}"#);
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

fn three_reached_handlers() -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", r#"{}"#);
    for name in ["Login", "Logout", "Reset"] {
        tree.write(
            &format!("web/handlers/{name}Handler.ts"),
            &format!("export function {name}() {{}}\n"),
        );
    }
    tree.write("web/app.ts", "export const app = [Login, Logout, Reset];\n");
    tree
}

#[test]
fn a_family_the_base_proves_judges_a_new_unreached_member() {
    let tree = three_reached_commands();
    tree.base();
    tree.write("src/commands/delta_command.rs", "pub fn run_delta() {}\n");

    let run = tree.run(&["reachability"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("derived: reachability src/commands/*_command.rs"),
        "{}",
        run.out
    );
    assert!(run.says("1 new unreached file(s)"), "{}", run.out);
    assert!(run.says("src/commands/delta_command.rs"), "{}", run.out);
}

#[test]
fn json_exposes_the_derived_family_topology() {
    let tree = three_reached_commands();
    tree.base();

    let run = tree.run(&["gate", "--json", "--gate", "reachability"]);

    assert_eq!(run.code, 0, "{}", run.out);
    let report = run.json();
    let derived = report["derived"]
        .as_array()
        .unwrap_or_else(|| panic!("no derived list in {report}"));
    let family = &derived[0]["value"][0];
    assert_eq!(derived[0]["section"], "reachability", "{}", run.out);
    assert_eq!(
        family["roots"],
        serde_json::json!(["src/commands"]),
        "{}",
        run.out
    );
    assert_eq!(family["pattern"], "*_command.rs", "{}", run.out);
}

#[test]
fn a_new_member_another_file_references_passes() {
    let tree = three_reached_commands();
    tree.base();
    tree.write("src/commands/delta_command.rs", "pub fn run_delta() {}\n");
    tree.write("src/other.rs", "fn other() { run_delta(); }\n");

    let run = tree.run(&["reachability"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("4 file(s) judged, 0 unreached"), "{}", run.out);
}

#[test]
fn losing_the_last_external_reference_is_worsened() {
    let tree = three_reached_commands();
    tree.base();
    tree.write("src/main.rs", "fn main() { run_beta(); run_gamma(); }\n");

    let run = tree.run(&["reachability"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("got worse"), "{}", run.out);
    assert!(run.says("src/commands/alpha_command.rs"), "{}", run.out);
}

#[test]
fn a_reference_from_the_same_file_does_not_reach_it() {
    let tree = three_reached_commands();
    tree.base();
    tree.write(
        "src/commands/delta_command.rs",
        "pub fn run_delta() {}\nfn again() { run_delta(); }\n",
    );

    let run = tree.run(&["reachability"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/commands/delta_command.rs"), "{}", run.out);
}

#[test]
fn a_method_name_reference_alone_does_not_reach_a_file() {
    let tree = three_reached_commands();
    tree.base();
    tree.write(
        "src/commands/delta_command.rs",
        "pub struct Delta;\nimpl Delta { pub fn go(&self) {} }\n",
    );
    tree.write("src/other.rs", "fn other() { held.go(); }\n");

    let run = tree.run(&["reachability"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/commands/delta_command.rs"), "{}", run.out);
}

#[test]
fn a_file_with_only_entry_points_is_measured_and_not_judged() {
    let tree = three_reached_commands();
    tree.base();
    tree.write("src/commands/delta_command.rs", "#[test]\nfn works() {}\n");

    let run = tree.run(&["reachability"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("1 measured with no eligible declaration"),
        "{}",
        run.out
    );
}

#[test]
fn one_ambiguous_reference_reaches_every_matching_declaration() {
    let tree = three_reached_commands();
    tree.base();
    tree.write("src/commands/delta_command.rs", "pub fn run_extra() {}\n");
    tree.write("src/commands/epsilon_command.rs", "pub fn run_extra() {}\n");
    tree.write("src/other.rs", "fn other() { run_extra(); }\n");

    let run = tree.run(&["reachability"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("5 file(s) judged, 0 unreached"), "{}", run.out);
}

#[test]
fn the_remedy_names_a_proven_reached_sibling() {
    let tree = three_reached_commands();
    tree.base();
    tree.write("src/commands/delta_command.rs", "pub fn run_delta() {}\n");

    let run = tree.run(&["reachability"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("a reached sibling is src/commands/"),
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
fn a_reference_in_another_language_does_not_reach_a_rust_family_member() {
    let tree = three_reached_commands();
    tree.base();
    tree.write("src/commands/delta_command.rs", "pub fn run_delta() {}\n");
    tree.write(
        "web/caller.ts",
        "export function caller() { run_delta(); }\n",
    );

    let run = tree.run(&["reachability"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/commands/delta_command.rs"), "{}", run.out);
}

#[test]
fn a_typescript_family_is_derived_and_judged_in_its_language() {
    let tree = three_reached_handlers();
    tree.base();
    tree.write(
        "web/handlers/ProfileHandler.ts",
        "export function Profile() {}\n",
    );

    let run = tree.run(&["reachability"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("web/handlers/*Handler.ts"), "{}", run.out);
    assert!(run.says("web/handlers/ProfileHandler.ts"), "{}", run.out);
}

#[test]
fn two_members_do_not_derive_a_family() {
    let tree = Tree::new();
    tree.write("src/commands/alpha_command.rs", "pub fn run_alpha() {}\n");
    tree.write("src/commands/beta_command.rs", "pub fn run_beta() {}\n");
    tree.write("src/main.rs", "fn main() { run_alpha(); run_beta(); }\n");
    tree.base();

    let run = tree.run(&["reachability"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("derived: reachability"), "{}", run.out);
    assert!(run.says("0 file(s) judged"), "{}", run.out);
}

#[test]
fn one_unreached_member_derives_no_broad_family() {
    let tree = three_reached_commands();
    tree.write("src/main.rs", "fn main() { run_alpha(); run_beta(); }\n");
    tree.base();

    let run = tree.run(&["reachability"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("derived: reachability"), "{}", run.out);
}

#[test]
fn a_file_the_grammar_rejects_keeps_the_unparsed_rule() {
    let tree = three_reached_commands();
    tree.base();
    tree.write("src/commands/delta_command.rs", "pub fn broken( {\n");

    let run = tree.run(&["reachability"]);

    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("1 unreadable"), "{}", run.out);
}

#[test]
fn changed_mode_still_resolves_against_unchanged_callers() {
    let tree = three_reached_commands();
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
fn excepting_a_file_measured_at_the_base_reports_lost_coverage() {
    let tree = three_reached_commands();
    tree.base();
    tree.write(
        "klin.json",
        r#"{"reachability":{"except":"src/commands/alpha_command.rs"}}"#,
    );

    let run = tree.run(&["reachability"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("src/commands/alpha_command.rs was measured at the base"),
        "{}",
        run.out
    );
}

#[test]
fn false_disables_reachability_and_a_person_authored_family_is_rejected() {
    let tree = three_reached_commands();
    tree.base();
    tree.write("klin.json", r#"{"reachability":false}"#);

    let disabled = tree.run(&["gate", "--list"]);
    assert_eq!(disabled.code, 0, "{}", disabled.out);
    assert!(disabled.says("reachability — excluded"), "{}", disabled.out);

    tree.write(
        "klin.json",
        r#"{"reachability":[{"name":"mine","roots":["src"],"pattern":"*_command.rs"}]}"#,
    );
    let retired = tree.run(&["reachability"]);
    assert_eq!(retired.code, 2, "{}", retired.out);
    assert!(
        retired.says("no longer accepts a list of entries"),
        "{}",
        retired.out
    );
}

#[test]
fn typescript_and_tsx_derive_separate_concrete_families_in_one_order() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{}"#);
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

    let run = tree.run(&["reachability"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("web/handlers/*Handler.ts, web/handlers/*Handler.tsx"),
        "{}",
        run.out
    );
}

#[test]
fn init_does_not_serialize_derived_reachability_topology() {
    let tree = three_reached_commands();
    tree.base();

    let run = tree.run(&["init", "--pin"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!config(&tree)["reachability"].is_array());
}

#[test]
fn init_pin_keeps_an_explicit_false() {
    let tree = three_reached_commands();
    tree.base();
    tree.write("klin.json", r#"{"reachability":false}"#);

    let run = tree.run(&["init", "--pin"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(config(&tree)["reachability"], Value::Bool(false));
}

fn derives_no_family(tree: &Tree) {
    let run = tree.run(&["reachability"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("derived: reachability"), "{}", run.out);
}

#[test]
fn a_test_directory_under_a_family_root_stays_in_the_cohort_it_must_prove() {
    let tree = three_reached_commands();
    tree.write(
        "src/commands/__tests__/delta_command.rs",
        "pub fn run_delta() {}\n",
    );
    tree.base();

    derives_no_family(&tree);
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

    derives_no_family(&tree);
}

#[test]
fn a_member_reached_only_through_ambiguity_derives_no_family() {
    let tree = three_reached_commands();
    tree.write("src/commands/gamma_command.rs", "pub fn run_alpha() {}\n");
    tree.base();

    derives_no_family(&tree);
}

#[test]
fn a_member_with_no_eligible_declaration_derives_no_family() {
    let tree = three_reached_commands();
    tree.write("src/commands/gamma_command.rs", "#[test]\nfn works() {}\n");
    tree.base();

    derives_no_family(&tree);
}

#[test]
fn a_member_the_grammar_rejects_derives_no_family() {
    let tree = three_reached_commands();
    tree.write("src/commands/gamma_command.rs", "pub fn run_gamma( {\n");
    tree.base();

    derives_no_family(&tree);
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

    derives_no_family(&tree);
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

    let run = tree.run(&["reachability"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("derived: reachability src/commands/*_command.rs,"),
        "{}",
        run.out
    );
    assert!(!run.says("*_user_command.rs"), "{}", run.out);

    tree.write("src/commands/list_command.rs", "pub fn list() {}\n");
    tree.base();

    let run = tree.run(&["reachability"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("derived: reachability src/commands/*_user_command.rs,"),
        "{}",
        run.out
    );
    assert!(!run.says("src/commands/*_command.rs"), "{}", run.out);
}

#[test]
fn an_accepted_path_holds_a_new_unreached_member() {
    let tree = three_reached_commands();
    tree.base();
    tree.write("src/commands/delta_command.rs", "pub fn run_delta() {}\n");
    tree.write(
        "klin.json",
        r#"{"accepted":[{"gate":"reachability","file":"src/commands/delta_command.rs","text":"file","unreached":1}]}"#,
    );

    let run = tree.run(&["reachability"]);

    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn a_deleted_member_is_no_reachability_finding() {
    let tree = three_reached_commands();
    tree.base();
    tree.remove("src/commands/alpha_command.rs");

    let run = tree.run(&["reachability"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("2 file(s) judged, 0 unreached"), "{}", run.out);
}

#[test]
fn a_changed_run_judges_a_member_a_dispatch_edit_stopped_referencing() {
    let tree = three_reached_commands();
    tree.write(
        "src/main.rs",
        "fn main() { run_alpha(); run_beta(); run_gamma(); }\n",
    );
    tree.base();
    tree.write("src/main.rs", "fn main() { run_gamma(); }\n");

    let changed = tree.run(&["gate", "--changed", "--gate", "reachability"]);
    let whole = tree.run(&["gate", "--gate", "reachability"]);

    assert_eq!(changed.code, 1, "{}", changed.out);
    assert_eq!(changed.code, whole.code, "{}\n{}", changed.out, whole.out);
    assert!(
        changed.says("src/commands/alpha_command.rs"),
        "{}",
        changed.out
    );
    assert!(
        changed.says("src/commands/beta_command.rs"),
        "{}",
        changed.out
    );
    assert!(!changed.says("all held at the base"), "{}", changed.out);
    assert_eq!(
        findings(&changed),
        findings(&whole),
        "{}\n{}",
        changed.out,
        whole.out
    );
}

#[test]
fn the_stop_hook_blocks_a_turn_that_left_a_member_unreached() {
    let tree = three_reached_commands();
    tree.base();
    let prompt = harness::feed(tree.root(), &["radius"], A_PROMPT);
    assert_eq!(prompt.code, 0, "{}", prompt.out);
    tree.write("src/main.rs", "fn main() { run_gamma(); }\n");

    let run = harness::feed(tree.root(), &["gate", "--hook", "--changed"], A_STOP);

    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("src/commands/alpha_command.rs"), "{}", run.out);
    assert!(run.says("src/commands/beta_command.rs"), "{}", run.out);
}
