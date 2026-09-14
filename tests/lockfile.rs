mod harness;

use harness::Tree;

fn manifest(dependencies: &str) -> String {
    format!("[package]\nname = \"t\"\n\n[dependencies]\n{dependencies}")
}

fn locked(names: &[&str]) -> String {
    names
        .iter()
        .map(|name| format!("[[package]]\nname = \"{name}\"\nversion = \"1.0.0\"\n\n"))
        .collect()
}

fn rust_tree() -> Tree {
    let tree = Tree::new();
    tree.write("Cargo.toml", &manifest("serde = \"=1.0.0\"\n"));
    tree.write("Cargo.lock", &locked(&["serde"]));
    tree.base();
    tree
}

#[test]
fn a_dependency_in_the_base_state_is_held() {
    let tree = rust_tree();
    let run = tree.run(&["gate", "--gate", "lockfile"]);
    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn a_new_rust_dependency_with_no_lockfile_entry_fails_as_new() {
    let tree = rust_tree();
    tree.write(
        "Cargo.toml",
        &manifest("serde = \"=1.0.0\"\nregex = \"=1.0.0\"\n"),
    );
    let run = tree.run(&["gate", "--gate", "lockfile"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("1 new dependenc(ies)"), "{}", run.out);
    assert!(
        run.says("Cargo.toml:0  unlocked 1, unpinned 0  regex"),
        "{}",
        run.out
    );
}

#[test]
fn a_rust_lockfile_entry_that_went_fails_as_worsened() {
    let tree = rust_tree();
    tree.write("Cargo.lock", &locked(&["other"]));
    let run = tree.run(&["gate", "--gate", "lockfile"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("dependenc(ies) got worse"), "{}", run.out);
    assert!(
        run.says("unlocked 1, unpinned 0, was unlocked 0, unpinned 0  serde"),
        "{}",
        run.out
    );
}

#[test]
fn a_rust_pin_that_became_a_range_fails_as_worsened() {
    let tree = rust_tree();
    tree.write("Cargo.toml", &manifest("serde = \"1.0\"\n"));
    let run = tree.run(&["gate", "--gate", "lockfile"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("unlocked 0, unpinned 1, was unlocked 0, unpinned 0  serde"),
        "{}",
        run.out
    );
}

#[test]
fn a_new_dependency_with_a_range_and_a_lockfile_entry_does_not_fail() {
    let tree = rust_tree();
    tree.write(
        "Cargo.toml",
        &manifest("serde = \"=1.0.0\"\nregex = \"1.0\"\n"),
    );
    tree.write("Cargo.lock", &locked(&["serde", "regex"]));
    let run = tree.run(&["gate", "--gate", "lockfile"]);
    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn a_path_a_git_and_a_workspace_dependency_are_not_judged_for_unlocked() {
    let tree = rust_tree();
    tree.write(
        "Cargo.toml",
        &manifest(
            "serde = \"=1.0.0\"\nnear = { path = \"../near\" }\n\
             far = { git = \"https://example.com/far\" }\nshared.workspace = true\n",
        ),
    );
    let run = tree.run(&["gate", "--gate", "lockfile"]);
    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn a_deleted_lockfile_fails_every_dependency_of_its_manifest() {
    let tree = Tree::new();
    tree.write(
        "Cargo.toml",
        &manifest("serde = \"=1.0.0\"\nregex = \"=1.0.0\"\n"),
    );
    tree.write("Cargo.lock", &locked(&["serde", "regex"]));
    tree.base();
    tree.remove("Cargo.lock");
    let run = tree.run(&["gate", "--gate", "lockfile"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("2 dependenc(ies) got worse"), "{}", run.out);
    assert!(run.says("  regex"), "{}", run.out);
    assert!(run.says("  serde"), "{}", run.out);
}

#[test]
fn a_workspace_lockfile_above_the_member_manifest_is_found() {
    let tree = Tree::new();
    tree.write("crates/a/Cargo.toml", &manifest("serde = \"=1.0.0\"\n"));
    tree.write("Cargo.lock", &locked(&["serde"]));
    tree.base();
    tree.write(
        "crates/a/Cargo.toml",
        &manifest("serde = \"=1.0.0\"\nregex = \"=1.0.0\"\n"),
    );
    let run = tree.run(&["gate", "--gate", "lockfile"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("crates/a/Cargo.toml:0  unlocked 1, unpinned 0  regex"),
        "{}",
        run.out
    );
}

#[test]
fn a_manifest_with_no_lockfile_in_either_tree_is_a_note_and_no_finding() {
    let tree = Tree::new();
    tree.write("Cargo.toml", &manifest("serde = \"=1.0.0\"\n"));
    tree.base();
    tree.write(
        "Cargo.toml",
        &manifest("serde = \"=1.0.0\"\nregex = \"=1.0.0\"\n"),
    );
    let run = tree.run(&["gate", "--gate", "lockfile"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("NOTE: Cargo.toml has no lockfile beside it in either tree"),
        "{}",
        run.out
    );
}

fn npm_tree(lockfile: &str) -> Tree {
    let tree = Tree::new();
    tree.write("package.json", r#"{"dependencies": {"left-pad": "1.0.0"}}"#);
    tree.write("package-lock.json", lockfile);
    tree.base();
    tree
}

const NPM_V3: &str = r#"{"lockfileVersion": 3,
    "packages": {"": {"name": "t"}, "node_modules/left-pad": {"version": "1.0.0"}}}"#;
const NPM_V1: &str = r#"{"lockfileVersion": 1,
    "dependencies": {"left-pad": {"version": "1.0.0"}}}"#;

#[test]
fn both_npm_lockfile_versions_hold_a_dependency_in_the_base_state() {
    for lockfile in [NPM_V3, NPM_V1] {
        let tree = npm_tree(lockfile);
        let run = tree.run(&["gate", "--gate", "lockfile"]);
        assert_eq!(run.code, 0, "{}", run.out);
    }
}

#[test]
fn a_new_npm_dependency_with_no_lockfile_entry_fails_as_new() {
    for lockfile in [NPM_V3, NPM_V1] {
        let tree = npm_tree(lockfile);
        tree.write(
            "package.json",
            r#"{"dependencies": {"left-pad": "1.0.0", "right-pad": "2.0.0"}}"#,
        );
        let run = tree.run(&["gate", "--gate", "lockfile"]);
        assert_eq!(run.code, 1, "{}", run.out);
        assert!(run.says("1 new dependenc(ies)"), "{}", run.out);
        assert!(
            run.says("package.json:0  unlocked 1, unpinned 0  right-pad"),
            "{}",
            run.out
        );
    }
}

#[test]
fn an_npm_lockfile_entry_that_went_fails_and_a_pin_that_became_a_range_fails() {
    let tree = npm_tree(NPM_V3);
    tree.write(
        "package-lock.json",
        r#"{"lockfileVersion": 3, "packages": {}}"#,
    );
    let gone = tree.run(&["gate", "--gate", "lockfile"]);
    assert_eq!(gone.code, 1, "{}", gone.out);
    assert!(
        gone.says("unlocked 1, unpinned 0, was unlocked 0, unpinned 0  left-pad"),
        "{}",
        gone.out
    );

    let tree = npm_tree(NPM_V3);
    tree.write(
        "package.json",
        r#"{"dependencies": {"left-pad": "^1.0.0"}}"#,
    );
    let range = tree.run(&["gate", "--gate", "lockfile"]);
    assert_eq!(range.code, 1, "{}", range.out);
    assert!(
        range.says("unlocked 0, unpinned 1, was unlocked 0, unpinned 0  left-pad"),
        "{}",
        range.out
    );
}

#[test]
fn an_unreadable_lockfile_format_is_a_note_and_judges_no_manifest() {
    let tree = Tree::new();
    tree.write("package.json", r#"{"dependencies": {"left-pad": "1.0.0"}}"#);
    tree.write("yarn.lock", "left-pad@1.0.0:\n  version \"1.0.0\"\n");
    tree.base();
    let run = tree.run(&["gate", "--gate", "lockfile", "--json"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("yarn.lock is a lockfile format klin cannot read yet"),
        "{}",
        run.out
    );
    assert!(run.says("\"findings\":[]"), "{}", run.out);
}

#[test]
fn a_malformed_lockfile_is_a_tool_error_naming_the_file() {
    let tree = Tree::new();
    tree.write("package.json", r#"{"dependencies": {"left-pad": "1.0.0"}}"#);
    tree.write("package-lock.json", "{ not json");
    tree.base();
    let run = tree.run(&["gate", "--gate", "lockfile"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(
        run.says("package-lock.json is not valid JSON"),
        "{}",
        run.out
    );
}

fn derived_tree() -> Tree {
    let tree = Tree::new();
    tree.write("Cargo.toml", &manifest("serde = \"=1.0.0\"\n"));
    tree.write("Cargo.lock", &locked(&["serde"]));
    tree.write("src/main.rs", "fn main() {}\n");
    tree
}

#[test]
fn a_derived_manifest_klin_cannot_parse_is_a_note_and_every_other_manifest_is_judged() {
    let tree = derived_tree();
    tree.write("testdata/broken/package.json", "{ not json");
    tree.base();
    tree.write(
        "Cargo.toml",
        &manifest("serde = \"=1.0.0\"\nregex = \"=1.0.0\"\n"),
    );
    let run = tree.run(&["gate", "--gate", "lockfile"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("NOTE: testdata/broken/package.json is not valid JSON"),
        "{}",
        run.out
    );
    assert!(
        run.says("Cargo.toml:0  unlocked 1, unpinned 0  regex"),
        "{}",
        run.out
    );
}

#[test]
fn a_derived_manifest_that_parsed_at_the_base_and_does_not_parse_now_is_a_tool_error() {
    let tree = derived_tree();
    tree.write("tools/package.json", r#"{"dependencies": {}}"#);
    tree.base();
    tree.write("tools/package.json", "{ not json");
    let run = tree.run(&["gate", "--gate", "lockfile"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(
        run.says("tools/package.json is not valid JSON"),
        "{}",
        run.out
    );
}

#[test]
fn a_derived_manifest_that_did_not_parse_at_the_base_is_judged_once_it_parses() {
    let tree = derived_tree();
    tree.write("tools/package.json", "{ not json");
    tree.write(
        "tools/package-lock.json",
        r#"{"lockfileVersion": 3, "packages": {}}"#,
    );
    tree.base();
    tree.write(
        "tools/package.json",
        r#"{"dependencies": {"left-pad": "1.0.0"}}"#,
    );
    let run = tree.run(&["gate", "--gate", "lockfile"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("tools/package.json:0  unlocked 1, unpinned 0  left-pad"),
        "{}",
        run.out
    );
}

/// The manifests are the ones the survey finds, so a manifest klin cannot parse in either tree is
/// a fixture and a NOTE, and no person's list can claim otherwise. Spec 8.2.1, ADR 0040.
#[test]
fn a_manifest_klin_could_never_parse_is_a_note_and_no_tool_error() {
    let tree = Tree::new();
    tree.write("package.json", "{ not json");
    tree.write("package-lock.json", NPM_V3);
    tree.base();
    let run = tree.run(&["gate", "--gate", "lockfile"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("NOTE: package.json is not valid JSON"),
        "{}",
        run.out
    );
}

#[test]
fn an_in_that_selects_no_manifest_is_a_tool_error() {
    let tree = rust_tree();
    tree.write("klin.json", r#"{"lockfile": {"in": "web"}}"#);
    let run = tree.run(&["gate", "--gate", "lockfile"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(
        run.says("\"lockfile\" has an \"in\" scope with no applicable manifest"),
        "{}",
        run.out
    );
}

/// Two manifests of one workspace share the lockfile above them, and each is judged against it.
#[test]
fn two_manifests_that_share_one_lockfile_are_each_judged_against_it() {
    let tree = Tree::new();
    tree.write("crates/a/Cargo.toml", &manifest("serde = \"=1.0.0\"\n"));
    tree.write("crates/b/Cargo.toml", &manifest("regex = \"=1.0.0\"\n"));
    tree.write("Cargo.lock", &locked(&["serde", "regex"]));
    tree.base();
    tree.write("Cargo.lock", &locked(&["serde"]));
    let run = tree.run(&["gate", "--gate", "lockfile"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says(
            "crates/b/Cargo.toml:0  unlocked 1, unpinned 0, was unlocked 0, unpinned 0  regex"
        ),
        "{}",
        run.out
    );
}

fn go_tree() -> Tree {
    let tree = Tree::new();
    tree.write(
        "go.mod",
        "module t\n\ngo 1.22\n\nrequire (\n\texample.com/a v1.0.0\n)\n",
    );
    tree.write("go.sum", "example.com/a v1.0.0 h1:abc=\n");
    tree.base();
    tree
}

#[test]
fn a_go_require_in_the_base_state_is_held_and_a_new_one_fails() {
    let tree = go_tree();
    let held = tree.run(&["gate", "--gate", "lockfile"]);
    assert_eq!(held.code, 0, "{}", held.out);

    tree.write(
        "go.mod",
        "module t\n\ngo 1.22\n\nrequire (\n\texample.com/a v1.0.0\n\texample.com/b v2.0.0\n)\n",
    );
    let run = tree.run(&["gate", "--gate", "lockfile"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("go.mod:0  unlocked 1, unpinned 0  example.com/b"),
        "{}",
        run.out
    );
}

#[test]
fn a_go_sum_entry_that_went_fails_as_worsened() {
    let tree = go_tree();
    tree.write("go.sum", "example.com/other v1.0.0 h1:abc=\n");
    let run = tree.run(&["gate", "--gate", "lockfile"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("unlocked 1, unpinned 0, was unlocked 0, unpinned 0  example.com/a"),
        "{}",
        run.out
    );
}

#[test]
fn a_go_module_replaced_by_a_local_path_is_not_judged_for_unlocked() {
    let tree = go_tree();
    tree.write(
        "go.mod",
        "module t\n\ngo 1.22\n\nrequire (\n\texample.com/a v1.0.0\n\texample.com/b v2.0.0\n)\n\
         \nreplace example.com/b => ../b\n",
    );
    let run = tree.run(&["gate", "--gate", "lockfile"]);
    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn under_changed_a_changed_lockfile_with_an_unchanged_manifest_is_still_judged() {
    let tree = rust_tree();
    tree.write("Cargo.lock", &locked(&["other"]));
    let run = tree.run(&["gate", "--gate", "lockfile", "--changed"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("unlocked 1, unpinned 0, was unlocked 0, unpinned 0  serde"),
        "{}",
        run.out
    );
}

#[test]
fn an_accepted_entry_keyed_by_the_manifest_and_the_name_holds_a_finding() {
    let tree = rust_tree();
    tree.write(
        "Cargo.toml",
        &manifest("serde = \"=1.0.0\"\nregex = \"=1.0.0\"\n"),
    );
    tree.write(
        "klin.json",
        r#"{"accepted": [{"gate": "lockfile", "file": "Cargo.toml", "text": "regex",
                          "unlocked": 1, "unpinned": 0}]}"#,
    );
    let run = tree.run(&["gate", "--gate", "lockfile"]);
    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn a_manifest_the_scope_takes_out_is_not_judged() {
    let tree = rust_tree();
    tree.write(
        "Cargo.toml",
        &manifest("serde = \"=1.0.0\"\nregex = \"=1.0.0\"\n"),
    );
    tree.write("klin.json", r#"{"lockfile": {"except": "Cargo.toml"}}"#);
    let run = tree.run(&["gate", "--gate", "lockfile"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("(1 file(s) found, 0 measured, 1 excluded, 0 unreadable)"),
        "{}",
        run.out
    );
}

#[test]
fn the_survey_derives_the_manifests_of_a_tree_with_no_configuration() {
    let tree = Tree::new();
    tree.write("Cargo.toml", &manifest("serde = \"=1.0.0\"\n"));
    tree.write("Cargo.lock", &locked(&["serde"]));
    tree.write("src/main.rs", "fn main() {}\n");
    tree.base();
    tree.write(
        "Cargo.toml",
        &manifest("serde = \"=1.0.0\"\nregex = \"=1.0.0\"\n"),
    );
    let run = tree.run(&["gate", "--gate", "lockfile"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("derived: lockfile manifests Cargo.toml"),
        "{}",
        run.out
    );
    assert!(
        run.says("Cargo.toml:0  unlocked 1, unpinned 0  regex"),
        "{}",
        run.out
    );
}

#[test]
fn a_sub_table_and_a_target_table_are_read_like_any_dependency_table() {
    let tree = Tree::new();
    let held = "[package]\nname = \"t\"\n\n[dependencies.serde]\nversion = \"=1.0.0\"\n\
                features = [\"derive\"]\n\n[dependencies.near]\npath = \"../near\"\n\n\
                [target.'cfg(unix)'.dependencies]\nlibc = \"=0.2.0\"\n";
    tree.write("Cargo.toml", held);
    tree.write("Cargo.lock", &locked(&["serde", "libc"]));
    tree.base();
    let green = tree.run(&["gate", "--gate", "lockfile"]);
    assert_eq!(green.code, 0, "{}", green.out);

    tree.write(
        "Cargo.toml",
        &format!("{held}\n[dependencies.regex]\nversion = \"=1.0.0\"\n"),
    );
    let run = tree.run(&["gate", "--gate", "lockfile"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("Cargo.toml:0  unlocked 1, unpinned 0  regex"),
        "{}",
        run.out
    );
}

#[test]
fn a_dotted_key_states_one_field_and_undoes_nothing_another_line_stated() {
    let tree = Tree::new();
    tree.write("Cargo.toml", &manifest("serde.version = \"=1.0.0\"\n"));
    tree.write("Cargo.lock", &locked(&["serde"]));
    tree.base();
    tree.write(
        "Cargo.toml",
        &manifest("serde.version = \"=1.0.0\"\nserde.features = [\"derive\"]\n"),
    );
    let run = tree.run(&["gate", "--gate", "lockfile"]);
    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn a_dotted_path_key_is_not_judged_for_unlocked_either() {
    let tree = rust_tree();
    tree.write(
        "Cargo.toml",
        &manifest("serde = \"=1.0.0\"\nnear.path = \"../near\"\n"),
    );
    let run = tree.run(&["gate", "--gate", "lockfile"]);
    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn an_inline_table_written_over_several_lines_is_read_as_one_dependency() {
    let tree = Tree::new();
    tree.write(
        "Cargo.toml",
        &manifest("serde = { version = \"=1.0.0\", features = [\"derive\"] }\n"),
    );
    tree.write("Cargo.lock", &locked(&["serde"]));
    tree.base();
    tree.write(
        "Cargo.toml",
        &manifest("serde = {\n    version = \"=1.0.0\",\n    features = [\n        \"derive\",\n    ],\n}\n"),
    );
    let run = tree.run(&["gate", "--gate", "lockfile"]);
    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn a_brace_inside_a_comment_hides_no_dependency_below_it() {
    let tree = rust_tree();
    tree.write(
        "Cargo.toml",
        &manifest("# serde_json { is slower\nserde = \"1.0\"\n"),
    );
    let run = tree.run(&["gate", "--gate", "lockfile"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("unlocked 0, unpinned 1, was unlocked 0, unpinned 0  serde"),
        "{}",
        run.out
    );
}

#[test]
fn a_renamed_dependency_is_locked_by_the_package_the_lockfile_records() {
    let tree = rust_tree();
    tree.write(
        "Cargo.toml",
        &manifest("serde = \"=1.0.0\"\nserde1 = { package = \"serde\", version = \"=1.0.0\" }\n"),
    );
    let run = tree.run(&["gate", "--gate", "lockfile"]);
    assert_eq!(run.code, 0, "{}", run.out);
}
