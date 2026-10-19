// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// Ported from: glean/db/Glean/Database/Backup/Backend.hs (95 lines).
// Manifest unit: rust/glean-db/src/database/backup/backend.rs
// (migration/manifest.tsv line 56, crate glean-db). RULE §4: one source
// file -> one target file; the manifest path is authoritative.
//
// WHAT THIS FILE IS: the backup-backend interfaces. A `Backend` (this
// module's trait — a homonym of `crate::backend::types::Backend`, the RPC
// backend; the source has the same two classes, `Glean.Backend.Backend` and
// `Glean.Database.Backup.Backend.Backend`, so path-qualified imports keep
// them apart) parses a textual site path into a `Site`; a `Site` is a
// place a database can be
// backed up to, inspected, restored from, deleted from and enumerated
// (:32-64). The module also owns the `Data` result (a byte count), the
// `RestoreSource` sum handed to restore continuations (:27-29), the
// `Backends` lookup table (:96), and the file-based restore helper
// `restoreViaFile` (:67-85) that downloads to a unique scratch file and
// cleans it up on every exit path.
//
// ---------------------------------------------------------------------------
// INVENTORY (RULE §5; migration/inventory*.tsv)
// ---------------------------------------------------------------------------
//
// ZERO rows name this source file. Exact command, run at the repo root:
//   $ grep -n 'Database/Backup/Backend\.hs' migration/inventory*.tsv
//   (no output; exit status 1)
// Repeated across all six inventory files (inventory.tsv,
// inventory.ffi.tsv, inventory.sites.tsv, inventory.codeflow.tsv,
// inventory.protocol.tsv, inventory.stm.tsv): no row names
// Backup/Backend.hs. A broader `grep -n 'Backup/Backend'` is likewise
// empty (exit status 1).
//
// NEAR-MISSES ARE NOT GOVERNING (checked so the zero-hit claim is not
// mistaken for an omission):
//   * migration/inventory.protocol.tsv:26 is the Backup.hs row (manifest
//     line 125) for doBackup/restore-handler exception taxonomy — its
//     source column is glean/db/Glean/Database/Backup.hs, not this file.
//   * migration/inventory.sites.tsv rows 198-214 and 598-599 are likewise
//     Backup.hs sites (the backuper/restore paths), not this file.
// No INVENTORY-BUG-CANDIDATE applies: this unit diverges from no row.
//
// ---------------------------------------------------------------------------
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, checked not assumed)
// ---------------------------------------------------------------------------
//
//   * OUTBOUND (2):
//       edges.tsv:160  Backup/Backend.hs -> Database/Meta.hs
//                      (manifest line 48, SHIPPED: src/database/meta.rs,
//                      which re-exports `Meta` from crate::internal::types).
//       edges.tsv:161  Backup/Backend.hs -> glean/util/Glean/Util/Some.hs
//                      (NO manifest unit — the existential, see
//                      TRANSLATION NOTES: the port is the two closed enums
//                      below, not an import of Util.Some).
//     Every other source import is base: Control.Exception (`finally`),
//     Data.HashMap.Strict, Data.Text, Data.Unique, System.Directory,
//     System.FilePath, System.IO — no edges.
//   * INBOUND (13), NONE edited by this unit: edges.tsv:142 Backup.hs,
//     162 Locator.hs, 167 Mock.hs, 214 Config.hs, 309 List.hs,
//     353 Restore.hs, 404 Storage.hs, 411 Storage/DB.hs,
//     420 Storage/LMDB.hs, 435 Storage/RocksDB.hs, 446 Types.hs,
//     731 S3.hs, 1777 tools/gleancli/GleanCLI/Restore.hs.
//
// ---------------------------------------------------------------------------
// TRANSLATION NOTES
// ---------------------------------------------------------------------------
//
// * EXISTENTIALS -> TWO CLOSED ENUMS. `Some Backend` and `Some Site`
//   (:43-44, :87-94) are the existential wrappers of Glean.Util.Some (no
//   manifest unit; RULE §2's existential row). The variant set is CLOSED:
//   `grep -rn 'instance Backend|instance Site' glean/db glean/db/backup`
//   finds only Mock.hs:44/:47 and S3.hs:101/:265; the two `instance …
//   (Some …)` hits in this file ARE the forwarding instances, and
//   Locator.hs defines none. The images are therefore
//   `SomeBackupBackend { Mock, S3 }` and `SomeBackupSite { Mock, S3 }`
//   (the SomeBackend precedent, backend/types.rs:745-778), with
//   method-for-method forwarding impls of the two traits.
//   `Box<dyn Site>` is additionally IMPOSSIBLE, not merely unidiomatic:
//   `Site::restore` is generic over the continuation's result `B`
//   (Haskell's `IO b`), and a generic method makes the trait not
//   object-safe. The closed enum is forced by object safety as well as by
//   the closed variant set.
//   The names cannot be `SomeBackend`/`SomeSite`: the source has no name
//   of its own for these types, and `SomeBackend` is already taken by
//   backend/types.rs. The variant payload types are forward references to
//   rows 61 (Mock.hs -> src/database/backup/mock.rs) and 177 (S3.hs ->
//   src/database/backup/s3.rs), same crate.
//
// * TEXT/FILEPATH -> &str/String. Haskell `Text` (site paths) and
//   `FilePath` (= String) both become `&str` on input and `String` on
//   output, per RULE §1. No lossy conversions are introduced.
//
// * Handle -> std::fs::File. `System.IO.Handle` (:29) has no std image;
//   the pinned replacement is `std::fs::File` (RULE §1: std::fs/std::io).
//   ASSUMPTION: both in-tree consumers only READ the stream —
//   Storage.hs:74 materializes it with `hCopyToFile` and RocksDB.hs:178
//   untars from it (`unTarFromHandle`) — so a readable `File` covers the
//   observed contract. A consumer needing a non-file handle (pipe, socket)
//   would extend this variant rather than re-type it; no such consumer
//   exists in the 13 inbound edges.
//
// * Int -> i64. `Data.dataSize :: Int` (:23) becomes `i64`. The field is
//   exported by `Data(..)` and therefore stays `pub`.
//
// * IO -> Result. Every `IO` in `Site` becomes
//   `Result<_, BackupError>`. `BackupError` is this area's dynamic-
//   exception funnel (RULE §2), shaped exactly like StoreError
//   (catalog/store.rs:305-335): `Io` for std::fs/std::io failures and
//   `Other` for the open leg. Forward contract: the throwing unit (row
//   177, S3) adds a typed `InvalidLocator` arm only if a consumer needs to
//   match it; until then callers downcast `Other`.
//
// * `finally removePathForcibly` (:82). Cleanup runs on success, on
//   failure AND on panic, and a cleanup ERROR replaces the body's result
//   (Haskell `finally` semantics): the normal path computes
//   `(result, cleanup)` and applies `(_, Err(e)) => Err(e)` before
//   `(r, Ok(())) => r`. A `RemoveOnDrop` guard covers the panic path
//   (disarmed once the explicit cleanup has run).
//
// * `Data.Unique` newUnique/hashUnique -> a process-global
//   `AtomicU64`. The generated name is scratch-internal, so only
//   uniqueness matters: the counter value is NOT the Haskell hash and is
//   not observable by any consumer. The name is built with row 52's
//   `combine` (`scratch </> …`), widened to `pub(crate)` for this one
//   owner rather than forking a second `</>`.
//
// * Send + Sync OBLIGATION (rows 61/177): the backuper thread in Backup.hs
//   holds `Backends` across threads. The source has no such constraint,
//   but the Rust Mock/S3 payloads must be `Send + Sync` (or the table must
//   be confined to one thread) for that use. Recorded here so rows 61/177
//   satisfy it in their impls; this file adds no supertrait the source
//   lacks.
//
// * SHOW: neither `Data` nor `RestoreSource` has a derived Show in the
//   source, so no Display/Debug is invented for them. `BackupError`
//   derives Debug only (the area error type has no source Show text).
//
// EXPORT MAPPING (6/6, module export list :11-17):
//   :12  Backend(..)       -> `Backend` trait + `SomeBackupBackend`
//   :13  Site(..)          -> `Site` trait + `SomeBackupSite`
//   :14  Data(..)          -> `Data`
//   :15  Backends          -> `Backends`
//   :16  RestoreSource(..) -> `RestoreSource`
//   :17  restoreViaFile    -> `restore_via_file`

