// Ported from: glean/db/Glean/Database/Catalog/Local/Files.hs (104 lines).
// Manifest unit: rust/glean-db/src/database/catalog/local/files.rs
// (migration/manifest.tsv line 64, crate glean-db). RULE §4: one source
// file -> one target file; the manifest path is authoritative.
//
// WHAT THIS FILE IS: the on-disk metadata store. Each database's catalog
// entry is one JSON file at
//   <root>/<repo name>/<repo hash>/meta
// (:37-38, :101-104). 'list' scans that layout (:44-67), 'create' makes the
// database directory and writes the file (:69-75), 'delete' removes it with
// safeRemovePathForcibly (:77-81), 'put' rewrites it in place (:83-87) and
// 'get' reads it back (:89-99). The type is just the catalog root (:35).
//
// ---------------------------------------------------------------------------
// INVENTORY (RULE §5; migration/inventory*.tsv)
// ---------------------------------------------------------------------------
//
// ZERO rows name this file as their source. Exact command at the repo root:
//
//   $ grep -n 'Catalog/Local/Files\.hs' migration/inventory*.tsv
//   migration/inventory.tsv:111:glean/util/Glean/Util/IO.hs\tHandleIO class ...
//   migration/inventory.protocol.tsv:14:glean/util/Glean/Util/IO.hs\tHandleIO ...
//
// Both hits are the SAME row (inventory.tsv:111 == inventory.protocol.tsv:14,
// LAZY_IO, confirmed), whose source-file column is Util/IO.hs and whose
// evidence column names Files.hs:62 and :94 as the CONSUMERS of
// readFileContents. That row governs row 63 (util/io.rs), not this file.
// A broader grep ('grep -n Files\.hs migration/inventory*.tsv') returns
// only those same two lines. inventory.ffi.tsv, inventory.codeflow.tsv,
// inventory.sites.tsv and inventory.stm.tsv: zero hits.
//
// No INVENTORY-BUG-CANDIDATE applies: no row is diverged from.
//
// ---------------------------------------------------------------------------
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, checked not assumed)
// ---------------------------------------------------------------------------
//
//   * OUTBOUND (5), all shipped:
//       edges.tsv:177 Catalog/Store.hs     -> row 50, catalog/store.rs
//       edges.tsv:178 Database/Exception.hs-> row 53, database/exception.rs
//       edges.tsv:179 Database/Meta.hs     -> row 48, database/meta.rs
//       edges.tsv:180 Database/Repo.hs     -> row 52, database/repo.rs
//       edges.tsv:181 Util/IO.hs           -> row 63, util/io.rs
//   * INBOUND (1): edges.tsv:218 Config.hs -> Files.hs (manifest line 85,
//     NOT shipped). Config.hs:141 constructs the store with fileCatalog;
//     see SIBLING-UNIT CONTRACTS.
//   * The remaining imports are base or external: Control.Monad (:14),
//     Data.ByteString (:15), Data.HashMap.Strict (:16), Data.List (:17),
//     Data.Maybe (:18), Data.Text (:19), System.Directory (:20),
//     System.FilePath (:21), Thrift.Protocol.JSON (:23),
//     Util.Control.Exception (:24), Util.IO (:25), Util.Log (:26). The
//     generated Glean.Types (:32) and hsthrift (:23) are assumed imports
//     (RULE §4 B4). Util.IO and Util.Log are fb-util (no manifest unit).
//
// ---------------------------------------------------------------------------
// ASSUMED IMPORTS (RULE §4 B4)
// ---------------------------------------------------------------------------
//
//   * glean_thrift::Repo. Pinned shape (glean/if/glean.thrift:106-109):
//         struct Repo { 1: string name; 2: string hash; }
//     Pinned image, recorded as a commented spec block at
//     rust/glean-db/src/backend/types.rs:171-175:
//         #[derive(Clone, Debug, PartialEq, Eq, Hash)]
//         pub struct Repo { pub name: String, pub hash: String }
//     The source constructs the value positionally, 'Repo repo_name
//     (Text.pack hash)' (:65); the Rust literal uses the pinned field
//     names: name = subdir, hash = hash.
//   * Meta: row 48 shipped the re-export
//     'pub use crate::internal::types::Meta;' at database/meta.rs:529. Its
//     serde obligation is recorded at meta.rs:267-270 and :735-810: the
//     generated struct must implement Serialize + Deserialize with the
//     hsthrift JSON shape. This unit consumes exactly that obligation.
//   * serde_json for hsthrift's serializeJSON/deserializeJSON (RULE §1 JSON
//     row): 'to_vec' / 'from_slice', as in backup/mock.rs:238/:251;
//     meta.rs:770/:808 are the to_string/from_str siblings of the same
//     serde obligation.
//   * glean_util::log::log_error: Util.Log's logError has no manifest unit;
//     the pinned seam is 'pub fn log_error(msg: impl FnOnce() -> String)'
//     (A10; the catalog.rs and util/io.rs convention). Every message here is
//     forced immediately, as Util.Log.String.hs:32-35/:61-62 does.
//   * glean_util::util::io::{read_file_contents, write_file_contents}:
//     row 63 shipped both. 'readFileContents' is the bytes read used at
//     :62 and :94; 'writeFileContents' is the same atomic temp+rename as
//     fb-util's 'writeFileAtomically' (DIVERGENCES 1).
//   * fb-util's 'safeRemovePathForcibly' (Util/IO.hs:414-415) has no
//     manifest unit: it is 'callProcess "rm" ["-rf", path]'. The crate
//     already owns the same semantics in
//     'backup::backend::remove_path_forcibly' (backend.rs:391-403), so this
//     file reuses that one implementation instead of forking a second
//     (DIVERGENCES 2).
//
// ---------------------------------------------------------------------------
// SIBLING-UNIT CONTRACTS
// ---------------------------------------------------------------------------
//
//   * Store (row 50, SHIPPED, catalog/store.rs): the five methods this impl
//     provides. The trait's 'list' returns an OWNED map; this impl builds it
//     with one insert per database.
//   * StoreError (row 50, EXTENDED HERE): row 50 and exception.rs:111-117
//     recorded the forward contract that the Files 'get' arm should gain a
//     typed Db variant rather than erasing DbError into StoreError::Other.
//     This unit consumes it: StoreError now has 'Db(DbError)' and
//     'From<DbError>', and 'get' uses 'db_error(repo, ..)?' directly.
//   * CatalogStore (row 50, catalog/store.rs): its 'Files' variant holds
//     'crate::database::catalog::local::files::Files'. Files is
//     'Send + Sync' automatically (one String), which is the Step-4 thread
//     obligation recorded at store.rs (the committer holds an
//     Arc<CatalogInner>). The 'Memory' variant stays a forward reference
//     until row 65.
//   * Meta (row 48): 'crate::database::meta::Meta'; no field is read here,
//     only the serde round-trip.
//   * Repo (row 52, SHIPPED): 'database_path' and the pub(crate) 'combine'
//     are the single owners of System.FilePath's '</>'; this file calls them
//     and does not re-implement path joining.
//   * Exception (row 53, SHIPPED): 'db_error' is the single throw funnel;
//     'get' calls it at the parse-failure site (:97).
//   * Util/IO (row 63, SHIPPED): 'read_file_contents' and
//     'write_file_contents'.
//   * Backup/Backend (row 56, SHIPPED): 'remove_path_forcibly' is a
//     pub(crate) seam (backend.rs:391-403), already reused by storage.rs.
//   * Memory (row 65, future): 'catalog/local.rs' will gain
//     'pub mod memory;' and this module's sibling file; nothing here
//     references it.
//   * Config (row 85, future): Config.hs:141 calls 'fileCatalog path'; the
//     Rust call is 'file_catalog(path)'.
//
// ---------------------------------------------------------------------------
// EXPORT MAPPING (2/2 source exports, names at :10-11; StoreError addition is Rust-only)
// ---------------------------------------------------------------------------
//
//   :10 Files       -> pub struct Files (field private, like the newtype)
//   :11 fileCatalog -> pub fn file_catalog
// Private, as in the source:
//   :37  metaPath  -> meta_path
//   :101 writeMeta -> write_meta
//   instance Store Files (:43-99) -> impl Store for Files
// Rust-only helpers: does_file_exist (System.Directory's doesFileExist),
// read_directory, read_meta (the read+parse half of 'list'), and the
// imported remove_path_forcibly.
//
// ---------------------------------------------------------------------------
// FIDELITY NOTES
// ---------------------------------------------------------------------------
//
//   * 'list' (:44-67). 'doesDirectoryExist' is 'Path::is_dir' (both return
//     false on any lookup error). The root listing at :47 is NOT inside
//     catchAll, so its failure propagates as StoreError::Io. The per-repo
//     listing at :51-54 IS caught: it logs
//     "couldn't read directory <dir>: <err>" and yields []. The per-hash
//     read+parse at :55-67 is caught too: an IO error or a parse error logs
//     "<path>: <err>" and skips that hash. 'HashMap.fromList' becomes
//     repeated 'insert' (last wins, as fromList does).
//   * 'getDirectoryContents' is 'std::fs::read_dir', read to completion:
//     an error on any entry fails the whole listing, matching the Haskell
//     action, which throws before returning a list. read_dir omits the "."
//     and ".." entries that the source's filter removes anyway
//     ('isPrefixOf "."', :47/:51), so the visible set is identical.
//   * File names are OsString -> String with to_string_lossy
//     (FilePath -> String per RULE §1); a non-UTF-8 name is
//     replacement-decoded (DIVERGENCES 3).
//   * doesFileExist is 'not . pathIsDirectory' (OsPath.hs:1266-1272), not
//     Path::is_file: an existing FIFO, socket or device node counts as
//     present. does_file_exist reproduces the wider predicate; metadata
//     follows symlinks and any lookup error is False.
//   * 'create' (:69-75) checks doesFileExist on the meta FILE, then
//     createDirectoryIfMissing True on databasePath (create_dir_all), then
//     writes. The source's check-then-act race is preserved; only the write
//     itself is atomic. 'put' and 'delete' likewise act only when the meta
//     file exists.
//   * 'writeMeta' (:101-104) is writeFileAtomically: serialize the Meta to
//     JSON bytes and replace the file through a temp file in the same
//     directory. serde_json::to_vec is used with the same .expect as
//     meta.rs:771 (the generated shape cannot fail to serialize).
//   * 'get' (:89-99) has a SOURCE DEFECT, reproduced below: 'path' is
//     already metaPath ('.../meta') and the read appends "meta" again
//     (:94), so it opens '.../meta/meta' while writeMeta writes
//     '.../meta'. The parse-error message names the un-joined 'path', as
//     :97 does. See the source-defect marker at the site. Because
//     doesFileExist is false for a directory, the read's Ok/Err arms are
//     unreachable while the defect stands: a regular '.../meta' file makes
//     the read fail with ENOTDIR, and a directory there makes the
//     existence check fail. Both arms are ported as written.
//   * The source's three "make this atomic if necessary" TODO comments
//     (:70, :78, :84) are source notes, not port deferrals: the writes are
//     already temp+rename and the check-then-act windows are unchanged. No
//     port marker is added for them.
//
// ---------------------------------------------------------------------------
// DIVERGENCES (deliberate)
// ---------------------------------------------------------------------------
//
//   1. Temp-file name. fb-util's writeFileAtomically opens
//      '<target>.tmp<random>' beside the target (Util/IO.hs:118-127); row
//      63's write_file_contents opens '.glean<pid>-<n>' in the same
//      directory. Both write, close, rename and remove on error; only the
//      transient name differs, and it is not observable through this module
//      (the temp file is not in a directory 'list' scans).
//   2. safeRemovePathForcibly is not a subprocess. fb-util forks
//      'rm -rf' because GHC's unlink FFI can stall (Util/IO.hs:405-415);
//      that hazard is a GHC FFI concern, and the Rust std path already
//      issues the syscall directly. 'remove_path_forcibly' has the same
//      recursive-removal semantics for every path 'delete' can reach.
//   3. Non-UTF-8 file names are replacement-decoded (to_string_lossy); the
//      Haskell FilePath was a String produced by the locale-aware
//      getDirectoryContents.
//   4. Diagnostic text. 'show exc' (GHC's IOException rendering) and
//      hsthrift's parse-error strings are replaced by io::Error's Display
//      and serde_json's Display. In the LOG lines the '<path>: ' prefix is
//      exact and only the detail differs. On a PROPAGATED io::Error (the
//      root listing, create/put/delete, get's read) GHC's Show included the
//      path while Rust's Display does not; the error kind and the control
//      flow are unchanged, but the text carried to the caller differs.
//   5. No ordering contract is added or lost: both sides use unordered maps
//      (Data.HashMap.Strict and std HashMap).
//
// ---------------------------------------------------------------------------
// MARKERS
// ---------------------------------------------------------------------------
//
// One source-defect reproduction (the 'get' double-join, marked below).
// Zero port todos, zero performance deferrals, zero unsafe blocks.
//
// PORT STATUS: confidence=high todos=0

