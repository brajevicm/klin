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
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn a_new_rust_dependency_with_no_lockfile_entry_fails_as_new() {
    let tree = rust_tree();
    tree.write(
        "Cargo.toml",
        &manifest("serde = \"=1.0.0\"\nregex = \"=1.0.0\"\n"),
    );
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("1 new dependenc(ies)"), "{}", run.out);
    assert!(
        run.says("Cargo.toml:0  unlocked 1, unpinned 0, stale 0  regex"),
        "{}",
        run.out
    );
}

#[test]
fn a_rust_lockfile_entry_that_went_fails_as_worsened() {
    let tree = rust_tree();
    tree.write("Cargo.lock", &locked(&["other"]));
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("dependenc(ies) got worse"), "{}", run.out);
    assert!(
        run.says("unlocked 1, unpinned 0, stale 0, was unlocked 0, unpinned 0, stale 0  serde"),
        "{}",
        run.out
    );
}

#[test]
fn a_rust_pin_that_became_a_range_fails_as_worsened() {
    let tree = rust_tree();
    tree.write("Cargo.toml", &manifest("serde = \"1.0\"\n"));
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("unlocked 0, unpinned 1, stale 0, was unlocked 0, unpinned 0, stale 0  serde"),
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
    let run = tree.run(&["check", "lockfile"]);
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
    let run = tree.run(&["check", "lockfile"]);
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
    let run = tree.run(&["check", "lockfile"]);
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
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("crates/a/Cargo.toml:0  unlocked 1, unpinned 0, stale 0  regex"),
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
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("NOTE: Cargo.toml has no lockfile beside it in either tree"),
        "{}",
        run.out
    );
}

fn npm_tree_at(name: &str, lockfile: &str) -> Tree {
    let tree = Tree::new();
    tree.write("package.json", r#"{"dependencies": {"left-pad": "1.0.0"}}"#);
    tree.write(name, lockfile);
    tree.base();
    tree
}

fn npm_tree(lockfile: &str) -> Tree {
    npm_tree_at("package-lock.json", lockfile)
}

const NPM_V3: &str = r#"{"lockfileVersion": 3,
    "packages": {"": {"name": "t"}, "node_modules/left-pad": {"version": "1.0.0"}}}"#;
const NPM_V1: &str = r#"{"lockfileVersion": 1,
    "dependencies": {"left-pad": {"version": "1.0.0"}}}"#;

const PNPM_V5: &str = "lockfileVersion: 5.4\n\
packages:\n  /left-pad/1.0.0:\n    resolution: {integrity: sha512-test}\n";
const PNPM_V6: &str = "lockfileVersion: '6.0'\n\
packages:\n  /left-pad@1.0.0:\n    resolution: {integrity: sha512-test}\n";
const PNPM_V9: &str = "lockfileVersion: '9.0'\n\
importers:\n  .:\n    dependencies:\n      left-pad:\n        specifier: 1.0.0\n        version: 1.0.0\npackages:\n  left-pad@1.0.0:\n    resolution: {integrity: sha512-test}\n\
snapshots:\n  left-pad@1.0.0: {}\n";
const YARN_V1: &str = "# THIS IS AN AUTOGENERATED FILE. DO NOT EDIT THIS FILE DIRECTLY.\n\
# yarn lockfile v1\n\nleft-pad@1.0.0:\n  version \"1.0.0\"\n";
const YARN_V2: &str = r#"__metadata:
  version: 6
  cacheKey: 8

"left-pad@npm:1.0.0":
  version: 1.0.0
  resolution: "left-pad@npm:1.0.0"
  languageName: node
  linkType: hard
"#;

#[test]
fn both_npm_lockfile_versions_hold_a_dependency_in_the_base_state() {
    for lockfile in [NPM_V3, NPM_V1] {
        let tree = npm_tree(lockfile);
        let run = tree.run(&["check", "lockfile"]);
        assert_eq!(run.code, 0, "{}", run.out);
    }
}

