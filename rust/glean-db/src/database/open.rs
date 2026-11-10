// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// Ported from: glean/db/Glean/Database/Open.hs (895 lines).
// Manifest unit: rust/glean-db/src/database/open.rs
// (migration/manifest.tsv line 111, crate glean-db). RULE §4: one source
// file -> one target file; the manifest path is authoritative.
//
// WIRING (row 111's one-line obligation — LANDED: `pub mod open;` sits in
// database.rs at line 85, after `pub mod types;`, per the row-104/105
// append convention, database.rs:129-139; this unit wrote only its own
// target file). `crate::database::open` resolves.
//
// WHAT THIS FILE IS: the DB lifecycle state machine's driver — the exports
// of Open.hs (`usingActiveDatabase`, `withOpenDatabase`,
// `withOpenDatabaseStack`, `withWritableDatabase`, `readDatabase`,
// `readDatabaseWithBoundaries`, `asyncOpenDB`, `newDB`, `acquireDB`,
// `releaseDB`, `isDatabaseClosed`, `schemaUpdated`, `getDbSchemaVersion`,
// `updateLookupCacheStats`, `repoParent`, `repoParents`, `depParent`) plus
// its private machinery (`withActiveDatabase`, `withMaybeActiveDatabase`,
// `withOpenDBLookup`, `withOpenDBStack`, the ACL-slice builders,
// `setupSchema`, `setupWriting`, `baseSlices`/`depSlices`,
// `enqueueBatchDescriptorsFromDatabase`, `handling_failures`). The
// lifecycle STATE itself (DB/DBState/DbLifecycle/OpenDB/Writing) is
// unit 104's (`crate::database::types`) — this file drives its
// transitions; the write-queue enqueue funnel is unit 105's
// (`crate::database::write::queue`).
//
// ---------------------------------------------------------------------------
// INVENTORY (RULE §5; migration/inventory*.tsv — grep "Open.hs")
// ---------------------------------------------------------------------------
//
// Governing rows and their disposition:
//
//   * inventory.tsv:26 — `Database/Open.hs / DB lifecycle state machine /
//     dbState + dbUsers + odbIdleSince/odbOwnership — transitions driven by
//     open/close/finish/delete across 6 files / STM_GROUP`. Clauses:
//       - "struct Db { Mutex<DbLifecycle {...}> }" — FOLLOWED: the struct
//         is shipped (types.rs, unit 104); this file's `new_db` builds it
//         exactly (Closed, 0 users).
//       - "Create.hs:152-155 inserts into envActive AND sets dbState
//         Opening in ONE atomically -> in Rust take the Env active-map
//         lock, then the DB lock, and document order ENV_ACTIVE -> DB
//         (never reverse; Close.hs:39 readTVarIO envActive then per-DB is
//         read-then-lock, consistent)" — FOLLOWED: `acquire_active_database`
//         holds `env_active` across `acquire_db` (ENV_ACTIVE -> DB), and
//         `schema_updated`'s acquire does the same. `release_db`'s removal
//         also takes ENV_ACTIVE before the DB re-verify, never the reverse.
//       - "dbUsers decrement + idle eviction (Close.hs:44-49) reads 4 TVars
//         incl. writeQueueSize/wrCommit -> take DB lock and Writing lock
//         (DB -> WRITING order)" — that predicate is Close.hs's (row 121);
//         this file ships the primitives it reads (`release_db`, the
//         lifecycle fields). The order it must follow is types.rs's TOTAL
//         LOCK ORDER.
//       - "state Closing..Closed via RAII finally (Close.hs:61-75)" —
//         Close.hs's row; nothing here.
//       - "Delete mailbox (Delete.hs:68 TMVar (Maybe DB), :82-112) ->
//         dedicated deleter thread + mpsc<Option<DB>>" — Delete.hs's row;
//         this file ships `new_db` (Delete.hs:128's `DB repo <$> ...`
//         constructor) and nothing else of it.
//       - "Finish.hs:40-99 multi-var finish protocol sits with the Writing
//         commit row below" — Finish.hs's row; the `odb_writing = None`
//         write it needs (`writeTVar (dbState db) $ Open odb { odbWriting
//         = Nothing }`, Finish.hs:83) goes through `DbLifecycle` under the
//         DB lock — the row-26 merge, see types.rs TRANSLATION NOTES 2.
//       - "Ownership.hs:46/UserQuery.hs:813 odbOwnership readTVarIO ->
//         snapshot under DB lock" — those consumers read
//         `OpenDB.odb_ownership` (its own Mutex, types.rs); the in-file
//         snapshot sites (`build_layer_acl_slice_for_groups`,
//         `db_slices`) take that Mutex alone, never nested inside the DB
//         lock (the source's readTVarIO is likewise outside any
//         transaction).
//   * inventory.tsv:22 — `Database/Create.hs / markBroken /
//     handling_failures / ... / EXC_TAXONOMY`: evidence names
//     "Open.hs:449-450 on_failure :: SomeException -> IO(); :756-761
//     handling_failures (dbState Closed, log, on_failure, throwIO)".
//     FOLLOWED: `handling_failures` (inside `open_db_body`'s bracket)
//     reproduces exactly that sequence — dbState := Closed,
//     `logError "couldn't open: " <> show`, `on_failure exc`, rethrow —
//     and the `on_failure` parameter keeps the source's shape (Create.hs
//     passes `markBroken "couldn't create"`). The reason-string
//     discipline ("mark meta Broken with the SAME reason string") is the
//     on_failure caller's side (Create.rs), not this funnel's.
//     INVENTORY-BUG-CANDIDATE row 22: the row's evidence column mis-cites
//     the on_failure location (it names Open.hs:449-450 — releaseDB's
//     try/readMeta; onFailure is Open.hs:120); the implemented
//     handling_failures sequence is correct (Open.hs:755-759).
//   * inventory.tsv:124 — `Database/{Close,Finish,Create}.hs / DB lifecycle
//     transactions / ...`: "extend the Open.hs lifecycle row's Mutex
//     groups; ... single DB-state Mutex per db handle". FOLLOWED: every
//     lifecycle touch below goes through the ONE `db_lifecycle` Mutex;
//     this file adds no state group.
//   * inventory.sites.tsv:615 — Open.hs `inRepo` as consumer evidence —
//     `in_repo` comes from `crate::database::repo` (row 52, shipped).
//
// No row governs the Open.hs:96-113 CAS transaction's Catalog interaction
// (the TRANSACTION MAP records the resolution the port picked — flagged
// there), nor the Open.hs:99/:101/:607 `Opening -> retry` waits (flagged
// at `wait_for_lifecycle_change`), nor the Open.hs:628-638 schema swap
// (flagged at `commit_schema_update`). All three gaps are flagged
// in-file under RULE §5's protocol.
//
// ---------------------------------------------------------------------------
// TRANSACTION MAP (the atomicity the call sites dictate — read before
// touching the lock structure)
// ---------------------------------------------------------------------------
//
// Open.hs's own atomically blocks, and what the port does with each:
//
//   1. withActiveDatabase's acquire (:462-476): ONE transaction —
//      envActive lookup OR (Catalog.exists + envDeleting check + newDB +
//      insert), then acquireDB. Port: hold `env_active` across the whole
//      sequence; `acquire_db` takes the DB lock NESTED (ENV_ACTIVE -> DB,
//      row 26's mandate); `Catalog.exists` is taken while holding
//      `env_active` (ENV_ACTIVE -> CATALOG). The release (:477 `atomically
//      . releaseDB`) is release_db below.
//
//   2. releaseDB (:437-453): ONE transaction — dbUsers decrement, and on
//      the last user with state Closed: `try $ Catalog.readMeta` and
//      remove from envActive unless the meta is Incomplete. Port: the DB
//      lock covers the decrement + state snapshot; the Catalog read runs
//      with NOTHING else held (CATALOG is a leaf — clause 4); the removal
//      re-takes ENV_ACTIVE -> DB and re-verifies BOTH the state is still
//      Closed AND the user count is still 0. The STM transaction
//      re-executes FROM THE TOP on retry — Open.hs:443 re-reads dbUsers
//      BEFORE the `when (users == 1)` gate — so a concurrent re-acquire
//      between the decrement and the removal makes the retried
//      transaction see users == 2 and NOT remove: the gate is false and
//      the DB stays in envActive. The port's step-3 re-check is that
//      gate, both halves of it.
//
//   3. withOpenDatabase's CAS (:96-113): ONE transaction — read dbState;
//      Opening/Closing -> retry (PARK until the opener commits); Open ->
//      hand back the odb; Closed -> Catalog.readMeta + version/mode +
//      dbState := Opening. Port: `cas_open_state` —
//        step 1 (DB lock): the state read + AlreadyOpen handback;
//        step 2 (CATALOG alone): read_meta + version/mode;
//        step 3 (DB lock): re-verify the state and commit Opening.
//      If step 3 sees Open/Opening/Closing (another thread moved the
//      state), the whole sequence retries from step 1 — STM's
//      transaction retry. THE STALE-META WINDOW: between step 2 and step
//      3 a concurrent writeMeta would have aborted the source's
//      transaction and re-read; the port commits the step-2 snapshot
//      once the state re-verifies Closed. Version/mode off a stale meta
//      is the accepted cost of the ordering picked below.
//
//      TODO(port): the Catalog<->DB order. The review of unit 104
//      established the global sequence CATALOG -> ENV_ACTIVE -> DB ->
//      WRITING with the Catalog<->DB edge still open; this file must
//      pick one, and it picks: THE CATALOG IS A LEAF — no thread holds
//      CATALOG while taking ENV_ACTIVE/DB, and (by clause 3) no thread
//      holds DB while taking CATALOG. That is consistent with the
//      established sequence (each CATALOG acquisition completes before
//      any DB acquisition starts) and with the shipped catalog.rs, whose
//      `create` runs the Create.hs:152-155 callback OUTSIDE the catalog
//      lock (catalog.rs B1 note: "The only production caller, Create.hs:
//      151-155, touches only Env TVars") and whose `modify_meta` runs
//      its callback outside the lock too (so Finish.rs's
//      CATALOG-then-makeReadOnly transaction is also sequential, never
//      nested). The in-tree precedent for the split is catalog.rs's
//      optimistic modify_meta protocol ("That is STM's transaction
//      retry"). Revisit if a future unit must hold CATALOG across a DB
//      acquisition.
//
//   4. Why CATALOG-as-leaf is sound here: the only CATALOG users this
//      file has are read_meta/exists/db_failed/reset_failed, all
//      single-lock operations on catalog.rs's own state. The reverse
//      edge (CATALOG held, then ENV_ACTIVE/DB) exists in the SOURCE's
//      Finish.hs:40-59 and Create.hs:151-155 transactions, but the
//      shipped catalog.rs API images both with the callback/state
//      mutation OUTSIDE the catalog lock (B1), so the port's Finish.rs
//      and Create.rs take CATALOG first, RELEASE, then take
//      ENV_ACTIVE -> DB — the sequence, not the nest. (If that
//      catalog.rs posture ever changes, clause 3's TODO reopens.)
//
//   5. asyncOpenDB's bracket_ (:695-697): acquireDB ... releaseDB around
//      the open body — two separate transactions in the source (the
//      bracket edges), so the port takes the DB lock for the increment
//      and release_db for the decrement, nothing held across the body.
//
//   6. The Open commit (:750): `atomically $ writeTVar dbState $ Open
//      odb` — ONE transaction. The row-26 merge puts odb_writing/
//      odb_idle_since in the SAME DbLifecycle, so the port sets db_state
//      = Open(Arc), odb_writing and odb_idle_since in ONE DB-lock hold —
//      exactly the atomicity the source's single transaction gave the
//      observable state (Open.hs:729's idle TVar creation folds in;
//      Close.hs's eviction predicate reads all four under one lock).
//
//   7. schemaUpdated (:588-638): acquire (envActive snapshot + acquireDB
//      all — ENV_ACTIVE -> DB, one hold), per-DB state read (:604-610,
//      Opening -> retry — the same park as clause 3), then on success
//      the dbState swap (:628-638: Opening -> no-op, Open odb ->
//      dbState := Open odb{odbSchema = schema}, Closing/Closed ->
//      no-op). Port: one DB-lock hold for the swap; the record's
//      in-place mutation (and its residual window) is flagged at
//      `commit_schema_update`.
//
//   8. The write-queue counters (Close.hs:44-49's eviction read,
//      Finish.hs:65-84's finish check) are NOT this file's sites; they
//      own the DB -> WRITING / DB -> WRITE_QUEUE_STATE edges per row 124
//      and the types.rs TOTAL LOCK ORDER. Nothing here takes a queue
//      lock: `enqueue_batch_descriptors_from_database` runs AFTER the
//      Open commit with no lifecycle lock held (the source's order:
//      writeTVar Open, THEN enqueue — Open.hs:750-751), and unit 105's
//      enqueue funnel takes its own locks.
//
// ---------------------------------------------------------------------------
// DEPENDENCY TRUTH (source import line -> shipped Rust path)
// ---------------------------------------------------------------------------
//
//   :24 Control.Concurrent.Async (wait)
//                                       -> glean_util::util::warden::
//                                          Task (`wait_once`; warden.rs's
//                                          Open.hs:690 contract note)
//   :25-27 Control.Exception / Monad.Catch (try)
//                                       -> Result + OpenError (DIVERGENCES
//                                          1); the `try`-wrapped readMeta
//                                          in releaseDB -> the match on
//                                          Ok(Incomplete) vs everything
//                                          else
//   :28 Control.Monad.Extra            -> std control flow
//   :30 Util.Log                       -> glean_util::log::{vlog,
//                                          log_info, log_error} (the
//                                          pinned seam; data.rs:75-79,
//                                          catalog.rs:487-499)
//   :45 Util.Logger (loggingAction)    -> `log_action` below (the private
//                                          image; external hsthrift
//                                          package, no manifest unit —
//                                          pinned at ~/glean-build/
//                                          hsthrift/common/util/Util/
//                                          Logger.hs:29-51: timeIt/try,
//                                          success/failure log, swallow,
//                                          rethrow)
//   :46 Util.STM                       -> std::sync Mutex images (RULEBOOK
//                                          §2 STM row)
//   :51 Catalog                        -> crate::database::catalog::
//                                          {Catalog, CatalogError}
//   :52 Catalog.Filter (Locality)      -> crate::database::catalog::filter::
//                                          Locality (row 49)
//   :53 Database.Data (storeSchema, retrieveSchema, retrieveUnits,
//      retrieveSlices, storeUnits, storeSlices, retrieveFirstACLID,
//      retrieveACLGroupMapping)
//                                      -> crate::database::data (row 60,
//                                          shipped — the same eight fns)
//   :58 Database.Exception (dbError)   -> crate::database::exception::
//                                          db_error (the single funnel)
//   :59 Database.Repo (inRepo)         -> crate::database::repo::in_repo
//                                          (row 52)
//   :60 Database.Storage               -> crate::database::storage::
//                                          {can_open_version, Mode,
//                                          CreateSchema, DBVersion,
//                                          WriteLock, Storage, DatabaseOps}
//   :61 Database.Meta (Meta, getACLMode, ACLMode, showACLMode)
//                                      -> crate::database::meta::{Meta,
//                                          get_acl_mode, AclMode,
//                                          show_acl_mode}
//   :62-63 Database.Schema / Schema.Types
//                                      -> crate::database::schema::
//                                          {new_db_schema, from_stored_
//                                          schema, new_merged_db_schema,
//                                          to_stored_schema, schema_size,
//                                          read_only_content,
//                                          read_write_content} and
//                                          schema::types::{DbSchema,
//                                          SchemaSelector}
//   :64 Database.Types                 -> crate::database::types (Env, DB,
//                                          DBState, DbLifecycle, OpenDB,
//                                          Writing, withStorageFor —
//                                          NEVER re-declared)
//   :65 Database.Write.Queue (enqueueBatchDescriptorWithResultHandling)
//                                      -> crate::database::write::queue::
//                                          enqueue_batch_descriptor_with_
//                                          result_handling (row 105)
//   :66 Logger (runLogRepo)            -> log_action's SetMethod/
//                                          SetRepoName/SetRepoHash chain
//                                          (row 106's Logger.hs body is
//                                          UNSHIPPED — logger.rs is still
//                                          scaffolding; see the TODO at
//                                          log_action)
//   :67 Query.Codegen (Boundaries, flatBoundaries, stackedBoundaries)
//                                      -> crate::query::codegen (row 110,
//                                          UNSHIPPED — ASSUMED-IMPORT
//                                          block below)
//   :68 Repo.Text (showRepo)           -> glean_core::repo::text::show_repo
//   :69 RTS.Foreign.Lookup (Lookup, withCanLookup, firstFreeId)
//                                      -> glean_core::rts::foreign::lookup
//   :71 LookupCache                    -> glean_core::rts::foreign::
//                                          lookup_cache (new,
//                                          read_stats_and_reset_counters)
//   :72 Ownership (UnitId, UsetId, slice, slicedStack)
//                                      -> glean_core::rts::foreign::
//                                          ownership
//   :74 Stacked (stacked)              -> glean_core::rts::foreign::stacked
//   :75 RTS.Types (Pid)                -> glean_core::rts::types::Pid
//   :76 ServerConfig.Types             -> glean_thrift::Config (the
//                                          ServerConfig alias, types.rs:654)
//   :77 Internal.Types (Thrift)        -> glean_thrift (ASSUMED-IMPORT);
//                                          Completeness ->
//                                          crate::internal::types (row 142,
//                                          the data.rs:135 precedent)
//   :78-79 Types (Thrift: Repo, PredicateStats, Dependencies)
//                                      -> glean_thrift
//   :80 Util.Mutex                     -> dissolves to std::sync::Mutex
//                                          (unit 102 ships zero items)
//   :81 Observed                       -> glean_util::util::observed::
//                                          Observed (`get` returns A)
//   :82 Util.Time (TimePoint, getTimePoint)
//                                      -> std::time::Instant (types.rs's
//                                          pinned image; getTimePoint ->
//                                          Instant::now())
//   :84 Warden (spawnMask)             -> glean_util::util::warden::
//                                          spawn_mask (warden.rs pins
//                                          Open.hs:690 -> spawn_mask,
//                                          Task return, wait_once)
//   :85 Write.Stats (bump, lookupCacheStats)
//                                      -> glean_core::write::stats::
//                                          {bump, LOOKUP_CACHE_STATS}
//
// ---------------------------------------------------------------------------
// ASSUMED-IMPORTS (RULE §4 B4)
// ---------------------------------------------------------------------------
//
//   * `glean_thrift` — generated from glean/if/glean.thrift. Shapes used
//     here, pinned to the IDL / shipped precedents:
//       - `Repo { name: String, hash: String }` (repo.rs:52's pin);
//       - `Dependencies::Pruned(Pruned)` /
//         `Dependencies::Stacked(Stacked)` (catalog.rs:892-901's pin);
//         `Pruned { base: Repo, units: Vec<Vec<u8>>, exclude: bool, guid:
//         Option<String> }` and `Stacked { name: String, hash: String,
//         guid: Option<String> }` — the bare IDL field names
//         (glean.thrift:119-132; queue.rs:214-216's "the Rust field is
//         the bare IDL name");
//       - `UnknownDatabase { repo: Repo, call_stack: String }`
//         (glean.thrift:243-246; catalog.rs:1220-1223 constructs exactly
//         this, call_stack empty — Rust has no call-stack capture);
//       - `PredicateStats { count: i64, size: i64 }` (glean.thrift:344-347);
//         the Haskell Semigroup instance (`HashMap.fromListWith (<>)`,
//         Open.hs:546) is componentwise addition, reproduced in
//         `merge_predicate_stats`;
//       - `BatchDescriptor` (queue.rs's pin), `SchemaId = String`
//         (types.rs:259's pin), `DatabaseProperties =
//         BTreeMap<String, String>` (backend/types.rs:169-170's pin).
//     Generated Clone on the records, and generated Display + Error +
//     Send + Sync + 'static on the exception structs, are the
//     batch_location.rs:66-69 Step-4 generator obligations
//     (queue.rs:240-244 extended the same sketch).
//   * `crate::internal::types::{Completeness, StoredSchema}` — row 142's
//     generated Meta home (meta.rs:522 imports Completeness;
//     queue.rs:497 constructs `Completeness::Broken(..)`; data.rs:135
//     imports StoredSchema). Completeness variants
//     `Incomplete(_)`/`Complete(_)`/`Finalizing(_)`/`Broken(_)` per
//     meta.rs:628-633. `StoredSchema: Clone` — the generated record's
//     (needed only to carry the `Create` payload past `Storage.open`'s
//     by-value consumption, see `SetupSchema`).
//   * `crate::query::codegen::{Boundaries, flat_boundaries,
//     stacked_boundaries}` — manifest row 110 (Codegen.hs), UNSHIPPED:
//     query/codegen.rs is still row 86's scaffolding. The source shapes
//     (Codegen.hs:180-195): `enum Boundaries { Flat(SectionBoundaries),
//     Stacked(SectionBoundaries, SectionBoundaries) }`,
//     `struct SectionBoundaries { start: Fid, end: Fid }`,
//     `flat_boundaries(&Lookup) -> Result<Boundaries, FfiError>` and
//     `stacked_boundaries(&Lookup, &Lookup) -> Result<Boundaries,
//     FfiError>` (sectionBounds = Lookup.startingId/firstFreeId — the
//     FFI is the only thrower), `Clone` (the value crosses into the
//     caller's callback). This is the data.rs:69-80 ->
//     crate::internal::types precedent: import from the owner's manifest
//     path, document the assumed shape, never fork the owner. A different
//     ported spelling is a Step-4 compile error.
//   * `glean_util::log::{vlog, log_info, log_error}` — the pinned seam
//     (DECISIONS.md 2026-10-18; first sketch
//     query/thrift/internal.rs:190-213):
//         pub fn vlog(level: i64, msg: impl FnOnce() -> String);
//         pub fn log_info(msg: impl FnOnce() -> String);
//         pub fn log_error(msg: impl FnOnce() -> String);
//
// ---------------------------------------------------------------------------
// SIBLING-UNIT CONTRACTS (consumed, never re-declared)
// ---------------------------------------------------------------------------
//
//   * crate::database::types (unit 104, SHIPPED): `Env` / `DB` /
//     `DbLifecycle` / `DBState` / `OpenDB` / `Writing` / `WriteState` /
//     `WriteQueueState` / `SomeDatabaseOps` / `with_storage_for` are USED
//     here, not re-defined. Field notes: `odb_writing` and `odb_idle_since`
//     live in `DbLifecycle` (row 26's record split — TRANSLATION NOTES 2
//     there), so every source read of `odbWriting odb` / `odbIdleSince
//     odb` below goes through the DB lock; `DBState::Open(Arc<OpenDB>)` is
//     the shared-handle image (TRANSLATION NOTES 9). The lock order is
//     that file's TOTAL LOCK ORDER (ENV_ACTIVE -> DB -> WRITING ->
//     WRITE_QUEUES -> WRITE_QUEUE_STATE); this file takes the ENV_ACTIVE
//     -> DB prefix and never a suffix.
//   * crate::database::write::queue (row 105, SHIPPED):
//     `enqueue_batch_descriptor_with_result_handling(env, repo, &Arc<
//     WriteQueue>, &DbSchema, &SomeDatabaseOps, &BatchDescriptor,
//     is_async) -> Result<MVar<..>, EnqueueWriteError>` is the ONLY
//     enqueue entry this file calls (Open.hs:769-770), inside
//     `enqueue_batch_descriptors_from_database`.
//   * crate::database::catalog (row 54, SHIPPED): `read_meta` /
//     `exists(&[Locality], repo)` / `db_failed(repo, impl Into<Box<..>>)` /
//     `reset_failed` — all `Result<_, CatalogError>`. `db_failed` is the
//     row-22 funnel.
//   * crate::database::data (row 60, SHIPPED): the eight store/retrieve
//     fns, all `Result<_, DbError>`.
//   * crate::database::storage (row 58, SHIPPED): `can_open_version`
//     (generic over the StorageTrait), `Mode`/`CreateSchema` — `Mode` is
//     consumed BY VALUE by `Storage::open`, so the post-open
//     scrutinisations of `mode` (:706/:709/:712) are extracted into the
//     private `SetupSchema`/bools BEFORE the call (see `open_db_body`);
//     `DatabaseOps` (close/predicate_stats/get_unit_id/get_ownership/
//     get_unprocessed_batch_descriptors), `WriteLock`, `DBVersion = i64`.
//   * crate::database::schema (row 66/89, SHIPPED): the four DbSchema
//     constructors (`Option<&Mutex<DbSchemaCache>>` first arg —
//     `env.env_db_schema_cache` is the MVar-as-mutex, types.rs:1565-1568),
//     `DbContent`, `SchemaSelector`, `schema_size`.
//   * glean_core::rts::foreign::lookup (SHIPPED): `Lookup` is a
//     NON-OWNING view (ptr + name, no Drop, not Clone); `CanLookup` is
//     implemented by value (`Lookup`, `Stacked<B, A>`, `SlicedStack<B>`,
//     `SomeDatabaseOps`). The source's Lookup is a shared by-value
//     handle, so this file's `lookup_view` re-points a fresh view over
//     the same ptr wherever the source would pass the value down another
//     level — the image of Haskell's shared handle, not a copy of the C
//     object.
//   * glean_core::rts::foreign::ownership (SHIPPED): `Slice` wraps an
//     Arc-backed `ForeignPtr` (ffi.rs:442-455) but does NOT derive Clone
//     as shipped — the four `catMaybes` images that need it are flagged
//     at the first one; `slice(&Ownership, &[Slice], &[UnitId], bool)`
//     and `sliced_stack(Vec<Slice>, B)` are the direct images.
//   * glean_util::util::warden (SHIPPED): `spawn_mask(&Warden,
//     |&CancelToken| -> A) -> Task<A>`; warden.rs pins THIS site: "Async
//     OpenDB return (:686) becomes Task<OpenDB>; Create.hs:166's
//     Async.wait opener is opener.wait_once() (OpenDB is not Clone)". The
//     port returns `Task<Arc<OpenDB>>` — the Open commit stores an `Arc`
//     clone in the DBState (types.rs TRANSLATION NOTES 9), so the task
//     cannot also move the bare OpenDB out; the Arc is the same sharing,
//     one level up. `wait_once` rethrows a task panic via
//     `resume_unwind`; the source's `wait` rethrows the task's exception —
//     the image pair is panic_any (throwIO) / catch_unwind+downcast (the
//     caller's catch), the queue.rs:282-286 pinned protocol.
//
// ---------------------------------------------------------------------------
// DIVERGENCES (deliberate; documented, not silent — RULE §5/A9)
// ---------------------------------------------------------------------------
//
//   1. `OpenError` — Rust-only enum carrying the exception types that
//      escape this file's frames (RULE §2 dynamic-exceptions row; the
//      queue.rs `EnqueueWriteError` shape): `Catalog(CatalogError)` (the
//      readMeta/exists/resetFailed channel), `UnknownDatabase` (the
//      thrift exception withActiveDatabase throws directly — Server.hs
//      catches it by type), and `Other` (the SomeException leg: open
//      failures, dbError sites, FFI errors — DbError/FfiError box into
//      it via From). The task-frame helpers (setup_schema, setup_writing,
//      the open body) use `Box<dyn Error + Send + Sync>` directly — the
//      SomeException image (types.rs TRANSLATION NOTES 5) — because the
//      task boundary is a panic_any/resume_unwind pair, not a Result.
//   2. `with_open_database`'s action takes `(&Arc<DB>, &Arc<OpenDB>)`,
//      not the source's bare `OpenDB`: row 26 moved `odbWriting`/
//      `odbIdleSince` behind `DbLifecycle`, so an action that reads them
//      (withWritableDatabase's `case odbWriting of`) needs the DB
//      handle. The wrapper exports (`readDatabase`,
//      `readDatabaseWithBoundaries`, `withOpenDatabaseStack`) keep the
//      source's callback shapes. `with_writable_database`'s action keeps
//      the source's exact tuple `(WriteQueue, DbSchema, Some DatabaseOps)`
//      as borrows.
//   3. Log-text images: `show (Set.toList units)` in the vlog line
//      (Open.hs:834-837) renders Haskell ByteString Show (quoted,
//      char-escaped); the port renders the byte-slice Debug. Same
//      information, different punctuation — log-only, no consumer parses
//      it. `show (schemaSize dbSchema)` / `show (unDBVersion version)` /
//      `show (length ...)` are numeric renders, identical.
//   4. `unDBVersion` — DBVersion is `i64` (storage.rs's gen-hs2 pin), so
//      the constructor/unconstructor dissolve into the value.
//   5. `lookup_view` — see the SIBLING-UNIT CONTRACTS lookup note. Not a
//      source export; the image of by-value Lookup passing.
//
// ---------------------------------------------------------------------------
// MARKERS: six port todos (the row-26 lifecycle Condvar behind the
// Opening/Closing retry parks; the Catalog<->DB order resolution; the
// Open.hs:636 schema swap's residual concurrent-handback window, pending
// types.rs's odb_schema interior mutability; the asyncOpenDB spawn's Env
// Send decision — its warden-shutdown path is handled at the wait site;
// the Slice Clone derive owed by row 57; the log_action
// weight/allocated-bytes fidelity), six inventory-bug candidates (row
// 26's retry primitive; the row-26/104 schema-swap transaction absent
// from the rows' evidence — the caller-snapshot skip is fixed at
// `commit_schema_update`, the concurrent-handback window remains; row
// 22's evidence mis-citing the on_failure location; and three row-26
// partials — the two out-of-lock ownership snapshots and schema_updated's
// per-DB split of the batched release), zero source-defect reproductions,
// zero performance deferrals, zero SAFETY notes (no unsafe in this
// file). The TODO sites are marked once each in the body; the tally
// above avoids the literal marker spellings.
// ---------------------------------------------------------------------------

