// Ported from: glean/db/Glean/Database/Storage/DB.hs (449 lines).
// Manifest unit: rust/glean-db/src/database/storage/db.rs
// (migration/manifest.tsv line 67, crate glean-db). RULE §4: one source
// file -> one target file; the manifest path is authoritative.
//
// WHAT THIS FILE IS: the RocksDB storage engine's open-database handle.
// It owns the C++ Database* (through the shared ForeignPtr layer),
// implements CanLookup (the handle IS a Lookup: storage/ffi.cpp:99-101
// returns the Database* itself) and the whole DatabaseOps surface, and
// adds the container helper plus the checkpoint free function. The
// Haskell module is titled "Generic DB operations shared by the RocksDB
// and LMDB backends", but every extern it declares is glean_rocksdb_*;
// LMDB.hs (row 70) reuses this handle and its free symbol
// (inventory.ffi.tsv:24 records the shared-symbol contract).
//
// ---------------------------------------------------------------------------
// INVENTORY (RULE §5; migration/inventory*.tsv)
// ---------------------------------------------------------------------------
//
// GOVERNING ROW — migration/inventory.tsv:28 (GUARD_KIND, confirmed).
// Columns, with TABs shown as \t:
//
//   glean/db/Glean/Database/Storage/DB.hs \t storage FFI \t
//   28 ccall; NULL returns mixed failure kinds \t GUARD_KIND \t
//   cxx bridge [SUPERSEDED by §1/A2: raw extern-C — 2026-10-07]; each fn
//   classified: allocation-guard (alloc failures abort) vs
//   precondition-guard (missing db -> NotFound result) \t
//   DB.hs:28 foreign imports (largest FFI file); rocksdb/lmdb impls
//   behind same ABI; memory impl is pure Rust reference \t confirmed
//
// Followed:
//   * Raw extern-C, no cxx bridge: RULE §1/A2 (the row's own supersession
//     note). All 28 declarations are one unsafe extern "C" block.
//   * Each function is classified in the per-call SAFETY notes: the two
//     iterator acquisitions and the ownership getter carry the
//     allocation-guard/precondition-guard split the row names. The
//     "missing db" outcomes are VALUES in this ABI (getUnitId writes
//     UINT64_MAX, getUnit writes null/0; storage/ffi.cpp:214-245), which
//     the source maps to Nothing/None — reproduced verbatim, never
//     converted into an error.
//   * 28 foreign imports: all 28 Haskell ccall declarations are present.
//     The Rust extern block declares 29 functions — those 28 (26 fallible
//     const char* returns, 2 void mutators: container_free and
//     container_close) plus glean_rocksdb_database_free, whose Haskell
//     declaration lives in RocksDB.hs:145 (DIVERGENCES 1).
//
// The 28 per-import SITES rows are migration/inventory.sites.tsv:24-51
// (all 'ffi-import', all citing "FFI ownership/marshal (error-recovery
// rule)"); inventory.ffi.tsv:3 is a header comment naming this file among
// the ccall groups, not a row. No inventory divergence flag applies.
//
// ---------------------------------------------------------------------------
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, checked not assumed)
// ---------------------------------------------------------------------------
//
//   * OUTBOUND (9), all shipped:
//       :411 Backup/Backend.hs      row 56, database/backup/backend.rs
//       :412 Storage.hs             row 58, database/storage.rs
//       :413 hs/Glean/FFI.hs        row  9, glean-core/src/ffi.rs
//       :414 RTS/Foreign/FactSet.hs row 32, rts/foreign/fact_set.rs
//       :415 RTS/Foreign/Lookup.hs  row 19, rts/foreign/lookup.rs
//       :416 RTS/Foreign/Ownership  row 57, rts/foreign/ownership.rs
//       :417 RTS/Foreign/Stats.hs   row 31, rts/foreign/stats.rs
//       :418 RTS/Types.hs           row 11, rts/types.rs
//       :419 hs/Glean/Repo/Text.hs  row 51, glean-core/src/repo/text.rs
//   * INBOUND (2), both future:
//       :423 Storage/LMDB.hs        row 70
//       :438 Storage/RocksDB.hs     row 72
//     Both consume DB, DB::from_raw, Container/with_container and
//     checkpoint; neither may re-declare the finalizer (DIVERGENCES 1).
//
// ---------------------------------------------------------------------------
// ASSUMED IMPORTS (RULE §4 B4)
// ---------------------------------------------------------------------------
//
//   * glean_thrift::{Repo, Config, BatchDescriptor, BatchFormat,
//     PredicateStats}. Pinned shapes:
//       Repo            glean.thrift:106-109, Rust image
//                       backend/types.rs:171-175.
//       Config          server_config.thrift:192 (storage.rs:132-140
//                       records the same caveat).
//       BatchDescriptor write/send_batch.rs:263: { location: String,
//                       format: BatchFormat }.
//       BatchFormat     glean.thrift:909-912 (JSON=0, Binary=1). The
//                       generated Haskell instance (gen-hs2
//                       Glean/Types.hs:9056-9062) has a THIRD
//                       constructor, BatchFormat__UNKNOWN Int, because
//                       ThriftEnum.toThriftEnum never fails; the Rust
//                       image must preserve an unknown ordinal too
//                       (DIVERGENCES 4).
//       PredicateStats  glean.thrift (pid/count/size), the type
//                       marshal_predicate_stats already returns.
//   * glean_core::ffi — invoke, with, using, create, Object, Static,
//     Destroy, ForeignPtr, unsafe_with_bytes, unsafe_malloced_byte_string,
//     unsafe_malloced_utf8, using_many_malloced, with_utf8_text, FfiError.
//   * glean_core::repo::text::repo_to_text (row 51, shipped).
//   * glean_core::rts::{foreign::{fact_set, inventory, lookup, ownership,
//     stats}, types::{Fid, Pid}} — rows 32/19/57/31/11, all shipped.
//   * crate::database::{backup::backend::{Data, RemoveOnDrop,
//     remove_path_forcibly}, exception::DbError, repo::combine,
//     storage::{AxiomOwnership, DatabaseOps, WriteLock}} — rows 56/53/52/58.
//
// ---------------------------------------------------------------------------
// SIBLING-UNIT CONTRACTS
// ---------------------------------------------------------------------------
//
//   * DatabaseOps (storage.rs:400-507) is the trait this impl fills. Its methods
//     take &self and return Result<_, DbError>; commit takes &FactSet (the
//     C++ commit borrows it, storage/ffi.cpp:103-105).
//   * Ownership::compute / compute_derived_ownership (ownership.rs:478,
//     :808) own the whole FFI bracket for the iterator argument; this file
//     only acquires the iterator (using) and maps FfiError -> DbError.
//   * marshal_predicate_stats (stats.rs:116) is the exact image of
//     'marshalPredicateStats . glean_rocksdb_database_predicateStats'.
//   * Row 70 (LMDB) and row 72 (RocksDB) construct DB through
//     DB::from_raw(ptr, repo); row 72 must NOT attach a second finalizer
//     and must NOT call create itself.
//   * Row 72 also owns the RocksDB Storage impl (open/delete/restore) and
//     calls checkpoint from here for the checkpoint-style backup.
//
// ---------------------------------------------------------------------------
// EXPORT MAPPING (4/4 source exports, DB.hs:10-12)
// ---------------------------------------------------------------------------
//
//   :11 DB(..)          -> pub struct DB { pub db_ptr: Db, pub db_repo }
//                          (the source's two field selectors; Db is the
//                          Rust image of the ForeignPtr's target, see
//                          DIVERGENCES 1)
//   :11 Container(..)   -> pub struct Container(pub *mut c_void)
//   :11 withContainer   -> pub fn with_container
//   :11 checkpoint      -> pub fn checkpoint
// Trait impls (the source's instances, not exports):
//   * CanLookup for DB (DB.hs:52-55)
//   * DatabaseOps for DB (DB.hs:57-267)
//   * Static for Container (DB.hs:287-288)
//   * Object for Db — Rust-only, the image of the ForeignPtr the source
//     builds in RocksDB.hs:145 (DIVERGENCES 1)
//
// ---------------------------------------------------------------------------
// FIDELITY NOTES / DIVERGENCES (all deliberate)
// ---------------------------------------------------------------------------
//
//   1. ForeignPtr target and finalizer home. The Haskell DB holds
//      ForeignPtr DB and RocksDB.hs:145 attaches
//      glean_rocksdb_database_free with newForeignPtr. Rust's Object impl
//      must name the destroy fn at the type, and Object::wrap receives
//      only the pointer (the wrapper also needs the Repo), so the pointer
//      is owned by a Rust-only marker Db (a ForeignPtr<Db> newtype) and
//      DB::from_raw(ptr, repo) is the one construction door. The C symbol
//      is the same; only its declaration moved into this unit.
//   2. lookupName is materialized. CanLookup::lookup_name returns &str,
//      while the source computes "rocksdb:" <> repoToText repo on demand;
//      DB stores the string at construction. Same value, one allocation
//      per open.
//   3. optimize's second argument is Rust bool, matching C++ bool
//      (storage/ffi.h:56). The Haskell declares CBool (CInt) and passes
//      fromIntegral (fromEnum compact); on the SysV ABI only the low byte
//      is read, so behavior is identical and bool states the contract.
//   4. BatchFormat keeps unknown ordinals. fromThriftEnum is total in the
//      generated Haskell (BatchFormat__UNKNOWN Int), so the Rust image
//      here matches Json/Binary/Unknown(u32); the two-variant sketch in
//      write/send_batch.rs:263 is extended, not contradicted (that file
//      never converts from an int).
//   5. checkpoint vs bracketOnError. A RemoveOnDrop guard is armed AFTER
//      the acquire succeeds (so an acquire failure does not delete a
//      directory the C side never created), is disarmed on success, and
//      covers the panic path. On the Err path the cleanup is called
//      explicitly so a cleanup error replaces the body error, exactly as
//      bracketOnError's release exception does; Drop cannot report.
//   6. makeAbsolute is System.Directory.makeAbsolute (directory-1.3.8.5
//      OsPath.hs:948-951): prepend the cwd when relative, normalise
//      lexically (System.FilePath.normalise: collapse separators and '.',
//      resolve '..' against the preceding component), then match the
//      INPUT's trailing-separator presence. A non-UTF-8 current directory
//      cannot be represented by the port's String FilePath; it is reported
//      as InvalidData rather than silently mangled.
//   7. The three array frees in get_unprocessed_batch_descriptors /
//      get_units_by_prefix go through using_many_malloced, i.e. in reverse
//      array order; the Haskell frees locations, sizes, formats forward.
//      The arrays are independent mallocs, so the order is unobservable.
//   8. Paths cross the FFI as NUL-terminated UTF-8 (CString). Haskell's
//      withCString uses the locale encoding; RULE §1 maps FilePath to
//      String, so UTF-8 is the port's fixed encoding.
//   9. GHC's safe/unsafe ccall annotation has no Rust image: it controls
//      GHC's runtime locking, not the C ABI, and none of these callees
//      re-enters Haskell.
//  10. getUnitId keeps the source's w64 > 0xffffffff test (DB.hs:195):
//      the C side uses UINT64_MAX as its not-found sentinel
//      (storage/ffi.cpp:226), so every value above u32::MAX maps to
//      Nothing. Reproduced as written.
//  11. retrieve reads the out-params only when found /= 0, and getUnit
//      reads the buffer only when unit_size /= 0; the C++ writes neither
//      pair on the missing path (storage/ffi.cpp:45-61, :231-245), so no
//      buffer is leaked and none is freed twice.
//  12. addOwnership with an empty map does not call the C side
//      (DB.hs:133, 'when (not $ HashMap.null owned)').
//
// MARKERS: zero of every class — no port todos (all 28 externs are
// declared and every DatabaseOps method is implemented), no source-defect
// reproductions (the missing-value paths are the source's own design), no
// performance deferrals, no inventory divergences.
//
// SAFETY count: 36 // SAFETY: comment lines over 34 unsafe blocks and 4
// unsafe declarations (the Db finalizer shim, the Object impl,
// DB::from_raw, and the 29-function extern block).
//
// PORT STATUS: confidence=high todos=0

