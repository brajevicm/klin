"""Prepare an isolated copy for the existing performance harness; no shipped edits."""
import shutil
import subprocess
import sys
from pathlib import Path

here = Path(__file__).resolve().parent
repo = here.parent.parent
checkout = Path(sys.argv[1]).resolve()
checkout.mkdir(parents=True, exist_ok=True)
archive = subprocess.Popen(['git', 'archive', 'c1805539'], cwd=repo, stdout=subprocess.PIPE)
subprocess.run(['tar', '-x', '-C', str(checkout)], stdin=archive.stdout, check=True)
archive.stdout.close()
assert archive.wait() == 0
shutil.copytree(here / 'proto/src', checkout / 'src/dup_research', dirs_exist_ok=True)
source = checkout / 'src/main.rs'
s = source.read_text()
s = '#[allow(dead_code)]\n#[path = "dup_research/main.rs"]\nmod dup_research;\nmod dup_accounting;\n' + s
s = s.replace('fn main() -> ExitCode {', 'fn main() -> ExitCode {\n    let _dup_report = dup_accounting::Report;')
source.write_text(s)
source = checkout / 'src/syntax/structural/mod.rs'
s = source.read_text().replace('fn measured(file: &ParsedFile, names: &mut Names) -> Result<Outcome, Error> {', '''fn measured(file: &ParsedFile, names: &mut Names) -> Result<Outcome, Error> {
    if std::env::var("KLIN_DUP_RESEARCH").as_deref() == Ok("1") {
        let started = std::time::Instant::now();
        let (tokens, prints) = crate::dup_research::consume_shared(file.path, file.bytes(), file.root());
        crate::dup_accounting::record(tokens, prints, started.elapsed());
    }''')
source.write_text(s)
(checkout / 'src/dup_accounting.rs').write_text('''use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
static FILES: AtomicU64 = AtomicU64::new(0);
static TOKENS: AtomicU64 = AtomicU64::new(0);
static PRINTS: AtomicU64 = AtomicU64::new(0);
static NANOS: AtomicU64 = AtomicU64::new(0);
pub fn record(tokens: usize, prints: usize, elapsed: Duration) {
    if tokens == 0 { return; }
    FILES.fetch_add(1, Ordering::Relaxed);
    TOKENS.fetch_add(tokens as u64, Ordering::Relaxed);
    PRINTS.fetch_add(prints as u64, Ordering::Relaxed);
    NANOS.fetch_add(elapsed.as_nanos() as u64, Ordering::Relaxed);
}
pub struct Report;
impl Drop for Report {
    fn drop(&mut self) {
        if std::env::var_os("KLIN_DUP_RESEARCH_REPORT").is_none() { return; }
        eprintln!("DUP_RESEARCH {}", serde_json::json!({
            "shared_files": FILES.load(Ordering::Relaxed),
            "tokens": TOKENS.load(Ordering::Relaxed),
            "fingerprints": PRINTS.load(Ordering::Relaxed),
            "normalize_ms": NANOS.load(Ordering::Relaxed) as f64 / 1e6,
            "extra_parses": 0, "unchanged_source_reads": 0,
            "whole_tree_walks": 0, "external_processes": 0,
            "scope": "shared normalization/fingerprinting lower bound; excludes index and lineage"
        }));
    }
}
''')
source = checkout / 'tests/performance.rs'
s = source.read_text() + '''
#[test]
#[ignore = "isolated exact-region shared-tree research"]
fn duplication_shared_tree_research() {
    let fixture = Fixture::with_profile(5_000, DENSE_1M);
    fixture.prime();
    fixture.change20();
    for phase in ["warm20", "warm100", "cold"] {
        if phase == "warm100" { fixture.change_delta(); }
        for iteration in 0..5 {
            for mode in if iteration % 2 == 0 { ["0", "1"] } else { ["1", "0"] } {
                if phase == "cold" {
                    let run = fixture.tree.run(&["cache", "clean"]);
                    assert_eq!(run.code, 0, "{}", run.out);
                }
                let started = Instant::now();
                let run = fixture.tree.run_with(
                    &[("KLIN_DUP_RESEARCH", mode), ("KLIN_DUP_RESEARCH_REPORT", "1")],
                    &["gate", "--hook", "--changed"],
                );
                let wall_ms = started.elapsed().as_secs_f64() * 1000.0;
                assert_eq!(run.code, 0, "{}", run.out);
                let counters: serde_json::Value = serde_json::from_str(
                    run.out.lines().find_map(|line| line.strip_prefix("DUP_RESEARCH ")).unwrap()
                ).unwrap();
                println!("DUP_AB {}", serde_json::json!({
                    "phase": phase, "iteration": iteration, "enabled": mode == "1",
                    "end_to_end_ms": wall_ms, "counters": counters
                }));
            }
        }
    }
}
'''
source.write_text(s)
print(checkout)
