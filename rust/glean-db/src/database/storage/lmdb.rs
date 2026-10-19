// Ported from: glean/db/Glean/Database/Storage/LMDB.hs (207 lines).
// Manifest unit: rust/glean-db/src/database/storage/lmdb.rs
// (migration/manifest.tsv line 70, crate glean-db). RULE §4: one source
// file -> one target file; the manifest path is authoritative.
//
// WHAT THIS FILE IS: the LMDB backend of the Storage class. One LMDB
// directory (or squashfs image) per repo; the open database reuses the
// RocksDB handle type DB (row 67) and the shared glean_rocksdb_database_free
// finalizer. Contents: the LMDB record (:44-51), newStorage (:53-62), the
// data-family instance (:64), CanLookup (:66-68), the Storage instance
// (:70-147), containerPath (:149-150), the DatabaseOps instance (:152-191)
// and the two lmdb externs (:193-205).
//
// ---------------------------------------------------------------------------
// INVENTORY (RULE §5; migration/inventory*.tsv)
// ---------------------------------------------------------------------------
//
//   * inventory.tsv:29 and inventory.ffi.tsv:24 — the same OWNED_MUTABLE
//     row, confirmed in both files; the inventory.tsv copy additionally
//     carries "[SUPERSEDED by §1/A2: raw extern-C — 2026-10-07]" on the
//     bridge column, and the ffi copy does not. Quoting the tail of the
//     target_translation cell: "Database<LMDB> owned by a guard whose
//     Drop = glean_rocksdb_database_free — keep ONE free symbol for both
//     backends in the bridge (source already shares it); open failure ->
//     precondition-guard Err, container freed by `using`; squashfs
//     mount/unsquash/mksquashfs stay std::process::Command". Followed
//     exactly: this file declares NO finalizer; the open path hands the
//     Database* to DB::from_raw (row 67), whose Db marker owns
//     destroy_database -> glean_rocksdb_database_free. The container is
//     destroyed by `using`; the four process calls stay Command (via the
//     call_process seam).
//   * inventory.sites.tsv:52-54 — the three foreign imports at LMDB.hs:193,
//     :199 and :206. Two are declared in the sys block below; the third
//     (:206, the shared free symbol) is row 67's destroy_database, reached
//     through DB::from_raw rather than declared again.
//   * No other row names this file; the broad
//     `grep -n 'Storage/LMDB' migration/*.tsv` returns exactly six lines:
//     the three above, inventory.tsv:29, inventory.ffi.tsv:24 (the same
//     OWNED_MUTABLE row repeated) and manifest.tsv:70 (this unit's row).
//
// ---------------------------------------------------------------------------
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, checked not assumed)
// ---------------------------------------------------------------------------
//
//   * OUTBOUND (10), all shipped:
//       edges.tsv:420 Backup/Backend (line 56) — Data;
//       :421 Database/Repo (52) — databasePath;
//       :422 Database/Storage (58) — the traits, Mode, WriteLock;
//       :423 Database/Storage/DB (67) — DB, Container, the shared free;
//       :424 Glean/FFI (the Util.FFI re-export; assumed, see below);
//       :425 RTS/Foreign/Lookup (19) — CanLookup;
//       :426 RTS/Foreign/Ownership (57) — Ownership, nextUsetId, UsetId;
//       :427 RTS/Types (11) — Fid, invalidFid;
//       :428 Impl/MemoryReader (68) — totalMemCapacityKB;
//       :429 Util/Disk (69) — the three capacity queries.
//   * INBOUND (1): edges.tsv:223 Database/Config.hs (manifest line 85, NOT
//     shipped) builds LMDB through newStorage.
//   * External, no manifest unit and no edge: Util.FFI and Util.IO (neither
//     module is in the OSS tree), System.IO.Temp, System.Directory,
//     System.Process, System.FilePath, Data.Int, Data.Text, plus the
//     imports the port satisfies from std: Control.Exception (bracket,
//     catch), Control.Monad (when, unless), Foreign.C.String,
//     Foreign.C.Types, Foreign.ForeignPtr, Foreign.Ptr.
//
// ---------------------------------------------------------------------------
// ASSUMED IMPORTS (RULE §4 B4)
// ---------------------------------------------------------------------------
//
//   * glean_thrift::{Config, Repo, DBVersion}. DBVersion is
//     `typedef i64 DBVersion (hs.newtype)` (server_config.thrift:28) and
//     collapses to `i64` (storage.rs:147-150; re-export at :201). The Config fields this unit
//     reads keep their THRIFT names in the generated Rust struct — the
//     source's `config_` prefix is hsthrift's record-field prefix:
//       db_rocksdb_disk_mem_capacity_ratio_limit — server_config.thrift:327,
//         `optional i32` -> Option<i32> (newStorage, :60);
//       db_lmdb_mksquashfs_args — :335, `list<string>` -> Vec<String>
//         (backup, :189);
//       db_lmdb_restore_unpack — :346, `bool` -> bool (restore, :140).
//   * Util.FFI (invoke/using) is not in the OSS tree. The shipped
//     glean_core::ffi carries both with the same semantics (row 67's
//     precedent); the externs below are declared here.
//   * Util.IO.safeRemovePathForcibly is not in the OSS tree either; the
//     shipped image is database/backup/backend.rs's remove_path_forcibly
//     (row 56; the same mapping rows 64 and 67 use).
//   * System.IO.Temp.withTempDirectory, System.Directory.getFileSize and
//     System.Process.callProcess have no manifest unit; this unit added
//     them as pub(crate) seams in database/storage.rs (see that file's
//     "ROW 70'S SEAMS" section).
//
// ---------------------------------------------------------------------------
// TRANSLATION NOTES
// ---------------------------------------------------------------------------
//
//   * :44-51 record -> pub struct LMDB { lmdb_root: String,
//     lmdb_max_disk_size: Option<i64> }, Clone (the Database keeps one).
//   * :53-62 newStorage: totalMemCapacityKB (row 68) then
//     `(* (mem * 1024)) . fromIntegral <$> Maybe Int` — Nothing when either
//     side is Nothing; wrapping_mul keeps GHC's silent Int wrap.
//   * :64 `data instance Database LMDB = Database DB LMDB` -> pub struct
//     Database { db: DB, lmdb: LMDB } (the associated type of Storage).
//   * :66-68 CanLookup forwards to the DB, so lookupName stays
//     "rocksdb:<repo>" for LMDB too — DB.hs:53 hardcodes that prefix and
//     this unit inherits it (DIVERGENCES 2).
//   * :70-147 Storage. describe (:71) prefixes "lmdb:"; both version lists
//     are [3] (:73-75, the source's own comment says RocksDB started at 3).
//   * :77-103 open: mode -> (cmode, start, ownership) with cmode 0/1/2
//     matching storage/db.h:82's `enum class Mode`; ReadOnly checks
//     "data.mdb", mounts the squashfs with squashfuse_ll when it is absent,
//     ReadWrite is a no-op, Create creates the directory and passes
//     (start, ownership) through. Then the FFI: withCString ->
//     CString::new over the bytes up to the first NUL (base truncates at an
//     embedded NUL rather than raising), `using (invoke open)` -> invoke +
//     using, the out-param read into a Container. Inside: maybe firstUsetId
//     nextUsetId ownership ->
//     next_uset_id (ownership.rs:1100, an FFI call) or FIRST_USET_ID, then
//     open_database, then newForeignPtr glean_rocksdb_database_free ->
//     DB::from_raw.
//   * :105 delete / :107-108 safeRemoveForcibly -> remove_path_forcibly on
//     containerPath / databasePath respectively.
//   * :110-131 capacities: getDiskSize/getUsedDiskSpace/getFreeDiskSpace
//     (row 69) with the source's doesDirectoryExist guards and the
//     min / "total - used" arithmetic; the free-capacity fallback is
//     unguarded, exactly as in the source (row 69's header records it).
//   * :133 withScratchRoot -> action(combine(root, ".scratch")).
//   * :135-147 restore: withTempDirectory "restore", then
//     withMaterializedFile, createDirectoryIfMissing (takeDirectory target),
//     and the unpack / renameFile split on db_lmdb_restore_unpack.
//   * :152-191 DatabaseOps: every method forwards to the DB except close
//     (:153-158, which closes the DB and then umounts the squashfs,
//     swallowing an IOException) and backup (:184-191, which asks the DB
//     for a checkpoint directory, mksquashfs's it, and hands the file plus
//     its size to the continuation).
//   * :193-205 the two externs. glean_fact_id_t is int64_t (rts/ffi.h:105),
//     so `start: Fid` is ABI-correct (Fid is repr(transparent) over i64);
//     first_unit_id is uint32_t.
//
// ---------------------------------------------------------------------------
// SOURCE DEFECT (reproduced)
// ---------------------------------------------------------------------------
//
// BUG(port): :80 tests `doesDirectoryExist (path </> "data.mdb")`, but
// data.mdb is a FILE, so the predicate is always False and the
// squashfs-mount branch runs even when an unpacked database already exists.
// The intended check is clearly doesFileExist (the sibling haveSquash
// uses it). Repro: a repo directory containing both data.mdb and
// db.squashfs still runs squashfuse_ll on open (the mount then shadows the
// unpacked directory or fails). Reproduced verbatim as Path::is_dir();
// RULE §2's bug rule — fixes only as flagged post-parity changes.
//
// ---------------------------------------------------------------------------
// DIVERGENCES
// ---------------------------------------------------------------------------
//
//   1. mask_ (:94) has no Rust counterpart: it keeps an async exception
//      from landing between open_database and newForeignPtr, and Rust has
//      no async exceptions. The out-param is read by the invoke block and
//      DB::from_raw adopts it in the next statement, so the window is
//      absent by construction rather than closed by a mask.
//   2. NOT a divergence, recorded so row 72 does not "fix" it: lookupName
//      is "rocksdb:<repo>" for LMDB as well — the source's DB.hs:53
//      hardcodes the prefix for both backends, row 67 reproduces it, and
//      this unit forwards it.
//   3. Temp-directory names are "restore<unique>"/"out<unique>" (the
//      process-global counter) instead of mkdtemp's "restoreXXXXXX"; only
//      the returned path is observable and no caller parses it.
//   4. getFileSize's Integer is narrowed to i64 (u64 st_size -> i64);
//      only files above 2^63 bytes would differ.
//   5. callProcess's Show-quoted arguments are reproduced with the Debug
//      formatter, which differs from Haskell's show for non-ASCII input
//      (Haskell renders a numeric escape, Rust the character) and for
//      control characters (Haskell uses mnemonics such as \SOH, Rust
//      \u{1}).
//   6. takeDirectory is a local copy of the glean-util port
//      (util/io.rs:402-420), which is private there. It matches
//      System.FilePath for every path this unit builds ("<root>/<repo>/db");
//      the two differ only for an all-separator prefix, where filepath keeps
//      the separators (takeDirectory "//foo" = "//", "///" = "///") and the
//      port collapses to "/".
//   7. callProcess's delegate_ctlc is not reproduced (the call_process
//      seam uses Command::status). The child stays in the parent's process
//      group in BOTH implementations — callProcess sets only delegate_ctlc
//      and proc's create_group defaults to False — so the terminal delivers
//      SIGINT to both. The difference is the parent: GHC sets SIG_IGN on
//      SIGINT/SIGQUIT for the duration of the wait and, if the child then
//      died from that signal, raises UserInterrupt afterwards
//      (process-1.6.17.0's "delegate control-C" note); Rust keeps the
//      default disposition, so a Ctrl-C during mksquashfs/squashfuse_ll
//      ends the parent at once rather than after the child, and without
//      converting it into an orderly shutdown.
//   8. newStorage drops the source's IO wrapper (:53). totalMemCapacityKB
//      is total IO (row 68's decision: no Result), so the only IO in
//      newStorage is a call that cannot fail; the signature is therefore
//      `fn new_storage(root, config) -> LMDB`. Row 72 inherits the same
//      shape.
//
// ---------------------------------------------------------------------------
// SIBLING-UNIT CONTRACTS
// ---------------------------------------------------------------------------
//
//   * row 58 (storage.rs): the Storage/DatabaseOps traits, Mode, WriteLock,
//     and this unit's three new pub(crate) seams (with_temp_directory,
//     get_file_size, call_process).
//   * row 67 (storage/db.rs): DB::from_raw is the ONLY construction door
//     (it owns the shared finalizer); Container + its Static impl free the
//     container. This unit must never declare glean_rocksdb_database_free
//     again.
//   * rows 68/69: newStorage's memory capacity and the three capacity
//     queries; both shipped, so this is the first glean-db unit whose
//     glean-util imports all exist.
//   * row 72 (storage/rocks_db.rs) will mirror this file's shape (same
//     DB/Container/from_raw path, same seams) and must reuse the seams
//     rather than declare its own.
//   * row 104 (database/types.rs) owns the closed Storage enum that will
//     hold this Database type.
//
// MARKERS: one source-defect reproduction (the doesDirectoryExist check),
// zero port todos, zero performance deferrals, zero inventory divergences.
// SAFETY count: 5 — three unsafe blocks (the two invoke calls and
// DB::from_raw), the Container adoption note (safe code carrying an unsafe
// invariant), and the sys block's declaration note.