use std::ffi::{c_char, c_int, c_void, CString};
use std::io;
use std::ptr;

use glean_core::ffi::{
    create, invoke, unsafe_malloced_byte_string, unsafe_malloced_utf8, unsafe_with_bytes, using,
    using_many_malloced, with, with_utf8_text, Destroy, FfiError, ForeignPtr, Object, Static,
};
use glean_core::repo::text::repo_to_text;
use glean_core::rts::foreign::fact_set::FactSet;
use glean_core::rts::foreign::inventory::Inventory;
use glean_core::rts::foreign::lookup::{CanLookup, Lookup};
use glean_core::rts::foreign::ownership::{
    compute, compute_derived_ownership, ComputedOwnership, DefineOwnership,
    DerivedFactOwnershipIterator, Ownership, UnitId, UnitIterator,
};
use glean_core::rts::foreign::stats::marshal_predicate_stats;
use glean_core::rts::types::{Fid, Pid};
use glean_thrift as thrift;

use crate::database::backup::backend::{remove_path_forcibly, Data, RemoveOnDrop};
use crate::database::exception::DbError;
use crate::database::repo::combine;
use crate::database::storage::{AxiomOwnership, DatabaseOps, WriteLock};

// ---------------------------------------------------------------------------
// The C++ Database object and its Rust handle
// ---------------------------------------------------------------------------

