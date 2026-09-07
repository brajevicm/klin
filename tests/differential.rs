mod harness;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use harness::{Run, Tree};
use serde_json::Value;

const BEFORE: &str = include_str!("fixtures/differential/before.py");
const SHIFTED: &str = include_str!("fixtures/differential/shifted.py");
const MORE: &str = include_str!("fixtures/differential/more.py");
const FEWER: &str = include_str!("fixtures/differential/fewer.py");
const INLINE_TESTS: &str = include_str!("fixtures/differential/inline_tests.rs");

const MARKERS: &[&str] = &["OK:", "FAIL:", "WARN:", "NOTE:"];
const SCRIPTS: &[&str] = &["check-doc-size.py", "check-escapes.py"];

#[test]
fn a_shifted_declaration_keeps_its_site_in_both() {
    let Some(cleat) = cleat() else { return };
    let tree = Tree::new();
    configure(&tree, "python");
    tree.write("src/sites.py", BEFORE);
    accept(&tree, &cleat);
    tree.write("src/sites.py", SHIFTED);
    agree(&tree, &cleat);
}

#[test]
fn two_identical_declarations_ratchet_as_one_count_in_both() {
    let Some(cleat) = cleat() else { return };
    let tree = Tree::new();
    configure(&tree, "python");
    tree.write("src/sites.py", BEFORE);
    accept(&tree, &cleat);
    tree.write("src/sites.py", MORE);
    agree(&tree, &cleat);
}

#[test]
fn a_baseline_looser_than_the_code_reads_the_same_to_both() {
    let Some(cleat) = cleat() else { return };
    let tree = Tree::new();
    configure(&tree, "python");
    tree.write("src/sites.py", BEFORE);
    accept(&tree, &cleat);
    tree.write("src/sites.py", FEWER);
    agree(&tree, &cleat);
}

#[test]
fn an_unchanged_rust_tree_holds_the_same_way_in_both() {
    let Some(cleat) = cleat() else { return };
    let tree = Tree::new();
    configure(&tree, "rust");
    tree.write("src/lib.rs", INLINE_TESTS);
    accept(&tree, &cleat);
    agree(&tree, &cleat);
}

fn cleat() -> Option<PathBuf> {
    let pointed_at = std::env::var("CLEAT_SRC");
    let root = match &pointed_at {
        Ok(named) => PathBuf::from(named),
        Err(_) => Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("cleat"),
    };
    let bin = root.join("quality").join("bin");
    if let Some(script) = SCRIPTS.iter().find(|script| !bin.join(script).is_file()) {
        let missing = format!("{} holds no {script}", bin.display());
        if pointed_at.is_ok() {
            panic!("CLEAT_SRC names no cleat checkout: {missing}");
        }
        return skipped(&format!("{missing} — set CLEAT_SRC to a cleat checkout"));
    }
    if Command::new("python3").arg("--version").output().is_err() {
        return skipped("no python3 on PATH, and cleat is a Python program");
    }
    Some(bin)
}

fn skipped(why: &str) -> Option<PathBuf> {
    static ANNOUNCED: std::sync::Once = std::sync::Once::new();
    ANNOUNCED.call_once(|| {
        let mut loud = std::io::stderr();
        let _ = writeln!(
            loud,
            "\nSKIPPED the differential test against cleat: {why}.\n"
        );
    });
    None
}

fn configure(tree: &Tree, language: &str) {
    let config = |baseline: &str| {
        format!(
            r#"{{"escapes": {{"roots": ["src"], "languages": ["{language}"],
                              "baseline": "{baseline}"}},
                "doc_size": [{{"file": "short.md", "ceiling": 10}},
                             {{"file": "near.md", "ceiling": 100}},
                             {{"file": "long.md", "ceiling": 100}}]}}"#
        )
    };
    tree.write("quality.json", &config("escapes-baseline.json"));
    tree.write("cleat-quality.json", &config("cleat-escapes-baseline.json"));
    tree.words("short.md", 5);
    tree.words("near.md", 99);
    tree.words("long.md", 101);
}

fn accept(tree: &Tree, cleat: &Path) {
    let mine = tree.run(&["escapes", "--config", "quality.json", "--write-baseline"]);
    assert_eq!(mine.code, 0, "{}", mine.out);
    let theirs = cleat_run(
        tree,
        cleat,
        "check-escapes.py",
        &["--config", "cleat-quality.json", "--write-baseline"],
    );
    assert_eq!(theirs.code, 0, "{}", theirs.out);
    let mine = json(&tree.path("escapes-baseline.json"));
    let theirs = json(&tree.path("cleat-escapes-baseline.json"));
    assert_eq!(mine["entries"], theirs["entries"]);
    assert_eq!(mine["provenance"]["tool"], theirs["provenance"]["tool"]);
    assert_eq!(
        mine["provenance"]["version"],
        theirs["provenance"]["version"]
    );
}

fn agree(tree: &Tree, cleat: &Path) {
    for extra in [Vec::new(), vec!["--strict"]] {
        let mut mine = vec!["escapes", "--config", "quality.json"];
        mine.extend(extra.iter().copied());
        let mut theirs = vec!["--config", "cleat-quality.json"];
        theirs.extend(extra.iter().copied());
        let mine = tree.run(&mine);
        let theirs = cleat_run(tree, cleat, "check-escapes.py", &theirs);
        same_verdict(&mine, &theirs, kinds);
    }
    let mine = tree.run(&["doc-size", "--config", "quality.json"]);
    let theirs = cleat_run(
        tree,
        cleat,
        "check-doc-size.py",
        &["--config", "cleat-quality.json"],
    );
    same_verdict(&mine, &theirs, markers);
}

fn same_verdict(mine: &Run, theirs: &Run, headlines: fn(&str) -> Vec<&str>) {
    let why = format!("detent said:\n{}\ncleat said:\n{}", mine.out, theirs.out);
    assert_eq!(mine.code, theirs.code, "{why}");
    assert_eq!(rows(&mine.out), rows(&theirs.out), "{why}");
    assert_eq!(headlines(&mine.out), headlines(&theirs.out), "{why}");
}

fn rows(out: &str) -> Vec<&str> {
    out.lines().filter(|line| line.starts_with("  ")).collect()
}

fn markers(out: &str) -> Vec<&str> {
    out.lines()
        .filter(|line| MARKERS.iter().any(|marker| line.starts_with(marker)))
        .collect()
}

fn kinds(out: &str) -> Vec<&str> {
    markers(out)
        .into_iter()
        .filter_map(|line| line.split_once(':'))
        .map(|(kind, _)| kind)
        .collect()
}

fn cleat_run(tree: &Tree, cleat: &Path, script: &str, args: &[&str]) -> Run {
    let done = Command::new("python3")
        .arg(cleat.join(script))
        .args(args)
        .current_dir(tree.root())
        .output();
    let Ok(done) = done else {
        panic!("cleat's {script} could not run")
    };
    let Some(code) = done.status.code() else {
        panic!("cleat's {script} was killed")
    };
    Run {
        code,
        out: String::from_utf8_lossy(&done.stdout).to_string()
            + &String::from_utf8_lossy(&done.stderr),
    }
}

fn json(path: &Path) -> Value {
    let Ok(text) = std::fs::read_to_string(path) else {
        panic!("{} could not be read", path.display())
    };
    match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(why) => panic!("{} is not JSON: {why}", path.display()),
    }
}