use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use glean_core::ffi::FfiError;
use glean_core::repo::text::show_repo;
use glean_core::rts::foreign::lookup::{
    first_free_id, with_can_lookup, CanLookup, Lookup,
};
use glean_core::rts::foreign::lookup_cache::{
    read_stats_and_reset_counters, LookupCache,
};
use glean_core::rts::foreign::ownership::{
    slice, sliced_stack, Ownership, Slice, UnitId, UsetId,
};
use glean_core::rts::foreign::stacked::stacked;
use glean_core::rts::types::Pid;
use glean_core::write::stats::{bump, LOOKUP_CACHE_STATS};
use glean_thrift as thrift;
use glean_thrift::Config as ServerConfig;
use glean_util::log::{log_error, log_info, vlog};
use glean_util::util::warden::{spawn_mask, Task};

use crate::database::catalog::filter::Locality;
use crate::database::catalog::CatalogError;
use crate::database::data::{
    retrieve_acl_group_mapping, retrieve_first_acl_id, retrieve_schema, retrieve_slices,
    retrieve_units, store_slices, store_schema, store_units,
};
use crate::database::exception::{db_error, DbError};
use crate::database::meta::{get_acl_mode, show_acl_mode, AclMode};
use crate::database::repo::in_repo;
use crate::database::schema::types::{DbSchema, SchemaSelector};
use crate::database::schema::{
    from_stored_schema, new_db_schema, new_merged_db_schema, read_only_content,
    read_write_content, schema_size, to_stored_schema,
};
use crate::database::storage::{
    can_open_version, CreateSchema, DBVersion, DatabaseOps, Mode, Storage as StorageTrait,
    WriteLock,
};
use crate::database::types::{
    with_storage_for, DB, DbLifecycle, DBState, Env, OpenDB, SomeDatabaseOps, WriteQueue,
    WriteQueueState, WriteState, Writing,
};
use crate::database::write::queue::enqueue_batch_descriptor_with_result_handling;
use crate::internal::types::{Completeness, StoredSchema};
use crate::logger::server::GleanServerLog;
use crate::query::codegen::{flat_boundaries, stacked_boundaries, Boundaries};