use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_void};
use std::path::Path;

use glean_core::ffi::{invoke, using, FfiError};
use glean_core::rts::foreign::fact_set::FactSet;
use glean_core::rts::foreign::inventory::Inventory;
use glean_core::rts::foreign::lookup::{CanLookup, Lookup};
use glean_core::rts::foreign::ownership::{
    next_uset_id, ComputedOwnership, DefineOwnership, Ownership, UnitId, FIRST_USET_ID,
};
use glean_core::rts::types::{invalid_fid, Fid, Pid};
use glean_thrift as thrift;
use glean_util::r#impl::memory_reader::total_mem_capacity_kb;
use glean_util::util::disk::{get_disk_size, get_free_disk_space, get_used_disk_space};

use crate::database::backup::backend::{remove_path_forcibly, Data, RestoreSource};
use crate::database::exception::DbError;
use crate::database::repo::{combine, database_path};
use crate::database::storage::db::{Container, DB};
use crate::database::storage::{
    call_process, get_file_size, with_materialized_file, with_temp_directory, AxiomOwnership,
    DBVersion, DatabaseOps, Mode, Storage, WriteLock,
};

/// 'data LMDB = LMDB { lmdbRoot :: FilePath, lmdbMaxDiskSize :: Maybe Int }'
/// (:44-51).
#[derive(Clone)]
pub struct LMDB {
    /// 'lmdbRoot' — the directory holding every repo's container.
    pub lmdb_root: String,
    /// 'lmdbMaxDiskSize' — the virtual limit to REPORT; it is not enforced
    /// (:47-50).
    pub lmdb_max_disk_size: Option<i64>,
}

