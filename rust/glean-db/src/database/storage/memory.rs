// Ported from: glean/db/Glean/Database/Storage/Memory.hs (134 lines)
// Manifest unit: rust/glean-db/src/database/storage/memory.rs
// (migration/manifest.tsv line 71). RULE §4: one source file -> one target
// file; the manifest path is authoritative.
//
// This is the in-memory backend: a map of open databases, each holding a
// writable FactSet plus an arbitrary byte key/value map. The source's own
// module doc calls it a skeleton "only usable for tests which don't produce
// or query facts" (:9-11) — the port keeps that shape rather than filling
// in the TODOs.
//
// ---------------------------------------------------------------------------
// INVENTORY (RULE §5; migration/inventory*.tsv)
// ---------------------------------------------------------------------------
//
//   * inventory.tsv:30 and inventory.stm.tsv:24 — the STM_GROUP row. Its
//     Rust sketch is
//       struct MemoryStorage { Mutex<HashMap<Repo, Arc<MemoryDb>>> }
//       MemoryDb { Mutex<HashMap<Vec<u8>,Vec<u8>>> }
//     with "create txn (55-60: read map + insert new DB) = one outer
//     critical section; put/delete per-db (98-100) = inner lock only".
//     Followed with two divergences, both flagged below: the type keeps the
//     SOURCE's name `Memory` (RULE §4 naming, export list :14-17) instead
//     of the sketch's `MemoryStorage`, and MemoryDb also holds
//     `facts: Mutex<FactSet>` — the row's sketch has no home for
//     `dbFacts :: FactSet` (:41). The module path already disambiguates
//     `Memory` from the catalog's Memory.
//
//   * inventory.sites.tsv:301-306 — the six STM sites this unit replaces
//     with critical sections: :34 (the map), :42 (the byte map), :55
//     (create's read+insert), :69 (delete), :98 (store), :100 (retrieve).
//   * The same row notes "key/value byte-equality and iteration order of
//     list matter; note HashMap order is unspecified in source". Nothing
//     here returns the map in order, so only byte-equality is observable,
//     and HashMap<Vec<u8>, Vec<u8>> gives it.
//
// INVENTORY-BUG-CANDIDATE row 30: the target_translation sketch names
// MemoryStorage and omits dbFacts (Memory.hs:41); the port keeps the
// source's Memory per RULE §4 and adds facts: Mutex<FactSet> to MemoryDb.
// The row is incomplete, not wrong.
//
// ---------------------------------------------------------------------------
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, checked not assumed)
// ---------------------------------------------------------------------------
//
//   * OUTBOUND (5), all shipped:
//       edges.tsv:430 Database/Exception (53) — DbError, db_error;
//       :431 Database/Storage (58) — the traits, Mode, WriteLock;
//       :432 RTS/Foreign/FactSet (32) — FactSet;
//       :433 RTS/Foreign/Lookup (19) — CanLookup;
//       :434 Repo/Text (51) — repoToText.
//     Rust-only import: Data, from row 56 (backup/backend.rs). Storage.hs
//     does not export it; the trait signature names it, so this file imports
//     it from its owner directly (storage.rs imports it privately and only
//     re-exports RestoreSource).
//   * INBOUND (1): edges.tsv:224 Database/Config.hs (manifest line 85, NOT
//     shipped) builds this backend through newStorage.
//   * External, no manifest unit and no edge: System.IO.Temp
//     (withSystemTempDirectory, :82) and Util.STM (:24). Neither module is
//     in the OSS tree; see DIVERGENCES 1 and 4.
//
// ---------------------------------------------------------------------------
// ASSUMED IMPORTS (RULE §4 B4)
// ---------------------------------------------------------------------------
//
//   * glean_thrift::{Repo, BatchDescriptor, PredicateStats, Config}. The
//     generated Repo carries Eq + Hash (repo/text.rs:69-70's derive;
//     row 65's Mutex<HashMap<Repo, Meta>> is the shipped precedent for
//     using it as a map key). PredicateStats is glean.thrift:344-347
//     {1: i64 count; 2: i64 size} — the type FactSet::predicate_stats
//     already returns, so this file never marshals. BatchDescriptor
//     (glean.thrift:915-918) and Config (server_config.thrift:192) are
//     passed through opaquely: this backend stores no descriptors and reads
//     no config fields.
//   * glean_core::rts::foreign::fact_set::FactSet — new, append,
//     predicate_stats, with_lookup (rows 32/19, both shipped).
//   * glean_core::repo::text::repo_to_text (row 51) — the source's
//     `repoToText` (:88).
//   * System.IO.Temp.withSystemTempDirectory: row 70's
//     with_temp_directory(parent, prefix) seam in storage.rs, with
//     std::env::temp_dir() as the parent (DIVERGENCES 4).
//
// ---------------------------------------------------------------------------
// TRANSLATION NOTES
// ---------------------------------------------------------------------------
//
//   * :34 `newtype Memory = Memory (TVar (HashMap Repo (Database Memory)))`
//     -> pub struct Memory { dbs: Mutex<HashMap<Repo, Arc<MemoryDb>>> }.
//     Arc because the value stored in the map and the value returned by
//     open must share one database (the source shares the TVars).
//   * :36-37 newStorage = Memory <$> newTVarIO HashMap.empty ->
//     pub fn new_storage() -> Memory (constructing a Mutex cannot fail).
//   * :39-43 `data instance Database Memory = Database { dbRepo, dbFacts,
//     dbData }` -> pub struct Database { repo, db: Arc<MemoryDb>,
//     lookup_name }, with MemoryDb { facts: Mutex<FactSet>, data:
//     Mutex<HashMap<Vec<u8>, Vec<u8>>> }. lookup_name is precomputed
//     because CanLookup::lookup_name returns &str.
//   * :46-85 Storage: describe "memory:"; both version lists [0]; open
//     (Create makes the FactSet BEFORE taking the lock, then one critical
//     section for lookup+insert, exactly :53-62); delete and
//     safeRemoveForcibly are the same map removal; the three capacities are
//     None/None/maxBound; withScratchRoot wraps withSystemTempDirectory;
//     restore is dbError "unimplemented 'restore'".
//   * :87-89 CanLookup: lookupName = "memory:" <> repoToText dbRepo;
//     withLookup forwards to the FactSet.
//   * :91-134 DatabaseOps: close/optimize/flush/addOwnership/
//     storeOwnership/cacheOwnership/prepareFactOwnerCache are no-ops;
//     predicateStats and commit go to the FactSet; store/retrieve to the
//     byte map; the batch-descriptor methods are the source's constant
//     no-ops (False / []); getUnitsByPrefix is []; getOwnership is Nothing;
//     registerACLUnits and the five `return (error ...)` methods keep their
//     messages; backup is dbError "unimplemented 'backup'".
//
// ---------------------------------------------------------------------------
// DIVERGENCES
// ---------------------------------------------------------------------------
//
//   1. STM -> Mutex. Util.STM is not in the manifest (grep 'STM'
//      manifest.tsv: no rows) and no Rust STM module exists, so each TVar
//      becomes a std::sync::Mutex and each `atomically` block becomes one
//      critical section. The observable contract is preserved for the
//      operations here (create's read+insert is one lock; store/retrieve/
//      delete are single operations), but STM's composability is not: a
//      future multi-TVar transaction would need a design, not a lock.
//      Poisoned locks are recovered with into_inner (row 65's helper), so a
//      panic in one caller does not wedge the backend. For the byte map that
//      is sound (the map's invariants hold after any panic). For the facts
//      lock it is an ASSUMPTION: a panic inside FactSet::append leaves the
//      C++ object in whatever state factset.cpp left it, and this file
//      neither inspects nor repairs that — the same posture the source takes
//      (a throw out of append leaves the ForeignPtr usable-looking).
//   2. FactSet is deliberately !Send + !Sync (glean-core fact_set.rs's
//      struct doc: the C++ contract forbids interleaving modification with
//      reads). That propagates all the way out: Mutex<T> is Send/Sync only
//      where T: Send, so Mutex<FactSet> is neither, MemoryDb is neither,
//      Arc<MemoryDb> is !Send, and both Memory and Database end up !Send AND
//      !Sync — while the Haskell TVar backend is freely shared. The Mutex
//      therefore does NOT make this backend thread-safe; it only gives
//      append its &mut. No unsafe impl is added (a handle that cannot be
//      shared cannot race). The consequence for the shared Env is the open
//      item recorded above the Memory struct.
//   3. `return (error "...")` (:120-129) is a LAZY exception in Haskell: the
//      thunk is returned and blows up when forced. Rust has no laziness, so
//      these methods panic when called. The messages are byte-identical
//      ("unimplemented computeOwnership", "unimplemented getUnitId",
//      "unimplemented getUnit", "unimplemented addDefineOwnership",
//      "unimplemented computeDerivedOwnership"); `registerACLUnits` uses
//      plain `error` in the source and panics the same way. The Haskell
//      ErrorCall is a SomeException a caller could catch and turn into a
//      DbError; a Rust panic unwinds past the Result channel, so it is not
//      one — RULE §2's panic rule governs, and no in-tree consumer forces
//      these methods on the Memory backend.
//   4. withSystemTempDirectory "glean" (:82) has no parent argument. The
//      row-70 seam takes (parent, prefix), so the parent is
//      std::env::temp_dir() — the same getTemporaryDirectory the Haskell
//      uses (TMPDIR, else /tmp). The generated name is "glean<unique>"
//      rather than "gleanXXXXXX" (row 70's DIVERGENCES 3). A non-UTF-8
//      TMPDIR becomes U+FFFD through to_string_lossy, where the Haskell
//      String round-trips it — inherited from row 70's &str seam.
//   5. withLookup holds the facts lock for the duration of the callback.
//      The source hands the raw FactSet pointer to f with no lock, so a
//      caller that committed from inside the callback would interleave a
//      write with the lookup — which the C++ contract (factset.h:282-283)
//      forbids. Because nothing here is Send, a second thread cannot reach
//      this handle at all, so the lock buys no cross-thread protection: its
//      only effect is that same-thread re-entry from inside the callback
//      (commit, predicate_stats, or a nested with_lookup) self-deadlocks.
//      No in-tree caller does it (Batch.hs:224 commits after the
//      withBase/withLookupCache scope opened at :191-192 and closed at :198).
//   6. `maxBound :: Int` (:80) -> i64::MAX. Exact on a 64-bit target; on a
//      32-bit one Haskell's Int would be 2^31-1 and the trait's i64 would
//      not (the migration targets 64-bit only).
//
// ---------------------------------------------------------------------------
// SIBLING-UNIT CONTRACTS
// ---------------------------------------------------------------------------
//
//   * row 58 (storage.rs): the Storage/DatabaseOps/CanLookup traits, Mode,
//     WriteLock, AxiomOwnership, BatchLocation, Data, RestoreSource, and the
//     with_temp_directory seam this unit reuses.
//   * row 65 (catalog/local/memory.rs): the Mutex<HashMap<Repo, …>> shape
//     and the PoisonError::into_inner helper convention.
//   * rows 32/19: FactSet and CanLookup; this file never declares an extern
//     and never owns a raw pointer.
//   * row 104 (database/types.rs) owns the closed Storage enum that will
//     hold this type; row 72 is the RocksDB backend, unaffected by it.
//
// MARKERS (closed set — 1 TODO(port), 1 INVENTORY-BUG-CANDIDATE,
// 0 BUG(port), 0 PERF(port)):
//   * 1 TODO(port): the !Send representation choice (above the Memory
//     struct) — the shared-Env consumer must decide.
//   * 1 INVENTORY-BUG-CANDIDATE: row 30's incomplete sketch (line 37).
//   * 0 BUG(port) / 0 PERF(port): the six panicking stubs (:112-113,
//     :120-129) and the two Err-returning stubs (:85, :134) are the
//     SOURCE's own, not migration debt.
//