// `Either SomeException a`'s image for the task frame (types.rs
// TRANSLATION NOTES 5; the same lossy image every shipped unit uses).
type Exc = Box<dyn std::error::Error + Send + Sync>;

/// The crate's poison policy (catalog.rs:252-254, types.rs:2038-2040): a
/// poisoned lock still exposes its data; recover rather than propagate.
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Box any error into the task-frame channel (no blanket From impl: it
/// would collide with the std reflexive impl and the per-type ones).
fn into_exc<E: std::error::Error + Send + Sync + 'static>(e: E) -> Exc {
    Box::new(e)
}

/// Box any error into `OpenError::Other` — the inner-closure image of
/// `map_err(into_exc)` where the surrounding function's channel is
/// [`OpenError`].
fn other<E: std::error::Error + Send + Sync + 'static>(e: E) -> OpenError {
    OpenError::Other(Box::new(e))
}

/// Open.hs:120 — `onFailure ex = atomically $ Catalog.dbFailed envCatalog
/// dbRepo ex`. Shared by with_open_database's `on_failure` closure and its
/// wait-site failure branch (the outer funnel's on_failure leg — see
/// async_open_db's doc). `db_failed` takes the exception BY VALUE
/// (catalog.rs:1600-1604, `impl Into<Box<dyn Error + Send + Sync>>`);
/// `Exc` is a `Box<dyn ..>` and not `Clone`, so the value passed is the
/// exception's `show` text — the same shape the queue.rs:497-504 Broken
/// reason uses.
fn db_failed_exc(env: &Env, repo: &thrift::Repo, exc: &Exc) -> Result<(), Exc> {
    env.env_catalog
        .db_failed(repo, format!("{}", exc))
        .map_err(into_exc)
}

// ---------------------------------------------------------------------------
// OpenError (DIVERGENCES 1) — the caller-frame exception channels
// ---------------------------------------------------------------------------

/// The exceptions that escape this file's caller frames.
pub enum OpenError {
    /// The Catalog operations' failure channel (`readMeta`, `exists`,
    /// `resetFailed` — all `Result<_, CatalogError>` in the port; the
    /// source's STM just throws).
    Catalog(CatalogError),
    /// `CallStack.throwSTM $ Thrift.UnknownDatabase repo`
    /// (Open.hs:468/:471) — the thrift exception, thrown directly (not
    /// through the catalog). Callers match it by type.
    UnknownDatabase(thrift::UnknownDatabase),
    /// The `SomeException` leg: open failures (the task's rethrow),
    /// `dbError` sites, FFI errors.
    Other(Exc),
}

impl OpenError {
    /// `Thrift.UnknownDatabase repo` — the catalog.rs:1220-1223
    /// construction (call_stack empty; Rust has no call-stack capture).
    fn unknown_database(repo: &thrift::Repo) -> OpenError {
        OpenError::UnknownDatabase(thrift::UnknownDatabase {
            repo: repo.clone(),
            call_stack: String::new(),
        })
    }
}

impl std::fmt::Display for OpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // CatalogError's own Display (constructor-shaped per row 54).
            OpenError::Catalog(e) => std::fmt::Display::fmt(e, f),
            // The generated exception's Display (derived-Show image).
            OpenError::UnknownDatabase(e) => std::fmt::Display::fmt(e, f),
            // `show (exc :: SomeException)` — the inner error's text
            // (exception.rs's convention).
            OpenError::Other(e) => std::fmt::Display::fmt(e, f),
        }
    }
}

impl std::fmt::Debug for OpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

impl std::error::Error for OpenError {}

impl From<CatalogError> for OpenError {
    fn from(e: CatalogError) -> Self {
        OpenError::Catalog(e)
    }
}

/// `dbError` sites and Storage/Data failures — the DBError exception is
/// one of the exceptions SomeException models, so it boxes into `Other`
/// (its Display is the bare message, exception.rs's pinned shape).
impl From<DbError> for OpenError {
    fn from(e: DbError) -> Self {
        OpenError::Other(Box::new(e))
    }
}

/// The FFI channel (`with_lookup`'s bound, the boundaries/ACL paths).
impl From<FfiError> for OpenError {
    fn from(e: FfiError) -> Self {
        OpenError::Other(Box::new(e))
    }
}

/// Downcast a task panic payload back into the exception channel — the
/// writer's tryAll image (queue.rs:282-286's pinned protocol):
/// `panic_any(Box<Exc>)` payloads come back as `Other`; TaskError
/// messages (warden.rs's `resume_unwind(Box::new(e.message))`) come back
/// as `Other` carrying the message text; anything else renders Debug.
fn downcast_exc(payload: Box<dyn std::any::Any + Send>) -> OpenError {
    // chain the downcasts, rebinding the payload each step (the Box is
    // consumed by every downcast::<T>() — coordinator follow-up for the
    // pre-existing E0382s the fix round flagged)
    let payload = match payload.downcast::<Exc>() {
        Ok(e) => return OpenError::Other(*e),
        Err(p) => p,
    };
    let payload = match payload.downcast::<String>() {
        Ok(s) => return OpenError::Other(into_exc(std::io::Error::other((*s).clone()))),
        Err(p) => p,
    };
    let payload = match payload.downcast::<&'static str>() {
        Ok(s) => return OpenError::Other(into_exc(std::io::Error::other((*s).to_string()))),
        Err(p) => p,
    };
    OpenError::Other(into_exc(std::io::Error::other(format!("{payload:?}"))))
}

// ---------------------------------------------------------------------------
// The lifecycle primitives (newDB / acquireDB / releaseDB)
// ---------------------------------------------------------------------------

/// 'newDB :: Repo -> STM DB' (:428-431) — `DB repo <$> newTVar Closed <*>
/// newTVar 0`.
///
/// `odb_idle_since` starts at `Instant::now()`: the row-26 merge requires
/// the field at construction, but the source only creates it when the DB
/// opens (Open.hs:729 `idle <- newTVarIO =<< getTimePoint`) — for a
/// Closed DB it is meaningless (types.rs TRANSLATION NOTES 2's invariant:
/// "meaningful only in the Open states"). First open overwrites it.
pub fn new_db(repo: thrift::Repo) -> DB {
    DB {
        db_repo: repo,
        db_lifecycle: Mutex::new(DbLifecycle {
            db_state: DBState::Closed,
            db_users: 0,
            odb_idle_since: Instant::now(),
            odb_writing: None,
        }),
    }
}

/// 'acquireDB :: DB -> STM ()' (:433-434) — `modifyTVar' (dbUsers db)
/// (+1)`. Infallible; the caller nests it under `env_active` where the
/// source's transaction held both (row 26: ENV_ACTIVE -> DB, never
/// reverse).
pub fn acquire_db(db: &DB) {
    lock(&db.db_lifecycle).db_users += 1;
}

/// 'releaseDB' (:437-453) — MUST be paired with [`acquire_db`]. The
/// source signature is `Catalog -> TVar (HashMap Repo DB) -> DB -> STM
/// ()`; the port takes the `Env` (the catalog and the active map are its
/// fields — the same values the source's callers destructure).
///
/// Source semantics, transaction-for-transaction: decrement dbUsers; on
/// the LAST user (the pre-decrement count was 1) with state Closed:
/// `try $ Catalog.readMeta` — keep the DB in envActive iff the meta is
/// `Incomplete{}` (a failed DB stays openable for the retry,
/// Open.hs:451); ANY other outcome (an exception from readMeta, or
/// Complete/Finalizing/Broken) removes it. Infallible: the `try` makes
/// the readMeta failure a removal trigger, not an error.
///
/// Port protocol (the TRANSACTION MAP, clause 2): (1) DB lock —
/// decrement + last-user + state snapshot; (2) CATALOG alone — read_meta
/// (the optimistic split; the CATALOG-is-a-leaf rule); (3) ENV_ACTIVE ->
/// DB — re-verify BOTH the state is still Closed AND the user count is
/// still 0, then remove. The source's STM transaction re-executes from
/// the top on retry (Open.hs:443 re-reads dbUsers before the
/// `when (users == 1)` gate), so a concurrent re-acquire between the
/// decrement and the removal makes the retried transaction see
/// users == 2 and NOT remove. The port's step-3 re-check is that gate.
pub fn release_db(env: &Env, db: &DB) {
    let (was_last, state_closed) = {
        let mut life = lock(&db.db_lifecycle);
        let users = life.db_users;
        life.db_users = users - 1;
        let state_closed = matches!(life.db_state, DBState::Closed);
        (users == 1, state_closed)
    };
    if !(was_last && state_closed) {
        return;
    }
    // `meta <- try $ Catalog.readMeta catalog dbRepo` (:449) —
    // `Right Meta{metaCompleteness = Thrift.Incomplete{}}` keeps the DB
    // in the active map; everything else removes it.
    let keep = match env.env_catalog.read_meta(&db.db_repo) {
        Ok(meta) => matches!(meta.meta_completeness, Completeness::Incomplete(_)),
        Err(_) => false,
    };
    if keep {
        return;
    }
    // `modifyTVar' active $ HashMap.delete dbRepo` (:452) — ENV_ACTIVE ->
    // DB. The STM transaction re-executes from the top on retry, so the
    // removal fires only if BOTH the state is still Closed AND no
    // concurrent re-acquire happened: Open.hs:443 re-reads dbUsers before
    // the `when (users == 1)` gate, so users != 0 on a re-acquired DB
    // means the gate is false and the DB STAYS in envActive.
    let mut active = lock(&env.env_active);
    let mut life = lock(&db.db_lifecycle);
    let remove = matches!(life.db_state, DBState::Closed) && life.db_users == 0;
    if remove {
        active.remove(&db.db_repo);
    }
}

/// `bracket`'s release edge for the withActiveDatabase-family brackets
/// (Open.hs:477/:490): [`release_db`] must run on normal return AND on
/// unwind — the source's bracket finalizer fires on any exception exit,
/// and a panic through the action would otherwise leak the acquired
/// dbUsers forever (no later release could reach 0). Drop-guard, the
/// warden.rs WardenGuard precedent ("the guard's Drop is the bracket
/// release: it runs on normal return and during unwinding").
struct ReleaseDbGuard<'a>(&'a Env, &'a DB);

impl Drop for ReleaseDbGuard<'_> {
    fn drop(&mut self) {
        release_db(self.0, self.1);
    }
}

// ---------------------------------------------------------------------------
// withActiveDatabase / usingActiveDatabase / withMaybeActiveDatabase
// ---------------------------------------------------------------------------

/// The `atomically` acquire of 'withActiveDatabase' (:462-476), broken
/// out for the bracket. ONE critical section in the source; the port
/// holds `env_active` across all of it (the TRANSACTION MAP, clause 1):
/// lookup, or exists + deleting-check + newDB + insert, then acquireDB
/// (ENV_ACTIVE -> DB, row 26).
fn acquire_active_database(env: &Env, repo: &thrift::Repo) -> Result<Arc<DB>, OpenError> {
    let mut active = lock(&env.env_active);
    if let Some(db) = active.get(repo) {
        let db = Arc::clone(db);
        acquire_db(&db);
        return Ok(db);
    }
    // `exists <- Catalog.exists envCatalog [Local] repo` (:467) —
    // CATALOG under ENV_ACTIVE (ENV_ACTIVE -> CATALOG; the Catalog is
    // never held while taking ENV_ACTIVE — the TRANSACTION MAP, clause 4).
    let exists = env.env_catalog.exists(&[Locality::Local], repo)?;
    if !exists {
        return Err(OpenError::unknown_database(repo));
    }
    // `deleting <- HashMap.member repo <$> readTVar envDeleting` (:469)
    // — ENV_ACTIVE -> DELETING (env_deleting is a leaf map).
    let deleting = lock(&env.env_deleting).contains_key(repo);
    // "TODO: different error?" (:470 — the source's own TODO, kept).
    if deleting {
        return Err(OpenError::unknown_database(repo));
    }
    // `db <- newDB repo; modifyTVar' envActive $ HashMap.insert repo db`
    // (:472-473).
    let db = Arc::new(new_db(repo.clone()));
    active.insert(repo.clone(), Arc::clone(&db));
    // `acquireDB db` (:475) — nested under env_active: ENV_ACTIVE -> DB.
    acquire_db(&db);
    Ok(db)
}

/// 'withActiveDatabase' (:455-478) — `bracket (atomically acquire)
/// (atomically . releaseDB envCatalog envActive)`. The release is
/// infallible (release_db's `try`-wrapped readMeta), so the bracket's
/// only error channel is the action's — and it runs on BOTH exits,
/// including a panic through the action ([`ReleaseDbGuard`]).
fn with_active_database<T, F>(env: &Env, repo: &thrift::Repo, act: F) -> Result<T, OpenError>
where
    F: FnOnce(&Arc<DB>) -> Result<T, OpenError>,
{
    let db = acquire_active_database(env, repo)?;
    // The guard drops after `act` returns (action-then-release, the
    // source's bracket order) and during unwinding.
    let _release = ReleaseDbGuard(env, &db);
    act(&db)
}

/// 'usingActiveDatabase' (:480-490) — the bracket over `Maybe DB`: the
/// acquire looks the repo up and (when present) acquires it; the release
/// releases it back. The action is unconstrained (`IO a`); the port's
/// `T` is the caller's choice (the source's throw-in-action image is
/// `T = Result<..>`).
pub fn using_active_database<T, F>(env: &Env, repo: &thrift::Repo, act: F) -> T
where
    F: FnOnce(Option<&Arc<DB>>) -> T,
{
    let db = {
        let active = lock(&env.env_active);
        let r = active.get(repo).cloned();
        if let Some(db) = &r {
            acquire_db(db);
        }
        r
    };
    // The bracket release (:490 — `mapM_ (releaseDB ...)`) — the guard
    // runs it on BOTH exits, including a panic through `act` (see
    // with_active_database).
    let _release = db.as_ref().map(|db| ReleaseDbGuard(env, db));
    act(db.as_ref())
}

