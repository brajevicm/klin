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

fn rank(part: &str) -> (u8, u64) {
    match part.parse::<u64>() {
        Ok(number) => (0, number),
        Err(_) => (1, 0),
    }
}

/// Orders two dot-separated prerelease identifiers: a numeric one by its number and before any
/// word, a word by its ASCII order, and a shorter list first when one is the start of the other.
fn identifiers(a: &str, b: &str) -> Ordering {
    a.split('.')
        .map(|part| (rank(part), part))
        .cmp(b.split('.').map(|part| (rank(part), part)))
}

/// Orders two release numbers, earliest first.
pub fn compare(a: &str, b: &str) -> Ordering {
    let (a, b) = (parse(a), parse(b));
    a.core.cmp(&b.core).then_with(|| match (a.prerelease, b.prerelease) {
        (None, None) => Ordering::Equal,
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (Some(x), Some(y)) => identifiers(x, y),
    })
}
