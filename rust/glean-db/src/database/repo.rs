// Ported from: glean/db/Glean/Database/Repo.hs (27 lines).
// Manifest unit: rust/glean-db/src/database/repo.rs (migration/manifest.tsv
// line 52, crate glean-db). RULE §4: one source file -> one target file;
// the manifest path is authoritative.
//
// INVENTORY (RULE §5) — exact commands, run at the repo root:
//   $ grep -n 'Database/Repo\.hs' migration/inventory*.tsv
//   (no output; exit status 1)
// Zero rows across inventory.tsv, inventory.ffi.tsv, inventory.sites.tsv,
// inventory.codeflow.tsv, inventory.protocol.tsv and inventory.stm.tsv:
// no row names this source file. A broader grep does hit once:
//   $ grep -n -E 'Repo\.hs|databaseSubdir|databasePath|inRepo' \
//       migration/inventory*.tsv
//   migration/inventory.sites.tsv:615:exc-classify<TAB>
//     glean/db/Glean/Database/Open.hs<TAB>757<TAB>exception taxonomy<TAB>
//     logError $ inRepo dbRepo "couldn't open: " ++ show (exc :: SomeException)
// That row's SOURCE FILE column is Open.hs, not Repo.hs: it records the
// `inRepo dbRepo "couldn't open: " …` logError at Open.hs:757, i.e. this
// unit's function as CONSUMER EVIDENCE inside Open.hs's exception-taxonomy
// row. It does not govern Repo.hs, so no inventory decision is diverged
// from and there is no INVENTORY-BUG-CANDIDATE.
//
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, checked not assumed):
//   * Outbound edge: edges.tsv:352
//     `glean/db/Glean/Database/Repo.hs -> glean/hs/Glean/Repo/Text.hs`,
//     i.e. manifest row 51, which SHIPPED at 6ce4e79 as
//     rust/glean-core/src/repo/text.rs. The source's other two imports are
//     System.FilePath (:15, only `</>` is used) and Glean.Types (:18),
//     both non-manifest (base/filepath and generated thrift).
//   * Inbound edges (13), by edges.tsv line: 150 Backup.hs; 175 Catalog.hs;
//     180 Catalog/Local/Files.hs; 199 CompletePredicates.hs; 245 Create.hs;
//     262 Delete.hs; 287 Exception.hs; 299 Janitor.hs; 323 Open.hs; 421
//     Storage/LMDB.hs; 436 Storage/RocksDB.hs; 478 Write/Batch.hs; 1782
//     tools/gleancli/GleanCLI/Restore.hs.
//     Verified call sites: databaseSubdir Backup.hs:538; databasePath
//     Catalog/Local/Files.hs:38/:73, Storage/LMDB.hs:108/:150,
//     Storage/RocksDB.hs:147/:290; inRepo Backup.hs:273/:360/:385/:503,
//     Catalog.hs:280/:285, CompletePredicates.hs:193/:196, Create.hs:164/
//     :173, Delete.hs:56/:58/:85/:90/:103, Exception.hs:34, Janitor.hs:94,
//     Open.hs:699/:705/:707/:735/:757, Write/Batch.hs:184/:200/:265,
//     Restore.hs:115.
//
// ASSUMED-IMPORT (RULE §4 B4): the generated thrift crate.
//   * The source's `import Glean.Types (Repo(..))` (:18) supplies `Repo`,
//     which the generated Rust crate spells `glean_thrift::Repo`. This
//     file uses ONE spelling: `use glean_thrift as thrift;`, then
//     `thrift::Repo` everywhere (the row-51 convention).
//   * Pinned shape (glean/if/glean.thrift:106-109):
//         struct Repo { 1: string name; 2: string hash; }
//     Rust image (rust/glean-db/src/backend/types.rs:171-175):
//         #[derive(Clone, Debug, PartialEq, Eq, Hash)]
//         pub struct Repo { pub name: String, pub hash: String }
//     The Haskell type derives Eq, Show and Ord (gen-hs2
//     Glean/Types.hs:629-630). This unit only borrows the value and reads
//     no fields directly — `show_repo_sep` (row 51) reads `name`/`hash` —
//     so it imposes no additional derive demand (the Ord owed to
//     Retention.hs:150 is recorded in row 51's block).
//   * glean-db's Cargo.toml lists glean-thrift only as an anticipated
//     Step-4 edge, so this path is not resolvable at the draft compile
//     step; that is the expected state for every generated-type consumer
//     (RULE §4 B4: import from the generated crate, never fork a hand
//     translation of it).
//
// SIBLING-UNIT CONTRACTS:
//   * Row 51 (SHIPPED, 6ce4e79) owns `show_repo_sep` at
//     rust/glean-core/src/repo/text.rs:165. `database_subdir` calls it as
//     `glean_core::repo::text::show_repo_sep("/", repo)` (RULE §4
//     cross-crate form) and does NOT re-implement it — no second owner.
//     glean-core has no Cargo.toml/lib.rs yet (Step 4 owns the scaffold),
//     so the import is unresolvable until that scaffold lands; the edge is
//     recorded as a commented bullet in glean-db/Cargo.toml, not a real
//     dependency entry.
//   * Downstream ports consume this module as
//     `crate::database::repo::{database_subdir, database_path, in_repo}`:
//     Backup.hs (databaseSubdir/inRepo), Catalog.hs (inRepo),
//     Catalog/Local/Files.hs, Storage/LMDB.hs and Storage/RocksDB.hs
//     (databasePath), CompletePredicates.hs, Create.hs, Delete.hs,
//     Exception.hs, Janitor.hs, Open.hs, Write/Batch.hs and
//     gleancli/GleanCLI/Restore.hs (inRepo). They pass the returned String
//     to std::fs through `Path::new` (see FIDELITY NOTES).
//   * This unit appends `pub mod repo;` to rust/glean-db/src/database.rs
//     after `pub mod catalog;`, binding `crate::database::repo` at the
//     crate root. database.rs is Rust-only scaffolding; no lib.rs change
//     is needed (it already declares `pub mod database;`).
//
// EXPORT MAPPING (3/3, module export list :10-12):
//   :20-21  databaseSubdir -> database_subdir
//   :26-27  databasePath   -> database_path
//   :23-24  inRepo         -> in_repo
//
// FIDELITY NOTES:
//   * Return type is `String`, not `PathBuf`. Haskell's `FilePath` is a
//     type synonym for `String`, and no Rust CODE in this tree uses
//     std::path/PathBuf (the only occurrences are this file's own prose).
//     Returning String keeps the source's type image; consumers wrap it
//     with `std::path::Path::new(&s)` when calling std::fs.
//   * `databasePath` uses System.FilePath's `</>`, not
//     `std::path::Path::join`, which diverges on the empty-second-argument
//     case (`Path::new("root").join("")` is `"root/"`, while
//     `"root" </> ""` is `"root"`). The private `combine` mirrors
//     System.FilePath.Posix's exact branches:
//         "" </> b            = b
//         a </> ""            = a
//         a </> absolute_b    = absolute_b   (absolute = starts with '/')
//         a ends with '/'     = a ++ b
//         otherwise           = a ++ "/" ++ b
//     Probe: GHC's compiled probe (/tmp/fp_probe, built from
//     /tmp/fp_probe.hs) prints "root" </> "" = "root", "root/" </> "/abs"
//     = "/abs", "root//" </> "a" = "root//a", "root" </> "./a" =
//     "root/./a", "/" </> "x" = "/x", "" </> "" = "". The Rust probe
//     (/tmp/opencode/repo52-combine.rs) produces byte-identical output for
//     all 11 cases. GHC itself is no longer on PATH in this environment;
//     the coordinator's probe binary was re-run to confirm the table.
//   * The target is POSIX (Linux pinned): `System.FilePath` resolves to
//     the Posix module on this build, so Windows drive-letter/UNC
//     semantics are out of scope for `combine`.
//   * `inRepo` is `format!("{}: {}", database_subdir(repo), s)`: the
//     source concatenates the subdir ONCE (Repo.hs:24), not a path join,
//     and appends `: ` before the message.
//   * Haskell `Repo ->` becomes `&thrift::Repo` (immutable sharing, the
//     catalog/store.rs convention); `String`/`FilePath` inputs become
//     `&str`. There is no error channel: every function is pure and
//     total.

use glean_core::repo::text::show_repo_sep;
use glean_thrift as thrift;

/// Source: databaseSubdir (:20-21) — `showRepoSep "/"`.
pub fn database_subdir(repo: &thrift::Repo) -> String {
    show_repo_sep("/", repo)
}

/// Source: databasePath (:26-27) — `root </> databaseSubdir repo`.
pub fn database_path(root: &str, repo: &thrift::Repo) -> String {
    combine(root, &database_subdir(repo))
}

/// Source: inRepo (:23-24) — `databaseSubdir repo ++ ": " ++ s`.
pub fn in_repo(repo: &thrift::Repo, s: &str) -> String {
    format!("{}: {}", database_subdir(repo), s)
}

/// System.FilePath.Posix `</>` — see FIDELITY NOTES for the probed table.
/// Private: the source exports only the three functions above.
fn combine(a: &str, b: &str) -> String {
    if a.is_empty() {
        return b.to_string();
    }
    if b.is_empty() {
        return a.to_string();
    }
    if b.starts_with('/') {
        return b.to_string();
    }
    if a.ends_with('/') {
        return format!("{}{}", a, b);
    }
    format!("{}/{}", a, b)
}

//
// PORT STATUS: confidence=high todos=0