/// The C++ Database object (glean/storage/ffi.h:35).
///
/// Rust-only marker. The Haskell names the pointer's target DB as well (the
/// same name as the wrapper), so 'ForeignPtr DB' points at a C struct that
/// has no Haskell fields; Rust's Object::wrap must build the value from the
/// pointer alone, and the wrapper also needs the Repo, so the pointer's
/// owner is this marker and DB holds it. Construction goes through
/// DB::from_raw only (DIVERGENCES 1).
pub struct Db {
    fp: ForeignPtr<Db>,
}

// SAFETY: destroy() is the finalizer RocksDB.hs:145 attaches to the same
// pointer (glean_rocksdb_database_free), and wrap/unwrap round-trip over
// the same clone-shared ForeignPtr. The only door onto a live handle is
// create() inside DB::from_raw.
unsafe impl Object for Db {
    fn destroy() -> Destroy<Self> {
        destroy_database
    }

    fn wrap(fp: ForeignPtr<Self>) -> Self {
        Db { fp }
    }

    fn unwrap(&self) -> ForeignPtr<Self> {
        self.fp.clone()
    }
}

unsafe extern "C" fn destroy_database(p: *mut Db) {
    // SAFETY: p is the Database* adopted at construction; ForeignPtr's cell
    // guarantees at-most-once execution, and glean_rocksdb_database_free
    // (storage/ffi.cpp:91-93, ffi::free_) cannot throw across.
    unsafe { sys::glean_rocksdb_database_free(p) }
}

/// 'data DB = DB { dbPtr :: ForeignPtr DB, dbRepo :: Repo }' (DB.hs:47-49).
pub struct DB {
    /// 'dbPtr' — the shared owning handle for the C++ Database.
    pub db_ptr: Db,
    /// 'dbRepo' — the repo this database belongs to.
    pub db_repo: thrift::Repo,
    /// The on-demand 'lookupName' (DIVERGENCES 2), computed once.
    lookup_name: String,
}

impl DB {
    /// 'DB (castForeignPtr fp) repo' (RocksDB.hs:145): adopt a live
    /// Database* and its repo.
    ///
    /// SAFETY: ptr must be a live, uniquely-owned Database* from a
    /// successful glean_rocksdb_container_open_database call (never null,
    /// never already adopted or freed elsewhere). Ownership transfers: the
    /// finalizer glean_rocksdb_database_free runs at most once, at drop.
    pub unsafe fn from_raw(ptr: *mut c_void, repo: thrift::Repo) -> DB {
        let lookup_name = format!("rocksdb:{}", repo_to_text(&repo));
        // SAFETY: from_raw's contract is create's contract verbatim.
        let db_ptr = unsafe { create::<Db>(ptr.cast()) };
        DB {
            db_ptr,
            db_repo: repo,
            lookup_name,
        }
    }
}

// ---------------------------------------------------------------------------
// CanLookup (DB.hs:52-55)
// ---------------------------------------------------------------------------

impl CanLookup for DB {
    fn with_lookup<T, E: From<FfiError>>(
        &self,
        f: impl FnOnce(*mut c_void) -> Result<T, E>,
    ) -> Result<T, E> {
        // SAFETY: the handle owns a live Database* for as long as &self is
        // borrowed; glean_rocksdb_database_lookup returns the Database*
        // itself (storage/ffi.cpp:99-101), and f must not retain it.
        unsafe {
            with(&self.db_ptr, |p| {
                f(sys::glean_rocksdb_database_lookup(p.cast()))
            })
        }
    }

    fn lookup_name(&self) -> &str {
        &self.lookup_name
    }
}

// ---------------------------------------------------------------------------
// DatabaseOps (DB.hs:57-267)
// ---------------------------------------------------------------------------

impl DatabaseOps for DB {
    fn close(&self) -> Result<(), DbError> {
        // SAFETY: with_container yields the live Container* for this call;
        // close consumes nothing else and returns void.
        with_container(self, |c| unsafe { sys::glean_rocksdb_container_close(c) });
        Ok(())
    }