/// 'newStorage :: FilePath -> ServerConfig.Config -> IO LMDB' (:53-62).
pub fn new_storage(root: &str, config: &thrift::Config) -> LMDB {
    let mem_capacity = total_mem_capacity_kb();
    LMDB {
        lmdb_root: root.to_string(),
        // case mem_capacity of
        //   Just mem -> (* (mem * 1024)) . fromIntegral <$> Maybe Int
        //   Nothing  -> Nothing
        lmdb_max_disk_size: match mem_capacity {
            Some(mem) => config
                .db_rocksdb_disk_mem_capacity_ratio_limit
                .map(|ratio| mem.wrapping_mul(1024).wrapping_mul(ratio as i64)),
            None => None,
        },
    }
}

/// 'data instance Database LMDB = Database DB LMDB' (:64) — the open
/// database of this backend.
pub struct Database {
    /// The shared DB wrapper (row 67); owns the C++ Database*.
    db: DB,
    /// The backend record the database was opened from.
    lmdb: LMDB,
}

// ---------------------------------------------------------------------------
// CanLookup (LMDB.hs:66-68)
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
// Storage (LMDB.hs:70-147)
// ---------------------------------------------------------------------------

impl Storage for LMDB {
    type Database = Database;

    /// 'describe db = "lmdb:" <> lmdbRoot db' (:71).
    fn describe(&self) -> String {
        format!("lmdb:{}", self.lmdb_root)
    }

