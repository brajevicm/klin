mod harness;

use harness::{Tree, binary, empty_home, feed};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

const ITERATIONS: usize = 5;
const EVENTS: usize = 1_000;

const DECLARATIONS_PER_KLOC: std::ops::RangeInclusive<usize> = 270..=290;

#[derive(Clone, Copy)]
struct Profile {
    name: &'static str,
    units: Option<fn(usize) -> usize>,
    expected: Option<Generated>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Generated {
    loc: usize,
    declarations: usize,
    digest: u64,
}

const BASE: Profile = Profile {
    name: "file-count",
    units: None,
    expected: None,
};
const DENSE_300K: Profile = Profile {
    name: "source-dense-300k",
    units: Some(one_unit),
    expected: Some(Generated {
        loc: 325_077,
        declarations: 90_007,
        digest: 17_738_620_850_890_864_555,
    }),
};
const DENSE_1M: Profile = Profile {
    name: "source-dense-1m",
    units: Some(three_or_four_units),
    expected: Some(Generated {
        loc: 1_033_827,
        declarations: 292_507,
        digest: 5_045_938_053_977_738_811,
    }),
};

fn one_unit(_: usize) -> usize {
    1
}

fn three_or_four_units(index: usize) -> usize {
    3 + usize::from(index.is_multiple_of(4))
}

struct Fixture {
    tree: Tree,
    files_per_language: usize,
    tsx: usize,
    generated: Generated,
    profile: Profile,
    scope: &'static str,
    config: &'static str,
    /// A directory of stand-in `cargo` and `tsc` commands and the `PATH` that puts it first,
    /// so a configuration that derives the build measures its preparation and no compiler.
    toolchain: Option<(Tree, String)>,
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
    match std::env::var("KLIN_PERF_ROW").as_deref() {
        Err(_) => base_rows(),
        Ok("structural_300k") => run_fixture(5_000, DENSE_300K),
        Ok("structural_1m") => run_fixture(5_000, DENSE_1M),
        Ok("source_areas") => source_area_rows(),
        Ok(other) => {
            panic!("KLIN_PERF_ROW={other}: expected structural_300k, structural_1m or source_areas")
        }
    }
}

fn base_rows() {
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

/// The same 2,000 source files split over 2, 100 and 500 derived source areas, each row the
/// strict run over a tree with no pinned roots, so every area is a root the survey derives.
/// The rows read the same when area count adds no walk and no git process. #159.
const SOURCE_AREAS: &[usize] = &[2, 100, 500];

fn source_area_rows() {
    for areas in SOURCE_AREAS {
        let tree = source_areas(1_000, *areas);
        let primed = strict_run(&tree, "prime survey");
        assert_eq!(primed.json()["status"], "PASS");
        let strict = repeat(|| {
            let started = Instant::now();
            let run = strict_run(&tree, "source areas");
            Sample {
                total: started.elapsed().as_millis(),
                gates: gate_times(&run.json()),
            }
        });
        println!(
            "2000 files over {areas} source areas strict: cache=warm, iterations={ITERATIONS}, median_ms={}, {}",
            median(&strict.total),
            gate_medians(&strict)
        );
    }
    println!("klin version: {}", env!("CARGO_PKG_VERSION"));
    println!(
        "machine: {}/{}",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
}

/// A repository whose Rust and TypeScript files sit in `areas` directories that hold nothing
/// but source, under a manifest directory that holds more, so each is its own derived root.
fn source_areas(files_per_language: usize, areas: usize) -> Tree {
    let tree = Tree::bare();
    tree.repository();
    tree.write("klin.json", "{}\n");
    tree.write(
        "README.md",
        "The fixture for the source-area rows of #159.\n",
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
        "web/tsconfig.json",
        "{\"compilerOptions\":{\"strict\":true},\"include\":[\"src\"]}\n",
    );
    let per_language = (areas / 2).max(1);
    for index in 4..files_per_language + 4 {
        let area = index % per_language;
        tree.write(
            &format!("rust/a{area:03}/module_{index:04}.rs"),
            &rust_source_for(index, files_per_language, BASE),
        );
        tree.write(
            &format!("web/a{area:03}/module_{index:04}.ts"),
            &typescript_source_for(index, 0, files_per_language, BASE),
        );
    }
    tree.base();
    tree
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
        let scope = chosen("KLIN_PERF_SCOPE", &["whole", "rust"]);
        let config = chosen("KLIN_PERF_CONFIG", &["build-off", "empty", "legacy"]);
        write_project_files(&tree, scope, config);
        let generated = write_sources(&tree, files_per_language, tsx, profile);
        if let Some(expected) = profile.expected {
            assert_eq!(generated, expected, "{} generated sources", profile.name);
            let density = generated.declarations * 1_000 / generated.loc;
            assert!(
                DECLARATIONS_PER_KLOC.contains(&density),
                "{} declarations per kLoC: {density}",
                profile.name
            );
        }
        tree.base();
        if config == "legacy" {
            pin_legacy(&tree);
        }

        let counts = count_paths(git_paths(tree.root(), ["ls-files", "-z"]));
        assert_eq!(counts.rust, files_per_language, "Rust source file count");
        assert_eq!(
            counts.typescript, files_per_language,
            "TypeScript source file count"
        );
        assert_eq!(counts.tsx, tsx, "TSX source file count");
        assert_shape(files_per_language, tsx, profile);
        Fixture {
            tree,
            files_per_language,
            tsx,
            generated,
            profile,
            scope,
            config,
            toolchain: (config == "empty").then(toolchain),
        }
    }

    /// A hook run, through the stand-in toolchain when the configuration derives the build.
    fn hook(&self) -> harness::Run {
        let args = ["gate", "--hook", "--changed"];
        match &self.toolchain {
            Some((_, path)) => self.tree.run_with(&[("PATH", path)], &args),
            None => self.tree.run(&args),
        }
    }

    fn dense_gate_shape_is_present(&self, samples: &Samples) {
        if self.profile.units.is_none() || std::env::var_os("KLIN_BIN").is_some() {
            return;
        }
        for name in ["complexity", "dead-symbols", "reachability"] {
            assert!(
                samples.gates.contains_key(&format!("{name}_ms")),
                "{name} gate timing is missing: {:?}",
                samples.gates.keys().collect::<Vec<_>>()
            );
        }
    }

    fn measure(&self) -> Measurements {
        let primed = self.tree.run(&["radius"]);
        assert_eq!(primed.code, 0, "prime state: {}", primed.out);
        let primed = self.hook();
        assert_eq!(primed.code, 0, "prime survey: {}", primed.out);
        assert!(primed.out.is_empty(), "prime survey: {}", primed.out);

        self.change();
        let changed = changed_counts(self.tree.root());
        assert_eq!(changed.rust, 10, "changed Rust file count");
        assert_eq!(changed.typescript, 10, "changed TypeScript file count");
        assert_eq!(changed.rust + changed.typescript, 20, "changed file count");

        let warm = repeat(|| {
            let stops = journal(&self.tree).len();
            let started = Instant::now();
            let run = self.hook();
            let total = started.elapsed().as_millis();
            assert_eq!(run.code, 0, "warm hook: {}", run.out);
            assert!(run.out.is_empty(), "warm hook: {}", run.out);
            let lines = journal(&self.tree);
            assert!(lines.len() > stops, "warm hook wrote no journal line");
            Sample {
                total,
                gates: gate_times(&lines[lines.len() - 1]),
            }
        });
        let cold = repeat(|| {
            let cleaned = self.tree.run(&["cache", "clean"]);
            assert_eq!(cleaned.code, 0, "cache clean: {}", cleaned.out);
            let started = Instant::now();
            let run = strict_run(&self.tree, "cold survey");
            Sample {
                total: started.elapsed().as_millis(),
                gates: gate_times(&run.json()),
            }
        });
        let strict = repeat(|| {
            let started = Instant::now();
            let run = strict_run(&self.tree, "strict");
            Sample {
                total: started.elapsed().as_millis(),
                gates: gate_times(&run.json()),
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

/// The fixture's configuration: the build switched off, the empty opt-in marker, or the one the
/// running binary's `init --force` pins after the base, which only a pre-#180 binary reads.
/// The value an environment variable names from a fixed set, and the first when it is unset.
fn chosen(variable: &str, allowed: &[&'static str]) -> &'static str {
    let Ok(named) = std::env::var(variable) else {
        return allowed[0];
    };
    allowed
        .iter()
        .copied()
        .find(|held| *held == named)
        .unwrap_or_else(|| panic!("{variable}={named}: expected one of {allowed:?}"))
}

fn write_project_files(tree: &Tree, scope: &str, config: &str) {
    let complexity = match scope {
        "rust" => r#""complexity":{"in":"rust"}"#,
        _ => "",
    };
    let config = match (config, complexity.is_empty()) {
        ("build-off", true) => r#"{"build":[]}"#.to_string(),
        ("build-off", false) => format!(r#"{{"build":[],{complexity}}}"#),
        _ => format!("{{{complexity}}}"),
    };
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

fn write_sources(
    tree: &Tree,
    files_per_language: usize,
    tsx: usize,
    profile: Profile,
) -> Generated {
    let mut generated = Generated {
        loc: 0,
        declarations: 0,
        digest: 0xcbf2_9ce4_8422_2325,
    };
    for index in 0..files_per_language {
        let source = rust_source_for(index, files_per_language, profile);
        record(tree, &mut generated, &rust_path(index), &source);
    }
    for index in 0..files_per_language {
        let source = typescript_source_for(index, tsx, files_per_language, profile);
        record(tree, &mut generated, &typescript_path(index, tsx), &source);
    }
    generated
}

fn record(tree: &Tree, generated: &mut Generated, path: &str, source: &str) {
    generated.loc += source.bytes().filter(|byte| *byte == b'\n').count();
    generated.declarations += source.lines().filter(|line| declares(line)).count();
    for byte in path.bytes().chain([0]).chain(source.bytes()).chain([0]) {
        generated.digest = (generated.digest ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3);
    }
    tree.write(path, source);
}

fn declares(line: &str) -> bool {
    [
        "const ",
        "struct ",
        "type ",
        "impl ",
        "fn ",
        "pub fn ",
        "    fn ",
        "interface ",
        "class ",
        "    adjust(",
        "function ",
        "export function ",
    ]
    .iter()
    .any(|start| line.starts_with(start))
}

fn assert_shape(files_per_language: usize, tsx: usize, profile: Profile) {
    let Some(units) = profile.units else {
        return;
    };
    assert!(
        files_per_language >= 300,
        "dense structural fixture is too small"
    );
    assert_rust_shape(
        &rust_source_for(0, files_per_language, profile),
        &rust_source_for(4, files_per_language, profile),
        &rust_source_for(5, files_per_language, profile),
        &rust_source_for(260, files_per_language, profile),
    );
    assert_eq!(
        rust_source_for(4, files_per_language, profile)
            .matches("pub fn value_")
            .count(),
        units(4),
        "Rust units per file"
    );
    assert_typescript_shape(
        [tsx + 4, tsx + 5, tsx + 260, 4, 3]
            .map(|index| typescript_source_for(index, tsx, files_per_language, profile)),
    );
}

fn assert_rust_shape(lib: &str, first: &str, next: &str, duplicate: &str) {
    assert!(lib.contains("pub mod module_0004;"));
    assert!(lib.contains("fn main()"));
    assert!(first.contains("pub fn value_0004("));
    assert!(first.contains("struct Record_0004 "));
    assert!(first.contains("const PHASE_0004:"));
    assert!(first.contains("fn shared_04_0("));
    assert!(first.contains("impl Record_0004 "));
    assert!(first.contains("fn branch_0004("));
    assert!(first.contains("if value % 2"));
    assert!(next.contains("value_0004(input)"));
    assert!(duplicate.contains("fn shared_04_0("));
}

fn assert_typescript_shape([first, next, duplicate, tsx, test]: [String; 5]) {
    assert!(first.contains("export function value_0054("));
    assert!(first.contains("interface Record_0054 "));
    assert!(first.contains("class Holder_0054 "));
    assert!(first.contains("const PHASE_0054 "));
    assert!(first.contains("function shared_54_0("));
    assert!(first.contains("function branch_0054("));
    assert!(first.contains("switch (result % 3)"));
    assert!(next.contains("value_0054(input)"));
    assert!(duplicate.contains("function shared_54_0("));
    assert!(tsx.contains("<span>{input}</span>"));
    assert!(test.contains("import { value_0054 }"));
    assert!(test.contains("return value_0054(1);"));
}

/// The fully pinned configuration an earlier klin wrote: every section its survey derived,
/// written by that binary's own `init --force`, with the build switched off, and committed so
/// the base records it.
fn pin_legacy(tree: &Tree) {
    let run = tree.run(&["init", "--force"]);
    assert_eq!(run.code, 0, "legacy pin: {}", run.out);
    let text = std::fs::read_to_string(tree.path("klin.json")).unwrap_or_default();
    let mut config: Value = serde_json::from_str(&text).unwrap_or_default();
    config["build"] = json!([]);
    tree.write("klin.json", &config.to_string());
    tree.commit("pin the legacy configuration");
}

/// Stand-in `cargo` and `tsc` commands that succeed at once.
fn toolchain() -> (Tree, String) {
    let tools = Tree::bare();
    for name in ["cargo", "tsc"] {
        let path = tools.write(name, "#!/bin/sh\nexit 0\n");
        let mode = std::os::unix::fs::PermissionsExt::from_mode(0o755);
        assert!(std::fs::set_permissions(&path, mode).is_ok(), "{name}");
    }
    let path = format!("{}:/usr/bin:/bin", tools.root().display());
    (tools, path)
}

fn print_rows(fixture: &Fixture, rows: &Measurements) {
    let counts = count_paths(git_paths(fixture.tree.root(), ["ls-files", "-z"]));
    let changed = changed_counts(fixture.tree.root());
    let size = fixture.files_per_language * 2;
    println!(
        "fixture {} ({}, complexity_scope={}, config={}): loc={}, declarations={}, digest={:016x}, rust_files={}, typescript_files={}, tsx_files={}, changed_files={} ({} rust, {} typescript)",
        size,
        fixture.profile.name,
        fixture.scope,
        fixture.config,
        fixture.generated.loc,
        fixture.generated.declarations,
        fixture.generated.digest,
        counts.rust,
        counts.typescript,
        counts.tsx,
        changed.rust + changed.typescript,
        changed.rust,
        changed.typescript
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
    println!(
        "resource: strict_peak_rss_kb={}",
        peak_rss(fixture.tree.root())
            .map_or_else(|| "unavailable".to_string(), |kb| kb.to_string())
    );
    println!("note: hook timings exclude the project's build command");
    println!("klin version: {}", env!("CARGO_PKG_VERSION"));
    println!(
        "machine: {}/{}",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
}

/// Peak RSS is controlled-machine evidence, not a contributor-test threshold. `/usr/bin/time`
/// is intentionally optional so the fixture remains runnable where the platform has no report.
fn peak_rss(root: &Path) -> Option<u64> {
    let (time_args, marker, divisor) = if cfg!(target_os = "macos") {
        (&["-l"][..], "maximum resident set size", 1024)
    } else {
        (&["-v"][..], "Maximum resident set size (kbytes):", 1)
    };
    let mut command = Command::new("/usr/bin/time");
    command.args(time_args);
    command.env("LC_ALL", "C");
    for (name, _) in std::env::vars().filter(|(name, _)| name.starts_with("GITHUB_")) {
        command.env_remove(name);
    }
    let done = command
        .arg(binary())
        .args(["gate", "--strict", "--json"])
        .env("HOME", empty_home())
        .current_dir(root)
        .output()
        .ok()?;
    let output = String::from_utf8_lossy(&done.stderr);
    output.lines().find_map(|line| {
        line.contains(marker)
            .then(|| {
                let value = line.split_whitespace().last()?.parse::<u64>().ok()?;
                Some(value / divisor)
            })
            .flatten()
    })
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

fn gate_times(report: &Value) -> BTreeMap<String, u64> {
    let gates = report["gates"].as_array();
    assert!(gates.is_some(), "gate timing rows: {report}");
    let mut times = BTreeMap::new();
    for gate in gates.into_iter().flatten() {
        let Some(name) = gate["name"].as_str() else {
            continue;
        };
        if let Some(ms) = gate["ms"].as_u64() {
            times.insert(format!("{name}_ms"), ms);
        }
        for field in ["reads", "parses", "extracted", "shared", "ms"] {
            if let Some(value) = gate["facts"][field].as_u64() {
                times.insert(format!("{name}_facts_{field}"), value);
            }
        }
        for field in ["reads", "parses"] {
            if let Some(value) = gate["work"][field].as_u64() {
                times.insert(format!("{name}_work_{field}"), value);
            }
        }
    }
    times
}

fn journal(tree: &Tree) -> Vec<Value> {
    std::fs::read_to_string(tree.state("journal.jsonl"))
        .unwrap_or_default()
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

fn gate_medians(samples: &Samples) -> String {
    samples
        .gates
        .iter()
        .map(|(name, values)| format!("{name}={}", median(values)))
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
    match profile.units {
        Some(units) => dense_rust_source(index, total, units(index)),
        None => rust_source(index, total),
    }
}

fn dense_rust_source(index: usize, total: usize, units: usize) -> String {
    let body = dense_rust_body(index, total, units);
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

fn dense_rust_body(index: usize, total: usize, units: usize) -> String {
    let dependency = if index == 0 || index == 4 {
        total - 1
    } else {
        index - 1
    };
    let bucket = index % 256;
    let mut source = format!("use crate::module_{dependency:04}::value_{dependency:04};\n");
    for unit in 0..units {
        let id = unit_id(index, unit);
        let calls = unit_calls(index, unit, units, dependency);
        source.push_str(&format!(
            "const SLOT_{id}: usize = {index};\nconst PHASE_{id}: &str = \"base\";\nstruct Record_{id} {{\n    value: usize,\n}}\ntype Alias_{id} = Record_{id};\nfn shared_{bucket:02}_{unit}(input: usize) -> usize {{\n    input + SLOT_{id} + PHASE_{id}.len()\n}}\nimpl Record_{id} {{\n    fn adjust(&self) -> usize {{\n        self.value + SLOT_{id}\n    }}\n}}\npub fn value_{id}(input: usize) -> usize {{\n    let record: Alias_{id} = Record_{id} {{ value: input }};\n    shared_{bucket:02}_{unit}(record.adjust()) + branch_{id}(input){calls}\n}}\nfn branch_{id}(mut value: usize) -> usize {{\n    if value % 2 == 0 {{\n        value += SLOT_{id};\n    }} else {{\n        value += 1;\n    }}\n    match value % 3 {{\n        0 => value,\n        1 => value + 1,\n        _ => value + 2,\n    }}\n}}\n"
        ));
    }
    source
}

fn unit_id(index: usize, unit: usize) -> String {
    match unit {
        0 => format!("{index:04}"),
        _ => format!("{index:04}_{unit}"),
    }
}

fn unit_calls(index: usize, unit: usize, units: usize, dependency: usize) -> String {
    let mut calls = String::new();
    if unit == 0 {
        calls.push_str(&format!(" + value_{dependency:04}(input)"));
    }
    if unit + 1 < units {
        calls.push_str(&format!(" + value_{}(input)", unit_id(index, unit + 1)));
    }
    calls
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
    match profile.units {
        Some(units) => dense_typescript_source(index, tsx, total, units(index)),
        None => typescript_source(index, tsx),
    }
}

fn dense_typescript_source(index: usize, tsx: usize, total: usize, units: usize) -> String {
    let body = dense_typescript_body(index, tsx, total, units);
    let first = tsx + 4;
    let component = if (4..first).contains(&index) {
        format!(
            "export const Component_{:04} = (input: number) => <span>{{input}}</span>;\n",
            index - 4
        )
    } else {
        String::new()
    };
    match index {
        0 => format!("export {{ value_{first:04} }} from \"./module_{first:04}\";\n{body}"),
        1 => format!(
            "export function heldEscape(input: unknown): unknown {{\n    return input as any;\n}}\n{body}"
        ),
        2 => format!(
            "export function heldStub(): never {{\n    throw new Error(\"not implemented\");\n}}\n{body}"
        ),
        3 => format!(
            "import {{ value_{first:04} }} from \"./module_{first:04}\";\nexport function test_fixture(): number {{\n    return value_{first:04}(1);\n}}\nit(\"keeps the fixture\", () => value_{first:04}(1));\n{body}"
        ),
        _ => component + &body,
    }
}

fn dense_typescript_body(index: usize, tsx: usize, total: usize, units: usize) -> String {
    let dependency = if index == 0 || index == tsx + 4 {
        total - 1
    } else {
        index - 1
    };
    let bucket = index % 256;
    let mut source =
        format!("import {{ value_{dependency:04} }} from \"./module_{dependency:04}\";\n");
    for unit in 0..units {
        let id = unit_id(index, unit);
        let calls = unit_calls(index, unit, units, dependency);
        source.push_str(&format!(
            "const SLOT_{id}: number = {index};\nconst PHASE_{id} = \"base\";\ninterface Record_{id} {{\n    value: number;\n}}\ntype Alias_{id} = Record_{id};\nclass Holder_{id} {{\n    constructor(private value: number) {{}}\n    adjust(): number {{\n        return this.value + SLOT_{id};\n    }}\n}}\nfunction shared_{bucket:02}_{unit}(input: number): number {{\n    return input + SLOT_{id} + PHASE_{id}.length;\n}}\nexport function value_{id}(input: number): number {{\n    const record: Alias_{id} = {{ value: input }};\n    const holder = new Holder_{id}(record.value);\n    return shared_{bucket:02}_{unit}(holder.adjust()) + branch_{id}(input){calls};\n}}\nfunction branch_{id}(value: number): number {{\n    let result = value;\n    if (result % 2 === 0) {{\n        result += SLOT_{id};\n    }} else {{\n        result += 1;\n    }}\n    switch (result % 3) {{\n        case 0: return result;\n        case 1: return result + 1;\n        default: return result + 2;\n    }}\n}}\n"
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