    fn predicate_stats(&self) -> Result<Vec<(Pid, thrift::PredicateStats)>, DbError> {
        // SAFETY: p is the live Database* for the call; the four out-params
        // are initialised stack locals owned by marshal_predicate_stats,
        // whose C contract (storage/ffi.h:138-143) fills them.
        let result = unsafe {
            with(&self.db_ptr, |p| {
                marshal_predicate_stats(|count, pids, counts, sizes| {
                    sys::glean_rocksdb_database_predicateStats(p.cast(), count, pids, counts, sizes)
                })
            })
        };
        result.map_err(DbError::from)
    }

    fn store(&self, key: &[u8], value: &[u8]) -> Result<(), DbError> {
        // SAFETY: both byte buffers outlive the call (borrowed from &self's
        // arguments); the C side reads them as const void* and does not
        // retain them (storage/ffi.cpp:32-43).
        let result = unsafe {
            with_container(self, |c| {
                unsafe_with_bytes(key, |kp, kl| {
                    unsafe_with_bytes(value, |vp, vl| {
                        invoke(|| sys::glean_rocksdb_container_write_data(c, kp, kl, vp, vl))
                    })
                })
            })
        };
        result.map_err(DbError::from)
    }

    fn retrieve(&self, key: &[u8]) -> Result<Option<Vec<u8>>, DbError> {
        // SAFETY: key outlives the call; the out-params are stack locals and
        // the C side writes value/value_size only when found is set
        // (storage/ffi.cpp:45-61). A found buffer is malloc'd and adopted by
        // unsafe_malloced_byte_string, which frees it exactly once.
        let result: Result<Option<Vec<u8>>, FfiError> = unsafe {
            with_container(self, |c| {
                unsafe_with_bytes(key, |kp, kl| {
                    let mut value: *mut c_void = ptr::null_mut();
                    let mut value_size: usize = 0;
                    let mut found: u8 = 0;
                    invoke(|| {
                        sys::glean_rocksdb_container_read_data(
                            c,
                            kp,
                            kl,
                            &mut value,
                            &mut value_size,
                            &mut found,
                        )
                    })?;
                    if found != 0 {
                        Ok(Some(unsafe_malloced_byte_string(value, value_size)))
                    } else {
                        Ok(None)
                    }
                })
            })
        };
        result.map_err(DbError::from)
    }

    fn add_batch_descriptor(&self, descriptor: &thrift::BatchDescriptor) -> Result<(), DbError> {
        let format = batch_format_to_int(&descriptor.format);
        // SAFETY: p is the live Database* and the location bytes outlive the
        // call; the C side copies them into a std::string
        // (storage/ffi.cpp:107-118).
        let result = unsafe {
            with(&self.db_ptr, |p| {
                with_utf8_text(&descriptor.location, |lp, ll| {
                    invoke(|| {
                        sys::glean_rocksdb_add_batch_descriptor(p.cast(), lp.cast(), ll, format)
                    })
                })
            })
        };
        result.map_err(DbError::from)
    }

    fn mark_batch_descriptor_as_written(&self, location: &String) -> Result<(), DbError> {
        // SAFETY: p is the live Database* and the location bytes outlive the
        // call; the C side only reads them (storage/ffi.cpp:120-129).
        let result = unsafe {
            with(&self.db_ptr, |p| {
                with_utf8_text(location, |lp, ll| {
                    invoke(|| {
                        sys::glean_rocksdb_mark_batch_descriptor_as_written(p.cast(), lp.cast(), ll)
                    })
                })
            })
        };
        result.map_err(DbError::from)
    }

    fn is_batch_descriptor_stored(&self, location: &String) -> Result<bool, DbError> {
        // SAFETY: p is the live Database*, the location bytes outlive the
        // call, and found is an initialised stack byte the C side writes
        // (storage/ffi.cpp:131-144).
        let result: Result<bool, FfiError> = unsafe {
            with(&self.db_ptr, |p| {
                with_utf8_text(location, |lp, ll| {
                    let mut found: u8 = 0;
                    invoke(|| {
                        sys::glean_rocksdb_is_batch_descriptor_stored(
                            p.cast(),
                            lp.cast(),
                            ll,
                            &mut found,
                        )
                    })?;
                    Ok(found != 0)
                })
            })
        };
        result.map_err(DbError::from)
    }

    fn get_unprocessed_batch_descriptors(&self) -> Result<Vec<thrift::BatchDescriptor>, DbError> {
        // SAFETY: p is the live Database*; the four out-params are stack
        // locals. The C side always mallocs the three arrays, even for
        // count == 0 (storage/ffi.cpp:146-170), so using_many_malloced
        // frees them unconditionally; each element buffer is adopted and
        // freed by unsafe_malloced_utf8.
        let result: Result<Vec<thrift::BatchDescriptor>, FfiError> = unsafe {
            with(&self.db_ptr, |p| {
                let mut count: usize = 0;
                let mut locations: *mut *mut c_char = ptr::null_mut();
                let mut location_sizes: *mut usize = ptr::null_mut();
                let mut formats: *mut u32 = ptr::null_mut();
                invoke(|| {
                    sys::glean_rocksdb_get_unprocessed_batch_descriptors(
                        p.cast(),
                        &mut count,
                        &mut locations,
                        &mut location_sizes,
                        &mut formats,
                    )
                })?;
                let arrays = [
                    locations.cast::<c_void>(),
                    location_sizes.cast::<c_void>(),
                    formats.cast::<c_void>(),
                ];
                Ok(using_many_malloced(&arrays, || {
                    let mut results = Vec::with_capacity(count);
                    for i in 0..count {
                        let location = unsafe_malloced_utf8(
                            (*locations.add(i)).cast(),
                            *location_sizes.add(i),
                        );
                        let format = *formats.add(i);
                        results.push(thrift::BatchDescriptor {
                            location,
                            format: batch_format_from_int(format),
                        });
                    }
                    results
                }))
            })
        };
        result.map_err(DbError::from)
    }

    fn commit(&self, facts: &FactSet) -> Result<(), DbError> {
        // SAFETY: p is the live Database*; with(facts, ...) keeps the
        // FactSet handle alive for the call and the C++ commit only borrows
        // it (storage/ffi.cpp:103-105).
        let result = unsafe {
            with(&self.db_ptr, |p| {
                with(facts, |f| {
                    invoke(|| sys::glean_rocksdb_commit(p.cast(), f.cast()))
                })
            })
        };
        result.map_err(DbError::from)
    }