    /// 'we started at 3 because that's what RocksDB was on' (:73).
    fn readable_versions(&self) -> Vec<DBVersion> {
        vec![3]
    }

    fn writable_versions(&self) -> Vec<DBVersion> {
        vec![3]
    }

    /// 'open lmdb repo mode (DBVersion version)' (:77-103).
    fn open(
        &self,
        repo: &thrift::Repo,
        mode: Mode,
        version: DBVersion,
    ) -> Result<Database, DbError> {
        let path = container_path(self, repo);
        let squash = format!("{}.squashfs", path);
        let (cmode, start, ownership) = match mode {
            Mode::ReadOnly => {
                // BUG(port): the source asks doesDirectoryExist on a FILE
                // (see the source-defect note); is_dir() reproduces the
                // always-False predicate.
                let exists = Path::new(&combine(&path, "data.mdb")).is_dir();
                if !exists {
                    let have_squash = does_file_exist(&squash);
                    if have_squash {
                        std::fs::create_dir_all(&path)?;
                        call_process("squashfuse_ll", &[&squash, &path])?;
                    }
                }
                // Mode::ReadOnly = 0 (storage/db.h:82).
                (0, invalid_fid(), None)
            }
            // Mode::ReadWrite = 1.
            Mode::ReadWrite => (1, invalid_fid(), None),
            Mode::Create {
                first_id,
                ownership,
                ..
            } => {
                std::fs::create_dir_all(&path)?;
                // Mode::Create = 2.
                (2, first_id, ownership)
            }
        };
        // withCString path: the C string lives for the calls below. base's
        // newCAString does NOT reject an embedded NUL — the C side stops at
        // it, so the source would open the TRUNCATED path. Reproduced
        // (probed on the pinned GHC 9.4.7: withCString "a\0b" yields "a").
        let cpath = {
            let bytes = path.as_bytes();
            let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
            CString::new(&bytes[..end]).expect("no NUL remains after truncation")
        };
        let mut container_ptr: *mut c_void = std::ptr::null_mut();
        // SAFETY: cpath is a live NUL-terminated string for the call and the
        // out-param is a valid local. On success the C++ body stores the one
        // newly-allocated Container* there (lmdb/ffi.cpp:27-31).
        unsafe {
            invoke(|| sys::glean_lmdb_container_open(cpath.as_ptr(), cmode, &mut container_ptr))?;
        }
        // SAFETY: non-null on success (invoke returned Ok); using frees it
        // with glean_rocksdb_container_free exactly once on every exit path,
        // matching the source's using (DB.hs:287-288).
        let container = Container(container_ptr);
        using(
            || container,
            |container| -> Result<Database, DbError> {
                // maybe (return firstUsetId) nextUsetId ownership (:95).
                let first_unit_id = match &ownership {
                    Some(ownership) => next_uset_id(ownership)?.0,
                    None => FIRST_USET_ID.0,
                };
                let mut db_ptr: *mut c_void = std::ptr::null_mut();
                // SAFETY: the container is live for this scope; the
                // out-param is a valid local and receives the one
                // newly-allocated Database* on success (lmdb/ffi.cpp:33-44,
                // which MOVES out of the container).
                unsafe {
                    invoke(|| {
                        sys::glean_lmdb_container_open_database(
                            container.0,
                            start,
                            first_unit_id,
                            version,
                            &mut db_ptr,
                        )
                    })?;
                }
                // SAFETY: db_ptr is the live, uniquely-owned Database* from
                // the successful call above; ownership moves into the Db
                // marker, whose Drop runs glean_rocksdb_database_free at
                // most once (row 67's DIVERGENCES 1). Destroying the
                // container afterwards is safe: open_database moved out of
                // it.
                let db = unsafe { DB::from_raw(db_ptr, repo.clone()) };
                Ok(Database {
                    db,
                    lmdb: self.clone(),
                })
            },
        )
    }

