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
fn the_reference_prints_one_matrix_of_languages_against_checks() {
    let out = printed();
    let Some((_, coverage)) = out.split_once("\n## Built-in language coverage\n") else {
        panic!("no language coverage in: {out}");
    };
    let matrix: Vec<&str> = coverage
        .lines()
        .skip_while(|line| !line.starts_with("| Language |"))
        .take_while(|line| line.starts_with('|'))
        .collect();

    assert_eq!(
        matrix.first().copied(),
        Some(
            "| Language | `escapes` | `stubs` | `complexity` | `dead_symbols` | `reachability` \
             | `layering` | `public_api` | `conventions` |"
        ),
        "no matrix header in: {coverage}"
    );
    assert!(
        matrix.contains(&"| `shell` | yes | — | — | — | — | — | — | — |"),
        "no shell row naming only escapes in: {coverage}"
    );
    assert!(
        matrix
            .iter()
            .any(|row| row.starts_with("| `tsx` | — | — | yes |")),
        "no tsx row apart from typescript in: {coverage}"
    );
    assert!(
        coverage.find("| Language |") < coverage.find("\n### `escapes`"),
        "the matrix does not come before the per-check tables in: {coverage}"
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