    fn add_ownership<W>(&self, _lock: &WriteLock<W>, owned: AxiomOwnership) -> Result<(), DbError> {
        // DB.hs:133: no C call for an empty map.
        if owned.is_empty() {
            return Ok(());
        }
        // The four parallel arrays the C side reads
        // (storage/ffi.cpp:172-189). The unit/fact buffers are borrowed from
        // 'owned', which outlives the call.
        let mut unit_ptrs: Vec<*const c_void> = Vec::with_capacity(owned.len());
        let mut unit_sizes: Vec<usize> = Vec::with_capacity(owned.len());
        let mut fact_ptrs: Vec<*const Fid> = Vec::with_capacity(owned.len());
        let mut fact_sizes: Vec<usize> = Vec::with_capacity(owned.len());
        for (unit, facts) in owned.iter() {
            unit_ptrs.push(unit.as_ptr().cast());
            unit_sizes.push(unit.len());
            fact_ptrs.push(facts.as_ptr());
            fact_sizes.push(facts.len());
        }
        let count = owned.len();
        // SAFETY: p is the live Database*; all four arrays are live for the
        // call, and Fid is repr(transparent) over the int64_t the C side
        // reads. The C side copies the ranges and does not retain them.
        let result = unsafe {
            with(&self.db_ptr, |p| {
                invoke(|| {
                    sys::glean_rocksdb_add_ownership(
                        p.cast(),
                        count,
                        unit_ptrs.as_ptr(),
                        unit_sizes.as_ptr(),
                        fact_ptrs.as_ptr().cast(),
                        fact_sizes.as_ptr(),
                    )
                })
            })
        };
        result.map_err(DbError::from)
    }

    fn register_acl_units<W>(
        &self,
        _lock: &WriteLock<W>,
        names: &[Vec<u8>],
    ) -> Result<UnitId, DbError> {
        // The two parallel arrays (storage/ffi.cpp:191-212); the name
        // buffers are borrowed from 'names', which outlives the call.
        let name_ptrs: Vec<*const c_void> = names.iter().map(|n| n.as_ptr().cast()).collect();
        let name_sizes: Vec<usize> = names.iter().map(|n| n.len()).collect();
        let mut first: u32 = 0;
        // SAFETY: p is the live Database*; the arrays and names are live for
        // the call; first is an initialised stack out-param.
        let result = unsafe {
            with(&self.db_ptr, |p| {
                invoke(|| {
                    sys::glean_rocksdb_register_units(
                        p.cast(),
                        names.len(),
                        name_ptrs.as_ptr(),
                        name_sizes.as_ptr(),
                        &mut first,
                    )
                })
            })
        };
        result.map_err(DbError::from)?;
        Ok(UnitId(first))
    }

    fn optimize(&self, compact: bool) -> Result<(), DbError> {
        // SAFETY: with_container yields the live Container* for the call;
        // bool is the C++ parameter type (DIVERGENCES 3).
        let result = with_container(self, |c| unsafe {
            invoke(|| sys::glean_rocksdb_container_optimize(c, compact))
        });
        result.map_err(DbError::from)
    }

    fn flush(&self) -> Result<(), DbError> {
        // SAFETY: with_container yields the live Container* for the call.
        let result = with_container(self, |c| unsafe {
            invoke(|| sys::glean_rocksdb_container_flush(c))
        });
        result.map_err(DbError::from)
    }

    fn compute_ownership(
        &self,
        base: Option<&Lookup>,
        inventory: &Inventory,
    ) -> Result<ComputedOwnership, DbError> {
        let mut iter: *mut c_void = ptr::null_mut();
        // SAFETY: p is the live Database*; iter is an initialised stack
        // out-param. The acquire is the allocation-guard leg the governing
        // row names; a null result is impossible on success.
        unsafe {
            with(&self.db_ptr, |p| {
                invoke(|| sys::glean_rocksdb_get_ownership_unit_iterator(p.cast(), &mut iter))
            })?;
        }
        // SAFETY: iter is the fresh, uniquely-owned iterator from the
        // successful call; using destroys it exactly once on every path.
        let iter = unsafe { UnitIterator::from_raw(iter) };
        using(|| iter, |it| compute(inventory, self, base, it)).map_err(DbError::from)
    }

    fn store_ownership<W>(
        &self,
        _lock: &WriteLock<W>,
        ownership: &ComputedOwnership,
    ) -> Result<(), DbError> {
        // SAFETY: p is the live Database*; with(ownership, ...) keeps the
        // ComputedOwnership handle alive and the C++ copies it
        // (storage/ffi.cpp:287-291).
        let result = unsafe {
            with(&self.db_ptr, |p| {
                with(ownership, |o| {
                    invoke(|| sys::glean_rocksdb_store_ownership(p.cast(), o.cast()))
                })
            })
        };
        result.map_err(DbError::from)
    }

    fn get_ownership(&self) -> Result<Option<Ownership>, DbError> {
        let mut ownership: *mut c_void = ptr::null_mut();
        // SAFETY: p is the live Database*; ownership is an initialised stack
        // out-param. The C side always releases a fresh Ownership* on
        // success (storage/ffi.cpp:293-295), adopted below.
        unsafe {
            with(&self.db_ptr, |p| {
                invoke(|| sys::glean_rocksdb_get_ownership(p.cast(), &mut ownership))
            })?;
        }
        // SAFETY: ownership is the fresh, uniquely-owned Ownership* from the
        // successful call; from_raw adopts it with its finalizer.
        Ok(Some(unsafe { Ownership::from_raw(ownership) }))
    }