#[test]
fn pnpm_lockfile_key_styles_hold_and_missing_dependencies_fail() {
    for lockfile in [PNPM_V5, PNPM_V6, PNPM_V9] {
        let tree = npm_tree_at("pnpm-lock.yaml", lockfile);
        let held = tree.run(&["check", "lockfile"]);
        assert_eq!(held.code, 0, "{}", held.out);
        assert!(!held.says("cannot read"), "{}", held.out);

        tree.write(
            "package.json",
            r#"{"dependencies": {"left-pad": "1.0.0", "right-pad": "2.0.0"}}"#,
        );
        let run = tree.run(&["check", "lockfile"]);
        assert_eq!(run.code, 1, "{}", run.out);
        assert!(
            run.says("package.json:0  unlocked 1, unpinned 0, stale 0  right-pad"),
            "{}",
            run.out
        );
    }
}

#[test]
fn both_yarn_lockfile_formats_hold_and_missing_dependencies_fail() {
    for lockfile in [YARN_V1, YARN_V2] {
        let tree = npm_tree_at("yarn.lock", lockfile);
        let held = tree.run(&["check", "lockfile"]);
        assert_eq!(held.code, 0, "{}", held.out);
        assert!(!held.says("cannot read"), "{}", held.out);

        tree.write(
            "package.json",
            r#"{"dependencies": {"left-pad": "1.0.0", "right-pad": "2.0.0"}}"#,
        );
        let run = tree.run(&["check", "lockfile"]);
        assert_eq!(run.code, 1, "{}", run.out);
        assert!(
            run.says("package.json:0  unlocked 1, unpinned 0, stale 0  right-pad"),
            "{}",
            run.out
        );
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
        let run = tree.run(&["check", "lockfile"]);
        assert_eq!(run.code, 1, "{}", run.out);
        assert!(run.says("1 new dependenc(ies)"), "{}", run.out);
        assert!(
            run.says("package.json:0  unlocked 1, unpinned 0, stale 0  right-pad"),
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
    let gone = tree.run(&["check", "lockfile"]);
    assert_eq!(gone.code, 1, "{}", gone.out);
    assert!(
        gone.says("unlocked 1, unpinned 0, stale 0, was unlocked 0, unpinned 0, stale 0  left-pad"),
        "{}",
        gone.out
    );

    let tree = npm_tree(NPM_V3);
    tree.write(
        "package.json",
        r#"{"dependencies": {"left-pad": "^1.0.0"}}"#,
    );
    let range = tree.run(&["check", "lockfile"]);
    assert_eq!(range.code, 1, "{}", range.out);
    assert!(
        range
            .says("unlocked 0, unpinned 1, stale 0, was unlocked 0, unpinned 0, stale 0  left-pad"),
        "{}",
        range.out
    );
}

#[test]
fn an_unrecognized_lockfile_format_is_a_note_and_judges_no_manifest() {
    let tree = Tree::new();
    tree.write("package.json", r#"{"dependencies": {"left-pad": "1.0.0"}}"#);
    tree.write("yarn.lock", "left-pad@1.0.0:\n  version \"1.0.0\"\n");
    tree.base();
    let run = tree.run(&["check", "lockfile", "--json"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("yarn.lock is a lockfile format klin cannot read yet"),
        "{}",
        run.out
    );
    assert!(run.says("\"findings\":[]"), "{}", run.out);
}

#[test]
fn a_malformed_lockfile_the_base_held_too_is_a_coverage_note_naming_the_file() {
    let tree = Tree::new();
    tree.write("package.json", r#"{"dependencies": {"left-pad": "1.0.0"}}"#);
    tree.write("package-lock.json", "{ not json");
    tree.base();
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("NOTE: package-lock.json is not measured (unreadable)")
            && run.says("package-lock.json is not valid JSON"),
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
fn a_derived_manifest_klin_cannot_parse_is_a_coverage_note_and_every_other_manifest_is_judged() {
    let tree = derived_tree();
    tree.write("testdata/broken/package.json", "{ not json");
    tree.base();
    tree.write(
        "Cargo.toml",
        &manifest("serde = \"=1.0.0\"\nregex = \"=1.0.0\"\n"),
    );
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("NOTE: testdata/broken/package.json is not measured (unreadable)"),
        "{}",
        run.out
    );
    assert!(
        run.says("Cargo.toml:0  unlocked 1, unpinned 0, stale 0  regex"),
        "{}",
        run.out
    );
}

#[test]
fn a_derived_manifest_that_parsed_at_the_base_and_does_not_parse_now_is_a_measurement_lost_fail() {
    let tree = derived_tree();
    tree.write("tools/package.json", r#"{"dependencies": {}}"#);
    tree.base();
    tree.write("tools/package.json", "{ not json");
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("FAIL  measurement-lost") && run.says("tools/package.json is not valid JSON"),
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
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("tools/package.json:0  unlocked 1, unpinned 0, stale 0  left-pad"),
        "{}",
        run.out
    );
}

#[test]
fn a_derived_manifest_the_change_adds_and_klin_cannot_parse_is_a_review_item_and_passes_the_hook() {
    let tree = derived_tree();
    tree.base();
    tree.write("testdata/broken/package.json", "{ not json");
    let run = tree.run(&["check", "lockfile"]);
    let stop = harness::feed(
        tree.root(),
        harness::AGENT,
        r#"{"hook_event_name": "Stop", "stop_hook_active": false}"#,
    );
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("REVIEW: testdata/broken/package.json is not measured (unreadable)")
            && run.says("testdata/broken/package.json is not valid JSON"),
        "{}",
        run.out
    );
    assert_eq!(stop.code, 0, "{}", stop.out);
}

#[test]
fn a_derived_manifest_klin_cannot_parse_that_the_change_only_renamed_is_a_coverage_note() {
    let tree = derived_tree();
    tree.write("tools/a/package.json", "{ not json");
    tree.base();
    assert!(std::fs::create_dir_all(tree.path("tools/b")).is_ok());
    tree.git(&["mv", "tools/a/package.json", "tools/b/package.json"]);
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("NOTE: tools/b/package.json is not measured (unreadable)"),
        "{}",
        run.out
    );
}

#[test]
fn a_renamed_manifest_is_judged_against_the_lockfile_beside_it_at_the_base() {
    let tree = derived_tree();
    tree.write(
        "tools/a/package.json",
        r#"{"dependencies": {"left-pad": "1.0.0"}}"#,
    );
    tree.write("tools/a/package-lock.json", NPM_V3);
    tree.base();
    assert!(std::fs::create_dir_all(tree.path("tools/b")).is_ok());
    tree.git(&["mv", "tools/a/package.json", "tools/b/package.json"]);
    tree.write(
        "tools/b/package-lock.json",
        r#"{"lockfileVersion": 3, "packages": {"": {"name": "t"}}}"#,
    );
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("1 dependenc(ies) got worse")
            && run.says(
                "unlocked 1, unpinned 0, stale 0, was unlocked 0, unpinned 0, stale 0  left-pad"
            ),
        "{}",
        run.out
    );
}

#[test]
fn a_lockfile_only_the_base_could_not_read_is_named_at_the_base() {
    let tree = derived_tree();
    tree.write(
        "tools/a/package.json",
        r#"{"dependencies": {"left-pad": "1.0.0"}}"#,
    );
    tree.write(
        "tools/a/yarn.lock",
        "left-pad@1.0.0:\n  version \"1.0.0\"\n",
    );
    tree.base();
    assert!(std::fs::create_dir_all(tree.path("tools/b")).is_ok());
    tree.git(&["mv", "tools/a/package.json", "tools/b/package.json"]);
    tree.git(&["mv", "tools/a/yarn.lock", "tools/b/yarn.lock"]);
    tree.write(
        "tools/b/yarn.lock",
        "# yarn lockfile v1\n\nleft-pad@1.0.0:\n  version \"1.0.0\"\n",
    );
    let run = tree.run(&["check", "lockfile"]);
    let json = tree.run(&["check", "lockfile", "--json"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says(
            "tools/a/yarn.lock at the base is a lockfile format klin cannot read yet, so the \
             dependencies of tools/b/package.json are not judged"
        ),
        "{}",
        run.out
    );
    assert!(
        json.says("\"file\":\"tools/b/package.json\""),
        "{}",
        json.out
    );
}

#[test]
fn a_manifest_renamed_to_another_format_has_no_base_to_hide_behind() {
    let tree = derived_tree();
    tree.base();
    tree.git(&["mv", "Cargo.toml", "package.json"]);
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("REVIEW: package.json is not measured (unreadable)")
            && run.says("package.json is not valid JSON"),
        "{}",
        run.out
    );
}

#[test]
fn a_manifest_klin_could_never_parse_is_a_coverage_note_and_no_tool_error() {
    let tree = Tree::new();
    tree.write("package.json", "{ not json");
    tree.write("package-lock.json", NPM_V3);
    tree.base();
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("NOTE: package.json is not measured (unreadable)"),
        "{}",
        run.out
    );
}

#[test]
fn an_in_that_selects_no_manifest_is_a_tool_error() {
    let tree = rust_tree();
    tree.write("klin.json", r#"{"lockfile": {"in": "web"}}"#);
    let run = tree.run(&["check", "lockfile"]);
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
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says(
            "crates/b/Cargo.toml:0  unlocked 1, unpinned 0, stale 0, was unlocked 0, unpinned 0, stale 0  regex"
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
    let held = tree.run(&["check", "lockfile"]);
    assert_eq!(held.code, 0, "{}", held.out);

    tree.write(
        "go.mod",
        "module t\n\ngo 1.22\n\nrequire (\n\texample.com/a v1.0.0\n\texample.com/b v2.0.0\n)\n",
    );
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("go.mod:0  unlocked 1, unpinned 0, stale 0  example.com/b"),
        "{}",
        run.out
    );
}

#[test]
fn a_go_sum_entry_that_went_fails_as_worsened() {
    let tree = go_tree();
    tree.write("go.sum", "example.com/other v1.0.0 h1:abc=\n");
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says(
            "unlocked 1, unpinned 0, stale 0, was unlocked 0, unpinned 0, stale 0  example.com/a"
        ),
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
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn under_changed_a_changed_lockfile_with_an_unchanged_manifest_is_still_judged() {
    let tree = rust_tree();
    tree.write("Cargo.lock", &locked(&["other"]));
    let run = tree.run(&["check", "lockfile", "--changed"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("unlocked 1, unpinned 0, stale 0, was unlocked 0, unpinned 0, stale 0  serde"),
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
    let run = tree.run(&["check", "lockfile"]);
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
    let run = tree.run(&["check", "lockfile"]);
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
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("derived: lockfile manifests Cargo.toml"),
        "{}",
        run.out
    );
    assert!(
        run.says("Cargo.toml:0  unlocked 1, unpinned 0, stale 0  regex"),
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
    let green = tree.run(&["check", "lockfile"]);
    assert_eq!(green.code, 0, "{}", green.out);

    tree.write(
        "Cargo.toml",
        &format!("{held}\n[dependencies.regex]\nversion = \"=1.0.0\"\n"),
    );
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("Cargo.toml:0  unlocked 1, unpinned 0, stale 0  regex"),
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
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn a_dotted_path_key_is_not_judged_for_unlocked_either() {
    let tree = rust_tree();
    tree.write(
        "Cargo.toml",
        &manifest("serde = \"=1.0.0\"\nnear.path = \"../near\"\n"),
    );
    let run = tree.run(&["check", "lockfile"]);
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
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn a_brace_inside_a_comment_hides_no_dependency_below_it() {
    let tree = rust_tree();
    tree.write(
        "Cargo.toml",
        &manifest("# serde_json { is slower\nserde = \"1.0\"\n"),
    );
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("unlocked 0, unpinned 1, stale 0, was unlocked 0, unpinned 0, stale 0  serde"),
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
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 0, "{}", run.out);
}

fn npm_lock(version: &str) -> String {
    format!(
        r#"{{"lockfileVersion": 3, "packages": {{"": {{"name": "t"}},
            "node_modules/typescript": {{"version": "{version}"}}}}}}"#
    )
}

fn typescript_tree(version: &str) -> Tree {
    let tree = Tree::new();
    tree.write(
        "package.json",
        &format!(r#"{{"devDependencies": {{"typescript": "{version}"}}}}"#),
    );
    tree.write("package-lock.json", &npm_lock(version));
    tree.base();
    tree
}

#[test]
fn a_manifest_pin_the_lockfile_records_at_another_version_fails_as_worsened() {
    let tree = typescript_tree("5.4.0");
    tree.write(
        "package.json",
        r#"{"devDependencies": {"typescript": "5.6.3"}}"#,
    );
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says(
            "unlocked 0, unpinned 0, stale 1, was unlocked 0, unpinned 0, stale 0  typescript"
        ),
        "{}",
        run.out
    );
    assert!(
        run.says("Install again, so the lockfile records the pinned version."),
        "{}",
        run.out
    );
    assert!(!run.says("Run the project's own install"), "{}", run.out);
}

#[test]
fn a_new_pin_the_lockfile_records_at_another_version_fails_as_new() {
    let tree = Tree::new();
    tree.write("package.json", r#"{"dependencies": {}}"#);
    tree.write("package-lock.json", &npm_lock("5.4.0"));
    tree.base();
    tree.write(
        "package.json",
        r#"{"devDependencies": {"typescript": "5.6.3"}}"#,
    );
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("package.json:0  unlocked 0, unpinned 0, stale 1  typescript"),
        "{}",
        run.out
    );
}

#[test]
fn npm_judges_the_version_under_a_package_root_and_not_a_nested_one() {
    let tree = typescript_tree("5.4.0");
    tree.write(
        "package-lock.json",
        r#"{"lockfileVersion": 3, "packages": {"": {"name": "t"},
            "node_modules/typescript": {"version": "5.4.0"},
            "node_modules/tool/node_modules/typescript": {"version": "5.6.3"}}}"#,
    );
    tree.write(
        "package.json",
        r#"{"devDependencies": {"typescript": "5.6.3"}}"#,
    );
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("unlocked 0, unpinned 0, stale 1, was unlocked 0, unpinned 0, stale 0"),
        "{}",
        run.out
    );

    for lockfile in [
        npm_lock("5.6.3"),
        r#"{"lockfileVersion": 1, "dependencies": {"typescript": {"version": "5.6.3"}}}"#
            .to_string(),
    ] {
        tree.write("package-lock.json", &lockfile);
        let run = tree.run(&["check", "lockfile"]);
        assert_eq!(run.code, 0, "{}", run.out);
    }
}

