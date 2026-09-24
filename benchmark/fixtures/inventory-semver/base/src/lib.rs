use std::cmp::Ordering;

/// Orders two release numbers, earliest first.
pub fn compare(a: &str, b: &str) -> Ordering {
    let core = |version: &str| -> Vec<u64> {
        let release = version.split('-').next().unwrap_or(version);
        release.split('.').map(|part| part.parse().unwrap_or(0)).collect()
    };
    core(a).cmp(&core(b))
}
