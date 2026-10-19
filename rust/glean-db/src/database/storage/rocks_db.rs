// Ported from: glean/db/Glean/Database/Storage/RocksDB.hs (326 lines)
// Manifest unit: rust/glean-db/src/database/storage/rocks_db.rs
// (migration/manifest.tsv line 72). RULE §4: one source file -> one target
// file; the manifest path is authoritative.
//
// The production storage backend: one RocksDB container per repo, a shared
// cache handle, tar-based restore/backup, and the DB wrapper row 67 owns.
// Row 70 (LMDB) is the twin; wherever the source repeats itself the two
// files are deliberately parallel.
//
// ---------------------------------------------------------------------------
// INVENTORY (RULE §5; migration/inventory*.tsv)
// ---------------------------------------------------------------------------
//
//   * inventory.tsv:31 (GUARD_KIND, confirmed): "7 ccall + error string
//     out-param", raw extern-C per §1/A2 (the cxx-bridge cell is marked
//     SUPERSEDED), error string -> Result. Followed: six externs in the
//     sys block (the seventh, database_free, is row 67's), every fallible
//     call through invoke.
//   * inventory.sites.tsv:55-61 — the seven imports at :292 (new_cache),
//     :294 (free_cache), :296 (cache_capacity), :305 (container_open),
//     :313 (open_database), :320 (database_free) and :323 (restore). Six are
//     declared below; database_free is row 67's, reached through
//     DB::from_raw.
//   * inventory.sites.tsv:445 (lazy-io, :246 "err <- BS.hGetContents herr")
//     — the stderr read is made STRICT (read_to_end before wait), which is
//     what the source's evaluate (BS.length err) achieves.
//
// ---------------------------------------------------------------------------
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, checked not assumed)
// ---------------------------------------------------------------------------
//
//   * OUTBOUND (10), nine shipped and one assumed:
//       edges.tsv:435 Backup/Backend (56) — Data;
//       :436 Database/Repo (52) — databasePath, combine;
//       :437 Database/Storage (58) — the traits, Mode, WriteLock, seams;
//       :438 Database/Storage/DB (67) — DB, Container, checkpoint;
//       :439 Glean/FFI (assumed, see below);
//       :440 RTS/Foreign/Lookup (19) — CanLookup;
//       :441 RTS/Foreign/Ownership (57) — the ownership types;
//       :442 RTS/Types (11) — Fid, invalidFid, Pid;
//       :443 Impl/MemoryReader (68) — totalMemCapacityKB;
//       :444 Util/Disk (69) — the three capacity queries.
//   * INBOUND (1): edges.tsv:225 Database/Config.hs (manifest line 85, NOT
//     shipped) builds this backend through newStorage.
//   * External, no manifest unit and no edge: System.IO.Temp,
//     System.IO.Extra (withTempFile), System.Process, Util.Log, Util.FFI.
//
// ---------------------------------------------------------------------------
// ASSUMED IMPORTS (RULE §4 B4)
// ---------------------------------------------------------------------------
//
//   * glean_thrift::Config fields this unit reads keep their THRIFT names
//     (the config_ prefix is hsthrift's record-field prefix):
//       db_rocksdb_cache_mb — server_config.thrift:248, i32 -> i32;
//       db_rocksdb_cache_to_mem_ratio — :332, optional float -> Option<f32>;
//       db_rocksdb_cache_index_and_filter_blocks — :288, bool -> bool;
//       db_backup_use_checkpoint — :359, bool -> bool.
//   * glean_core::ffi::{create, invoke, using, Destroy, ForeignPtr, Object}.
//     The Cache handle is a ForeignPtr, exactly as the source's newtype is
//     (DIVERGENCES 1); create/Object are the shipped row-67 mechanism.
//   * glean_util::log::log_info — the unit-45/46 convention: the lazy
//     message thunk is the closure (log_info ALWAYS forces it).
//
// ---------------------------------------------------------------------------
// TRANSLATION NOTES
// ---------------------------------------------------------------------------
//
//   * :52-57 Cache: newtype over ForeignPtr with destroy =
//     glean_rocksdb_free_cache -> pub struct Cache { fp: ForeignPtr<Cache> }
//     plus an Object impl. newCache (:60-61) -> new_cache.
//   * :63-65 withCache (Just c) f = with c f; Nothing f = f nullPtr ->
//     with_cache: Some borrows through ForeignPtr::with_raw, None passes
//     null (the C function accepts null; cache_capacity returns 0 for it).
//   * :67-79 RocksDB record -> pub struct RocksDB { rocks_root,
//     rocks_cache: Option<Cache>, rocks_cache_index_and_filter_blocks,
//     rocks_max_disk_size: Option<i64> }. No Clone: the Database here is
//     just DB, so nothing else holds the record (unlike row 70).
//   * :86-90 cacheSize: the ratio branch multiplies in f32 and rounds with
//     Haskell's ties-to-even; the fallback is cache_mb * 1024 * 1024.
//   * :92-115 newStorage: capacity -> cache size -> allocate a Cache only
//     when the size is > 0. Result because newCache is a fallible FFI call.
//   * :119-140 open: the same mode mapping and container/open_database
//     sequence as row 70, plus the cache pointer and the
//     cache_index_and_filter_blocks flag. NO squashfs branch.
//   * :144-171 delete/safeRemoveForcibly/the three capacities: identical to
//     row 70's; :172 withScratchRoot is root/.scratch.
//   * :174-182 restore: withTempDirectory scratch "restore", untar, then
//     restoreFromDir.
//   * :208-220 backup: mkTar splits the checkpoint path, tars it into a
//     temp file, sizes it, and hands (path, Data) to the caller; the branch
//     picks DB::checkpoint or DB::backup and logs the source's two lines.
//   * :224-291 the tar helpers: unTar, unTarFromHandle (stdin from the
//     handle, stderr drained fully before wait), restoreFromDir, tar.
//
// ---------------------------------------------------------------------------
// DIVERGENCES
// ---------------------------------------------------------------------------
//
//   1. Cache is a glean-core ForeignPtr<Cache> + Object impl rather than a
//      raw pointer with a Drop. That is the source's own shape (ForeignPtr)
//      and gives the same contract: finalizer at most once, clone-shared,
//      Send + Sync (the shipped impls, ffi.rs:455-463). It also keeps
//      RocksDB Send, which the Haskell record is.
//   2. withTempFile (System.IO.Extra, :210) is emulated with
//      with_temp_directory + a "db.tar" inside: the callback sees a path in
//      a fresh temp dir that is deleted afterwards. The generated name is
//      observable only to the callback, which passes it straight on.
//   3. userError is rendered the way GHC's show renders it: the io::Error
//      carries "user error (<msg>)", so a caller logging show (exc) sees the
//      source's text. (io::Error::other(msg) alone would not.)
//   4. findExecutable is a PATH scan for a regular file. The Haskell also
//      requires the executable bit, so a non-executable PATH hit differs;
//      tar would then fail to spawn rather than report "Cannot find tar
//      executable".
//   5. mask_ (:141) has no counterpart: no async exceptions in Rust, and the
//      out-param is read by the invoke block immediately before from_raw.
//   6. fromEnum of the cache flag (:127) is a CBool (4-byte CInt) in the
//      source while the C++ parameter is bool (ffi.h:40). The port passes a
//      Rust bool (1 byte), matching the C++ declaration; the Haskell works
//      only because the low byte of the int is what the callee reads.
//   7. Stderr decoding follows the source per call site. unTarFromHandle
//      reads a ByteString and uses BS.Char8.unpack (:246), i.e. one char
//      per byte, so latin1() maps the bytes 1:1. tar gets a String from
//      readProcessWithExitCode, which the process package decodes with the
//      handle's locale encoding; the port uses from_utf8_lossy (UTF-8,
//      invalid bytes replaced rather than raising a decode error).
//
// ---------------------------------------------------------------------------
// SIBLING-UNIT CONTRACTS
// ---------------------------------------------------------------------------
//
//   * row 67 (storage/db.rs): DB::from_raw is the ONLY database-finalizer
//     door; Container's Static impl frees the container; checkpoint and
//     DB::backup are the two backup producers this unit calls.
//   * row 70 (storage/lmdb.rs): the same open/capacity/restore shape; its
//     three seams plus take_directory (promoted to storage.rs here) are
//     reused rather than re-implemented.
//   * rows 68/69: newStorage's memory capacity and the capacity queries.
//   * row 104 (database/types.rs) owns the closed Storage enum; row 85
//     (Config) maps new_storage's error into its own channel.
//
// MARKERS: zero port todos, zero source-defect reproductions, zero
// performance deferrals, zero inventory divergences.
// SAFETY count: 11 (each unsafe call site carries its own SAFETY note;
// the Object impl is the only unsafe impl).