#[test]
fn every_lockfile_reader_fails_a_pin_it_records_at_another_version() {
    let pnpm = "lockfileVersion: '9.0'\npackages:\n  left-pad@2.0.0:\n    resolution: {integrity: sha512-test}\n";
    let classic = "# yarn lockfile v1\n\nleft-pad@2.0.0:\n  version \"2.0.0\"\n";
    let berry = "__metadata:\n  version: 6\n\n\"left-pad@npm:2.0.0\":\n  version: 2.0.0\n";
    for (name, lockfile) in [
        ("pnpm-lock.yaml", pnpm),
        ("yarn.lock", classic),
        ("yarn.lock", berry),
    ] {
        let tree = Tree::new();
        tree.write("package.json", r#"{"dependencies": {"left-pad": "1.0.0"}}"#);
        tree.write(name, lockfile);
        let run = tree.run(&["check", "lockfile"]);
        assert_eq!(run.code, 1, "{name}: {}", run.out);
        assert!(
            run.says("package.json:0  unlocked 0, unpinned 0, stale 1  left-pad"),
            "{name}: {}",
            run.out
        );
    }

    let tree = Tree::new();
    tree.write("Cargo.toml", &manifest("serde = \"=2.0.0\"\n"));
    tree.write("Cargo.lock", &locked(&["serde"]));
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("Cargo.toml:0  unlocked 0, unpinned 0, stale 1  serde"),
        "{}",
        run.out
    );

    let tree = go_tree();
    tree.write(
        "go.mod",
        "module t\n\ngo 1.22\n\nrequire (\n\texample.com/a v1.1.0\n)\n",
    );
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says(
            "unlocked 0, unpinned 0, stale 1, was unlocked 0, unpinned 0, stale 0  example.com/a"
        ),
        "{}",
        run.out
    );
    tree.write(
        "go.sum",
        "example.com/a v1.0.0 h1:abc=\nexample.com/a v1.1.0/go.mod h1:def=\n",
    );
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn a_range_with_a_lockfile_entry_at_any_version_passes() {
    let tree = Tree::new();
    tree.write(
        "package.json",
        r#"{"devDependencies": {"typescript": "^5.0.0"}}"#,
    );
    tree.write("package-lock.json", &npm_lock("5.4.0"));
    tree.base();
    tree.write(
        "package.json",
        r#"{"devDependencies": {"typescript": "^5.0.0", "left-pad": "~1.0.0"}}"#,
    );
    tree.write(
        "package-lock.json",
        r#"{"lockfileVersion": 3, "packages": {"": {"name": "t"},
            "node_modules/typescript": {"version": "4.0.0"},
            "node_modules/left-pad": {"version": "9.9.9"}}}"#,
    );
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn staleness_the_base_already_had_is_held() {
    let tree = Tree::new();
    tree.write(
        "package.json",
        r#"{"devDependencies": {"typescript": "5.6.3"}}"#,
    );
    tree.write("package-lock.json", &npm_lock("5.4.0"));
    tree.base();
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn a_lockfile_with_several_versions_of_one_name_is_stale_only_when_none_equals_the_pin() {
    let tree = Tree::new();
    let lock = "[[package]]\nname = \"serde\"\nversion = \"1.0.0\"\n\n\
                [[package]]\nname = \"serde\"\nversion = \"2.0.0\"\n";
    tree.write("Cargo.toml", &manifest("serde = \"=2.0.0\"\n"));
    tree.write("Cargo.lock", lock);
    tree.base();
    let held = tree.run(&["check", "lockfile"]);
    assert_eq!(held.code, 0, "{}", held.out);

    tree.write("Cargo.toml", &manifest("serde = \"=3.0.0\"\n"));
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("unlocked 0, unpinned 0, stale 1, was unlocked 0, unpinned 0, stale 0"),
        "{}",
        run.out
    );
}

