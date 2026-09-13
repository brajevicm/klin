mod harness;

use harness::{Tree, feed};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

const ITERATIONS: usize = 5;
const EVENTS: usize = 1_000;

#[derive(Clone, Copy)]
struct Profile {
    name: &'static str,
    blocks: Option<usize>,
    expected_loc: Option<usize>,
}

const BASE: Profile = Profile {
    name: "file-count",
    blocks: None,
    expected_loc: None,
};
const DENSE_300K: Profile = Profile {
    name: "source-dense-300k",
    blocks: Some(0),
    expected_loc: Some(312_077),
};
const DENSE_1M: Profile = Profile {
    name: "source-dense-1m",
    blocks: Some(4),
    expected_loc: Some(989_077),
};

struct Fixture {
    tree: Tree,
    files_per_language: usize,
    tsx: usize,
    loc: usize,
    profile: Profile,
}

#[derive(Default)]
struct Measurements {
    warm: Samples,
    cold: Samples,
    strict: Samples,
}

#[derive(Default)]
struct Samples {
    total: Vec<u128>,
    gates: BTreeMap<String, Vec<u64>>,
}

struct Sample {
    total: u128,
    gates: BTreeMap<String, u64>,
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

#[test]
#[cfg_attr(
    not(any()),
    ignore = "expensive; run with cargo test --test performance -- --ignored base_2k --nocapture"
)]
fn base_2k() {
    run_fixture(1_000, BASE);
}

#[test]
#[cfg_attr(
    not(any()),
    ignore = "expensive; run with cargo test --test performance -- --ignored base_10k --nocapture"
)]
fn base_10k() {
    run_fixture(5_000, BASE);
}

#[test]
#[cfg_attr(
    not(any()),
    ignore = "manual; run with cargo test --test performance -- --ignored structural_300k --nocapture"
)]
fn structural_300k() {
    run_fixture(5_000, DENSE_300K);
}

#[test]
#[cfg_attr(
    not(any()),
    ignore = "manual; run with cargo test --test performance -- --ignored structural_1m --nocapture"
)]
fn structural_1m() {
    run_fixture(5_000, DENSE_1M);
}

fn run_fixture(files_per_language: usize, profile: Profile) {
    let fixture = Fixture::with_profile(files_per_language, profile);
    let rows = fixture.measure();
    print_rows(&fixture, &rows);
}

impl Fixture {
    fn new(files_per_language: usize) -> Fixture {
        Fixture::with_profile(files_per_language, BASE)
    }

    fn with_profile(files_per_language: usize, profile: Profile) -> Fixture {
        let tree = Tree::bare();
        tree.repository();
        let tsx = files_per_language / 100;
        write_project_files(&tree, profile);
        let loc = write_sources(&tree, files_per_language, tsx, profile);
        tree.base();

        let counts = count_paths(git_paths(tree.root(), ["ls-files", "-z"]));
        assert_eq!(counts.rust, files_per_language, "Rust source file count");
        assert_eq!(
            counts.typescript, files_per_language,
            "TypeScript source file count"
        );
        assert_eq!(counts.tsx, tsx, "TSX source file count");
        if let Some(expected) = profile.expected_loc {
            assert_eq!(loc, expected, "{} source LoC", profile.name);
        }
        assert_shape(files_per_language, tsx, profile);
        Fixture {
            tree,
            files_per_language,
            tsx,
            loc,
            profile,
        }
    }

    fn dense_gate_shape_is_present(&self, samples: &Samples) {
        if self.profile.blocks.is_none() {
            return;
        }
        for name in ["complexity", "dead-symbols", "reachability"] {
            assert!(
                samples.gates.contains_key(name),
                "{name} gate timing is missing: {:?}",
                samples.gates.keys().collect::<Vec<_>>()
            );
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
            let total = started.elapsed().as_millis();
            let timed = self.tree.run(&["gate", "--changed", "--json"]);
            assert_eq!(timed.code, 0, "warm gate timings: {}", timed.out);
            Sample {
                total,
                gates: gate_times(&timed),
            }
        });
        let cold = repeat(|| {
            let cleaned = self.tree.run(&["cache", "clean"]);
            assert_eq!(cleaned.code, 0, "cache clean: {}", cleaned.out);
            let started = Instant::now();
            let run = strict_run(&self.tree, "cold survey");
            Sample {
                total: started.elapsed().as_millis(),
                gates: gate_times(&run),
            }
        });
        let strict = repeat(|| {
            let started = Instant::now();
            let run = strict_run(&self.tree, "strict");
            Sample {
                total: started.elapsed().as_millis(),
                gates: gate_times(&run),
            }
        });
        self.dense_gate_shape_is_present(&warm);
        self.dense_gate_shape_is_present(&cold);
        self.dense_gate_shape_is_present(&strict);
        Measurements { warm, cold, strict }
    }

    fn change(&self) {
        for offset in 0..10 {
            let rust = offset + 4;
            self.tree.write(
                &rust_path(rust),
                &rust_source_for(rust, self.files_per_language, self.profile)
                    .replace("base", "turn"),
            );
            let typescript = offset + 4 + self.tsx;
            self.tree.write(
                &typescript_path(typescript, self.tsx),
                &typescript_source_for(typescript, self.tsx, self.files_per_language, self.profile)
                    .replace("base", "turn"),
            );
        }
    }
}

