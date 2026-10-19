// Ported from: glean/db/Glean/Database/Storage.hs (324 lines)
// Manifest unit: rust/glean-db/src/database/storage.rs
// (migration/manifest.tsv line 58, crate glean-db). RULE §4: one source
// file -> one target file; the manifest path is authoritative.
//
// TYPE-MAPPING TRUTH (this unit is a trait layer, not an FFI wrapper):
//   * 'class Storage s' (Storage.hs:130-180) -> trait Storage with the
//     associated type Database (the image of 'data family Database s',
//     :127). The data family becomes an associated type because every
//     instance fixes exactly one Database; Rust has no data families and
//     an associated type is the direct encoding (RULE §2 GADT/family row).
//   * 'class CanLookup db => DatabaseOps db' (:182-294) -> trait
//     DatabaseOps: CanLookup (the supertrait is the source's context).
//   * 'data family Database s' (:127) -> Storage::Database; the exported
//     NAME is preserved by the type alias Database<S> below.
//   * 'data WriteLock w = WriteLock w' (:122) -> WriteLock<W>(pub W),
//     keeping the VALUE (not a phantom): the only instantiations are at
//     WriteLock () (Open.hs:653 builds the mutex's token;
//     CompletePredicates.hs:263 passes 'undefined :: WriteLock ()' in the
//     no-lock branch), but the type parameter stays for parity with the
//     source's w.
//   * 'instance CanLookup (Some DatabaseOps)' (:296-298) and 'instance
//     DatabaseOps (Some DatabaseOps)' (:300-324) are NOT translated here:
//     inventory.tsv row 137 pins 'Some DatabaseOps' to a CLOSED
//     'enum Storage {Rocks, Lmdb, Memory}' owned by
//     rust/glean-db/src/database/types.rs (manifest line 104,
//     glean/db/Glean/Database/Types.hs). Defining that enum
//     before the three engine types exist would be a forward reference to
//     unshipped modules; the forwarding impls are owed by that unit.
//     TODO(port) below.
//
// ABI/representation truth: this file has NO externs and NO unsafe. The
// concrete engines (Storage/RocksDB.hs, Storage/LMDB.hs, Storage/Memory.hs,
// Storage/DB.hs) own the C ABI in their own units; this unit only fixes
// the trait surface they implement and the helper functions.
//
// ROW 67'S WIRING CONTRACT — CONSUMED (row 67 shipped
// src/database/storage/db.rs): the append is `pub mod db;` at the bottom of
// this file, binding Storage/DB.hs's port as a child of this file (the
// source's module nesting is Storage.hs <- Storage/DB.hs). The remaining
// engine units (row 70 LMDB, row 71 Memory, row 72 RocksDB) will append
// their own lines.
//
// INVENTORY (RULE §5):
//   * inventory.tsv row 19 (line 19, the strict-eager-reads group) names
//     Storage.hs:81 as a COMMENT stating the source already avoids lazy
//     IO. Followed: h_copy_to_file is a strict, chunked read/write loop
//     (the source's own design, :79-98), never a lazy stream.
//   * inventory.tsv row 125 (line 125, DB admin STM_GROUP) lists
//     Storage.hs among the "atomically/env reads only; no new state
//     groups" files. Followed: the trait declares no state; the file
//     helpers (with_materialized_file/h_copy_to_file) touch only the
//     filesystem, and can_open_version/current_version are pure.
//   * inventory.tsv row 137 (line 137) governs the 'Some DatabaseOps'
//     handle: "enum Storage {Rocks, Lmdb, Memory} (closed — RULEBOOK §2
//     GADT row); match dispatch, no dyn". Followed by DEFERRAL to
//     database/types.rs (see above), not by a dyn object: DatabaseOps has
//     the generic CanLookup::with_lookup<T, E> method, so it is not
//     object-safe anyway — the closed enum is the only encoding. The
//     Storage trait is likewise not object-safe (it has an associated type
//     and the generic with_scratch_root), which is harmless: the source is
//     a type class and every use is statically dispatched — the existential
//     'Some Storage' (Types.hs:251) is unwrapped into a
//     'forall s. Storage s => s -> IO a' continuation (Types.hs:305-336).
//     That SECOND existential also needs a closed-enum image; it is owed by
//     database/types.rs too. inventory row 136 governs the general
//     'Some c' construct (closed enum per use site), while row 137 names
//     only 'Some DatabaseOps'; the two uses are recorded separately rather
//     than inferred from one another.
//   * inventory.sites.tsv row 444 (lazy-io, Storage.hs:81) — the same
//     comment; the chunked copy is the sanctioned implementation.
//
// RULE §1 mappings: Text -> String; ByteString -> Vec<u8>; Handle ->
// std::fs::File; Int -> i64; HashMap -> std::collections::HashMap;
// VS.Vector Fid -> Vec<Fid>; FilePath -> String.
//
// RULE §2 error-recovery: the source methods throw IO exceptions and
// DBError. Every trait method returns Result<_, DbError>, the area funnel
// from database/exception.rs (manifest line 53; inventory.tsv row 24 states
// the funnel: "all db throws funnel through DbError"). The io::Error ->
// DbError conversion is the additive seam in exception.rs (DbError::Other
// is exactly the SomeDBError leg). The helpers are the exception:
// with_materialized_file is generic over E: From<io::Error>, and
// h_copy_to_file returns io::Result; both are lifted at the trait boundary.
//
// DIVERGENCES (deliberate):
//   1. newUnique/hashUnique (Storage.hs:75-76) -> backend.rs's
//      process-global AtomicU64 counter (next_unique). Only uniqueness is
//      observable; the Haskell hash value is not part of any name
//      contract.
//   2. withMaterializedFile is generic over the action's error E with
//      E: From<io::Error> (instead of IO exceptions). Cleanup precedence
//      mirrors 'finally' on the normal path: a cleanup error replaces the
//      body's result, and RemoveOnDrop keeps the panic path leak-free (the
//      same guard backend.rs uses).
//   3. The trait methods take &self for the Haskell's db argument (a
//      shared handle). The source's by-value arguments are taken by
//      reference wherever the implementations only borrow them: commit's
//      &FactSet (the C++ commit only borrows it, Storage/DB.hs:128-129
//      'with facts ... invoke'), the &Repo/&Config handles (the crate-wide
//      Repo convention is &thrift::Repo, backend/types.rs:435; engines
//      that retain the Repo clone it), the &BatchDescriptor, the
//      Option<&Lookup>, the &Inventory, store/retrieve's &[u8] byte
//      strings, registerACLUnits's &[Vec<u8>] name list, and the
//      ownership handles of storeOwnership / addDefineOwnership /
//      computeDerivedOwnership. AxiomOwnership stays by value in
//      addOwnership: it is the one payload the source consumes and the
//      engine must own.
//   4. AxiomOwnership uses Vec<Fid> for VS.Vector Fid (RULE §1's
//      third-party-deps row: std Vec, no vector crate; the glean-util
//      vector unit is an empty module for vector >= 0.13).
//   5. 'UseSpecificSchema SchemaId' (:103): SchemaId is
//      'typedef string SchemaId (hs.newtype)' (glean/if/glean.thrift:91);
//      gen-hs2 collapses a newtype over Text to an alias — the
//      backend/types.rs:162-167 precedent — so it flattens to String.
//   6. RestoreSource::Stream owns a std::fs::File, so the File is closed
//      when with_materialized_file returns; the Haskell Handle (:74-77)
//      is caller-owned and stays open. No caller reads the stream after
//      the call (the continuation receives only the materialized path).
//   7. On a PANIC in the action, RemoveOnDrop's Drop cannot report a
//      cleanup failure, so the panic wins; Haskell's finally would let a
//      cleanup exception replace the body exception. The normal-return
//      precedence is exact. This is the same panic-path caveat
//      backend.rs documents for restoreViaFile.
//
// ASSUMED IMPORTS (RULE §4 B4):
//   * crate::internal::types::StoredSchema — the generated
//     Glean.Internal.Types image. inventory.tsv row 142 is the confirmed
//     row: the module is generated from glean/if/internal.thrift, has no
//     manifest row, and lives at `crate::internal::types` per DECISIONS.md
//     (2026-10-05); the cross-crate caveat (glean-schema also imports it)
//     is recorded there and only moves the import path. Shape sketch
//     (internal.thrift:20-37): struct StoredSchema { schema: Vec<u8>;
//     predicateIds: HashMap<Id, PredicateRef>; versions: HashMap<String,
//     Version> }. It must implement Debug: CreateSchema derives Show in
//     the source (:105) and RULE §2 maps Show to Display; the Display impl
//     below renders the payload with Debug as the closest available image
//     of the generated type's Show.
//   * glean_thrift::{Config, DBVersion, Repo, BatchDescriptor,
//     PredicateStats} — the generated crate (RULE §1 A10; first sketch at
//     backend/types.rs:87-226). DBVersion is 'typedef i64 DBVersion
//     (hs.newtype)' (server_config.thrift:28); gen-hs2 collapses the
//     newtype to `pub type DBVersion = i64` (meta.rs:107-108), so the
//     source's DBVersion(..) constructor and unDBVersion become the i64
//     value itself. Config is server_config.thrift:192. A per-file
//     generated crate would move only these import paths (meta.rs:95-105
//     records the same caveat).
//
// SIBLING-UNIT CONTRACTS (RULE §4; DONE siblings' real APIs read):
//   * glean_core::rts::foreign::lookup — CanLookup, Lookup.
//   * glean_core::rts::foreign::inventory — Inventory.
//   * glean_core::rts::foreign::fact_set — FactSet (borrowed by commit).
//   * glean_core::rts::foreign::ownership — Ownership, ComputedOwnership,
//     DefineOwnership, UnitId (unit 57, this commit's predecessor).
//   * glean_core::rts::types — Fid, Pid.
//   * crate::database::backup::backend (unit 56) — Data, RestoreSource
//     (the source's SourceFile/SourceStream constructors are named
//     File/Stream there, backend.rs:165-166), plus the now-pub(crate)
//     next_unique / remove_path_forcibly / RemoveOnDrop seams added for
//     this unit.
//   * crate::database::exception (unit 53) — DbError, plus the additive
//     From<io::Error> seam.
//   * OPEN CROSS-UNIT ISSUE (no marker here; owed by the engine units):
//     commit takes &FactSet, matching the C++ commit's borrow. Row 67 (DB)
//     has since shipped and needed no choice: every one of its facts
//     consumers only borrows the handle (storage/db.rs commit/add_ownership).
//     The Memory engine's image of 'FactSet.append (dbFacts db) facts'
//     (Storage/Memory.hs:91) is the open case — it needs a mutable fact
//     store behind interior mutability, and FactSet is deliberately
//     !Send + !Sync while append takes &mut self (fact_set.rs:415-424,
//     :879). Row 71 must choose its fact representation or revisit that
//     posture; the trait surface here does not force either answer.