#[test]
fn a_new_dependency_missing_from_the_lockfile_gets_a_remedy_that_names_the_install() {
    let tree = rust_tree();
    tree.write(
        "Cargo.toml",
        &manifest("serde = \"=1.0.0\"\nregex = \"=1.0.0\"\n"),
    );
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("Run the project's own install, so the lockfile records the dependency."),
        "{}",
        run.out
    );
    assert!(
        run.says("If you cannot run the install, report why"),
        "{}",
        run.out
    );
    assert!(
        run.says("Do not write lockfile entries by hand"),
        "{}",
        run.out
    );
    assert!(!run.says("Restore the exact version"), "{}", run.out);
    assert!(!run.says("Install again"), "{}", run.out);
}

#[test]
fn a_finding_with_several_values_prints_the_remedy_for_each() {
    let tree = rust_tree();
    tree.write("Cargo.toml", &manifest("serde = \"1.0\"\n"));
    tree.write("Cargo.lock", &locked(&["other"]));
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("unlocked 1, unpinned 1, stale 0, was unlocked 0, unpinned 0, stale 0  serde"),
        "{}",
        run.out
    );
    assert!(
        run.says(
            "Run the project's own install, so the lockfile records the dependency. A \
             dependency the lockfile does not know is one no install has ever resolved. If you cannot \
             run the install, report why. Do not write lockfile entries by hand. Restore \
             the exact version the base pinned, or pin an exact version for a new dependency."
        ),
        "{}",
        run.out
    );
    assert!(!run.says("Install again"), "{}", run.out);
}

