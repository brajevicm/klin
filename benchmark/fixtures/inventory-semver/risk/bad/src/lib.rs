use std::cmp::Ordering;

struct Version<'a> {
    core: Vec<u64>,
    prerelease: Option<&'a str>,
}

fn parse(version: &str) -> Version<'_> {
    let without_build = version.split('+').next().unwrap_or(version);
    let (release, prerelease) = match without_build.split_once('-') {
        Some((release, prerelease)) => (release, Some(prerelease)),
        None => (without_build, None),
    };
    Version {
        core: release.split('.').map(|part| part.parse().unwrap_or(0)).collect(),
        prerelease,
    }
}

/// Orders two release numbers, earliest first.
pub fn compare(a: &str, b: &str) -> Ordering {
    let (a, b) = (parse(a), parse(b));
    a.core.cmp(&b.core).then_with(|| match (a.prerelease, b.prerelease) {
        (None, None) => Ordering::Equal,
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (Some(x), Some(y)) => x.cmp(y),
    })
}
