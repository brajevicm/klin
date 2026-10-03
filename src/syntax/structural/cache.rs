//! The structural outcomes of one base commit's files, kept in the state directory between runs,
//! so a changed run that is not strict reads the base's facts instead of parsing the base again.
//! The cache is never a fact of its own. A file that is gone, truncated, corrupt, or written for
//! another commit, checkout, schema or build reads as nothing, and the run extracts the base as
//! if no cache were there. Spec 8.4.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::SystemTime;

use super::facts::{
    Declaration, DeclarationKind, Export, ExportLeaf, ExternCrate, FileFacts, Import, ModuleDecl,
    Names, Outcome, QualifiedPath, Reference, Visibility,
};
use crate::syntax::{LANGUAGES, Language, LanguageId, Unparsed};
use crate::write::{AtomicWrite, atomic_write};

/// Raise this when what a file's facts mean changes in a way the sources below do not show.
const EPOCH: u64 = 6;

const MAGIC: &[u8] = b"klin structural cache\n";
const KEPT: usize = 4;
const BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const PRIME: u64 = 0x100_0000_01b3;

/// The code whose change can change the facts of one file, or how they are written here. A
/// build whose copy of these differs reads another build's cache as nothing.
const SOURCES: &[&[u8]] = &[
    include_bytes!("mod.rs"),
    include_bytes!("facts.rs"),
    include_bytes!("rust.rs"),
    include_bytes!("typescript.rs"),
    include_bytes!("cache.rs"),
    include_bytes!("../mod.rs"),
    include_bytes!("../convention.rs"),
    include_bytes!("../../../Cargo.lock"),
];

/// Where one base commit's structural outcomes are kept, and the identity a kept file must
/// carry to be read.
pub struct Cache {
    file: PathBuf,
    identity: Vec<u8>,
}

impl Cache {
    /// The cache of this commit in the directory `under`, for a base checked out as `checkout`
    /// describes it, and `None` when the name is not a full object id, so a branch name never
    /// keys one.
    pub fn at(under: &Path, commit: &str, checkout: &[u8]) -> Option<Cache> {
        let object = matches!(commit.len(), 40 | 64)
            && commit
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
        object.then(|| Cache {
            file: under.join(commit),
            identity: identity(EPOCH, env!("CARGO_PKG_VERSION"), commit, checkout),
        })
    }

    pub fn read(&self) -> Option<HashMap<String, Outcome>> {
        decoded(&std::fs::read(&self.file).ok()?, &self.identity)
    }

    /// The outcomes written whole over whatever the file held. A cache klin cannot write costs
    /// the next run the extraction and nothing else.
    pub fn write(&self, outcomes: &[(&str, &Outcome)]) {
        if let Some(under) = self.file.parent() {
            let _ = std::fs::create_dir_all(under);
        }
        let written = atomic_write(AtomicWrite {
            target: &self.file,
            bytes: &encoded(&self.identity, outcomes),
            keep_mode_from: None,
        });
        if written.is_ok() {
            self.evict();
        }
    }

    /// Keep the four newest base-commit cache files; #193 measures their storage.
    fn evict(&self) {
        let Some(Ok(entries)) = self.file.parent().map(std::fs::read_dir) else {
            return;
        };
        let mut older: Vec<(SystemTime, PathBuf)> = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| *path != self.file)
            .filter_map(|path| Some((path.metadata().ok()?.modified().ok()?, path)))
            .collect();
        older.sort_by(|(left_time, left_path), (right_time, right_path)| {
            right_time
                .cmp(left_time)
                .then_with(|| left_path.cmp(right_path))
        });
        for (_, path) in older.into_iter().skip(KEPT - 1) {
            let _ = std::fs::remove_file(path);
        }
    }
}

fn identity(epoch: u64, version: &str, commit: &str, checkout: &[u8]) -> Vec<u8> {
    let mut identity = Writer(MAGIC.to_vec());
    identity.number(epoch);
    identity.text(version);
    identity.number(
        SOURCES
            .iter()
            .fold(BASIS, |sum, source| checksum(sum, source)),
    );
    identity.text(commit);
    identity.number(checksum(BASIS, checkout));
    identity.0
}

fn encoded(identity: &[u8], outcomes: &[(&str, &Outcome)]) -> Vec<u8> {
    let mut body = Writer(Vec::new());
    body.number(outcomes.len() as u64);
    for (file, outcome) in outcomes {
        body.text(file);
        body.outcome(outcome);
    }
    let mut out = identity.to_vec();
    out.extend_from_slice(&checksum(BASIS, &body.0).to_le_bytes());
    out.extend_from_slice(&body.0);
    out
}