fn write_project_files(tree: &Tree, profile: Profile) {
    let structural = match profile.blocks {
        Some(_) => {
            r#","dead_symbols":{"roots":["rust/src","web/src"],"languages":["rust","typescript"]},"reachability":[{"name":"rust-modules","roots":["rust/src"],"pattern":"module_*.rs","languages":["rust"]},{"name":"typescript-modules","roots":["web/src"],"pattern":"module_*.ts","languages":["typescript"]}]"#
        }
        None => "",
    };
    let config = format!(
        r#"{{"project":"performance","version":"{}","build":[],"complexity":{{"languages":["rust","typescript"]}}{structural}}}"#,
        env!("CARGO_PKG_VERSION"),
    );
    assert!(serde_json::from_str::<Value>(&config).is_ok(), "{config}");
    tree.write("klin.json", &config);
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
}

fn write_sources(tree: &Tree, files_per_language: usize, tsx: usize, profile: Profile) -> usize {
    write_rust_sources(tree, files_per_language, profile)
        + write_typescript_sources(tree, files_per_language, tsx, profile)
}

fn write_rust_sources(tree: &Tree, files_per_language: usize, profile: Profile) -> usize {
    let mut loc = 0;
    for index in 0..files_per_language {
        let path = rust_path(index);
        let source = rust_source_for(index, files_per_language, profile);
        loc += source.bytes().filter(|byte| *byte == b'\n').count();
        if profile.blocks.is_some() {
            assert_eq!(
                source,
                rust_source_for(index, files_per_language, profile),
                "Rust source is deterministic: {path}"
            );
        }
        tree.write(&path, &source);
    }
    loc
}

fn write_typescript_sources(
    tree: &Tree,
    files_per_language: usize,
    tsx: usize,
    profile: Profile,
) -> usize {
    let mut loc = 0;
    for index in 0..files_per_language {
        let path = typescript_path(index, tsx);
        let source = typescript_source_for(index, tsx, files_per_language, profile);
        loc += source.bytes().filter(|byte| *byte == b'\n').count();
        if profile.blocks.is_some() {
            assert_eq!(
                source,
                typescript_source_for(index, tsx, files_per_language, profile),
                "TypeScript source is deterministic: {path}"
            );
        }
        tree.write(&path, &source);
    }
    loc
}

fn assert_shape(files_per_language: usize, tsx: usize, profile: Profile) {
    if profile.blocks.is_none() {
        return;
    }
    assert!(
        files_per_language >= 40,
        "dense structural fixture is too small"
    );
    assert_rust_shape(
        &rust_source_for(0, files_per_language, profile),
        &rust_source_for(4, files_per_language, profile),
        &rust_source_for(5, files_per_language, profile),
        &rust_source_for(260, files_per_language, profile),
    );
    assert_typescript_shape(
        &typescript_source_for(tsx + 4, tsx, files_per_language, profile),
        &typescript_source_for(tsx + 5, tsx, files_per_language, profile),
        &typescript_source_for(tsx + 260, tsx, files_per_language, profile),
        &typescript_source_for(4, tsx, files_per_language, profile),
    );
}

fn assert_rust_shape(lib: &str, first: &str, next: &str, duplicate: &str) {
    assert!(lib.contains("pub mod module_0004;"));
    assert!(lib.contains("fn main()"));
    assert!(first.contains("pub fn value_0004"));
    assert!(first.contains("struct Record_0004"));
    assert!(first.contains("const PHASE_0004"));
    assert!(first.contains("fn shared_04"));
    assert!(first.contains("impl Record_0004"));
    assert!(first.contains("if value % 2"));
    assert!(next.contains("value_0004"));
    assert!(duplicate.contains("fn shared_04"));
}

fn assert_typescript_shape(first: &str, next: &str, duplicate: &str, tsx: &str) {
    assert!(first.contains("export function value_0054"));
    assert!(first.contains("interface Record_0054"));
    assert!(first.contains("class Holder_0054"));
    assert!(first.contains("const PHASE_0054"));
    assert!(first.contains("function shared_54"));
    assert!(first.contains("switch (result % 3)"));
    assert!(next.contains("value_0054"));
    assert!(duplicate.contains("function shared_54"));
    assert!(tsx.contains("<span>{input}</span>"));
}