    /// 'delete lmdb = safeRemovePathForcibly . containerPath lmdb' (:105).
    fn delete(&self, repo: &thrift::Repo) -> Result<(), DbError> {
        Ok(remove_path_forcibly(&container_path(self, repo))?)
    }

    /// 'safeRemoveForcibly lmdb = safeRemovePathForcibly . databasePath
    /// (lmdbRoot lmdb)' (:107-108).
    fn safe_remove_forcibly(&self, repo: &thrift::Repo) -> Result<(), DbError> {
        Ok(remove_path_forcibly(&database_path(&self.lmdb_root, repo))?)
    }

    /// 'getTotalCapacity lmdb' (:110-118).
    fn get_total_capacity(&self) -> Result<Option<i64>, DbError> {
        let exists = Path::new(&self.lmdb_root).is_dir();
        if exists {
            let full_disk_capacity = get_disk_size(&self.lmdb_root)?;
            Ok(Some(match self.lmdb_max_disk_size {
                Some(max_disk_size) => max_disk_size.min(full_disk_capacity),
                None => full_disk_capacity,
            }))
        } else {
            Ok(None)
        }
    }

    /// 'getUsedCapacity lmdb' (:120-124).
    fn get_used_capacity(&self) -> Result<Option<i64>, DbError> {
        let exists = Path::new(&self.lmdb_root).is_dir();
        if exists {
            Ok(Some(get_used_disk_space(&self.lmdb_root)?))
        } else {
            Ok(None)
        }
    }