use std::ffi::CString;
use std::io::Read;
use std::os::raw::{c_char, c_int, c_void};
use std::path::Path;
use std::process::{Command, Stdio};

use glean_core::ffi::{create, invoke, using, Destroy, FfiError, ForeignPtr, Object};
use glean_core::rts::foreign::fact_set::FactSet;
use glean_core::rts::foreign::inventory::Inventory;
use glean_core::rts::foreign::lookup::{CanLookup, Lookup};
use glean_core::rts::foreign::ownership::{
    next_uset_id, ComputedOwnership, DefineOwnership, Ownership, UnitId, FIRST_USET_ID,
};
use glean_core::rts::types::{invalid_fid, Fid, Pid};
use glean_thrift as thrift;
use glean_util::log::log_info;
use glean_util::r#impl::memory_reader::total_mem_capacity_kb;
use glean_util::util::disk::{get_disk_size, get_free_disk_space, get_used_disk_space};

use crate::database::backup::backend::{Data, RestoreSource};
use crate::database::exception::DbError;
use crate::database::repo::{combine, database_path};
use crate::database::storage::db::{checkpoint, Container, DB};
use crate::database::storage::{
    get_file_size, take_directory, with_temp_directory, AxiomOwnership, DBVersion, DatabaseOps,
    Mode, Storage, WriteLock,
};