/// 'withMaybeActiveDatabase' (:492-499) — `active <- readTVar envActive;
/// fn (HashMap.lookup repo active)`. The source's callback is an STM
/// action running INSIDE the transaction; its only in-tree use
/// (isDatabaseClosed, plus the Server's same-shape reads) reads
/// `dbState` — a single TVar — so the port snapshots the `Arc` out of
/// the map and runs the callback outside the lock (the B1 window: the
/// Arc keeps the DB alive; a concurrent removal is indistinguishable
/// from the source's transaction seeing the map before/after the
/// delete).
fn with_maybe_active_database<T, F>(env: &Env, repo: &thrift::Repo, f: F) -> T
where
    F: FnOnce(Option<&Arc<DB>>) -> T,
{
    let r = lock(&env.env_active).get(repo).cloned();
    f(r.as_ref())
}

/// 'isDatabaseClosed :: Env -> Repo -> STM Bool' (:886-895).
pub fn is_database_closed(env: &Env, repo: &thrift::Repo) -> bool {
    with_maybe_active_database(env, repo, |r| match r {
        Some(db) => matches!(lock(&db.db_lifecycle).db_state, DBState::Closed),
        None => true,
    })
}

// ---------------------------------------------------------------------------
// The Opening/Closing retry-park (Open.hs:99 / :101 / :607)
// ---------------------------------------------------------------------------

/// The image of the source's `Opening -> retry` / `Closing -> retry`
/// inside an STM transaction (Open.hs:99/:101 in the open CAS, :607 in
/// schemaUpdated's per-DB read): the transaction PARKS until another
/// thread changes `dbState`, then re-executes from the top.
///
/// INVENTORY-BUG-CANDIDATE row 26: the row's `Db` sketch (and the shipped
/// types.rs image) is `Mutex<DbLifecycle>` with NO Condvar, but RULEBOOK
/// §2's STM-retry row mandates "the guarding Mutex group gets a Condvar;
/// blocking re-check mirrors the retry predicate" for exactly this
/// construct. The wait primitive has no home in the shipped type and
/// this unit may only write its own file, so the UNKNOWN rule's most
/// conservative representation is used: a bounded re-check poll (1 ms
/// sleep) whose predicate IS the retry predicate. Waiters observe a
/// state change up to 1 ms late (the source wakes immediately); CPU
/// cost is one 1 ms sleep per waiter per tick.
///
/// TODO(port): add `db_lifecycle_changed: Condvar` beside
/// `DB.db_lifecycle` (row 104's owner, the same Rust-only-companion
/// shape as `WriteQueue.write_queue_changed`), notify it on every
/// `db_state` transition (the Open commit, handling_failures' Closed
/// write, Close.rs's Closing/Closed writes), and replace this poll's
/// body with `wait_timeout` on it.
fn wait_for_lifecycle_change() {
    std::thread::sleep(Duration::from_millis(1));
}

// ---------------------------------------------------------------------------
// withOpenDatabase and the open CAS
// ---------------------------------------------------------------------------

/// The CAS outcome (Open.hs:96-113).
enum OpenCas {
    /// `Open odb -> return $ Left odb` (:100).
    AlreadyOpen(Arc<OpenDB>),
    /// `Closed -> ... writeTVar dbState Opening; return $ Right (version,
    /// mode)` (:102-113).
    Proceed { version: DBVersion, mode: Mode },
}

/// The Open.hs:96-113 transaction — see the TRANSACTION MAP, clause 3,
/// for the three-step split and its TODO(port) (the Catalog<->DB order).
fn cas_open_state(env: &Env, db: &DB) -> Result<OpenCas, OpenError> {
    loop {
        // Step 1 (DB lock): the state read.
        {
            let life = lock(&db.db_lifecycle);
            match &life.db_state {
                DBState::Open(odb) => return Ok(OpenCas::AlreadyOpen(Arc::clone(odb))),
                DBState::Opening | DBState::Closing => {
                    // `Opening -> retry` / `Closing -> retry` (:99/:101):
                    // park until the opener/closer commits, re-execute.
                    drop(life);
                    wait_for_lifecycle_change();
                    continue;
                }
                DBState::Closed => { /* fall through to the meta read */ }
            }
        }
        // Step 2 (CATALOG alone): `meta <- Catalog.readMeta envCatalog
        // dbRepo` (:103) — the CATALOG-is-a-leaf rule; see the
        // TODO(port) above.
        let meta = env.env_catalog.read_meta(&db.db_repo)?;
        let version = meta.meta_version;
        // :105-111 —
        //   mode | envReadOnly = ReadOnly
        //        | Complete{} <- completeness = ReadOnly
        //        | otherwise = ReadWrite
        // ("Note: Finalizing also needs to be ReadWrite, because
        // compaction modifies the DB" — Finalizing falls into
        // `otherwise`, the source's own note, kept.)
        let mode = if env.env_read_only {
            Mode::ReadOnly
        } else if matches!(meta.meta_completeness, Completeness::Complete(_)) {
            Mode::ReadOnly
        } else {
            Mode::ReadWrite
        };
        // Step 3 (DB lock): re-verify + `writeTVar dbState Opening`
        // (:112). On any state move, retry the whole sequence (the
        // STM transaction's re-execution).
        let committed = {
            let mut life = lock(&db.db_lifecycle);
            match &life.db_state {
                DBState::Open(odb) => return Ok(OpenCas::AlreadyOpen(Arc::clone(odb))),
                DBState::Opening | DBState::Closing => false,
                DBState::Closed => {
                    life.db_state = DBState::Opening;
                    true
                }
            }
        };
        if committed {
            return Ok(OpenCas::Proceed { version, mode });
        }
        wait_for_lifecycle_change();
    }
}

/// 'withOpenDatabase' (:87-130).
///
/// The action takes `(&Arc<DB>, &Arc<OpenDB>)` (DIVERGENCES 2): row 26's
/// record split moved `odbWriting`/`odbIdleSince` behind `DbLifecycle`,
/// so a consumer that reads them (withWritableDatabase) needs the DB
/// handle; the wrappers below keep the source's callback shapes.
pub fn with_open_database<A, F>(env: &Env, repo: &thrift::Repo, action: F) -> Result<A, OpenError>
where
    F: FnOnce(&Arc<DB>, &Arc<OpenDB>) -> Result<A, OpenError>,
{
    with_active_database(env, repo, |db| {
        let odb = match cas_open_state(env, db)? {
            OpenCas::AlreadyOpen(odb) => odb,
            OpenCas::Proceed { version, mode } => {
                // :117 — the SECOND meta read, a separate transaction in
                // the source too (`meta <- atomically $ Catalog.readMeta
                // envCatalog dbRepo`).
                let meta = env.env_catalog.read_meta(&db.db_repo)?;
                let deps = meta.meta_dependencies.clone();
                // :120 — `onFailure ex = atomically $ Catalog.dbFailed
                // envCatalog dbRepo ex` (row 22's Open.hs:449-450 site).
                // The source's on_failure runs ONCE (the inner
                // handling_failures; the outer wrapper only guarded
                // spawnMask itself — see async_open_db's doc). A
                // db_failed failure replaces the propagated error, as
                // the handler's own throw would.
                let on_failure = {
                    let env = env;
                    let repo = db.db_repo.clone();
                    move |exc: &Exc| -> Result<(), Exc> { db_failed_exc(env, &repo, exc) }
                };
                // "opening a DB has long uninterruptible sections so do
                // it on a separate thread in case we get cancelled"
                // (:121-122) — the spawn; the wait rethrows the task's
                // failure (the `restore $ wait opener` of :127).
                with_storage_for(env, &db.db_repo, &meta, |storage| {
                    let opener = async_open_db(
                        env,
                        storage,
                        db,
                        version,
                        mode,
                        deps.as_ref(),
                        // :125 — `(return ())`, the no-op on_success.
                        || Ok(()),
                        on_failure,
                    );
                    match catch_unwind(AssertUnwindSafe(|| opener.wait_once())) {
                        Ok(odb) => Ok(odb),
                        Err(payload) => match payload.downcast::<BodyFunnelled>() {
                            // A body failure: the inner handling_failures
                            // already ran (dbState := Closed + log +
                            // on_failure) — propagate verbatim.
                            Ok(f) => Err(OpenError::Other(f.0)),
                            // The spawn-cancelled image (the warden was
                            // shut down; the task finished cancelled and
                            // the body NEVER ran) and any other unfunnelled
                            // task failure: run the source's OUTER funnel
                            // here, BEFORE propagating — Open.hs:754-759's
                            // failure branch (`writeTVar dbState Closed` ->
                            // logError -> on_failure -> rethrow), which the
                            // source executes synchronously around
                            // spawnMask (:689-690) and this port defers to
                            // the waiter. Without it a cancelled open
                            // strands dbState = Opening forever (every
                            // future cas_open_state/schema_updated waiter
                            // parks; Close blocks).
                            Err(payload) => {
                                let exc = downcast_exc(payload);
                                // downcast_exc constructs `OpenError::Other`
                                // only — the on_failure leg needs the boxed
                                // exception itself; the catch-all keeps the
                                // extraction total.
                                let inner: Exc = match exc {
                                    OpenError::Other(e) => e,
                                    other => Box::new(other),
                                };
                                {
                                    let mut life = lock(&db.db_lifecycle);
                                    life.db_state = DBState::Closed;
                                    // The :756 write discards the whole
                                    // record — the row-26 split's
                                    // odb_writing goes with it (the
                                    // types.rs:1187-1189 invariant).
                                    life.odb_writing = None;
                                }
                                log_error(|| {
                                    in_repo(&db.db_repo, &format!("couldn't open: {}", inner))
                                });
                                match db_failed_exc(env, &db.db_repo, &inner) {
                                    Ok(()) => Err(OpenError::Other(inner)),
                                    Err(e2) => Err(OpenError::Other(e2)),
                                }
                            }
                        },
                    }
                })?
            }
        };
        // :128-130 — `action odb `finally` do t <- getTimePoint;
        // atomically $ writeTVar (odbIdleSince odb) t`. The idle write
        // runs on BOTH exits; `odbIdleSince` is `DbLifecycle`
        // .odb_idle_since (row 26).
        let result = action(db, &odb);
        let t = Instant::now();
        lock(&db.db_lifecycle).odb_idle_since = t;
        result
    })
}

/// 'withOpenDatabaseStack' (:133-140) — "Runs the action on each DB in
/// the stack, in top-to-bottom order": `mapM (\repo ->
/// withOpenDatabase env repo action) (repo : parents)`. Each repo is an
/// independent bracket; `mapM` short-circuits on the first failure.
pub fn with_open_database_stack<A, F>(
    env: &Env,
    repo: &thrift::Repo,
    action: F,
) -> Result<Vec<A>, OpenError>
where
    F: Fn(&Arc<DB>, &Arc<OpenDB>) -> Result<A, OpenError>,
{
    let parents = repo_parents(env, repo)?;
    let mut out = Vec::with_capacity(parents.len() + 1);
    out.push(with_open_database(env, repo, |db, odb| action(db, odb))?);
    for parent in parents {
        out.push(with_open_database(env, &parent, |db, odb| action(db, odb))?);
    }
    Ok(out)
}

/// 'repoParents' (:142-150) — the parent chain, top-to-bottom
/// (`Nothing` ends it). One readMeta per level.
pub fn repo_parents(env: &Env, repo: &thrift::Repo) -> Result<Vec<thrift::Repo>, OpenError> {
    fn go(env: &Env, repo: &thrift::Repo, out: &mut Vec<thrift::Repo>) -> Result<(), OpenError> {
        let meta = env.env_catalog.read_meta(repo)?;
        match &meta.meta_dependencies {
            None => Ok(()),
            Some(dep) => {
                let parent = dep_parent(dep);
                out.push(parent.clone());
                go(env, &parent, out)
            }
        }
    }
    let mut out = Vec::new();
    go(env, repo, &mut out)?;
    Ok(out)
}

/// 'repoParent' (:152-155).
pub fn repo_parent(env: &Env, repo: &thrift::Repo) -> Result<Option<thrift::Repo>, OpenError> {
    let meta = env.env_catalog.read_meta(repo)?;
    Ok(meta.meta_dependencies.as_ref().map(dep_parent))
}

/// 'depParent' (:157-161) — the bare IDL field names (the
/// ASSUMED-IMPORTS pin; catalog.rs:892-901 is the shipped precedent).
pub fn dep_parent(deps: &thrift::Dependencies) -> thrift::Repo {
    match deps {
        thrift::Dependencies::Pruned(pruned) => pruned.base.clone(),
        thrift::Dependencies::Stacked(stacked) => thrift::Repo {
            name: stacked.name.clone(),
            hash: stacked.hash.clone(),
        },
    }
}

// ---------------------------------------------------------------------------
// The lookup plumbing (withOpenDBLookup / withOpenDBStack / the ACL slices)
// ---------------------------------------------------------------------------

/// `withCanLookup`'s image inlined: `withLookup lookup $ \p -> f (Lookup p
/// (lookupName lookup))` (Lookup.hs:47-49). Used wherever the source
/// passes the `Lookup` VALUE down another level — the Rust view is
/// non-owning, so the port re-points a fresh view over the same ptr
/// (DIVERGENCES 5).
fn lookup_view(lookup: &Lookup) -> Lookup {
    Lookup::new(lookup.ptr(), lookup.name())
}

/// 'withOpenDBLookup' (:163-187) — the boundaries + ACL-slice wrapper
/// around one layer's lookup (flat) or the whole stack (stacked).
///
/// The `Lookup` plumbing carries OWNED views (`Lookup` is not Clone and
/// `CanLookup` is implemented by value), so each level re-points via
/// [`lookup_view`] — the SIBLING-UNIT CONTRACTS lookup note.
fn with_open_db_lookup<T, F>(
    env: &Env,
    repo: &thrift::Repo,
    odb: &OpenDB,
    acl_enabled: bool,
    f: F,
) -> Result<T, OpenError>
where
    // `f :: Boundaries -> Lookup -> IO a` — the source passes the
    // boundaries BY VALUE and the lookup as the shared handle; the port
    // hands a fresh view. `F: Fn` because both arms use it.
    F: Fn(&OpenDB, Boundaries, &Lookup) -> Result<T, OpenError>,
{
    odb.odb_handle.with_lookup(|top_ptr| {
        let lookup = Lookup::new(top_ptr, odb.odb_handle.lookup_name());
        match repo_parent(env, repo)? {
            None => {
                // :175-178 — flat: boundaries over this layer's lookup,
                // then the flat ACL slice.
                let bounds = flat_boundaries(&lookup)?;
                with_flat_acl_lookup(env, repo, odb, acl_enabled, lookup_view(&lookup), |sliced| {
                    f(odb, bounds, sliced)
                })
            }
            Some(base_repo) => {
                // :179-187 — stacked: open the base, stack the lookups,
                // stack the ACL slices.
                with_open_database(env, &base_repo, |_db, base_odb| {
                    base_odb.odb_handle.with_lookup(|base_ptr| {
                        let base_lookup = Lookup::new(base_ptr, base_odb.odb_handle.lookup_name());
                        with_open_db_stack(env, &base_repo, base_lookup, |base| {
                            let bounds = stacked_boundaries(base, &lookup)?;
                            // `let ownershipSlices = catMaybes baseSlices`
                            // (:184) — the base stack's pre-built slices,
                            // shared by reference in the source. See the
                            // TODO at build_open_db's storeSlices site:
                            // the same Slice-Clone gap.
                            let ownership_slices: Vec<Slice> = odb
                                .odb_base_slices
                                .iter()
                                .filter_map(|s| s.as_ref().map(|s| s.clone()))
                                .collect();
                            with_stacked_acl_lookup(
                                env,
                                repo,
                                odb,
                                &base_repo,
                                base_odb,
                                acl_enabled,
                                ownership_slices,
                                base,
                                lookup_view(&lookup),
                                |stacked_lookup| f(odb, bounds, stacked_lookup),
                            )
                        })
                    })
                })
            }
        }
    })
}