    /// 'getFreeCapacity lmdb' (:126-131). The fallback is unguarded in the
    /// source, so a missing root surfaces the IOException here (row 69's
    /// header records the same contract).
    fn get_free_capacity(&self) -> Result<i64, DbError> {
        let used = self.get_used_capacity()?;
        let total = self.get_total_capacity()?;
        match (used, total) {
            (Some(used), Some(total)) => Ok(total - used),
            _ => Ok(get_free_disk_space(&self.lmdb_root)?),
        }
    }

    /// 'withScratchRoot rocks f = f $ lmdbRoot rocks </> ".scratch"' (:133).
    fn with_scratch_root<T, F>(&self, action: F) -> Result<T, DbError>
    where
        F: FnOnce(&str) -> Result<T, DbError>,
    {
        action(&combine(&self.lmdb_root, ".scratch"))
    }

    /// 'restore lmdb cfg repo scratch src' (:135-147).
    fn restore(
        &self,
        config: &thrift::Config,
        repo: &thrift::Repo,
        scratch: &str,
        source: RestoreSource,
    ) -> Result<(), DbError> {
        with_temp_directory(scratch, "restore", |scratch_restore| {
            with_materialized_file(scratch_restore, source, |file| {
                let target = container_path(self, repo);
                std::fs::create_dir_all(take_directory(&target))?;
                if config.db_lmdb_restore_unpack {
                    let db = combine(scratch_restore, "db");
                    std::fs::create_dir_all(&db)?;
                    call_process("unsquashfs", &["-d", &db, file])?;
                    std::fs::rename(&db, &target)?;
                    Ok(())
                } else {
                    std::fs::rename(file, format!("{}.squashfs", target))?;
                    Ok(())
                }
            })
        })
    }
}

