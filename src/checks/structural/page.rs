//! The module scripts of a Vite app's `index.html`, which name the app's entries. Spec 8.4.

use crate::modules::resolver::joined;

const OPEN: &str = "<script";
const CLOSE: &str = "</script";
const COMMENT: &str = "<!--";
const COMMENT_END: &str = "-->";
const OFF_SITE: &[&str] = &["http://", "https://", "//"];

/// Why klin cannot read what an `index.html` names: an inline module script with code, or a
/// `src` that holds a `%...%` placeholder.
pub(super) enum Unread {
    Inline,
    Placeholder,
}

/// One `<script type="module">` outside an HTML comment: the `src` it names, or code it holds
/// inline.
enum Script {
    Src(String),
    Inline,
}

/// The tree paths the module scripts of the `index.html` at a package directory name, in order.
/// An inline module script gives the reason before a placeholder does, wherever each sits.
pub(super) fn named(html: &str, package: &str) -> Result<Vec<String>, Unread> {
    let mut srcs = Vec::new();
    for script in modules(html) {
        match script {
            Script::Src(src) => srcs.push(src),
            Script::Inline => return Err(Unread::Inline),
        }
    }
    if srcs.iter().any(|src| placeholder(src)) {
        return Err(Unread::Placeholder);
    }
    Ok(srcs.iter().filter_map(|src| path(src, package)).collect())
}

/// Every module script of an HTML text, in order. A script of any other `type`, or none, and an
/// inline module script that holds only whitespace, is none.
fn modules(html: &str) -> Vec<Script> {
    let lower = html.to_ascii_lowercase();
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(found) = lower[at..].find('<') {
        let start = at + found;
        at = start + 1;
        if lower[start..].starts_with(COMMENT) {
            at = lower[start..]
                .find(COMMENT_END)
                .map_or(html.len(), |end| start + end + COMMENT_END.len());
            continue;
        }
        let head = start + OPEN.len();
        let tag = lower[start..].starts_with(OPEN)
            && lower[head..].starts_with(|c: char| c.is_whitespace() || c == '>' || c == '/');
        if !tag {
            continue;
        }
        let (attributes, body) = attributes(&html[head..]);
        let open = html.len() - body.len();
        let close = lower[open..]
            .find(CLOSE)
            .map_or(html.len(), |end| open + end);
        at = close;
        out.extend(script(&attributes, &html[open..close]));
    }
    out
}

fn script(attributes: &[(String, &str)], code: &str) -> Option<Script> {
    let value = |name: &str| {
        attributes
            .iter()
            .find(|(held, _)| held == name)
            .map(|(_, value)| *value)
    };
    if !value("type").is_some_and(|kind| kind.trim().eq_ignore_ascii_case("module")) {
        return None;
    }
    match value("src") {
        Some(src) => Some(Script::Src(src.to_string())),
        None => (!code.trim().is_empty()).then_some(Script::Inline),
    }
}

/// The attributes of a tag, each name lowercased, and the text after the tag's `>`. A value may
/// use double quotes, single quotes or none.
fn attributes(tag: &str) -> (Vec<(String, &str)>, &str) {
    let mut out = Vec::new();
    let mut rest = tag;
    loop {
        rest = rest.trim_start_matches(|c: char| c.is_whitespace() || c == '/');
        if let Some(after) = rest.strip_prefix('>') {
            return (out, after);
        }
        if rest.is_empty() {
            return (out, rest);
        }
        let end = rest
            .find(|c: char| c.is_whitespace() || matches!(c, '=' | '>' | '/'))
            .unwrap_or(rest.len());
        let name = rest[..end].to_ascii_lowercase();
        rest = rest[end..].trim_start();
        let value = match rest.strip_prefix('=') {
            Some(after) => {
                let (value, after) = value(after.trim_start());
                rest = after;
                value
            }
            None => "",
        };
        out.push((name, value));
    }
}

fn value(text: &str) -> (&str, &str) {
    match text.chars().next() {
        Some(quote @ ('"' | '\'')) => {
            let body = &text[1..];
            let end = body.find(quote).unwrap_or(body.len());
            (&body[..end], body.get(end + 1..).unwrap_or(""))
        }
        _ => {
            let end = text
                .find(|c: char| c.is_whitespace() || c == '>')
                .unwrap_or(text.len());
            (&text[..end], &text[end..])
        }
    }
}

/// The tree path a module script's `src` names, less any `?query` or `#hash`. A leading `/`
/// resolves from the package directory, Vite's default `root`, and any other path from the HTML
/// file's directory, which is the same directory. A URL off the site names none.
fn path(src: &str, package: &str) -> Option<String> {
    let path = src.split(['?', '#']).next()?.trim();
    if OFF_SITE.iter().any(|scheme| path.starts_with(scheme)) {
        return None;
    }
    joined(package, path)
}

/// Whether a `src` holds a `%...%` placeholder, which Vite replaces from env variables.
fn placeholder(src: &str) -> bool {
    let parts: Vec<&str> = src.split('%').collect();
    parts.len() > 2
        && parts[1..parts.len() - 1]
            .iter()
            .any(|part| !part.is_empty() && !part.contains(char::is_whitespace))
}