/// 'withOpenDBStack' (:189-203) — stack the base lookups under `lookup`.
/// Each recursion level wraps `f` with one more `Stacked.stacked`,
/// exactly as the source's `Just baseRepo -> ... $ \base -> withCanLookup
/// (Stacked.stacked base lookup) f` does; the leaf calls the original f
/// with the fully-stacked lookup. The recursion's `lookup` moves down
/// OWNED (the source's by-value handle).
fn with_open_db_stack<T, F>(
    env: &Env,
    repo: &thrift::Repo,
    lookup: Lookup,
    f: F,
) -> Result<T, OpenError>
where
    F: Fn(&Lookup) -> Result<T, OpenError>,
{
    match repo_parent(env, repo)? {
        None => f(&lookup),
        Some(base_repo) => with_open_database(env, &base_repo, |_db, base_odb| {
            base_odb.odb_handle.with_lookup(|base_ptr| {
                let base_lookup = Lookup::new(base_ptr, base_odb.odb_handle.lookup_name());
                with_open_db_stack(env, &base_repo, base_lookup, |base| {
                    // `Stacked.stacked base lookup` — both sides are the
                    // source's shared handles; fresh views per call keep
                    // this an `Fn` (f is only ever borrowed).
                    with_can_lookup(&stacked(lookup_view(base), lookup_view(&lookup)), &f)
                })
            })
        }),
    }
}

/// 'withWritableDatabase' (:205-212) — the action receives the source's
/// exact tuple `(WriteQueue, DbSchema, Some DatabaseOps)` as borrows
/// (DIVERGENCES 2).
pub fn with_writable_database<A, F>(
    env: &Env,
    repo: &thrift::Repo,
    action: F,
) -> Result<A, OpenError>
where
    F: FnOnce(&Arc<WriteQueue>, &DbSchema, &SomeDatabaseOps) -> Result<A, OpenError>,
{
    with_open_database(env, repo, |db, odb| {
        // `case odbWriting of` — a pure record read in the source; the
        // row-26 split puts it behind the DB lock (one hold, nothing
        // else taken inside).
        let writing = lock(&db.db_lifecycle).odb_writing.clone();
        match writing {
            Some(w) => action(&w.wr_queue, &odb.odb_schema, &odb.odb_handle),
            None => db_error(repo, "can't write to a read-only database").map_err(OpenError::from),
        }
    })
}

/// 'readDatabase' (:214-221).
pub fn read_database<T, F>(env: &Env, repo: &thrift::Repo, f: F) -> Result<T, OpenError>
where
    F: Fn(&OpenDB, &Lookup) -> Result<T, OpenError>,
{
    read_database_with_boundaries(env, repo, false, |odb, _bounds, lookup| f(odb, lookup))
}

/// 'readDatabaseWithBoundaries' (:223-232).
pub fn read_database_with_boundaries<T, F>(
    env: &Env,
    repo: &thrift::Repo,
    acl_enabled: bool,
    f: F,
) -> Result<T, OpenError>
where
    F: Fn(&OpenDB, Boundaries, &Lookup) -> Result<T, OpenError>,
{
    with_open_database(env, repo, |_db, odb| {
        with_open_db_lookup(env, repo, odb, acl_enabled, |odb, bounds, lookup| {
            f(odb, bounds, lookup)
        })
    })
}

// ---------------------------------------------------------------------------
// ACL Slice Construction (the section's source doc comments, kept)
// ---------------------------------------------------------------------------

/// 'resolveAclGroupUnits' (:239-245) — "Resolve ACL group names to their
/// unit ids in this DB. Each group is stored as an "acl:<name>" ownership
/// unit; names that don't resolve are dropped."
fn resolve_acl_group_units(handle: &SomeDatabaseOps, names: &[String]) -> Result<Vec<u32>, Exc> {
    let mut out = Vec::new();
    for name in names {
        let unit_name = format!("acl:{}", name).into_bytes();
        if let Some(unit_id) = handle.get_unit_id(&unit_name).map_err(into_exc)? {
            let UnitId(uid) = unit_id;
            out.push(uid);
        }
    }
    Ok(out)
}

/// 'loadAclMetadata' (:250-262) — "Read a DB's ACL metadata at open
/// time: the firstACLID boundary and the group name -> UsetId mapping.
/// Both are DB-invariant; empty/Nothing for DBs without ACLs."
fn load_acl_metadata(
    handle: &SomeDatabaseOps,
) -> Result<(Option<UsetId>, BTreeMap<String, UsetId>), Exc> {
    let m_first_acl_id = retrieve_first_acl_id(handle).map_err(into_exc)?;
    let m_mapping = retrieve_acl_group_mapping(handle).map_err(into_exc)?;
    let acl_mapping = match m_mapping {
        None => BTreeMap::new(),
        Some(parsed) => parsed
            .into_iter()
            .map(|(name, uid)| (name, UsetId(uid)))
            .collect(),
    };
    Ok((m_first_acl_id, acl_mapping))
}

/// 'buildLayerACLSlice' (:279-304) — "Build an ACL-aware ownership slice
/// for a single DB layer, and pass it to a continuation. Passes Nothing
/// if ACL filtering is not needed for this layer." The EXCLUDE-mode
/// rationale in the source doc comment (:264-278) is the design contract
/// for [`build_layer_acl_slice_for_groups`].
fn build_layer_acl_slice<T, F>(
    env: &Env,
    repo: &thrift::Repo,
    odb: &OpenDB,
    acl_enabled: bool,
    f: F,
) -> Result<T, OpenError>
where
    F: FnOnce(Option<Slice>) -> Result<T, OpenError>,
{
    if !acl_enabled {
        vlog(1, || {
            "[ACL-Slice] ACL filtering disabled, skipping ACL slice".to_string()
        });
        return f(None);
    }
    if odb.odb_acl_mode == AclMode::AclDisabled {
        vlog(1, || {
            "[ACL-Slice] ACL mode disabled, skipping ACL slice".to_string()
        });
        return f(None);
    }
    // "Resolve the caller's subset of THIS layer's ACL group names..." —
    // the per-layer resolution note in the source comment (:294-298),
    // kept.
    let candidates: Vec<String> = odb.odb_acl_mapping.keys().cloned().collect();
    let acl_group_names = if candidates.is_empty() {
        Vec::new()
    } else {
        (env.env_resolve_acl_groups)(&candidates).map_err(other)?
    };
    build_layer_acl_slice_for_groups(repo, odb, &acl_group_names, f)
}

/// 'buildLayerACLSliceForGroups' (:311-351) — "Build the ACL ownership
/// slice for a single layer given the caller's already-resolved subset of
/// that layer's ACL group names." Both `Nothing` cases fail CLOSED via
/// dbError, exactly as the source does.
fn build_layer_acl_slice_for_groups<T, F>(
    repo: &thrift::Repo,
    odb: &OpenDB,
    acl_group_names: &[String],
    f: F,
) -> Result<T, OpenError>
where
    F: FnOnce(Option<Slice>) -> Result<T, OpenError>,
{
    let first_acl = match odb.odb_first_acl_id {
        None => {
            return db_error(
                repo,
                "ACL-enabled DB has no firstACLID; refusing to serve unfiltered results (fail closed)",
            )
            .map_err(OpenError::from)
        }
        Some(UsetId(first_acl)) => first_acl,
    };
    // `maybeOwnership <- readTVarIO (odbOwnership odb)` (:324) — the
    // ownership Mutex alone, never nested inside the DB lock (row 26's
    // clause; the source's readTVarIO is likewise outside any
    // transaction).
    // INVENTORY-BUG-CANDIDATE row 26 (partial): ownership snapshot taken
    // under its own Mutex, not nested under DB — matches the source's
    // out-of-transaction readTVarIO (Open.hs:324/:843); rationale at the
    // header's row-26 clause (the Ownership.hs:46/UserQuery.hs:813
    // bullet).
    let maybe_ownership = lock(&odb.odb_ownership).clone();
    let ownership = match maybe_ownership {
        None => {
            return db_error(
                repo,
                "ACL-enabled DB has no ownership data; refusing to serve unfiltered results (fail closed)",
            )
            .map_err(OpenError::from)
        }
        Some(o) => o,
    };
    // "All ACL-group units occupy the contiguous range [firstACLID,
    // firstACLID + #groups) above every regular file unit" (:331-333).
    let num_groups = odb.odb_acl_mapping.len();
    let all_groups: Vec<u32> = if num_groups == 0 {
        Vec::new()
    } else {
        (first_acl..first_acl + num_groups as u32).collect()
    };
    let user_groups = resolve_acl_group_units(&odb.odb_handle, acl_group_names).map_err(other)?;
    let user_set: std::collections::HashSet<u32> = user_groups.into_iter().collect();
    vlog(1, || {
        format!(
            "[ACL-Slice] (exclude) allGroups={}, userGroups={}",
            all_groups.len(),
            user_set.len()
        )
    });
    // "Exclude (in EXCLUDE mode) the ACL-group units the caller is NOT a
    // member of; everything else stays visible by default." (:343-344).
    // `sort (filter ...)` — ascending by construction here, so the
    // collect order IS the sorted order.
    let exclude_units: Vec<UnitId> = all_groups
        .into_iter()
        .filter(|u| !user_set.contains(u))
        .map(UnitId)
        .collect();
    let acl_slice = slice(&ownership, &[], &exclude_units, true)?;
    vlog(1, || "[ACL-Slice] Slice created successfully".to_string());
    f(Some(acl_slice))
}

/// 'withMaybeSliced' (:357-361) — "Wrap a lookup with the given
/// ownership/ACL slices, or pass it through unchanged when there are
/// none. An empty list must NOT be handed to slicedStack: an empty
/// Slices treats every UsetId as not-covered and hides all facts." (The
/// warning is the source's own; the port's `is_empty` arm is it.)
fn with_maybe_sliced<T, F>(slices: Vec<Slice>, lookup: Lookup, f: F) -> Result<T, OpenError>
where
    F: FnOnce(Lookup) -> Result<T, OpenError>,
{
    if slices.is_empty() {
        f(lookup)
    } else {
        with_can_lookup(&sliced_stack(slices, lookup), |l| f(lookup_view(l)))
    }
}

/// 'withFlatACLLookup' (:365-375).
fn with_flat_acl_lookup<T, F>(
    env: &Env,
    repo: &thrift::Repo,
    odb: &OpenDB,
    acl_enabled: bool,
    lookup: Lookup,
    f: F,
) -> Result<T, OpenError>
where
    F: FnOnce(&Lookup) -> Result<T, OpenError>,
{
    build_layer_acl_slice(env, repo, odb, acl_enabled, |m_acl_slice| {
        with_maybe_sliced(m_acl_slice.into_iter().collect(), lookup, |sliced| f(&sliced))
    })
}

/// 'withStackedACLSlices' (:380-398) — "Build ACL slices for all layers
/// in a stacked DB (base layers only). Walks the DB stack recursively and
/// builds an ACL slice for each layer that has ACL data (firstACLID).
/// Returns the list of ACL slices."
fn with_stacked_acl_slices<T, F>(
    env: &Env,
    base_repo: &thrift::Repo,
    base_odb: &OpenDB,
    acl_enabled: bool,
    f: F,
) -> Result<T, OpenError>
where
    F: FnOnce(Vec<Slice>) -> Result<T, OpenError>,
{
    build_layer_acl_slice(env, base_repo, base_odb, acl_enabled, |m_slice| {
        match repo_parent(env, base_repo)? {
            None => f(m_slice.into_iter().collect()),
            Some(parent_repo) => with_open_database(env, &parent_repo, |_db, parent_odb| {
                with_stacked_acl_slices(env, &parent_repo, parent_odb, acl_enabled, |parent_slices| {
                    let mut all: Vec<Slice> = m_slice.into_iter().collect();
                    all.extend(parent_slices);
                    f(all)
                })
            }),
        }
    })
}

/// 'withStackedACLLookup' (:404-426) — "Apply ownership + ACL slices for
/// a stacked database and run the continuation with the combined
/// stacked/stacked lookup." The withMaybeSliced pair guards both sides
/// against the empty-list-hides-everything hazard (:422-423, kept).
#[allow(clippy::too_many_arguments)]
fn with_stacked_acl_lookup<T, F>(
    env: &Env,
    repo: &thrift::Repo,
    odb: &OpenDB,
    base_repo: &thrift::Repo,
    base_odb: &OpenDB,
    acl_enabled: bool,
    ownership_slices: Vec<Slice>,
    base: &Lookup,
    lookup: Lookup,
    f: F,
) -> Result<T, OpenError>
where
    F: FnOnce(&Lookup) -> Result<T, OpenError>,
{
    with_stacked_acl_slices(env, base_repo, base_odb, acl_enabled, |base_acl_slices| {
        build_layer_acl_slice(env, repo, odb, acl_enabled, |m_top_acl_slice| {
            let base_merged: Vec<Slice> = ownership_slices
                .into_iter()
                .chain(base_acl_slices)
                .collect();
            with_maybe_sliced(base_merged, lookup_view(base), |sliced_base| {
                with_maybe_sliced(
                    m_top_acl_slice.into_iter().collect(),
                    lookup,
                    |sliced_top| {
                        with_can_lookup(&stacked(sliced_base, sliced_top), |st| f(lookup_view(st)))
                    },
                )
            })
        })
    })
}

// ---------------------------------------------------------------------------
// setupSchema
// ---------------------------------------------------------------------------

/// `HashMap.fromListWith (<>)`'s image (Open.hs:546): componentwise sum
/// over the generated `PredicateStats { count, size }` (the ASSUMED-
/// IMPORTS pin; the Haskell Semigroup on the record sums both fields).
fn merge_predicate_stats(
    acc: &mut HashMap<Pid, thrift::PredicateStats>,
    pid: Pid,
    stats: thrift::PredicateStats,
) {
    let entry = acc
        .entry(pid)
        .or_insert(thrift::PredicateStats { count: 0, size: 0 });
    entry.count += stats.count;
    entry.size += stats.size;
}

/// `stackStats` (:539-546) — this DB's stats plus every parent's, merged
/// componentwise. Opens each parent through `withOpenDatabase` (the
/// source's own recursion).
fn stack_stats(
    env: &Env,
    repo: &thrift::Repo,
    handle: &SomeDatabaseOps,
) -> Result<HashMap<Pid, thrift::PredicateStats>, Exc> {
    let parents = repo_parents(env, repo).map_err(into_exc)?;
    let mut acc: HashMap<Pid, thrift::PredicateStats> = HashMap::new();
    // `concat (stats : statss)` — the source list starts with THIS
    // handle's stats; fromListWith sums, so the fold order is
    // unobservable (commutative).
    let own = handle.predicate_stats().map_err(into_exc)?;
    for (pid, stats) in own {
        merge_predicate_stats(&mut acc, pid, stats);
    }
    for parent in &parents {
        let v = with_open_database(env, parent, |_db, odb| {
            odb.odb_handle.predicate_stats().map_err(other)
        })
        .map_err(into_exc)?;
        for (pid, stats) in v {
            merge_predicate_stats(&mut acc, pid, stats);
        }
    }
    Ok(acc)
}

