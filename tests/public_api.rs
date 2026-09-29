mod harness;

use harness::{Run, Tree};

const A_STOP: &str = r#"{"hook_event_name": "Stop", "stop_hook_active": false}"#;
const A_SECOND_STOP: &str = r#"{"hook_event_name": "Stop", "stop_hook_active": true}"#;
const PACKAGE: &str = "[package]\nname = \"core\"\nversion = \"0.1.0\"\nedition = \"2024\"\n";
const CLIENT: &str = "pub struct Client {\n    pub name: String,\n    secret: u8,\n}\n\nimpl Client {\n    pub fn new(name: &str) -> Client {\n        Client { name: name.to_string(), secret: 0 }\n    }\n\n    fn hidden(&self) -> u8 {\n        self.secret\n    }\n}\n";
const LIB: &str = "mod client;\npub mod model;\nmod hidden;\npub use client::Client;\npub(crate) fn internal() -> u8 {\n    1\n}\npub fn parse(input: &str) -> u8 {\n    input.len() as u8\n}\n";

/// A library crate: a root item, a public module, a private module whose item a `pub use`
/// exposes, a private module nothing exposes, and a restricted item. Committed as the base.
fn library(tree: &Tree) {
    tree.write("klin.json", "{}");
    tree.write("Cargo.toml", PACKAGE);
    tree.write("src/lib.rs", LIB);
    tree.write("src/client.rs", CLIENT);
    tree.write(
        "src/model.rs",
        "pub struct Document {\n    pub title: String,\n}\n",
    );
    tree.write("src/hidden.rs", "pub fn not_external() {}\n");
    tree.base();
}

fn by_hand(tree: &Tree) -> Run {
    tree.run(&["public-api"])
}

fn report(tree: &Tree) -> Run {
    tree.run(&["public-api", "--report"])
}

fn changed(tree: &Tree) -> Run {
    tree.run(&["gate", "--changed", "--gate", "public-api"])
}

fn hook(tree: &Tree) -> Run {
    harness::feed(tree.root(), &["gate", "--hook", "--changed"], A_STOP)
}

#[test]
fn an_unchanged_library_passes_and_the_ok_line_says_what_was_judged() {
    let tree = Tree::new();
    library(&tree);

    let run = by_hand(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says(
            "OK: 5 external item(s) on 1 surface(s) judged against the base, 4 measured, 1 opaque"
        ),
        "{}",
        run.out
    );
    assert!(run.says("1 Rust library target(s)"), "{}", run.out);
}

#[test]
fn a_root_pub_item_a_pub_mod_chain_and_a_pub_use_are_external_and_the_rest_is_not() {
    let tree = Tree::new();
    library(&tree);

    let run = report(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
    for line in [
        "core::parse  function  measured  src/lib.rs:8",
        "fn parse(_: &str) -> u8;",
        "core::model  module  opaque",
        "core::model::Document  type  measured  src/model.rs:1",
        "struct Document { title: String }",
        "core::Client  type  measured  src/client.rs:1",
        "struct Client { name: String, .. }",
        "core::Client::new  method  measured  src/client.rs:7",
        "fn new(_: &str) -> Client;",
    ] {
        assert!(run.says(line), "no {line} in: {}", run.out);
    }
    for absent in ["not_external", "internal", "hidden", "core::client::Client"] {
        assert!(!run.says(absent), "{absent} in: {}", run.out);
    }
}

#[test]
fn a_binary_only_package_has_no_surface_and_is_named_as_not_applicable() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write("Cargo.toml", PACKAGE);
    tree.write("src/main.rs", "pub fn shown() {}\nfn main() {}\n");
    tree.base();

    let run = by_hand(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("0 external item(s) on 0 surface(s)") && run.says("0 Rust library target(s)"),
        "{}",
        run.out
    );
    assert!(
        run.says("NOTE: 1 package(s) or target(s) with no supported public surface")
            && run.says("Rust package core (Cargo.toml): has no library target"),
        "{}",
        run.out
    );
}

#[test]
fn a_custom_library_root_is_discovered_from_the_manifest() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write(
        "Cargo.toml",
        &format!("{PACKAGE}[lib]\nname = \"corelib\"\npath = \"src/other.rs\"\n"),
    );
    tree.write(
        "src/other.rs",
        "pub fn parse(input: &str) -> u8 {\n    1\n}\n",
    );
    tree.base();

    let run = report(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("surface corelib  Rust — library target src/other.rs of Cargo.toml")
            && run.says("corelib::parse  function  measured"),
        "{}",
        run.out
    );
}

#[test]
fn two_workspace_library_packages_are_distinct_surfaces() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/alpha\", \"crates/beta\"]\n",
    );
    for name in ["alpha", "beta"] {
        tree.write(
            &format!("crates/{name}/Cargo.toml"),
            &format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"),
        );
        tree.write(
            &format!("crates/{name}/src/lib.rs"),
            "pub fn run() -> u8 {\n    1\n}\n",
        );
    }
    tree.base();
    tree.write("crates/beta/src/lib.rs", "pub fn go() -> u8 {\n    1\n}\n");

    let run = by_hand(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("1 new compatibility break(s)"), "{}", run.out);
    assert!(
        run.says("beta:1  removed") && run.says("run (function)"),
        "{}",
        run.out
    );
    assert!(!run.says("alpha:"), "{}", run.out);
}

#[test]
fn restricted_visibility_is_not_external_and_removing_it_passes() {
    let tree = Tree::new();
    library(&tree);
    tree.write(
        "src/lib.rs",
        &LIB.replace("pub(crate) fn internal() -> u8 {\n    1\n}\n", ""),
    );
    tree.write(
        "src/hidden.rs",
        "pub(super) fn not_external() {}\npub(in crate) fn other() {}\n",
    );

    let run = by_hand(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!report(&tree).says("internal"), "{}", report(&tree).out);
}

#[test]
fn an_alias_changes_the_external_name_and_renaming_it_is_a_removal() {
    let tree = Tree::new();
    library(&tree);
    tree.write(
        "src/lib.rs",
        &LIB.replace(
            "pub use client::Client;",
            "pub use client::Client as Handle;",
        ),
    );

    let run = by_hand(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("Client (type)") && run.says("Client::new (method)") && run.says("2 new"),
        "{}",
        run.out
    );
    assert!(report(&tree).says("core::Handle  type  measured  src/client.rs:1"));
}

#[test]
fn a_glob_re_export_exposes_every_public_item_of_its_module() {
    let tree = Tree::new();
    library(&tree);
    tree.write(
        "src/lib.rs",
        &LIB.replace("pub mod model;", "mod model;\npub use model::*;"),
    );
    tree.write(
        "src/model.rs",
        "pub struct Document {\n    pub title: String,\n}\npub enum Kind {\n    A,\n    B(u8),\n}\nfn private() {}\n",
    );
    tree.base();

    let run = report(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("core::Document  type  measured") && run.says("core::Kind  type  measured"),
        "{}",
        run.out
    );
    assert!(run.says("enum Kind { A, B(u8) }"), "{}", run.out);
    assert!(
        !run.says("private") && !run.says("core::model"),
        "{}",
        run.out
    );
}

#[test]
fn a_re_export_of_another_crate_is_opaque_and_judged_on_presence() {
    let tree = Tree::new();
    library(&tree);
    tree.write("src/lib.rs", &format!("{LIB}pub use serde::Serialize;\n"));
    tree.base();
    let shown = report(&tree);
    tree.write("src/lib.rs", LIB);

    let run = by_hand(&tree);

    assert!(
        shown.says("core::Serialize  item  opaque (serde::Serialize)"),
        "{}",
        shown.out
    );
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("Serialize (item)"), "{}", run.out);
}

const MAP_MODE: &str = "pub enum MapMode {\n    Read,\n}\n\nimpl MapMode {\n    pub fn read() -> MapMode {\n        MapMode::Read\n    }\n}\n";

/// A workspace of two library crates, `a`, which depends on `types` by path, and `types`, whose
/// roots hold `a` and `types`.
fn workspace(tree: &Tree, a: &str, types: &str) {
    workspace_depending(tree, "types = { path = \"../types\" }", a, types);
}