// ---------------------------------------------------------------------------
// Cache (RocksDB.hs:52-65)
// ---------------------------------------------------------------------------

/// 'newtype Cache = Cache (ForeignPtr Cache)' (:52) with
/// 'instance Object Cache where destroy = glean_rocksdb_free_cache'
/// (:54-57). The marker type names the C++ SharedCache* (ffi.h:31); the
/// pointer is never dereferenced on the Rust side.
pub struct Cache {
    fp: ForeignPtr<Cache>,
}

// SAFETY: destroy() is the finalizer the source attaches to this pointer
// (glean_rocksdb_free_cache, ffi.h:34), and wrap/unwrap round-trip over the
// same clone-shared handle. The only door onto a live handle is new_cache.
unsafe impl Object for Cache {
    fn destroy() -> Destroy<Self> {
        destroy_cache
    }

    fn wrap(fp: ForeignPtr<Self>) -> Self {
        Cache { fp }
    }

    fn unwrap(&self) -> ForeignPtr<Self> {
        self.fp.clone()
    }
}

unsafe extern "C" fn destroy_cache(p: *mut Cache) {
    // SAFETY: p is the SharedCache* adopted at construction; ForeignPtr's
    // cell guarantees at-most-once execution, and glean_rocksdb_free_cache
    // (rocksdb/ffi.cpp:35-37, ffi::free_) cannot throw across.
    unsafe { sys::glean_rocksdb_free_cache(p.cast()) }
}

/// 'newCache size = construct $ invoke $ glean_rocksdb_new_cache
/// (fromIntegral size)' (:60-61).
fn new_cache(size: i64) -> Result<Cache, FfiError> {
    let mut ptr: *mut c_void = std::ptr::null_mut();
    // SAFETY: the out-param is a valid local; on success the C++ body stores
    // the one newly-allocated SharedCache* there (rocksdb/ffi.cpp:30-33).
    unsafe {
        invoke(|| sys::glean_rocksdb_new_cache(size as usize, &mut ptr))?;
        Ok(create::<Cache>(ptr.cast()))
    }
}

/// 'withCache (Just cache) f = with cache f; withCache Nothing f = f
/// nullPtr' (:63-65).
fn with_cache<R>(cache: Option<&Cache>, f: impl FnOnce(*mut c_void) -> R) -> R {
    match cache {
        // SAFETY: the handle keeps the finalizer alive across f; f must not
        // retain the pointer (the same obligation as withForeignPtr).
        Some(cache) => unsafe { cache.fp.with_raw(|p| f(p.cast())) },
        None => f(std::ptr::null_mut()),
    }
}

// ---------------------------------------------------------------------------
// RocksDB (RocksDB.hs:67-115)
// ---------------------------------------------------------------------------

/// 'data RocksDB = RocksDB { rocksRoot, rocksCache, ... }' (:67-79).
pub struct RocksDB {
    /// 'rocksRoot'.
    pub rocks_root: String,
    /// 'rocksCache' — Nothing when the computed size is <= 0 (:100-102).
    pub rocks_cache: Option<Cache>,
    /// 'rocksCacheIndexAndFilterBlocks'.
    pub rocks_cache_index_and_filter_blocks: bool,
    /// 'rocksMaxDiskSize' — the virtual limit to REPORT; not enforced.
    pub rocks_max_disk_size: Option<i64>,
}

