mod cohorts;
mod storage;
#[allow(dead_code)]
mod reference {
    include!("../../../duplication-speed-2026-10-05/proto/src/main.rs");
    pub fn measure(root: &std::path::Path, mode: &str, map: Option<&str>) {
        fn visit(root: &std::path::Path, dir: &std::path::Path, paths: &mut Vec<String>) {
            for entry in std::fs::read_dir(dir).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if name.starts_with('.')
                    || matches!(name.as_ref(), "target" | "node_modules" | "dist" | "build")
                {
                    continue;
                }
                if entry.file_type().unwrap().is_dir() {
                    visit(root, &path, paths);
                } else {
                    let relative = path
                        .strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/");
                    if Lang::of(&relative).is_some() {
                        paths.push(relative);
                    }
                }
            }
        }
        let mut paths = Vec::new();
        visit(root, root, &mut paths);
        paths.sort();
        let mut shared = Shared::new();
        let mut files = Vec::new();
        for path in paths {
            let source = std::fs::read(root.join(&path)).unwrap();
            let lang = Lang::of(&path).unwrap();
            let syntax = shared.parse(lang, &source);
            let parsed = normalized(lang, &path, &source, &syntax, true);
            files.push(super::storage::FileTokens {
                path,
                tokens: parsed
                    .raw
                    .unwrap()
                    .into_iter()
                    .zip(parsed.safe)
                    .map(|(text, safe)| safe.then_some(text))
                    .collect(),
                rows: parsed.rows,
                unsafe_units: parsed.unsafe_units,
                error: parsed.error,
            });
        }
        let count: usize = files.iter().map(|f| f.tokens.len()).sum();
        if mode == "cohorts" {
            let started = Instant::now();
            let regions = super::cohorts::regions(&files, 60);
            let matcher_ms = started.elapsed().as_secs_f64() * 1000.;
            let occurrences: usize = regions.iter().map(|r| r.occurrences.len()).sum();
            if let Some(path) = map {
                let rows: Vec<_> = regions
                    .iter()
                    .map(|r| {
                        let text: Vec<_> = r
                            .text
                            .iter()
                            .map(|v| {
                                serde_json::to_string(
                                    &v.iter().map(|b| format!("{b:02x}")).collect::<String>(),
                                )
                                .unwrap()
                            })
                            .collect();
                        let occurrences: Vec<_> = r
                            .occurrences
                            .iter()
                            .map(|&(f, start)| {
                                format!(
                                    "[{},{},{}]",
                                    serde_json::to_string(&files[f].path).unwrap(),
                                    start,
                                    start + r.text.len()
                                )
                            })
                            .collect();
                        format!(
                            "{{\"text\":[{}],\"occurrences\":[{}]}}",
                            text.join(","),
                            occurrences.join(",")
                        )
                    })
                    .collect();
                std::fs::write(path, format!("[{}]", rows.join(","))).unwrap();
            }
            println!(
                "{{\"files\":{},\"tokens\":{count},\"threshold\":60,\"families\":{},\"occurrences\":{occurrences},\"matcher_ms\":{matcher_ms},\"normalization_timed\":false,\"full_rebuild\":true,\"lineage_included\":false}}",
                files.len(),
                regions.len()
            );
            return;
        }
        let mut timings = Vec::new();
        let mut bytes = 0;
        let mut breakdown = String::new();
        for iteration in 0..5 {
            let started = Instant::now();
            let (packed, parts) = super::storage::pack_with_breakdown(&files);
            breakdown = format!(
                "{{\"dictionary\":{},\"chains\":{},\"rows\":{},\"metadata\":{},\"container\":{}}}",
                parts.dictionary, parts.chains, parts.rows, parts.metadata, parts.container
            );
            let pack_ms = started.elapsed().as_secs_f64() * 1000.;
            let started = Instant::now();
            let unpacked = super::storage::unpack(&packed).unwrap();
            let unpack_ms = started.elapsed().as_secs_f64() * 1000.;
            assert_eq!(unpacked, files);
            bytes = packed.len();
            timings.push(format!(
                "{{\"iteration\":{iteration},\"pack_ms\":{pack_ms},\"unpack_ms\":{unpack_ms}}}"
            ));
        }
        println!(
            "{{\"files\":{},\"tokens\":{count},\"packed_bytes\":{bytes},\"breakdown\":{breakdown},\"candidate32_1m_reference_plus_evidence_bytes\":{},\"roundtrip_equal\":true,\"lineage_included\":false,\"normalization_timed\":false,\"disk_io_timed\":false,\"iterations\":[{}]}}",
            files.len(),
            bytes + 2_184_542,
            timings.join(",")
        );
    }
}
fn storage_check() {
    use storage::{FileTokens, pack, unpack};
    let files: Vec<_> = [0, 1, 3, 4, 100, 1000]
        .into_iter()
        .map(|n| FileTokens {
            path: format!("{n}.rs"),
            tokens: (0..n)
                .map(|i| {
                    if i % 17 == 0 {
                        None
                    } else {
                        Some(format!("rust:{}", i % 5).into_bytes())
                    }
                })
                .collect(),
            rows: (0..n).map(|i| (i / 4) as u32).collect(),
            unsafe_units: 2,
            error: n == 3,
        })
        .collect();
    assert_eq!(unpack(&pack(&[])).unwrap(), Vec::<FileTokens>::new());
    let packed = pack(&files);
    assert_eq!(unpack(&packed).unwrap(), files);
    for n in 0..packed.len() {
        assert!(unpack(&packed[..n]).is_err());
    }
    for n in 0..packed.len() {
        let mut corrupt = packed.clone();
        corrupt[n] ^= 1;
        assert!(unpack(&corrupt).is_err());
    }
    println!(
        "storage-check: empty/unsafe/overlap/rows/metadata roundtrips, every truncated prefix and single-byte mutation passed"
    );
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("storage-check") {
        storage_check();
        return;
    }
    assert!(matches!(
        args.get(1).map(String::as_str),
        Some("storage" | "cohorts")
    ));
    reference::measure(
        std::path::Path::new(&args[2]),
        &args[1],
        args.get(3).map(String::as_str),
    );
}