use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use glean_core::ffi::FfiError;
use glean_core::repo::text::repo_to_text;
use glean_core::rts::foreign::fact_set::FactSet;
use glean_core::rts::foreign::inventory::Inventory;
use glean_core::rts::foreign::lookup::{CanLookup, Lookup};
use glean_core::rts::foreign::ownership::{ComputedOwnership, DefineOwnership, Ownership, UnitId};
use glean_core::rts::types::Pid;
use glean_thrift as thrift;

use crate::database::backup::backend::{Data, RestoreSource};
use crate::database::exception::{db_error, DbError};
use crate::database::storage::{
    with_temp_directory, AxiomOwnership, BatchLocation, DBVersion, DatabaseOps, Mode, Storage,
    WriteLock,
};

// TODO(port): Memory and Database are !Send as well as !Sync — FactSet is
// !Send (fact_set.rs:408-424) and Mutex/Arc inherit it (DIVERGENCES 2) —
// while the Haskell TVar backend is freely shared. Config.hs:159-161 puts
// this backend in the shared Env (Types.hs:251 envStorage) and the backuper
// thread (Backup.hs:512-519, storage used at :255/:290/:377) reads it, so
// rows 104 (the closed Storage enum) and 85 (Config) must
// choose: a thread-local Memory, or revisiting FactSet's !Send posture. If
// an unsafe impl is ever chosen, RULE §3 confines it to glean-core's FFI
// wrapper, not this file. (Rows 61/177 own a different Send+Sync question:
// the Backends table payloads, backend.rs:134-138.)