/// 'cacheSize :: Config -> Maybe Int -> Int' (:86-90).
///
/// The ratio branch is 'round $ fromIntegral ramKB * ratio * 1024': the
/// multiplication happens in f32 (thrift float) and Haskell's round is
/// ties-to-EVEN, hence round_ties_even. The fallback is
/// 'cache_mb * 1024 * 1024'.
///
/// Degenerate ratios are the one place the two languages part: a NaN or
/// infinite product makes Haskell's round raise, and an out-of-range one
/// wraps the Int, while 'as i64' yields 0 for NaN and saturates. A server
/// config that asks for a non-finite cache size has no sensible answer in
/// either language.
fn cache_size(config: &thrift::Config, mem_capacity_kb: Option<i64>) -> i64 {
    match (mem_capacity_kb, config.db_rocksdb_cache_to_mem_ratio) {
        (Some(ram_kb), Some(ratio)) => (ram_kb as f32 * ratio * 1024.0).round_ties_even() as i64,
        _ => config.db_rocksdb_cache_mb as i64 * 1024 * 1024,
    }
}

/// 'newStorage root config' (:92-115).
pub fn new_storage(root: &str, config: &thrift::Config) -> Result<RocksDB, FfiError> {
    let mem_capacity = total_mem_capacity_kb();
    let cache_size = cache_size(config, mem_capacity);
    let cache = if cache_size > 0 {
        Some(new_cache(cache_size)?)
    } else {
        None
    };
    Ok(RocksDB {
        rocks_root: root.to_string(),
        rocks_cache: cache,
        rocks_cache_index_and_filter_blocks: config.db_rocksdb_cache_index_and_filter_blocks,
        // Same arithmetic as row 70's newStorage (GHC Int wraps).
        rocks_max_disk_size: match mem_capacity {
            Some(mem) => config
                .db_rocksdb_disk_mem_capacity_ratio_limit
                .map(|ratio| mem.wrapping_mul(1024).wrapping_mul(ratio as i64)),
            None => None,
        },
    })
}

/// 'newtype instance Database RocksDB = Database DB deriving (CanLookup)'
/// (:117) — the associated type of Storage.
pub struct Database {
    db: DB,
}

// ---------------------------------------------------------------------------
// CanLookup (derived instance, RocksDB.hs:117)
// ---------------------------------------------------------------------------

impl CanLookup for Database {
    fn with_lookup<T, E: From<FfiError>>(
        &self,
        f: impl FnOnce(*mut c_void) -> Result<T, E>,
    ) -> Result<T, E> {
        self.db.with_lookup(f)
    }

    fn lookup_name(&self) -> &str {
        self.db.lookup_name()
    }
}

// ---------------------------------------------------------------------------
// Storage (RocksDB.hs:119-182)
// ---------------------------------------------------------------------------

impl Storage for RocksDB {
    type Database = Database;

    /// 'describe rocks = "rocksdb:" <> rocksRoot rocks' (:120).
    fn describe(&self) -> String {
        format!("rocksdb:{}", self.rocks_root)
    }

    /// 'readableVersions _ = [DBVersion 3]' (:122).
    fn readable_versions(&self) -> Vec<DBVersion> {
        vec![3]
    }

    /// 'writableVersions _ = [DBVersion 3]' (:123).
    fn writable_versions(&self) -> Vec<DBVersion> {
        vec![3]
    }