fn print_rows(fixture: &Fixture, rows: &Measurements) {
    let counts = count_paths(git_paths(fixture.tree.root(), ["ls-files", "-z"]));
    let size = fixture.files_per_language * 2;
    println!(
        "fixture {} ({}): loc={}, rust_files={}, typescript_files={}, tsx_files={}, changed_files=20 (10 rust, 10 typescript)",
        size, fixture.profile.name, fixture.loc, counts.rust, counts.typescript, counts.tsx
    );
    println!(
        "{} warm hook: cache=warm, iterations={ITERATIONS}, median_ms={}, {}, project_build=excluded",
        size,
        median(&rows.warm.total),
        gate_medians(&rows.warm)
    );
    println!(
        "{} cold survey: cache=cold, iterations={ITERATIONS}, median_ms={}, {}",
        size,
        median(&rows.cold.total),
        gate_medians(&rows.cold)
    );
    println!(
        "{} strict: cache=warm, iterations={ITERATIONS}, median_ms={}, {}",
        size,
        median(&rows.strict.total),
        gate_medians(&rows.strict)
    );
    println!("note: hook timings exclude the project's build command");
    println!("klin version: {}", env!("CARGO_PKG_VERSION"));
    println!(
        "machine: {}/{}",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
}

fn repeat(mut run: impl FnMut() -> Sample) -> Samples {
    let mut samples = Samples::default();
    for _ in 0..ITERATIONS {
        let sample = run();
        samples.total.push(sample.total);
        for (name, milliseconds) in sample.gates {
            samples.gates.entry(name).or_default().push(milliseconds);
        }
    }
    samples
}

fn repeat_totals(mut run: impl FnMut() -> u128) -> Vec<u128> {
    (0..ITERATIONS).map(|_| run()).collect()
}

fn median<T: Copy + Ord>(values: &[T]) -> T {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    sorted[sorted.len() / 2]
}

fn strict_run(tree: &Tree, label: &str) -> harness::Run {
    let run = tree.run(&["gate", "--strict", "--json"]);
    assert_eq!(run.code, 0, "{label}: {}", run.out);
    let report = run.json();
    assert_eq!(report["status"], "PASS", "{label}: {report}");
    assert_eq!(report["exit"], 0, "{label}: {report}");
    run
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

fn gate_times(run: &harness::Run) -> BTreeMap<String, u64> {
    run.json()["gates"]
        .as_array()
        .map(|gates| {
            gates
                .iter()
                .filter_map(|gate| Some((gate["name"].as_str()?.to_string(), gate["ms"].as_u64()?)))
                .collect()
        })
        .unwrap_or_default()
}

fn gate_medians(samples: &Samples) -> String {
    ["complexity", "dead-symbols", "reachability"]
        .iter()
        .filter_map(|name| Some(format!("{name}_ms={}", median(samples.gates.get(*name)?))))
        .collect::<Vec<_>>()
        .join(", ")
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

fn rust_source_for(index: usize, total: usize, profile: Profile) -> String {
    match profile.blocks {
        Some(blocks) => dense_rust_source(index, total, dense_blocks(index, blocks)),
        None => rust_source(index, total),
    }
}

fn dense_rust_source(index: usize, total: usize, blocks: usize) -> String {
    let body = dense_rust_body(index, total, blocks);
    match index {
        0 => format!(
            "pub mod held_escape;\npub mod held_stub;\npub mod module_0004;\npub mod module_0005;\nfn main() {{ value_0000(1); }}\n{body}"
        ),
        1 => format!(
            "pub fn held_escape(input: usize) -> usize {{\n    Some(input).{}\n}}\n{body}",
            "unwrap()"
        ),
        2 => format!("pub fn held_stub() -> usize {{\n    todo!()\n}}\n{body}"),
        3 => {
            format!("#[test]\nfn test_fixture_contract() {{\n    assert_eq!(2 + 2, 4);\n}}\n{body}")
        }
        _ => body,
    }
}

fn dense_rust_body(index: usize, total: usize, blocks: usize) -> String {
    let dependency = if index == 0 || index == 4 {
        total - 1
    } else {
        index - 1
    };
    let bucket = index % 256;
    let helper = if blocks == 0 {
        String::new()
    } else {
        format!(" + helper_{index}_{last}(input)", last = blocks - 1)
    };
    let mut source = format!(
        "use crate::module_{dependency:04}::value_{dependency:04};\nconst SLOT_{index:04}: usize = {index};\nconst PHASE_{index:04}: &str = \"base\";\nstruct Record_{index:04} {{\n    value: usize,\n}}\ntype Alias_{index:04} = Record_{index:04};\nfn shared_{bucket:02}(input: usize) -> usize {{\n    input + SLOT_{index:04} + PHASE_{index:04}.len()\n}}\nimpl Record_{index:04} {{\n    fn adjust(&self) -> usize {{\n        self.value + SLOT_{index:04}\n    }}\n}}\npub fn value_{index:04}(input: usize) -> usize {{\n    let record: Alias_{index:04} = Record_{index:04} {{ value: input }};\n    shared_{bucket:02}(record.adjust()) + value_{dependency:04}(input){helper}\n}}\n"
    );
    for unit in 0..blocks {
        let previous = if unit == 0 {
            String::new()
        } else {
            format!(
                "    value = helper_{index}_{previous}(value);\n",
                previous = unit - 1
            )
        };
        source.push_str(&format!(
            "fn helper_{index}_{unit}(mut value: usize) -> usize {{\n{previous}    let bias = SLOT_{index:04} + {unit};\n    if value % 2 == 0 {{\n        value += bias;\n    }} else {{\n        value += bias + 1;\n    }}\n    for step in 0..3 {{\n        value += step;\n    }}\n    match value % 3 {{\n        0 => value,\n        1 => value + 1,\n        _ => value + 2,\n    }}\n}}\n"
        ));
    }
    source
}

fn dense_blocks(index: usize, base: usize) -> usize {
    base + usize::from(index % 10 < 7)
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

fn typescript_source_for(index: usize, tsx: usize, total: usize, profile: Profile) -> String {
    match profile.blocks {
        Some(blocks) => dense_typescript_source(index, tsx, total, dense_blocks(index, blocks)),
        None => typescript_source(index, tsx),
    }
}

fn dense_typescript_source(index: usize, tsx: usize, total: usize, blocks: usize) -> String {
    let body = dense_typescript_body(index, tsx, total, blocks);
    let first = tsx + 4;
    let component = (index >= 4 && index < first)
        .then(|| {
            format!(
                "export const Component_{:04} = (input: number) => <span>{{input}}</span>;\n",
                index - 4
            )
        })
        .unwrap_or_default();
    match index {
        0 => format!("export {{ value_{first:04} }} from \"./module_{first:04}\";\n{body}"),
        1 => format!(
            "export function heldEscape(input: unknown): unknown {{\n    return input as any;\n}}\n{body}"
        ),
        2 => format!(
            "export function heldStub(): never {{\n    throw new Error(\"not implemented\");\n}}\n{body}"
        ),
        3 => format!(
            "import {{ value_{first:04} }} from \"./module_{first:04}\";\nexport function test_fixture(): number {{\n    return value_{first}(1);\n}}\nit(\"keeps the fixture\", () => value_{first}(1));\n{body}"
        ),
        _ => component + &body,
    }
}

fn dense_typescript_body(index: usize, tsx: usize, total: usize, blocks: usize) -> String {
    let dependency = if index == 0 || index == tsx + 4 {
        total - 1
    } else {
        index - 1
    };
    let bucket = index % 256;
    let helper = if blocks == 0 {
        String::new()
    } else {
        format!(" + helper_{index}_{last}(input)", last = blocks - 1)
    };
    let import_line =
        format!("import {{ value_{dependency:04} }} from \"./module_{dependency:04}\";\n");
    let mut source = format!(
        "{import_line}const SLOT_{index:04}: number = {index};\nconst PHASE_{index:04} = \"base\";\ninterface Record_{index:04} {{\n    value: number;\n}}\ntype Alias_{index:04} = Record_{index:04};\nclass Holder_{index:04} {{\n    constructor(private value: number) {{}}\n    adjust(): number {{\n        return this.value + SLOT_{index:04};\n    }}\n}}\nfunction shared_{bucket:02}(input: number): number {{\n    return input + SLOT_{index:04} + PHASE_{index:04}.length;\n}}\nexport function value_{index:04}(input: number): number {{\n    const record: Alias_{index:04} = {{ value: input }};\n    const holder = new Holder_{index:04}(record.value);\n    return shared_{bucket:02}(holder.adjust()) + value_{dependency:04}(input){helper};\n}}\n"
    );
    for unit in 0..blocks {
        let previous = if unit == 0 {
            String::new()
        } else {
            format!(
                "    value = helper_{index}_{previous}(value);\n",
                previous = unit - 1
            )
        };
        source.push_str(&format!(
            "function helper_{index}_{unit}(value: number): number {{\n{previous}    let result = value + SLOT_{index:04} + {unit};\n    if (result % 2 === 0) {{\n        result += 1;\n    }} else {{\n        result += 2;\n    }}\n    for (const step of [0, 1, 2]) {{\n        result += step;\n    }}\n    switch (result % 3) {{\n        case 0: return result;\n        case 1: return result + 1;\n        default: return result + 2;\n    }}\n}}\n"
        ));
    }
    source
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
    repeat_totals(|| {
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