//! The storage-backend trait layer: Storage (engines), DatabaseOps (an
//! open database), Mode/CreateSchema/WriteLock, the two pure version
//! helpers (can_open_version/current_version), and with_materialized_file.

use std::collections::HashMap;
use std::fmt;
use std::io::{self, Read, Write};

use glean_core::rts::foreign::fact_set::FactSet;
use glean_core::rts::foreign::inventory::Inventory;
use glean_core::rts::foreign::lookup::{CanLookup, Lookup};
use glean_core::rts::foreign::ownership::{ComputedOwnership, DefineOwnership, Ownership, UnitId};
use glean_core::rts::types::{Fid, Pid};
use glean_thrift as thrift;

use crate::database::backup::backend::{next_unique, remove_path_forcibly, Data, RemoveOnDrop};
use crate::database::exception::DbError;
use crate::database::repo::combine;
use crate::internal::types::StoredSchema;

pub use crate::database::backup::backend::RestoreSource;
pub use glean_thrift::DBVersion;

/// 'canOpenVersion s mode version' (Storage.hs:52-58): whether this
/// storage can open the given binary-representation version in the given
/// mode. Create uses the writable set.
pub fn can_open_version<S: Storage>(storage: &S, mode: &Mode, version: DBVersion) -> bool {
    let versions = match mode {
        Mode::ReadOnly => storage.readable_versions(),
        Mode::ReadWrite | Mode::Create { .. } => storage.writable_versions(),
    };
    versions.contains(&version)
}