    /// 'open rocks repo mode (DBVersion version)' (:125-140).
    fn open(
        &self,
        repo: &thrift::Repo,
        mode: Mode,
        version: DBVersion,
    ) -> Result<Database, DbError> {
        let path = container_path(self, repo);
        let (cmode, start, ownership) = match mode {
            // ReadOnly = 0 (storage/db.h:82). No squashfs handling here.
            Mode::ReadOnly => (0, invalid_fid(), None),
            // ReadWrite = 1.
            Mode::ReadWrite => (1, invalid_fid(), None),
            Mode::Create {
                first_id,
                ownership,
                ..
            } => {
                std::fs::create_dir_all(&path)?;
                // Create = 2.
                (2, first_id, ownership)
            }
        };
        let cpath = c_string(&path);
        let cache_index = self.rocks_cache_index_and_filter_blocks;
        with_cache(self.rocks_cache.as_ref(), |cache_ptr| {
            let mut container_ptr: *mut c_void = std::ptr::null_mut();
            // SAFETY: cpath is a live NUL-terminated string for the call,
            // cache_ptr is either a live SharedCache* or null (the C++
            // accepts null, rocksdb/container-impl.cpp), and the out-param
            // is a valid local.
            unsafe {
                invoke(|| {
                    sys::glean_rocksdb_container_open(
                        cpath.as_ptr(),
                        cmode,
                        cache_index,
                        cache_ptr,
                        &mut container_ptr,
                    )
                })?;
            }
            // SAFETY: non-null on success; using frees it with
            // glean_rocksdb_container_free exactly once on every exit path.
            let container = Container(container_ptr);
            using(
                || container,
                |container| -> Result<Database, DbError> {
                    // maybe (return firstUsetId) nextUsetId ownership (:143).
                    let first_unit_id = match &ownership {
                        Some(ownership) => next_uset_id(ownership)?.0,
                        None => FIRST_USET_ID.0,
                    };
                    let mut db_ptr: *mut c_void = std::ptr::null_mut();
                    // SAFETY: the container is live for this scope and the
                    // out-param receives the one newly-allocated Database*
                    // on success (rocksdb/ffi.cpp:60-70).
                    unsafe {
                        invoke(|| {
                            sys::glean_rocksdb_container_open_database(
                                container.0,
                                start,
                                first_unit_id,
                                version,
                                &mut db_ptr,
                            )
                        })?;
                    }
                    // SAFETY: db_ptr is the live, uniquely-owned Database*
                    // from the successful call; DB::from_raw adopts it and
                    // its Db marker is the one finalizer (row 67).
                    let db = unsafe { DB::from_raw(db_ptr, repo.clone()) };
                    Ok(Database { db })
                },
            )
        })
    }

    /// 'delete rocks = safeRemovePathForcibly . containerPath rocks' (:144).
    fn delete(&self, repo: &thrift::Repo) -> Result<(), DbError> {
        Ok(crate::database::backup::backend::remove_path_forcibly(
            &container_path(self, repo),
        )?)
    }

    /// 'safeRemoveForcibly rocks = ... databasePath (rocksRoot rocks)'
    /// (:146-147).
    fn safe_remove_forcibly(&self, repo: &thrift::Repo) -> Result<(), DbError> {
        Ok(crate::database::backup::backend::remove_path_forcibly(
            &database_path(&self.rocks_root, repo),
        )?)
    }

    /// 'getTotalCapacity rocksdb' (:149-157).
    fn get_total_capacity(&self) -> Result<Option<i64>, DbError> {
        let exists = Path::new(&self.rocks_root).is_dir();
        if exists {
            let full_disk_capacity = get_disk_size(&self.rocks_root)?;
            Ok(Some(match self.rocks_max_disk_size {
                Some(max_disk_size) => max_disk_size.min(full_disk_capacity),
                None => full_disk_capacity,
            }))
        } else {
            Ok(None)
        }
    }

    /// 'getUsedCapacity rocksdb' (:159-163).
    fn get_used_capacity(&self) -> Result<Option<i64>, DbError> {
        let exists = Path::new(&self.rocks_root).is_dir();
        if exists {
            Ok(Some(get_used_disk_space(&self.rocks_root)?))
        } else {
            Ok(None)
        }
    }

    /// 'getFreeCapacity rocksdb' (:165-171). The fallback is unguarded in
    /// the source, so a missing root surfaces the IOException (row 69's
    /// contract).
    fn get_free_capacity(&self) -> Result<i64, DbError> {
        let used = self.get_used_capacity()?;
        let total = self.get_total_capacity()?;
        match (used, total) {
            (Some(used), Some(total)) => Ok(total - used),
            _ => Ok(get_free_disk_space(&self.rocks_root)?),
        }
    }

    /// 'withScratchRoot rocks f = f $ rocksRoot rocks </> ".scratch"' (:172).
    fn with_scratch_root<T, F>(&self, action: F) -> Result<T, DbError>
    where
        F: FnOnce(&str) -> Result<T, DbError>,
    {
        action(&combine(&self.rocks_root, ".scratch"))
    }

