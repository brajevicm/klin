mod harness;

use harness::Tree;

fn printed() -> String {
    let tree = Tree::bare();
    let run = tree.run(&["policy", "--reference"]);
    assert_eq!(run.code, 0, "{}", run.out);
    run.printed
}

#[test]
fn the_reference_names_every_top_level_key_and_every_section() {
    let out = printed();

    for key in ["`build`", "`accepted`", "`radius`", "`journal`"] {
        assert!(out.contains(key), "no {key} in: {out}");
    }
    for retired in ["`project`", "`version`"] {
        assert!(
            !out.contains(&format!("| {retired} |")),
            "{retired} in: {out}"
        );
    }
    for section in [
        "### `complexity`",
        "### `dead_symbols`",
        "### `escapes`",
        "### `stubs`",
        "### `inventory`",
        "### `doc_size`",
        "### `doc_citations`",
        "### `lockfile`",
        "### `sarif`",
    ] {
        assert!(out.contains(section), "no {section} in: {out}");
    }
}

#[test]
fn every_key_says_whether_it_is_required_and_where_its_value_comes_from() {
    let out = printed();

    assert!(out.contains("| Key | Holds | Required | Source | Derivation rule | Default |"));
    assert!(
        out.contains(
            "| `cc` | the cyclomatic complexity a function may not pass | no | derived when absent"
        ),
        "no derived complexity row in: {out}"
    );
    assert!(
        out.contains("| `in` | a repository-relative path") && out.contains("| no | pinned only"),
        "no pinned-only compact scope row in: {out}"
    );
    assert!(
        out.contains("95th percentile"),
        "no ceiling derivation rule in: {out}"
    );
    assert!(
        out.contains("| `<document path>` | the words the document at that path"),
        "no document ceiling row in: {out}"
    );
    assert!(
        out.contains("### `doc_citations`\n\nNo keys: the section is absent, or `false`"),
        "no keyless doc_citations section in: {out}"
    );
    assert!(
        out.contains("`publish = false` and `\"private\": true` do not make a package not applicable (ADR 0050)"),
        "no public_api publication rule in: {out}"
    );
}

#[test]
fn the_reference_states_built_in_languages_and_compact_scope() {
    let out = printed();

    assert!(
        out.contains("| `typescript` | `.ts`, `.mts`, `.cts`, `.tsx` |"),
        "no complexity typescript row in: {out}"
    );
    assert!(
        out.contains("| `javascript` | `.js`, `.jsx`, `.mjs`, `.cjs` |"),
        "no complexity javascript row in: {out}"
    );
    assert!(
        out.contains("capabilities of the binary, not selectors accepted in `klin.json`"),
        "no built-in language fact in: {out}"
    );
    assert!(
        out.contains("`in` narrows a check to a repository-relative path"),
        "no compact scope fact in: {out}"
    );
    assert!(
        out.contains("reject the retired `roots`, `languages`, `patterns`, `skip_dirs`"),
        "no retired topology migration fact in: {out}"
    );
}

#[test]
fn the_committed_reference_is_what_the_binary_prints() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/REFERENCE.md");
    let committed = std::fs::read_to_string(&path).unwrap_or_default();

    assert_eq!(
        committed,
        printed(),
        "docs/REFERENCE.md is not what `klin reference` prints — run `klin reference > \
         docs/REFERENCE.md`"
    );
}