#[test]
fn a_pnpm_5_peer_suffix_is_no_part_of_the_name_or_the_version() {
    let pnpm = "lockfileVersion: 5.4\npackages:\n  /react-dom/18.2.0_react@18.2.0:\n    resolution: {integrity: sha512-test}\n  /left-pad/1.0.0_4ylqtpvmpcfm7fo6rfbbf6yr4a:\n    resolution: {integrity: sha512-test}\n";
    let tree = Tree::new();
    tree.write(
        "package.json",
        r#"{"dependencies": {"react-dom": "18.2.0", "left-pad": "1.0.0"}}"#,
    );
    tree.write("pnpm-lock.yaml", pnpm);
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn a_go_module_a_replace_sends_to_another_version_is_judged_at_that_version() {
    let tree = go_tree();
    tree.write(
        "go.mod",
        "module t\n\ngo 1.22\n\nrequire (\n\texample.com/a v1.0.0\n)\n\
         \nreplace example.com/a => example.com/a v1.2.0\n",
    );
    tree.write("go.sum", "example.com/a v1.2.0 h1:abc=\n");
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn an_exact_pin_beside_a_range_of_the_same_dependency_is_still_judged_for_stale() {
    let tree = Tree::new();
    tree.write(
        "Cargo.toml",
        &format!(
            "{}\n[target.'cfg(unix)'.dependencies]\nserde = \"=2.0.0\"\n",
            manifest("serde = \"1\"\n")
        ),
    );
    tree.write("Cargo.lock", &locked(&["serde"]));
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("Cargo.toml:0  unlocked 0, unpinned 1, stale 1  serde"),
        "{}",
        run.out
    );
}

#[test]
fn an_npm_pin_with_only_a_nested_lockfile_entry_is_stale() {
    let tree = Tree::new();
    tree.write(
        "package.json",
        r#"{"devDependencies": {"typescript": "5.6.3"}}"#,
    );
    tree.write(
        "package-lock.json",
        r#"{"lockfileVersion": 3, "packages": {"": {"name": "t"},
            "node_modules/tool": {"version": "1.0.0"},
            "node_modules/tool/node_modules/typescript": {"version": "5.6.3"}}}"#,
    );
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("package.json:0  unlocked 0, unpinned 0, stale 1  typescript"),
        "{}",
        run.out
    );
}