    /// 'restore rocks _ repo scratch src' (:174-182).
    fn restore(
        &self,
        _config: &thrift::Config,
        repo: &thrift::Repo,
        scratch: &str,
        source: RestoreSource,
    ) -> Result<(), DbError> {
        with_temp_directory(scratch, "restore", |scratch_restore| {
            match source {
                RestoreSource::File(path) => un_tar(&path, scratch_restore)?,
                RestoreSource::Stream(handle) => un_tar_from_handle(handle, scratch_restore)?,
            }
            restore_from_dir(self, repo, scratch, scratch_restore)
        })
    }
}

// ---------------------------------------------------------------------------
// DatabaseOps (RocksDB.hs:184-220)
// ---------------------------------------------------------------------------

impl DatabaseOps for Database {
    /// 'close (Database db) = close db' (:185).
    fn close(&self) -> Result<(), DbError> {
        self.db.close()
    }

    fn predicate_stats(&self) -> Result<Vec<(Pid, thrift::PredicateStats)>, DbError> {
        self.db.predicate_stats()
    }

    fn store(&self, key: &[u8], value: &[u8]) -> Result<(), DbError> {
        self.db.store(key, value)
    }

    fn retrieve(&self, key: &[u8]) -> Result<Option<Vec<u8>>, DbError> {
        self.db.retrieve(key)
    }

    fn add_batch_descriptor(&self, descriptor: &thrift::BatchDescriptor) -> Result<(), DbError> {
        self.db.add_batch_descriptor(descriptor)
    }

    fn mark_batch_descriptor_as_written(&self, location: &String) -> Result<(), DbError> {
        self.db.mark_batch_descriptor_as_written(location)
    }

    fn is_batch_descriptor_stored(&self, location: &String) -> Result<bool, DbError> {
        self.db.is_batch_descriptor_stored(location)
    }

    fn get_unprocessed_batch_descriptors(&self) -> Result<Vec<thrift::BatchDescriptor>, DbError> {
        self.db.get_unprocessed_batch_descriptors()
    }

    fn commit(&self, facts: &FactSet) -> Result<(), DbError> {
        self.db.commit(facts)
    }

    fn add_ownership<W>(
        &self,
        lock: &WriteLock<W>,
        ownership: AxiomOwnership,
    ) -> Result<(), DbError> {
        self.db.add_ownership(lock, ownership)
    }

    fn register_acl_units<W>(
        &self,
        lock: &WriteLock<W>,
        names: &[Vec<u8>],
    ) -> Result<UnitId, DbError> {
        self.db.register_acl_units(lock, names)
    }

    fn optimize(&self, compact: bool) -> Result<(), DbError> {
        self.db.optimize(compact)
    }

    fn flush(&self) -> Result<(), DbError> {
        self.db.flush()
    }

    fn compute_ownership(
        &self,
        base: Option<&Lookup>,
        inventory: &Inventory,
    ) -> Result<ComputedOwnership, DbError> {
        self.db.compute_ownership(base, inventory)
    }

    fn store_ownership<W>(
        &self,
        lock: &WriteLock<W>,
        ownership: &ComputedOwnership,
    ) -> Result<(), DbError> {
        self.db.store_ownership(lock, ownership)
    }

    fn get_ownership(&self) -> Result<Option<Ownership>, DbError> {
        self.db.get_ownership()
    }

    fn get_unit_id(&self, unit: &[u8]) -> Result<Option<UnitId>, DbError> {
        self.db.get_unit_id(unit)
    }

    fn get_unit(&self, id: UnitId) -> Result<Option<Vec<u8>>, DbError> {
        self.db.get_unit(id)
    }

    fn get_units_by_prefix(&self, prefix: &[u8]) -> Result<Vec<(Vec<u8>, UnitId)>, DbError> {
        self.db.get_units_by_prefix(prefix)
    }

    fn add_define_ownership<W>(
        &self,
        lock: &WriteLock<W>,
        define: &DefineOwnership,
    ) -> Result<(), DbError> {
        self.db.add_define_ownership(lock, define)
    }

    fn compute_derived_ownership<W>(
        &self,
        lock: &WriteLock<W>,
        ownership: &Ownership,
        base: Option<&Lookup>,
        pid: Pid,
    ) -> Result<ComputedOwnership, DbError> {
        self.db
            .compute_derived_ownership(lock, ownership, base, pid)
    }

    fn cache_ownership(&self) -> Result<(), DbError> {
        self.db.cache_ownership()
    }