use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

use glean_thrift as thrift;

use crate::database::meta::Meta;
use crate::database::repo::combine;

/// Source: `newtype Data = Data { dataSize :: Int }` (:23). The field is
/// exported by `Data(..)`; `Int` -> `i64`.
pub struct Data {
    pub data_size: i64,
}

/// Source: `RestoreSource` (:27-29). `SourceFile FilePath` ->
/// `File(String)`; `SourceStream Handle` -> `Stream(std::fs::File)`
/// (the Handle image is an assumption documented in TRANSLATION NOTES).
pub enum RestoreSource {
    File(String),
    Stream(std::fs::File),
}

/// The backup area's `SomeException` image (RUST-ONLY; see the
/// TRANSLATION NOTES entry for `IO -> Result`). Shaped like StoreError
/// (catalog/store.rs:305-335): `Io` for std::fs/std::io failures,
/// `Other` for caller-raised errors. Row 177 may add a typed
/// `InvalidLocator` arm if a consumer needs to match it.
#[derive(Debug)]
pub enum BackupError {
    Io(std::io::Error),
    Other(Box<dyn std::error::Error + Send + Sync>),
}

impl From<std::io::Error> for BackupError {
    fn from(e: std::io::Error) -> Self {
        BackupError::Io(e)
    }
}