/// 'newtype Memory = Memory (TVar (HashMap Repo (Database Memory)))' (:34).
///
/// The fields are private because no consumer reads them: the source
/// exports Memory(..) (:14) and Database(..) (:16, constructor plus
/// dbRepo/dbFacts/dbData), but nothing outside Memory.hs touches either —
/// the only dbRepo elsewhere is Types.hs:148's different DB record. The
/// export surface is therefore narrower than the source's, with no loss.
pub struct Memory {
    dbs: Mutex<HashMap<thrift::Repo, Arc<MemoryDb>>>,
}

/// The mutable half of 'Database Memory' (:39-43): the FactSet and the byte
/// map. Shared by the copy in Memory::dbs and every handle open returns.
struct MemoryDb {
    /// 'dbFacts :: FactSet' (:41).
    facts: Mutex<FactSet>,
    /// 'dbData :: TVar (HashMap ByteString ByteString)' (:42).
    data: Mutex<HashMap<Vec<u8>, Vec<u8>>>,
}

/// 'data instance Database Memory = Database { dbRepo, dbFacts, dbData }'
/// (:39-43) — the associated type of Storage.
pub struct Database {
    /// 'dbRepo'.
    repo: thrift::Repo,
    /// 'dbFacts' + 'dbData'.
    db: Arc<MemoryDb>,
    /// 'lookupName Database{..} = "memory:" <> repoToText dbRepo' (:88),
    /// precomputed because the trait returns &str.
    lookup_name: String,
}

