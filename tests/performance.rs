mod harness;

use harness::{Tree, binary, empty_home, feed};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

const ITERATIONS: usize = 5;
const EVENTS: usize = 1_000;
const STRUCTURAL_CACHE: &str = "cache/structural";

#[derive(Clone, Copy, PartialEq)]
enum PerfCase {
    Full,
    Warm20,
    Warm100,
}

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
    changed: Counts,
    warm: Samples,
    uncached: Samples,
    cold: Samples,
    strict: Samples,
    warm_delta: Option<(usize, Samples)>,
    resources: Resources,
}

#[derive(Default)]
struct Samples {
    total: Vec<u128>,
    gates: BTreeMap<String, Vec<u64>>,
}

#[derive(Default)]
struct Resources {
    cache_files: usize,
    cache_bytes: u64,
    warm_rss: Option<u64>,
    uncached_rss: Option<u64>,
    dead_symbols_rss: Option<u64>,
    strict_rss: Option<u64>,
}

struct Sample {
    total: u128,
    gates: BTreeMap<String, u64>,
}

#[derive(Clone, Copy, Default)]
struct Counts {
    rust: usize,
    typescript: usize,
    tsx: usize,
}

#[test]
#[ignore = "expensive; run with cargo test -- --ignored perf --nocapture"]
fn performance_fixture() {
    let case = perf_case();
    match std::env::var("KLIN_PERF_ROW").as_deref() {
        Err(_) => {
            require_300k_case(case);
            base_rows();
        }
        Ok("structural_300k") => run_fixture(5_000, DENSE_300K, case),
        Ok("structural_1m") => {
            require_300k_case(case);
            run_fixture(5_000, DENSE_1M, PerfCase::Full);
        }
        Ok("source_areas") => {
            require_300k_case(case);
            source_area_rows();
        }
        Ok(other) => {
            panic!("KLIN_PERF_ROW={other}: expected structural_300k, structural_1m or source_areas")
        }
    }
}

fn require_300k_case(case: PerfCase) {
    assert!(
        case == PerfCase::Full,
        "KLIN_PERF_CASE=warm20 or warm100 requires KLIN_PERF_ROW=structural_300k"
    );
}

fn perf_case() -> PerfCase {
    match std::env::var("KLIN_PERF_CASE").as_deref() {
        Err(_) | Ok("full") => PerfCase::Full,
        Ok("warm20") => PerfCase::Warm20,
        Ok("warm100") => PerfCase::Warm100,
        Ok(other) => panic!(
            "KLIN_PERF_CASE={other}: expected full, warm20 or warm100.\n\
             300k warm20: KLIN_PERF_ROW=structural_300k KLIN_PERF_CASE=warm20 cargo test --release --test performance -- --ignored perf --nocapture\n\
             300k warm100: KLIN_PERF_ROW=structural_300k KLIN_PERF_CASE=warm100 cargo test --release --test performance -- --ignored perf --nocapture"
        ),
    }
}