/// The `Create _ _ initial _` payload's image, extracted BEFORE
/// `Storage.open` consumes the `Mode` (whose `Create` arm carries the
/// non-Clone `Ownership`, so `Mode` itself cannot be cloned). Rust-only
/// (the source re-scrutinises the value at :706); `StoredSchema: Clone`
/// is the generated record's (ASSUMED-IMPORTS).
enum SetupSchema {
    UseDefaultSchema,
    UseSpecificSchema(String),
    UseThisSchema(StoredSchema),
}

impl SetupSchema {
    fn of_create(schema: &CreateSchema) -> SetupSchema {
        match schema {
            CreateSchema::UseDefaultSchema => SetupSchema::UseDefaultSchema,
            CreateSchema::UseSpecificSchema(id) => SetupSchema::UseSpecificSchema(id.clone()),
            CreateSchema::UseThisSchema(info) => SetupSchema::UseThisSchema(info.clone()),
        }
    }
}

/// 'setupSchema' (:506-565) — the two-mode schema loader. The task-frame
/// error channel is `Exc` (it runs inside async_open_db's thread and
/// under schema_updated's loggingAction; both are exception-shaped).
/// `initial` is `Some` iff the mode was `Create{}` (see `SetupSchema`);
/// `read_only` is the `ReadOnly <- mode` guard of :530.
fn setup_schema(
    env: &Env,
    repo: &thrift::Repo,
    handle: &SomeDatabaseOps,
    initial: Option<&SetupSchema>,
    read_only: bool,
) -> Result<DbSchema, Exc> {
    // The Create arm (:513-525).
    if let Some(initial) = initial {
        let index = env.env_schema_source.get();
        let db_schema = match initial {
            SetupSchema::UseDefaultSchema => new_db_schema(
                Some(&env.env_db_schema_cache),
                &index,
                &SchemaSelector::LatestSchema,
                &read_write_content(),
                &env.env_debug,
            )
            .map_err(into_exc)?,
            SetupSchema::UseSpecificSchema(schema_id) => new_db_schema(
                Some(&env.env_db_schema_cache),
                &index,
                &SchemaSelector::SpecificSchemaId(schema_id.clone()),
                &read_write_content(),
                &env.env_debug,
            )
            .map_err(into_exc)?,
            SetupSchema::UseThisSchema(info) => from_stored_schema(
                Some(&env.env_db_schema_cache),
                info,
                &read_write_content(),
                &env.env_debug,
            )
            .map_err(into_exc)?,
        };
        store_schema(handle, &to_stored_schema(&db_schema)).map_err(into_exc)?;
        return Ok(db_schema);
    }
    // The open arm (:526-565).
    let stored = retrieve_schema(repo, handle).map_err(into_exc)?;
    match stored {
        Some(info) => {
            if read_only {
                // `mergeSchema` (:548-555) — "merge the schema in the DB
                // with the current schema, so that we can query for
                // derived predicates that weren't stored in the DB when
                // it was created." (The source comment, kept.)
                let index = env.env_schema_source.get();
                let stats = stack_stats(env, repo, handle)?;
                new_merged_db_schema(
                    Some(&env.env_db_schema_cache),
                    &info,
                    &index,
                    &read_only_content(stats),
                    &env.env_debug,
                )
                .map_err(into_exc)
            } else {
                // "while writing, we don't allow new predicates to be
                // added to the schema. This is the easiest way to prevent
                // facts being added to the DB that aren't in the original
                // stored schema. It might be possible to do something
                // safe here, e.g. only merge non-stored derived
                // predicates. (TODO)" — the source's own TODO, kept.
                from_stored_schema(
                    Some(&env.env_db_schema_cache),
                    &info,
                    &read_write_content(),
                    &env.env_debug,
                )
                .map_err(into_exc)
            }
        }
        None => {
            let meta = env.env_catalog.read_meta(repo).map_err(into_exc)?;
            let failure = match &meta.meta_completeness {
                Completeness::Broken(broken) => {
                    // `Text.unwords $ ["Broken:"] ++ [task <> ":" | not
                    // (Text.null task)] ++ [reason]` (:558-564).
                    let mut parts = vec!["Broken:".to_string()];
                    if !broken.task.is_empty() {
                        parts.push(format!("{}:", broken.task));
                    }
                    parts.push(broken.reason.clone());
                    parts.join(" ")
                }
                _ => String::new(),
            };
            db_error(repo, &format!("DB has no stored schema. {}", failure)).map_err(into_exc)
        }
    }
}

// ---------------------------------------------------------------------------
// schemaUpdated
// ---------------------------------------------------------------------------

/// 'schemaUpdated' (:577-639) — "Update the schema for all open DBs when
/// the prevailing schema has changed."
///
/// Preconditions (the source doc comment, kept):
///   * when schema_updated is invoked, calls to `Observed.get` on
///     `env_schema_source` are guaranteed to get the updated schema.
///   * schema_updated does not run concurrently with itself.
pub fn schema_updated(env: &Env, mb_repo: Option<&thrift::Repo>) -> Result<(), OpenError> {
    // The acquire transaction (:588-591): snapshot the (filtered)
    // active DBs and acquireDB each — ONE hold of `env_active` with the
    // DB locks nested (row 26's ENV_ACTIVE -> DB), so no concurrent
    // release can remove a DB between the snapshot and its acquire.
    let selected: Vec<Arc<DB>> = {
        let active = lock(&env.env_active);
        let sel: Vec<Arc<DB>> = match mb_repo {
            None => active.values().cloned().collect(),
            Some(repo) => active.get(repo).cloned().into_iter().collect(),
        };
        for db in &sel {
            acquire_db(db);
        }
        sel
    };

    // The bracket body (:602-639).
    let body = (|| -> Result<(), OpenError> {
        // "Empty the DbSchema cache. We probably have a new SchemaIndex
        // now which invalidates all the entries anyway, and also we
        // don't prune this cache anywhere else." (:594-596 — the source
        // comment, kept.) `modifyMVar_ envDbSchemaCache $ \_ -> return
        // HashMap.empty` — the MVar-as-mutex reading (types.rs:1565-1568).
        if mb_repo.is_none() {
            *lock(&env.env_db_schema_cache) = HashMap::new();
            // "previously failed DBs might now open successfully" (:599).
            env.env_catalog.reset_failed()?;
        }
        for db in &selected {
            // :604-610 — the per-DB state read; `Opening -> retry` parks
            // (the same wait primitive as the open CAS). The :612
            // `case odbWriting of` reads the SAME lifecycle (row 26), so
            // both leaves come from one hold's snapshot.
            let maybe = loop {
                let life = lock(&db.db_lifecycle);
                match &life.db_state {
                    DBState::Opening => {
                        drop(life);
                        wait_for_lifecycle_change();
                        continue;
                    }
                    DBState::Open(odb) => {
                        let writing = life.odb_writing.is_some();
                        break Some((Arc::clone(odb), writing));
                    }
                    DBState::Closing | DBState::Closed => break None,
                }
            };
            if let Some((odb, writing)) = maybe {
                if writing {
                    // :613-618 — "see setupSchema above, we don't update
                    // the schema for a writable DB." (The source comment,
                    // kept.)
                    log_info(|| {
                        format!(
                            "not updating schema for writable DB: {}",
                            show_repo(&db.db_repo)
                        )
                    });
                } else {
                    log_info(|| format!("updating schema for: {}", show_repo(&db.db_repo)));
                    // :621-624 — `tryAll $ loggingAction (runLogRepo
                    // "update-schema" env dbRepo) (const mempty) $
                    // setupSchema env dbRepo odbHandle ReadOnly`. The
                    // loggingAction logs the failure INSIDE (its
                    // failureLog leg) and rethrows; tryAll turns it into
                    // `Left`.
                    let r = log_action(env, "update-schema", &db.db_repo, || {
                        setup_schema(env, &db.db_repo, &odb.odb_handle, None, true)
                    });
                    match r {
                        Err(err) => {
                            // :626-627.
                            log_error(|| {
                                format!(
                                    "schema update for {} failed: {}",
                                    show_repo(&db.db_repo),
                                    err
                                )
                            });
                        }
                        Ok(schema) => {
                            // :628-638 — the commit transaction. The source
                            // RE-READS dbState and updates THAT record
                            // (Open.hs:629/:636), so the snapshot Arc is
                            // dropped first: holding it pinned the record
                            // shared and made every swap fail (see
                            // commit_schema_update).
                            drop(odb);
                            commit_schema_update(&db.db_lifecycle, schema);
                        }
                    }
                }
            }
        }
        Ok(())
    })();

    // The bracket release (:592): release every acquired DB.
    // INVENTORY-BUG-CANDIDATE row 26 (partial): batched release split into
    // per-DB critical sections — an observer can see half the batch
    // released; source Open.hs:592 (`atomically . mapM_ (releaseDB ...)`).
    for db in &selected {
        release_db(env, db);
    }
    body?;
    // :639.
    log_info(|| "done updating schema for open DBs".to_string());
    Ok(())
}

/// The Open.hs:628-638 commit transaction: under the DB lock,
/// `Opening -> return ()` ("if we are Opening now, this must have
/// happened after the transaction above, so it will already pick up the
/// new schema" — the source comment, kept), `Closing`/`Closed` -> no-op,
/// `Open odb -> writeTVar dbState $ Open odb { odbSchema = schema }`.
///
/// INVENTORY-BUG-CANDIDATE rows 26/104: the rows' evidence (and types.rs's
/// `DBState::Open(Arc<OpenDB>)` image) do not cover this third dbState
/// swap — the schema update REPLACES the OpenDB record's `odbSchema`
/// (Open.hs:636). The faithful image is the record update itself: old
/// readers keep the old schema, new readers get the new one. The shipped
/// `OpenDB` cannot express the unconditional form — `odb_schema` is a
/// plain field (types.rs:1137) and the record is neither `Clone`
/// (`SomeDatabaseOps`/`Slice` are not) nor interior-mutable — so the
/// port mutates the record IN PLACE through the existing DbLifecycle
/// lock whenever the record is unshared (`Arc::get_mut`; no take-out /
/// put-back, no `Arc::try_unwrap`). The earlier draft's swap NEVER
/// landed: its `Arc::try_unwrap` raced schema_updated's own snapshot
/// Arc, which was still alive at the commit (the "contention-dependent
/// skip window" the old flag claimed did not exist — the skip was
/// unconditional). schema_updated now drops that snapshot before
/// committing (the source re-reads the record from the TVar at
/// Open.hs:629, so the commit must not depend on the snapshot), and the
/// only remaining holders are concurrent `with_open_database` handbacks
/// — each holds one Arc clone for its action's duration, and while one
/// does, the swap is skipped AND LOST (the source would commit the new
/// record to the TVar at once; the port's next handback still sees the
/// old schema).
///
/// TODO(port): give `OpenDB.odb_schema` interior mutability in types.rs
/// (row 104's owner — the same shape as `odb_ownership`'s own
/// `Mutex<Option<Arc<Ownership>>>`, which exists for the identical
/// "value changes over the DB's life" reason), then this becomes an
/// unconditional store under the DB lock and the concurrent-handback
/// window closes.
fn commit_schema_update(lifecycle: &Mutex<DbLifecycle>, schema: DbSchema) {
    let mut life = lock(lifecycle);
    // `readTVar dbState` + the four-way case, one hold: the Open arm
    // mutates the record in place; the other arms (:631/:637/:638) are
    // no-ops.
    if let DBState::Open(odb) = &mut life.db_state {
        if let Some(record) = Arc::get_mut(odb) {
            record.odb_schema = schema;
        }
    }
}

// ---------------------------------------------------------------------------
// setupWriting
// ---------------------------------------------------------------------------

/// 'setupWriting' (:642-665). The `lookup` parameter is the CanLookup the
/// source abstracts over — the port takes the concrete handle (the only
/// caller passes `odbHandle`) and points a view at it.
fn setup_writing(env: &Env, handle: &SomeDatabaseOps) -> Result<Writing, Exc> {
    handle.with_lookup(|ptr| {
        let lookup = Lookup::new(ptr, handle.lookup_name());
        let scfg: ServerConfig = env.env_server_config.get();
        // "Convert to Word64 from Int32 to prevent silent truncation."
        // (:645 — the source comment, kept. `fromIntegral` Int32 ->
        // Word64 is the two's-complement wrap: a negative config value
        // becomes a huge capacity, which the allocation then rejects —
        // the source's own arithmetic.)
        let cache_limit_mb: u64 = scfg.config_db_lookup_cache_limit_mb as u64;
        // `cache_limit_mb * 1024 * 1024` in Word64 arithmetic — wrapping,
        // as Haskell's Word64 `*` does (RULE §2 numeric-conversions row).
        let capacity = cache_limit_mb.wrapping_mul(1024).wrapping_mul(1024);
        // `fromIntegral $ config_db_writer_threads scfg` — Int32 -> the
        // C `int`/`usize` pair; a negative value sign-extends to a huge
        // shard count exactly as the source's fromIntegral -> CSize does
        // (RULE §2 A5).
        let shards = scfg.config_db_writer_threads as usize;
        let lookup_cache =
            LookupCache::new(capacity, shards, &env.env_lookup_cache_stats).map_err(into_exc)?;
        // `next_id <- newIORef =<< Lookup.firstFreeId lookup` (:652).
        let next_id = first_free_id(&lookup).map_err(into_exc)?;
        // `mutex <- newMutex (Storage.WriteLock ())` (:653) — the
        // unit-102 mutex reading (types.rs:1090-1092).
        let mutex = Mutex::new(WriteLock(()));
        // `queue <- WriteQueue <$> newTQueueIO <*> newTVarIO 0 <*>
        // newTVarIO 0 <*> newTVarIO 0 <*> newTVarIO 0` (:654-655) — the
        // empty queue + the four zero counters (types.rs's
        // WriteQueueState fields, in source order
        // queue/active/count/size/latency).
        let queue = Arc::new(WriteQueue {
            write_queue_state: Mutex::new(WriteQueueState {
                queue: VecDeque::new(),
                active: 0,
                count: 0,
                size: 0,
                latency: Duration::ZERO,
            }),
            write_queue_changed: Condvar::new(),
        });
        // `anchorName <- newTVarIO Nothing` (:656); `commit <- newTVarIO
        // Nothing` (:657) — inside the row-34 WriteState.
        Ok(Writing {
            wr_lock: mutex,
            wr_state: Mutex::new(WriteState {
                wr_next_id: next_id,
                wr_commit: None,
            }),
            wr_lookup_cache: lookup_cache,
            wr_lookup_cache_anchor_name: Mutex::new(None),
            wr_queue: queue,
        })
    })
}

// ---------------------------------------------------------------------------
// asyncOpenDB and the open body
// ---------------------------------------------------------------------------

/// Marks a task-body failure whose `handling_failures` funnel has ALREADY
/// run inside the task (open_db_body's Err arm: dbState := Closed +
/// logError + on_failure). The wait site (with_open_database) propagates
/// this payload verbatim — on_failure must run ONCE per failure — and
/// runs the funnel itself for every OTHER failure (the spawn-cancelled
/// image: the warden was shut down and the body never executed; see
/// async_open_db's doc).
struct BodyFunnelled(Exc);