use std::collections::HashMap;
use std::path::Path;

use glean_thrift as thrift;
use glean_util::log::log_error;
use glean_util::util::io::{read_file_contents, write_file_contents};

use crate::database::backup::backend::remove_path_forcibly;
use crate::database::catalog::store::{Store, StoreError};
use crate::database::exception::db_error;
use crate::database::meta::Meta;
use crate::database::repo::{combine, database_path};

/// 'Files' (:35): the catalog root. The source's newtype hides its field
/// (the module exports only the type name, :10-11), so the field is private
/// here too; 'file_catalog' is its only constructor.
pub struct Files {
    root: String,
}

/// 'metaPath' (:37-38): 'databasePath root repo </> "meta"'.
fn meta_path(root: &str, repo: &thrift::Repo) -> String {
    combine(&database_path(root, repo), "meta")
}

/// 'fileCatalog' (:40-41): the newtype constructor.
pub fn file_catalog(root: &str) -> Files {
    Files {
        root: root.to_string(),
    }
}

impl Store for Files {
    /// :44-67. A missing root is the empty catalog; root-listing failures
    /// propagate; per-repo and per-hash failures are logged and skipped.
    fn list(&self) -> Result<HashMap<thrift::Repo, Meta>, StoreError> {
        if !Path::new(&self.root).is_dir() {
            return Ok(HashMap::new());
        }

        let dirs = read_directory(&self.root)?;
        let mut entries = HashMap::new();

        for subdir in dirs.into_iter().filter(|name| !name.starts_with('.')) {
            let dir = combine(&self.root, &subdir);
            let hashes = match read_directory(&dir) {
                Ok(hashes) => hashes,
                Err(e) => {
                    log_error(|| format!("couldn't read directory {}: {}", dir, e));
                    Vec::new()
                }
            };

            for hash in hashes.into_iter().filter(|name| !name.starts_with('.')) {
                let path = combine(&dir, &hash);
                match read_meta(&path) {
                    Ok(meta) => {
                        entries.insert(
                            thrift::Repo {
                                name: subdir.clone(),
                                hash,
                            },
                            meta,
                        );
                    }
                    Err(msg) => log_error(|| format!("{}: {}", path, msg)),
                }
            }
        }

        Ok(entries)
    }

