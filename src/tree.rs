use std::cell::{Cell, OnceCell};
use std::path::{Path, PathBuf};

use crate::error::Error;
use crate::syntax::structural::Extracted;
use crate::{files, scope};

/// One tree's files, read once. The list is every file under the root by its relative path,
/// sorted, less the default skip set, everything git ignores, and symbolic links. Hidden
/// directories are in it, because two checks read them, and a caller that skips them filters
/// the list. The list reads no file's contents. A structural check's files are read, parsed and
/// extracted when a check first asks for them, and held with the tree for the run. Spec 4.1,
/// 4.3, ADR 0038.
pub struct Tree {
    root: PathBuf,
    files: OnceCell<Result<Vec<String>, String>>,
    listing: Cell<files::Listing>,
    extracted: Extracted,
    test_roots: OnceCell<scope::Roots>,
}

impl Tree {
    /// A tree at this root, with nothing read yet.
    pub fn at(root: &Path) -> Tree {
        Tree {
            root: root.to_path_buf(),
            files: OnceCell::new(),
            listing: Cell::new(files::Listing::default()),
            extracted: Extracted::default(),
            test_roots: OnceCell::new(),
        }
    }

    /// A tree at this root whose file list a caller already knows, because it read the list
    /// from somewhere other than a walk: the base laid out without a checkout reads it from the
    /// base commit's index. The list carries the same rules a walk gives it. Spec 4.3, 8.4.
    pub fn listed(root: &Path, files: Vec<String>) -> Tree {
        let tree = Tree::at(root);
        let _ = tree.files.set(Ok(files));
        tree
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Each file's structural outcome, extracted once for every check that selects the file.
    pub fn extracted(&self) -> &Extracted {
        &self.extracted
    }

    /// The test roots of this tree, derived by the survey on the first call and held for the
    /// run, because every check that skips tests asks for them. `survey::tests` is the one
    /// caller, so the first derivation is the only one. Spec 5.4.
    pub fn test_roots(&self, derive: impl FnOnce(&Tree) -> scope::Roots) -> &scope::Roots {
        self.test_roots.get_or_init(|| derive(self))
    }

    /// Every file, read on the first call and held for the run. A directory the walk could not
    /// read is an error naming it, as it was for every walk before. Spec 4.3, 14.
    pub fn files(&self) -> Result<&[String], Error> {
        self.files
            .get_or_init(|| {
                files::listing(&self.root)
                    .map(|(files, cost)| {
                        self.listing.set(cost);
                        files
                    })
                    .map_err(|why| why.to_string())
            })
            .as_deref()
            .map_err(|why| Error(why.clone()))
    }

    /// What reading the file list cost, handed over once: a second call, or a call before the
    /// list is read, is zero. Spec 11.2.
    pub fn listing_cost(&self) -> files::Listing {
        self.listing.take()
    }
}

/// Whether a walk of a tree reaches a file at this path: it descends no directory of the
/// default skip set. A walk keeps a file whatever the file itself is called, so only the
/// directories above it decide. This is `files::found`'s coverage for a file the tree does
/// not list yet, which is what the base laid out from an index has. Spec 4.3.
pub fn reached(path: &str) -> bool {
    path.rsplit_once('/')
        .is_none_or(|(parents, _)| !parents.split('/').any(files::skipped))
}