/// 'asyncOpenDB' (:673-759) — "Open a database asynchronously, returning
/// an 'Async' that can be waited on. This is the only way to open a
/// database since this operation has uninterruptible chunks which might
/// take a long time." / "NOTE: This should be called with 'dbState' set
/// to 'Opening' and async exceptions masked."
///
/// The task returns `Arc<OpenDB>` (not the bare `OpenDB` warden.rs's
/// sketch names): the Open commit (:750) stores an `Arc` clone in the
/// DBState, and the record cannot be in two places — the Arc is
/// types.rs's TRANSLATION NOTES 9 sharing, one level up. The wait side is
/// `catch_unwind(wait_once)` in with_open_database.
///
/// `handling_failures` (:754-759) is the row-22 funnel, reproduced at the
/// inner position the source puts it — INSIDE the acquireDB/releaseDB
/// bracket, wrapping the open body (see `open_db_body`): on ANY body
/// failure — dbState := Closed, `logError (inRepo dbRepo "couldn't open:
/// " <> show exc)`, `on_failure exc`, rethrow. The source's OUTER
/// `handling_failures $ Warden.spawnMask ...` wraps only spawnMask
/// itself ("Be paranoid about 'spawnMask' itself throwing", :689) — the
/// async body's exceptions surface at `wait opener`, NOT through the
/// outer wrapper, so on_failure runs ONCE. The port's spawn is
/// infallible (spawn_mask returns an already-cancelled task when the
/// warden is shut down — the AsyncCancelled case surfaces as a wait_once
/// failure at the waiter), so the outer wrapper has no live failure to
/// catch IN THIS FRAME: its funnel runs at the WAIT SITE instead —
/// with_open_database's failure branch executes the full Open.hs:754-759
/// sequence (dbState := Closed, logError, on_failure) for every task
/// failure NOT marked [`BodyFunnelled`], so a cancelled spawn no longer
/// strands dbState = Opening; the marker keeps on_failure at exactly one
/// run per failure.
///
/// TODO(port): the spawn closure captures `&Env`, which is `!Send + !Sync`
/// pending the rows 71/104 decision (types.rs's Env TODO; the same
/// blocker queue.rs marked at the WriteJob construction). The warden
/// thread boundary cannot be crossed until that lands. (The warden-
/// shutdown path itself is handled: the source's outer handler —
/// writeTVar dbState Closed + on_failure for a failed/AsyncCancelled
/// spawn, :754-759 — runs at with_open_database's wait site; only the
/// Send boundary remains.)
#[allow(clippy::too_many_arguments)]
pub fn async_open_db(
    env: &Env,
    storage: &crate::database::types::Storage,
    db: &Arc<DB>,
    version: DBVersion,
    mode: Mode,
    deps: Option<&thrift::Dependencies>,
    on_success: impl FnOnce() -> Result<(), Exc>,
    on_failure: impl Fn(&Exc) -> Result<(), Exc>,
) -> Task<Arc<OpenDB>> {
    // `thrift::Dependencies: Clone` — the generated record's (the
    // ASSUMED-IMPORTS pin): the task needs its own copy, since the
    // source's caller-owned value is shared by reference.
    let deps = deps.cloned();
    let db = Arc::clone(db);
    let env_ref = env;
    // `Warden.spawnMask envWarden $ \restore -> loggingAction
    // (runLogRepo "open" env dbRepo) (const mempty) $ ...` (:690-691).
    // The `\restore` is the CancelToken (warden.rs's contract note);
    // there is no mask to restore, and the body's `restore` block (:704)
    // is a plain one — no token checkpoints are added (the source's
    // restore only enables async-exception delivery; cooperative
    // cancellation has none, the B2 let-drain posture).
    spawn_mask(&env.env_warden, move |_token| {
        match log_action(env_ref, "open", &db.db_repo, || {
            open_db_body(
                env_ref,
                &db,
                storage,
                version,
                mode,
                deps.as_ref(),
                on_success,
                &on_failure,
            )
        }) {
            Ok(odb) => odb,
            // The task's throwIO image (queue.rs's pinned protocol),
            // marked BodyFunnelled: the inner handling_failures already
            // ran, so with_open_database's wait site propagates this
            // payload WITHOUT re-running the funnel.
            Err(exc) => std::panic::panic_any(Box::new(BodyFunnelled(exc))),
        }
    })
}

/// The open body: `when (not $ canOpenVersion ...) $ dbError ...` then
/// `bracket_ (acquireDB) (releaseDB) $ handling_failures $ ...` (:692-752).
/// The mode is consumed BY VALUE by `Storage.open`, so the three
/// post-open scrutinisations of `mode` (:706's Create arm, :709's
/// ReadOnly arm, :712's Create arm) are extracted here, BEFORE the move
/// (see `SetupSchema`).
#[allow(clippy::too_many_arguments)]
fn open_db_body(
    env: &Env,
    db: &Arc<DB>,
    storage: &crate::database::types::Storage,
    version: DBVersion,
    mode: Mode,
    deps: Option<&thrift::Dependencies>,
    on_success: impl FnOnce() -> Result<(), Exc>,
    on_failure: &dyn Fn(&Exc) -> Result<(), Exc>,
) -> Result<Arc<OpenDB>, Exc> {
    // `when (not $ canOpenVersion storage mode version) $ dbError dbRepo $
    // "can't open database version " ++ show (unDBVersion version)`
    // (:692-694). DBVersion is i64 (DIVERGENCES 4).
    if !can_open_version(storage, &mode, version) {
        return db_error(
            &db.db_repo,
            &format!("can't open database version {}", version),
        )
        .map_err(into_exc);
    }
    let initial = match &mode {
        Mode::Create { schema, .. } => Some(SetupSchema::of_create(schema)),
        _ => None,
    };
    let is_read_only = matches!(mode, Mode::ReadOnly);
    let is_create = initial.is_some();
    // The bracket_ acquire (:696).
    acquire_db(db);
    // The bracket_ release (:697) must run on panic too (the source's
    // bracket_ releases on all exceptions): the body is panic-safe-wrapped
    // here — a panic unwinds through catch_unwind (below), which runs the
    // release BEFORE resuming the panic for with_open_database's funnel.
    // Drop guard: the bracket_ release runs on unwind too (source
    // bracket_ semantics). release_db is idempotent-guarded by
    // db_users==0, so double-release is a no-op by construction.
    let _release_guard = ReleaseDbGuard(env, db);
    let result = (|| -> Result<Arc<OpenDB>, Exc> {
        // `handling_failures` (:754-759) — INSIDE the bracket, exactly
        // where the source puts it (the row-22 funnel; see
        // async_open_db's doc for the outer wrapper's disposition).
        match open_db_acquired(
            env,
            db,
            storage,
            mode,
            version,
            initial.as_ref(),
            is_read_only,
            is_create,
            deps,
            on_success,
        ) {
            Ok(odb) => Ok(odb),
            Err(exc) => {
                // `atomically (writeTVar dbState Closed)` (:756) — the
                // plain write, from any state. The source's write
                // REPLACES the whole OpenDB record (its odbWriting
                // included); the row-26 split keeps odb_writing beside
                // db_state, so it is cleared with it — leaving it Some
                // alongside Closed would break the DbLifecycle invariant
                // (types.rs:1187-1189: odb_writing.is_some() implies
                // db_state is Open). This is the post-commit-enqueue
                // failure path: the Open commit above set odb_writing.
                {
                    let mut life = lock(&db.db_lifecycle);
                    life.db_state = DBState::Closed;
                    life.odb_writing = None;
                }
                // `logError $ inRepo dbRepo "couldn't open: " ++ show exc`
                // (:757) — Display is the `show (exc :: SomeException)`
                // image (exception.rs:221-224).
                log_error(|| {
                    in_repo(&db.db_repo, &format!("couldn't open: {}", exc))
                });
                // `on_failure exc` (:758). The handler's own throw would
                // replace the propagated exception.
                if let Err(e2) = on_failure(&exc) {
                    return Err(e2);
                }
                // `throwIO exc` (:759).
                Err(exc)
            }
        }
    })();
    // The bracket_ release (:697) — ALWAYS, on both exits AND on unwind
    // (the ReleaseDbGuard Drop runs when this function exits by panic;
    // the explicit call is the normal-path image — release_db re-checks
    // db_users==0 so the double path is a no-op by construction).
    release_db(env, db);
    result
}

/// The bracketed body: `logInfo "opening"; bracketOnError (Storage.open
/// ...) Storage.close $ \handle -> ...` (:699-752). `mode` is moved into
/// `Storage.open`; the `initial`/`is_read_only`/`is_create` triple is
/// what remains of it (see `open_db_body`).
#[allow(clippy::too_many_arguments)]
fn open_db_acquired(
    env: &Env,
    db: &Arc<DB>,
    storage: &crate::database::types::Storage,
    mode: Mode,
    version: DBVersion,
    initial: Option<&SetupSchema>,
    is_read_only: bool,
    is_create: bool,
    deps: Option<&thrift::Dependencies>,
    on_success: impl FnOnce() -> Result<(), Exc>,
) -> Result<Arc<OpenDB>, Exc> {
    log_info(|| in_repo(&db.db_repo, "opening"));
    // `Storage.open storage dbRepo mode version` — the bracket's
    // acquire; an error here needs no close (bracketOnError runs the
    // finalizer only after a successful acquire).
    let handle = storage.open(&db.db_repo, mode, version).map_err(into_exc)?;
    // The `restore $ do ...` block (:704-749) computes every fallible
    // field while BORROWING the handle; the record construction itself
    // (:739-749) is infallible and consumes it. Splitting the two keeps
    // the bracketOnError close reachable on every failure path.
    match build_open_db(env, db, &handle, initial, is_read_only, is_create, deps, on_success) {
        Ok((odb_schema, writing, maybe_slices, ownership, acl_mode, acl_mapping, m_first_acl_id)) => {
            // :739-749 — the record, field-for-field; `odbWriting`/
            // `odbIdleSince` live in the DbLifecycle (row 26).
            let odb = Arc::new(OpenDB {
                odb_handle: handle,
                odb_schema,
                odb_base_slices: maybe_slices,
                odb_ownership: Mutex::new(ownership),
                odb_acl_mode: acl_mode,
                odb_acl_mapping: acl_mapping,
                odb_first_acl_id: m_first_acl_id,
            });
            // :750-752 — the Open commit + the descriptor re-enqueue,
            // INSIDE the bracketOnError scope (a throw from either runs
            // Storage.close, then the failure funnel).
            {
                // The TRANSACTION MAP, clause 6: the row-26 merge sets
                // state/writing/idle in ONE hold. The source's :729
                // `idle <- newTVarIO =<< getTimePoint` folds in here.
                // `writing` is CLONED into the lifecycle (the row-26
                // record split) so the just-built value survives for the
                // enqueue below: the enqueue must see the record THIS
                // open built (Open.hs:741/:765), not a re-read that a
                // concurrent Finish could flip to None mid-open.
                let mut life = lock(&db.db_lifecycle);
                life.odb_idle_since = Instant::now();
                life.odb_writing = writing.clone();
                life.db_state = DBState::Open(Arc::clone(&odb));
            }
            // The enqueue sits inside the bracketOnError callback in the
            // source (:751), so its failure runs Storage.close too — the
            // record owns the handle by now, so the close goes through
            // it; the failure then flows to handling_failures (which
            // writes Closed over the just-committed Open, exactly as the
            // source's sequence does).
            if let Err(exc) =
                enqueue_batch_descriptors_from_database(env, db, &odb, writing.as_ref())
            {
                let _ = odb.odb_handle.close();
                return Err(exc);
            }
            Ok(odb)
        }
        Err(exc) => {
            // bracketOnError's finalizer: `Storage.close` (:701).
            let _ = handle.close();
            Err(exc)
        }
    }
}

/// The `restore $ do ...` block (:704-749): storage-opened handle ->
/// schema -> writing -> slices -> ownership -> ACL meta -> on_success.
/// BORROWS the handle (the source moves the existential into the record,
/// `odbHandle = Some handle`, only at the very end — the record
/// construction is infallible, so the caller consumes the handle after
/// this returns Ok and closes it on Err).
#[allow(clippy::too_many_arguments)]
fn build_open_db(
    env: &Env,
    db: &Arc<DB>,
    handle: &SomeDatabaseOps,
    initial: Option<&SetupSchema>,
    is_read_only: bool,
    is_create: bool,
    deps: Option<&thrift::Dependencies>,
    on_success: impl FnOnce() -> Result<(), Exc>,
) -> Result<
    (
        DbSchema,
        Option<Arc<Writing>>,
        Vec<Option<Slice>>,
        Option<Arc<Ownership>>,
        AclMode,
        BTreeMap<String, UsetId>,
        Option<UsetId>,
    ),
    Exc,
> {
    log_info(|| in_repo(&db.db_repo, "opened"));
    // `dbSchema <- setupSchema env dbRepo handle mode` (:706).
    let odb_schema = setup_schema(env, &db.db_repo, handle, initial, is_read_only)?;
    log_info(|| {
        in_repo(
            &db.db_repo,
            &format!("schema has {} predicates", schema_size(&odb_schema)),
        )
    });
    // `writing <- case mode of ReadOnly -> return Nothing; _ -> Just
    // <$> setupWriting env handle` (:709-711).
    let writing = if is_read_only {
        None
    } else {
        Some(Arc::new(setup_writing(env, handle)?))
    };
    // `maybeSlices <- case mode of Create{} -> ...; _ -> ...`
    // (:712-728).
    let odb_base_slices: Vec<Option<Slice>> = if is_create {
        // `let units = case deps of Just (Dependencies_pruned
        // Pruned{..}) -> pruned_units; _ -> []` (:714-717).
        let units: Vec<Vec<u8>> = match deps {
            Some(thrift::Dependencies::Pruned(pruned)) => pruned.units.clone(),
            _ => Vec::new(),
        };
        // `storeUnits handle units` (:718).
        store_units(handle, &units).map_err(into_exc)?;
        // `slices <- baseSlices env deps (Just units)` (:719).
        let slices = base_slices(env, deps, Some(&units))?;
        // `storeSlices handle (catMaybes slices)` (:720).
        //
        // TODO(port): `Slice` must derive Clone in glean-core (row 57's
        // Slice — its ForeignPtr is Arc-backed and Clone, ffi.rs:442-455,
        // so a one-line derive is sound and matches the source's
        // shareable ForeignPtr newtype, Ownership.hsc:127). Until then
        // this file does not typecheck. The four catMaybes images that
        // need it: this storeSlices site, with_open_db_lookup's
        // ownership_slices, db_slices' bases, and dep_slices'
        // optimisation return.
        let flat: Vec<Slice> = slices
            .iter()
            .filter_map(|s| s.as_ref().map(|s| s.clone()))
            .collect();
        store_slices(handle, &flat).map_err(into_exc)?;
        slices
    } else {
        // `m <- retrieveSlices dbRepo handle` (:723).
        match retrieve_slices(&db.db_repo, handle).map_err(into_exc)? {
            None => {
                // `stored_units <- retrieveUnits dbRepo handle;
                // baseSlices env deps stored_units` (:725-727).
                let stored_units = retrieve_units(&db.db_repo, handle).map_err(into_exc)?;
                base_slices(env, deps, stored_units.as_deref())?
            }
            Some(slices) => slices.into_iter().map(Some).collect(),
        }
    };
    // `ownership <- newTVarIO =<< Storage.getOwnership handle` (:730).
    let odb_ownership = handle.get_ownership().map_err(into_exc)?.map(Arc::new);
    // ":732 — Get database properties to check ACL mode."
    let meta = env.env_catalog.read_meta(&db.db_repo).map_err(into_exc)?;
    let odb_acl_mode = get_acl_mode(&meta.meta_properties);
    // ":734-736 — Log ACL metadata on database open."
    log_info(|| {
        in_repo(
            &db.db_repo,
            &format!(
                "[Open.hs] Database opening, ACL mode: {}",
                show_acl_mode(odb_acl_mode)
            ),
        )
    });
    // `(mFirstACLID, aclMapping) <- loadAclMetadata handle` (:737).
    let (odb_first_acl_id, odb_acl_mapping) = load_acl_metadata(handle)?;
    // `on_success` (:738) — "Action to run on success before setting
    // the db status to 'Open'. If this fails, we will call the
    // failure action." (The source doc, kept.)
    on_success()?;
    Ok((
        odb_schema,
        writing,
        odb_base_slices,
        odb_ownership,
        odb_acl_mode,
        odb_acl_mapping,
        odb_first_acl_id,
    ))
}