/// 'newStorage :: IO Memory' (:36-37): an empty map.
pub fn new_storage() -> Memory {
    Memory {
        dbs: Mutex::new(HashMap::new()),
    }
}

/// The row-65 helper convention: a panic in one caller must not wedge the
/// backend, so a poisoned lock is recovered rather than propagated.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

// ---------------------------------------------------------------------------
// Storage (Memory.hs:46-85)
// ---------------------------------------------------------------------------

impl Storage for Memory {
    type Database = Database;

    /// 'describe = const "memory:"' (:48).
    fn describe(&self) -> String {
        "memory:".to_string()
    }

    /// 'readableVersions _ = [DBVersion 0]' (:50).
    fn readable_versions(&self) -> Vec<DBVersion> {
        vec![0]
    }

    /// 'writableVersions _ = [DBVersion 0]' (:51).
    fn writable_versions(&self) -> Vec<DBVersion> {
        vec![0]
    }

    /// 'open (Memory v) repo (Create start _unit _ _) _' (:53-62) and the
    /// lookup-only case (:63-67).
    fn open(
        &self,
        repo: &thrift::Repo,
        mode: Mode,
        _version: DBVersion,
    ) -> Result<Database, DbError> {
        match mode {
            Mode::Create { first_id, .. } => {
                // The source creates the FactSet BEFORE entering atomically
                // (:54), so an "already exists" error still allocates and
                // then frees one. Reproduced.
                let facts = FactSet::new(first_id)?;
                // One critical section for read-then-insert (:55-61).
                let mut dbs = lock(&self.dbs);
                if dbs.contains_key(repo) {
                    return db_error(repo, "database already exists");
                }
                let db = Arc::new(MemoryDb {
                    facts: Mutex::new(facts),
                    data: Mutex::new(HashMap::new()),
                });
                dbs.insert(repo.clone(), db.clone());
                Ok(Database {
                    repo: repo.clone(),
                    lookup_name: format!("memory:{}", repo_to_text(repo)),
                    db,
                })
            }
            // 'open (Memory v) repo _ _ = ... readTVarIO v' (:63-67).
            _ => {
                let dbs = lock(&self.dbs);
                match dbs.get(repo) {
                    Some(db) => Ok(Database {
                        repo: repo.clone(),
                        lookup_name: format!("memory:{}", repo_to_text(repo)),
                        db: db.clone(),
                    }),
                    None => db_error(repo, "database doesn't exist"),
                }
            }
        }
    }