/// Constructor-shaped, per RULE §2's deriving-Show default (the type has
/// no source counterpart, so there is no Show text to replicate).
impl fmt::Display for BackupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BackupError::Io(e) => write!(f, "Io({})", e),
            BackupError::Other(e) => write!(f, "Other({})", e),
        }
    }
}

/// `source()` stays the default: the catch taxonomy is Step 4's, as with
/// BackendError and StoreError.
impl std::error::Error for BackupError {}

/// Source: `class Backend a` (:32-37) — parse the textual representation
/// of a `Site`. `Text` -> `&str`.
pub trait Backend {
    fn from_path(&self, path: &str) -> Option<SomeBackupSite>;
}

/// Source: `class Site a` (:46-64). Source order is preserved; every
/// `IO` becomes `Result<_, BackupError>`, and `a ->` becomes `&self`.
pub trait Site {
    /// :47-53 — back up `repo` into `from`, returning the size written.
    fn backup(
        &self,
        repo: &thrift::Repo,
        meta: &Meta,
        ttl: Option<i64>,
        from: &str,
    ) -> Result<Data, BackupError>;

    /// :54 — read the backup metadata for `repo`.
    fn inspect(&self, repo: &thrift::Repo) -> Result<Meta, BackupError>;

    /// :55-62 — restore `repo`, passing a `RestoreSource` to `action`
    /// and returning the action's result together with the backup Meta.
    /// `scratch` is a scratch directory implementations may use for
    /// intermediate files.
    fn restore<B, F>(
        &self,
        repo: &thrift::Repo,
        scratch: &str,
        action: F,
    ) -> Result<(B, Meta), BackupError>
    where
        F: FnOnce(RestoreSource) -> Result<B, BackupError>;

    /// :63 — delete the backup for `repo`.
    fn delete(&self, repo: &thrift::Repo) -> Result<(), BackupError>;

    /// :64 — all backups known to this site.
    fn enumerate(&self) -> Result<Vec<(thrift::Repo, Meta)>, BackupError>;

    /// :65 — the textual site path (the inverse of `from_path`).
    fn to_path(&self) -> String;
}

/// The closed-set image of Haskell's existential `Some Backend` (:43-44).
/// The variant set is closed (only Mock.hs and S3.hs define instances —
/// see TRANSLATION NOTES); the payload types are rows 61/177's forward
/// references.
pub enum SomeBackupBackend {
    Mock(crate::database::backup::mock::MockBackend),
    S3(crate::database::backup::s3::S3Backend),
}

/// `instance Backend (Some Backend)` (:43-44): forward to the wrapped
/// backend.
impl Backend for SomeBackupBackend {
    fn from_path(&self, path: &str) -> Option<SomeBackupSite> {
        match self {
            SomeBackupBackend::Mock(x) => x.from_path(path),
            SomeBackupBackend::S3(x) => x.from_path(path),
        }
    }
}

/// The closed-set image of Haskell's existential `Some Site` (:87-94).
/// `Box<dyn Site>` cannot express this trait (generic `restore`), so the
/// enum is forced by object safety as well as by the closed variant set.
pub enum SomeBackupSite {
    Mock(crate::database::backup::mock::MockSite),
    S3(crate::database::backup::s3::S3Site),
}

/// `instance Site (Some Site)` (:87-94): all six methods forward to the
/// wrapped site, in source order.
impl Site for SomeBackupSite {
    fn backup(
        &self,
        repo: &thrift::Repo,
        meta: &Meta,
        ttl: Option<i64>,
        from: &str,
    ) -> Result<Data, BackupError> {
        match self {
            SomeBackupSite::Mock(x) => x.backup(repo, meta, ttl, from),
            SomeBackupSite::S3(x) => x.backup(repo, meta, ttl, from),
        }
    }

    fn inspect(&self, repo: &thrift::Repo) -> Result<Meta, BackupError> {
        match self {
            SomeBackupSite::Mock(x) => x.inspect(repo),
            SomeBackupSite::S3(x) => x.inspect(repo),
        }
    }

    fn restore<B, F>(
        &self,
        repo: &thrift::Repo,
        scratch: &str,
        action: F,
    ) -> Result<(B, Meta), BackupError>
    where
        F: FnOnce(RestoreSource) -> Result<B, BackupError>,
    {
        match self {
            SomeBackupSite::Mock(x) => x.restore(repo, scratch, action),
            SomeBackupSite::S3(x) => x.restore(repo, scratch, action),
        }
    }