    /// :69-75. Create the database directory and the meta file only when
    /// the meta file does not exist.
    fn create(&self, repo: &thrift::Repo, meta: &Meta) -> Result<bool, StoreError> {
        let new = !does_file_exist(&meta_path(&self.root, repo));
        if new {
            std::fs::create_dir_all(database_path(&self.root, repo))?;
            write_meta(&self.root, repo, meta)?;
        }
        Ok(new)
    }

    /// :77-81. Remove the meta file if it exists; report whether it did.
    fn delete(&self, repo: &thrift::Repo) -> Result<bool, StoreError> {
        let path = meta_path(&self.root, repo);
        let exists = does_file_exist(&path);
        if exists {
            remove_path_forcibly(&path)?;
        }
        Ok(exists)
    }

    /// :83-87. Rewrite the meta file only when it exists.
    fn put(&self, repo: &thrift::Repo, meta: &Meta) -> Result<bool, StoreError> {
        let path = meta_path(&self.root, repo);
        let exists = does_file_exist(&path);
        if exists {
            write_meta(&self.root, repo, meta)?;
        }
        Ok(exists)
    }

    /// :89-99. Read the meta file if it exists; a parse failure is a
    /// DbError through the single throw funnel (:97).
    fn get(&self, repo: &thrift::Repo) -> Result<Option<Meta>, StoreError> {
        let path = meta_path(&self.root, repo);
        if !does_file_exist(&path) {
            return Ok(None);
        }

        // BUG(port): the source sets 'path = metaPath root repo' (already
        // '.../meta') and then reads 'path </> "meta"' (:90, :94), i.e.
        // '.../meta/meta', while writeMeta writes the '.../meta' FILE.
        // Reproduced bug-for-bug per RULE §2; the parse-error message also
        // names the un-joined 'path', as :97 does.
        let bytes = read_file_contents(&combine(&path, "meta"))?;
        match serde_json::from_slice::<Meta>(&bytes) {
            Ok(meta) => Ok(Some(meta)),
            Err(err) => db_error(repo, &format!("can't parse '{}': {}", path, err))?,
        }
    }
}