fn decoded(bytes: &[u8], identity: &[u8]) -> Option<HashMap<String, Outcome>> {
    let (sum, body) = bytes.strip_prefix(identity)?.split_first_chunk::<8>()?;
    if u64::from_le_bytes(*sum) != checksum(BASIS, body) {
        return None;
    }
    let mut names = Names::default();
    let mut reader = Reader(body, &mut names);
    let outcomes = reader.list(Reader::entry)?;
    reader.0.is_empty().then_some(outcomes)
}

/// A word-at-a-time FNV-1a sum. It finds a torn or interleaved write, and it is no defence
/// against a writer who means harm, which the guard keeps out of the state directory.
fn checksum(sum: u64, bytes: &[u8]) -> u64 {
    let (words, rest) = bytes.as_chunks::<8>();
    let sum = words.iter().fold(sum, |sum, word| {
        (sum ^ u64::from_le_bytes(*word)).wrapping_mul(PRIME)
    });
    rest.iter().fold(sum, |sum, byte| {
        (sum ^ u64::from(*byte)).wrapping_mul(PRIME)
    }) ^ bytes.len() as u64
}

const FOREIGN: u64 = 0;
const UNSUPPORTED: u64 = 1;
const UNPARSED: u64 = 2;
const FACTS: u64 = 3;

struct Writer(Vec<u8>);

impl Writer {
    fn number(&mut self, mut value: u64) {
        while value >= 0x80 {
            self.0.push((value as u8) | 0x80);
            value >>= 7;
        }
        self.0.push(value as u8);
    }

    fn text(&mut self, text: &str) {
        self.number(text.len() as u64);
        self.0.extend_from_slice(text.as_bytes());
    }

    fn texts(&mut self, texts: &[String]) {
        self.number(texts.len() as u64);
        for text in texts {
            self.text(text);
        }
    }

    fn optional(&mut self, text: Option<&str>) {
        match text {
            Some(text) => {
                self.number(1);
                self.text(text);
            }
            None => self.number(0),
        }
    }

    fn outcome(&mut self, outcome: &Outcome) {
        match outcome {
            Outcome::Foreign => self.number(FOREIGN),
            Outcome::Unsupported(language) => {
                self.number(UNSUPPORTED);
                self.text(language);
            }
            Outcome::Unparsed(file) => {
                self.number(UNPARSED);
                self.text(file.language);
            }
            Outcome::Facts(facts) => {
                self.number(FACTS);
                self.facts(facts);
            }
        }
    }

    fn facts(&mut self, facts: &FileFacts) {
        let language = LANGUAGES.iter().find(|row| row.id == facts.language);
        self.text(language.map_or("", |row| row.name));
        self.number(facts.declarations.len() as u64);
        for declaration in &facts.declarations {
            self.declaration(declaration);
        }
        self.number(facts.imports.len() as u64);
        for import in &facts.imports {
            self.number(import.line);
            self.number(import.start_byte);
            self.text(&import.text);
            self.texts(&import.nesting);
            self.number(u64::from(import.in_function));
            self.optional(import.module.as_deref());
            self.texts(&import.names);
            self.texts(&import.paths);
        }
        self.number(facts.module_declarations.len() as u64);
        for module in &facts.module_declarations {
            self.number(module.line);
            self.text(&module.text);
            self.text(&module.name);
            self.texts(&module.nesting);
            self.number(u64::from(module.inline));
            self.number(u64::from(module.in_block));
            self.optional(module.path.as_deref());
            self.number(visibility_number(module.visibility));
        }
        self.number(facts.references.len() as u64);
        for reference in &facts.references {
            self.text(reference.name.as_str());
            self.number(reference.line);
        }
        self.number(facts.paths.len() as u64);
        for path in &facts.paths {
            self.number(path.line);
            self.texts(&path.nesting);
            self.text(&path.path);
        }
        self.number(facts.exports.len() as u64);
        for export in &facts.exports {
            self.export(export);
        }
        self.number(facts.crates.len() as u64);
        for held in &facts.crates {
            self.texts(&held.nesting);
            self.text(&held.name);
            self.text(&held.alias);
        }
    }