/// The same workspace, with `a` declaring `dependency` in place of its path dependency.
fn workspace_depending(tree: &Tree, dependency: &str, a: &str, types: &str) {
    tree.write("klin.json", "{}");
    tree.write("Cargo.toml", "[workspace]\nmembers = [\"a\", \"types\"]\n");
    tree.write(
        "a/Cargo.toml",
        &format!(
            "[package]\nname = \"a\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\n{dependency}\n"
        ),
    );
    tree.write(
        "types/Cargo.toml",
        "[package]\nname = \"types\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    tree.write("a/src/lib.rs", a);
    tree.write("types/src/lib.rs", types);
}

#[test]
fn an_item_moved_into_a_workspace_sibling_and_re_exported_under_its_name_passes() {
    let tree = Tree::new();
    workspace(&tree, MAP_MODE, "");
    tree.base();
    workspace(&tree, "pub use types::MapMode;\n", MAP_MODE);

    let run = by_hand(&tree);
    let scoped = changed(&tree);
    let shown = report(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(scoped.code, 0, "{}", scoped.out);
    assert!(
        shown.says("a::MapMode  type  measured  types/src/lib.rs:1")
            && shown.says("a::MapMode::read  method  measured  types/src/lib.rs:6"),
        "{}",
        shown.out
    );
}

#[test]
fn an_item_moved_into_a_workspace_sibling_with_a_changed_contract_fails_as_changed() {
    let tree = Tree::new();
    workspace(&tree, MAP_MODE, "");
    tree.base();
    workspace(
        &tree,
        "pub use types::MapMode;\n",
        &MAP_MODE.replace("Read,", "Read,\n    Write,"),
    );

    let run = by_hand(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("1 new compatibility break(s)"), "{}", run.out);
    assert!(
        run.says("a:1  changed, declared at types/src/lib.rs:1, was `enum MapMode { Read }`, now `enum MapMode { Read, Write }`  MapMode (type)"),
        "{}",
        run.out
    );
}

#[test]
fn a_re_export_through_a_pub_extern_crate_alias_of_a_sibling_is_judged_the_same_way() {
    let tree = Tree::new();
    workspace(&tree, MAP_MODE, "");
    tree.base();
    let aliased = "pub extern crate types as wgt;\npub use wgt::MapMode;\n";
    workspace(&tree, aliased, MAP_MODE);
    let green = by_hand(&tree);
    let shown = report(&tree);
    workspace(
        &tree,
        aliased,
        &MAP_MODE.replace("Read,", "Read,\n    Write,"),
    );

    let red = by_hand(&tree);

    assert_eq!(green.code, 0, "{}", green.out);
    assert!(
        shown.says("a::MapMode  type  measured  types/src/lib.rs:1")
            && shown.says("a::wgt  item  opaque (types)")
            && !shown.says("a::wgt::"),
        "{}",
        shown.out
    );
    assert_eq!(red.code, 1, "{}", red.out);
    assert!(red.says("1 new compatibility break(s)"), "{}", red.out);
    assert!(
        red.says("was `enum MapMode { Read }`, now `enum MapMode { Read, Write }`  MapMode (type)"),
        "{}",
        red.out
    );
}

#[test]
fn a_name_a_sibling_provides_through_a_glob_is_measured_where_the_glob_reaches() {
    let tree = Tree::new();
    workspace(&tree, MAP_MODE, "");
    tree.base();
    workspace(
        &tree,
        "pub extern crate types as wgt;\npub use wgt::{MapMode};\n",
        "mod buffer;\npub use buffer::*;\n",
    );
    tree.write("types/src/buffer.rs", MAP_MODE);

    let run = by_hand(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        report(&tree).says("a::MapMode  type  measured  types/src/buffer.rs:1"),
        "{}",
        report(&tree).out
    );
}

#[test]
fn a_re_export_after_a_leading_path_separator_reaches_an_extern_prelude_name_and_no_use_alias() {
    let tree = Tree::new();
    workspace(&tree, MAP_MODE, "");
    tree.base();
    workspace(
        &tree,
        "extern crate types as t;\npub use ::types as wgt;\npub use ::types::MapMode;\npub use ::t::MapMode as Other;\npub use ::wgt::MapMode as Mode;\n",
        MAP_MODE,
    );

    let run = by_hand(&tree);
    let shown = report(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        shown.says("a::MapMode  type  measured  types/src/lib.rs:1")
            && shown.says("a::Other  type  measured  types/src/lib.rs:1")
            && shown.says("a::Mode  item  opaque (::wgt::MapMode)"),
        "{}",
        shown.out
    );
}

#[test]
fn a_crate_the_manifest_takes_from_a_registry_stays_opaque_though_the_tree_holds_its_name() {
    let tree = Tree::new();
    workspace_depending(
        &tree,
        "types = \"1\"",
        "pub use types::MapMode;\n",
        MAP_MODE,
    );
    tree.base();

    let run = report(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("a::MapMode  item  opaque (types::MapMode)"),
        "{}",
        run.out
    );
}

#[test]
fn a_crate_name_two_libraries_of_the_tree_share_reaches_the_one_the_manifest_names() {
    let tree = Tree::new();
    workspace_depending(
        &tree,
        "types = { path = \"../fork/types\" }",
        "pub use types::MapMode;\n",
        "pub struct MapMode;\n",
    );
    tree.write(
        "fork/types/Cargo.toml",
        "[package]\nname = \"types\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    tree.write("fork/types/src/lib.rs", MAP_MODE);
    tree.base();

    let run = report(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("a::MapMode  type  measured  fork/types/src/lib.rs:1"),
        "{}",
        run.out
    );
}

#[test]
fn a_dependency_the_manifest_renames_is_followed_under_its_new_name() {
    let tree = Tree::new();
    workspace_depending(
        &tree,
        "wgt = { package = \"types\", path = \"../types\" }",
        "pub use wgt::MapMode;\n",
        MAP_MODE,
    );
    tree.base();

    let run = report(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("a::MapMode  type  measured  types/src/lib.rs:1"),
        "{}",
        run.out
    );
}

#[test]
fn a_dependency_inherited_from_the_workspace_is_followed_from_the_workspace_path() {
    let tree = Tree::new();
    workspace_depending(
        &tree,
        "wgt = { workspace = true }",
        "pub use wgt::MapMode;\n",
        MAP_MODE,
    );
    tree.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"a\", \"types\"]\n\n[workspace.dependencies]\nwgt = { package = \"types\", path = \"types\" }\n",
    );
    tree.base();

    let run = report(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("a::MapMode  type  measured  types/src/lib.rs:1"),
        "{}",
        run.out
    );
}

#[test]
fn a_target_specific_path_dependency_is_followed() {
    let tree = Tree::new();
    workspace_depending(
        &tree,
        "\n[target.'cfg(unix)'.dependencies]\ntypes = { path = \"../types\" }",
        "pub use types::MapMode;\n",
        MAP_MODE,
    );
    tree.base();

    let run = report(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("a::MapMode  type  measured  types/src/lib.rs:1"),
        "{}",
        run.out
    );
}

const TYPES: &str = "[package]\nname = \"types\"\nversion = \"0.1.0\"\nedition = \"2024\"\n";

#[test]
fn a_dependency_inherited_from_a_workspace_below_the_tree_root_is_followed() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write(
        "sub/Cargo.toml",
        "[workspace]\nmembers = [\"a\", \"types\"]\n\n[workspace.dependencies]\ntypes = { path = \"types\" }\n",
    );
    tree.write(
        "sub/a/Cargo.toml",
        "[package]\nname = \"a\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\ntypes = { workspace = true }\n",
    );
    tree.write("sub/types/Cargo.toml", TYPES);
    tree.write("sub/a/src/lib.rs", "pub use types::MapMode;\n");
    tree.write("sub/types/src/lib.rs", MAP_MODE);
    tree.base();

    let run = report(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("a::MapMode  type  measured  sub/types/src/lib.rs:1"),
        "{}",
        run.out
    );
}

#[test]
fn a_dependency_a_package_inherits_from_its_own_workspace_is_followed() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write(
        "sub/Cargo.toml",
        "[package]\nname = \"a\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[workspace]\nmembers = [\"types\"]\n\n[workspace.dependencies]\ntypes = { path = \"types\" }\n\n[dependencies]\ntypes = { workspace = true }\n",
    );
    tree.write("sub/types/Cargo.toml", TYPES);
    tree.write("sub/src/lib.rs", "pub use types::MapMode;\n");
    tree.write("sub/types/src/lib.rs", MAP_MODE);
    tree.base();

    let run = report(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("a::MapMode  type  measured  sub/types/src/lib.rs:1"),
        "{}",
        run.out
    );
}