#[test]
fn an_npm_workspace_link_is_judged_at_the_version_of_the_package_it_links() {
    let tree = Tree::new();
    tree.write("package.json", r#"{"dependencies": {"shared": "1.0.0"}}"#);
    let lockfile = |version: &str| {
        format!(
            r#"{{"lockfileVersion": 3, "packages": {{"": {{"name": "t"}},
                "node_modules/shared": {{"resolved": "packages/shared", "link": true}},
                "packages/shared": {{"version": "{version}"}}}}}}"#
        )
    };
    tree.write("package-lock.json", &lockfile("1.0.0"));
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 0, "{}", run.out);

    tree.write("package-lock.json", &lockfile("2.0.0"));
    let run = tree.run(&["check", "lockfile"]);
    assert!(
        run.says("package.json:0  unlocked 0, unpinned 0, stale 1  shared"),
        "{}",
        run.out
    );
}

#[test]
fn each_npm_workspace_manifest_is_judged_against_its_own_entry_and_then_the_root() {
    let tree = Tree::new();
    tree.write(
        "package.json",
        r#"{"devDependencies": {"typescript": "5.6.3"}}"#,
    );
    tree.write(
        "packages/a/package.json",
        r#"{"devDependencies": {"typescript": "5.6.3", "left-pad": "1.0.0"}}"#,
    );
    tree.write(
        "package-lock.json",
        r#"{"lockfileVersion": 3, "packages": {"": {"name": "t"},
            "node_modules/typescript": {"version": "5.4.0"},
            "node_modules/left-pad": {"version": "1.0.0"},
            "packages/a/node_modules/typescript": {"version": "5.6.3"}}}"#,
    );
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("  package.json:0  unlocked 0, unpinned 0, stale 1  typescript"),
        "{}",
        run.out
    );
    assert!(!run.says("packages/a/package.json:0"), "{}", run.out);
}