    fn declaration(&mut self, declaration: &Declaration) {
        self.text(&declaration.name);
        self.number(kind_number(declaration.kind));
        self.number(declaration.line);
        self.number(declaration.end);
        self.text(&declaration.text);
        self.number(
            u64::from(declaration.externally_visible)
                | u64::from(declaration.entry_point) << 1
                | u64::from(declaration.associated) << 2,
        );
        self.texts(&declaration.bindings);
        self.texts(&declaration.nesting);
        self.number(visibility_number(declaration.visibility));
        self.optional(declaration.exported_as.as_deref());
        self.optional(declaration.owner.as_deref());
        self.optional(declaration.signature.as_deref());
    }

    fn export(&mut self, export: &Export) {
        self.number(export.line);
        self.number(export.start_byte);
        self.text(&export.text);
        self.texts(&export.nesting);
        self.optional(export.source.as_deref());
        self.number(u64::from(export.type_only) | u64::from(export.supported) << 1);
        self.number(export.leaves.len() as u64);
        for leaf in &export.leaves {
            self.text(&leaf.path);
            self.optional(leaf.name.as_deref());
        }
        self.optional(export.contract.as_deref());
    }
}

struct Reader<'a, 'b>(&'a [u8], &'b mut Names);

impl Reader<'_, '_> {
    fn number(&mut self) -> Option<u64> {
        let mut value = 0u64;
        for shift in (0..64).step_by(7) {
            let (byte, rest) = self.0.split_first()?;
            self.0 = rest;
            value |= u64::from(byte & 0x7f).checked_shl(shift)?;
            if byte & 0x80 == 0 {
                return Some(value);
            }
        }
        None
    }

    /// A count, never more than the bytes left could hold, so a corrupt one allocates nothing.
    fn count(&mut self) -> Option<usize> {
        usize::try_from(self.number()?)
            .ok()
            .filter(|count| *count <= self.0.len())
    }

    /// A counted run of items, and `None` when any one of them does not read.
    fn list<T, C: FromIterator<T>>(&mut self, each: fn(&mut Self) -> Option<T>) -> Option<C> {
        let count = self.count()?;
        (0..count).map(|_| each(self)).collect()
    }

    fn text(&mut self) -> Option<String> {
        let length = self.count()?;
        let (text, rest) = self.0.split_at_checked(length)?;
        self.0 = rest;
        String::from_utf8(text.to_vec()).ok()
    }

    fn optional(&mut self) -> Option<Option<String>> {
        match self.number()? {
            0 => Some(None),
            1 => self.text().map(Some),
            _ => None,
        }
    }

