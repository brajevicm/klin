//! Code patterns: a piece of code a person writes with `$NAME` and `$$$ARGS` holes, and the
//! places a parsed file holds it. The matching engine is `ast-grep-core`, and every type of it
//! stays in this module. A pattern language is a logical language of the one registry in
//! `syntax`: a pattern compiles once per grammar variant of that language, so `.ts` and `.tsx`
//! files are both TypeScript.
//!
//! A person writes the fragment they mean, such as `_ => Ok(0)` or `RefCell<Records>`, and the
//! adapter reads it in every place its language lists: as written, as an expression, as a match
//! arm, as a type. Every clean reading is kept, and a file matches through any of them, so klin
//! never picks one reading over another. Adding a language is one adapter. ADR 0006, ADR 0037.

use std::borrow::Cow;
use std::collections::BTreeMap;

use ast_grep_core::matcher::{PatternBuilder, PatternNode};
use ast_grep_core::tree_sitter::{LanguageExt, StrDoc, TSLanguage};
use ast_grep_core::{AstGrep, PatternError};

use crate::config::key;
use crate::syntax::{LANGUAGES, Language, LanguageId, ParsedFile, tree_of, walk};

/// What a language's code patterns need: the character a hole is written with once a pattern
/// reaches the grammar, and the places a fragment may read as. Rust reads no `$` in a name, so a
/// hole there becomes `µ` before it is parsed, and a block such as `if $COND { $$$BODY }` still
/// reads as a block.
struct Adapter {
    hole: char,
    readings: &'static [Reading],
}

/// One place a fragment may sit in a language: what a person calls it, and the code written
/// around the fragment to put it there.
struct Reading {
    called: &'static str,
    before: &'static str,
    after: &'static str,
}

const AS_WRITTEN: Reading = Reading {
    called: "code as written",
    before: "",
    after: "",
};

const RUST: Adapter = Adapter {
    hole: 'µ',
    readings: &[
        AS_WRITTEN,
        Reading {
            called: "an expression",
            before: "fn klin_context() { ",
            after: " }",
        },
        Reading {
            called: "a match arm",
            before: "match klin_context { ",
            after: " }",
        },
        Reading {
            called: "a type",
            before: "type KlinContext = ",
            after: ";",
        },
        Reading {
            called: "a field",
            before: "struct KlinContext { ",
            after: " }",
        },
    ],
};

const TYPESCRIPT: Adapter = Adapter {
    hole: '$',
    readings: &[
        AS_WRITTEN,
        Reading {
            called: "a type",
            before: "type KlinContext = ",
            after: ";",
        },
    ],
};

fn adapter(id: LanguageId) -> Option<&'static Adapter> {
    match id {
        LanguageId::Rust => Some(&RUST),
        LanguageId::TypeScript => Some(&TYPESCRIPT),
        _ => None,
    }
}

/// One grammar variant a pattern compiles against.
#[derive(Clone)]
struct Grammar {
    language: &'static Language,
    adapter: &'static Adapter,
}

impl ast_grep_core::Language for Grammar {
    fn pre_process_pattern<'q>(&self, query: &'q str) -> Cow<'q, str> {
        if self.adapter.hole == '$' {
            return Cow::Borrowed(query);
        }
        let mut out = String::with_capacity(query.len());
        let mut dollars = 0;
        for next in query.chars() {
            if next == '$' {
                dollars += 1;
                continue;
            }
            let named = next.is_ascii_uppercase() || next == '_' || dollars == 3;
            out.extend(std::iter::repeat_n(self.sigil(named), dollars));
            dollars = 0;
            out.push(next);
        }
        out.extend(std::iter::repeat_n(self.sigil(dollars == 3), dollars));
        Cow::Owned(out)
    }

    fn expando_char(&self) -> char {
        self.adapter.hole
    }

    fn kind_to_id(&self, kind: &str) -> u16 {
        self.get_ts_language().id_for_node_kind(kind, true)
    }

    fn field_to_id(&self, field: &str) -> Option<u16> {
        self.get_ts_language()
            .field_id_for_name(field)
            .map(|id| id.get())
    }

    fn build_pattern(
        &self,
        builder: &PatternBuilder,
    ) -> Result<ast_grep_core::Pattern, PatternError> {
        builder.build(|source| self.document(source))
    }
}

impl LanguageExt for Grammar {
    fn get_ts_language(&self) -> TSLanguage {
        (self.language.grammar)()
    }
}

impl Grammar {
    fn sigil(&self, named: bool) -> char {
        match named {
            true => self.adapter.hole,
            false => '$',
        }
    }

    /// A text the grammar reads cleanly: no node it could not read and no token it had to supply.
    /// A pattern built around either matches nothing and passes in silence.
    fn document(&self, source: &str) -> Result<StrDoc<Grammar>, String> {
        let tree = tree_of(source, self.language)
            .map_err(|why| why.to_string())?
            .ok_or_else(|| format!("the {} grammar could not read it", self.language.name))?;
        let mut clean = true;
        walk(tree.root_node(), &mut |node| {
            clean &= !node.is_error() && !node.is_missing();
        });
        if !clean {
            return Err(format!(
                "the {} grammar could not read it",
                self.language.name
            ));
        }
        Ok(StrDoc {
            src: source.to_string(),
            lang: self.clone(),
            tree,
        })
    }

    /// Every place this grammar reads the fragment cleanly, as the pattern each reading compiles
    /// to. A reading that is only a hole would match every node, so it is not one.
    fn readings(&self, text: &str) -> Vec<(&'static str, ast_grep_core::Pattern)> {
        self.adapter
            .readings
            .iter()
            .filter_map(|reading| Some((reading.called, self.read_as(text, reading)?)))
            .filter(|(_, pattern)| !matches!(pattern.node, PatternNode::MetaVar { .. }))
            .collect()
    }