#[test]
fn a_cargo_lock_block_is_read_whatever_the_order_of_its_fields() {
    let tree = Tree::new();
    tree.write("Cargo.toml", &manifest("serde = \"=2.0.0\"\n"));
    tree.write(
        "Cargo.lock",
        "[[package]]\nversion = \"1.0.0\"\nname = \"serde\"\n",
    );
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("Cargo.toml:0  unlocked 0, unpinned 0, stale 1  serde"),
        "{}",
        run.out
    );
}

#[test]
fn only_a_pin_with_fewer_than_three_numbers_holds_the_versions_it_is_a_prefix_of() {
    let tree = Tree::new();
    tree.write("Cargo.toml", &manifest("serde = \"=1.2\"\n"));
    tree.write(
        "Cargo.lock",
        "[[package]]\nname = \"serde\"\nversion = \"1.2.5\"\n",
    );
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 0, "{}", run.out);

    let tree = Tree::new();
    tree.write(
        "package.json",
        r#"{"dependencies": {"left-pad": "1.0.0-alpha"}}"#,
    );
    tree.write(
        "package-lock.json",
        r#"{"lockfileVersion": 3, "packages": {"": {"name": "t"},
            "node_modules/left-pad": {"version": "1.0.0-alpha.1"}}}"#,
    );
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("package.json:0  unlocked 0, unpinned 0, stale 1  left-pad"),
        "{}",
        run.out
    );
}