    fn language(&mut self) -> Option<&'static Language> {
        let name = self.text()?;
        LANGUAGES.iter().find(|row| row.name == name)
    }

    fn entry(&mut self) -> Option<(String, Outcome)> {
        let file = self.text()?;
        let outcome = self.outcome(&file)?;
        Some((file, outcome))
    }

    fn outcome(&mut self, file: &str) -> Option<Outcome> {
        match self.number()? {
            FOREIGN => Some(Outcome::Foreign),
            UNSUPPORTED => self.language().map(|row| Outcome::Unsupported(row.name)),
            UNPARSED => self.language().map(|row| {
                Outcome::Unparsed(Unparsed {
                    file: file.to_string(),
                    language: row.name,
                })
            }),
            FACTS => self
                .language()
                .and_then(|row| self.facts(file, row.id))
                .map(|facts| Outcome::Facts(Rc::new(facts))),
            _ => None,
        }
    }

    fn facts(&mut self, file: &str, language: LanguageId) -> Option<FileFacts> {
        Some(FileFacts {
            file: file.to_string(),
            language,
            declarations: self.list(Reader::declaration)?,
            imports: self.list(Reader::import)?,
            module_declarations: self.list(Reader::module)?,
            references: self.list(Reader::reference)?,
            paths: self.list(Reader::qualified)?,
            exports: self.list(Reader::export)?,
            crates: self.list(Reader::extern_crate)?,
        })
    }

    fn declaration(&mut self) -> Option<Declaration> {
        let (name, kind, line, end, text) = self.site()?;
        let flags = self.number().filter(|flags| *flags <= 7)?;
        let bindings = self.list(Reader::text)?;
        self.contract(Declaration {
            name,
            bindings,
            kind,
            line,
            end,
            text,
            externally_visible: flags & 1 == 1,
            entry_point: flags & 2 == 2,
            nesting: Vec::new(),
            associated: flags & 4 == 4,
            visibility: Visibility::Private,
            exported_as: None,
            owner: None,
            signature: None,
        })
    }

    /// The exposure and contract fields a declaration ends with, read into it.
    fn contract(&mut self, mut declaration: Declaration) -> Option<Declaration> {
        declaration.nesting = self.list(Reader::text)?;
        declaration.visibility = visibility_of(self.number()?)?;
        declaration.exported_as = self.optional()?;
        declaration.owner = self.optional()?;
        declaration.signature = self.optional()?;
        Some(declaration)
    }

    /// The name, kind, lines and text a declaration starts with.
    fn site(&mut self) -> Option<(String, DeclarationKind, u64, u64, String)> {
        let name = self.text()?;
        let kind = kind_of(self.number()?)?;
        let (line, end) = (self.number()?, self.number()?);
        Some((name, kind, line, end, self.text()?))
    }

    fn export(&mut self) -> Option<Export> {
        let (line, start_byte, text, nesting) = self.statement()?;
        let source = self.optional()?;
        let flags = self.number().filter(|flags| *flags <= 3)?;
        Some(Export {
            line,
            start_byte,
            text,
            nesting,
            source,
            type_only: flags & 1 == 1,
            supported: flags & 2 == 2,
            leaves: self.list(Reader::leaf)?,
            contract: self.optional()?,
        })
    }

    fn leaf(&mut self) -> Option<ExportLeaf> {
        Some(ExportLeaf {
            path: self.text()?,
            name: self.optional()?,
        })
    }

    fn extern_crate(&mut self) -> Option<ExternCrate> {
        Some(ExternCrate {
            nesting: self.list(Reader::text)?,
            name: self.text()?,
            alias: self.text()?,
        })
    }

    /// The line, byte offset, text and nesting an import or export starts with.
    fn statement(&mut self) -> Option<(u64, u64, String, Vec<String>)> {
        Some((
            self.number()?,
            self.number()?,
            self.text()?,
            self.list(Reader::text)?,
        ))
    }

    fn import(&mut self) -> Option<Import> {
        let (line, start_byte, text, nesting) = self.statement()?;
        Some(Import {
            line,
            start_byte,
            text,
            nesting,
            in_function: self.number().filter(|flag| *flag <= 1)? == 1,
            module: self.optional()?,
            names: self.list(Reader::text)?,
            paths: self.list(Reader::text)?,
        })
    }

    fn module(&mut self) -> Option<ModuleDecl> {
        let (line, text, name) = (self.number()?, self.text()?, self.text()?);
        let nesting = self.list(Reader::text)?;
        let inline = self.number().filter(|flag| *flag <= 1)? == 1;
        self.module_tail(line, text, name, nesting, inline)
    }

    fn module_tail(
        &mut self,
        line: u64,
        text: String,
        name: String,
        nesting: Vec<String>,
        inline: bool,
    ) -> Option<ModuleDecl> {
        Some(ModuleDecl {
            line,
            text,
            name,
            nesting,
            inline,
            in_block: self.number().filter(|flag| *flag <= 1)? == 1,
            path: self.optional()?,
            visibility: visibility_of(self.number()?)?,
        })
    }

    fn qualified(&mut self) -> Option<QualifiedPath> {
        Some(QualifiedPath {
            line: self.number()?,
            nesting: self.list(Reader::text)?,
            path: self.text()?,
        })
    }

    fn reference(&mut self) -> Option<Reference> {
        let length = self.count()?;
        let (text, rest) = self.0.split_at_checked(length)?;
        self.0 = rest;
        let text = std::str::from_utf8(text).ok()?;
        let name = self.1.intern(text);
        Some(Reference {
            name,
            line: self.number()?,
        })
    }
}

fn kind_number(kind: DeclarationKind) -> u64 {
    match kind {
        DeclarationKind::Function => 0,
        DeclarationKind::Method => 1,
        DeclarationKind::Type => 2,
        DeclarationKind::Constant => 3,
        DeclarationKind::Variable => 4,
    }
}

const KINDS: [DeclarationKind; 5] = [
    DeclarationKind::Function,
    DeclarationKind::Method,
    DeclarationKind::Type,
    DeclarationKind::Constant,
    DeclarationKind::Variable,
];

fn kind_of(number: u64) -> Option<DeclarationKind> {
    KINDS.into_iter().find(|kind| kind_number(*kind) == number)
}

const VISIBILITIES: [Visibility; 3] = [
    Visibility::Private,
    Visibility::Restricted,
    Visibility::Public,
];

fn visibility_number(visibility: Visibility) -> u64 {
    match visibility {
        Visibility::Private => 0,
        Visibility::Restricted => 1,
        Visibility::Public => 2,
    }
}

