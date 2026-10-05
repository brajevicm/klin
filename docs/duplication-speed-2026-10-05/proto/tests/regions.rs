use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Tree(PathBuf);

impl Tree {
    fn new(extension: &str, left: &str, right: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "dup-regions-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(path.join("source")).unwrap();
        std::fs::write(path.join(format!("source/a.{extension}")), left).unwrap();
        std::fs::write(path.join(format!("source/b.{extension}")), right).unwrap();
        Self(path)
    }

    fn run(&self, command: &str, arguments: &[&str]) -> String {
        let output = Command::new(env!("CARGO_BIN_EXE_dup-speed"))
            .arg(command)
            .arg(self.0.join("source"))
            .arg(self.0.join("index"))
            .args(arguments)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    }
}

impl Drop for Tree {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

fn count(output: &str, field: &str) -> usize {
    output
        .split(&format!("\"{field}\":"))
        .nth(1)
        .unwrap()
        .split([',', '}'])
        .next()
        .unwrap()
        .parse()
        .unwrap()
}

#[test]
fn short_functions_obey_the_region_threshold() {
    for (extension, source) in [
        ("ts", "function alpha(x: number) { return x + 1; }"),
        ("rs", "fn alpha(x: u32) -> u32 { x + 1 }"),
    ] {
        for right in [source.to_owned(), source.replace("alpha", "beta")] {
            let tree = Tree::new(extension, source, &right);
            let build = tree.run("build", &[]);
            assert!(!build.contains("function_bytes"));
            for threshold in ["60", "80", "100"] {
                for changed in ["1", "2"] {
                    let query = tree.run("query", &[changed, threshold]);
                    assert_eq!(count(&query, "proven"), 0);
                    assert_eq!(count(&query, "check_blocked"), 0);
                    assert!(!query.contains("function_hits"));
                }
            }
        }
    }
}

#[test]
fn names_remain_part_of_the_region_stream() {
    let source = "function alpha(x: number) { return x + 1; }";
    // k=w=1 exposes all tokens, so an exact 14-token function reaches T=14.
    for (right, expected) in [(source.to_owned(), 1), (source.replace("alpha", "beta"), 0)] {
        let tree = Tree::new("ts", source, &right);
        tree.run("build", &["1", "1", "64", "0"]);
        for changed in ["1", "2"] {
            let query = tree.run("query", &[changed, "14"]);
            assert_eq!(count(&query, "proven"), expected);
            assert_eq!(count(&query, "check_blocked"), expected);
        }
    }
}

#[test]
fn old_indexes_require_a_rebuild() {
    let tree = Tree::new("ts", "const a = 1;", "const b = 2;");
    tree.run("build", &[]);
    let path = tree.0.join("index/index.bin");
    let mut bytes = std::fs::read(&path).unwrap();
    bytes[..4].copy_from_slice(&41u32.to_le_bytes());
    std::fs::write(path, bytes).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_dup-speed"))
        .arg("query")
        .arg(tree.0.join("source"))
        .arg(tree.0.join("index"))
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("rebuild the index"));
}

#[test]
fn long_functions_and_renamed_bodies_are_found_as_regions() {
    for (extension, source) in [
        (
            "ts",
            format!(
                "function alpha(x: number) {{ {} return x; }}",
                (0..40).map(|i| format!("x += {i}; ")).collect::<String>()
            ),
        ),
        (
            "rs",
            format!(
                "fn alpha(mut x: u32) -> u32 {{ {} x }}",
                (0..40).map(|i| format!("x += {i}; ")).collect::<String>()
            ),
        ),
    ] {
        for right in [source.clone(), source.replace("alpha", "beta")] {
            let tree = Tree::new(extension, &source, &right);
            tree.run("build", &[]);
            for changed in ["1", "2"] {
                let query = tree.run("query", &[changed, "100"]);
                assert!(count(&query, "proven") > 0);
                assert_eq!(count(&query, "false_regions"), 0);
            }
        }
    }
}

#[test]
fn independent_text_oracle_and_parser_probes() {
    let output = Command::new("python3")
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/../verify.py"))
        .env("DUP_SPEED", env!("CARGO_BIN_EXE_dup-speed"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("69 oracle/probe cases passed"));
}