    fn get_unit_id(&self, unit: &[u8]) -> Result<Option<UnitId>, DbError> {
        let mut w64: u64 = 0;
        // SAFETY: p is the live Database*; the unit bytes outlive the call;
        // w64 is an initialised stack out-param. The C side writes
        // UINT64_MAX for "not found" (storage/ffi.cpp:214-228).
        unsafe {
            with(&self.db_ptr, |p| {
                unsafe_with_bytes(unit, |up, ul| {
                    invoke(|| sys::glean_rocksdb_get_unit_id(p.cast(), up.cast_mut(), ul, &mut w64))
                })
            })?;
        }
        // DB.hs:195-198, verbatim.
        if w64 > 0xffff_ffff {
            Ok(None)
        } else {
            Ok(Some(UnitId(w64 as u32)))
        }
    }

    fn get_unit(&self, id: UnitId) -> Result<Option<Vec<u8>>, DbError> {
        let mut unit: *mut c_void = ptr::null_mut();
        let mut unit_size: usize = 0;
        // SAFETY: p is the live Database*; unit/unit_size are initialised
        // stack out-params. On the missing path the C side writes null/0
        // (storage/ffi.cpp:231-245), so nothing is adopted.
        unsafe {
            with(&self.db_ptr, |p| {
                invoke(|| sys::glean_rocksdb_get_unit(p.cast(), id, &mut unit, &mut unit_size))
            })?;
        }
        if unit_size != 0 {
            // SAFETY: unit is the malloc'd buffer the C side released; the
            // adoption frees it exactly once.
            Ok(Some(unsafe {
                unsafe_malloced_byte_string(unit, unit_size)
            }))
        } else {
            Ok(None)
        }
    }

    fn get_units_by_prefix(&self, prefix: &[u8]) -> Result<Vec<(Vec<u8>, UnitId)>, DbError> {
        // SAFETY: p is the live Database*; the prefix outlives the call; the
        // four out-params are stack locals. The C side always mallocs the
        // three arrays (storage/ffi.cpp:247-285), freed unconditionally by
        // using_many_malloced; each name buffer is adopted and freed by
        // unsafe_malloced_byte_string.
        let result: Result<Vec<(Vec<u8>, UnitId)>, FfiError> = unsafe {
            with(&self.db_ptr, |p| {
                let mut count: usize = 0;
                let mut names: *mut *mut c_void = ptr::null_mut();
                let mut name_sizes: *mut usize = ptr::null_mut();
                let mut ids: *mut u32 = ptr::null_mut();
                unsafe_with_bytes(prefix, |pp, pl| {
                    invoke(|| {
                        sys::glean_rocksdb_get_units_by_prefix(
                            p.cast(),
                            pp,
                            pl,
                            &mut count,
                            &mut names,
                            &mut name_sizes,
                            &mut ids,
                        )
                    })
                })?;
                let arrays = [
                    names.cast::<c_void>(),
                    name_sizes.cast::<c_void>(),
                    ids.cast::<c_void>(),
                ];
                Ok(using_many_malloced(&arrays, || {
                    let mut results = Vec::with_capacity(count);
                    for i in 0..count {
                        let name = unsafe_malloced_byte_string(*names.add(i), *name_sizes.add(i));
                        results.push((name, UnitId(*ids.add(i))));
                    }
                    results
                }))
            })
        };
        result.map_err(DbError::from)
    }

    fn add_define_ownership<W>(
        &self,
        _lock: &WriteLock<W>,
        define: &DefineOwnership,
    ) -> Result<(), DbError> {
        // SAFETY: p is the live Database*; with(define, ...) keeps the
        // DefineOwnership handle alive and the C++ copies it
        // (storage/ffi.cpp:297-302).
        let result = unsafe {
            with(&self.db_ptr, |p| {
                with(define, |d| {
                    invoke(|| sys::glean_rocksdb_add_define_ownership(p.cast(), d.cast()))
                })
            })
        };
        result.map_err(DbError::from)
    }

    fn compute_derived_ownership<W>(
        &self,
        _lock: &WriteLock<W>,
        ownership: &Ownership,
        base: Option<&Lookup>,
        pid: Pid,
    ) -> Result<ComputedOwnership, DbError> {
        let mut iter: *mut c_void = ptr::null_mut();
        // SAFETY: p is the live Database*; pid.0 as u64 is the source's
        // fromIntegral (two's complement for negative ids); iter is an
        // initialised stack out-param.
        unsafe {
            with(&self.db_ptr, |p| {
                invoke(|| {
                    sys::glean_rocksdb_get_derived_fact_ownership_iterator(
                        p.cast(),
                        pid.0 as u64,
                        &mut iter,
                    )
                })
            })?;
        }
        // SAFETY: iter is the fresh, uniquely-owned iterator; using destroys
        // it exactly once.
        let iter = unsafe { DerivedFactOwnershipIterator::from_raw(iter) };
        using(|| iter, |it| compute_derived_ownership(ownership, base, it)).map_err(DbError::from)
    }

    fn cache_ownership(&self) -> Result<(), DbError> {
        // SAFETY: p is the live Database* for the call.
        let result = unsafe {
            with(&self.db_ptr, |p| {
                invoke(|| sys::glean_rocksdb_cache_ownership(p.cast()))
            })
        };
        result.map_err(DbError::from)
    }

    fn prepare_fact_owner_cache(&self) -> Result<(), DbError> {
        // SAFETY: p is the live Database* for the call.
        let result = unsafe {
            with(&self.db_ptr, |p| {
                invoke(|| sys::glean_rocksdb_prepare_fact_owner_cache(p.cast()))
            })
        };
        result.map_err(DbError::from)
    }

    fn backup<A, F>(
        &self,
        _config: &thrift::Config,
        scratch: &str,
        process: F,
    ) -> Result<A, DbError>
    where
        F: FnOnce(&str, Data) -> Result<A, DbError>,
    {
        let path = combine(scratch, "backup");
        std::fs::create_dir_all(&path)?;
        // SAFETY: with_container yields the live Container*; the path is
        // NUL-terminated for the call only (with_c_string owns the CString).
        unsafe {
            with_container(self, |c| {
                with_c_string(&path, |p| {
                    invoke(|| sys::glean_rocksdb_container_backup(c, p))
                })
            })?;
        }
        process(&path, Data { data_size: 0 })
    }
}