    /// The fragment read in one place, and `None` when the grammar cannot read it cleanly there or
    /// no one node is the whole fragment.
    fn read_as(&self, text: &str, reading: &Reading) -> Option<ast_grep_core::Pattern> {
        if reading.before.is_empty() && reading.after.is_empty() {
            return ast_grep_core::Pattern::try_new(text, self.clone()).ok();
        }
        let fragment = ast_grep_core::Language::pre_process_pattern(self, text);
        let placed = format!("{}{fragment}{}", reading.before, reading.after);
        let root = AstGrep::doc(self.document(&placed).ok()?);
        let span = reading.before.len()..reading.before.len() + fragment.len();
        let mut node = root.root().dfs().find(|node| node.range() == span)?;
        while node.children().len() == 1 {
            node = node.child(0)?;
        }
        Some(ast_grep_core::Pattern::from(node))
    }
}

/// One code pattern, read in every place its language lists and compiled against every grammar
/// variant of that language. A variant may hold no reading: a JSX pattern is TSX and not plain
/// TypeScript, and no `.ts` file can hold it, so it matches nothing there.
pub struct Pattern {
    compiled: Vec<(Grammar, Vec<(&'static str, ast_grep_core::Pattern)>)>,
}

impl Pattern {
    /// The pattern in the named language, or what makes it one no file could match: no grammar
    /// variant of the language reads it in any place.
    pub fn compile(text: &str, language: &str) -> Result<Pattern, Unread> {
        let variants: Vec<Grammar> = LANGUAGES
            .iter()
            .filter(|row| row.names.first() == Some(&language))
            .filter_map(|row| {
                Some(Grammar {
                    language: row,
                    adapter: adapter(row.id)?,
                })
            })
            .collect();
        let Some(first) = variants.first().cloned() else {
            return Err(Unread {
                language: called(language),
                tried: Vec::new(),
            });
        };
        let compiled: Vec<(Grammar, Vec<(&'static str, ast_grep_core::Pattern)>)> = variants
            .into_iter()
            .map(|grammar| {
                let readings = grammar.readings(text);
                (grammar, readings)
            })
            .collect();
        if compiled.iter().all(|(_, readings)| readings.is_empty()) {
            return Err(unread(&first));
        }
        Ok(Pattern { compiled })
    }

    /// What the pattern reads as, in the order its language lists the places, each once.
    pub fn readings(&self) -> Vec<&'static str> {
        let mut out: Vec<&'static str> = Vec::new();
        for (called, _) in self.compiled.iter().flat_map(|(_, readings)| readings) {
            if !out.contains(called) {
                out.push(called);
            }
        }
        out
    }
}

/// What stopped a pattern: no place in its language reads it. `language` is what a person calls
/// the language, and `tried` names every place klin tried.
pub struct Unread {
    pub language: &'static str,
    pub tried: Vec<&'static str>,
}

impl std::fmt::Display for Unread {
    fn fmt(&self, out: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            out,
            "the {} grammar reads it as none of: {} — write one piece of code the language holds \
             in one of those places",
            self.language,
            self.tried.join(", ")
        )
    }
}

fn unread(grammar: &Grammar) -> Unread {
    Unread {
        language: grammar.language.name,
        tried: grammar
            .adapter
            .readings
            .iter()
            .map(|reading| reading.called)
            .collect(),
    }
}

/// What a person calls a language a code pattern is written in, such as `Rust`.
pub fn called(language: &str) -> &'static str {
    LANGUAGES
        .iter()
        .find(|row| row.names.first() == Some(&language))
        .map_or("", |row| row.name)
}

/// The names a code pattern may be written in, each with the extensions it reads.
pub fn language_extensions() -> Vec<(&'static str, String)> {
    key::extensions_by_name(
        LANGUAGES
            .iter()
            .filter(|row| adapter(row.id).is_some())
            .map(|row| (row.names[..1].to_vec(), row.extensions.to_vec())),
    )
}

/// Every name a code pattern may be written in.
pub fn languages() -> Vec<&'static str> {
    language_extensions()
        .into_iter()
        .map(|(name, _)| name)
        .collect()
}

/// The language a code pattern over this path is written in, and `None` for a path no pattern
/// reads.
pub fn language_of(path: &str) -> Option<&'static str> {
    let row = crate::syntax::language_of(path)?;
    adapter(row.id)?;
    row.names.first().copied()
}

/// The rows, from 0, where each pattern matches this file, one list per pattern in the order
/// given. A pattern written in another language matches nothing here.
pub fn rows(file: &ParsedFile, patterns: &[&Pattern]) -> Vec<Vec<usize>> {
    let Some(adapter) = adapter(file.language.id) else {
        return vec![Vec::new(); patterns.len()];
    };
    let root = AstGrep::doc(StrDoc {
        src: file.source.to_string(),
        lang: Grammar {
            language: file.language,
            adapter,
        },
        tree: file.tree.clone(),
    });
    patterns
        .iter()
        .map(|pattern| matched(&root, pattern, file.language))
        .collect()
}

/// The row of every node any reading of the pattern matches. A node is one match however many
/// readings hold it, and two nested nodes are two.
fn matched(root: &AstGrep<StrDoc<Grammar>>, pattern: &Pattern, language: &Language) -> Vec<usize> {
    let readings = pattern
        .compiled
        .iter()
        .filter(|(grammar, _)| grammar.language.name == language.name)
        .flat_map(|(_, readings)| readings);
    let mut nodes = BTreeMap::new();
    for (_, reading) in readings {
        for found in root.root().find_all(reading) {
            let range = found.range();
            nodes.insert((range.start, range.end), found.start_pos().line());
        }
    }
    nodes.into_values().collect()
}
