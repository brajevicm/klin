//! What the facts one run holds cost in population, sparsity and owned bytes, and what one of
//! each structural value costs before its strings and lists. The counters are deterministic, so
//! a representation decision reads them in place of process memory. Spec 11.2.

use std::collections::HashSet;
use std::rc::Rc;

use super::{Declaration, Export, FileFacts, Import, ModuleDecl, Reference};
use super::{ExportLeaf, QualifiedPath};

#[derive(Default, Clone, Copy)]
pub struct Footprint {
    pub files: usize,
    pub declarations: usize,
    pub references: usize,
    pub imports: usize,
    pub module_declarations: usize,
    pub exports: usize,
    pub export_leaves: usize,
    pub qualified_paths: usize,
    pub path_bytes: usize,
    pub declaration_name_bytes: usize,
    pub declaration_text_bytes: usize,
    pub reference_name_bytes: usize,
    pub signatures: usize,
    pub signature_bytes: usize,
    pub owners: usize,
    pub owner_bytes: usize,
    pub exported_aliases: usize,
    pub exported_alias_bytes: usize,
    pub nestings: usize,
    pub nesting_entries: usize,
    pub nesting_bytes: usize,
    pub import_text_bytes: usize,
    pub export_text_bytes: usize,
    pub module_text_bytes: usize,
}

impl Footprint {
    /// Every counter under its name, in the order the report writes them.
    pub fn rows(&self) -> [(&'static str, usize); 24] {
        [
            ("files", self.files),
            ("declarations", self.declarations),
            ("references", self.references),
            ("imports", self.imports),
            ("module_declarations", self.module_declarations),
            ("exports", self.exports),
            ("export_leaves", self.export_leaves),
            ("qualified_paths", self.qualified_paths),
            ("path_bytes", self.path_bytes),
            ("declaration_name_bytes", self.declaration_name_bytes),
            ("declaration_text_bytes", self.declaration_text_bytes),
            ("reference_name_bytes", self.reference_name_bytes),
            ("signatures", self.signatures),
            ("signature_bytes", self.signature_bytes),
            ("owners", self.owners),
            ("owner_bytes", self.owner_bytes),
            ("exported_aliases", self.exported_aliases),
            ("exported_alias_bytes", self.exported_alias_bytes),
            ("nestings", self.nestings),
            ("nesting_entries", self.nesting_entries),
            ("nesting_bytes", self.nesting_bytes),
            ("import_text_bytes", self.import_text_bytes),
            ("export_text_bytes", self.export_text_bytes),
            ("module_text_bytes", self.module_text_bytes),
        ]
    }
}

/// The facts of both trees, each file counted once however many trees hold the same extraction.
pub fn of(trees: [&[Rc<FileFacts>]; 2]) -> Footprint {
    let mut held = HashSet::new();
    let mut out = Footprint::default();
    for facts in trees.into_iter().flatten() {
        if held.insert(Rc::as_ptr(facts)) {
            counted(facts, &mut out);
            weighed(facts, &mut out);
        }
    }
    out
}

/// What one of each structural value costs, without the bytes its strings and lists own.
pub fn sizes() -> [(&'static str, usize); 7] {
    [
        ("file_facts", size_of::<FileFacts>()),
        ("declaration", size_of::<Declaration>()),
        ("reference", size_of::<Reference>()),
        ("import", size_of::<Import>()),
        ("module_declaration", size_of::<ModuleDecl>()),
        ("export", size_of::<Export>()),
        ("export_leaf", size_of::<ExportLeaf>()),
    ]
}

fn counted(facts: &FileFacts, out: &mut Footprint) {
    out.files += 1;
    out.declarations += facts.declarations.len();
    out.references += facts.references.len();
    out.imports += facts.imports.len();
    out.module_declarations += facts.module_declarations.len();
    out.exports += facts.exports.len();
    out.export_leaves += facts
        .exports
        .iter()
        .map(|held| held.leaves.len())
        .sum::<usize>();
    out.qualified_paths += facts.paths.len();
}

fn weighed(facts: &FileFacts, out: &mut Footprint) {
    out.path_bytes += facts.file.len();
    for declaration in &facts.declarations {
        declared(declaration, out);
    }
    for reference in &facts.references {
        out.reference_name_bytes += reference.name.len();
    }
    for import in &facts.imports {
        imported(import, out);
    }
    for module in &facts.module_declarations {
        out.module_text_bytes += module.text.len();
        nesting(&module.nesting, out);
    }
    for export in &facts.exports {
        exported(export, out);
    }
    for path in &facts.paths {
        qualified(path, out);
    }
}

fn declared(declaration: &Declaration, out: &mut Footprint) {
    out.declaration_name_bytes += declaration.name.len();
    out.declaration_text_bytes += declaration.text.len();
    sparse(
        declaration.signature.as_deref(),
        &mut out.signatures,
        &mut out.signature_bytes,
    );
    sparse(
        declaration.owner.as_deref(),
        &mut out.owners,
        &mut out.owner_bytes,
    );
    sparse(
        declaration.exported_as.as_deref(),
        &mut out.exported_aliases,
        &mut out.exported_alias_bytes,
    );
    nesting(&declaration.nesting, out);
}

fn imported(import: &Import, out: &mut Footprint) {
    out.import_text_bytes += import.text.len()
        + import.module.as_deref().map_or(0, str::len)
        + texts(&import.names)
        + texts(&import.paths);
    nesting(&import.nesting, out);
}

/// One export statement's own text, the module it names, and its leaves' paths and names.
fn exported(export: &Export, out: &mut Footprint) {
    out.export_text_bytes += export.text.len() + export.source.as_deref().map_or(0, str::len);
    for leaf in &export.leaves {
        out.export_text_bytes += leaf.path.len() + leaf.name.as_deref().map_or(0, str::len);
    }
    nesting(&export.nesting, out);
}

fn qualified(path: &QualifiedPath, out: &mut Footprint) {
    out.module_text_bytes += path.path.len();
    nesting(&path.nesting, out);
}

/// One optional field: how many values carry it, and the bytes they hold.
fn sparse(held: Option<&str>, count: &mut usize, bytes: &mut usize) {
    if let Some(text) = held {
        *count += 1;
        *bytes += text.len();
    }
}

/// One list of inline module names, counted only where the value carries any.
fn nesting(nesting: &[String], out: &mut Footprint) {
    if nesting.is_empty() {
        return;
    }
    out.nestings += 1;
    out.nesting_entries += nesting.len();
    out.nesting_bytes += texts(nesting);
}

fn texts(texts: &[String]) -> usize {
    texts.iter().map(String::len).sum()
}