/// 'currentVersion = maximum . writableVersions' (Storage.hs:61-62).
///
/// Partial like the source: an empty writable list is the Haskell
/// 'maximum []' panic; the message is kept. The source's callers rely on
/// laziness — 'fromMaybe (currentVersion storage) configured' (Create.hs:
/// 124-127) never forces the default when a version is configured — so a
/// Rust caller must use 'unwrap_or_else(|_| current_version(s))' (or an
/// equivalent thunk), not an eager 'unwrap_or(current_version(s))'.
pub fn current_version<S: Storage>(storage: &S) -> DBVersion {
    storage
        .writable_versions()
        .into_iter()
        .max()
        .unwrap_or_else(|| panic!("Prelude.maximum: empty list"))
}

/// 'withMaterializedFile' (Storage.hs:64-77): pass a real file path to the
/// action. A SourceFile is passed through; a SourceStream is copied to a
/// unique file in the scratch directory, which is removed once the
/// continuation returns.
///
/// Error precedence mirrors 'finally': cleanup runs on every path and a
/// cleanup error replaces the body's result. RemoveOnDrop covers the
/// panic path (Drop cannot report, so a cleanup error there is swallowed,
/// exactly as backend.rs documents).
pub fn with_materialized_file<T, E, F>(
    scratch: &str,
    source: RestoreSource,
    action: F,
) -> Result<T, E>
where
    E: From<io::Error>,
    F: FnOnce(&str) -> Result<T, E>,
{
    match source {
        RestoreSource::File(path) => action(&path),
        RestoreSource::Stream(mut handle) => {
            let tmp_file = combine(scratch, &format!("stream-fallback-{}", next_unique()));
            let mut guard = RemoveOnDrop::new(&tmp_file);

            let result: Result<T, E> = match h_copy_to_file(&mut handle, &tmp_file) {
                Ok(()) => action(&tmp_file),
                Err(e) => Err(E::from(e)),
            };

            let cleanup = remove_path_forcibly(&tmp_file);
            guard.disarm();
            match (result, cleanup) {
                (_, Err(e)) => Err(E::from(e)),
                (r, Ok(())) => r,
            }
        }
    }
}

