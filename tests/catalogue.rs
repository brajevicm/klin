mod harness;

use harness::Tree;

/// The two checks that run only inside a gate and have no command of their own. A check added
/// here is a decision a person makes, so a new check that forgot its Clap command fails the
/// test below rather than passing in silence. ADR 0036.
const ONLY_IN_A_GATE: &[&str] = &["inventory", "lockfile"];

/// The commands that are not checks: the runner, the survey, the stamp movers and the readers.
const TOOLS: &[&str] = &[
    "gate",
    "init",
    "guard",
    "cache",
    "radius",
    "turn",
    "stats",
    "reference",
    "update",
    "help",
];

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

/// Every check the catalogue holds, read off the error the runner prints when a written
/// configuration names no gate over a tree the survey finds nothing in.
fn catalogue() -> Vec<String> {
    let tree = Tree::new();
    tree.write("klin.json", r#"{"project":"catalogue"}"#);
    let run = tree.run(&["gate"]);
    assert_eq!(run.code, 2, "{}", run.out);
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
fn every_check_command_has_a_catalogue_row() {
    let commands = subcommands();
    let catalogue = catalogue();

    let checks: Vec<&String> = commands
        .iter()
        .filter(|name| !TOOLS.contains(&name.as_str()))
        .collect();
    assert!(!checks.is_empty(), "no check subcommands in {commands:?}");
    for command in checks {
        assert!(
            catalogue.contains(command),
            "the command {command} is in the CLI and not in the catalogue: {catalogue:?}"
        );
    }
}

#[test]
fn every_catalogue_check_has_a_command_or_is_named_as_gate_only() {
    let commands = subcommands();

    for check in catalogue() {
        assert!(
            commands.contains(&check) || ONLY_IN_A_GATE.contains(&check.as_str()),
            "the check {check} is in the catalogue with no CLI command, and this test does not \
             name it as one that runs only inside a gate"
        );
    }
}

#[test]
fn every_catalogue_section_is_a_key_the_configuration_accepts() {
    let tree = Tree::new();
    tree.words("README.md", 5);
    let mut config = String::from("{\"doc_size\": [{\"file\": \"README.md\", \"ceiling\": 10}]");
    for section in sections() {
        if section != "doc_size" {
            config += &format!(", \"{section}\": false");
        }
    }
    config += "}";
    tree.write("klin.json", &config);

    let run = tree.run(&["gate", "--list"]);

    assert_eq!(run.code, 0, "{config}\n{}", run.out);
    assert!(!run.says("not a key klin reads"), "{}", run.out);
    assert!(run.says("doc-size — runs"), "{}", run.out);
}

/// Every section `klin reference` prints under one of its headings, which it prints off the
/// same catalogue. The page names a section twice, once under `## Sections` with its keys and
/// once under built-in language coverage with its file sets, so each heading is read on its own.
fn printed_under(heading: &str) -> Vec<String> {
    let tree = Tree::bare();
    let run = tree.run(&["reference"]);
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

        let run = tree.run(&["gate"]);

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

    let run = tree.run(&["gate", "--list"]);

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
