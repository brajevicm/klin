mod harness;

use harness::Tree;

fn printed() -> String {
    let tree = Tree::bare();
    let run = tree.run(&["reference"]);
    assert_eq!(run.code, 0, "{}", run.out);
    run.printed
}

#[test]
fn the_reference_names_every_top_level_key_and_every_section() {
    let out = printed();

    for key in [
        "`project`",
        "`version`",
        "`build`",
        "`accepted`",
        "`radius`",
        "`gates`",
    ] {
        assert!(out.contains(key), "no {key} in: {out}");
    }
    for section in [
        "### `complexity`",
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
        out.contains("| `roots` | the directories the check reads | yes | derived when absent"),
        "no derived roots row in: {out}"
    );
    assert!(
        out.contains(
            "| `exclude_except` | the files an `exclude` glob must not drop | no | \
                      pinned only"
        ),
        "no pinned-only exclude_except row in: {out}"
    );
    assert!(
        out.contains("95th percentile"),
        "no ceiling derivation rule in: {out}"
    );
    assert!(
        out.contains(
            "| `ceiling` | the words the document may not pass | yes | derived with the section"
        ),
        "no derived-with-the-section row in: {out}"
    );
}

#[test]
fn the_reference_states_the_language_names_and_the_exclusion_facts() {
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
        out.contains("`skip_dirs` adds to that list"),
        "no skip_dirs fact in: {out}"
    );
    assert!(
        out.contains("It cannot bring back a file under a skipped directory"),
        "no exclude_except fact in: {out}"
    );
    assert!(
        out.contains("Only `complexity` reads `exclude_except`"),
        "no exclude_except scope fact in: {out}"
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
