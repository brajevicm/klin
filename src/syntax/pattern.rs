//! Code patterns: a piece of code a person writes with `$NAME` and `$$$ARGS` holes, and the
//! places a parsed file holds it. The matching engine is `ast-grep-core`, and every type of it
//! stays in this module. A pattern language is a logical language of the one registry in
//! `syntax`: a pattern compiles once per grammar variant of that language, so `.ts` and `.tsx`
//! files are both TypeScript. Adding a language is one arm in `hole`. ADR 0006, ADR 0037.

use std::borrow::Cow;

use ast_grep_core::matcher::PatternBuilder;
use ast_grep_core::tree_sitter::{LanguageExt, StrDoc, TSLanguage};
use ast_grep_core::{AstGrep, PatternError};

use crate::reference;
use crate::syntax::{LANGUAGES, Language, LanguageId, ParsedFile, tree_of, walk};

/// The character a hole is written with once a pattern reaches the grammar. Rust reads no `$` in
/// a name, so a hole there becomes `µ` before it is parsed, and a block such as
/// `if $COND { $$$BODY }` still reads as a block. `None` for a language no pattern is written in.
fn hole(id: LanguageId) -> Option<char> {
    match id {
        LanguageId::Rust => Some('µ'),
        LanguageId::TypeScript => Some('$'),
        _ => None,
    }
}

/// One grammar variant a pattern compiles against.
#[derive(Clone)]
struct Grammar {
    language: &'static Language,
    hole: char,
}

impl ast_grep_core::Language for Grammar {
    fn pre_process_pattern<'q>(&self, query: &'q str) -> Cow<'q, str> {
        if self.hole == '$' {
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
        self.hole
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
            true => self.hole,
            false => '$',
        }
    }

    /// A pattern's own text read by the grammar. A text with a node the grammar could not read
    /// is refused here, because a pattern built around one matches nothing and passes in
    /// silence. A missing token, such as the `;` a Rust expression leaves off, is not refused.
    fn document(&self, source: &str) -> Result<StrDoc<Grammar>, String> {
        let tree = tree_of(source, self.language)
            .map_err(|why| why.to_string())?
            .ok_or_else(|| format!("the {} grammar could not read it", self.language.name))?;
        let mut unread = None;
        walk(tree.root_node(), &mut |node| {
            if node.is_error() && unread.is_none() {
                unread = Some(node.start_position().column + 1);
            }
        });
        if let Some(column) = unread {
            return Err(format!(
                "the {} grammar could not read it from column {column}",
                self.language.name
            ));
        }
        Ok(StrDoc {
            src: source.to_string(),
            lang: self.clone(),
            tree,
        })
    }
}

/// One code pattern, compiled against every grammar variant of its language. A variant may refuse
/// it: a JSX pattern is TSX and not plain TypeScript, and no `.ts` file can hold it, so it matches
/// nothing there.
pub struct Pattern {
    compiled: Vec<(Grammar, Result<ast_grep_core::Pattern, String>)>,
}

impl Pattern {
    /// The pattern in the named language, or what makes it one no file could match: no grammar
    /// variant of the language reads it.
    pub fn compile(text: &str, language: &str) -> Result<Pattern, String> {
        let variants: Vec<Grammar> = LANGUAGES
            .iter()
            .filter(|row| row.names.first() == Some(&language))
            .filter_map(|row| {
                Some(Grammar {
                    language: row,
                    hole: hole(row.id)?,
                })
            })
            .collect();
        if variants.is_empty() {
            return Err(format!("no code pattern is written in {language}"));
        }
        let compiled: Vec<(Grammar, Result<ast_grep_core::Pattern, String>)> = variants
            .into_iter()
            .map(|grammar| {
                let pattern =
                    ast_grep_core::Pattern::try_new(text, grammar.clone()).map_err(refusal);
                (grammar, pattern)
            })
            .collect();
        if compiled.iter().all(|(_, pattern)| pattern.is_err())
            && let Some((_, Err(why))) = compiled.first()
        {
            return Err(why.clone());
        }
        Ok(Pattern { compiled })
    }
}

/// Why a grammar variant refused a pattern, in a person's words.
fn refusal(problem: PatternError) -> String {
    match problem {
        PatternError::Parse(why) => why,
        PatternError::NoContent(_) => "it holds no code".to_string(),
        PatternError::MultipleNode(_) => {
            "it is more than one piece of code — write one expression, statement or item"
                .to_string()
        }
        PatternError::RootMultiMetaVar(_) => {
            "a `$$$` hole matches a list, so it needs code around it".to_string()
        }
        _ => "it is not a pattern klin can read".to_string(),
    }
}

/// The names a code pattern may be written in, each with the extensions it reads.
pub fn language_extensions() -> Vec<(&'static str, String)> {
    reference::extensions_by_name(
        LANGUAGES
            .iter()
            .filter(|row| hole(row.id).is_some())
            .map(|row| (&row.names[..1], row.extensions)),
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
    hole(row.id)?;
    row.names.first().copied()
}

/// The rows, from 0, where each pattern matches this file, one list per pattern in the order
/// given. A pattern written in another language matches nothing here.
pub fn rows(file: &ParsedFile, patterns: &[&Pattern]) -> Vec<Vec<usize>> {
    let Some(hole) = hole(file.language.id) else {
        return vec![Vec::new(); patterns.len()];
    };
    let root = AstGrep::doc(StrDoc {
        src: file.source.to_string(),
        lang: Grammar {
            language: file.language,
            hole,
        },
        tree: file.tree.clone(),
    });
    patterns
        .iter()
        .map(|pattern| {
            pattern
                .compiled
                .iter()
                .find(|(grammar, _)| grammar.language.name == file.language.name)
                .and_then(|(_, compiled)| compiled.as_ref().ok())
                .map(|compiled| {
                    root.root()
                        .find_all(compiled)
                        .map(|found| found.start_pos().line())
                        .collect()
                })
                .unwrap_or_default()
        })
        .collect()
}