// ---------------------------------------------------------------------------
// checkpoint (DB.hs:269-281)
// ---------------------------------------------------------------------------

/// Create an openable snapshot (a RocksDB Checkpoint) of the DB under
/// scratch/db and hand the resulting directory to process.
///
/// The target directory must not already exist; it is passed to the FFI as
/// an absolute path (RocksDB requires it). The cleanup precedence mirrors
/// bracketOnError (DIVERGENCES 5).
pub fn checkpoint<A, F>(db: &DB, scratch: &str, process: F) -> Result<A, DbError>
where
    F: FnOnce(&str, Data) -> Result<A, DbError>,
{
    let path = combine(scratch, "db");
    let abs_path = make_absolute(&path)?;
    // SAFETY: with_container yields the live Container*; the absolute path
    // is NUL-terminated for the call only.
    unsafe {
        with_container(db, |c| {
            with_c_string(&abs_path, |p| {
                invoke(|| sys::glean_rocksdb_container_checkpoint(c, p))
            })
        })?;
    }
    // Armed only after a successful acquire: bracketOnError does not run its
    // release when acquire throws. Covers the panic path; the Err path below
    // calls remove explicitly so a cleanup error can replace the body error.
    let mut guard = RemoveOnDrop::new(&path);
    match process(&path, Data { data_size: 0 }) {
        Ok(a) => {
            guard.disarm();
            Ok(a)
        }
        Err(e) => {
            let cleanup = remove_path_forcibly(&path);
            guard.disarm();
            match cleanup {
                Err(io) => Err(DbError::from(io)),
                Ok(()) => Err(e),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Container (DB.hs:284-292)
// ---------------------------------------------------------------------------

/// 'newtype Container = Container (Ptr Container)' (DB.hs:284-285): the C++
/// Container* handed out by glean_rocksdb_database_container. It is owned
/// by the DB, not by this value, so no Drop is attached.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct Container(pub *mut c_void);

impl Static for Container {
    fn destroy_static(&self) {
        // SAFETY: self.0 is a live Container* owned by its DB; the Haskell
        // declares the same destroyStatic (DB.hs:287-288) and never calls it
        // on a container the DB still owns (no call sites in the tree).
        unsafe { sys::glean_rocksdb_container_free(*self) }
    }
}

/// 'withContainer db f = unsafeWithForeignPtr (dbPtr db) $ f .
/// glean_rocksdb_database_container' (DB.hs:290-292).
pub fn with_container<R>(db: &DB, f: impl FnOnce(Container) -> R) -> R {
    // SAFETY: the handle owns a live Database* for as long as &db is
    // borrowed; the returned Container* points into that Database and f
    // must not retain it beyond the call.
    unsafe {
        with(&db.db_ptr, |p| {
            f(sys::glean_rocksdb_database_container(p.cast()))
        })
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// 'withCString' for a UTF-8 path/string (DIVERGENCES 8). GHC's
/// newCAString rejects an embedded NUL with this message; keep it.
fn with_c_string<R>(s: &str, f: impl FnOnce(*const c_char) -> R) -> R {
    let c = CString::new(s).expect("invalid argument (embedded NUL)");
    f(c.as_ptr())
}

/// 'makeAbsolute path' (System.Directory.makeAbsolute, directory-1.3.8.5
/// OsPath.hs:948-951): prepend the current directory when relative, then
/// normalise, then match the INPUT's trailing-separator presence
/// (DIVERGENCES 6).
fn make_absolute(path: &str) -> io::Result<String> {
    let joined = if std::path::Path::new(path).is_absolute() {
        path.to_string()
    } else {
        let joined = std::env::current_dir()?.join(path);
        match joined.into_os_string().into_string() {
            Ok(s) => s,
            Err(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "current directory is not valid UTF-8",
                ))
            }
        }
    };
    Ok(match_trailing_separator(path, normalise(&joined)))
}

/// 'System.FilePath.normalise' (POSIX): collapse repeated separators and
/// '.' components, resolve '..' lexically against the preceding component,
/// keep a leading '/' and a trailing separator, and map "" to ".".
fn normalise(path: &str) -> String {
    if path.is_empty() {
        return ".".to_string();
    }
    let absolute = path.starts_with('/');
    let trailing = path.len() > 1 && path.ends_with('/');
    let mut parts: Vec<&str> = Vec::new();
    for comp in path.split('/') {
        match comp {
            "" | "." => {}
            ".." => {
                if parts.last().is_some_and(|last| *last != "..") {
                    parts.pop();
                } else if !absolute {
                    parts.push("..");
                }
            }
            other => parts.push(other),
        }
    }
    let mut out = String::new();
    if absolute {
        out.push('/');
    }
    out.push_str(&parts.join("/"));
    if out.is_empty() {
        out.push('.');
    }
    if trailing && !out.ends_with('/') {
        out.push('/');
    }
    out
}

/// 'matchTrailingSeparator path' (OsPath.hs:957-961): add or drop the
/// trailing separator of the normalised path so it matches the input.
fn match_trailing_separator(original: &str, path: String) -> String {
    if original.ends_with('/') {
        if path.ends_with('/') {
            path
        } else {
            format!("{}/", path)
        }
    } else if path.len() > 1 && path.ends_with('/') {
        path[..path.len() - 1].to_string()
    } else {
        path
    }
}

/// 'fromThriftEnum' for BatchFormat (gen-hs2 Glean/Types.hs:9056-9062):
/// JSON=0, Binary=1, and the UNKNOWN catch-all preserves the ordinal
/// (DIVERGENCES 4).
fn batch_format_to_int(format: &thrift::BatchFormat) -> c_int {
    match format {
        thrift::BatchFormat::Json => 0,
        thrift::BatchFormat::Binary => 1,
        thrift::BatchFormat::Unknown(n) => *n as c_int,
    }
}

/// 'toThriftEnum' for BatchFormat: total in the generated Haskell, so an
/// unknown ordinal becomes Unknown, never an error.
fn batch_format_from_int(n: u32) -> thrift::BatchFormat {
    match n {
        0 => thrift::BatchFormat::Json,
        1 => thrift::BatchFormat::Binary,
        other => thrift::BatchFormat::Unknown(other),
    }
}

// ---------------------------------------------------------------------------
// The C ABI (DB.hs:294-449): 28 declarations, one extern block
// ---------------------------------------------------------------------------

mod sys {
    use std::ffi::{c_char, c_int, c_void};

    use super::{Container, Db, UnitId};

    unsafe extern "C" {
        // DB.hs:294-303 — void mutators and the two path calls.
        pub fn glean_rocksdb_container_free(container: Container);
        pub fn glean_rocksdb_container_close(container: Container);
        pub fn glean_rocksdb_container_backup(
            container: Container,
            path: *const c_char,
        ) -> *const c_char;
        pub fn glean_rocksdb_container_checkpoint(
            container: Container,
            path: *const c_char,
        ) -> *const c_char;

        // DB.hs:306-321 — key/value access.
        pub fn glean_rocksdb_container_write_data(
            container: Container,
            key: *const c_void,
            key_size: usize,
            value: *const c_void,
            value_size: usize,
        ) -> *const c_char;
        pub fn glean_rocksdb_container_read_data(
            container: Container,
            key: *const c_void,
            key_size: usize,
            value: *mut *mut c_void,
            value_size: *mut usize,
            found: *mut u8,
        ) -> *const c_char;

        // DB.hs:323-349 — batch descriptors.
        pub fn glean_rocksdb_add_batch_descriptor(
            db: *mut c_void,
            location: *const c_char,
            location_size: usize,
            format: c_int,
        ) -> *const c_char;
        pub fn glean_rocksdb_mark_batch_descriptor_as_written(
            db: *mut c_void,
            location: *const c_char,
            location_size: usize,
        ) -> *const c_char;
        pub fn glean_rocksdb_is_batch_descriptor_stored(
            db: *mut c_void,
            location: *const c_char,
            location_size: usize,
            found: *mut u8,
        ) -> *const c_char;
        pub fn glean_rocksdb_get_unprocessed_batch_descriptors(
            db: *mut c_void,
            count: *mut usize,
            locations: *mut *mut *mut c_char,
            location_sizes: *mut *mut usize,
            formats: *mut *mut u32,
        ) -> *const c_char;

        // DB.hs:351-361 — container/database accessors.
        pub fn glean_rocksdb_container_optimize(
            container: Container,
            compact: bool,
        ) -> *const c_char;
        pub fn glean_rocksdb_container_flush(container: Container) -> *const c_char;
        pub fn glean_rocksdb_database_container(db: *mut c_void) -> Container;
        pub fn glean_rocksdb_database_lookup(db: *mut c_void) -> *mut c_void;

        // DB.hs:363-383 — commit and ownership input.
        pub fn glean_rocksdb_commit(db: *mut c_void, facts: *mut c_void) -> *const c_char;
        pub fn glean_rocksdb_add_ownership(
            db: *mut c_void,
            count: usize,
            units: *const *const c_void,
            unit_sizes: *const usize,
            ids: *const *const i64,
            id_sizes: *const usize,
        ) -> *const c_char;
        pub fn glean_rocksdb_register_units(
            db: *mut c_void,
            count: usize,
            units: *const *const c_void,
            unit_sizes: *const usize,
            first_unit_id: *mut u32,
        ) -> *const c_char;

        // DB.hs:385-396 — ownership iterators and predicate stats.
        pub fn glean_rocksdb_get_ownership_unit_iterator(
            db: *mut c_void,
            iter: *mut *mut c_void,
        ) -> *const c_char;
        pub fn glean_rocksdb_database_predicateStats(
            db: *mut c_void,
            count: *mut usize,
            ids: *mut *mut i64,
            counts: *mut *mut u64,
            sizes: *mut *mut u64,
        ) -> *const c_char;

        // DB.hs:398-420 — units.
        pub fn glean_rocksdb_get_unit_id(
            db: *mut c_void,
            unit: *mut c_void,
            unit_size: usize,
            unit_id: *mut u64,
        ) -> *const c_char;
        pub fn glean_rocksdb_get_unit(
            db: *mut c_void,
            unit_id: UnitId,
            unit: *mut *mut c_void,
            unit_size: *mut usize,
        ) -> *const c_char;
        pub fn glean_rocksdb_get_units_by_prefix(
            db: *mut c_void,
            prefix: *const c_void,
            prefix_size: usize,
            count: *mut usize,
            unit_names: *mut *mut *mut c_void,
            unit_name_sizes: *mut *mut usize,
            unit_ids: *mut *mut u32,
        ) -> *const c_char;

        // DB.hs:422-435 — ownership output.
        pub fn glean_rocksdb_store_ownership(
            db: *mut c_void,
            ownership: *mut c_void,
        ) -> *const c_char;
        pub fn glean_rocksdb_get_ownership(
            db: *mut c_void,
            ownership: *mut *mut c_void,
        ) -> *const c_char;
        pub fn glean_rocksdb_add_define_ownership(
            db: *mut c_void,
            define: *mut c_void,
        ) -> *const c_char;

        // DB.hs:437-449 — derived ownership and caches.
        pub fn glean_rocksdb_get_derived_fact_ownership_iterator(
            db: *mut c_void,
            pid: u64,
            iter: *mut *mut c_void,
        ) -> *const c_char;
        pub fn glean_rocksdb_cache_ownership(db: *mut c_void) -> *const c_char;
        pub fn glean_rocksdb_prepare_fact_owner_cache(db: *mut c_void) -> *const c_char;

        // The shared finalizer (RocksDB.hs:145; DIVERGENCES 1).
        pub fn glean_rocksdb_database_free(db: *mut Db);
    }
}