fn base_rows() {
    let small = Fixture::new(1_000);
    let small_rows = small.measure(PerfCase::Full);
    let guard_rows = guard(&small.tree);
    print_rows(&small, &small_rows, PerfCase::Full);
    println!(
        "guard 1000 events: cache=separate, iterations={ITERATIONS}, median_ms={}, per_event_ms={:.3}",
        median(&guard_rows),
        median(&guard_rows) as f64 / EVENTS as f64
    );

    let large = Fixture::new(5_000);
    let large_rows = large.measure(PerfCase::Full);
    print_rows(&large, &large_rows, PerfCase::Full);
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
        "{\"name\":\"fixture-web\",\"private\":true,\"version\":\"0.1.0\",\"exports\":\"./src/index.ts\"}\n",
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

fn run_fixture(files_per_language: usize, profile: Profile, case: PerfCase) {
    let fixture = Fixture::with_profile(files_per_language, profile);
    let rows = fixture.measure(case);
    print_rows(&fixture, &rows, case);
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
        let layering = profile.units.is_some() && std::env::var_os("KLIN_BIN").is_none();
        write_project_files(&tree, scope, config, layering);
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

    fn current_dense(&self) -> bool {
        self.profile.units.is_some() && std::env::var_os("KLIN_BIN").is_none()
    }

    fn dense_gate_shape_is_present(&self, samples: &Samples) {
        if !self.current_dense() {
            return;
        }
        for name in [
            "complexity",
            "dead-symbols",
            "reachability",
            "layering",
            "public-api",
        ] {
            assert!(
                samples.gates.contains_key(&format!("{name}_ms")),
                "{name} gate timing is missing: {:?}",
                samples.gates.keys().collect::<Vec<_>>()
            );
        }
    }

    fn dense_cache_shape_is_present(&self, samples: &Samples, changed: usize) {
        if !self.current_dense() {
            return;
        }
        let median_counter = |name: &str| median(&samples.gates[name]);
        let unchanged = (self.files_per_language * 2 - changed) as u64;
        for name in [
            "complexity_work_reads",
            "complexity_work_parses",
            "dead-symbols_facts_reads",
            "dead-symbols_facts_parses",
            "dead-symbols_facts_extracted",
            "stubs_work_reads",
            "stubs_work_parses",
        ] {
            assert_eq!(median_counter(name), (changed * 2) as u64, "{name}");
        }
        assert_eq!(median_counter("escapes_work_reads"), (changed * 2) as u64);
        assert_eq!(median_counter("escapes_work_parses"), changed as u64);
        assert_eq!(median_counter("dead-symbols_facts_cached"), unchanged);
        assert_eq!(median_counter("dead-symbols_facts_shared"), unchanged);
        for name in [
            "layering_facts_reads",
            "layering_facts_parses",
            "public-api_facts_reads",
            "public-api_facts_parses",
        ] {
            assert_eq!(median_counter(name), 0, "{name}");
        }
        assert!(median_counter("layering_graph_modules") > 0);
        assert_eq!(median_counter("public-api_surface_surfaces"), 4);
        assert!(median_counter("public-api_surface_items") > 0);
        assert_eq!(median_counter("public-api_surface_holes"), 0);
    }

    fn measure(&self, case: PerfCase) -> Measurements {
        self.prime();
        let changed = self.change20();
        match case {
            PerfCase::Warm20 => self.warm20(changed),
            PerfCase::Warm100 => self.warm100(),
            PerfCase::Full => self.full(changed),
        }
    }

    fn prime(&self) {
        let primed = self.tree.run(&["radius"]);
        assert_eq!(primed.code, 0, "prime state: {}", primed.out);
        let primed = self.hook();
        assert_eq!(primed.code, 0, "prime survey: {}", primed.out);
        assert!(primed.out.is_empty(), "prime survey: {}", primed.out);
    }

    fn change20(&self) -> Counts {
        self.change();
        let changed = changed_counts(self.tree.root());
        assert_eq!(changed.rust, 10, "changed Rust file count");
        assert_eq!(changed.typescript, 10, "changed TypeScript file count");
        assert_eq!(changed.rust + changed.typescript, 20, "changed file count");
        changed
    }

    fn warm(&self, changed: Counts) -> Samples {
        let warm = repeat(|| self.timed_hook());
        self.dense_cache_shape_is_present(&warm, changed.rust + changed.typescript);
        warm
    }

    fn warm20(&self, changed: Counts) -> Measurements {
        let warm = self.warm(changed);
        self.dense_gate_shape_is_present(&warm);
        Measurements {
            changed,
            warm,
            ..Default::default()
        }
    }

    fn warm100(&self) -> Measurements {
        let (changed, delta) = self.change_delta();
        self.dense_gate_shape_is_present(&delta);
        self.dense_cache_shape_is_present(&delta, 100);
        Measurements {
            changed,
            warm_delta: Some((changed.rust + changed.typescript, delta)),
            ..Default::default()
        }
    }

    fn full(&self, changed: Counts) -> Measurements {
        let warm = self.warm(changed);
        let uncached = repeat(|| {
            self.remove_structural_cache();
            self.timed_hook()
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
        self.dense_gate_shape_is_present(&uncached);
        self.dense_gate_shape_is_present(&cold);
        self.dense_gate_shape_is_present(&strict);
        let resources = resources(self);
        let warm_delta = if self.profile.units.is_some() {
            let (changed, delta) = self.change_delta();
            self.dense_gate_shape_is_present(&delta);
            self.dense_cache_shape_is_present(&delta, 100);
            Some((changed.rust + changed.typescript, delta))
        } else {
            None
        };
        Measurements {
            changed,
            warm,
            uncached,
            cold,
            strict,
            warm_delta,
            resources,
        }
    }

    /// One Stop, timed, with its gate values read from the journal line it wrote.
    fn timed_hook(&self) -> Sample {
        let stops = journal(&self.tree).len();
        let started = Instant::now();
        let run = self.hook();
        let total = started.elapsed().as_millis();
        assert_eq!(run.code, 0, "warm hook: {}", run.out);
        assert!(run.out.is_empty(), "warm hook: {}", run.out);
        let lines = journal(&self.tree);
        assert!(lines.len() > stops, "warm hook wrote no journal line");
        let line = &lines[lines.len() - 1];
        let mut gates = gate_times(line);
        counters(&mut gates, "stop", &line["timing"], "", &STOP_TIMING);
        Sample { total, gates }
    }

    fn remove_structural_cache(&self) {
        let _ = std::fs::remove_dir_all(self.tree.state(STRUCTURAL_CACHE));
    }

    /// The files of the structural cache and the bytes they hold on disk.
    fn structural_cache(&self) -> (usize, u64) {
        std::fs::read_dir(self.tree.state(STRUCTURAL_CACHE))
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|entry| entry.metadata().ok())
            .fold((0, 0), |(files, bytes), held| {
                (files + 1, bytes + held.len())
            })
    }

    fn change(&self) {
        self.change_range(0, 10);
    }

    fn change_delta(&self) -> (Counts, Samples) {
        self.change_range(10, 50);
        let changed = changed_counts(self.tree.root());
        assert_eq!(changed.rust, 50, "scaled changed Rust file count");
        assert_eq!(
            changed.typescript, 50,
            "scaled changed TypeScript file count"
        );
        assert_eq!(
            changed.rust + changed.typescript,
            100,
            "scaled changed file count"
        );
        let primed = self.hook();
        assert_eq!(primed.code, 0, "scaled survey: {}", primed.out);
        assert!(primed.out.is_empty(), "scaled survey: {}", primed.out);
        (changed, repeat(|| self.timed_hook()))
    }

    fn change_range(&self, from: usize, to: usize) {
        for offset in from..to {
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

/// The layering policy of the dense rows: one layer per language, every cycle judged, and the
/// files whose generated imports name no module left out of scope.
fn layering_policy() -> Value {
    json!({
        "in": ["rust/src", "web/src"],
        "except": [
            "web/src/components",
            "web/src/held_escape.ts",
            "web/src/held_stub.ts",
            "web/src/fixture.test.ts"
        ],
        "acyclic": true,
        "layers": {
            "rust": {"in": "rust/src", "can_use": []},
            "web": {"in": "web/src", "can_use": []}
        }
    })
}

fn write_project_files(tree: &Tree, scope: &str, config: &str, layering: bool) {
    let complexity = match scope {
        "rust" => r#""complexity":{"in":"rust"}"#,
        _ => "",
    };
    let config = match (config, complexity.is_empty()) {
        ("build-off", true) => r#"{"build":[]}"#.to_string(),
        ("build-off", false) => format!(r#"{{"build":[],{complexity}}}"#),
        _ => format!("{{{complexity}}}"),
    };
    let mut config: Value = serde_json::from_str(&config).unwrap_or_default();
    assert!(config.is_object(), "{config}");
    if layering {
        config["layering"] = layering_policy();
    }
    tree.write("klin.json", &config.to_string());
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
        "{\"name\":\"fixture-web\",\"private\":true,\"version\":\"0.1.0\",\"exports\":\"./src/index.ts\"}\n",
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

fn print_rows(fixture: &Fixture, rows: &Measurements, case: PerfCase) {
    let counts = count_paths(git_paths(fixture.tree.root(), ["ls-files", "-z"]));
    let changed = rows.changed;
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
    print_samples(size, rows, case);
    match case {
        PerfCase::Full => print_resources(&rows.resources),
        _ => {
            let (files, bytes) = fixture.structural_cache();
            println!("structural cache: files={files}, bytes={bytes}");
            let experiment = worktree_experiment(fixture.tree.root())
                .into_iter()
                .map(|(name, ms)| format!("{name}={ms}"))
                .collect::<Vec<_>>()
                .join(", ");
            println!("worktree experiment: iterations={ITERATIONS}, {experiment}");
        }
    }
    println!("note: hook timings exclude the project's build command");
    println!("klin version: {}", env!("CARGO_PKG_VERSION"));
    println!(
        "machine: {}/{}",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
}

fn print_samples(size: usize, rows: &Measurements, case: PerfCase) {
    if case != PerfCase::Warm100 {
        println!(
            "{} warm hook: cache=warm, iterations={ITERATIONS}, median_ms={}, {}, project_build=excluded",
            size,
            median(&rows.warm.total),
            gate_medians(&rows.warm)
        );
    }
    if case == PerfCase::Full {
        println!(
            "{} warm hook without the structural cache: cache=warm, structural_cache=removed, iterations={ITERATIONS}, median_ms={}, {}, project_build=excluded",
            size,
            median(&rows.uncached.total),
            gate_medians(&rows.uncached)
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
    }
    if case != PerfCase::Warm20
        && let Some((changed, samples)) = &rows.warm_delta
    {
        println!(
            "{} warm hook, changed_files={changed}: cache=warm, iterations={ITERATIONS}, median_ms={}, {}, project_build=excluded",
            size,
            median(&samples.total),
            gate_medians(samples)
        );
    }
}

/// The structural cache the cold rows' `cache clean` removed is written again by one Stop first,
/// so its size and the warm peak describe a Stop that reads it.
fn resources(fixture: &Fixture) -> Resources {
    let primed = fixture.hook();
    assert_eq!(primed.code, 0, "structural cache prime: {}", primed.out);
    let (files, bytes) = fixture.structural_cache();
    let warm = peak_rss(fixture, &["gate", "--hook", "--changed"]);
    let dead_symbols = peak_rss(
        fixture,
        &["gate", "--changed", "--json", "--gate", "dead-symbols"],
    );
    let strict = peak_rss(fixture, &["gate", "--strict", "--json"]);
    fixture.remove_structural_cache();
    let uncached = peak_rss(fixture, &["gate", "--hook", "--changed"]);
    Resources {
        cache_files: files,
        cache_bytes: bytes,
        warm_rss: warm,
        uncached_rss: uncached,
        dead_symbols_rss: dead_symbols,
        strict_rss: strict,
    }
}

fn print_resources(resources: &Resources) {
    let rss = |kb: Option<u64>| kb.map_or_else(|| "unavailable".to_string(), |kb| kb.to_string());
    println!(
        "structural cache: files={}, bytes={}",
        resources.cache_files, resources.cache_bytes
    );
    println!(
        "resource: warm_hook_peak_rss_kb={}, warm_hook_without_structural_cache_peak_rss_kb={}, dead_symbols_changed_peak_rss_kb={}, strict_peak_rss_kb={}",
        rss(resources.warm_rss),
        rss(resources.uncached_rss),
        rss(resources.dead_symbols_rss),
        rss(resources.strict_rss)
    );
}

/// Peak RSS is controlled-machine evidence, not a contributor-test threshold. `/usr/bin/time`
/// is intentionally optional so the fixture remains runnable where the platform has no report.
fn peak_rss(fixture: &Fixture, args: &[&str]) -> Option<u64> {
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
    if let (true, Some((_, path))) = (args.contains(&"--hook"), &fixture.toolchain) {
        command.env("PATH", path);
    }
    let done = command
        .arg(binary())
        .args(args)
        .env("HOME", empty_home())
        .current_dir(fixture.tree.root())
        .output()
        .ok()?;
    let output = String::from_utf8_lossy(&done.stderr);
    output.lines().find_map(|line| {
        line.contains(marker)
            .then(|| {
                let value = line
                    .split_whitespace()
                    .find_map(|word| word.parse::<u64>().ok())?;
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
        work_counters(&mut times, name, gate);
        name_counters(&mut times, name, gate);
        footprint_counters(&mut times, name, gate);
    }
    times
}

/// The extraction, content, module-graph and surface groups of one gate's row.
fn work_counters(times: &mut BTreeMap<String, u64>, name: &str, gate: &Value) {
    for (group, fields) in [
        (
            "facts",
            &[
                "reads",
                "parses",
                "extracted",
                "shared",
                "cached",
                "ms",
                "cache_read_ms",
                "cache_write_ms",
            ][..],
        ),
        ("work", &["reads", "parses"][..]),
        ("graph", &["modules", "dependencies", "ms"][..]),
        (
            "surface",
            &["surfaces", "items", "measured", "opaque", "holes", "ms"][..],
        ),
    ] {
        counters(times, name, &gate[group], group, fields);
    }
}

/// The name-evidence group of `dead-symbols` and `reachability`, each tree apart, with the
/// parts of the whole base's layout on the row that laid it out. #199, #202.
fn name_counters(times: &mut BTreeMap<String, u64>, name: &str, gate: &Value) {
    counters(
        times,
        name,
        &gate["names"],
        "names",
        &["base_ms", "lost_ms"],
    );
    counters(
        times,
        name,
        &gate["names"]["layout"],
        "names_layout",
        &LAYOUT,
    );
    for tree in ["before", "after"] {
        counters(
            times,
            name,
            &gate["names"][tree],
            &format!("names_{tree}"),
            &TREE_NAMES,
        );
    }
}

/// The representation counters and the type sizes of the facts one run held. #200.
fn footprint_counters(times: &mut BTreeMap<String, u64>, name: &str, gate: &Value) {
    counters(times, name, &gate["footprint"], "footprint", &FOOTPRINT);
    counters(
        times,
        name,
        &gate["footprint"]["sizes"],
        "footprint_size",
        &TYPE_SIZES,
    );
}

/// The population, sparsity and byte counters of the facts one run holds. #200.
const FOOTPRINT: [&str; 24] = [
    "files",
    "declarations",
    "references",
    "imports",
    "module_declarations",
    "exports",
    "export_leaves",
    "qualified_paths",
    "path_bytes",
    "declaration_name_bytes",
    "declaration_text_bytes",
    "reference_name_bytes",
    "signatures",
    "signature_bytes",
    "owners",
    "owner_bytes",
    "exported_aliases",
    "exported_alias_bytes",
    "nestings",
    "nesting_entries",
    "nesting_bytes",
    "import_text_bytes",
    "export_text_bytes",
    "module_text_bytes",
];

const TYPE_SIZES: [&str; 7] = [
    "file_facts",
    "declaration",
    "reference",
    "import",
    "module_declaration",
    "export",
    "export_leaf",
];

/// The parts of one whole-base layout, on the row of the gate that laid it out. #202.
const LAYOUT: [&str; 7] = [
    "written",
    "worktree_add_ms",
    "changes_ms",
    "renames_ms",
    "cache_name_ms",
    "ignored_ms",
    "walk_ms",
];

/// The stop's own timing, off the journal line of a warm hook. #202.
const STOP_TIMING: [&str; 5] = [
    "total_ms",
    "build_ms",
    "lock_ms",
    "base_remove_ms",
    "base_prune_ms",
];

const TREE_NAMES: [&str; 7] = [
    "measure_ms",
    "index_ms",
    "query_ms",
    "files",
    "declarations",
    "references",
    "distinct_names",
];

/// One gate's counters of one group, each under the gate, the group and its own name.
fn counters(
    times: &mut BTreeMap<String, u64>,
    name: &str,
    held: &Value,
    group: &str,
    fields: &[&str],
) {
    let group = match group.is_empty() {
        true => String::new(),
        false => format!("{group}_"),
    };
    for field in fields {
        if let Some(value) = held[*field].as_u64() {
            times.insert(format!("{name}_{group}{field}"), value);
        }
    }
}

/// The git commands a lighter whole-base layout would run, each timed on the fixture's
/// repository apart from every stop, `ITERATIONS` times, median. The whole detached checkout and
/// its removal are what klin runs today; the no-checkout worktree, the `read-tree` into it, the
/// `ls-files --stage` listing, the `checkout-index` of the changed files' base paths, its
/// removal and the prune are candidate E of #202. An estimate of a candidate, not klin.
fn worktree_experiment(root: &Path) -> BTreeMap<String, u64> {
    let changed: Vec<String> = git_paths(root, ["diff", "--name-only", "-z", "HEAD"])
        .into_iter()
        .map(|path| String::from_utf8_lossy(&path).into_owned())
        .collect();
    let mut samples: BTreeMap<&str, Vec<u64>> = BTreeMap::new();
    for round in 0..ITERATIONS {
        let dir = std::env::temp_dir().join(format!(
            "klin-worktree-experiment-{}-{round}",
            std::process::id()
        ));
        experiment_round(root, &dir, &changed, &mut samples);
        let _ = std::fs::remove_dir_all(&dir);
    }
    samples
        .into_iter()
        .map(|(name, values)| (format!("worktree_{name}_ms"), median(&values)))
        .collect()
}

fn experiment_round<'a>(
    root: &Path,
    dir: &Path,
    changed: &[String],
    samples: &mut BTreeMap<&'a str, Vec<u64>>,
) {
    let at = dir.to_string_lossy().into_owned();
    let mut step = |name: &'a str, cwd: &Path, args: &[&str]| {
        samples.entry(name).or_default().push(git_ms(cwd, args));
    };
    step(
        "add_whole",
        root,
        &["worktree", "add", "--detach", "--quiet", &at, "HEAD"],
    );
    step(
        "remove_whole",
        root,
        &["worktree", "remove", "--force", &at],
    );
    step(
        "add_no_checkout",
        root,
        &[
            "worktree",
            "add",
            "--detach",
            "--no-checkout",
            "--quiet",
            &at,
            "HEAD",
        ],
    );
    step("read_tree", dir, &["read-tree", "HEAD"]);
    step("ls_files_stage", dir, &["ls-files", "-z", "--stage"]);
    let mut checkout = vec!["checkout-index", "-f", "--"];
    checkout.extend(changed.iter().map(String::as_str));
    step("checkout_index_changed", dir, &checkout);
    step(
        "remove_no_checkout",
        root,
        &["worktree", "remove", "--force", &at],
    );
    step("prune", root, &["worktree", "prune"]);
}

fn git_ms(cwd: &Path, args: &[&str]) -> u64 {
    let started = Instant::now();
    let status = Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    let elapsed = started.elapsed().as_millis();
    assert!(
        status.is_ok_and(|status| status.success()),
        "git {} failed in {}",
        args.join(" "),
        cwd.display()
    );
    u64::try_from(elapsed).unwrap_or(u64::MAX)
}

#[test]
fn the_worktree_experiment_runs_every_command_it_times() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write("src/lib.rs", "pub fn api() {}\n");
    tree.write("src/held.rs", "pub fn held() {}\n");
    tree.base();
    tree.write("src/lib.rs", "pub fn api() { held() }\n");

    let medians = worktree_experiment(tree.root());

    let names: Vec<&str> = medians.keys().map(String::as_str).collect();
    assert_eq!(
        names,
        [
            "worktree_add_no_checkout_ms",
            "worktree_add_whole_ms",
            "worktree_checkout_index_changed_ms",
            "worktree_ls_files_stage_ms",
            "worktree_prune_ms",
            "worktree_read_tree_ms",
            "worktree_remove_no_checkout_ms",
            "worktree_remove_whole_ms",
        ]
    );
    let listed = git_paths(tree.root(), ["worktree", "list", "--porcelain", "-z"]);
    assert_eq!(
        listed
            .iter()
            .filter(|line| line.starts_with(b"worktree "))
            .count(),
        1,
        "the experiment leaves no worktree behind"
    );
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
