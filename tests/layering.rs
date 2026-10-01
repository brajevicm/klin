mod harness;

use harness::Tree;

const LAYERS: &str = r#"{"layering":{"layers":{"ui":{"in":"src/ui","can_use":["domain"]},"domain":{"in":"src/domain","can_use":[]}}}}"#;

fn two_layers(tree: &Tree, domain: &str) {
    tree.write("klin.json", LAYERS);
    tree.write("src/lib.rs", "mod domain;\nmod ui;\n");
    tree.write("src/ui/mod.rs", "pub fn show() {}\n");
    tree.write("src/domain/mod.rs", domain);
}

#[test]
fn a_new_forbidden_dependency_fails_as_new() {
    let tree = Tree::new();
    two_layers(&tree, "use crate::ui::show;\npub fn rule() { show(); }\n");

    let run = tree.run(&["layering"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("1 new"), "{}", run.out);
    assert!(
        run.says("src/domain/mod.rs:1") && run.says("domain → ui: src/ui/mod.rs"),
        "{}",
        run.out
    );
}

#[test]
fn a_forbidden_dependency_the_base_holds_is_held() {
    let tree = Tree::new();
    two_layers(&tree, "use crate::ui::show;\npub fn rule() { show(); }\n");
    tree.base();

    let run = tree.run(&["layering"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("the base already held"), "{}", run.out);
}

#[test]
fn an_allowed_and_a_same_layer_dependency_pass() {
    let tree = Tree::new();
    two_layers(
        &tree,
        "mod rules;\npub fn rule() { self::rules::check(); }\n",
    );
    tree.write("src/domain/rules.rs", "pub fn check() {}\n");
    tree.write(
        "src/ui/mod.rs",
        "pub fn show() { crate::domain::rule(); }\n",
    );

    let run = tree.run(&["layering"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("0 forbidden"), "{}", run.out);
}

#[test]
fn can_use_null_lets_a_layer_use_every_layer() {
    let tree = Tree::new();
    two_layers(&tree, "pub fn rule() { crate::ui::show(); }\n");
    tree.write(
        "klin.json",
        r#"{"layering":{"layers":{"ui":{"in":"src/ui","can_use":[]},"domain":{"in":"src/domain","can_use":null}}}}"#,
    );

    let run = tree.run(&["layering"]);

    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn a_file_in_two_layers_is_a_configuration_error_naming_both() {
    let tree = Tree::new();
    two_layers(&tree, "pub fn rule() {}\n");
    tree.write(
        "klin.json",
        r#"{"layering":{"layers":{"all":{"in":"src","can_use":[]},"domain":{"in":"src/domain","can_use":[]}}}}"#,
    );

    let run = tree.run(&["layering"]);

    assert_eq!(run.code, 2, "{}", run.out);
    assert!(
        run.says("src/domain/mod.rs") && run.says("\"all\"") && run.says("\"domain\""),
        "{}",
        run.out
    );
}

#[test]
fn retired_topology_and_unknown_keys_are_refused() {
    for (config, said) in [
        (
            r#"{"layering":{"roots":["src"],"layers":{}}}"#,
            "no longer reads \"roots\"",
        ),
        (
            r#"{"layering":{"layers":{"ui":{"in":"src/ui","allow_same":true}}}}"#,
            "unknown field \"allow_same\"",
        ),
        (
            r#"{"layering":{"layers":{"ui":{"in":"src/*","can_use":[]}}}}"#,
            "never a glob",
        ),
        (
            r#"{"layering":{"layers":{"ui":{"in":"src/ui","can_use":["nowhere"]}}}}"#,
            "\"nowhere\"",
        ),
    ] {
        let tree = Tree::new();
        two_layers(&tree, "pub fn rule() {}\n");
        tree.write("klin.json", config);

        let run = tree.run(&["layering"]);

        assert_eq!(run.code, 2, "{config}: {}", run.out);
        assert!(run.says(said), "{config}: {}", run.out);
    }
}

const ACYCLIC: &str = r#"{"layering":{"acyclic":true,"layers":{"app":{"in":"src","can_use":[]}}}}"#;
const WEB: &str = r#"{"layering":{"acyclic":true,"layers":{"view":{"in":"web/view","can_use":["model"]},"model":{"in":"web/model","can_use":[]}}}}"#;
const A_STOP: &str = r#"{"hook_event_name": "Stop", "stop_hook_active": false}"#;
const PACKAGE: &str = "[package]\nname = \"t\"\nversion = \"0.1.0\"\nedition = \"2024\"\n";

fn changed(tree: &Tree) -> harness::Run {
    tree.run(&["gate", "--changed", "--gate", "layering"])
}

#[test]
fn a_super_path_inside_an_inline_module_resolves_from_that_module() {
    let tree = Tree::new();
    two_layers(
        &tree,
        "pub fn rule() {}\nmod checks {\n    pub fn run() { super::super::ui::show(); }\n}\n",
    );

    let run = tree.run(&["layering"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("src/domain/mod.rs:3") && run.says("domain → ui: src/ui/mod.rs"),
        "{}",
        run.out
    );
}

fn shared_by_path(tree: &Tree) {
    tree.write("klin.json", LAYERS);
    tree.write(
        "src/lib.rs",
        "mod domain;\nmod ui;\n#[path = \"domain/shared.rs\"]\nmod shared;\n",
    );
    tree.write("src/ui/mod.rs", "pub fn show() {}\n");
    tree.write("src/ui/shared.rs", "pub fn help() {}\n");
    tree.write("src/domain/shared.rs", "pub fn help() {}\n");
    tree.write(
        "src/domain/mod.rs",
        "pub fn rule() { crate::shared::help(); }\n",
    );
}

#[test]
fn a_path_attribute_that_retargets_an_unchanged_file_is_new_debt_in_a_changed_run() {
    let tree = Tree::new();
    shared_by_path(&tree);
    tree.base();
    tree.write(
        "src/lib.rs",
        "mod domain;\nmod ui;\n#[path = \"ui/shared.rs\"]\nmod shared;\n",
    );

    let run = changed(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("src/domain/mod.rs:1") && run.says("domain → ui: src/ui/shared.rs"),
        "{}",
        run.out
    );
}

#[test]
fn a_manifest_that_moves_the_library_root_changes_what_an_unchanged_file_reaches() {
    let tree = Tree::new();
    shared_by_path(&tree);
    tree.write("Cargo.toml", PACKAGE);
    tree.write(
        "src/other.rs",
        "mod domain;\nmod ui;\n#[path = \"ui/shared.rs\"]\nmod shared;\n",
    );
    tree.base();
    assert_eq!(changed(&tree).code, 0);
    tree.write(
        "Cargo.toml",
        &format!("{PACKAGE}[lib]\npath = \"src/other.rs\"\n"),
    );

    let run = changed(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("domain → ui: src/ui/shared.rs"), "{}", run.out);
}

#[test]
fn a_workspace_member_that_inherits_its_edition_is_attached_by_its_manifest() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{"layering":{"layers":{"ui":{"in":"crates/app/src/ui","can_use":[]},"domain":{"in":"crates/app/src/domain","can_use":[]}}}}"#,
    );
    tree.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/app\"]\n\n[workspace.package]\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    tree.write(
        "crates/app/Cargo.toml",
        "[package]\nname = \"app\"\nversion.workspace = true\nedition.workspace = true\n",
    );
    tree.write(
        "crates/app/src/main.rs",
        "mod domain;\nmod ui;\nfn main() {}\n",
    );
    tree.write("crates/app/src/ui/mod.rs", "pub fn show() {}\n");
    tree.write("crates/app/src/domain/mod.rs", "pub fn rule() {}\n");

    let run = tree.run(&["layering"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("3 file(s) attached, 3 by a Cargo manifest and 0 by a conventional root"),
        "{}",
        run.out
    );
}

#[test]
fn a_file_two_targets_reach_is_judged_as_a_module_of_each() {
    let tree = Tree::new();
    two_layers(&tree, "pub fn rule() { crate::ui::show(); }\n");
    tree.write("Cargo.toml", PACKAGE);
    tree.write(
        "src/main.rs",
        "mod domain;\n#[path = \"ui/other.rs\"]\nmod ui;\nfn main() {}\n",
    );
    tree.write("src/ui/other.rs", "pub fn show() {}\n");

    let run = tree.run(&["layering"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("2 new"), "{}", run.out);
    assert!(run.says("domain → ui: src/ui/mod.rs"), "{}", run.out);
    assert!(run.says("domain → ui: src/ui/other.rs"), "{}", run.out);
}

#[test]
fn a_module_two_files_answered_at_the_base_too_is_a_note_by_hand_and_under_strict() {
    let tree = Tree::new();
    two_layers(&tree, "pub fn rule() {}\n");
    tree.write("src/ui.rs", "pub fn show() {}\n");
    tree.base();

    for args in [&["layering"][..], &["layering", "--strict"]] {
        let run = tree.run(args);
        assert_eq!(run.code, 0, "{args:?}: {}", run.out);
        assert!(
            run.says("NOTE: 1 dependency form(s) klin resolves could not be resolved")
                && run.says("names more than one file: src/ui.rs, src/ui/mod.rs"),
            "{args:?}: {}",
            run.out
        );
    }
}

#[test]
fn a_module_two_files_answer_is_unresolved_by_hand_and_a_note_in_the_hook() {
    let tree = Tree::new();
    two_layers(&tree, "pub fn rule() {}\n");
    tree.base();
    tree.write("src/ui.rs", "pub fn show() {}\n");

    let run = tree.run(&["layering"]);
    let hook = harness::feed(tree.root(), &["gate", "--hook", "--changed"], A_STOP);

    assert_eq!(run.code, 2, "{}", run.out);
    assert!(
        run.says("names more than one file: src/ui.rs, src/ui/mod.rs"),
        "{}",
        run.out
    );
    assert_eq!(hook.code, 0, "{}", hook.out);
    assert!(hook.says("could not be resolved"), "{}", hook.out);
}

#[test]
fn typescript_relative_imports_resolve_and_package_imports_are_counted_not_guessed() {
    let tree = Tree::new();
    tree.write("klin.json", WEB);
    tree.write(
        "web/model/index.ts",
        "import React from \"react\";\nexport const model = 1;\n",
    );
    tree.write(
        "web/view/render.tsx",
        "import { model } from \"../model\";\nexport const render = () => <p>{model}</p>;\n",
    );

    let green = tree.run(&["layering"]);
    tree.write(
        "web/model/index.ts",
        "import { render } from \"../view/render.js\";\nexport const model = 1;\n",
    );
    let red = tree.run(&["layering"]);

    assert_eq!(green.code, 0, "{}", green.out);
    assert!(
        green.says("1 dependency site(s) judged")
            && green.says("1 external or unsupported dependenc(ies)"),
        "{}",
        green.out
    );
    assert_eq!(red.code, 1, "{}", red.out);
    assert!(
        red.says("model → view: web/view/render.tsx") && red.says("cycle: web/model/index.ts"),
        "{}",
        red.out
    );
}

#[test]
fn two_typescript_files_one_specifier_names_are_unresolved() {
    let tree = Tree::new();
    tree.write("klin.json", WEB);
    tree.write("web/model/index.ts", "export const model = 1;\n");
    tree.write("web/view/x.ts", "export const x = 1;\n");
    tree.write("web/view/x/index.ts", "export const x = 2;\n");
    tree.write("web/view/use.ts", "import { x } from \"./x\";\n");

    let run = tree.run(&["layering"]);

    assert_eq!(run.code, 2, "{}", run.out);
    assert!(
        run.says("names more than one TypeScript file: web/view/x.ts, web/view/x/index.ts"),
        "{}",
        run.out
    );
}

#[test]
fn a_second_copy_of_a_form_the_base_could_not_resolve_is_new() {
    let tree = Tree::new();
    tree.write("klin.json", WEB);
    tree.write("web/model/index.ts", "export const model = 1;\n");
    tree.write("web/view/x.ts", "export const x = 1;\n");
    tree.write("web/view/x/index.ts", "export const x = 2;\n");
    tree.write("web/view/use.ts", "import { x } from \"./x\";\n");
    tree.base();
    tree.write(
        "web/view/use.ts",
        "import { x } from \"./x\";\nimport { x as y } from \"./x\";\n",
    );

    let run = tree.run(&["layering"]);

    assert_eq!(run.code, 2, "{}", run.out);
    assert!(
        run.says("NOTE: 1 dependency form(s) klin resolves could not be resolved")
            && run.says("FAIL: 1 dependency form(s) klin resolves could not be resolved"),
        "{}",
        run.out
    );
}

#[test]
fn a_new_cycle_fails_and_a_cycle_the_base_holds_is_held() {
    let tree = Tree::new();
    tree.write("klin.json", ACYCLIC);
    tree.write("src/lib.rs", "mod a;\nmod b;\nmod c;\n");
    tree.write("src/a.rs", "pub fn x() { crate::b::y(); }\n");
    tree.write("src/b.rs", "pub fn y() { crate::a::x(); }\n");
    tree.write(
        "src/c.rs",
        "pub enum K { A }\nuse self::K::A;\npub fn z() {}\n",
    );
    tree.base();

    let held = tree.run(&["layering"]);
    tree.write("src/c.rs", "pub fn z() { crate::a::x(); }\n");
    tree.write("src/a.rs", "pub fn x() { crate::b::y(); crate::c::z(); }\n");
    let new = tree.run(&["layering"]);

    assert_eq!(held.code, 0, "{}", held.out);
    assert!(held.says("the base already held"), "{}", held.out);
    assert_eq!(new.code, 1, "{}", new.out);
    assert!(new.says("2 new"), "{}", new.out);
    assert!(
        new.says("cycle: src/c.rs") && new.says("src/a.rs → src/c.rs → src/a.rs"),
        "{}",
        new.out
    );
}

#[test]
fn a_module_that_imports_itself_is_a_cycle() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{"layering":{"acyclic":true,"layers":{"view":{"in":"web","can_use":[]}}}}"#,
    );
    tree.write(
        "web/a.ts",
        "import { a } from \"./a\";\nexport const a = 1;\n",
    );

    let run = tree.run(&["layering"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("cycle: web/a.ts"), "{}", run.out);
}

#[test]
fn a_rust_path_to_its_own_module_is_no_cycle() {
    let tree = Tree::new();
    tree.write("klin.json", ACYCLIC);
    tree.write("src/lib.rs", "mod a;\n");
    tree.write("src/a.rs", "pub fn x() {}\n");
    tree.base();
    tree.write(
        "src/a.rs",
        "pub enum K { A }\nuse self::K::A;\npub fn x() -> K { crate::a::K::A }\n",
    );

    let run = tree.run(&["layering"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("0 cyclic"), "{}", run.out);
}

#[test]
fn a_retarget_to_an_inline_module_of_the_same_file_is_new() {
    let tree = Tree::new();
    two_layers(&tree, "pub fn rule() { crate::ui::show(); }\n");
    tree.write(
        "src/ui/mod.rs",
        "pub fn show() {}\npub mod inner {\n    pub fn show() {}\n}\n",
    );
    tree.base();
    tree.write(
        "src/domain/mod.rs",
        "pub fn rule() { crate::ui::inner::show(); }\n",
    );

    let run = tree.run(&["layering"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("domain → ui: src/ui/mod.rs::inner"), "{}", run.out);
}

#[test]
fn a_typescript_re_export_is_a_dependency() {
    let tree = Tree::new();
    tree.write("klin.json", WEB);
    tree.write("web/view/render.ts", "export const render = 1;\n");
    tree.write(
        "web/model/index.ts",
        "export { render } from \"../view/render\";\n",
    );

    let run = tree.run(&["layering"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("model → view: web/view/render.ts"), "{}", run.out);
}

#[test]
fn a_package_renamed_with_its_manifest_keeps_its_base_debt() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{"layering":{"layers":{"x":{"in":["lib/src/a.rs","pkg/src/a.rs"],"can_use":[]},"y":{"in":["lib/src/b.rs","pkg/src/b.rs"],"can_use":[]}}}}"#,
    );
    tree.write(
        "lib/Cargo.toml",
        &format!("{PACKAGE}[lib]\npath = \"src/root.rs\"\n"),
    );
    tree.write("lib/src/root.rs", "mod a;\nmod b;\n");
    tree.write("lib/src/a.rs", "pub fn f() { crate::b::g(); }\n");
    tree.write("lib/src/b.rs", "pub fn g() {}\n");
    tree.base();
    tree.git(&["mv", "lib", "pkg"]);

    let run = changed(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("the base already held"), "{}", run.out);
}

#[test]
fn a_missing_target_root_a_manifest_names_is_unresolved_whatever_the_scope() {
    let tree = Tree::new();
    two_layers(&tree, "pub fn rule() {}\n");
    tree.write(
        "klin.json",
        r#"{"layering":{"in":"src","layers":{"ui":{"in":"src/ui","can_use":[]},"domain":{"in":"src/domain","can_use":[]}}}}"#,
    );
    tree.write(
        "Cargo.toml",
        &format!("{PACKAGE}[lib]\npath = \"src/gone.rs\"\n"),
    );

    let run = tree.run(&["layering"]);

    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("src/gone.rs"), "{}", run.out);
}

#[test]
fn a_module_declaration_is_containment_and_not_a_dependency() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{"layering":{"acyclic":true,"layers":{"outer":{"in":"src/outer.rs","can_use":[]},"inner":{"in":"src/outer","can_use":["outer"]}}}}"#,
    );
    tree.write("src/lib.rs", "mod outer;\n");
    tree.write("src/outer.rs", "mod inner;\npub fn api() {}\n");
    tree.write("src/outer/inner.rs", "pub fn helper() { super::api(); }\n");

    let run = tree.run(&["layering"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("1 dependency site(s) judged, 0 forbidden, 0 cyclic"),
        "{}",
        run.out
    );
}

fn closed_through_a_bare_child(tree: &Tree) {
    tree.write("klin.json", ACYCLIC);
    tree.write("src/lib.rs", "mod a;\nmod b;\n");
    tree.write("src/a.rs", "use crate::b::X;\npub struct Y;\n");
    tree.write("src/b/mod.rs", "mod inner;\npub use inner::X;\n");
    tree.write("src/b/inner.rs", "use crate::a::Y;\npub struct X;\n");
}

#[test]
fn a_cycle_closed_through_a_bare_child_path_is_a_cycle() {
    let tree = Tree::new();
    closed_through_a_bare_child(&tree);

    let run = tree.run(&["layering"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("cycle: src/a.rs") && run.says("src/b/mod.rs → src/b/inner.rs"),
        "{}",
        run.out
    );
}

#[test]
fn a_cycle_the_base_held_through_a_bare_child_path_stays_held() {
    let tree = Tree::new();
    closed_through_a_bare_child(&tree);
    tree.base();

    let run = tree.run(&["layering"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("the base already held"), "{}", run.out);
}

const NESTED: &str = r#"{"layering":{"layers":{"outer":{"in":"src/outer.rs","can_use":[]},"inner":{"in":"src/outer","can_use":["outer"]}}}}"#;

#[test]
fn a_call_through_a_child_the_file_declares_is_a_dependency_on_it() {
    let tree = Tree::new();
    tree.write("klin.json", NESTED);
    tree.write("src/lib.rs", "mod outer;\n");
    tree.write(
        "src/outer.rs",
        "mod inner;\npub fn api() { inner::helper(); }\n",
    );
    tree.write("src/outer/inner.rs", "pub fn helper() {}\n");

    let run = tree.run(&["layering"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("src/outer.rs:2") && run.says("outer → inner: src/outer/inner.rs"),
        "{}",
        run.out
    );
}

#[test]
fn a_first_segment_that_names_no_declared_module_stays_external() {
    let tree = Tree::new();
    tree.write("klin.json", NESTED);
    tree.write("src/lib.rs", "mod outer;\n");
    tree.write(
        "src/outer.rs",
        "mod inner;\nuse other::helper;\npub fn api() { other::inner::helper(); }\n",
    );
    tree.write("src/outer/inner.rs", "pub fn helper() {}\n");

    let run = tree.run(&["layering"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("0 dependency site(s) judged")
            && run.says("1 external or unsupported dependenc(ies)"),
        "{}",
        run.out
    );
}

#[test]
fn a_child_declared_inside_an_inline_module_is_not_reached_from_beside_it() {
    let tree = Tree::new();
    tree.write("klin.json", NESTED);
    tree.write("src/lib.rs", "mod outer;\n");
    tree.write(
        "src/outer.rs",
        "mod wrap {\n    pub mod inner;\n}\npub fn api() { inner::helper(); }\n",
    );
    tree.write("src/outer/wrap/inner.rs", "pub fn helper() {}\n");

    let run = tree.run(&["layering"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("0 dependency site(s) judged"), "{}", run.out);
}

fn a_child_shadowing_a_root_module(tree: &Tree, edition: &str) {
    tree.write(
        "klin.json",
        r#"{"layering":{"layers":{"top":{"in":"src/a.rs","can_use":["low"]},"low":{"in":"src/b","can_use":[]}}}}"#,
    );
    tree.write(
        "Cargo.toml",
        &format!("[package]\nname = \"t\"\nversion = \"0.1.0\"\n{edition}"),
    );
    tree.write("src/lib.rs", "mod a;\nmod b;\n");
    tree.write("src/a.rs", "pub struct X;\n");
    tree.write(
        "src/b/mod.rs",
        "mod a;\npub use a::X;\npub fn g() { a::f(); }\n",
    );
    tree.write("src/b/a.rs", "pub struct X;\npub fn f() {}\n");
}

#[test]
fn a_bare_use_path_in_edition_2015_starts_at_the_crate_root() {
    let tree = Tree::new();
    a_child_shadowing_a_root_module(&tree, "");

    let run = tree.run(&["layering"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("src/b/mod.rs:2") && run.says("low → top: src/a.rs") && run.says("1 new"),
        "{}",
        run.out
    );
}

#[test]
fn a_bare_use_path_from_edition_2018_starts_at_the_declared_child() {
    let tree = Tree::new();
    a_child_shadowing_a_root_module(&tree, "edition = \"2018\"\n");

    let run = tree.run(&["layering"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("0 forbidden"), "{}", run.out);
}

#[test]
fn a_module_declared_inside_a_function_is_not_reached_by_a_bare_path_beside_it() {
    let tree = Tree::new();
    tree.write("klin.json", ACYCLIC);
    tree.write("src/lib.rs", "mod a;\n");
    tree.write(
        "src/a.rs",
        "pub fn api() {\n    mod inner {\n        pub fn helper() { super::other(); }\n    }\n}\npub fn other() { inner::helper(); }\n",
    );

    let run = tree.run(&["layering"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("0 cyclic"), "{}", run.out);
}

#[test]
fn a_module_declared_inside_a_constant_initializer_is_not_reached_by_a_bare_path() {
    let tree = Tree::new();
    tree.write("klin.json", ACYCLIC);
    tree.write("src/lib.rs", "mod a;\n");
    tree.write(
        "src/a.rs",
        "const _: () = {\n    mod inner {\n        pub fn helper() { crate::a::other(); }\n    }\n};\npub fn other() { inner::helper(); }\n",
    );

    let run = tree.run(&["layering"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("0 cyclic"), "{}", run.out);
}

#[test]
fn a_child_of_a_block_local_module_is_reached_from_that_module() {
    let tree = Tree::new();
    tree.write("klin.json", ACYCLIC);
    tree.write("src/lib.rs", "mod a;\n");
    tree.write(
        "src/a.rs",
        "const _: () = {\n    mod outer {\n        mod inner {\n            pub fn helper() {}\n        }\n        pub fn call() { inner::helper(); }\n    }\n};\n",
    );

    let run = tree.run(&["layering"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("1 dependency site(s) judged"), "{}", run.out);
}

fn renamed_layers(tree: &Tree) {
    tree.write("klin.json", LAYERS);
    tree.write("src/lib.rs", "mod domain;\nmod ui;\n");
    tree.write("src/domain/mod.rs", "mod rules;\npub fn rule() {}\n");
    tree.write(
        "src/domain/rules.rs",
        "pub fn check() { crate::ui::show(); }\n",
    );
    tree.write("src/ui/mod.rs", "mod widget;\npub fn show() {}\n");
    tree.write("src/ui/widget.rs", "pub fn draw() { crate::ui::show(); }\n");
    tree.base();
}

#[test]
fn a_file_renamed_inside_its_layer_keeps_its_base_debt() {
    let tree = Tree::new();
    renamed_layers(&tree);
    tree.git(&["mv", "src/domain/rules.rs", "src/domain/checks.rs"]);
    tree.write("src/domain/mod.rs", "mod checks;\npub fn rule() {}\n");

    let run = changed(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn a_file_renamed_into_another_layer_is_placed_in_its_base_layer_at_the_base() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{"layering":{"layers":{"view":{"in":"web/view","can_use":["shared"]},"model":{"in":"web/model","can_use":[]},"shared":{"in":"web/shared","can_use":[]}}}}"#,
    );
    tree.write("web/shared/s.ts", "export const s = 1;\n");
    tree.write("web/model/m.ts", "export const m = 1;\n");
    tree.write(
        "web/view/widget.ts",
        "import { s } from \"../shared/s\";\nexport const widget = s;\n",
    );
    tree.base();
    tree.git(&["mv", "web/view/widget.ts", "web/model/widget.ts"]);

    let run = changed(&tree);
    let strict = tree.run(&["gate", "--strict", "--gate", "layering"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("web/model/widget.ts:1") && run.says("model → shared: web/shared/s.ts"),
        "{}",
        run.out
    );
    assert_eq!(strict.code, 1, "{}", strict.out);
}

#[test]
fn a_changed_run_beside_a_gate_that_lays_out_changed_files_judges_the_whole_base() {
    let tree = Tree::new();
    two_layers(&tree, "use crate::ui::show;\npub fn rule() { show(); }\n");
    tree.base();
    tree.write("src/ui/mod.rs", "pub fn show() {}\npub fn more() {}\n");

    let run = tree.run(&[
        "gate",
        "--changed",
        "--gate",
        "layering",
        "--gate",
        "dead-symbols",
    ]);

    assert!(run.says("ok    layering"), "{}", run.out);
}

#[test]
fn a_cached_changed_run_reads_and_parses_only_the_changed_file() {
    let tree = Tree::new();
    two_layers(&tree, "pub fn rule() {}\n");
    tree.base();
    tree.write("src/ui/more.rs", "pub fn more() {}\n");

    let run = || tree.run(&["gate", "--json", "--changed", "--gate", "layering"]);
    let (first, again) = (run().json(), run().json());
    let counted = |report: &serde_json::Value| {
        ["reads", "parses", "extracted", "cached"].map(|field| {
            report["gates"][0]["facts"][field]
                .as_u64()
                .unwrap_or(u64::MAX)
        })
    };

    assert_eq!(counted(&first), [4, 4, 4, 0], "{first}");
    assert_eq!(counted(&again), [1, 1, 1, 3], "{again}");
    assert_eq!(first["status"], again["status"]);
    assert!(
        again["gates"][0]["graph"]["modules"].as_u64() >= Some(6),
        "{again}"
    );
}

#[test]
fn an_accepted_forbidden_edge_is_held() {
    let tree = Tree::new();
    two_layers(&tree, "pub fn rule() { crate::ui::show(); }\n");
    tree.write(
        "klin.json",
        r#"{"layering":{"layers":{"ui":{"in":"src/ui","can_use":["domain"]},"domain":{"in":"src/domain","can_use":[]}}},"accepted":[{"gate":"layering","file":"src/domain/mod.rs","text":"domain → ui: src/ui/mod.rs","edge":1}]}"#,
    );

    let run = tree.run(&["layering"]);

    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn without_a_section_the_gate_needs_one_a_person_writes() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write("src/lib.rs", "pub fn one() {}\n");
    tree.base();

    let run = tree.run(&["gate", "--list"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("layering — needs a section a person writes"),
        "{}",
        run.out
    );
}

#[test]
fn a_tree_with_no_typescript_path_dispatches_only_the_rust_resolver() {
    let tree = Tree::new();
    two_layers(&tree, "use crate::ui::show;\npub fn rule() { show(); }\n");

    let report = tree.run(&["gate", "--json", "--gate", "layering"]).json();
    let graph = &report["gates"][0]["graph"];

    assert_eq!(
        graph["dispatches"],
        serde_json::json!({"rust": 1, "typescript": 0}),
        "{report}"
    );
    assert_eq!(graph["sources"], graph["modules"], "{report}");
    assert_eq!(graph["edges"], 1, "{report}");
}

#[test]
fn rust_source_the_grammar_rejects_still_dispatches_the_rust_resolver() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{"layering":{"layers":{"core":{"in":"src"}}}}"#,
    );
    tree.write("src/lib.rs", "fn broken( {\n");

    let run = tree.run(&["gate", "--json", "--gate", "layering"]);
    let report = run.json();

    assert_eq!(
        report["gates"][0]["graph"]["dispatches"]["rust"], 1,
        "{report}"
    );
    assert_eq!(report["gates"][0]["graph"]["modules"], 1, "{report}");
    assert!(
        tree.run(&["layering"])
            .says("1 file(s) attached, 0 by a Cargo manifest and 1 by a conventional root"),
        "{report}"
    );
}

#[test]
fn typescript_source_the_grammar_rejects_still_dispatches_the_typescript_resolver() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{"layering":{"layers":{"web":{"in":"web"}}}}"#,
    );
    tree.write("web/a.ts", "export function broken( {\n");

    let report = tree.run(&["gate", "--json", "--gate", "layering"]).json();

    assert_eq!(
        report["gates"][0]["graph"]["dispatches"],
        serde_json::json!({"rust": 0, "typescript": 1}),
        "{report}"
    );
    assert_eq!(report["gates"][0]["graph"]["modules"], 1, "{report}");
}

#[test]
fn a_file_two_targets_reach_that_swaps_what_each_target_reaches_is_new() {
    let tree = Tree::new();
    two_layers(&tree, "pub fn rule() { crate::ui::show(); }\n");
    tree.write("Cargo.toml", PACKAGE);
    tree.write(
        "src/main.rs",
        "mod domain;\n#[path = \"ui/other.rs\"]\nmod ui;\nfn main() {}\n",
    );
    tree.write("src/ui/other.rs", "pub fn show() {}\n");
    tree.base();
    tree.write(
        "src/lib.rs",
        "mod domain;\n#[path = \"ui/other.rs\"]\nmod ui;\n",
    );
    tree.write("src/main.rs", "mod domain;\nmod ui;\nfn main() {}\n");

    let run = tree.run(&["layering"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("2 new"), "{}", run.out);
}

#[test]
fn an_accepted_edge_does_not_follow_its_dependency_to_another_file() {
    let tree = Tree::new();
    two_layers(&tree, "mod rules;\npub fn rule() {}\n");
    tree.write(
        "src/domain/rules.rs",
        "pub fn check() { crate::ui::show(); }\n",
    );
    tree.write(
        "klin.json",
        r#"{"layering":{"layers":{"ui":{"in":"src/ui","can_use":["domain"]},"domain":{"in":"src/domain","can_use":[]}}},"accepted":[{"gate":"layering","file":"src/domain/rules.rs","text":"domain → ui: src/ui/mod.rs","edge":1}]}"#,
    );
    tree.base();
    tree.write("src/domain/rules.rs", "pub fn check() {}\n");
    tree.write(
        "src/domain/mod.rs",
        "mod rules;\npub fn rule() { crate::ui::show(); }\n",
    );

    let run = tree.run(&["layering", "--strict"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("1 new"), "{}", run.out);
    assert!(run.says("matched nothing"), "{}", run.out);
}
