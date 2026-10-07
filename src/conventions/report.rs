//! `klin policy conventions [NAME]`: what each convention forbids, where, how klin reads its
//! pattern, and the fix. It reads the configuration and walks the tree's paths, and it parses no
//! source and compares nothing with the base, so it runs no check. Spec 8.4, 11.6, ADR 0037.

use crate::config::Config;
use crate::error::Error;
use crate::project::Project;
use crate::scope::Selector;
use crate::syntax::pattern;

use super::rules::{
    Code, Convention, Hole, Matcher, Place, Unresolved, conventions, holes, joined, resolved,
    walked,
};

/// Every convention explained, or only the one `NAME` names.
pub fn explain(project: &Project, named: Option<&str>) -> Result<Vec<String>, Error> {
    let config = &project.config;
    let conventions = conventions(config)?;
    let places = walked(config, project.tree())?;
    let holes = holes(&conventions, &places);
    if let Some(name) = named
        && !conventions.iter().any(|convention| convention.name == name)
    {
        return Err(unknown(config, name, &conventions));
    }
    Ok(conventions
        .iter()
        .filter(|convention| named.is_none_or(|name| name == convention.name))
        .flat_map(|convention| explained(config, convention, &places, &holes))
        .collect())
}

fn explained(
    config: &Config,
    convention: &Convention,
    places: &[Place],
    holes: &[Hole],
) -> Vec<String> {
    let mut out = vec![
        format!("{}:", convention.name),
        format!("  {}", forbids(convention)),
    ];
    match resolved(convention, places) {
        Ok(rule) => out.extend(
            rule.code
                .as_ref()
                .map(|code| format!("  {}", read_as(config, code))),
        ),
        Err(problem) => out.push(format!("  {}", Unresolved::told(&problem))),
    }
    out.extend(empty(holes, convention).map(|line| format!("  {line}")));
    out.push(format!("  Fix: {}", convention.remedy));
    out
}

fn empty<'a>(holes: &'a [Hole], convention: &'a Convention) -> impl Iterator<Item = String> + 'a {
    holes
        .iter()
        .filter(move |hole| hole.convention == convention.name)
        .map(|hole| {
            format!(
                "The \"{}\" path {} matches nothing in the tree.",
                hole.key, hole.path
            )
        })
}

fn unknown(config: &Config, name: &str, conventions: &[Convention]) -> Error {
    let names: Vec<&str> = conventions
        .iter()
        .map(|convention| convention.name.as_str())
        .collect();
    Error(format!(
        "{}: no convention is named \"{name}\" — the conventions are {}",
        config.file.display(),
        joined(&names, "and")
    ))
}

fn forbids(convention: &Convention) -> String {
    let written = &convention.written;
    let what = match convention.matcher {
        Matcher::Text(_) => format!("the text \"{written}\""),
        Matcher::Code(_) => written.clone(),
        Matcher::Files(_) => format!("files matching {written}"),
    };
    format!("Forbids {what} in {}.", scope(convention))
}

fn scope(convention: &Convention) -> String {
    let names = |paths: &[Selector]| {
        let names: Vec<&str> = paths.iter().map(Selector::as_str).collect();
        joined(&names, "and")
    };
    let within = match convention.within.is_empty() {
        true => "the repository".to_string(),
        false => names(&convention.within),
    };
    match convention.except.is_empty() {
        true => within,
        false => format!("{within} except {}", names(&convention.except)),
    }
}

/// What a code pattern reads as, in which language, and how the language was settled.
fn read_as(config: &Config, code: &Code) -> String {
    let settled = match code.derived {
        true => "derived from the files in scope".to_string(),
        false => format!(
            "pinned in {}",
            config.file.file_name().map_or_else(
                || config.file.display().to_string(),
                |name| name.to_string_lossy().to_string()
            )
        ),
    };
    format!(
        "Reads as {} in {}. The language is {settled}.",
        joined(&code.pattern.readings(), "or"),
        pattern::called(code.language)
    )
}