#[test]
fn a_private_extern_crate_alias_of_a_sibling_is_followed() {
    let tree = Tree::new();
    workspace(
        &tree,
        "extern crate types as wgt;\nextern crate wgt as again;\npub use wgt::MapMode;\npub use again::MapMode as Again;\n",
        MAP_MODE,
    );
    tree.base();

    let run = report(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("a::MapMode  type  measured  types/src/lib.rs:1")
            && run.says("a::Again  item  opaque (again::MapMode)")
            && !run.says("a::wgt"),
        "{}",
        run.out
    );
}

#[test]
fn a_name_two_globs_provide_is_a_hole_where_a_re_export_names_it() {
    let tree = Tree::new();
    crate_of(&tree, "pub struct Mode;\n");
    tree.write("src/lib.rs", "mod inner;\npub use inner::Mode;\n");
    tree.write(
        "src/inner.rs",
        "mod one;\nmod two;\npub use one::*;\npub use two::*;\n",
    );
    tree.write("src/inner/one.rs", "pub struct Mode;\n");
    tree.write("src/inner/two.rs", "pub enum Mode {\n    A,\n}\n");

    let run = by_hand(&tree);

    assert_eq!(run.code, 2, "{}", run.out);
    assert!(
        run.says("Mode is provided by this glob and by the glob at src/inner.rs:3"),
        "{}",
        run.out
    );
}

#[test]
fn a_re_export_of_a_crate_the_tree_does_not_hold_stays_opaque() {
    let tree = Tree::new();
    workspace(
        &tree,
        "pub use serde::Serialize;\npub extern crate serde_json as json;\npub use json::Value;\n",
        MAP_MODE,
    );
    tree.base();
    let shown = report(&tree);
    workspace(&tree, "pub extern crate serde_json as json;\n", MAP_MODE);

    let run = by_hand(&tree);

    for line in [
        "a::Serialize  item  opaque (serde::Serialize)",
        "a::json  item  opaque (serde_json)",
        "a::Value  item  opaque (json::Value)",
    ] {
        assert!(shown.says(line), "no {line} in: {}", shown.out);
    }
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("2 new compatibility break(s)")
            && run.says("Serialize (item)")
            && run.says("Value (item)"),
        "{}",
        run.out
    );
}

#[test]
fn a_local_use_named_like_a_dependency_keeps_its_path_opaque_though_its_item_changes() {
    let tree = Tree::new();
    let a = "mod local;\nuse crate::local as types;\npub use types::MapMode;\n";
    workspace(&tree, a, MAP_MODE);
    tree.write("a/src/local.rs", "pub struct MapMode;\n");
    tree.base();
    tree.write("a/src/local.rs", "pub struct MapMode(u8);\n");

    let run = by_hand(&tree);
    let shown = report(&tree);

    assert!(
        shown.says("a::MapMode  item  opaque (types::MapMode)") && !shown.says("a::MapMode  type"),
        "{}",
        shown.out
    );
    assert!(!run.says("declared at types/"), "{}", run.out);
}

#[test]
fn a_private_item_or_import_hides_the_name_a_glob_of_a_sibling_provides() {
    for a in [
        "struct MapMode;\npub use types::*;\n",
        "mod local;\nuse local::MapMode;\npub use types::*;\n",
    ] {
        let tree = Tree::new();
        workspace(&tree, a, MAP_MODE);
        tree.write("a/src/local.rs", "pub struct MapMode;\n");
        tree.base();

        let run = report(&tree);

        assert_eq!(run.code, 0, "{a}: {}", run.out);
        assert!(
            run.says("types::MapMode  type") && !run.says("a::MapMode"),
            "{a}: {}",
            run.out
        );
    }
}

#[test]
fn an_extern_crate_alias_named_like_a_dependency_keeps_its_path_opaque() {
    for (a, item) in [
        (
            "pub mod m {\n    extern crate alloc as types;\n    pub use types::Other;\n}\n",
            "a::m::Other  item  opaque (types::Other)",
        ),
        (
            "extern crate alloc as types;\npub use types::Other;\n",
            "a::Other  item  opaque (types::Other)",
        ),
    ] {
        let tree = Tree::new();
        workspace(&tree, a, "pub struct Other;\n");
        tree.base();

        let run = report(&tree);

        assert!(run.says(item), "{a}: {}", run.out);
    }
}

#[test]
fn only_a_module_level_binding_in_the_same_namespace_hides_a_name_a_glob_provides() {
    for a in [
        "pub struct X;\nimpl TryFrom<u8> for X {\n    type Error = ();\n    fn try_from(_: u8) -> Result<X, ()> {\n        Ok(X)\n    }\n}\npub use types::*;\n",
        "mod sleep {}\nfn Error() {}\npub use types::*;\n",
        "fn f() {\n    use std::fmt::Error;\n    use std::thread::sleep;\n}\npub use types::*;\n",
    ] {
        let tree = Tree::new();
        workspace(&tree, a, "pub struct Error;\npub fn sleep() {}\n");
        tree.base();

        let run = report(&tree);

        assert!(
            run.says("a::Error  type  measured  types/src/lib.rs:1")
                && run.says("a::sleep  function  measured  types/src/lib.rs:2"),
            "{a}: {}",
            run.out
        );
    }
}

#[test]
fn a_use_inside_a_function_named_like_a_dependency_leaves_the_dependency_followed() {
    let tree = Tree::new();
    workspace(
        &tree,
        "mod local {}\nfn f() {\n    use crate::local as types;\n}\npub use types::MapMode;\n",
        MAP_MODE,
    );
    tree.base();

    let run = report(&tree);

    assert!(
        run.says("a::MapMode  type  measured  types/src/lib.rs:1"),
        "{}",
        run.out
    );
}

#[test]
fn a_glob_of_a_workspace_sibling_lists_its_items() {
    let tree = Tree::new();
    workspace(&tree, "pub use types::*;\n", MAP_MODE);
    tree.base();

    let run = report(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("a::MapMode  type  measured  types/src/lib.rs:1") && !run.says("globs"),
        "{}",
        run.out
    );
}

#[test]
fn an_item_of_an_inherent_impl_is_an_item_under_its_type() {
    let tree = Tree::new();
    crate_of(
        &tree,
        "pub struct X;\nimpl X {\n    pub const MAX: u8 = 1;\n}\npub trait T {\n    const N: u8;\n}\n",
    );

    let run = report(&tree);

    assert!(
        run.says("core::X::MAX  constant  measured  src/lib.rs:3")
            && !run.says("core::MAX")
            && !run.says("core::N"),
        "{}",
        run.out
    );
}