    /// 'delete (Memory v) = atomically . modifyTVar' v . HashMap.delete'
    /// (:69). The value is dropped, so a handle held elsewhere keeps the
    /// database alive (the source's TVars behave the same way).
    fn delete(&self, repo: &thrift::Repo) -> Result<(), DbError> {
        // Bind the removed handle so it drops AFTER the map guard (a bare
        // statement temporary would drop before the guard, at the semicolon):
        // the last reference runs glean_factset_free outside the lock.
        let _removed = lock(&self.dbs).remove(repo);
        Ok(())
    }

    /// 'safeRemoveForcibly = delete' (:71).
    fn safe_remove_forcibly(&self, repo: &thrift::Repo) -> Result<(), DbError> {
        self.delete(repo)
    }

    /// 'getTotalCapacity _ = return Nothing' (:73-74, "TODO" in the source).
    fn get_total_capacity(&self) -> Result<Option<i64>, DbError> {
        Ok(None)
    }

    /// 'getUsedCapacity _ = return Nothing' (:76-77, "TODO").
    fn get_used_capacity(&self) -> Result<Option<i64>, DbError> {
        Ok(None)
    }

    /// 'getFreeCapacity _ = return maxBound' (:79-80, "TODO").
    fn get_free_capacity(&self) -> Result<i64, DbError> {
        Ok(i64::MAX)
    }

    /// 'withScratchRoot _ f = withSystemTempDirectory "glean" f' (:82).
    fn with_scratch_root<T, F>(&self, action: F) -> Result<T, DbError>
    where
        F: FnOnce(&str) -> Result<T, DbError>,
    {
        let parent = std::env::temp_dir();
        with_temp_directory(&parent.to_string_lossy(), "glean", action)
    }