    fn delete(&self, repo: &thrift::Repo) -> Result<(), BackupError> {
        match self {
            SomeBackupSite::Mock(x) => x.delete(repo),
            SomeBackupSite::S3(x) => x.delete(repo),
        }
    }

    fn enumerate(&self) -> Result<Vec<(thrift::Repo, Meta)>, BackupError> {
        match self {
            SomeBackupSite::Mock(x) => x.enumerate(),
            SomeBackupSite::S3(x) => x.enumerate(),
        }
    }

    fn to_path(&self) -> String {
        match self {
            SomeBackupSite::Mock(x) => x.to_path(),
            SomeBackupSite::S3(x) => x.to_path(),
        }
    }
}

/// Source: `type Backends = HashMap Text (Some Backend)` (:96).
pub type Backends = HashMap<String, SomeBackupBackend>;

/// Source: `restoreViaFile` (:67-85). Download into a unique file under
/// `scratch`, hand a `SourceFile` to `action`, and remove the file on
/// every exit path (the `finally removePathForcibly` mapping above).
pub fn restore_via_file<B, D, A>(
    scratch: &str,
    download: D,
    action: A,
) -> Result<(B, Meta), BackupError>
where
    D: FnOnce(&str) -> Result<Meta, BackupError>,
    A: FnOnce(RestoreSource) -> Result<B, BackupError>,
{
    let tmp_file = combine(scratch, &format!("glean-restore{}", next_unique()));
    let mut guard = RemoveOnDrop::new(&tmp_file);

    let result: Result<(B, Meta), BackupError> = (|| {
        let meta = download(&tmp_file)?;
        let result = action(RestoreSource::File(tmp_file.clone()))?;
        Ok((result, meta))
    })();

    // Haskell's finally: the cleanup runs on every path, and a cleanup
    // error replaces the body's result (the first match arm).
    let cleanup = remove_path_forcibly(&tmp_file);
    guard.disarm();
    match (result, cleanup) {
        (_, Err(e)) => Err(BackupError::Io(e)),
        (r, Ok(())) => r,
    }
}

/// Process-global replacement for `Data.Unique.newUnique` (:80). Only
/// uniqueness is observable; the value is not the Haskell hash.
static NEXT_UNIQUE: AtomicU64 = AtomicU64::new(0);

/// SIBLING-UNIT SEAM (added for migration unit 58 — database/storage.rs):
/// `withMaterializedFile` needs the same unique-file source. Additive; no
/// behavior change.
pub(crate) fn next_unique() -> u64 {
    NEXT_UNIQUE.fetch_add(1, Ordering::Relaxed)
}

/// System.Directory's removePathForcibly: a missing path is success; a
/// directory is removed recursively; anything else is removed as a file.
/// `symlink_metadata` keeps a symlink-to-directory from being followed.
///
/// SIBLING-UNIT SEAM (unit 58): `pub(crate)` so storage.rs's
/// `with_materialized_file` uses the one implementation.
pub(crate) fn remove_path_forcibly(path: &str) -> std::io::Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(md) => {
            if md.file_type().is_dir() {
                std::fs::remove_dir_all(path)
            } else {
                std::fs::remove_file(path)
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

/// The panic-path half of Haskell's finally: removes `path` on drop unless
/// the explicit cleanup already ran. Disarmed after the explicit cleanup so
/// a normal return does not remove twice. A cleanup ERROR on the panic path
/// is swallowed (Drop cannot propagate); the normal path applies the
/// finally precedence above.
/// SIBLING-UNIT SEAM (unit 58): `pub(crate)` so storage.rs's
/// `with_materialized_file` gets the same panic-path guard.
pub(crate) struct RemoveOnDrop<'a> {
    path: &'a str,
    armed: bool,
}

impl<'a> RemoveOnDrop<'a> {
    pub(crate) fn new(path: &'a str) -> Self {
        RemoveOnDrop { path, armed: true }
    }

    pub(crate) fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for RemoveOnDrop<'_> {
    fn drop(&mut self) {
        if self.armed {
            let _ = remove_path_forcibly(self.path);
        }
    }
}

//
// COVERAGE: 6/6 exports mapped (see EXPORT MAPPING). Source order is
// preserved except that the two closed enums are grouped with their
// forwarding impls and `restoreViaFile` follows them, so its unique
// counter and Drop guard sit beside their only caller (the source has
// restoreViaFile at :67-85, before the Site forwarding instance at :87-94
// and Backends at :96).
//
// MARKERS: zero of every class — 0 port todos (the two payload enums are
// forward references, documented above, not deferrals), 0 source-defect
// reproductions, 0 performance deferrals, 0 inventory divergences, 0
// SAFETY notes (no unsafe code).
//
// PORT STATUS: confidence=medium todos=0
