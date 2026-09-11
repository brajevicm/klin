mod harness;

use harness::{Tree, feed};
use serde_json::{Value, json};
use std::path::Path;
use std::process::Command;
use std::time::Instant;

const ITERATIONS: usize = 5;
const EVENTS: usize = 1_000;

struct Fixture {
    tree: Tree,
    files_per_language: usize,
    tsx: usize,
}

struct Measurements {
    warm: Vec<u128>,
    cold: Vec<u128>,
    strict: Vec<u128>,
}

#[derive(Default)]
struct Counts {
    rust: usize,
    typescript: usize,
    tsx: usize,
}

#[test]
#[ignore = "expensive; run with cargo test -- --ignored perf --nocapture"]
fn performance_fixture() {
    let small = Fixture::new(1_000);
    let small_rows = small.measure();
    let guard_rows = guard(&small.tree);
    print_rows(&small, &small_rows);
    println!(
        "guard 1000 events: cache=separate, iterations={ITERATIONS}, median_ms={}, per_event_ms={:.3}",
        median(&guard_rows),
        median(&guard_rows) as f64 / EVENTS as f64
    );

    let large = Fixture::new(5_000);
    let large_rows = large.measure();
    print_rows(&large, &large_rows);
}

impl Fixture {
    fn new(files_per_language: usize) -> Fixture {
        let tree = Tree::bare();
        tree.repository();
        let tsx = files_per_language / 100;
        tree.write(
            "klin.json",
            &format!(
                r#"{{"project":"performance","version":"{}","build":[],"complexity":{{"languages":["rust","typescript"]}}}}"#,
                env!("CARGO_PKG_VERSION")
            ),
        );
        tree.write(
            "README.md",
            "The Rust tree is `rust/src/module_0004.rs`; the TypeScript tree is `web/src/index.ts`; the manifests are `rust/Cargo.toml` and `web/package.json`.\n",
        );
        tree.write(
            "rust/Cargo.toml",
            "[package]\nname = \"fixture-rust\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        );
        tree.write("rust/Cargo.lock", "version = 3\n");
        tree.write(
            "web/package.json",
            "{\"name\":\"fixture-web\",\"private\":true,\"version\":\"0.1.0\"}\n",
        );
        tree.write(
            "web/package-lock.json",
            "{\"name\":\"fixture-web\",\"version\":\"0.1.0\",\"lockfileVersion\":3,\"packages\":{\"\":{}}}\n",
        );
        tree.write(
            "web/tsconfig.json",
            "{\"compilerOptions\":{\"strict\":true},\"include\":[\"src\"]}\n",
        );
        for index in 0..files_per_language {
            tree.write(&rust_path(index), &rust_source(index, files_per_language));
        }
        for index in 0..files_per_language {
            tree.write(&typescript_path(index, tsx), &typescript_source(index, tsx));
        }
        tree.base();

        let counts = source_counts(tree.root());
        assert_eq!(counts.rust, files_per_language, "Rust source file count");
        assert_eq!(
            counts.typescript, files_per_language,
            "TypeScript source file count"
        );
        assert_eq!(counts.tsx, tsx, "TSX source file count");
        Fixture {
            tree,
            files_per_language,
            tsx,
        }
    }

    fn measure(&self) -> Measurements {
        let primed = self.tree.run(&["radius"]);
        assert_eq!(primed.code, 0, "prime state: {}", primed.out);
        let primed = self.tree.run(&["gate", "--hook", "--changed"]);
        assert_eq!(primed.code, 0, "prime survey: {}", primed.out);
        assert!(primed.out.is_empty(), "prime survey: {}", primed.out);

        self.change();
        let changed = changed_counts(self.tree.root());
        assert_eq!(changed.rust, 10, "changed Rust file count");
        assert_eq!(changed.typescript, 10, "changed TypeScript file count");
        assert_eq!(changed.rust + changed.typescript, 20, "changed file count");

        let warm = repeat(|| {
            let started = Instant::now();
            let run = self.tree.run(&["gate", "--hook", "--changed"]);
            assert_eq!(run.code, 0, "warm hook: {}", run.out);
            assert!(run.out.is_empty(), "warm hook: {}", run.out);
            started.elapsed().as_millis()
        });
        let cold = repeat(|| {
            let cleaned = self.tree.run(&["cache", "clean"]);
            assert_eq!(cleaned.code, 0, "cache clean: {}", cleaned.out);
            let started = Instant::now();
            let _ = strict_run(&self.tree, "cold survey");
            started.elapsed().as_millis()
        });
        let strict = repeat(|| {
            let started = Instant::now();
            let _ = strict_run(&self.tree, "strict");
            started.elapsed().as_millis()
        });
        Measurements { warm, cold, strict }
    }

    fn change(&self) {
        for offset in 0..10 {
            let rust = offset + 4;
            self.tree.write(
                &rust_path(rust),
                &rust_source(rust, self.files_per_language).replace("base", "turn"),
            );
            let typescript = offset + 4 + self.tsx;
            self.tree.write(
                &typescript_path(typescript, self.tsx),
                &typescript_source(typescript, self.tsx).replace("base", "turn"),
            );
        }
    }
}

fn print_rows(fixture: &Fixture, rows: &Measurements) {
    let counts = source_counts(fixture.tree.root());
    let size = fixture.files_per_language * 2;
    println!(
        "fixture {}: rust_files={}, typescript_files={}, tsx_files={}, changed_files=20 (10 rust, 10 typescript)",
        size, counts.rust, counts.typescript, counts.tsx
    );
    println!(
        "{} warm hook: cache=warm, iterations={ITERATIONS}, median_ms={}, project_build=excluded",
        size,
        median(&rows.warm)
    );
    println!(
        "{} cold survey: cache=cold, iterations={ITERATIONS}, median_ms={}",
        size,
        median(&rows.cold)
    );
    println!(
        "{} strict: cache=warm, iterations={ITERATIONS}, median_ms={}",
        size,
        median(&rows.strict)
    );
    println!("note: hook timings exclude the project's build command");
    println!("klin version: {}", env!("CARGO_PKG_VERSION"));
}

fn repeat(mut run: impl FnMut() -> u128) -> Vec<u128> {
    (0..ITERATIONS).map(|_| run()).collect()
}

fn median(values: &[u128]) -> u128 {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    sorted[sorted.len() / 2]
}

fn strict_run(tree: &Tree, label: &str) -> harness::Run {
    let run = tree.run(&["gate", "--strict"]);
    assert_eq!(run.code, 0, "{label}: {}", run.out);
    assert!(run.says("gate(s), all passed."), "{label}: {}", run.out);
    run
}

fn source_counts(root: &Path) -> Counts {
    count_paths(git_paths(root, ["ls-files", "-z"]))
}

fn changed_counts(root: &Path) -> Counts {
    count_paths(git_paths(root, ["diff", "--name-only", "-z"]))
}

fn count_paths(paths: impl IntoIterator<Item = Vec<u8>>) -> Counts {
    paths
        .into_iter()
        .fold(Counts::default(), |mut counts, path| {
            if path.ends_with(b".rs") {
                counts.rust += 1;
            }
            if path.ends_with(b".ts") || path.ends_with(b".tsx") {
                counts.typescript += 1;
            }
            if path.ends_with(b".tsx") {
                counts.tsx += 1;
            }
            counts
        })
}

fn git_paths<const N: usize>(root: &Path, args: [&str; N]) -> Vec<Vec<u8>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .expect("git command");
    assert!(output.status.success(), "git command failed");
    output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn rust_path(index: usize) -> String {
    match index {
        0 => "rust/src/lib.rs".to_string(),
        1 => "rust/src/held_escape.rs".to_string(),
        2 => "rust/src/held_stub.rs".to_string(),
        3 => "rust/tests/fixture_test.rs".to_string(),
        _ => format!("rust/src/module_{index:04}.rs"),
    }
}

fn typescript_path(index: usize, tsx: usize) -> String {
    match index {
        0 => "web/src/index.ts".to_string(),
        1 => "web/src/held_escape.ts".to_string(),
        2 => "web/src/held_stub.ts".to_string(),
        3 => "web/src/fixture.test.ts".to_string(),
        index if index < tsx + 4 => {
            format!("web/src/components/component_{:04}.tsx", index - 4)
        }
        _ => format!("web/src/module_{index:04}.ts"),
    }
}

fn rust_source(index: usize, total: usize) -> String {
    match index {
        0 => {
            "pub mod held_escape;\npub mod held_stub;\npub mod module_0004;\npub mod module_0005;\n"
                .to_string()
        }
        1 => {
            "pub fn held_escape(input: usize) -> usize {\n    Some(input).unwrap()\n}\n".to_string()
        }
        2 => "pub fn held_stub() -> usize {\n    todo!()\n}\n".to_string(),
        3 => "#[test]\nfn test_fixture_contract() {\n    assert_eq!(2 + 2, 4);\n}\n".to_string(),
        _ if index.is_multiple_of(10) => rust_complex(index, total),
        _ => rust_simple(index, total),
    }
}

fn rust_simple(index: usize, total: usize) -> String {
    let dependency = if index == 4 { total - 1 } else { index - 1 };
    format!(
        "use crate::module_{dependency:04}::value_{dependency};\n\npub fn value_{index}(input: usize) -> usize {{\n    input + value_{dependency}(input) + {index}\n}}\n// base\n"
    )
}

fn rust_complex(index: usize, total: usize) -> String {
    let dependency = if index == 4 { total - 1 } else { index - 1 };
    format!(
        "use crate::module_{dependency:04}::value_{dependency};\n\npub fn value_{index}(input: usize) -> usize {{\n    input + value_{dependency}(input) + {index}\n}}\n\npub fn branch_{index}(mut value: usize) -> usize {{\n    if value % 2 == 0 {{\n        value += 1;\n    }} else {{\n        value += 2;\n    }}\n    for step in 0..3 {{\n        if step == 1 {{\n            value += step;\n        }}\n    }}\n    if value % 5 == 0 {{\n        value += 5;\n    }}\n    if value % 7 == 0 {{\n        value += 7;\n    }}\n    match value % 3 {{\n        0 => value,\n        1 => value + 1,\n        _ => value + 2,\n    }}\n}}\n// base\n"
    )
}

fn typescript_source(index: usize, tsx: usize) -> String {
    match index {
        0 => format!(
            "export {{ value_{first:04} }} from \"./module_{first:04}\";\n",
            first = tsx + 4
        ),
        1 => "export function heldEscape(input: unknown): unknown {\n    return input as any;\n}\n"
            .to_string(),
        2 => "export function heldStub(): never {\n    throw new Error(\"not implemented\");\n}\n"
            .to_string(),
        3 => format!(
            "import {{ value_{first:04} }} from \"./module_{first:04}\";\nexport function test_fixture(): number {{\n    return value_{first}(1);\n}}\nit(\"keeps the fixture\", () => value_{first}(1));\n",
            first = tsx + 4
        ),
        index if index < tsx + 4 => format!(
            "export const Component_{index:04} = (input: number) => <span>{{input}}</span>;\n// base\n"
        ),
        _ if index.is_multiple_of(10) => typescript_complex(index, tsx),
        _ => typescript_simple(index, tsx),
    }
}

fn typescript_simple(index: usize, tsx: usize) -> String {
    let first = tsx + 4;
    if index == first {
        format!(
            "export function value_{index}(input: number): number {{\n    return input + {index};\n}}\n// base\n"
        )
    } else {
        let dependency = index - 1;
        format!(
            "import {{ value_{dependency:04} }} from \"./module_{dependency:04}\";\n\nexport function value_{index}(input: number): number {{\n    return input + value_{dependency}(input) + {index};\n}}\n// base\n"
        )
    }
}

fn typescript_complex(index: usize, tsx: usize) -> String {
    let first = tsx + 4;
    let import_line = if index == first {
        String::new()
    } else {
        format!(
            "import {{ value_{dependency:04} }} from \"./module_{dependency:04}\";\n\n",
            dependency = index - 1
        )
    };
    let value = if index == first {
        format!("    return input + {index};")
    } else {
        format!("    return input + value_{}(input) + {index};", index - 1)
    };
    format!(
        "{import_line}export function value_{index}(input: number): number {{\n{value}\n}}\n\nexport function branch_{index}(value: number): number {{\n    let result = value;\n    if (result % 2 === 0) {{\n        result += 1;\n    }} else {{\n        result += 2;\n    }}\n    for (const step of [0, 1, 2]) {{\n        if (step === 1) {{\n            result += step;\n        }}\n    }}\n    if (result % 5 === 0) {{\n        result += 5;\n    }}\n    if (result % 7 === 0) {{\n        result += 7;\n    }}\n    switch (result % 3) {{\n        case 0: return result;\n        case 1: return result + 1;\n        default: return result + 2;\n    }}\n}}\n// base\n"
    )
}

fn guard(tree: &Tree) -> Vec<u128> {
    let events: Vec<String> = (0..EVENTS).map(guard_event).collect();
    repeat(|| {
        let started = Instant::now();
        for (index, event) in events.iter().enumerate() {
            let run = feed(tree.root(), &["guard"], event);
            match index % 10 {
                2 | 9 => {
                    assert_eq!(run.code, 2, "guard deny: {}", run.out);
                    assert!(run.says("refused"), "guard deny: {}", run.out);
                    assert!(run.says("klin.json"), "guard deny: {}", run.out);
                }
                7 => {
                    assert_eq!(run.code, 0, "guard ask: {}", run.out);
                    assert!(run.says("permissionDecision"), "guard ask: {}", run.out);
                    assert!(run.says("klin.json"), "guard ask: {}", run.out);
                }
                _ => {
                    assert_eq!(run.code, 0, "guard allow: {}", run.out);
                    assert!(!run.says("permissionDecision"), "guard allow: {}", run.out);
                }
            }
        }
        started.elapsed().as_millis()
    })
}

fn guard_event(index: usize) -> String {
    let rust = format!("rust/src/module_{:04}.rs", 4 + index % 20);
    let typescript = format!("web/src/module_{:04}.ts", 24 + index % 20);
    match index % 10 {
        0 => claude("Read", json!({"file_path": rust})),
        1 => claude("Write", json!({"file_path": typescript})),
        2 => claude("Edit", json!({"file_path": "klin.json"})),
        3 => claude("Bash", json!({"command": format!("cat \"{rust}\" ")})),
        4 => claude(
            "Bash",
            json!({"command": format!("echo klin.json && cat '{typescript}'")}),
        ),
        5 => claude("Bash", json!({"command": "rm *.json"})),
        6 => claude(
            "Bash",
            json!({"command": "cat <<'EOF' > klin.json\n{}\nEOF"}),
        ),
        7 => claude("Bash", json!({"command": "rm \"klin.json\""})),
        8 => codex(
            "apply_patch",
            format!(
                "*** Begin Patch\n*** Update File: {rust}\n*** Update File: {typescript}\n@@\n+changed\n*** End Patch"
            ),
        ),
        _ => codex("Bash", "rm klin.json".to_string()),
    }
}

fn claude(tool: &str, input: Value) -> String {
    json!({
        "hook_event_name": "PreToolUse",
        "session_id": "performance",
        "tool_name": tool,
        "tool_input": input,
    })
    .to_string()
}

fn codex(tool: &str, command: String) -> String {
    json!({
        "turn_id": "performance",
        "session_id": "performance",
        "tool_name": tool,
        "tool_input": {"command": command},
    })
    .to_string()
}