    fn prepare_fact_owner_cache(&self) -> Result<(), DbError> {
        self.db.prepare_fact_owner_cache()
    }

    /// 'backup (Database db) cfg scratch process' (:208-220): the two
    /// producers (checkpoint vs BackupEngine) both hand a tar path to
    /// mkTar, which tars the path's basename into a temp file.
    fn backup<A, F>(&self, config: &thrift::Config, scratch: &str, action: F) -> Result<A, DbError>
    where
        F: FnOnce(&str, Data) -> Result<A, DbError>,
    {
        let mk_tar = |path: &str, _data: Data| -> Result<A, DbError> {
            let (base, dir) = split_file_name(path);
            let parent = std::env::temp_dir();
            with_temp_directory(&parent.to_string_lossy(), "extra", |tmpdir| {
                let tar_file = combine(tmpdir, "db.tar");
                tar(&["-cf", &tar_file, "-C", &base, &dir])?;
                let size = get_file_size(&tar_file)?;
                action(&tar_file, Data { data_size: size })
            })
        };
        if config.db_backup_use_checkpoint {
            log_info(|| "backup: packing as RocksDB checkpoint (db/ tarball)".to_string());
            checkpoint(&self.db, scratch, mk_tar)
        } else {
            log_info(|| "backup: packing as BackupEngine backup (backup/ tarball)".to_string());
            self.db.backup(config, scratch, mk_tar)
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers (RocksDB.hs:224-291)
// ---------------------------------------------------------------------------

/// 'unTar scratch_file scratch_restore = tar ["-xf", file, "-C", dir]'
/// (:224-226).
fn un_tar(scratch_file: &str, scratch_restore: &str) -> Result<(), DbError> {
    Ok(tar(&["-xf", scratch_file, "-C", scratch_restore])?)
}

/// 'unTarFromHandle h dir' (:230-253): extract a tar stream read from a
/// handle, draining stderr FULLY before waiting so a chatty tar cannot block
/// on a full pipe (the source's comment, :243-245).
fn un_tar_from_handle(handle: std::fs::File, dir: &str) -> Result<(), DbError> {
    let tar_path =
        find_executable("tar").ok_or_else(|| user_error("Cannot find tar executable"))?;
    let mut child = Command::new(&tar_path)
        .arg("-x")
        .arg("-C")
        .arg(dir)
        .stdin(Stdio::from(handle))
        .stdout(Stdio::inherit())
        .stderr(Stdio::piped())
        .spawn()?;
    let mut err = Vec::new();
    let read_result = match child.stderr.take() {
        Some(mut pipe) => pipe.read_to_end(&mut err),
        // withCreateProcess's CreatePipe always produces one; the source
        // throws here too (:238-239). Reap the child like the source's
        // bracket-on-exception would.
        None => {
            let _ = child.wait();
            return Err(user_error("tar: stderr pipe was not created").into());
        }
    };
    // A read failure still reaps the child (withCreateProcess guarantees
    // that on every exit path), but the read error is what the source
    // reports, so it wins over a wait failure.
    if let Err(e) = read_result {
        let _ = child.wait();
        return Err(e.into());
    }
    let status = child.wait()?;
    if !status.success() {
        return Err(user_error(&format!("tar -x from stream failed: {}", latin1(&err))).into());
    }
    Ok(())
}

/// 'restoreFromDir rocks repo scratch scratch_restore' (:255-278).
fn restore_from_dir(
    rocks: &RocksDB,
    repo: &thrift::Repo,
    scratch: &str,
    scratch_restore: &str,
) -> Result<(), DbError> {
    // A "backup/" entry means a RocksDB backup; "db/" means a plain tarball.
    let scratch_restore_backup = combine(scratch_restore, "backup");
    let db = if Path::new(&scratch_restore_backup).is_dir() {
        let scratch_db = combine(scratch, "db");
        std::fs::create_dir_all(&scratch_db)?;
        let target = c_string(&scratch_db);
        let source = c_string(&scratch_restore_backup);
        // SAFETY: both strings are live NUL-terminated paths for the call;
        // glean_rocksdb_restore copies what it needs (rocksdb/ffi.cpp:79-90).
        unsafe {
            invoke(|| sys::glean_rocksdb_restore(target.as_ptr(), source.as_ptr()))?;
        }
        scratch_db
    } else {
        let scratch_restore_db = combine(scratch_restore, "db");
        if Path::new(&scratch_restore_db).is_dir() {
            scratch_restore_db
        } else {
            return Err(user_error("unrecognised backup").into());
        }
    };
    let target = container_path(rocks, repo);
    std::fs::create_dir_all(take_directory(&target))?;
    std::fs::rename(&db, &target)?;
    Ok(())
}

/// 'tar args' (:280-287): find tar, run it, and on failure throw the
/// process's stderr as the userError message.
fn tar(args: &[&str]) -> Result<(), DbError> {
    let tar_path =
        find_executable("tar").ok_or_else(|| user_error("Cannot find tar executable"))?;
    let output = Command::new(&tar_path).args(args).output()?;
    if !output.status.success() {
        // readProcessWithExitCode hands back a String, decoded with the
        // handle's locale encoding (DIVERGENCES 7).
        return Err(user_error(&String::from_utf8_lossy(&output.stderr)).into());
    }
    Ok(())
}

/// 'containerPath RocksDB{..} repo = databasePath rocksRoot repo </> "db"'
/// (:289-290).
fn container_path(rocks: &RocksDB, repo: &thrift::Repo) -> String {
    combine(&database_path(&rocks.rocks_root, repo), "db")
}

/// 'getCacheCapacity rocks' (:299-302): withCache passes null when there is
/// no cache, and glean_rocksdb_cache_capacity returns 0 for null.
pub fn get_cache_capacity(rocks: &RocksDB) -> i64 {
    with_cache(rocks.rocks_cache.as_ref(), |cache| {
        // SAFETY: cache is either a live SharedCache* or null, both accepted
        // by rocksdb/ffi.cpp:39-44.
        unsafe { sys::glean_rocksdb_cache_capacity(cache.cast()) as i64 }
    })
}

/// System.Directory.findExecutable: the first PATH entry holding a regular
/// file of that name (DIVERGENCES 4).
fn find_executable(name: &str) -> Option<String> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
        .map(|candidate| candidate.to_string_lossy().into_owned())
}

/// System.FilePath.splitFileName: (directory WITH its trailing separator,
/// file name); "./" when there is no separator at all.
fn split_file_name(path: &str) -> (String, String) {
    match path.rfind('/') {
        Some(i) => (path[..=i].to_string(), path[i + 1..].to_string()),
        None => ("./".to_string(), path.to_string()),
    }
}

/// GHC's userError, rendered the way show (SomeException e) renders it
/// (DIVERGENCES 3).
fn user_error(msg: &str) -> std::io::Error {
    std::io::Error::other(format!("user error ({})", msg))
}

/// BS.Char8.unpack (unTarFromHandle's stderr only): one char per byte, so
/// the stream path's tar text is preserved byte for byte (DIVERGENCES 7).
fn latin1(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| b as char).collect()
}

/// withCString for a path: base truncates at an embedded NUL rather than
/// raising (row 70's translation note), so this does the same.
fn c_string(s: &str) -> CString {
    let bytes = s.as_bytes();
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    CString::new(&bytes[..end]).expect("no NUL remains after truncation")
}

// ---------------------------------------------------------------------------
// The rocksdb externs (RocksDB.hs:292-325)
// ---------------------------------------------------------------------------

mod sys {
    use super::{c_char, c_int, c_void, Fid};

    // SAFETY: these are the rocksdb/ffi.h declarations (ffi.h:33-51). Each is
    // called only with live handles or null where the C++ accepts null, a
    // valid out-param, and NUL-terminated paths; the const char* result is an
    // error string (null on success) consumed by invoke.
    unsafe extern "C" {
        pub fn glean_rocksdb_new_cache(capacity: usize, cache: *mut *mut c_void) -> *const c_char;

        pub fn glean_rocksdb_free_cache(cache: *mut c_void);

        pub fn glean_rocksdb_cache_capacity(cache: *mut c_void) -> usize;

        pub fn glean_rocksdb_container_open(
            path: *const c_char,
            mode: c_int,
            cache_index_and_filter_blocks: bool,
            cache: *mut c_void,
            container: *mut *mut c_void,
        ) -> *const c_char;

        pub fn glean_rocksdb_container_open_database(
            container: *mut c_void,
            start: Fid,
            first_unit_id: u32,
            version: i64,
            db: *mut *mut c_void,
        ) -> *const c_char;

        pub fn glean_rocksdb_restore(target: *const c_char, source: *const c_char)
            -> *const c_char;
    }
}

// PORT STATUS: confidence=high todos=0
