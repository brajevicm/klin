mod harness;

use harness::Tree;

/// The commands `klin --help` offers, which hold no check of their own. ADR 0066.
const COMMANDS: &[&str] = &["setup", "check", "policy", "report", "update", "help"];

/// Every subcommand `klin --help` offers. A command sits on a line indented by exactly two
/// spaces, so a description that wrapped onto its own deeper-indented line is not one.
fn subcommands() -> Vec<String> {
    let tree = Tree::bare();
    let run = tree.run(&["--help"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let Some((_, listed)) = run.out.split_once("Commands:\n") else {
        panic!("no command list in: {}", run.out);
    };
    listed
        .lines()
        .take_while(|line| !line.trim().is_empty())
        .filter(|line| line.starts_with("  ") && !line.starts_with("   "))
        .filter_map(|line| line.split_whitespace().next())
        .map(str::to_string)
        .collect()
}

/// Every check the catalogue holds, read off the hole the runner prints when a written
/// configuration names no gate over a tree the survey finds nothing in.
fn catalogue() -> Vec<String> {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    let run = tree.run(&["check"]);
    assert_eq!(run.code, 3, "{}", run.out);
    let Some((_, listed)) = run.out.split_once("one of: ") else {
        panic!("no check list in: {}", run.out);
    };
    listed
        .lines()
        .next()
        .unwrap_or_default()
        .split(", ")
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty())
        .collect()
}

#[test]
fn help_lists_exactly_the_public_commands() {
    assert_eq!(subcommands(), COMMANDS);
}

#[test]
fn every_catalogue_check_is_a_name_check_takes_or_one_the_policy_lists() {
    let tree = Tree::new();
    tree.words("README.md", 5);
    tree.write("src/lib.rs", "pub fn one(a: i32) -> i32 {\n    a + 1\n}\n");
    tree.write(
        "Cargo.toml",
        "[package]\nname = \"t\"\nversion = \"0.1.0\"\n",
    );
    tree.write("Cargo.lock", "version = 3\n");
    tree.base();
    let policy = tree.run(&["policy"]);
    assert_eq!(policy.code, 0, "{}", policy.out);

    for check in catalogue() {
        let run = tree.run(&["check", &check]);
        assert!(
            !run.says(&format!("no gate named {check}")) || policy.says(&format!("{check} — ")),
            "check {check} is neither taken by `klin check` nor listed by `klin policy`: {}\n{}",
            run.out,
            policy.out
        );
    }
}

#[test]
fn every_catalogue_section_is_a_key_the_configuration_accepts() {
    let tree = Tree::new();
    tree.words("README.md", 5);
    let mut config = String::from("{\"doc_size\": {\"README.md\": 10}");
    for section in sections() {
        if section != "doc_size" {
            config += &format!(", \"{section}\": false");
        }
    }
    config += "}";
    tree.write("klin.json", &config);

    let run = tree.run(&["policy"]);

    assert_eq!(run.code, 0, "{config}\n{}", run.out);
    assert!(!run.says("not a key klin reads"), "{}", run.out);
    assert!(run.says("doc-size — runs"), "{}", run.out);
}

/// Every section `klin policy --reference` prints under one of its headings, which it prints off the
/// same catalogue. The page names a section twice, once under `## Sections` with its keys and
/// once under built-in language coverage with its file sets, so each heading is read on its own.
fn printed_under(heading: &str) -> Vec<String> {
    let tree = Tree::bare();
    let run = tree.run(&["policy", "--reference"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let Some((_, rest)) = run.printed.split_once(heading) else {
        panic!("no {heading} in: {}", run.printed);
    };
    rest.lines()
        .take_while(|line| !line.starts_with("## "))
        .filter_map(|line| line.strip_prefix("### `")?.strip_suffix('`'))
        .map(str::to_string)
        .collect()
}

fn sections() -> Vec<String> {
    printed_under("\n## Sections\n")
}

#[test]
fn a_section_named_by_its_command_is_corrected_to_the_section_it_reads() {
    let sections = sections();
    let mut corrected = 0;
    for check in catalogue() {
        let section = check.replace('-', "_");
        if section == check {
            continue;
        }
        assert!(
            sections.contains(&section),
            "the check {check} reads {section}, which the reference does not print: {sections:?}"
        );
        corrected += 1;
        let tree = Tree::new();
        tree.words("README.md", 5);
        tree.write("klin.json", &format!("{{\"{check}\": false}}"));

        let run = tree.run(&["check"]);

        assert_eq!(run.code, 2, "{check}: {}", run.out);
        assert!(
            run.says(&format!(
                "\"{check}\" is what the command is called — the section it reads is \"{section}\""
            )),
            "{check}: {}",
            run.out
        );
    }
    assert!(corrected > 0, "no check has a name that is not its section");
}

#[test]
fn every_catalogue_check_has_a_section_the_reference_prints() {
    let sections = sections();
    let catalogue = catalogue();

    assert_eq!(
        sections.len(),
        catalogue.len(),
        "the reference prints {sections:?} and the catalogue holds {catalogue:?}"
    );
    for check in &catalogue {
        let section = check.replace('-', "_");
        assert!(
            sections.contains(&section),
            "the check {check} reads {section}, which the reference does not print: {sections:?}"
        );
    }
}

#[test]
fn every_language_table_belongs_to_a_section_the_reference_names() {
    let sections = sections();
    let named = printed_under("\n## Built-in language coverage\n");

    assert!(!named.is_empty(), "no language table in: {sections:?}");
    for section in &named {
        assert!(
            sections.contains(section),
            "the reference prints a language table for {section}, which it names no section for: \
             {sections:?}"
        );
    }
}

#[test]
fn every_catalogue_check_is_accounted_for_in_the_plan() {
    let tree = Tree::new();
    tree.words("README.md", 5);
    tree.write("src/lib.rs", "pub fn one(a: i32) -> i32 {\n    a + 1\n}\n");
    tree.write(
        "Cargo.toml",
        "[package]\nname = \"t\"\nversion = \"0.1.0\"\n",
    );
    tree.write("Cargo.lock", "version = 3\n");
    tree.base();

    let run = tree.run(&["policy"]);

    assert_eq!(run.code, 0, "{}", run.out);
    for check in catalogue() {
        assert!(
            run.says(&format!("{check} — runs"))
                || run.says(&format!("{check} — excluded"))
                || run.says(&format!("{check} — needs a section a person writes")),
            "the plan says nothing about {check}: {}",
            run.out
        );
    }
    assert!(
        run.says("sarif — needs a section a person writes"),
        "the survey derives no sarif section, so the plan must ask a person for one: {}",
        run.out
    );
}

/// The gates one run executed, in the order the report names them.
fn executed(tree: &Tree) -> Vec<String> {
    let run = tree.run(&["check", "--json"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let report = run.json();
    let Some(gates) = harness::gate_rows(&report).as_array() else {
        panic!("no gate list in: {}", run.out);
    };
    gates
        .iter()
        .filter_map(|gate| gate.get("name")?.as_str())
        .map(str::to_string)
        .collect()
}

#[test]
fn a_run_executes_its_gates_in_catalogue_order() {
    let tree = Tree::new();
    tree.words("README.md", 5);
    tree.write("src/lib.rs", "pub fn one(a: i32) -> i32 {\n    a + 1\n}\n");
    tree.write(
        "Cargo.toml",
        "[package]\nname = \"t\"\nversion = \"0.1.0\"\n",
    );
    tree.write("Cargo.lock", "version = 3\n");
    tree.base();

    let declared = catalogue();
    let ran: Vec<String> = executed(&tree)
        .into_iter()
        .filter(|name| declared.contains(name))
        .collect();

    assert!(
        ran.len() > 3,
        "too few gates ran to prove an order: {ran:?}"
    );
    let mut left = declared.iter();
    for name in &ran {
        assert!(
            left.any(|declared| declared == name),
            "{name} ran out of catalogue order: ran {ran:?}, catalogue {declared:?}"
        );
    }
}

#[test]
fn a_per_entry_check_plans_its_gates_in_the_order_the_section_lists_them() {
    let tree = Tree::new();
    tree.write("src/lib.rs", "pub fn one(a: i32) -> i32 {\n    a + 1\n}\n");
    tree.write(
        "klin.json",
        r#"{"sarif": [{"name": "zed", "report": "z.sarif"},
                     {"name": "alpha", "report": "a.sarif"}]}"#,
    );
    tree.base();

    let run = tree.run(&["policy"]);

    assert_eq!(run.code, 0, "{}", run.out);
    let Some(zed) = run.out.find("zed — runs") else {
        panic!("no zed gate in: {}", run.out);
    };
    let Some(alpha) = run.out.find("alpha — runs") else {
        panic!("no alpha gate in: {}", run.out);
    };
    let Some(escapes) = run.out.find("escapes — runs") else {
        panic!("no escapes gate in: {}", run.out);
    };
    assert!(zed < alpha, "the section's order was lost: {}", run.out);
    assert!(
        escapes < zed,
        "the entries left the catalogue position of sarif: {}",
        run.out
    );
}