fn visibility_of(number: u64) -> Option<Visibility> {
    VISIBILITIES
        .into_iter()
        .find(|held| visibility_number(*held) == number)
}

#[cfg(test)]
mod tests {
    use super::*;

    const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

    fn outcomes() -> Vec<(String, Outcome)> {
        let rust = "pub use crate::pay::{Refund, refund};\n#[path = \"other.rs\"]\nmod moved;\npub struct Charge;\nfn main() { refund(); }\n#[test]\nfn works() {}\nmod tests {\n    use super::*;\n    fn it() { crate::pay::charge(); }\n}\nextern crate serde as json;\nimpl Charge {\n    const N: u8 = 1;\n}\nfn local() {\n    use std::fmt::Write;\n}\n";
        let typescript = "import { refund } from \"./pay\";\nexport const view = () => <p>{refund()}</p>;\nexport default view;\nexport namespace N {\n    export type T = string;\n}\n";
        [
            ("src/pay.rs", rust),
            ("web/view.tsx", typescript),
            ("src/broken.rs", "fn broken( {\n"),
            ("src/job.py", "def job():\n    return 1\n"),
            ("README.md", "# klin\n"),
        ]
        .into_iter()
        .map(|(file, source)| match super::super::of(file, source) {
            Ok(outcome) => (file.to_string(), outcome),
            Err(why) => panic!("{file}: {}", why.0),
        })
        .collect()
    }

    fn ours() -> Vec<u8> {
        identity(EPOCH, env!("CARGO_PKG_VERSION"), COMMIT, b"/tree")
    }

    fn bytes_of(identity: &[u8], held: &[(String, Outcome)]) -> Vec<u8> {
        let mut sorted: Vec<(&str, &Outcome)> = held
            .iter()
            .map(|(file, outcome)| (file.as_str(), outcome))
            .collect();
        sorted.sort_by_key(|(file, _)| *file);
        encoded(identity, &sorted)
    }

    /// The whole cache decoded, and the facts of one file in it.
    fn read_back() -> (Vec<u8>, Vec<(String, Outcome)>) {
        let identity = ours();
        let written = bytes_of(&identity, &outcomes());
        match decoded(&written, &identity) {
            Some(read) => (written, read.into_iter().collect()),
            None => panic!("a whole cache read as nothing"),
        }
    }

    fn facts_of(read: &[(String, Outcome)], file: &str) -> Rc<FileFacts> {
        let found = read.iter().find_map(|(held, outcome)| match outcome {
            Outcome::Facts(facts) if held == file => Some(facts.clone()),
            _ => None,
        });
        match found {
            Some(facts) => facts,
            None => panic!("{file} read back with no facts"),
        }
    }

    #[test]
    fn what_a_rust_file_binds_reads_back_as_written() {
        let (_, read) = read_back();
        let pay = facts_of(&read, "src/pay.rs");
        let crates: Vec<(&str, &str)> = pay
            .crates
            .iter()
            .map(|held| (held.name.as_str(), held.alias.as_str()))
            .collect();
        assert_eq!(crates, vec![("serde", "json")]);
        let associated: Vec<&str> = pay
            .declarations
            .iter()
            .filter(|held| held.associated)
            .map(|held| held.name.as_str())
            .collect();
        assert_eq!(associated, vec!["N"]);
        let in_function: Vec<&str> = pay
            .imports
            .iter()
            .filter(|held| held.in_function)
            .map(|held| held.text.as_str())
            .collect();
        assert_eq!(in_function, vec!["use std::fmt::Write;"]);
    }

    #[test]
    fn every_outcome_reads_back_as_it_was_written() {
        let (written, read) = read_back();
        assert_eq!(read.len(), 5);
        assert_eq!(bytes_of(&ours(), &read), written);
        let pay = facts_of(&read, "src/pay.rs");
        let modules: Vec<(&str, bool, Option<&str>)> = pay
            .module_declarations
            .iter()
            .map(|held| (held.name.as_str(), held.inline, held.path.as_deref()))
            .collect();
        assert_eq!(
            modules,
            vec![("moved", false, Some("other.rs")), ("tests", true, None)]
        );
        let imports: Vec<(Vec<String>, Vec<String>)> = pay
            .imports
            .iter()
            .map(|held| (held.nesting.clone(), held.paths.clone()))
            .collect();
        assert_eq!(
            imports,
            vec![
                (
                    Vec::new(),
                    vec![
                        "crate::pay::Refund".to_string(),
                        "crate::pay::refund".to_string()
                    ]
                ),
                (vec!["tests".to_string()], vec!["super::*".to_string()]),
                (Vec::new(), vec!["std::fmt::Write".to_string()]),
            ]
        );
        let paths: Vec<(u64, &[String], &str)> = pay
            .paths
            .iter()
            .map(|held| (held.line, held.nesting.as_slice(), held.path.as_str()))
            .collect();
        assert_eq!(
            paths,
            vec![(10, &["tests".to_string()][..], "crate::pay::charge")]
        );
    }