/// 'hCopyToFile' (Storage.hs:79-98): copy a handle to a file in fixed-size
/// chunks. Strict and eager: never lazy IO, never a whole-handle read.
fn h_copy_to_file(handle: &mut std::fs::File, path: &str) -> io::Result<()> {
    const CHUNK_SIZE: usize = 1024 * 1024;
    let mut out = std::fs::File::create(path)?;
    let mut chunk = vec![0u8; CHUNK_SIZE];
    loop {
        // GHC's hGetSome path retries EINTR; std's Read does not, so retry
        // Interrupted explicitly.
        let n = loop {
            match handle.read(&mut chunk) {
                Ok(n) => break n,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        };
        if n == 0 {
            break;
        }
        out.write_all(&chunk[..n])?;
    }
    Ok(())
}

/// 'data CreateSchema' (Storage.hs:101-105) — which schema a new DB gets.
#[derive(Debug)]
pub enum CreateSchema {
    UseDefaultSchema,
    UseSpecificSchema(String),
    UseThisSchema(StoredSchema),
}

/// The source derives Show (:105); RULE §2 maps a derived Show on a sum
/// type to constructor-shaped Display text. The payloads are rendered with
/// their Debug/Display images (String's Debug is quoted like Haskell's
/// Show; StoredSchema's generated Debug is the closest available image of
/// its Show). No consumer displays this type today.
impl fmt::Display for CreateSchema {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CreateSchema::UseDefaultSchema => f.write_str("UseDefaultSchema"),
            CreateSchema::UseSpecificSchema(id) => write!(f, "UseSpecificSchema({:?})", id),
            CreateSchema::UseThisSchema(schema) => write!(f, "UseThisSchema({:?})", schema),
        }
    }
}

/// 'data Mode' (Storage.hs:107-115) — how a DB is opened.
pub enum Mode {
    ReadOnly,
    ReadWrite,
    Create {
        /// Starting fact id.
        first_id: Fid,
        /// Base DB ownership, if this DB is stacked on one.
        ownership: Option<Ownership>,
        schema: CreateSchema,
        /// Inherited GUID from the base DB.
        guid: Option<String>,
    },
}

/// 'type AxiomOwnership = HashMap ByteString (VS.Vector Fid)' (Storage.hs:
/// 117-119) — raw ownership data for axiomatic (non-derived) facts.
pub type AxiomOwnership = HashMap<Vec<u8>, Vec<Fid>>;