    /// 'restore _ _ repo _ _ = dbError repo "unimplemented 'restore'"'
    /// (:84-85).
    fn restore(
        &self,
        _config: &thrift::Config,
        repo: &thrift::Repo,
        _scratch: &str,
        _source: RestoreSource,
    ) -> Result<(), DbError> {
        db_error(repo, "unimplemented 'restore'")
    }
}

// ---------------------------------------------------------------------------
// CanLookup (Memory.hs:87-89)
// ---------------------------------------------------------------------------

impl CanLookup for Database {
    /// 'withLookup = withLookup . dbFacts' (:89).
    fn with_lookup<T, E: From<FfiError>>(
        &self,
        f: impl FnOnce(*mut c_void) -> Result<T, E>,
    ) -> Result<T, E> {
        lock(&self.db.facts).with_lookup(f)
    }

    /// 'lookupName Database{..} = "memory:" <> repoToText dbRepo' (:88).
    fn lookup_name(&self) -> &str {
        &self.lookup_name
    }
}

// ---------------------------------------------------------------------------
// DatabaseOps (Memory.hs:91-134)
// ---------------------------------------------------------------------------

/// The source's 'error "unimplemented …"' / 'return (error …)' stubs. Rust
/// has no lazy exception, so these fire at the call (DIVERGENCES 3); the
/// message text is the source's.
fn unimplemented<T>(msg: &str) -> T {
    panic!("{}", msg)
}

impl DatabaseOps for Database {
    /// 'close _ = return ()' (:92-93, "TODO").
    fn close(&self) -> Result<(), DbError> {
        Ok(())
    }

    /// 'predicateStats = FactSet.predicateStats . dbFacts' (:95).
    fn predicate_stats(&self) -> Result<Vec<(Pid, thrift::PredicateStats)>, DbError> {
        lock(&self.db.facts)
            .predicate_stats()
            .map_err(DbError::from)
    }

    /// 'store db key value = atomically $ modifyTVar' (dbData db) $
    /// HashMap.insert key value' (:97-98).
    fn store(&self, key: &[u8], value: &[u8]) -> Result<(), DbError> {
        lock(&self.db.data).insert(key.to_vec(), value.to_vec());
        Ok(())
    }

    /// 'retrieve db key = atomically $ HashMap.lookup key <$> readTVar
    /// (dbData db)' (:99-100).
    fn retrieve(&self, key: &[u8]) -> Result<Option<Vec<u8>>, DbError> {
        Ok(lock(&self.db.data).get(key).cloned())
    }

    /// 'addBatchDescriptor _ _ = return ()' (:102).
    fn add_batch_descriptor(&self, _descriptor: &thrift::BatchDescriptor) -> Result<(), DbError> {
        Ok(())
    }

    /// 'markBatchDescriptorAsWritten _ _ = return ()' (:103).
    fn mark_batch_descriptor_as_written(&self, _location: &BatchLocation) -> Result<(), DbError> {
        Ok(())
    }

    /// 'isBatchDescriptorStored _ _ = return False' (:104).
    fn is_batch_descriptor_stored(&self, _location: &BatchLocation) -> Result<bool, DbError> {
        Ok(false)
    }

    /// 'getUnprocessedBatchDescriptors _ = return []' (:105).
    fn get_unprocessed_batch_descriptors(&self) -> Result<Vec<thrift::BatchDescriptor>, DbError> {
        Ok(Vec::new())
    }

    /// 'commit db facts = FactSet.append (dbFacts db) facts' (:107).
    fn commit(&self, facts: &FactSet) -> Result<(), DbError> {
        lock(&self.db.facts).append(facts).map_err(DbError::from)
    }

    /// 'addOwnership _ _ _ = return ()' (:109-110, "TODO: ownership").
    fn add_ownership<W>(
        &self,
        _lock: &WriteLock<W>,
        _ownership: AxiomOwnership,
    ) -> Result<(), DbError> {
        Ok(())
    }