#[test]
fn a_body_comment_format_or_binding_name_change_passes() {
    let tree = Tree::new();
    library(&tree);
    tree.write(
        "src/lib.rs",
        &LIB.replace(
            "pub fn parse(input: &str) -> u8 {\n    input.len() as u8\n}\n",
            "/// Parses.\n#[inline]\npub fn parse(\n    text: &str,\n) -> u8 {\n    // changed body\n    text.len() as u8 + 1\n}\n",
        ),
    );
    tree.write(
        "src/client.rs",
        &CLIENT.replace(
            "pub fn new(name: &str) -> Client {",
            "pub fn new(handle: &str)->Client{",
        ),
    );

    let run = by_hand(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn a_changed_signature_and_a_removed_item_fail_and_an_addition_passes() {
    let tree = Tree::new();
    library(&tree);
    tree.write(
        "src/lib.rs",
        &format!("{}pub fn added() {{}}\n", LIB.replace("-> u8", "-> u16")),
    );
    tree.write(
        "src/client.rs",
        &CLIENT.replace(
            "pub fn new(name: &str) -> Client {",
            "pub fn open(name: &str) -> Client {",
        ),
    );

    let run = by_hand(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("2 new compatibility break(s)"), "{}", run.out);
    assert!(
        run.says("changed, declared at src/lib.rs:8, was `fn parse(_: &str) -> u8;`, now `fn parse(_: &str) -> u16;`  parse (function)"),
        "{}",
        run.out
    );
    assert!(
        run.says("removed, declared at src/client.rs:7  Client::new (method)"),
        "{}",
        run.out
    );
    assert!(!run.says("added"), "{}", run.out);
}

#[test]
fn a_removed_library_surface_fails_once_at_the_surface() {
    let tree = Tree::new();
    library(&tree);
    tree.remove("src/lib.rs");
    tree.write("src/main.rs", "fn main() {}\n");

    let run = by_hand(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("1 new compatibility break(s)"), "{}", run.out);
    assert!(
        run.says("core:0  the whole surface is gone  (surface)"),
        "{}",
        run.out
    );
    assert!(!run.says("Client (type)"), "{}", run.out);
}

#[test]
fn a_library_whose_package_has_publish_false_is_still_a_surface() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write("Cargo.toml", &format!("{PACKAGE}publish = false\n"));
    tree.write("src/lib.rs", "pub fn parse() {}\npub fn render() {}\n");
    tree.base();
    tree.write("src/lib.rs", "pub fn parse() {}\n");

    let run = by_hand(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("1 new compatibility break(s)"), "{}", run.out);
    assert!(run.says("render (function)"), "{}", run.out);
}

#[test]
fn a_source_move_behind_an_unchanged_external_identity_passes() {
    let tree = Tree::new();
    library(&tree);
    tree.remove("src/client.rs");
    tree.write("src/net/client.rs", CLIENT);
    tree.write(
        "src/lib.rs",
        &LIB.replace("mod client;\n", "mod net {\n    pub mod client;\n}\n")
            .replace("pub use client::Client;", "pub use net::client::Client;"),
    );

    let run = by_hand(&tree);
    let scoped = changed(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(scoped.code, 0, "{}", scoped.out);
    assert!(
        report(&tree).says("core::Client  type  measured  src/net/client.rs:1"),
        "{}",
        report(&tree).out
    );
}

#[test]
fn an_accepted_break_is_held_by_surface_and_item_identity() {
    let tree = Tree::new();
    library(&tree);
    tree.write("src/lib.rs", &LIB.replace("pub use client::Client;", ""));
    tree.write(
        "klin.json",
        r#"{"accepted":[{"gate":"public-api","file":"core","text":"Client (type)","break":1},{"gate":"public-api","file":"core","text":"Client::new (method)","break":1}]}"#,
    );

    let run = by_hand(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn a_break_in_the_hook_names_the_intended_change_route_and_leaves_acceptance_to_a_person() {
    let tree = Tree::new();
    library(&tree);
    tree.write("klin.json", r#"{"build": []}"#);
    tree.base();
    tree.write("src/lib.rs", &LIB.replace("pub fn parse", "pub fn read"));

    let first = hook(&tree);
    let second = harness::feed(tree.root(), &["gate", "--hook", "--changed"], A_SECOND_STOP);

    assert_eq!(first.code, 2, "{}", first.out);
    assert!(
        first.says("Do not change what the task asked for only to satisfy this gate"),
        "{}",
        first.out
    );
    assert!(
        first.says(
            "If this stop blocked on a break the task intends, say so in your reply and stop again"
        ) && first.says("the next stop may then end the turn"),
        "{}",
        first.out
    );
    assert!(
        first.says("Your reply does not accept the break")
            && first.says("a person accepts it with an accepted entry in a reviewed commit")
            && first.says("CI refuses it"),
        "{}",
        first.out
    );
    assert_eq!(second.code, 0, "{}", second.out);
    assert!(second.says("parse (function)"), "{}", second.out);
    assert!(
        second.says("If this stop blocked on a break the task intends")
            && !second.says("stop ends the turn"),
        "{}",
        second.out
    );
}

#[test]
fn a_break_by_hand_names_person_acceptance_and_no_second_stop() {
    let tree = Tree::new();
    library(&tree);
    tree.write("src/lib.rs", &LIB.replace("pub fn parse", "pub fn read"));

    let run = by_hand(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("Do not change what the task asked for only to satisfy this gate"),
        "{}",
        run.out
    );
    assert!(
        run.says("a person accepts it with an accepted entry in a reviewed commit"),
        "{}",
        run.out
    );
    assert!(!run.says("stop again"), "{}", run.out);
}

#[test]
fn a_glob_of_another_crate_is_a_hole_by_hand_and_a_note_in_the_hook() {
    let tree = Tree::new();
    library(&tree);
    tree.write("klin.json", r#"{"build": []}"#);
    tree.base();
    tree.write("src/lib.rs", &format!("{LIB}pub use serde::*;\n"));

    let run = by_hand(&tree);
    let stop = hook(&tree);

    assert_eq!(run.code, 2, "{}", run.out);
    assert!(
        run.says("FAIL: 1 form(s) inside a supported public surface could not be resolved")
            && run.says("serde::* globs another crate"),
        "{}",
        run.out
    );
    assert_eq!(stop.code, 0, "{}", stop.out);
    assert!(stop.says("could not be resolved"), "{}", stop.out);
}

#[test]
fn a_glob_the_base_holds_too_is_a_note_by_hand_and_under_strict() {
    let tree = Tree::new();
    library(&tree);
    tree.write("klin.json", r#"{"build": []}"#);
    tree.write("src/lib.rs", &format!("{LIB}pub use serde::*;\n"));
    tree.base();

    for args in [&["public-api"][..], &["public-api", "--strict"]] {
        let run = tree.run(args);
        assert_eq!(run.code, 0, "{args:?}: {}", run.out);
        assert!(
            run.says("NOTE: 1 form(s) inside a supported public surface could not be resolved")
                && run.says("serde::* globs another crate"),
            "{args:?}: {}",
            run.out
        );
    }
    tree.write(
        "src/lib.rs",
        &format!("{LIB}pub use serde::*;\npub use tokio::*;\n"),
    );
    let run = by_hand(&tree);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(
        run.says("FAIL: 1 form(s) inside a supported public surface could not be resolved")
            && run.says("tokio::* globs another crate"),
        "{}",
        run.out
    );
}

#[test]
fn a_module_no_file_answers_inside_a_library_is_a_hole_of_this_gate() {
    let tree = Tree::new();
    library(&tree);
    tree.write("src/lib.rs", &format!("{LIB}pub mod missing;\n"));

    let run = by_hand(&tree);

    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("names no file the tree holds"), "{}", run.out);
}

#[test]
fn the_section_is_absent_or_false_and_any_object_is_refused() {
    let tree = Tree::new();
    library(&tree);
    tree.write("klin.json", r#"{"public_api": false}"#);
    let listed = tree.run(&["gate", "--list"]);
    assert_eq!(listed.code, 0, "{}", listed.out);
    assert!(listed.says("public-api — excluded"), "{}", listed.out);

    for config in [
        r#"{"public_api": {"roots": ["src"]}}"#,
        r#"{"public_api": {"languages": ["rust"]}}"#,
        r#"{"public_api": {"surfaces": ["core"]}}"#,
        r#"{"public_api": {"no_widening": true}}"#,
        r#"{"public_api": {}}"#,
    ] {
        tree.write("klin.json", config);
        let run = by_hand(&tree);
        assert_eq!(run.code, 2, "{config}: {}", run.out);
        assert!(
            run.says("\"public_api\" reads no policy")
                && run.says("derived from Cargo library targets and package entry points"),
            "{config}: {}",
            run.out
        );
    }
}

#[test]
fn init_writes_no_public_api_section() {
    let tree = Tree::new();
    library(&tree);
    tree.remove("klin.json");

    let run = tree.run(&["init", "--pin"]);

    assert_eq!(run.code, 0, "{}", run.out);
    let written = std::fs::read_to_string(tree.path("klin.json")).unwrap_or_default();
    assert!(!written.contains("public_api"), "{written}");
}

const MANIFEST: &str = r#"{"name":"@acme/client","exports":{".":"./src/index.ts","./react":{"types":"./src/react.ts","import":"./dist/react.js"}},"main":"./dist/index.js"}"#;
const INDEX: &str = "export { Client } from \"./client\";\nexport type { Options } from \"./client\";\nexport * from \"./model\";\nexport * as util from \"./util\";\nexport { VERSION as version } from \"./model\";\nexport default function create(name: string): Client {\n    return new Client(name);\n}\n";
const TS_CLIENT: &str = "export interface Options {\n    retries?: number;\n}\nexport class Client {\n    constructor(public name: string) {}\n    connect(options: Options): Promise<void> {\n        return Promise.resolve();\n    }\n    private secret(): void {}\n}\n";
const MODEL: &str = "export const VERSION: string = \"1\";\nexport function describe(doc: { title: string }) {\n    return doc.title;\n}\n";

fn package(tree: &Tree) {
    tree.write("klin.json", "{}");
    tree.write("web/package.json", MANIFEST);
    tree.write("web/src/index.ts", INDEX);
    tree.write("web/src/client.ts", TS_CLIENT);
    tree.write("web/src/model.ts", MODEL);
    tree.write(
        "web/src/util.ts",
        "export function helper(): number {\n    return 1;\n}\n",
    );
    tree.write(
        "web/src/react.ts",
        "import { Client } from \"./client\";\nexport function useClient(): Client {\n    return new Client(\"x\");\n}\n",
    );
    tree.write(
        "web/src/internal.ts",
        "export function internal(): void {}\n",
    );
    tree.base();
}

#[test]
fn explicit_root_and_subpath_exports_are_surfaces_and_generated_javascript_is_not_reverse_mapped() {
    let tree = Tree::new();
    package(&tree);

    let run = report(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("surface @acme/client \".\"  TypeScript — web/package.json exports \".\" -> web/src/index.ts")
            && run.says("surface @acme/client \"./react\"  TypeScript — web/package.json exports \"./react\" -> web/src/react.ts"),
        "{}",
        run.out
    );
    for line in [
        "@acme/client \".\" Client  type  measured  web/src/client.ts:4",
        "class Client { connect(_: Options): Promise<void>; constructor(public name: string) }",
        "@acme/client \".\" Options  type  measured",
        "@acme/client \".\" VERSION  constant  measured",
        "@acme/client \".\" version  constant  measured  web/src/model.ts:1",
        "@acme/client \".\" describe  function  measured",
        "function describe(_: { title: string }): ?",
        "@acme/client \".\" default  function  measured  web/src/index.ts:6",
        "function create(_: string): Client",
        "@acme/client \".\" util  namespace  opaque (* as util from ./util)",
        "@acme/client \"./react\" useClient  function  measured",
    ] {
        assert!(run.says(line), "no {line} in: {}", run.out);
    }
    assert!(!run.says("internal") && !run.says("dist/"), "{}", run.out);
}

#[test]
fn a_package_whose_entries_are_generated_or_absent_is_not_applicable() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write(
        "web/package.json",
        r#"{"name":"@acme/built","main":"./dist/index.js","types":"./dist/index.d.ts"}"#,
    );
    tree.write("web/src/index.ts", "export function shown(): void {}\n");
    tree.write("tools/package.json", r#"{"name":"tools","private":true}"#);
    tree.write("tools/run.ts", "export const run = 1;\n");
    tree.base();

    let run = by_hand(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("0 TypeScript entry point(s)"), "{}", run.out);
    assert!(
        run.says("TypeScript package @acme/built (web/package.json): its types names no TypeScript source klin supports: ./dist/index.d.ts names no file the tree holds")
            && run.says("TypeScript package tools (tools/package.json): names no TypeScript source entry point"),
        "{}",
        run.out
    );
}

#[test]
fn a_private_package_with_a_typescript_entry_is_still_a_surface() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write(
        "web/package.json",
        r#"{"name":"@acme/app","private":true,"exports":"./src/index.ts"}"#,
    );
    tree.write(
        "web/src/index.ts",
        "export function parse(): void {}\nexport function render(): void {}\n",
    );
    tree.base();
    tree.write("web/src/index.ts", "export function parse(): void {}\n");

    let run = by_hand(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("1 new compatibility break(s)"), "{}", run.out);
    assert!(run.says("render (function)"), "{}", run.out);
}

#[test]
fn types_typings_and_a_direct_typescript_main_or_module_are_entry_points() {
    for (manifest, field) in [
        (r#"{"name":"a","types":"./src/index.ts"}"#, "types"),
        (r#"{"name":"a","typings":"./src/index.d.ts"}"#, "typings"),
        (r#"{"name":"a","main":"./src/index.ts"}"#, "main"),
        (
            r#"{"name":"a","main":"./dist/index.js","module":"./src/index.ts"}"#,
            "module",
        ),
    ] {
        let tree = Tree::new();
        tree.write("klin.json", "{}");
        tree.write("web/package.json", manifest);
        tree.write("web/src/index.ts", "export function shown(): void {}\n");
        tree.write(
            "web/src/index.d.ts",
            "export declare function shown(): void;\n",
        );
        tree.base();

        let run = report(&tree);

        assert_eq!(run.code, 0, "{manifest}: {}", run.out);
        assert!(
            run.says(&format!("web/package.json {field} \".\" -> web/src/index"))
                && run.says("a \".\" shown  function  measured"),
            "{manifest}: {}",
            run.out
        );
    }
}

#[test]
fn an_exported_file_outside_the_entry_traversal_is_not_package_api() {
    let tree = Tree::new();
    package(&tree);
    tree.write(
        "web/src/internal.ts",
        "export function renamed(): void {}\n",
    );

    let run = by_hand(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn an_ambiguous_star_export_is_a_hole() {
    let tree = Tree::new();
    package(&tree);
    tree.write("web/src/util.ts", "export function helper(): number {\n    return 1;\n}\nexport const VERSION: string = \"2\";\n");
    tree.write(
        "web/src/index.ts",
        &INDEX.replace(
            "export * as util from \"./util\";",
            "export * from \"./util\";",
        ),
    );
    let run = by_hand(&tree);

    assert_eq!(run.code, 2, "{}", run.out);
    assert!(
        run.says("VERSION is provided by this star export and by the one at line 3"),
        "{}",
        run.out
    );
}

#[test]
fn an_external_package_re_export_is_opaque_and_a_star_over_one_is_a_hole() {
    let tree = Tree::new();
    package(&tree);
    tree.write(
        "web/src/index.ts",
        &format!("{INDEX}export {{ pick as choose }} from \"lodash\";\n"),
    );
    tree.base();
    let shown = report(&tree);
    tree.write(
        "web/src/index.ts",
        &format!("{INDEX}export * from \"lodash\";\n"),
    );
    let starred = by_hand(&tree);

    assert!(
        shown.says("@acme/client \".\" choose  item  opaque (pick as choose from lodash)"),
        "{}",
        shown.out
    );
    assert_eq!(starred.code, 2, "{}", starred.out);
    assert!(
        starred.says("a star export of another package, lodash"),
        "{}",
        starred.out
    );
}

#[test]
fn tsx_is_typescript() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write(
        "web/package.json",
        r#"{"name":"ui","exports":"./src/index.tsx"}"#,
    );
    tree.write(
        "web/src/index.tsx",
        "export const View = (text: string) => <p>{text}</p>;\nexport function Button(props: { label: string }): string {\n    return props.label;\n}\n",
    );
    tree.base();

    let run = report(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("ui \".\" Button  function  measured")
            && run.says("ui \".\" View  constant  measured"),
        "{}",
        run.out
    );
    assert!(run.says("const View: ?"), "{}", run.out);
}

#[test]
fn a_typescript_body_comment_format_or_parameter_name_change_passes_and_a_signature_change_fails() {
    let tree = Tree::new();
    package(&tree);
    tree.write(
        "web/src/client.ts",
        &TS_CLIENT.replace(
            "    connect(options: Options): Promise<void> {\n        return Promise.resolve();\n    }\n",
            "    // reconnects\n    connect(settings: Options):Promise<void>{\n        return Promise.reject();\n    }\n",
        ),
    );
    let green = by_hand(&tree);
    tree.write(
        "web/src/client.ts",
        &TS_CLIENT.replace(
            "connect(options: Options): Promise<void>",
            "connect(options: Options, force: boolean): Promise<void>",
        ),
    );
    let red = by_hand(&tree);

    assert_eq!(green.code, 0, "{}", green.out);
    assert_eq!(red.code, 1, "{}", red.out);
    assert!(
        red.says("Client (type)") && red.says("now `class Client { connect(_: Options, _: boolean): Promise<void>; constructor(public name: string) }`"),
        "{}",
        red.out
    );
}

#[test]
fn an_inferred_contract_is_partial_and_a_body_change_behind_it_passes() {
    let tree = Tree::new();
    package(&tree);
    tree.write(
        "web/src/model.ts",
        &MODEL.replace("return doc.title;", "return doc.title.toUpperCase();"),
    );

    let run = by_hand(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        report(&tree).says("function describe(_: { title: string }): ?"),
        "{}",
        report(&tree).out
    );
}

#[test]
fn a_typescript_source_move_behind_an_unchanged_package_identity_passes() {
    let tree = Tree::new();
    package(&tree);
    tree.remove("web/src/client.ts");
    tree.write("web/src/core/client.ts", TS_CLIENT);
    tree.write(
        "web/src/index.ts",
        &INDEX.replace("\"./client\"", "\"./core/client\""),
    );
    tree.write(
        "web/src/react.ts",
        "import { Client } from \"./core/client\";\nexport function useClient(): Client {\n    return new Client(\"x\");\n}\n",
    );

    let run = by_hand(&tree);
    let scoped = changed(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(scoped.code, 0, "{}", scoped.out);
}

#[test]
fn a_removed_package_subpath_fails_once_at_the_surface() {
    let tree = Tree::new();
    package(&tree);
    tree.write(
        "web/package.json",
        r#"{"name":"@acme/client","exports":{".":"./src/index.ts"}}"#,
    );

    let run = by_hand(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("1 new compatibility break(s)"), "{}", run.out);
    assert!(
        run.says("@acme/client \"./react\":0  the whole surface is gone  (surface)"),
        "{}",
        run.out
    );
}

#[test]
fn a_changed_manifest_changes_what_an_unchanged_file_means_in_a_changed_run() {
    let tree = Tree::new();
    package(&tree);
    tree.write(
        "web/package.json",
        r#"{"name":"@acme/client","exports":{".":"./src/index.ts","./react":"./src/util.ts"}}"#,
    );

    let run = changed(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("useClient (function)"), "{}", run.out);
}

#[test]
fn a_cached_changed_run_reads_and_parses_only_the_changed_file() {
    let tree = Tree::new();
    library(&tree);
    tree.write(
        "src/model.rs",
        "pub struct Document {\n    pub title: String,\n}\npub struct Extra;\n",
    );

    let run = || tree.run(&["gate", "--json", "--changed", "--gate", "public-api"]);
    let (first, again) = (run().json(), run().json());
    let counted = |report: &serde_json::Value| {
        ["reads", "parses", "extracted", "cached"].map(|field| {
            report["gates"][0]["facts"][field]
                .as_u64()
                .unwrap_or(u64::MAX)
        })
    };

    assert_eq!(counted(&first), [5, 5, 5, 0], "{first}");
    assert_eq!(counted(&again), [2, 2, 2, 3], "{again}");
    assert_eq!(first["status"], "PASS", "{first}");
    assert_eq!(first["status"], again["status"]);
    assert_eq!(again["gates"][0]["surface"]["surfaces"], 2, "{again}");
    assert!(
        again["gates"][0]["surface"]["items"].as_u64() >= Some(11),
        "{again}"
    );
}

#[test]
fn a_tree_with_no_typescript_path_derives_only_rust_surfaces() {
    let tree = Tree::new();
    library(&tree);

    let report = tree.run(&["gate", "--json", "--gate", "public-api"]).json();
    let gate = &report["gates"][0];

    assert_eq!(
        gate["surface"]["dispatches"],
        serde_json::json!({"rust": 2, "typescript": 0}),
        "{report}"
    );
    assert_eq!(
        gate["graph"]["dispatches"],
        serde_json::json!({"rust": 2, "typescript": 0}),
        "{report}"
    );
}

/// A crate whose root holds `lib` alone, committed as the base.
fn crate_of(tree: &Tree, lib: &str) {
    tree.write("klin.json", "{}");
    tree.write("Cargo.toml", PACKAGE);
    tree.write("src/lib.rs", lib);
    tree.base();
}

/// A package whose one entry file holds `index`, committed as the base.
fn package_of(tree: &Tree, index: &str) {
    tree.write("klin.json", "{}");
    tree.write(
        "web/package.json",
        r#"{"name":"a","types":"./src/index.ts"}"#,
    );
    tree.write("web/src/index.ts", index);
    tree.base();
}

#[test]
fn non_exhaustive_added_to_a_type_or_a_variant_fails_and_was_and_now_differ() {
    let tree = Tree::new();
    crate_of(
        &tree,
        "pub struct S {\n    pub a: u8,\n}\npub enum E {\n    A,\n    B { x: u8 },\n}\n",
    );
    tree.write(
        "src/lib.rs",
        "/// A point.\n#[non_exhaustive]\n#[derive(Debug)]\npub struct S {\n    pub a: u8,\n}\n#[derive(Debug)]\npub enum E {\n    A,\n    #[non_exhaustive]\n    B { x: u8 },\n}\n",
    );

    let run = by_hand(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("2 new compatibility break(s)"), "{}", run.out);
    assert!(
        run.says("was `struct S { a: u8 }`, now `#[non_exhaustive] struct S { a: u8 }`"),
        "{}",
        run.out
    );
    assert!(
        run.says(
            "was `enum E { A, B { x: u8 } }`, now `enum E { A, #[non_exhaustive] B { x: u8 } }`"
        ),
        "{}",
        run.out
    );
}

#[test]
fn a_private_field_added_to_a_struct_whose_fields_were_all_public_fails() {
    let tree = Tree::new();
    crate_of(
        &tree,
        "pub struct S {\n    pub a: u8,\n}\npub struct T {\n    pub a: u8,\n    b: u8,\n}\n",
    );
    tree.write(
        "src/lib.rs",
        "pub struct S {\n    pub a: u8,\n    b: u8,\n}\npub struct T {\n    pub a: u8,\n    b: u8,\n    #[cfg(unix)]\n    c: u8,\n}\n",
    );

    let run = by_hand(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("1 new compatibility break(s)"), "{}", run.out);
    assert!(
        run.says("was `struct S { a: u8 }`, now `struct S { a: u8, .. }`"),
        "{}",
        run.out
    );
    assert!(!run.says("T (type)"), "{}", run.out);
}

#[test]
fn a_default_body_removed_from_a_trait_method_fails() {
    let tree = Tree::new();
    crate_of(
        &tree,
        "pub trait Tr {\n    fn f(&self) -> u8 {\n        1\n    }\n}\n",
    );
    tree.write(
        "src/lib.rs",
        "pub trait Tr {\n    fn f(&self) -> u8 {\n        2\n    }\n}\n",
    );
    let green = by_hand(&tree);
    tree.write("src/lib.rs", "pub trait Tr {\n    fn f(&self) -> u8;\n}\n");

    let red = by_hand(&tree);

    assert_eq!(green.code, 0, "{}", green.out);
    assert_eq!(red.code, 1, "{}", red.out);
    assert!(
        red.says(
            "was `trait Tr { fn f(&self) -> u8 { .. } }`, now `trait Tr { fn f(&self) -> u8; }`"
        ),
        "{}",
        red.out
    );
}

const OVERLOADS: &str = "export function parse(input: string): string;\nexport function parse(input: number): number;\nexport function parse(input: string | number): string | number {\n    return input;\n}\n";

#[test]
fn a_change_that_only_reorders_an_overload_set_in_one_file_fails() {
    let tree = Tree::new();
    package_of(&tree, OVERLOADS);
    tree.write(
        "web/src/index.ts",
        &OVERLOADS.replace(
            "input: string | number): string | number",
            "input: any): any",
        ),
    );
    let green = by_hand(&tree);
    let listed = report(&tree);
    tree.write(
        "web/src/index.ts",
        "export function parse(input: number): number;\nexport function parse(input: string): string;\nexport function parse(input: any): any {\n    return input;\n}\n",
    );

    let red = by_hand(&tree);

    assert_eq!(green.code, 0, "{}", green.out);
    assert!(
        listed.says("function parse(_: string): string; function parse(_: number): number\n"),
        "{}",
        listed.out
    );
    assert_eq!(red.code, 1, "{}", red.out);
    assert!(
        red.says("was `function parse(_: string): string; function parse(_: number): number`, now `function parse(_: number): number; function parse(_: string): string`"),
        "{}",
        red.out
    );
}

#[test]
fn a_trailing_default_is_optional_and_a_default_a_required_parameter_follows_is_not() {
    let tree = Tree::new();
    package_of(
        &tree,
        "export function f(a: number, unit: \"m\" | \"km\" = \"m\"): number {\n    return a;\n}\nexport function g(unit: string = \"m\", a: number): number {\n    return a;\n}\n",
    );
    let listed = report(&tree);
    tree.write(
        "web/src/index.ts",
        "export function f(a: number, unit?: \"m\" | \"km\"): number {\n    return a;\n}\nexport function g(unit: string = \"km\", a: number): number {\n    return a;\n}\n",
    );

    let green = by_hand(&tree);
    tree.write(
        "web/src/index.ts",
        "export function f(a: number, unit?: \"m\" | \"km\"): number {\n    return a;\n}\nexport function g(unit: string, a: number): number {\n    return a;\n}\n",
    );

    let red = by_hand(&tree);

    assert!(
        listed.says("function f(_: number, _?: \"m\" | \"km\"): number")
            && listed.says("function g(_: string = .., _: number): number"),
        "{}",
        listed.out
    );
    assert_eq!(green.code, 0, "{}", green.out);
    assert_eq!(red.code, 1, "{}", red.out);
    assert!(
        red.says("was `function g(_: string = .., _: number): number`, now `function g(_: string, _: number): number`"),
        "{}",
        red.out
    );
}

#[test]
fn an_optional_property_added_to_an_exported_interface_fails_and_the_remedy_names_the_additive_route()
 {
    let tree = Tree::new();
    package_of(
        &tree,
        "export interface Point {\n    lat: number;\n    lon: number;\n}\n",
    );
    tree.write(
        "web/src/index.ts",
        "export interface Point {\n    height?: number;\n    lat: number;\n    lon: number;\n}\n",
    );

    let run = by_hand(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("Point (type)"), "{}", run.out);
    assert!(
        run.says("a new item beside the unchanged one keeps the base's contract"),
        "{}",
        run.out
    );
    assert!(
        run.says("Do not change what the task asked for only to satisfy this gate"),
        "{}",
        run.out
    );
}

#[test]
fn a_removed_module_prints_as_one_group_and_json_keeps_each_item() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write("Cargo.toml", PACKAGE);
    tree.write("src/lib.rs", "pub mod cmd;\npub mod other;\n");
    tree.write("src/cmd.rs", "pub fn run() {}\n");
    tree.write("src/other.rs", "pub fn go() {}\n");
    tree.base();
    tree.remove("src/cmd.rs");
    tree.remove("src/other.rs");
    tree.write("src/lib.rs", "pub fn kept() {}\n");

    let run = by_hand(&tree);
    let json = tree.run(&["gate", "--gate", "public-api", "--json"]);

    assert_eq!(run.code, 1, "{}", run.out);
    let lines: Vec<&str> = run.out.lines().collect();
    for (module, item) in [
        ("cmd (module)", "cmd::run (function)"),
        ("other (module)", "other::go (function)"),
    ] {
        let lead = lines
            .iter()
            .position(|line| line.starts_with("  core:") && line.contains(module))
            .unwrap_or_else(|| panic!("no {module} line: {}", run.out));
        assert!(
            lines[lead + 1].starts_with("    core:") && lines[lead + 1].contains(item),
            "{}",
            run.out
        );
    }
    let report = json.json();
    let texts: Vec<&str> = report["findings"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|finding| finding["text"].as_str())
        .collect();
    assert_eq!(texts.len(), 4, "{}", json.out);
    assert!(texts.contains(&"cmd::run (function)"), "{}", json.out);
}

#[test]
fn a_method_overload_set_keeps_its_source_order_and_drops_its_implementation() {
    let tree = Tree::new();
    package_of(
        &tree,
        "export class L {\n    m(x: string): string;\n    m(x: number): number;\n    m(x: string | number): string | number {\n        return x;\n    }\n    get v(): number {\n        return 1;\n    }\n}\nexport interface I {\n    (x: string): string;\n    (x: number): number;\n    f(x: string): void;\n    f(x: number): void;\n}\n",
    );
    tree.write(
        "web/src/index.ts",
        "export class L {\n    m(x: string): string;\n    m(x: number): number;\n    m(x: any): any {\n        return x;\n    }\n    get v(): number {\n        return 1;\n    }\n}\nexport interface I {\n    (x: string): string;\n    (x: number): number;\n    f(x: string): void;\n    f(x: number): void;\n}\n",
    );
    let green = by_hand(&tree);
    let listed = report(&tree);
    tree.write(
        "web/src/index.ts",
        "export class L {\n    m(x: string): string;\n    m(x: number): number;\n    m(x: any): any {\n        return x;\n    }\n    get v(): number {\n        return 1;\n    }\n}\nexport interface I {\n    (x: number): number;\n    (x: string): string;\n    f(x: number): void;\n    f(x: string): void;\n}\n",
    );

    let red = by_hand(&tree);

    assert_eq!(green.code, 0, "{}", green.out);
    assert!(
        listed.says("class L { get v(): number; m(_: string): string; m(_: number): number }"),
        "{}",
        listed.out
    );
    assert_eq!(red.code, 1, "{}", red.out);
    assert!(
        red.says("was `interface I { (_: string): string; (_: number): number; f(_: string): void; f(_: number): void }`, now `interface I { (_: number): number; (_: string): string; f(_: number): void; f(_: string): void }`"),
        "{}",
        red.out
    );
}

#[test]
fn methods_whose_keys_hold_brackets_stay_distinct_and_an_overload_key_ignores_access_and_async() {
    let tree = Tree::new();
    package_of(
        &tree,
        "export class C {\n    [Symbol.for(\"a\")](x: string): void {}\n    [Symbol.for(\"b\")](x: string): void {}\n    public m(a: string): string;\n    m(a: number): number;\n    async m(a: any): Promise<any> {\n        return a;\n    }\n}\n",
    );
    tree.write(
        "web/src/index.ts",
        "export class C {\n    [Symbol.for(\"a\")](x: string): void {}\n    [Symbol.for(\"b\")](x: string): void {}\n    public m(a: string): string;\n    m(a: number): number;\n    async m(a: string | number): Promise<unknown> {\n        return a;\n    }\n}\n",
    );
    let green = by_hand(&tree);
    tree.write(
        "web/src/index.ts",
        "export class C {\n    [Symbol.for(\"a\")](x: string): void {}\n    [Symbol.for(\"b\")](x: number): void {}\n    m(a: number): number;\n    public m(a: string): string;\n    async m(a: any): Promise<any> {\n        return a;\n    }\n}\n",
    );

    let red = by_hand(&tree);

    assert_eq!(green.code, 0, "{}", green.out);
    assert_eq!(red.code, 1, "{}", red.out);
    assert!(
        red.says("was `class C { [Symbol.for(\"a\")](_: string): void; [Symbol.for(\"b\")](_: string): void; public m(_: string): string; m(_: number): number }`"),
        "{}",
        red.out
    );
}

#[test]
fn a_default_removed_from_a_trait_const_fails() {
    let tree = Tree::new();
    crate_of(&tree, "pub trait Tr {\n    const N: u8 = 1;\n}\n");
    tree.write("src/lib.rs", "pub trait Tr {\n    const N: u8 = 2;\n}\n");
    let green = by_hand(&tree);
    tree.write("src/lib.rs", "pub trait Tr {\n    const N: u8;\n}\n");

    let red = by_hand(&tree);

    assert_eq!(green.code, 0, "{}", green.out);
    assert_eq!(red.code, 1, "{}", red.out);
    assert!(
        red.says("was `trait Tr { const N: u8 = ..; }`, now `trait Tr { const N: u8; }`"),
        "{}",
        red.out
    );
}

#[test]
fn reordering_the_cfg_declarations_of_one_rust_item_passes() {
    let tree = Tree::new();
    crate_of(
        &tree,
        "#[cfg(unix)]\npub fn f(_: u8) {}\n#[cfg(not(unix))]\npub fn f(_: u16) {}\n",
    );
    tree.write(
        "src/lib.rs",
        "#[cfg(not(unix))]\npub fn f(_: u16) {}\n#[cfg(unix)]\npub fn f(_: u8) {}\n",
    );

    let run = by_hand(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn a_removed_module_of_two_surfaces_with_one_name_prints_each_item_once() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    for root in ["a", "b"] {
        tree.write(&format!("{root}/Cargo.toml"), PACKAGE);
        tree.write(
            &format!("{root}/src/lib.rs"),
            "pub mod m {\n    pub fn f() {}\n    pub mod n {\n        pub fn g() {}\n    }\n}\n",
        );
    }
    tree.base();
    for root in ["a", "b"] {
        tree.write(&format!("{root}/src/lib.rs"), "pub fn kept() {}\n");
    }

    let run = by_hand(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    let printed = run
        .out
        .lines()
        .filter(|line| line.trim_start().starts_with("core:"))
        .count();
    assert!(run.says("8 new compatibility break(s)"), "{}", run.out);
    assert_eq!(printed, 8, "{}", run.out);
}

#[test]
fn quoted_method_names_that_hold_brackets_stay_distinct() {
    let tree = Tree::new();
    package_of(
        &tree,
        "export class C {\n    \"x(a\"(): void {}\n    \"x(b\"(): void {}\n}\n",
    );
    tree.write(
        "web/src/index.ts",
        "export class C {\n    \"x(a\"(): void {}\n}\n",
    );

    let run = by_hand(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("C (type)"), "{}", run.out);
}

#[test]
fn a_constructor_implementation_that_leaves_the_set_keeps_its_parameter_properties() {
    let tree = Tree::new();
    package_of(
        &tree,
        "export class C {\n    constructor(a: string);\n    constructor(public a: any, private b?: number) {}\n}\n",
    );
    let listed = report(&tree);
    tree.write(
        "web/src/index.ts",
        "export class C {\n    constructor(a: string);\n    constructor(a: any, private b?: number) {}\n}\n",
    );

    let run = by_hand(&tree);

    assert!(
        listed.says("class C { constructor(_: string); public a: any }"),
        "{}",
        listed.out
    );
    assert_eq!(run.code, 1, "{}", run.out);
}

#[test]
fn a_quoted_overload_name_is_the_same_name_and_a_this_parameter_shows() {
    let tree = Tree::new();
    package_of(
        &tree,
        "export class C {\n    \"m\"(a: string): void;\n    m(a: any) {}\n}\nexport function h(this: Date, a: string): void {}\n",
    );
    let listed = report(&tree);
    tree.write(
        "web/src/index.ts",
        "export class C {\n    \"m\"(a: string): void;\n    m(a: unknown) {}\n}\nexport function h(x: Date, a: string): void {}\n",
    );

    let run = by_hand(&tree);

    assert!(
        listed.says("function h(this: Date, _: string): void"),
        "{}",
        listed.out
    );
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("h (function)") && !run.says("C (type)"),
        "{}",
        run.out
    );
}

#[test]
fn a_defaulted_parameter_property_is_not_an_optional_one() {
    let tree = Tree::new();
    package_of(
        &tree,
        "export class C {\n    constructor(public a: number = 1) {}\n}\n",
    );
    tree.write(
        "web/src/index.ts",
        "export class C {\n    constructor(public a?: number) {}\n}\n",
    );

    let run = by_hand(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("was `class C { constructor(public a: number = ..) }`"),
        "{}",
        run.out
    );
}

#[test]
fn a_reorder_of_overloads_whose_names_are_quoted_fails_and_a_private_constructor_shows() {
    let tree = Tree::new();
    package_of(
        &tree,
        "export interface I {\n    f(a: string): string;\n    [\"f\"](a: unknown): number;\n}\nexport class K {\n    n(): void {}\n}\n",
    );
    tree.write(
        "web/src/index.ts",
        "export interface I {\n    [\"f\"](a: unknown): number;\n    f(a: string): string;\n}\nexport class K {\n    private constructor() {}\n    n(): void {}\n}\n",
    );

    let run = by_hand(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("I (type)") && run.says("K (type)"), "{}", run.out);
    assert!(run.says("private constructor()"), "{}", run.out);
}

#[test]
fn an_object_type_literal_keeps_its_overloads_in_source_order() {
    let tree = Tree::new();
    package_of(
        &tree,
        "export type T = {\n    f(x: string): string;\n    f(x: number): number;\n};\n",
    );
    tree.write(
        "web/src/index.ts",
        "export type T = {\n    f(x: number): number;\n    f(x: string): string;\n};\n",
    );

    let run = by_hand(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("was `type T = { f(_: string): string; f(_: number): number; }`"),
        "{}",
        run.out
    );
}

const NAMESPACE: &str = "export declare namespace N {\n    type T = string;\n}\n";

#[test]
fn a_new_exported_declare_namespace_in_an_entry_file_is_an_item_and_passes() {
    let tree = Tree::new();
    package_of(&tree, "export type A = string;\n");
    tree.write(
        "web/src/index.ts",
        &format!("export type A = string;\n{NAMESPACE}"),
    );

    let run = tree.run(&["gate", "--gate", "public-api"]);
    let listed = report(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("could not be resolved"), "{}", run.out);
    assert!(
        listed.says(
            "a \".\" N  namespace  opaque (namespace N { type T = string; })  web/src/index.ts:2"
        ),
        "{}",
        listed.out
    );
}

#[test]
fn an_exported_namespace_the_working_tree_lacks_fails_as_removed() {
    let tree = Tree::new();
    package_of(&tree, &format!("export type A = string;\n{NAMESPACE}"));
    tree.write("web/src/index.ts", "export type A = string;\n");

    let run = by_hand(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("1 new compatibility break(s)"), "{}", run.out);
    assert!(
        run.says("removed, declared at web/src/index.ts:2  N (namespace)"),
        "{}",
        run.out
    );
}

#[test]
fn an_exported_namespace_whose_declaration_changed_fails_as_changed() {
    let tree = Tree::new();
    package_of(&tree, NAMESPACE);
    tree.write(
        "web/src/index.ts",
        "export declare namespace N {\n    // the text\n    type T =\n        string;\n}\n",
    );
    let green = by_hand(&tree);
    tree.write("web/src/index.ts", &NAMESPACE.replace("string", "number"));

    let red = by_hand(&tree);

    assert_eq!(green.code, 0, "{}", green.out);
    assert_eq!(red.code, 1, "{}", red.out);
    assert!(
        red.says("was `namespace N { type T = string; }`, now `namespace N { type T = number; }`  N (namespace)"),
        "{}",
        red.out
    );
}

#[test]
fn export_equals_and_an_ambient_module_are_still_holes() {
    let tree = Tree::new();
    package_of(&tree, "export type A = string;\n");
    tree.write(
        "web/src/index.ts",
        "export type A = string;\nexport declare module \"x\" {\n    const z: number;\n}\nexport = A;\n",
    );

    let run = by_hand(&tree);

    assert_eq!(run.code, 2, "{}", run.out);
    assert!(
        run.says("FAIL: 2 form(s) inside a supported public surface could not be resolved"),
        "{}",
        run.out
    );
    for line in [
        "web/src/index.ts:2  export declare module \"x\" {",
        "web/src/index.ts:5  export = A;",
    ] {
        assert!(
            run.says(&format!(
                "{line}  — a \".\" — an export form klin does not list: `export =` or an ambient module"
            )),
            "no {line} in: {}",
            run.out
        );
    }
    assert!(!run.says("namespace"), "{}", run.out);
}