/// 'writeMeta' (:101-104): writeFileAtomically with the hsthrift JSON
/// writer. The atomic temp+rename is row 63's write_file_contents
/// (DIVERGENCES 1).
fn write_meta(root: &str, repo: &thrift::Repo, meta: &Meta) -> std::io::Result<()> {
    let path = meta_path(root, repo);
    let bytes = serde_json::to_vec(meta)
        .expect("Meta JSON serialization is infallible for the generated shape");
    write_file_contents(&path, &bytes)
}

/// System.Directory's doesFileExist: 'True' when the path exists and is not
/// a directory, 'False' on any lookup error. It is NOT Path::is_file: a
/// FIFO, socket or device node is "a file" here (OsPath.hs:1266-1272,
/// 'not <$> pathIsDirectory' under catchIOError), and the source's
/// create/put/delete/get all branch on that wider predicate.
fn does_file_exist(path: &str) -> bool {
    match std::fs::metadata(path) {
        Ok(md) => !md.is_dir(),
        Err(_) => false,
    }
}

/// 'getDirectoryContents' (:47, :51) as one whole-directory read: an error
/// on any entry fails the listing, as the Haskell action throws before
/// returning its list. read_dir omits "." and "..", which the source's
/// filter removes anyway.
fn read_directory(path: &str) -> std::io::Result<Vec<String>> {
    let mut names = Vec::new();
    for entry in std::fs::read_dir(path)? {
        names.push(entry?.file_name().to_string_lossy().into_owned());
    }
    Ok(names)
}

/// The read-and-parse half of 'list' (:62-66): both an IO failure
/// (catchAll's 'show exc') and a parse failure (the Left string) become one
/// message for the '<path>: <msg>' log line (DIVERGENCES 4).
fn read_meta(path: &str) -> Result<Meta, String> {
    let bytes = read_file_contents(&combine(path, "meta")).map_err(|e| e.to_string())?;
    serde_json::from_slice::<Meta>(&bytes).map_err(|e| e.to_string())
}