    /// 'registerACLUnits _ _ _ = error "unimplemented registerACLUnits"'
    /// (:112-113).
    fn register_acl_units<W>(
        &self,
        _lock: &WriteLock<W>,
        _names: &[Vec<u8>],
    ) -> Result<UnitId, DbError> {
        unimplemented("unimplemented registerACLUnits")
    }

    /// 'optimize _ _ = return ()' (:115).
    fn optimize(&self, _compact: bool) -> Result<(), DbError> {
        Ok(())
    }

    /// 'flush _ = return ()' (:117).
    fn flush(&self) -> Result<(), DbError> {
        Ok(())
    }

    /// 'computeOwnership _ _ _ = return (error "unimplemented
    /// computeOwnership")' (:120).
    fn compute_ownership(
        &self,
        _base: Option<&Lookup>,
        _inventory: &Inventory,
    ) -> Result<ComputedOwnership, DbError> {
        unimplemented("unimplemented computeOwnership")
    }

    /// 'storeOwnership _ _ _ = return ()  -- can't fail, otherwise we fail
    /// tests' (:124).
    fn store_ownership<W>(
        &self,
        _lock: &WriteLock<W>,
        _ownership: &ComputedOwnership,
    ) -> Result<(), DbError> {
        Ok(())
    }

    /// 'getOwnership _ = return Nothing' (:125).
    fn get_ownership(&self) -> Result<Option<Ownership>, DbError> {
        Ok(None)
    }

    /// 'getUnitId _ _ = return (error "unimplemented getUnitId")' (:121).
    fn get_unit_id(&self, _unit: &[u8]) -> Result<Option<UnitId>, DbError> {
        unimplemented("unimplemented getUnitId")
    }

    /// 'getUnit _ _ = return (error "unimplemented getUnit")' (:122).
    fn get_unit(&self, _id: UnitId) -> Result<Option<Vec<u8>>, DbError> {
        unimplemented("unimplemented getUnit")
    }

    /// 'getUnitsByPrefix _ _ = return []' (:123).
    fn get_units_by_prefix(&self, _prefix: &[u8]) -> Result<Vec<(Vec<u8>, UnitId)>, DbError> {
        Ok(Vec::new())
    }

    /// 'addDefineOwnership _ _ = return (error "unimplemented
    /// addDefineOwnership")' (:126-127).
    fn add_define_ownership<W>(
        &self,
        _lock: &WriteLock<W>,
        _define: &DefineOwnership,
    ) -> Result<(), DbError> {
        unimplemented("unimplemented addDefineOwnership")
    }

    /// 'computeDerivedOwnership _ _ _ = return (error "unimplemented
    /// computeDerivedOwnership")' (:128-129).
    fn compute_derived_ownership<W>(
        &self,
        _lock: &WriteLock<W>,
        _ownership: &Ownership,
        _base: Option<&Lookup>,
        _pid: Pid,
    ) -> Result<ComputedOwnership, DbError> {
        unimplemented("unimplemented computeDerivedOwnership")
    }

    /// 'cacheOwnership _ = return ()' (:130).
    fn cache_ownership(&self) -> Result<(), DbError> {
        Ok(())
    }

    /// 'prepareFactOwnerCache _ = return ()' (:131).
    fn prepare_fact_owner_cache(&self) -> Result<(), DbError> {
        Ok(())
    }

    /// 'backup db _ _ _ = dbError (dbRepo db) "unimplemented 'backup'"'
    /// (:133-134, "TODO").
    fn backup<A, F>(
        &self,
        _config: &thrift::Config,
        _scratch: &str,
        _action: F,
    ) -> Result<A, DbError>
    where
        F: FnOnce(&str, Data) -> Result<A, DbError>,
    {
        db_error(&self.repo, "unimplemented 'backup'")
    }
}

// PORT STATUS: confidence=high todos=1