/// 'containerPath LMDB{..} repo = databasePath lmdbRoot repo </> "db"'
/// (:149-150).
fn container_path(lmdb: &LMDB, repo: &thrift::Repo) -> String {
    combine(&database_path(&lmdb.lmdb_root, repo), "db")
}

/// 'doesFileExist' (directory-1.3.6.2): the path exists AND is not a
/// directory, so a FIFO or socket counts as a file. Path::is_file() is
/// stricter (regular files only) and would skip the squashfs branch for a
/// non-regular image. Follows symlinks; a broken link is False, like
/// getFileStatus failing under handleIOError.
fn does_file_exist(path: &str) -> bool {
    std::fs::metadata(path)
        .map(|metadata| !metadata.is_dir())
        .unwrap_or(false)
}

/// System.FilePath.takeDirectory (the glean-util port, private there):
/// no directory component means ".", trailing separators are stripped, and
/// a run of separators collapses to the root "/".
fn take_directory(path: &str) -> String {
    let dir = match path.rfind('/') {
        Some(i) => &path[..=i],
        None => "",
    };
    let dir = if dir.is_empty() { "./" } else { dir };
    let stripped = dir.trim_end_matches('/');
    if stripped.is_empty() {
        "/".to_string()
    } else {
        stripped.to_string()
    }
}

// ---------------------------------------------------------------------------
// DatabaseOps (LMDB.hs:152-191)
// ---------------------------------------------------------------------------

impl DatabaseOps for Database {
    /// 'close (Database db@(DB _ repo) lmdb)' (:153-158): close the DB, then
    /// umount the squashfs if the image exists, swallowing the IOException.
    fn close(&self) -> Result<(), DbError> {
        self.db.close()?;
        let path = container_path(&self.lmdb, &self.db.db_repo);
        let squash = format!("{}.squashfs", path);
        let have_squash = does_file_exist(&squash);
        if have_squash {
            // catch \(_ :: IOException) -> return () (:158).
            let _ = call_process("umount", &[&path]);
        }
        Ok(())
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

    /// 'backup (Database db _) cfg scratch process' (:184-191): the DB's
    /// checkpoint directory, mksquashfs'd into scratch/out/db.squashfs.
    fn backup<A, F>(&self, config: &thrift::Config, scratch: &str, action: F) -> Result<A, DbError>
    where
        F: FnOnce(&str, Data) -> Result<A, DbError>,
    {
        self.db.backup(config, scratch, |path, _data| {
            with_temp_directory(scratch, "out", |tmpdir| {
                let out = combine(tmpdir, "db.squashfs");
                let mut args: Vec<&str> = vec![path, &out];
                args.extend(config.db_lmdb_mksquashfs_args.iter().map(|a| a.as_str()));
                call_process("mksquashfs", &args)?;
                let size = get_file_size(&out)?;
                action(&out, Data { data_size: size })
            })
        })
    }
}

// ---------------------------------------------------------------------------
// The lmdb externs (LMDB.hs:193-205)
// ---------------------------------------------------------------------------

mod sys {
    use super::{c_char, c_int, c_void, Fid};

    // SAFETY: both symbols are the lmdb/ffi.h declarations (ffi.h:29-37)
    // and are called only with a live container/database pointer and a
    // valid out-param (see the call sites' SAFETY notes). They return a
    // NUL-terminated error string (wrap.h kit) or null.
    unsafe extern "C" {
        pub fn glean_lmdb_container_open(
            path: *const c_char,
            mode: c_int,
            container: *mut *mut c_void,
        ) -> *const c_char;

        pub fn glean_lmdb_container_open_database(
            container: *mut c_void,
            start: Fid,
            first_unit_id: u32,
            version: i64,
            db: *mut *mut c_void,
        ) -> *const c_char;
    }
}

// PORT STATUS: confidence=high todos=0