    #[test]
    fn visibility_signature_nesting_and_exports_read_back_as_written() {
        let (_, read) = read_back();
        let pay = facts_of(&read, "src/pay.rs");
        let charge = pay
            .declarations
            .iter()
            .find(|held| held.name == "Charge")
            .unwrap_or_else(|| panic!("Charge read back with no declaration"));
        assert_eq!(charge.visibility, Visibility::Public);
        assert_eq!(charge.signature.as_deref(), Some("struct Charge;"));
        assert_eq!(charge.nesting, Vec::<String>::new());
        let exports: Vec<(&str, Option<&str>)> = pay
            .exports
            .iter()
            .flat_map(|held| &held.leaves)
            .map(|leaf| (leaf.path.as_str(), leaf.name.as_deref()))
            .collect();
        assert_eq!(
            exports,
            vec![
                ("crate::pay::Refund", Some("Refund")),
                ("crate::pay::refund", Some("refund"))
            ]
        );
        assert_eq!(pay.module_declarations[0].visibility, Visibility::Private);
        let view = facts_of(&read, "web/view.tsx");
        assert_eq!(view.declarations[0].exported_as, None);
        assert_eq!(
            view.declarations[0].signature.as_deref(),
            Some("const view: ?")
        );
        assert_eq!(view.exports.len(), 2);
        assert!(view.exports[0].supported && view.exports[0].source.is_none());
        assert_eq!(view.exports[0].leaves[0].name.as_deref(), Some("default"));
        assert_eq!(
            view.exports[1].contract.as_deref(),
            Some("namespace N { type T = string }")
        );
    }

    #[test]
    fn a_cut_or_a_changed_byte_reads_as_nothing() {
        let identity = ours();
        let written = bytes_of(&identity, &outcomes());
        for cut in 0..written.len() {
            assert!(
                decoded(&written[..cut], &identity).is_none(),
                "cut at {cut}"
            );
        }
        for at in 0..written.len() {
            let mut changed = written.clone();
            changed[at] ^= 0x10;
            assert!(decoded(&changed, &identity).is_none(), "byte {at}");
        }
        let longer = [written.as_slice(), &[0]].concat();
        assert!(decoded(&longer, &identity).is_none());
    }

    #[test]
    fn a_body_whose_sum_matches_but_whose_shape_does_not_reads_as_nothing() {
        let identity = ours();
        let mut body = Writer(Vec::new());
        body.number(1);
        body.text("src/pay.rs");
        body.number(FACTS);
        body.number(u64::MAX);
        let mut forged = identity.clone();
        forged.extend_from_slice(&checksum(BASIS, &body.0).to_le_bytes());
        forged.extend_from_slice(&body.0);
        assert!(decoded(&forged, &identity).is_none());
    }

    #[test]
    fn another_epoch_version_commit_or_checkout_reads_as_nothing() {
        let written = bytes_of(&ours(), &outcomes());
        let version = env!("CARGO_PKG_VERSION");
        let checkout = b"/tree";
        let other = "1123456789abcdef0123456789abcdef01234567";
        for identity in [
            identity(EPOCH + 1, version, COMMIT, checkout),
            identity(EPOCH, "9.9.9", COMMIT, checkout),
            identity(EPOCH, version, other, checkout),
            identity(EPOCH, version, COMMIT, b"/tree\0core.autocrlf=true"),
        ] {
            assert!(decoded(&written, &identity).is_none());
        }
    }

    #[test]
    fn a_name_that_is_not_a_full_object_id_keys_no_cache() {
        let state = Path::new("/state");
        let root = b"/tree";
        for name in [
            "main",
            "HEAD",
            "0123456",
            &COMMIT.to_uppercase(),
            "../escape",
        ] {
            assert!(Cache::at(state, name, root).is_none(), "{name}");
        }
        assert!(Cache::at(state, COMMIT, root).is_some());
        assert!(Cache::at(state, &"a".repeat(64), root).is_some());
    }
}