/// 'data WriteLock w = WriteLock w' (Storage.hs:121-122) — the token
/// proving the caller holds the DB write lock. Every live instantiation
/// uses (), but the parameter is kept.
pub struct WriteLock<W>(pub W);

/// 'type BatchLocation = Text' (Storage.hs:124-125). Kept private like the
/// source (it is not in the export list): a type alias is transparent, so
/// this declaration and every impl's `&String` are the same type.
type BatchLocation = String;

/// The exported name of 'data family Database s' (Storage.hs:127).
pub type Database<S> = <S as Storage>::Database;

/// 'class DatabaseOps (Database s) => Storage s' (Storage.hs:130-180) —
/// an abstract fact-database storage engine.
pub trait Storage {
    /// The engine's open-database handle.
    type Database: DatabaseOps;

    /// A short, user-readable description of the storage.
    fn describe(&self) -> String;

    /// Binary representation versions we can read.
    fn readable_versions(&self) -> Vec<DBVersion>;

    /// Binary representation versions we can write.
    fn writable_versions(&self) -> Vec<DBVersion>;

    /// Open a database.
    fn open(
        &self,
        repo: &thrift::Repo,
        mode: Mode,
        version: DBVersion,
    ) -> Result<Self::Database, DbError>;

    /// Delete a database if it exists.
    fn delete(&self, repo: &thrift::Repo) -> Result<(), DbError>;

    /// Unconditionally remove a database or anything stored where it would
    /// exist.
    fn safe_remove_forcibly(&self, repo: &thrift::Repo) -> Result<(), DbError>;

    /// Total capacity of the storage medium, if known.
    fn get_total_capacity(&self) -> Result<Option<i64>, DbError>;

    /// Used capacity of the storage medium, if known.
    fn get_used_capacity(&self) -> Result<Option<i64>, DbError>;

    /// Free capacity of the storage medium.
    fn get_free_capacity(&self) -> Result<i64, DbError>;

    /// Run the action with a scratch directory (not guaranteed to persist
    /// beyond the call or to be empty).
    fn with_scratch_root<T, F>(&self, action: F) -> Result<T, DbError>
    where
        F: FnOnce(&str) -> Result<T, DbError>;

    /// Restore a database from a file or a stream. The scratch directory
    /// is empty and is deleted afterwards.
    fn restore(
        &self,
        config: &thrift::Config,
        repo: &thrift::Repo,
        scratch: &str,
        source: RestoreSource,
    ) -> Result<(), DbError>;
}

/// 'class CanLookup db => DatabaseOps db' (Storage.hs:182-294) — an open
/// fact database.
pub trait DatabaseOps: CanLookup {
    /// Close the database.
    fn close(&self) -> Result<(), DbError>;

    /// Obtain the PredicateStats for each predicate.
    fn predicate_stats(&self) -> Result<Vec<(Pid, thrift::PredicateStats)>, DbError>;

    /// Store an arbitrary binary key/value pair (used sparingly; no
    /// performance guarantees).
    fn store(&self, key: &[u8], value: &[u8]) -> Result<(), DbError>;

    /// Retrieve a value previously stored with store.
    fn retrieve(&self, key: &[u8]) -> Result<Option<Vec<u8>>, DbError>;

    /// Store a batch descriptor (materialised before 'glean complete').
    fn add_batch_descriptor(&self, descriptor: &thrift::BatchDescriptor) -> Result<(), DbError>;

    /// Mark a batch descriptor as written.
    fn mark_batch_descriptor_as_written(&self, location: &BatchLocation) -> Result<(), DbError>;

    /// Check whether a batch descriptor is stored.
    fn is_batch_descriptor_stored(&self, location: &BatchLocation) -> Result<bool, DbError>;

    /// All unprocessed (not marked written) batch descriptors.
    fn get_unprocessed_batch_descriptors(&self) -> Result<Vec<thrift::BatchDescriptor>, DbError>;

    /// Commit a set of facts with the right ids (they are NOT renamed).
    fn commit(&self, facts: &FactSet) -> Result<(), DbError>;

    /// Add ownership data about a set of committed facts.
    fn add_ownership<W>(
        &self,
        lock: &WriteLock<W>,
        ownership: AxiomOwnership,
    ) -> Result<(), DbError>;