#[test]
fn a_go_replace_applies_only_to_the_version_it_names_and_judges_its_target() {
    let tree = go_tree();
    tree.write(
        "go.mod",
        "module t\n\ngo 1.22\n\nrequire (\n\texample.com/a v1.1.0\n)\n\
         \nreplace example.com/a v1.0.0 => example.com/a v1.2.0\n",
    );
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says(
            "unlocked 0, unpinned 0, stale 1, was unlocked 0, unpinned 0, stale 0  example.com/a"
        ),
        "{}",
        run.out
    );

    tree.write(
        "go.mod",
        "module t\n\ngo 1.22\n\nrequire (\n\texample.com/a v1.0.0\n)\n\
         \nreplace example.com/a => example.com/fork v1.2.0\n",
    );
    tree.write("go.sum", "example.com/fork v1.2.0 h1:abc=\n");
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn a_pnpm_manifest_is_judged_against_its_own_importer_and_not_the_package_pool() {
    let lockfile = |root: &str| {
        format!(
            "lockfileVersion: '9.0'\n\nimporters:\n\n  .:\n    dependencies:\n      foo:\n\
             \x20       specifier: 2.0.0\n        version: {root}\n\n  packages/a:\n\
             \x20   devDependencies:\n      foo:\n        specifier: 2.0.0\n\
             \x20       version: 2.0.0(react@18.2.0)\n\npackages:\n\n  foo@1.0.0:\n\
             \x20   resolution: {{integrity: sha512-a}}\n\n  foo@2.0.0:\n\
             \x20   resolution: {{integrity: sha512-b}}\n"
        )
    };
    let tree = Tree::new();
    tree.write("package.json", r#"{"dependencies": {"foo": "2.0.0"}}"#);
    tree.write(
        "packages/a/package.json",
        r#"{"devDependencies": {"foo": "2.0.0"}}"#,
    );
    tree.write("pnpm-lock.yaml", &lockfile("2.0.0"));
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 0, "{}", run.out);

    tree.write("pnpm-lock.yaml", &lockfile("1.0.0"));
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("  package.json:0  unlocked 0, unpinned 0, stale 1  foo"),
        "{}",
        run.out
    );
    assert!(!run.says("packages/a/package.json:0"), "{}", run.out);
}

#[test]
fn a_pnpm_5_root_dependency_section_is_its_importer() {
    let tree = Tree::new();
    tree.write("package.json", r#"{"dependencies": {"foo": "2.0.0"}}"#);
    tree.write(
        "pnpm-lock.yaml",
        "lockfileVersion: 5.4\n\nspecifiers:\n  foo: 2.0.0\n\ndependencies:\n  foo: 1.0.0\n\n\
         packages:\n\n  /foo/1.0.0:\n    resolution: {integrity: sha512-a}\n\n\
         \x20 /foo/2.0.0:\n    resolution: {integrity: sha512-b}\n",
    );
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("package.json:0  unlocked 0, unpinned 0, stale 1  foo"),
        "{}",
        run.out
    );
}

#[test]
fn a_worsened_finding_prints_only_the_remedy_of_the_value_that_rose() {
    let tree = Tree::new();
    tree.write("Cargo.toml", &manifest("serde = \"1\"\n"));
    tree.write("Cargo.lock", &locked(&["serde"]));
    tree.base();
    tree.write("Cargo.lock", &locked(&["other"]));
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("unlocked 1, unpinned 1, stale 0, was unlocked 0, unpinned 1, stale 0  serde"),
        "{}",
        run.out
    );
    assert!(run.says("Run the project's own install"), "{}", run.out);
    assert!(!run.says("Restore the exact version"), "{}", run.out);
}

#[test]
fn a_stale_pin_fails_under_a_condition_that_names_it() {
    let tree = Tree::new();
    tree.write("package.json", r#"{"dependencies": {}}"#);
    tree.write("package-lock.json", &npm_lock("5.4.0"));
    tree.base();
    tree.write(
        "package.json",
        r#"{"devDependencies": {"typescript": "5.6.3"}}"#,
    );
    let run = tree.run(&["check", "lockfile"]);
    assert!(
        run.says(
            "1 new dependenc(ies) that the lockfile beside the manifest does not lock at an exact pin"
        ),
        "{}",
        run.out
    );
}

#[test]
fn an_accepted_entry_that_gives_no_stale_holds_no_staleness() {
    let tree = typescript_tree("5.4.0");
    tree.write(
        "package.json",
        r#"{"devDependencies": {"typescript": "5.6.3"}}"#,
    );
    tree.write(
        "klin.json",
        r#"{"accepted": [{"gate": "lockfile", "file": "package.json", "text": "typescript",
                          "unlocked": 0, "unpinned": 0}]}"#,
    );
    let run = tree.run(&["check", "lockfile"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("stale 1, was unlocked 0, unpinned 0, stale 0  typescript"),
        "{}",
        run.out
    );
}