/// 'enqueueBatchDescriptorsFromDatabase' (:763-771) — "Re-enqueue all
/// unprocessed batch descriptors which were enqueued before but never
/// materialised and written to the DB". Private in the source. The source
/// pattern-matches `odbWriting` ON THE RECORD BUILT at :739-749 (its
/// `OpenDB{..}` argument IS that record), so the port takes the
/// just-built `writing` value instead of re-reading the lifecycle (row
/// 26): a re-read opens a Finish-race window — Finish.rs's
/// `odb_writing = None` (Finish.hs:83) landing between the Open commit
/// (:750-751) and this read would silently skip the re-enqueue the
/// source performs. Hands unit 105's funnel the same arguments
/// (:769-770), `True` for the async flag.
fn enqueue_batch_descriptors_from_database(
    env: &Env,
    db: &Arc<DB>,
    odb: &OpenDB,
    writing: Option<&Arc<Writing>>,
) -> Result<(), Exc> {
    match writing {
        Some(w) => {
            let descriptors = odb
                .odb_handle
                .get_unprocessed_batch_descriptors()
                .map_err(into_exc)?;
            for descriptor in &descriptors {
                enqueue_batch_descriptor_with_result_handling(
                    env,
                    &db.db_repo,
                    &w.wr_queue,
                    &odb.odb_schema,
                    &odb.odb_handle,
                    descriptor,
                    true,
                )
                .map_err(into_exc)?;
            }
            Ok(())
        }
        None => Ok(()),
    }
}

/// 'getDbSchemaVersion' (:775-784) — "Fetch the glean.schema_id property
/// of a DB, if it has one. This property is used to resolve queries."
pub fn get_db_schema_version(
    env: &Env,
    repo: &thrift::Repo,
) -> Result<Option<thrift::SchemaId>, OpenError> {
    let meta = env.env_catalog.read_meta(repo)?;
    Ok(meta.meta_properties.get("glean.schema_id").cloned())
}

/// 'updateLookupCacheStats' (:501-504) — read the cache's counters
/// (resetting them) and bump the write Stats. The source's `=<<` makes
/// the read happen first.
pub fn update_lookup_cache_stats(env: &Env) {
    let stats = read_stats_and_reset_counters(&env.env_lookup_cache_stats);
    bump(&env.env_stats, &LOOKUP_CACHE_STATS, &stats);
}

// ---------------------------------------------------------------------------
// baseSlices / depSlices
// ---------------------------------------------------------------------------

/// `type Unit = ByteString` (:786-787) — "why is pruned_units a
/// list<binary> instead of list<UnitName>?" (the source's own question,
/// kept). `Vec<u8>` per the RULE §1 strings row.
type Unit = Vec<u8>;

/// 'baseSlices' (:803-848) — "Create the slices for a DB stack." (The
/// source's long design comment :789-802 is the contract; its set
/// algebra lives at `dep_slices`.)
fn base_slices(
    env: &Env,
    deps: Option<&thrift::Dependencies>,
    stored_units: Option<&[Unit]>,
) -> Result<Vec<Option<Slice>>, Exc> {
    match deps {
        None => Ok(Vec::new()),
        Some(thrift::Dependencies::Stacked(stacked)) => db_slices(
            env,
            &thrift::Repo {
                name: stacked.name.clone(),
                hash: stacked.hash.clone(),
            },
            BTreeSet::new(),
            true,
        ),
        Some(thrift::Dependencies::Pruned(pruned)) => {
            // `let real_units = fromMaybe pruned_units stored_units`
            // (:813).
            let real_units: BTreeSet<Unit> = match stored_units {
                Some(units) => units.iter().cloned().collect(),
                None => pruned.units.iter().cloned().collect(),
            };
            db_slices(env, &pruned.base, real_units, pruned.exclude)
        }
    }
}

/// `dbSlices` (:824-848) — one level of the stack: open the DB, recurse
/// into ITS base (`rest`), then this level's slice on top (`slice :
/// rest`).
fn db_slices(
    env: &Env,
    repo: &thrift::Repo,
    units: BTreeSet<Unit>,
    exclude: bool,
) -> Result<Vec<Option<Slice>>, Exc> {
    with_open_database(env, repo, |_db, odb| {
        // `baseDeps <- atomically $ metaDependencies <$> Catalog.readMeta
        // (envCatalog env) repo` (:826-827).
        let base_deps = env
            .env_catalog
            .read_meta(repo)
            .map_err(into_exc)?
            .meta_dependencies;
        // `baseUnits <- retrieveUnits repo odbHandle` (:828).
        let base_units = retrieve_units(repo, &odb.odb_handle).map_err(other)?;
        let rest = dep_slices(
            env,
            base_deps.as_ref(),
            base_units.as_deref(),
            &units,
            exclude,
            &odb.odb_base_slices,
        )
        .map_err(other)?;
        // `slice <- if exclude && Set.null units then return Nothing else
        // ...` (:830-847).
        let slice = if exclude && units.is_empty() {
            None
        } else {
            let n = units.len();
            // The `let !n = Set.size units` force is the source's own
            // strictness annotation — the port's `n` is already a value.
            vlog(1, || {
                let listed = show_unit_list(&units);
                format!(
                    "computing slice for {} with {} {}{}",
                    show_repo(repo),
                    n,
                    if exclude { "excluded: " } else { "included: " },
                    listed
                )
            });
            // `unitIds <- fmap catMaybes $ forM (Set.toList units) ...`
            // (:838-842) — BTreeSet iterates ascending, the Set.toList
            // order at the element Ord's mercy (Data.Set is ordered by
            // the element Ord; Vec<u8>'s Ord matches).
            let mut unit_ids: Vec<UnitId> = Vec::with_capacity(units.len());
            for name in &units {
                let id = odb.odb_handle.get_unit_id(name).map_err(other)?;
                vlog(2, || format!("unit: {} = {:?}", show_unit(name), id));
                if let Some(id) = id {
                    unit_ids.push(id);
                }
            }
            // `maybeOwnership <- readTVarIO odbOwnership` (:843) — the
            // ownership Mutex alone (row 26's clause).
            // INVENTORY-BUG-CANDIDATE row 26 (partial): ownership snapshot
            // taken under its own Mutex, not nested under DB — matches the
            // source's out-of-transaction readTVarIO (Open.hs:324/:843);
            // rationale at the header's row-26 clause (the
            // Ownership.hs:46/UserQuery.hs:813 bullet).
            let maybe_ownership = lock(&odb.odb_ownership).clone();
            // `r <- forM maybeOwnership $ \ownership -> Ownership.slice
            // ownership (catMaybes rest) unitIds exclude` (:844-845).
            let r = match &maybe_ownership {
                Some(ownership) => {
                    let bases: Vec<Slice> = rest
                        .iter()
                        .filter_map(|s| s.as_ref().cloned())
                        .collect();
                    Some(slice(ownership, &bases, &unit_ids, exclude).map_err(other)?)
                }
                None => None,
            };
            // `logInfo $ "completed " <> show n <> " slice for " <>
            // showRepo repo` (:846).
            log_info(|| format!("completed {} slice for {}", n, show_repo(repo)));
            r
        };
        // `return (slice : rest)` (:848).
        let mut out = Vec::with_capacity(rest.len() + 1);
        out.push(slice);
        out.extend(rest);
        Ok(out)
    })
    .map_err(into_exc)
}

/// `show :: [ByteString] -> String`'s log-text image (DIVERGENCES 3):
/// Haskell renders a ByteString list as `["abc","def"]`; the port uses
/// the byte-slice Debug per element. Log-only.
fn show_unit_list(units: &BTreeSet<Unit>) -> String {
    let items: Vec<String> = units.iter().map(|u| format!("{:?}", u)).collect();
    format!("[{}]", items.join(","))
}

/// `show :: ByteString -> String`'s image for the single-unit vlog line.
fn show_unit(unit: &[u8]) -> String {
    String::from_utf8_lossy(unit).into_owned()
}

/// `depSlices` (:851-884) — "Create the slices for a stack of
/// dependencies." The four set-algebra arms of :876-884 are the source's
/// own exhaustively-commented table (`exclude`/`pruned_exclude`
/// combinations); the boolean conditions below mirror them arm for arm.
#[allow(clippy::too_many_arguments)]
fn dep_slices(
    env: &Env,
    dep: Option<&thrift::Dependencies>,
    dep_units: Option<&[Unit]>,
    units: &BTreeSet<Unit>,
    exclude: bool,
    slices: &[Option<Slice>],
) -> Result<Vec<Option<Slice>>, Exc> {
    match dep {
        None => Ok(Vec::new()),
        Some(thrift::Dependencies::Stacked(stacked)) => db_slices(
            env,
            &thrift::Repo {
                name: stacked.name.clone(),
                hash: stacked.hash.clone(),
            },
            units.clone(),
            exclude,
        ),
        Some(thrift::Dependencies::Pruned(pruned)) => {
            // `depUnits = Set.fromList $ fromMaybe pruned_units dep_units`
            // (:874-875) — "prefer the units stored in the DB" (the
            // source comment, kept).
            let dep_units_set: BTreeSet<Unit> = match dep_units {
                Some(units) => units.iter().cloned().collect(),
                None => pruned.units.iter().cloned().collect(),
            };
            // The optimisation guard (:866-870): "check if the slices
            // for the stack will be the same as the slices we already
            // have for this repo. TODO: could handle more of the
            // exclude/pruned_exclude combos here" — the source's own
            // TODO, kept. `return slices` shares the existing Slice
            // values (ForeignPtr-backed, shareable in Haskell); the
            // port's Clone need is the TODO at build_open_db's
            // storeSlices site.
            if exclude && pruned.exclude && units.is_subset(&dep_units_set) {
                Ok(slices.to_vec())
            } else {
                // `newUnits` (:876-884), arm for arm:
                //   exclude && pruned_exclude     -> units | depUnits
                //   not exclude && pruned_exclude -> units \ depUnits
                //   exclude && not pruned_exclude -> depUnits \ units
                //   otherwise                     -> units ^ depUnits
                let new_units: BTreeSet<Unit> = if exclude && pruned.exclude {
                    units.union(&dep_units_set).cloned().collect()
                } else if !exclude && pruned.exclude {
                    units.difference(&dep_units_set).cloned().collect()
                } else if exclude && !pruned.exclude {
                    dep_units_set.difference(units).cloned().collect()
                } else {
                    units.intersection(&dep_units_set).cloned().collect()
                };
                db_slices(env, &pruned.base, new_units, exclude && pruned.exclude)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// log_action — the Util.Logger.loggingAction image
// ---------------------------------------------------------------------------

/// `loggingAction (runLogRepo cmd env repo) (const mempty) action`
/// (Util.Logger.hs:29-51 + Logger.hs:45-48), the two call shapes this
/// file has ("open" at :691, "update-schema" at :622-623).
///
/// Source protocol: `timeIt $ try $ restore io` — time the action; on
/// success emit `timeLog time <> allocLog alloc <> successLog <> res x`
/// (with `res = const mempty` here); on failure emit `timeLog time <>
/// allocLog alloc <> failureLog ex` and RETHROW. The emit is
/// `swallow`-wrapped (log failures are dropped); the port's
/// `run_log(&self, ..) -> ()` is already infallible. Panics are NOT
/// caught here (the source's `try` catches exceptions, not async
/// delivery) — the task frame's catch_unwind is the with_open_database
/// wait site.
///
/// The ActionLog GleanServerLog instance (Logger.hs:28-34) maps:
/// timeLog -> SetTimeElapsed, allocLog -> SetAllocatedBytes .
/// fromIntegral, successLog -> SetSuccess True, failureLog ->
/// SetSuccess False <> SetError (show ex). runLogRepo appends
/// SetRepoName + SetRepoHash; runLogCmd appends SetMethod cmd +
/// SetWeight weight and calls `runLog logger`.
///
/// TODO(port): two fidelity gaps pending row 106 (Logger.hs /
/// runLogCmd-runLogRepo, UNSHIPPED — logger.rs is still scaffolding) and
/// the Util kit: (a) `whenAllowed (envLoggerRateLimit env) cmd` — the
/// rate limiter is types.rs's RateLimiterMap stub with no
/// `whenAllowed`; the log currently goes out UNRATE-LIMITED with no
/// SetWeight field; (b) `timeIt`'s allocated-bytes measurement has no
/// Rust analogue, so SetAllocatedBytes carries 0.
fn log_action<T>(
    env: &Env,
    cmd: &str,
    repo: &thrift::Repo,
    action: impl FnOnce() -> Result<T, Exc>,
) -> Result<T, Exc> {
    let start = Instant::now();
    let result = action();
    let elapsed = start.elapsed().as_secs_f64();
    let alloc: i64 = 0; // see the TODO above.
    let emit = |success: bool, err: Option<&Exc>| {
        // The mconcat chain, in the source's append order: timeLog <>
        // allocLog <> successLog <> [SetError] <> SetRepoName <>
        // SetRepoHash <> SetMethod <> [SetWeight].
        let mut l = GleanServerLog::GleanServerLogAnd(
            Box::new(GleanServerLog::GleanServerLogAnd(
                Box::new(GleanServerLog::SetTimeElapsed(elapsed)),
                Box::new(GleanServerLog::SetAllocatedBytes(alloc)),
            )),
            Box::new(GleanServerLog::SetSuccess(success)),
        );
        if let Some(err) = err {
            l = GleanServerLog::GleanServerLogAnd(
                Box::new(l),
                Box::new(GleanServerLog::SetError(format!("{}", err))),
            );
        }
        // `res x = const mempty` appends nothing.
        l = GleanServerLog::GleanServerLogAnd(
            Box::new(l),
            Box::new(GleanServerLog::SetRepoName(repo.name.clone())),
        );
        l = GleanServerLog::GleanServerLogAnd(
            Box::new(l),
            Box::new(GleanServerLog::SetRepoHash(repo.hash.clone())),
        );
        l = GleanServerLog::GleanServerLogAnd(
            Box::new(l),
            Box::new(GleanServerLog::SetMethod(cmd.to_string())),
        );
        // SetWeight: see the TODO above.
        env.env_server_logger.run_log(l);
    };
    match result {
        Ok(v) => {
            emit(true, None);
            Ok(v)
        }
        Err(e) => {
            emit(false, Some(&e));
            Err(e)
        }
    }
}

// PORT STATUS: confidence=medium todos=6