    /// Register ACL group names as units, allocating a UnitId for each
    /// unseen name. Returns the smallest UnitId among the names (the
    /// boundary between regular ownership units and ACL group units).
    fn register_acl_units<W>(
        &self,
        lock: &WriteLock<W>,
        names: &[Vec<u8>],
    ) -> Result<UnitId, DbError>;

    /// Optimise for reading (typically before backup); compact selects
    /// whether to compact.
    fn optimize(&self, compact: bool) -> Result<(), DbError>;

    /// Flush in-memory write buffers without compacting.
    fn flush(&self) -> Result<(), DbError>;

    /// Compute ownership from the unit iterator, with an optional base
    /// lookup for a stacked DB.
    fn compute_ownership(
        &self,
        base: Option<&Lookup>,
        inventory: &Inventory,
    ) -> Result<ComputedOwnership, DbError>;

    /// Store computed ownership.
    fn store_ownership<W>(
        &self,
        lock: &WriteLock<W>,
        ownership: &ComputedOwnership,
    ) -> Result<(), DbError>;

    /// The Ownership interface for this DB (Nothing if the backend does
    /// not support ownership).
    fn get_ownership(&self) -> Result<Option<Ownership>, DbError>;

    fn get_unit_id(&self, unit: &[u8]) -> Result<Option<UnitId>, DbError>;

    fn get_unit(&self, id: UnitId) -> Result<Option<Vec<u8>>, DbError>;

    fn get_units_by_prefix(&self, prefix: &[u8]) -> Result<Vec<(Vec<u8>, UnitId)>, DbError>;

    /// Called once per batch.
    fn add_define_ownership<W>(
        &self,
        lock: &WriteLock<W>,
        define: &DefineOwnership,
    ) -> Result<(), DbError>;

    /// Called once per derived predicate at the end of its derivation.
    fn compute_derived_ownership<W>(
        &self,
        lock: &WriteLock<W>,
        ownership: &Ownership,
        base: Option<&Lookup>,
        pid: Pid,
    ) -> Result<ComputedOwnership, DbError>;

    /// Cache ownership data for faster getOwner() operations.
    fn cache_ownership(&self) -> Result<(), DbError>;

    fn prepare_fact_owner_cache(&self) -> Result<(), DbError>;

    /// Backup a database. The scratch directory is empty and is deleted
    /// afterwards; the action receives the serialised database path and
    /// its Data.
    fn backup<A, F>(&self, config: &thrift::Config, scratch: &str, action: F) -> Result<A, DbError>
    where
        F: FnOnce(&str, Data) -> Result<A, DbError>;
}

// TODO(port): the 'instance CanLookup (Some DatabaseOps)' (Storage.hs:
// 296-298) and 'instance DatabaseOps (Some DatabaseOps)' (Storage.hs:
// 300-324) forwarding impls are owed by rust/glean-db/src/database/types.rs
// (manifest line 104), which owns the closed enum Storage {Rocks, Lmdb,
// Memory} per inventory row 137. They cannot be written here: the engine
// types are unshipped forward references, and DatabaseOps is not
// object-safe (CanLookup::with_lookup is generic), so a Box<dyn> is not an
// option either.

// glean/db/Glean/Database/Storage/DB.hs -> src/database/storage/db.rs
// (manifest line 67). This line binds the submodule of this file.
pub mod db;

// Marker breakdown (closed set — 1 TODO(port), 0 BUG(port), 0 PERF(port),
// 0 INVENTORY-BUG-CANDIDATE):
//   * 1 TODO(port): the Some DatabaseOps forwarding instances (above),
//     owned by database/types.rs per inventory row 137.
//   * 0 BUG(port): no source defect is reproduced. 'maximum []' keeps its
//     panic message; the normal-return finally precedence is exact (the
//     panic-path cleanup-error caveat is divergence 7, not a source defect).
//   * 0 PERF(port): the chunked copy is the source's own design (the
//     lazy-IO comment at Storage.hs:79-89); no copy is added.
//   * 0 INVENTORY-BUG-CANDIDATE: rows 19/125/136/137/142/444 are followed. Row 137
//     is followed by deferral to its named owner, not diverged from.
// SAFETY count: 0 — this file contains no unsafe construct (no externs,
// no raw pointers; the engines' FFI lives in their own units).
// PORT STATUS: confidence=high todos=1
