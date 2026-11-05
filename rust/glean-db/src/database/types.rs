// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// Ported from: glean/db/Glean/Database/Types.hs (335 lines).
// Manifest unit: rust/glean-db/src/database/types.rs
// (migration/manifest.tsv line 104, crate glean-db). RULE §4: one source
// file -> one target file; the manifest path is authoritative.
//
// WHAT THIS FILE IS: the database layer's shared vocabulary and its HUB.
// `Writing`/`OpenDB`/`DBState`/`DB` are the open-database state machine,
// `Write`/`WriteContent`/`WriteJob`/`WriteQueue`/`WriteQueues` are the
// write-pipeline records, `Derivation` is the in-flight derived-predicate
// record, `Env` is the process-wide environment (42 source fields), and
// `withStorageFor`/`withDefaultStorage` are the two rank-2 storage
// dispatchers. Dozens of later units (Open/Create/Close/Finish/Delete/
// Backup/Writes/Batch/Queue/Derive/CompletePredicates/Server/...) name
// these types, so the SIBLING-UNIT CONTRACTS block below is the contract
// they must code against.
//
// ---------------------------------------------------------------------------
// INVENTORY (RULE §5; migration/inventory*.tsv — every row grep-verified)
// ---------------------------------------------------------------------------
//
// Governing rows and their disposition:
//
//   * inventory.tsv:32 — `Database/Types.hs / Env / ~25 TVar/MVar/IORef
//     fields / STM_GROUP`: "single struct with Mutex/RwLock per coherence
//     group; TVar tuples that change together -> one Mutex<SubState>;
//     catalog lock ordering documented in module", evidence
//     `Types.hs:244-303 Env fields; :78-96 Writing{wrNextId IORef, TVar
//     (Maybe (Fid,FactSet))}; :186-196 WriteQueue TQueue+3 TVars`.
//     FOLLOWED: every TVar field is `Mutex<T>` (and `Arc<T>` where the
//     value escapes the lock scope, TRANSLATION NOTES 9); the two
//     cross-field coherence groups are `Writing.wr_state` (row 34) and
//     `WriteQueue.write_queue_state` (rows 35/36); lock ordering is
//     recorded under SIBLING-UNIT CONTRACTS.
//   * inventory.tsv:26 — `Database/Open.hs / DB lifecycle state machine /
//     ... / STM_GROUP`: "struct Db { Mutex<DbLifecycle { state: DBState,
//     users: i64, idle_since: TimePoint, writing: Option<Writing> }> } ...
//     dbUsers decrement + idle eviction (Close.hs:44-49) reads 4 TVars incl.
//     writeQueueSize/wrCommit -> take DB lock and Writing lock (DB ->
//     WRITING order)". FOLLOWED: `DB { db_repo, db_lifecycle:
//     Mutex<DbLifecycle> }`, `DbLifecycle { db_state, db_users,
//     odb_idle_since, odb_writing }`. The source's `odbIdleSince` (:110)
//     and `odbWriting` (:104) therefore live in `DbLifecycle`, not
//     `OpenDB`; the source record split is stated in TRANSLATION NOTES 2.
//   * inventory.tsv:33 — `Database/Types.hs / JanitorException /
//     JanitorFailed variants / EXC_TAXONOMY`: "enum JanitorError
//     { Run(JanitorException), Timeout, Stuck, Disabled }; enum
//     JanitorException { Other(Box<Error>), FetchBackups(Box<Error>) };
//     FetchBackups struct carries Option<last_good> + error".
//     PARTIALLY FOLLOWED / FLAGGED: the row is `JanitorRunResult` minus
//     `JanitorRunSuccess` (:229), which it OMITS; its `JanitorError` /
//     `JanitorFailed` names do not occur in the source at all (`grep -rn
//     "JanitorFailed\|JanitorError" glean/` returns nothing). The
//     `JanitorException` half is followed in shape, but the row's names
//     `Other`/`FetchBackups` are rendered with the source's names
//     `OtherJanitorException`/`JanitorFetchBackupsFailure`. The source is
//     authoritative: all five `JanitorRunResult` constructors are
//     translated, and Janitor.hs:373-384's `FetchBackups` is a different
//     type. See the row-33 flag at the type below.
//   * inventory.tsv:34 — `Database/Write/Batch.hs / Writing reserve/commit
//     / ...`: "struct Writing { Mutex<WriteState { next_id: Fid, pending:
//     Option<(Fid, FactSet)> }> } — merge wrNextId + wrCommit into ONE lock
//     ... LOCK ORDER: DB lifecycle lock -> Writing lock". FOLLOWED:
//     `Writing.wr_state: Mutex<WriteState>` with `wr_next_id`/`wr_commit`
//     inside (source names preserved as the fields of `WriteState`).
//   * inventory.tsv:35 — `Database/Write/Queue.hs / WriteQueue / STM queue
//     + batch coalescing`: "mpsc channel + Mutex<BatchState>; commit
//     boundary preserves wrNextId monotonicity". DIVERGED / FLAGGED: the
//     row's "mpsc channel + Mutex<BatchState>" sketch is not the port. The
//     port uses RULEBOOK §2's STM row (one `Mutex<State>` per
//     multi-variable transaction): `Mutex<WriteQueueState>` holds the
//     `VecDeque<WriteJob>` AND the four counters, with a Condvar beside
//     it. Commit-boundary `wrNextId` monotonicity is row 34's
//     `WriteState`, untouched. See the row-35 flag at `WriteQueueState`.
//   * inventory.tsv:36 — `Database/Writes.hs / writerThread + writeQueue
//     counters / ...`: "Counter updates 128-132 (4 TVars in sequence) ->
//     fold into the WriteQueue's single Mutex<QueueStats>; ... update stats
//     BEFORE signaling the mpsc oneshot". FOLLOWED: the four counters live
//     in `WriteQueueState` (`active`/`count`/`size`/`latency`, with the
//     source names preserved in their field docs) under the SAME lock as
//     the queue itself (rows 35/36 merge); the `try_put(write_done)`
//     ordering note is restated in the contracts.
//   * inventory.tsv:23 — `Database/Env.hs / spawnThreads +
//     envSchemaUpdateSignal / ...`: "envSchemaUpdateSignal TMVar ()
//     one-shot latch -> Condvar+AtomicBool or mpsc (tryPutTMVar =
//     non-blocking set)". FOLLOWED: `env_schema_update_signal: TMVar<()>`
//     (the RULE §2 TMVar image, `Mutex<Option<()>>` + `Condvar`).
//   * inventory.tsv:40 — `Database/Types.hs:210 derivationError /
//     EXC_TAXONOMY`: "the (duration, error) pair must survive for stats
//     reporting". FOLLOWED:
//     `derivation_error: Option<(Instant, Box<dyn Error + Send + Sync>)>`.
//   * inventory.tsv:41 — `Database/Query/Derive.hs / envDerivations +
//     envCompleting / STM_GROUP`: registries of in-flight `Async ()`
//     keyed by `(Repo, PredicateId)` / `Repo`. FOLLOWED:
//     `Mutex<HashMap<..., Task<()>>>`; the waiter/oneshot protocol is the
//     Derive unit's (row 41's own target).
//   * inventory.tsv:95 — `server/Glean/Server.hs / portVar +
//     waitForAlive/monitorJanitor`: "wait until (t,result) is Some AND t !=
//     last_seen". FOLLOWED: `env_database_janitor` is a
//     `Mutex<Option<(SystemTime, Arc<JanitorRunResult>)>>` (the Arc lets
//     the result outlive the guard, Server.hs:136-141/:158-166) and gains
//     the Rust-only companion `env_database_janitor_changed: Condvar` (the
//     STM-retry image; the server unit cannot add it because it does not
//     own `Env`).
//   * inventory.tsv:114 — `util/Glean/Util/Observed.hs / Observed`: `get`
//     returns a clone of A. FOLLOWED: `Observed<ServerConfig>` and
//     `Observed<Arc<SchemaIndex>>` (the Arc is forced by SchemaIndex's
//     missing Clone; see DIVERGENCES 1).
//   * inventory.tsv:117 — `util/Glean/Util/Warden.hs / Warden`: the
//     `Async ()` image is unit 103's `Task<()>`; `Warden` is the shipped
//     `glean_util::util::warden::Warden`. FOLLOWED.
//   * inventory.tsv:124 — `Database/{Close,Finish,Create}.hs`: "extend the
//     Open.hs lifecycle row's Mutex groups; Close.hs:61/75,123
//     (state->Closed + opens snapshot), Finish.hs:40/99/115, Create.hs:
//     132/155/181 markBroken; single DB-state Mutex per db handle, catalog
//     swaps stay in Catalog row". FOLLOWED: this file adds no state group
//     beyond rows 26/32/34/36.
//   * inventory.tsv:125 — the DB admin set (`Backup`, `Backup/Mock`,
//     `Delete`, `List`, `Restore`, `CompletePredicates`, `Config`,
//     `Storage`): covered by the Catalog row's lock model; no new group
//     here either.
//   * inventory.tsv:136 — `util/Glean/Util/Some.hs / Some c / MECHANICAL`:
//     "closed enum at each use site; Box<dyn Trait> only for truly-open
//     sites — CARVE-OUT ... logger sinks". FOLLOWED:
//     `SomeDatabaseOps`/`Storage` are closed enums; the two logger fields
//     are `Box<dyn ... + Send + Sync>`.
//   * inventory.tsv:137 — `Database/Types.hs / odbHandle :: Some
//     DatabaseOps / MECHANICAL`: "enum Storage {Rocks, Lmdb, Memory}
//     (closed — RULEBOOK §2 GADT row); match dispatch, no dyn". FOLLOWED
//     WITH A NAMING FLAG: the open-DB enum is `SomeDatabaseOps` (the
//     `Storage` name is already committed by the shipped config.rs to the
//     BACKENDS enum, a different existential — see the row-137 flag at the
//     enum below.
//   * inventory.sites.tsv:325 — `Database/Types.hs:267 STM transaction
//     group / envShuttingDown :: TVar Bool`; :365 — `Server/Sharding.hs:237
//     writeTVar (envShuttingDown env) True`. FOLLOWED:
//     `env_shutting_down: Mutex<bool>` (the row names no atomic).
//
// No row is silently diverged from: the three flagged rows (33, 35, 137)
// are marked at their sites (RULE §5/A9). The remaining differences are
// naming-only images, recorded here so they are documented rather than
// silent: row 26 `state/users/idle_since/writing` ->
// `db_state/db_users/odb_idle_since/odb_writing`; row 34
// `next_id/pending` -> `wr_next_id/wr_commit`; row 23
// `Condvar+AtomicBool or mpsc` -> `TMVar<()>`; row 41 `Arc<Job>` ->
// `Task<()>`.
//
// ---------------------------------------------------------------------------
// DEPENDENCY TRUTH (source import line -> shipped Rust path)
// ---------------------------------------------------------------------------
//
// The source's imports (Types.hs:23-75) and the shipped owners:
//
//   :23 Control.DeepSeq (NFData)      -> no analogue; documented non-goal
//                                        (TRANSLATION NOTES 3)
//   :24 Control.Concurrent.Async      -> glean_util::util::warden::Task
//                                        (Async (); unit 103)
//   :25 Control.Concurrent.MVar       -> the local `MVar<T>` image below
//                                        (RULE §2 MVar row; no shipped
//                                        generic slot exists)
//   :26 Control.Exception             -> Box<dyn Error + Send + Sync>
//                                        (database/exception.rs's image)
//   :27 Control.Trace (Tracer)        -> crate::database::config::Tracer
//                                        (config.rs:411's sketch; no
//                                        manifest unit for Control.Trace)
//   :28 Data.ByteString               -> Vec<u8> (RULE §1 strings row)
//   :29-30 Data.HashMap.Strict        -> std::collections::HashMap
//   :31 Data.HashSet                  -> std::collections::HashSet
//   :32 Data.IORef                    -> Mutex<T> (merged per row 34)
//   :33 Data.Map                      -> BTreeMap (standing facts)
//   :34 Data.Maybe                    -> Option
//   :35-36 Data.Text                  -> String
//   :37 Data.Typeable                 -> automatic (no Rust spelling)
//   :38 Data.Time (UTCTime)           -> std::time::SystemTime
//                                        (RULE §1 B3 time row)
//   :39 System.Clock (TimeSpec)       -> std::time::Duration
//   :41 Data.RateLimiterMap           -> local opaque sketch
//                                        (ASSUMED-IMPORT; no manifest unit)
//   :42 Util.EventBase                -> glean_thrift::EventBaseDataplane
//                                        (thrift_service.rs:325-380)
//   :43 Util.STM                      -> std::sync Mutex/Condvar images
//   :45 Glean.Angle.Types            -> glean_angle::angle::types::
//                                        PredicateId (Angle/Types.hs:566).
//                                        NOT `ACLMode`: that type is
//                                        Meta.hs:159, imported at :51.
//   :46 Backup.Backend                -> crate::database::backup::backend::
//                                        Backends (row 56)
//   :47 BatchLocation                 -> crate::database::batch_location::
//                                        SomeParser (row 62)
//   :48 Catalog                       -> crate::database::catalog::Catalog
//                                        (row 54)
//   :49 Config                        -> crate::database::config::{
//                                        DebugFlags, SchemaIndex,
//                                        StorageName, Tracer, rocksdb_name}
//                                        (row 85)
//   :50 Exception                     -> crate::database::exception::
//                                        {db_error, DbError} (row 53)
//   :51 Meta                          -> crate::database::meta::{AclMode,
//                                        Meta} (row 48; `Meta` is a
//                                        re-export of crate::internal::
//                                        types, `AclMode` is Meta.hs:159)
//   :52 Schema.Types                  -> crate::database::schema::types::
//                                        {DbSchema, DbSchemaCache} (row 89)
//   :53 Storage                       -> crate::database::storage::
//                                        {DatabaseOps, Mode, RestoreSource,
//                                        Storage, WriteLock, DBVersion,
//                                        AxiomOwnership} (row 58)
//   :54 Trace                         -> crate::database::trace::GleanTrace
//                                        (row 73)
//   :55 Internal.Types (StorageName)  -> crate::database::config::
//                                        StorageName (re-export, row 85)
//   :56 Logger.Server                 -> crate::logger::server::
//                                        GleanServerLogger (row 75)
//   :57 Logger.Database               -> crate::logger::database::
//                                        GleanDatabaseLogger (row 74)
//   :58 FactSet                       -> glean_core::rts::foreign::fact_set::
//                                        FactSet
//   :59-60 LookupCache                -> glean_core::rts::foreign::
//                                        lookup_cache::{LookupCache, Stats}
//   :61-62 Ownership                  -> glean_core::rts::foreign::
//                                        ownership::{Ownership, Slice,
//                                        DefineOwnership, UsetId}
//   :63 Subst                         -> glean_core::rts::foreign::subst::
//                                        Subst
//   :64 RTS.Types (Fid)               -> glean_core::rts::types::Fid
//   :65 ServerConfig.Types            -> glean_thrift::Config (config.rs:
//                                        341's alias)
//   :66 Thrift                        -> glean_thrift (ASSUMED-IMPORT)
//   :67 Metric (Point)                -> glean_util::util::metric::Point
//   :68 Util.Mutex                    -> dissolves to std::sync::Mutex
//                                        (unit 102 ships zero items)
//   :69 Observed                      -> glean_util::util::observed::
//                                        Observed
//   :70 ShardManager                  -> glean_util::util::shard_manager::
//                                        SomeShardManager
//   :71 Some                          -> per-use-site closed enums
//                                        (unit 38's some.rs)
//   :72 Util.Time (TimePoint)         -> std::time::Instant (monotonic;
//                                        external package, no manifest
//                                        unit)
//   :73 Util.Trace (Listener)         -> glean_util::util::trace::Listener
//   :74 Warden                        -> glean_util::util::warden::{
//                                        Warden, Task}
//   :75 Write.Stats                   -> glean_core::write::stats::Stats
//
// ---------------------------------------------------------------------------
// ASSUMED-IMPORTS (RULE §4 B4)
// ---------------------------------------------------------------------------
//
//   * `glean_thrift` — generated from glean/if/glean.thrift and
//     glean/config/server/server_config.thrift by build.rs; never
//     hand-translated. Shapes used here, pinned by shipped precedents:
//       - `Repo { name: String, hash: String }` (database/repo.rs:52);
//       - `SchemaId = String` (config.rs's `SchemaId` import, :341);
//       - `Exception { message: String }` (batch_location.rs:60-66);
//       - `Batch`, `Handle`, `UserQueryStats`, `BatchDescriptor`,
//         `PredicateStats`, `Config` (the IDL structs; `Config` is
//         aliased `ServerConfig` here exactly as config.rs:341 does).
//     `glean_thrift::Config`'s generated field is
//     `config_db_create_storage` — the same name as the source's
//     (server_config.thrift:350); config.rs:1089's
//     `config_schema_location` shows the generated name keeps the
//     `config_` prefix.
//   * `glean_thrift::EventBaseDataplane` — `Util.EventBase` has no manifest
//     unit; the kit contract (struct + `init_executor`/`destroy_executor`)
//     is pinned at util/thrift_service.rs:325-380 (EventBase.hs:37/:57-60/
//     :62-66/:75-79). Store-ownership is the kit's Step-4 decision; this
//     file only stores the handle by value, as the source does. Its
//     `Send`-ness is UNKNOWN at this unit (thrift_service.rs:356-358 is the
//     contract, and it says nothing about Send), so it is a SUSPECTED
//     cause of `Env`'s `!Send` — see the TODO at `Env`.
//   * `RateLimiterMap<K>` — `Data.RateLimiterMap` is an external hsthrift
//     package with no manifest unit (pinned source:
//     ~/glean-build/hsthrift/common/util/Data/RateLimiterMap.hs:37-42:
//     `data RateLimiterMap k = RateLimiterMap { rlmRef :: IORef (HashMap k
//     (IORef RateLimitWithCount)), rlmConfig :: IORef RateLimiterConfig,
//     rlmKeyPreprocessor :: k -> IO k }`). The local opaque sketch below
//     fixes the field type only; the functional port (and
//     `newRateLimiterMap`/`whenAllowed`) is owed by the first consumer,
//     logger.rs (row 106), or the Step-4 kit. This is the Tracer precedent
//     (config.rs:395-430): an external type with no manifest unit gets a
//     local sketch + a port todo, not an invented module.
//   * `Tracer<A>` — imported from config.rs:411 (the only sketch), not
//     re-declared here. Control.Trace has no manifest unit.
//
// ---------------------------------------------------------------------------
// TYPE-MAPPING TRUTH (the HUB field contract)
// ---------------------------------------------------------------------------
//
// Writing (Types.hs:78-96):
//   wrLock      :: Mutex (WriteLock ())   -> Mutex<WriteLock<()>>
//   wrNextId    :: IORef Fid              -> WriteState.wr_next_id: Fid
//   wrLookupCache :: LookupCache          -> LookupCache
//   wrLookupCacheAnchorName :: TVar (Maybe Text)
//                                         -> Mutex<Option<String>>
//   wrQueue     :: WriteQueue             -> Arc<WriteQueue> (the same
//                                            handle is in WriteQueues;
//                                            Queue.hs:143-150)
//   wrCommit    :: TVar (Maybe (Fid, FactSet))
//                                         -> WriteState.wr_commit:
//                                            Option<(Fid, FactSet)>
//   (wrNextId + wrCommit merged into `wr_state: Mutex<WriteState>`, row 34)
//
// OpenDB (Types.hs:99-129):
//   odbHandle   :: Some DatabaseOps       -> SomeDatabaseOps
//   odbWriting  :: Maybe Writing          -> moved to DbLifecycle.odb_writing
//   odbSchema   :: DbSchema               -> DbSchema
//   odbIdleSince :: TVar TimePoint        -> moved to
//                                            DbLifecycle.odb_idle_since:
//                                            Instant
//   odbBaseSlices :: [Maybe Slice]        -> Vec<Option<Slice>>
//   odbOwnership :: TVar (Maybe Ownership)-> Mutex<Option<Arc<Ownership>>>
//                                            (`Ownership` has no Clone:
//                                            ownership.rs:329-331)
//   odbACLMode  :: ACLMode                -> AclMode
//   odbACLMapping :: Map Text UsetId      -> BTreeMap<String, UsetId>
//   odbFirstACLID :: Maybe UsetId         -> Option<UsetId>
//
// DB (Types.hs:146-155) + the row-26 lifecycle:
//   dbRepo      :: Thrift.Repo            -> thrift::Repo
//   dbState     :: TVar DBState           -> DbLifecycle.db_state: DBState
//   dbUsers     :: TVar Int               -> DbLifecycle.db_users: i64
//
// DBState (:132-143) + `envActive`:
//   Open odb    :: OpenDB                 -> Open(Arc<OpenDB>)
//   envActive   :: TVar (HashMap Repo DB) -> Mutex<HashMap<Repo, Arc<DB>>>
//   (both escape the transaction: Open.hs:205-212 returns `wrQueue`/
//   `odbSchema`/`odbHandle` out of `OpenDB`, and :455-478 brackets an
//   owned `DB` used outside the `envActive` transaction)
//
// Write (Types.hs:158-161):
//   writeWait   :: MVar (Either SomeException Subst)
//                                         -> MVar<Result<Subst,
//                                            Box<dyn Error + Send + Sync>>>
//                                            (the SAME MVar as the
//                                            WriteJob's `write_done`;
//                                            Writes.hs:431-436)
//   writeTimeout :: TimePoint             -> Instant
//
// WriteContent (:164-167):
//   writeBatch  :: !Thrift.Batch          -> thrift::Batch
//   writeOwnership :: Maybe DefineOwnership
//                                         -> Option<DefineOwnership>
//
// WriteJob (:170-183):
//   writeSize   :: !Int                   -> i64
//   writeContentIO :: IO WriteContent     -> Box<dyn FnOnce() ->
//                                            WriteContent + Send>
//   writeDone   :: MVar (Either SomeException Subst)
//                                         -> MVar<Result<Subst,
//                                            Box<dyn Error + Send + Sync>>>
//                                            (producer side of the same
//                                            slot; Queue.hs:91/:114/:132)
//   writeStart  :: Point                  -> Point
//   writeOnComplete :: Either SomeException Subst -> IO ()
//                                         -> Box<dyn FnOnce(Result<Subst,
//                                            Box<dyn Error + Send + Sync>>)
//                                            + Send>
//   writeCheckpoint :: IO ()              -> Box<dyn FnOnce() + Send>
//
// WriteQueue (:186-196) — rows 35/36 merge the queue and the four counters
// into ONE lock (`WriteQueueState`), plus a Condvar beside it:
//   writeQueue :: TQueue WriteJob         -> WriteQueueState.queue:
//                                            VecDeque<WriteJob>
//   writeQueueActive :: TVar Int          -> WriteQueueState.active: i64
//   writeQueueCount :: TVar Int           -> WriteQueueState.count: i64
//   writeQueueSize :: TVar Int            -> WriteQueueState.size: i64
//   writeQueueLatency :: TVar TimeSpec    -> WriteQueueState.latency:
//                                            Duration
//
// WriteQueues (:199-202) — one lock (`WriteQueuesState`), plus a Condvar:
//   writeQueues :: TQueue (Thrift.Repo, WriteQueue)
//                                         -> WriteQueuesState.queues:
//                                            VecDeque<(thrift::Repo,
//                                            Arc<WriteQueue>)>
//   writeQueuesSize :: TVar Int           -> WriteQueuesState.size: i64
//   (the deque and the per-DB `Writing.wr_queue` name the SAME queue,
//   Queue.hs:143-150; hence the Arc)
//
// Derivation (:205-212):
//   derivationStart :: TimePoint          -> Instant
//   derivationFinished :: Bool            -> bool
//   derivationStats :: Thrift.UserQueryStats
//                                         -> thrift::UserQueryStats
//   derivationPendingWrites :: [Thrift.Handle]
//                                         -> Vec<thrift::Handle>
//   derivationError :: Maybe (TimePoint, SomeException)
//                                         -> Option<(Instant,
//                                            Box<dyn Error + Send + Sync>)>
//   derivationHandle :: Thrift.Handle     -> thrift::Handle
//
// Env (:244-303): all 42 fields are spelled in the struct; the two
// non-obvious images are `envDbSchemaCache :: MVar DbSchemaCache` ->
// `Mutex<DbSchemaCache>` (the MVar is used as a LOCK: `readMVar` at
// Schema.hs:324 and `modifyMVar_` at Schema.hs:335 / Open.hs:598 — the
// unit-102 mutex reading, not the A3 slot reading) and
// `envSchemaUpdateSignal :: TMVar ()` -> `TMVar<()>`. The fields whose
// source value is shared by reference become `Arc` (TRANSLATION NOTES 9):
// `envActive` (`Arc<DB>`), `envWrites` (`Arc<Write>`),
// `envDerivations` (`Arc<Derivation>`), `envDatabaseJanitor`
// (`Arc<JanitorRunResult>`) and `OpenDB.odbOwnership` (`Arc<Ownership>`).
//
// ---------------------------------------------------------------------------
// SIBLING-UNIT CONTRACTS (this file is a HUB — read before coding a consumer)
// ---------------------------------------------------------------------------
//
//   * FIELD NAMES. Every source field is PUBLIC under its snake_case name,
//     except the two source fields that row 26 moves into `DbLifecycle`
//     (`odb_idle_since`, `odb_writing`) and the five source fields rows
//     35/36 merge (`wr_next_id`/`wr_commit` inside `WriteState`;
//     `write_queue_active/count/size/latency` -> `active`/`count`/`size`/
//     `latency` inside `WriteQueueState`, and `write_queues_size` -> `size`
//     inside `WriteQueuesState`). Every folded source name is recorded in
//     the sub-struct's field doc.
//
//   * SHARED HANDLES / ACCESSOR CONTRACT. The source stores these values in
//     TVars/MVars and shares them by reference; Rust stores `Arc<...>` and
//     hands out clones. The bracket image is: take the lock, CLONE THE ARC,
//     DROP THE GUARD, then act on the clone. A guard must never be held
//     across the action (the source's `atomically` blocks end at the
//     bracket edge; holding a `std::sync::MutexGuard` across IO is a
//     deadlock and a `!Send` hazard). `Arc<DB>`, `Arc<OpenDB>`,
//     `Arc<Writing>`, `Arc<WriteQueue>`, `Arc<Derivation>`, `Arc<Write>`,
//     `Arc<JanitorRunResult>` and `Arc<Ownership>` are the images of the
//     source's shared payloads (Open.hs:163-212/:455-478,
//     Queue.hs:143-150, Derive.hs:494-503/:543-558, Writes.hs:334-342,
//     Server.hs:136-141; TRANSLATION NOTES 9). `MVar<T>` is itself a shared
//     handle (`Clone`), so a completion slot can sit in two records at once
//     exactly as in the source.
//
//   * TOTAL LOCK ORDER: ENV_ACTIVE -> DB -> WRITING -> WRITE_QUEUES ->
//     WRITE_QUEUE_STATE, with DB -> OWNERSHIP for the ownership snapshot.
//     `DB`'s lock is `db_lifecycle`; `WRITING` is `Writing.wr_state`;
//     `WRITE_QUEUES` is `WriteQueues.write_queues_state`; the per-queue
//     `WRITE_QUEUE_STATE` is `WriteQueue.write_queue_state`. The
//     close/eviction path takes the queue state after the DB lock
//     (Close.hs:44-49), hence DB -> WRITE_QUEUE_STATE. EVERY operation
//     acquires in this order; in particular enqueue takes WRITE_QUEUES
//     FIRST, even when the queue is non-empty, because the source's
//     "wasEmpty then register" pair is one STM transaction
//     (Queue.hs:100-120/:143-150). Never take these in the reverse order.
//
//   * CROSS-GROUP ATOMICITY. Writes.hs:128-132 updates the four per-queue
//     counters AND `writeQueuesSize` in one `atomically`. Rows 35/36 make
//     the per-queue side one lock (`WriteQueueState`); the global size is
//     `WriteQueuesState.size`. The Rust writer must take WRITE_QUEUES then
//     WRITE_QUEUE_STATE for that block (parent before child, the
//     ENV_ACTIVE -> DB shape).
//
//   * `WriteJob.write_done` SIGNAL ORDER (Writes.hs:134-137): update the
//     stats/`active` BEFORE `try_put(write_done)`, or `finish` can observe
//     `active /= 0` and fail. This is the source's own comment.
//
//   * `MVar<T>` / `TMVar<T>` are Rust-only images of RULE §2's MVar/TMVar
//     rows (no shipped generic slot exists). Later units with an MVar/TMVar
//     field should reuse these from `crate::database::types` (or move them
//     to a shared util at Step 4), not re-implement the latch.
//     `MVar::read`/`read_timeout`/`try_read` take a closure so a non-Clone
//     payload (`Subst`) can be inspected in place: the image of
//     `readMVar`/`tryReadMVar`, which share the value rather than copying.
//     `MVar<T>` is `Clone` and `Arc`-backed: the Haskell `MVar` is a shared
//     mutable reference, and `writeWait`/`writeDone` are two names for one
//     slot (Writes.hs:431-436, Queue.hs:91/:114/:132).
//
//   * `Storage` (the enum) implements `StorageTrait` with
//     `type Database = SomeDatabaseOps`. This is the closed-set image of
//     the source's rank-2 unpack in `withStorageFor`
//     (`forall s. Storage s => s -> IO a`): Rust has no rank-2 closure, and
//     `Storage` is not object-safe (`with_scratch_root` is generic), so the
//     enum is the only faithful dispatcher. Consumers call the trait
//     methods on `&Storage` exactly as the Haskell callback called them on
//     `s`. `SomeDatabaseOps` implements `CanLookup` + `DatabaseOps` (the
//     source's Storage.hs:296-324 forwarding instances; storage.rs:671-678
//     assigns them to this unit).
//
//   * `withStorageFor` / `withDefaultStorage` signatures:
//         pub fn with_storage_for<A, E, F>(
//             env: &Env, repo: &Repo, meta: &Meta, f: F,
//         ) -> Result<A, E>
//         where E: From<DbError>, F: FnOnce(&Storage) -> Result<A, E>;
//
//         pub fn with_default_storage<A, E, F>(env: &Env, f: F) -> Result<A, E>
//         where E: From<thrift::Exception>,
//               F: FnOnce(&StorageName, &Storage) -> Result<A, E>;
//     The generic `E` is the image of the source's `IO a` callback, whose
//     exceptions are unconstrained; the `From` bound is the
//     storage.rs:254 `with_materialized_file<E: From<io::Error>>`
//     precedent. `withStorageFor`'s own failure is `DbError` (source
//     `dbError repo`, :316-317); `withDefaultStorage`'s is
//     `thrift::Exception` (source `throwIO $ Thrift.Exception`,
//     :333-334) — the two source error channels are deliberately NOT
//     unified.
//
//   * `Env` is the process-wide environment. It is currently `!Send` +
//     `!Sync` in Rust (see the TODO at the struct): `Storage` carries the
//     Memory backend whose `Mutex<FactSet>` is `!Send` (memory.rs:218-227,
//     fact_set.rs:405-424), `Writing.wr_state` carries a `FactSet`, and
//     `Backends` has its own payload Send question (backend.rs:134-138);
//     `env_event_base: EventBaseDataplane` and `env_cached_restorable_dbs`
//     (which snapshots `Meta`) are SUSPECTED causes. This also blocks
//     `WriteJob`'s `+ Send` fields (the boxed `FnOnce`s capture `Env`:
//     JSON.hs:81-87, Derive.hs:138 — and `odbHandle`, Queue.hs:58-70), so
//     no `WriteJob` can be constructed until it is resolved. The source
//     `Env` is shared across every thread. Resolving this (thread-local
//     Memory, a revised FactSet posture in glean-core, or a Step-4 owner
//     model) is the open decision this row inherits from row 71 (rows 71
//     and 104 are the named owners); it is NOT made by adding an
//     `unsafe impl` here (RULE §3).
//
// ---------------------------------------------------------------------------
// TRANSLATION NOTES
// ---------------------------------------------------------------------------
//
//   1. DERIVES. The source declares:
//        - `JanitorRunResult deriving Show` -> `impl Display` with the
//          constructor-shaped text (RULE §2 A4/B6); no Debug/Clone.
//        - `JanitorException deriving (Typeable, Show)` +
//          `instance Exception` -> `impl Display` + `impl Debug`
//          (delegating to Display, the exception.rs convention) +
//          `impl std::error::Error`. `Typeable` is automatic in Rust and
//          is not spelled.
//        - `NFData Derivation` (hand-written, :214-222) -> documented
//          NON-GOAL: `rnf` only forces thunks, and Rust's owned fields are
//          already evaluated; there is no `deepseq` analogue and no
//          observable behaviour to reproduce (the instance is used by
//          Derive.hs only to force the record before storing it).
//        - Every other type has NO derives in the source, so none is
//          added here (RULE: no gratuitous derives). In particular
//          `EnableRecursion`, `DBState`, `Writing`, `OpenDB`, `DB`,
//          `Write`, `WriteContent`, `WriteJob`, `WriteQueue`, `WriteQueues`
//          and `Env` carry none.
//   2. ROW 26'S RECORD SPLIT. `DbLifecycle` owns `db_state`, `db_users`,
//      `odb_idle_since` and `odb_writing`. Invariant: `odb_writing` is
//      `Some` only while `db_state` is `DBState::Open(_)`; a read-only open
//      DB is `Open` with `odb_writing == None` (the source's own
//      "Nothing means DB is read only", :103). `odb_idle_since` is
//      meaningful only in the Open states (initialized at `newDB`,
//      Open.hs:729).
//   3. `NFData Derivation` — see note 1.
//   4. `Int` -> `i64` (LP64 standing fact); `Word32` (`UsetId`) and
//      `Word64` (`Fid`'s inner `Id`) keep their shipped newtypes.
//   5. `SomeException` -> `Box<dyn Error + Send + Sync>` (database/
//      exception.rs's pinned image). LOSSINESS: Haskell's `SomeException`
//      can wrap any `Exception` instance, including ones with no `Error`
//      analogue; the Rust box only holds `std::error::Error` values. This
//      is the same lossy image every shipped unit uses (DbError::Other,
//      BackendError::Other, LogDerivationResult), and it is exact for
//      every exception this tree raises.
//   6. `MVar` AS A LOCK. `envDbSchemaCache :: MVar DbSchemaCache` is not a
//      communication slot: Schema.hs:324 `readMVar` and Schema.hs:335 /
//      Open.hs:598 `modifyMVar_` use it as a mutex, i.e. the unit-102
//      MVar-as-mutex reading. It is therefore a plain
//      `std::sync::Mutex<DbSchemaCache>`. The A3 slot type (`MVar<T>`) is
//      used only for `writeWait`/`writeDone`, which are genuine
//      put-once/read-many completion slots.
//   7. `writeQueueLatency :: TVar TimeSpec` -> `Duration`: the value is a
//      monotonic difference (`writeEnd - pointStart writeStart`,
//      Writes.hs:119-121), and RULE §1 B3 maps durations to
//      `std::time::Duration`.
//   8. `envSchemaSource :: Observed SchemaIndex` -> `Observed<Arc<SchemaIndex>>`
//      (DIVERGENCES 1).
//   9. SHARED HANDLES. Haskell's `TVar`/`MVar` values are references: one
//      object can be named by two records and outlive the transaction that
//      looked it up. The Rust image is `Arc`, cloned out of the lock and
//      used after the guard is dropped (SIBLING-UNIT CONTRACTS):
//        - `MVar<T>` is `Clone` + `Arc`-backed, so `Write.write_wait` and
//          `WriteJob.write_done` are the same slot (Queue.hs:91/:114/:132,
//          Writes.hs:431-436);
//        - `Writing.wr_queue: Arc<WriteQueue>`, and the round-robin deque
//          element is `(Repo, Arc<WriteQueue>)` — both name the same queue
//          (Open.hs:209-212, Queue.hs:143-150);
//        - `env_active: Mutex<HashMap<Repo, Arc<DB>>>` and
//          `DBState::Open(Arc<OpenDB>)` (Open.hs:455-478, :163-201,
//          :205-212);
//        - `DbLifecycle.odb_writing: Option<Arc<Writing>>`;
//        - `env_derivations: Mutex<HashMap<_, Arc<Derivation>>>`
//          (Derive.hs:494-503, :560-566);
//        - `env_writes: Mutex<HashMap<_, Arc<Write>>>` (Writes.hs:334-342,
//          :394-412);
//        - `env_database_janitor: Mutex<Option<(SystemTime,
//          Arc<JanitorRunResult>)>>` (Server.hs:136-141, :158-166);
//        - `OpenDB.odb_ownership: Mutex<Option<Arc<Ownership>>>`
//          (`Ownership` has no `Clone`: ownership.rs:329-331;
//          Derive.hs:543-558).
//      These are images, not divergences: the sharing is the source's.
//
// ---------------------------------------------------------------------------
// DIVERGENCES (deliberate; no inventory row governs them)
// ---------------------------------------------------------------------------
//
//   1. `envSchemaSource` is `Observed<Arc<SchemaIndex>>`, not
//      `Observed<SchemaIndex>`. `SchemaIndex` is NOT `Clone`
//      (config.rs:188-191: HashedSchema/compute_ids.rs:264 has no Clone
//      and the source has no deriving clause), and `Observed::get` returns
//      A by value / `fixed_value` requires `A: Clone + Send + Sync`. The
//      shipped config already uses `ThriftSource<Arc<SchemaIndex>, C>` for
//      the same reason (config.rs DIVERGENCES 4). The Arc preserves
//      sharing exactly.
//   2. `with_storage_for`/`with_default_storage` take a generic error `E`
//      instead of a fixed error type. The source's callbacks are `IO a`
//      with unconstrained exceptions; a fixed `DbError` (or
//      `thrift::Exception`) callback would force every caller to invent a
//      conversion. `E: From<...>` keeps the source's two error channels
//      and lets the callback's own error type through (the
//      storage.rs:254 precedent).
//   3. The `Storage` enum implements `StorageTrait` (Rust-only forwarding
//      impl; the source has no `instance Storage (Some Storage)` because
//      it unpacks the existential instead). Required to image the rank-2
//      callback; see SIBLING-UNIT CONTRACTS.
//   4. `env_database_janitor_changed: Condvar` and
//      `env_shutting_down_changed: Condvar` are Rust-only companion fields
//      (row 95's STM-retry image; row 36's queues-closed wait). Neither is
//      a source field; the shutdown setter notifies the latter.
//   5. `WriteQueue`/`WriteQueues` carry `Condvar`s (`write_queue_changed`,
//      `write_queues_changed`) beside their `Mutex<WriteQueueState>` /
//      `Mutex<WriteQueuesState>`; the TQueue image is RULEBOOK §2's STM
//      row (`Mutex<State>` for the multi-variable transaction + Condvar),
//      not a RUNBOOK standing fact (RUNBOOK.md:59 lists only Int, thrift,
//      Set and Map).
//
// ---------------------------------------------------------------------------
// MARKERS: two port todos (the Env !Send decision; the RateLimiterMap
// kit), three inventory-bug candidates (rows 33, 35 and 137), zero
// source-defect reproductions, zero performance deferrals, zero SAFETY
// notes (no unsafe in this file). The two TODO sites are marked once each
// in the body; the tally above avoids the literal marker spellings.
// ---------------------------------------------------------------------------

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::error::Error;
use std::ffi::c_void;
use std::fmt;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError, TryLockError};
use std::time::{Duration, Instant, SystemTime};

use glean_angle::angle::types::PredicateId;
use glean_core::ffi::FfiError;
use glean_core::rts::foreign::fact_set::FactSet;
use glean_core::rts::foreign::inventory::Inventory;
use glean_core::rts::foreign::lookup::{CanLookup, Lookup};
use glean_core::rts::foreign::lookup_cache::{LookupCache, Stats as LookupCacheStats};
use glean_core::rts::foreign::ownership::{
    ComputedOwnership, DefineOwnership, Ownership, Slice, UnitId, UsetId,
};
use glean_core::rts::foreign::subst::Subst;
use glean_core::rts::types::{Fid, Pid};
use glean_core::write::stats::Stats;
use glean_thrift as thrift;
use glean_thrift::{Config as ServerConfig, EventBaseDataplane};
use glean_util::util::config_provider::ConfigError;
use glean_util::util::metric::Point;
use glean_util::util::observed::Observed;
use glean_util::util::shard_manager::SomeShardManager;
use glean_util::util::trace::Listener;
use glean_util::util::warden::{Task, Warden};

use crate::database::backup::backend::{Backends, Data};
use crate::database::batch_location::SomeParser;
use crate::database::catalog::Catalog;
use crate::database::config::{rocksdb_name, SchemaIndex, StorageName, Tracer};
use crate::database::exception::{db_error, DbError};
use crate::database::meta::{AclMode, Meta};
use crate::database::schema::types::{DbSchema, DbSchemaCache};
use crate::database::storage::lmdb::{Database as LmdbDatabase, LMDB};
use crate::database::storage::memory::{Database as MemoryDatabase, Memory};
use crate::database::storage::rocks_db::{Database as RocksDatabase, RocksDB};
use crate::database::storage::{
    AxiomOwnership, DBVersion, DatabaseOps, Mode, RestoreSource, Storage as StorageTrait, WriteLock,
};
use crate::database::trace::GleanTrace;
use crate::logger::database::GleanDatabaseLogger;
use crate::logger::server::GleanServerLogger;

// `DebugFlags(..)` is in the source's export list (:17) and is imported
// from Glean.Database.Config (:49): this is a re-export, not a definition.
pub use crate::database::config::DebugFlags;

// ---------------------------------------------------------------------------
// Storage / SomeDatabaseOps — the closed-set images of the two existentials
// ---------------------------------------------------------------------------

/// `envStorage :: HashMap StorageName (Some Storage)` (:251) and
/// `odbHandle :: Some DatabaseOps` (:101) are the two closed existentials
/// of this file. Inventory rows 136/137 + the shipped config.rs contract:
/// the storage engines are a closed set {RocksDB, LMDB, Memory}, so each
/// becomes a plain enum (RULE §2 GADT row), never `Box<dyn>`.
///
/// `Storage` is the BACKEND enum (`Some Storage`), the one config.rs:522-523
/// / :554 constructs as `Storage::Rocks(RocksDB)` / `Storage::Lmdb(LMDB)` /
/// `Storage::Memory(Memory)`. Its name is fixed by that shipped consumer.
/// It also implements [`StorageTrait`] with `Database = SomeDatabaseOps`,
/// the Rust image of `withStorageFor`'s rank-2 unpack (SIBLING-UNIT
/// CONTRACTS).
pub enum Storage {
    Rocks(RocksDB),
    Lmdb(LMDB),
    Memory(Memory),
}

/// `odbHandle :: Some DatabaseOps` (:101) — the open-database existential.
/// Rust-only name (the variant set is the source's).
///
/// INVENTORY-BUG-CANDIDATE row 137: the row's target names this enum
/// `Storage {Rocks, Lmdb, Memory}`, but the `Storage` name is already
/// committed by the shipped config.rs (row 85) to the BACKENDS existential
/// (`use crate::database::types::Storage;`, `Storage::Rocks(RocksDB)`).
/// Two existentials cannot share one name in one module, and config.rs is
/// the hard consumer. The variant set is exactly the row's; only the enum
/// name differs.
pub enum SomeDatabaseOps {
    Rocks(RocksDatabase),
    Lmdb(LmdbDatabase),
    Memory(MemoryDatabase),
}

impl StorageTrait for Storage {
    type Database = SomeDatabaseOps;

    fn describe(&self) -> String {
        match self {
            Storage::Rocks(s) => s.describe(),
            Storage::Lmdb(s) => s.describe(),
            Storage::Memory(s) => s.describe(),
        }
    }

    fn readable_versions(&self) -> Vec<DBVersion> {
        match self {
            Storage::Rocks(s) => s.readable_versions(),
            Storage::Lmdb(s) => s.readable_versions(),
            Storage::Memory(s) => s.readable_versions(),
        }
    }

    fn writable_versions(&self) -> Vec<DBVersion> {
        match self {
            Storage::Rocks(s) => s.writable_versions(),
            Storage::Lmdb(s) => s.writable_versions(),
            Storage::Memory(s) => s.writable_versions(),
        }
    }

    fn open(
        &self,
        repo: &thrift::Repo,
        mode: Mode,
        version: DBVersion,
    ) -> Result<Self::Database, DbError> {
        match self {
            Storage::Rocks(s) => s.open(repo, mode, version).map(SomeDatabaseOps::Rocks),
            Storage::Lmdb(s) => s.open(repo, mode, version).map(SomeDatabaseOps::Lmdb),
            Storage::Memory(s) => s.open(repo, mode, version).map(SomeDatabaseOps::Memory),
        }
    }

    fn delete(&self, repo: &thrift::Repo) -> Result<(), DbError> {
        match self {
            Storage::Rocks(s) => s.delete(repo),
            Storage::Lmdb(s) => s.delete(repo),
            Storage::Memory(s) => s.delete(repo),
        }
    }

    fn safe_remove_forcibly(&self, repo: &thrift::Repo) -> Result<(), DbError> {
        match self {
            Storage::Rocks(s) => s.safe_remove_forcibly(repo),
            Storage::Lmdb(s) => s.safe_remove_forcibly(repo),
            Storage::Memory(s) => s.safe_remove_forcibly(repo),
        }
    }

    fn get_total_capacity(&self) -> Result<Option<i64>, DbError> {
        match self {
            Storage::Rocks(s) => s.get_total_capacity(),
            Storage::Lmdb(s) => s.get_total_capacity(),
            Storage::Memory(s) => s.get_total_capacity(),
        }
    }

    fn get_used_capacity(&self) -> Result<Option<i64>, DbError> {
        match self {
            Storage::Rocks(s) => s.get_used_capacity(),
            Storage::Lmdb(s) => s.get_used_capacity(),
            Storage::Memory(s) => s.get_used_capacity(),
        }
    }

    fn get_free_capacity(&self) -> Result<i64, DbError> {
        match self {
            Storage::Rocks(s) => s.get_free_capacity(),
            Storage::Lmdb(s) => s.get_free_capacity(),
            Storage::Memory(s) => s.get_free_capacity(),
        }
    }

    fn with_scratch_root<T, F>(&self, action: F) -> Result<T, DbError>
    where
        F: FnOnce(&str) -> Result<T, DbError>,
    {
        match self {
            Storage::Rocks(s) => s.with_scratch_root(action),
            Storage::Lmdb(s) => s.with_scratch_root(action),
            Storage::Memory(s) => s.with_scratch_root(action),
        }
    }

    fn restore(
        &self,
        config: &thrift::Config,
        repo: &thrift::Repo,
        scratch: &str,
        source: RestoreSource,
    ) -> Result<(), DbError> {
        match self {
            Storage::Rocks(s) => s.restore(config, repo, scratch, source),
            Storage::Lmdb(s) => s.restore(config, repo, scratch, source),
            Storage::Memory(s) => s.restore(config, repo, scratch, source),
        }
    }
}

// The source's `instance CanLookup (Some DatabaseOps)` (Storage.hs:296-298)
// and `instance DatabaseOps (Some DatabaseOps)` (Storage.hs:300-324).
// storage.rs:671-678 assigns both to this unit.

impl CanLookup for SomeDatabaseOps {
    fn with_lookup<T, E: From<FfiError>>(
        &self,
        f: impl FnOnce(*mut c_void) -> Result<T, E>,
    ) -> Result<T, E> {
        match self {
            SomeDatabaseOps::Rocks(db) => db.with_lookup(f),
            SomeDatabaseOps::Lmdb(db) => db.with_lookup(f),
            SomeDatabaseOps::Memory(db) => db.with_lookup(f),
        }
    }

    fn lookup_name(&self) -> &str {
        match self {
            SomeDatabaseOps::Rocks(db) => db.lookup_name(),
            SomeDatabaseOps::Lmdb(db) => db.lookup_name(),
            SomeDatabaseOps::Memory(db) => db.lookup_name(),
        }
    }
}

impl DatabaseOps for SomeDatabaseOps {
    fn close(&self) -> Result<(), DbError> {
        match self {
            SomeDatabaseOps::Rocks(db) => db.close(),
            SomeDatabaseOps::Lmdb(db) => db.close(),
            SomeDatabaseOps::Memory(db) => db.close(),
        }
    }

    fn predicate_stats(&self) -> Result<Vec<(Pid, thrift::PredicateStats)>, DbError> {
        match self {
            SomeDatabaseOps::Rocks(db) => db.predicate_stats(),
            SomeDatabaseOps::Lmdb(db) => db.predicate_stats(),
            SomeDatabaseOps::Memory(db) => db.predicate_stats(),
        }
    }

    fn store(&self, key: &[u8], value: &[u8]) -> Result<(), DbError> {
        match self {
            SomeDatabaseOps::Rocks(db) => db.store(key, value),
            SomeDatabaseOps::Lmdb(db) => db.store(key, value),
            SomeDatabaseOps::Memory(db) => db.store(key, value),
        }
    }

    fn retrieve(&self, key: &[u8]) -> Result<Option<Vec<u8>>, DbError> {
        match self {
            SomeDatabaseOps::Rocks(db) => db.retrieve(key),
            SomeDatabaseOps::Lmdb(db) => db.retrieve(key),
            SomeDatabaseOps::Memory(db) => db.retrieve(key),
        }
    }

    fn add_batch_descriptor(&self, descriptor: &thrift::BatchDescriptor) -> Result<(), DbError> {
        match self {
            SomeDatabaseOps::Rocks(db) => db.add_batch_descriptor(descriptor),
            SomeDatabaseOps::Lmdb(db) => db.add_batch_descriptor(descriptor),
            SomeDatabaseOps::Memory(db) => db.add_batch_descriptor(descriptor),
        }
    }

    // storage.rs:499-502 keeps `BatchLocation = String` private; the alias
    // is transparent, so `&String` is the trait's `&BatchLocation`.
    fn mark_batch_descriptor_as_written(&self, location: &String) -> Result<(), DbError> {
        match self {
            SomeDatabaseOps::Rocks(db) => db.mark_batch_descriptor_as_written(location),
            SomeDatabaseOps::Lmdb(db) => db.mark_batch_descriptor_as_written(location),
            SomeDatabaseOps::Memory(db) => db.mark_batch_descriptor_as_written(location),
        }
    }

    fn is_batch_descriptor_stored(&self, location: &String) -> Result<bool, DbError> {
        match self {
            SomeDatabaseOps::Rocks(db) => db.is_batch_descriptor_stored(location),
            SomeDatabaseOps::Lmdb(db) => db.is_batch_descriptor_stored(location),
            SomeDatabaseOps::Memory(db) => db.is_batch_descriptor_stored(location),
        }
    }

    fn get_unprocessed_batch_descriptors(&self) -> Result<Vec<thrift::BatchDescriptor>, DbError> {
        match self {
            SomeDatabaseOps::Rocks(db) => db.get_unprocessed_batch_descriptors(),
            SomeDatabaseOps::Lmdb(db) => db.get_unprocessed_batch_descriptors(),
            SomeDatabaseOps::Memory(db) => db.get_unprocessed_batch_descriptors(),
        }
    }

    fn commit(&self, facts: &FactSet) -> Result<(), DbError> {
        match self {
            SomeDatabaseOps::Rocks(db) => db.commit(facts),
            SomeDatabaseOps::Lmdb(db) => db.commit(facts),
            SomeDatabaseOps::Memory(db) => db.commit(facts),
        }
    }

    fn add_ownership<W>(
        &self,
        lock: &WriteLock<W>,
        ownership: AxiomOwnership,
    ) -> Result<(), DbError> {
        match self {
            SomeDatabaseOps::Rocks(db) => db.add_ownership(lock, ownership),
            SomeDatabaseOps::Lmdb(db) => db.add_ownership(lock, ownership),
            SomeDatabaseOps::Memory(db) => db.add_ownership(lock, ownership),
        }
    }

    fn register_acl_units<W>(
        &self,
        lock: &WriteLock<W>,
        names: &[Vec<u8>],
    ) -> Result<UnitId, DbError> {
        match self {
            SomeDatabaseOps::Rocks(db) => db.register_acl_units(lock, names),
            SomeDatabaseOps::Lmdb(db) => db.register_acl_units(lock, names),
            SomeDatabaseOps::Memory(db) => db.register_acl_units(lock, names),
        }
    }

    fn optimize(&self, compact: bool) -> Result<(), DbError> {
        match self {
            SomeDatabaseOps::Rocks(db) => db.optimize(compact),
            SomeDatabaseOps::Lmdb(db) => db.optimize(compact),
            SomeDatabaseOps::Memory(db) => db.optimize(compact),
        }
    }

    fn flush(&self) -> Result<(), DbError> {
        match self {
            SomeDatabaseOps::Rocks(db) => db.flush(),
            SomeDatabaseOps::Lmdb(db) => db.flush(),
            SomeDatabaseOps::Memory(db) => db.flush(),
        }
    }

    fn compute_ownership(
        &self,
        base: Option<&Lookup>,
        inventory: &Inventory,
    ) -> Result<ComputedOwnership, DbError> {
        match self {
            SomeDatabaseOps::Rocks(db) => db.compute_ownership(base, inventory),
            SomeDatabaseOps::Lmdb(db) => db.compute_ownership(base, inventory),
            SomeDatabaseOps::Memory(db) => db.compute_ownership(base, inventory),
        }
    }

    fn store_ownership<W>(
        &self,
        lock: &WriteLock<W>,
        ownership: &ComputedOwnership,
    ) -> Result<(), DbError> {
        match self {
            SomeDatabaseOps::Rocks(db) => db.store_ownership(lock, ownership),
            SomeDatabaseOps::Lmdb(db) => db.store_ownership(lock, ownership),
            SomeDatabaseOps::Memory(db) => db.store_ownership(lock, ownership),
        }
    }

    fn get_ownership(&self) -> Result<Option<Ownership>, DbError> {
        match self {
            SomeDatabaseOps::Rocks(db) => db.get_ownership(),
            SomeDatabaseOps::Lmdb(db) => db.get_ownership(),
            SomeDatabaseOps::Memory(db) => db.get_ownership(),
        }
    }

    fn get_unit_id(&self, unit: &[u8]) -> Result<Option<UnitId>, DbError> {
        match self {
            SomeDatabaseOps::Rocks(db) => db.get_unit_id(unit),
            SomeDatabaseOps::Lmdb(db) => db.get_unit_id(unit),
            SomeDatabaseOps::Memory(db) => db.get_unit_id(unit),
        }
    }

    fn get_unit(&self, id: UnitId) -> Result<Option<Vec<u8>>, DbError> {
        match self {
            SomeDatabaseOps::Rocks(db) => db.get_unit(id),
            SomeDatabaseOps::Lmdb(db) => db.get_unit(id),
            SomeDatabaseOps::Memory(db) => db.get_unit(id),
        }
    }

    fn get_units_by_prefix(&self, prefix: &[u8]) -> Result<Vec<(Vec<u8>, UnitId)>, DbError> {
        match self {
            SomeDatabaseOps::Rocks(db) => db.get_units_by_prefix(prefix),
            SomeDatabaseOps::Lmdb(db) => db.get_units_by_prefix(prefix),
            SomeDatabaseOps::Memory(db) => db.get_units_by_prefix(prefix),
        }
    }

    fn add_define_ownership<W>(
        &self,
        lock: &WriteLock<W>,
        define: &DefineOwnership,
    ) -> Result<(), DbError> {
        match self {
            SomeDatabaseOps::Rocks(db) => db.add_define_ownership(lock, define),
            SomeDatabaseOps::Lmdb(db) => db.add_define_ownership(lock, define),
            SomeDatabaseOps::Memory(db) => db.add_define_ownership(lock, define),
        }
    }

    fn compute_derived_ownership<W>(
        &self,
        lock: &WriteLock<W>,
        ownership: &Ownership,
        base: Option<&Lookup>,
        pid: Pid,
    ) -> Result<ComputedOwnership, DbError> {
        match self {
            SomeDatabaseOps::Rocks(db) => db.compute_derived_ownership(lock, ownership, base, pid),
            SomeDatabaseOps::Lmdb(db) => db.compute_derived_ownership(lock, ownership, base, pid),
            SomeDatabaseOps::Memory(db) => db.compute_derived_ownership(lock, ownership, base, pid),
        }
    }

    fn cache_ownership(&self) -> Result<(), DbError> {
        match self {
            SomeDatabaseOps::Rocks(db) => db.cache_ownership(),
            SomeDatabaseOps::Lmdb(db) => db.cache_ownership(),
            SomeDatabaseOps::Memory(db) => db.cache_ownership(),
        }
    }

    fn prepare_fact_owner_cache(&self) -> Result<(), DbError> {
        match self {
            SomeDatabaseOps::Rocks(db) => db.prepare_fact_owner_cache(),
            SomeDatabaseOps::Lmdb(db) => db.prepare_fact_owner_cache(),
            SomeDatabaseOps::Memory(db) => db.prepare_fact_owner_cache(),
        }
    }

    fn backup<A, F>(&self, config: &thrift::Config, scratch: &str, action: F) -> Result<A, DbError>
    where
        F: FnOnce(&str, Data) -> Result<A, DbError>,
    {
        match self {
            SomeDatabaseOps::Rocks(db) => db.backup(config, scratch, action),
            SomeDatabaseOps::Lmdb(db) => db.backup(config, scratch, action),
            SomeDatabaseOps::Memory(db) => db.backup(config, scratch, action),
        }
    }
}

// ---------------------------------------------------------------------------
// Write pipeline records (:78-202)
// ---------------------------------------------------------------------------

/// 'data Writing' (:78-96) — the write caches of one writable DB.
///
/// Row 34 merges `wrNextId` (IORef Fid) and `wrCommit` (TVar (Maybe (Fid,
/// FactSet))) into ONE lock (`wr_state`): the source reads wrCommit and
/// conditionally writes it in a single transaction (Batch.hs:237-245) and
/// then bumps wrNextId (:249), and the finish path (Finish.hs:83) clears
/// writing under the DB lock. The source names are preserved as the public
/// fields of [`WriteState`].
pub struct Writing {
    /// `wrLock :: Mutex (WriteLock ())` (:80). The unit-102 mutex reading:
    /// the `Mutex` newtype dissolves to `std::sync::Mutex`.
    pub wr_lock: Mutex<WriteLock<()>>,

    /// The merged `wrNextId` + `wrCommit` lock (row 34).
    pub wr_state: Mutex<WriteState>,

    /// `wrLookupCache :: LookupCache` (:86). Not Clone (an owned C++
    /// handle); `Writing` is moved into its `Arc`, never copied.
    pub wr_lookup_cache: LookupCache,

    /// `wrLookupCacheAnchorName :: TVar (Maybe Text)` (:87).
    pub wr_lookup_cache_anchor_name: Mutex<Option<String>>,

    /// `wrQueue :: WriteQueue` (:90). `Arc` because the SAME queue is in
    /// the round-robin deque: Open.hs:209-212 hands `wrQueue` out of
    /// `OpenDB`, and Queue.hs:143-150 pushes the same handle into
    /// `WriteQueues.writeQueues` (TRANSLATION NOTES 9).
    pub wr_queue: Arc<WriteQueue>,
}

/// The row-34 merge of `wrNextId` (:83) and `wrCommit` (:95). Rust-only
/// (the source has no such record; the fields live directly on `Writing`).
///
/// Invariant (the source's two-phase commit): `wr_commit` is `Some((id,
/// facts))` only while a commit is in progress; `wr_next_id` is the first
/// free id of the pipeline and never decreases (Batch.hs:245-249).
pub struct WriteState {
    /// `wrNextId :: IORef Fid` (:83).
    pub wr_next_id: Fid,

    /// `wrCommit :: TVar (Maybe (Fid, FactSet))` (:95) — the pre-commit
    /// next id and the FactSet being committed.
    pub wr_commit: Option<(Fid, FactSet)>,
}

/// 'data OpenDB' (:99-129) — an open database.
///
/// Per inventory row 26, the lifecycle fields `odbIdleSince` (:110) and
/// `odbWriting` (:104) live in [`DbLifecycle`], not here (TRANSLATION
/// NOTES 2). Everything else is field-for-field, except the `Arc` on
/// `odb_ownership` (TRANSLATION NOTES 9).
pub struct OpenDB {
    /// `odbHandle :: Some DatabaseOps` (:101) — the closed enum image.
    pub odb_handle: SomeDatabaseOps,

    /// `odbSchema :: DbSchema` (:107).
    pub odb_schema: DbSchema,

    /// `odbBaseSlices :: [Maybe Slice]` (:114): this DB's base first, then
    /// the base's base, etc.
    pub odb_base_slices: Vec<Option<Slice>>,

    /// `odbOwnership :: TVar (Maybe Ownership)` (:117). `Arc` because the
    /// value escapes the lock (Derive.hs:543-558 reads the TVar, then uses
    /// it) and `Ownership` has no `Clone` (ownership.rs:329-331).
    pub odb_ownership: Mutex<Option<Arc<Ownership>>>,

    /// `odbACLMode :: ACLMode` (:120) — cached from metaProperties.
    pub odb_acl_mode: AclMode,

    /// `odbACLMapping :: Map Text UsetId` (:123) — ACL name-to-ID mapping
    /// for resolving group names at query time.
    pub odb_acl_mapping: BTreeMap<String, UsetId>,

    /// `odbFirstACLID :: Maybe UsetId` (:128) — the boundary between
    /// regular ownership USetIDs and ACL USetIDs. ACL group units occupy
    /// the contiguous range
    /// `[firstACLID, firstACLID + Map.size odbACLMapping)`.
    pub odb_first_acl_id: Option<UsetId>,
}

/// 'data DBState' (:132-143) — the four lifecycle states. No derives: the
/// source declares none.
pub enum DBState {
    /// In the process of being open (:134).
    Opening,
    /// Currently open (:137). `Arc` because `OpenDB` is handed out of the
    /// `envActive` transaction and outlives the DB lock (Open.hs:205-212:
    /// `withWritableDatabase` returns `wrQueue`/`odbSchema`/`odbHandle`
    /// out of `OpenDB`).
    Open(Arc<OpenDB>),
    /// In the process of being closed (:140).
    Closing,
    /// Currently closed (:143).
    Closed,
}

/// The row-26 lifecycle lock group. Rust-only (the source keeps the four
/// fields on two records; row 26 merges them).
///
/// The source keeps `dbState`/`dbUsers` in `DB` (:151/:154) and
/// `odbIdleSince`/`odbWriting` in `OpenDB` (:110/:104). The eviction
/// predicate (Close.hs:44-49) reads all four in one STM transaction, so
/// the port groups them under ONE lock (row 26's sketch).
///
/// Invariants:
///   * `odb_writing.is_some()` implies `matches!(db_state, DBState::Open(_))`
///     (a closed DB has no writing state);
///   * a read-only open DB is `DBState::Open(_)` with `odb_writing == None`
///     (source: "Nothing means DB is read only", :103);
///   * `odb_idle_since` is meaningful only in the Open states (created at
///     Open.hs:729).
pub struct DbLifecycle {
    /// `dbState :: TVar DBState` (:151).
    pub db_state: DBState,

    /// `dbUsers :: TVar Int` (:154) — the number of users.
    pub db_users: i64,

    /// `odbIdleSince :: TVar TimePoint` (:110) — last use (monotonic).
    pub odb_idle_since: Instant,

    /// `odbWriting :: Maybe Writing` (:104). `Arc` because the `Writing`
    /// record escapes the lock and is used outside the `envActive`
    /// transaction (Open.hs:209-212, :455-478).
    pub odb_writing: Option<Arc<Writing>>,
}

/// 'data DB' (:146-155) — a known database. Shared as `Arc<DB>`: the
/// `withActiveDatabase` bracket (Open.hs:455-478) hands an owned `DB` out
/// of the `envActive` transaction, and `withOpenDatabase` re-enters that
/// bracket recursively (Open.hs:163-201) while the outer `OpenDB` is live.
pub struct DB {
    /// `dbRepo :: Thrift.Repo` (:148).
    pub db_repo: thrift::Repo,

    /// The row-26 lock group (`dbState` + `dbUsers` + the OpenDB lifecycle
    /// fields).
    pub db_lifecycle: Mutex<DbLifecycle>,
}

/// 'data Write' (:158-161) — a write in progress, queryable via pollBatch.
///
/// `Write` escapes `envWrites`: Writes.hs:334-342 reads the map, then
/// `readMVar writeWait` AFTER the transaction, and Writes.hs:394-412
/// snapshots the map for stats. Hence `Arc<Write>` in `Env`.
pub struct Write {
    /// `writeWait :: MVar (Either SomeException Subst)` (:159). The SAME
    /// `MVar` as the `WriteJob.write_done` that completes it
    /// (Writes.hs:431-436 stores the job's MVar here), which is why
    /// [`MVar`] is a shared, `Clone` handle. Read (never consumed) by
    /// `readMVar`/`tryReadMVar` (Writes.hs:342/:412), hence the
    /// closure-based read on [`MVar`].
    pub write_wait: MVar<Result<Subst, Box<dyn Error + Send + Sync>>>,

    /// `writeTimeout :: TimePoint` (:160).
    pub write_timeout: Instant,
}

/// 'data WriteContent' (:164-167) — what we are going to write.
pub struct WriteContent {
    /// `writeBatch :: !Thrift.Batch` (:165). The source's strictness is
    /// implicit in Rust's owned field.
    pub write_batch: thrift::Batch,

    /// `writeOwnership :: Maybe DefineOwnership` (:166).
    pub write_ownership: Option<DefineOwnership>,
}

/// 'data WriteJob' (:170-183) — one job on a [`WriteQueue`].
///
/// The three IO actions are `FnOnce`: each is invoked exactly once by the
/// writer thread (`writeContentIO` at Writes.hs:197, `writeOnComplete` at
/// :126, `writeCheckpoint` when the checkpoint is reached). They must be
/// `Send` because the job moves across the queue's `Mutex` to the writer
/// thread. NOTE: `Env` is currently `!Send`, and these closures capture it
/// (or `odbHandle`), so no `WriteJob` can be constructed until that is
/// resolved — see the TODO at [`Env`] (rows 71/104).
pub enum WriteJob {
    /// 'WriteJob' (:171-178).
    WriteJob {
        /// `writeSize :: {-# UNPACK #-} !Int` (:172).
        write_size: i64,

        /// `writeContentIO :: IO WriteContent` (:173).
        write_content_io: Box<dyn FnOnce() -> WriteContent + Send>,

        /// `writeDone :: MVar (Either SomeException Subst)` (:174). The
        /// producer side of the completion slot (`tryPutMVar`,
        /// Writes.hs:137), and the same shared [`MVar`] that the caller
        /// stores in `Write.write_wait` (Writes.hs:431-436).
        write_done: MVar<Result<Subst, Box<dyn Error + Send + Sync>>>,

        /// `writeStart :: Point` (:175).
        write_start: Point,

        /// `writeOnComplete :: Either SomeException Subst -> IO ()`
        /// (:176) — the action run when the write completes.
        write_on_complete: Box<dyn FnOnce(Result<Subst, Box<dyn Error + Send + Sync>>) + Send>,
    },

    /// 'WriteCheckpoint' (:179-183).
    WriteCheckpoint {
        /// `writeCheckpoint :: IO ()` (:180) — invoke when all preceding
        /// writes (including in-progress ones) have completed.
        write_checkpoint: Box<dyn FnOnce() + Send>,
    },
}

/// 'data WriteQueue' (:186-196) — the queue of WriteJobs and their counters.
///
/// Rust-only merge (rows 35/36): the source's `TQueue WriteJob` and its
/// four `TVar` counters are read/written in one STM transaction
/// (Queue.hs:100-120, Writes.hs:128-137, Finish.hs:65-84), so the port puts
/// them under ONE lock, [`WriteQueueState`], with `write_queue_changed`
/// beside it (RULEBOOK §2's STM row: a `Mutex<State>` per multi-variable
/// transaction).
pub struct WriteQueue {
    /// The `writeQueue :: TQueue WriteJob` (:187) plus the four counters of
    /// :188-194, under one lock. Signal `write_queue_changed` after every
    /// push (the STM `writeTQueue` wakes a blocked `readTQueue`).
    pub write_queue_state: Mutex<WriteQueueState>,

    /// The Condvar paired with `write_queue_state` (Rust-only TQueue
    /// image).
    pub write_queue_changed: Condvar,
}

/// The rows-35/36 merge of the queue and its four counters. Rust-only.
///
/// INVENTORY-BUG-CANDIDATE row 35: the row's target sketches "mpsc channel
/// + Mutex<BatchState>"; the port instead uses RULEBOOK §2's STM-row image
/// — one `Mutex<State>` per multi-variable transaction, which is what the
/// source's single `atomically` blocks require. The `WriteJob` payload and
/// the commit-boundary `wrNextId` monotonicity are unchanged (the latter is
/// `WriteState`, row 34).
///
/// One lock, not two, because `dequeueLoop` reads the queue and
/// `writeQueueActive` in one transaction (Writes.hs:165-190) and
/// `finishDatabase` reads `writeQueueActive` and the queue's emptiness in
/// one transaction (Finish.hs:65-84).
pub struct WriteQueueState {
    /// `writeQueue :: TQueue WriteJob` (:187).
    pub queue: VecDeque<WriteJob>,

    /// `writeQueueActive :: TVar Int` (:188) — the number of active jobs,
    /// used to implement checkpoints.
    pub active: i64,

    /// `writeQueueCount :: TVar Int` (:191) — WriteJobs in the queue
    /// (excluding WriteCheckpoint).
    pub count: i64,

    /// `writeQueueSize :: TVar Int` (:193).
    pub size: i64,

    /// `writeQueueLatency :: TVar TimeSpec` (:194) — the latency of the
    /// most recent write to this queue. A monotonic difference
    /// (Writes.hs:119-121), hence `Duration` (RULE §1 B3).
    pub latency: Duration,
}

/// 'data WriteQueues' (:199-202) — the round-robin queue of write queues.
/// Rust-only merge: the source's `TQueue (Repo, WriteQueue)` and its
/// `TVar Int` size are written in one transaction (Queue.hs:143-150,
/// Writes.hs:128-132), so the port puts them under ONE lock.
pub struct WriteQueues {
    /// `writeQueues :: TQueue (Thrift.Repo, WriteQueue)` (:200) and
    /// `writeQueuesSize :: TVar Int` (:201), under one lock. The element
    /// is `Arc<WriteQueue>` because the SAME queue is `Writing.wr_queue`
    /// (Queue.hs:143-150).
    pub write_queues_state: Mutex<WriteQueuesState>,

    /// The Condvar paired with `write_queues_state` (Rust-only TQueue
    /// image).
    pub write_queues_changed: Condvar,
}

/// The rows-35/36 merge of the round-robin deque and its size counter.
/// Rust-only.
pub struct WriteQueuesState {
    /// `writeQueues :: TQueue (Thrift.Repo, WriteQueue)` (:200).
    pub queues: VecDeque<(thrift::Repo, Arc<WriteQueue>)>,

    /// `writeQueuesSize :: TVar Int` (:201).
    pub size: i64,
}

// ---------------------------------------------------------------------------
// Derivation / EnableRecursion / janitor types (:205-242)
// ---------------------------------------------------------------------------

/// 'data Derivation' (:205-212) — information about a derived stored
/// predicate being derived.
///
/// The source's hand-written `NFData` instance (:214-222) has no Rust
/// analogue: `rnf` only forces thunks, and these fields are owned and
/// already evaluated (TRANSLATION NOTES 1).
///
/// `Arc` in `envDerivations`: Derive.hs:494-503 reads the record, runs IO
/// (`finishedWrites`), then re-locks in `overDerivation`; Derive.hs:560-566
/// returns it. The value therefore escapes the `envDerivations` transaction
/// (TRANSLATION NOTES 9).
pub struct Derivation {
    /// `derivationStart :: TimePoint` (:206).
    pub derivation_start: Instant,

    /// `derivationFinished :: Bool` (:207).
    pub derivation_finished: bool,

    /// `derivationStats :: Thrift.UserQueryStats` (:208).
    pub derivation_stats: thrift::UserQueryStats,

    /// `derivationPendingWrites :: [Thrift.Handle]` (:209).
    pub derivation_pending_writes: Vec<thrift::Handle>,

    /// `derivationError :: Maybe (TimePoint, SomeException)` (:210). The
    /// `(duration, error)` pair must survive for stats reporting (row 40).
    pub derivation_error: Option<(Instant, Box<dyn Error + Send + Sync>)>,

    /// `derivationHandle :: Thrift.Handle` (:211).
    pub derivation_handle: thrift::Handle,
}

/// 'data EnableRecursion' (:224-226) — experimental recursive-query support
/// (testing only). No derives: the source declares none.
pub enum EnableRecursion {
    EnableRecursion,
    DisableRecursion,
}

/// 'data JanitorRunResult' (:228-234), `deriving Show`.
///
/// INVENTORY-BUG-CANDIDATE row 33: the row's target is this type minus
/// `JanitorRunSuccess` (:229), which it OMITS; it quotes `enum JanitorError
/// { Run(JanitorException), Timeout, Stuck, Disabled }`, but neither
/// `JanitorError` nor `JanitorFailed` occurs in the source (`grep -rn
/// "JanitorFailed\|JanitorError" glean/` is empty). The `JanitorException`
/// half is followed in shape, but the row's names `Other`/`FetchBackups`
/// are rendered with the source's names `OtherJanitorException`/
/// `JanitorFetchBackupsFailure`. The source is authoritative: all five
/// constructors are translated. Janitor.hs:373-384 is `FetchBackups`
/// (`ReusedPreviousBackups | FetchedNewBackups | FetchFailure{...}`), a
/// different type.
pub enum JanitorRunResult {
    JanitorRunSuccess,
    JanitorRunFailure(JanitorException),
    JanitorTimeout,
    JanitorStuck,
    JanitorDisabled,
}

/// Derived-Show image (RULE §2 A4/B6): constructor-shaped text, never
/// invented prose. GHC's `deriving Show` renders a constructor applied to
/// one argument as `C <arg>` (a space, not `C(<arg>)`), so the payload is
/// rendered with that space. The inner payload is this crate's `Display`,
/// not `Show` of `SomeException` (TRANSLATION NOTES 5) — the residual
/// divergence for the boxed error.
impl fmt::Display for JanitorRunResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JanitorRunResult::JanitorRunSuccess => f.write_str("JanitorRunSuccess"),
            JanitorRunResult::JanitorRunFailure(e) => write!(f, "JanitorRunFailure ({})", e),
            JanitorRunResult::JanitorTimeout => f.write_str("JanitorTimeout"),
            JanitorRunResult::JanitorStuck => f.write_str("JanitorStuck"),
            JanitorRunResult::JanitorDisabled => f.write_str("JanitorDisabled"),
        }
    }
}

/// 'data JanitorException' (:236-240), `deriving (Typeable, Show)`, plus
/// `instance Exception` (:242).
///
/// The `SomeException` payload image is `Box<dyn Error + Send + Sync>`
/// (database/exception.rs's pinned image; LOSSINESS in TRANSLATION NOTES
/// 5). `Typeable` is automatic in Rust. The `Show` half is `Display`; the
/// `Debug` impl delegates to it (the exception.rs convention, so log text
/// matches `show (exc :: SomeException)`); `instance Exception` is
/// `impl std::error::Error`.
pub enum JanitorException {
    OtherJanitorException(Box<dyn Error + Send + Sync>),
    /// Raised only when no remote DB list is available (:239).
    JanitorFetchBackupsFailure(Box<dyn Error + Send + Sync>),
}

impl fmt::Display for JanitorException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // GHC's derived Show puts a space between a constructor and its
            // single argument; the inner payload is Display, not the Show
            // of SomeException (TRANSLATION NOTES 5).
            JanitorException::OtherJanitorException(e) => {
                write!(f, "OtherJanitorException ({})", e)
            }
            JanitorException::JanitorFetchBackupsFailure(e) => {
                write!(f, "JanitorFetchBackupsFailure ({})", e)
            }
        }
    }
}

impl fmt::Debug for JanitorException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl Error for JanitorException {}

// ---------------------------------------------------------------------------
// Env (:244-303) — the process-wide environment
// ---------------------------------------------------------------------------

/// 'data Env' (:244-303) — every source field, in source order, plus the
/// two Rust-only Condvar companions (`env_database_janitor_changed`,
/// `env_shutting_down_changed`; DIVERGENCES 4).
///
/// TODO(port): `Env` is `!Send` + `!Sync` as written, while the source's
/// `Env` is shared across every thread. Three proven causes and two
/// SUSPECTED ones:
/// (a) `Storage` carries `Memory`, whose `Mutex<FactSet>` is `!Send`
/// (storage/memory.rs:218-227; fact_set.rs:405-424 deliberately withholds
/// `Send`/`Sync`); (b) `Writing.wr_state` carries a `FactSet`; (c)
/// `Backends`' payloads carry their own Send question
/// (backup/backend.rs:134-138); (d) `env_event_base: EventBaseDataplane`
/// is an ASSUMED kit type whose Send-ness is unknown — settle it at the
/// kit (thrift_service.rs:356-358 is the contract); (e)
/// `env_cached_restorable_dbs` snapshots `Meta`, so its accessor needs
/// `Meta: Clone`, and `Meta` comes from the unshipped
/// `crate::internal::types`, so neither its Clone nor its Send posture is
/// settled here.
///
/// This is also why `WriteJob`'s boxed `FnOnce ... + Send` fields cannot be
/// constructed yet: the closures capture `Env` (JSON.hs:81-87,
/// Derive.hs:138) or `odbHandle` (Queue.hs:58-70), so no `WriteJob` value
/// can exist until the Send question is resolved. Rows 71 and 104 jointly
/// own the choice (thread-local Memory, a revised FactSet posture in
/// glean-core, or a Step-4 owner model); this unit does NOT add an
/// `unsafe impl` (RULE §3 confines unsafe to glean-core/glean-storage FFI
/// wrappers). The field types are otherwise final; only the Send/Sync
/// posture is open.
///
/// The source `TVar`/`IORef` fields are `Mutex<T>` (RULEBOOK §2's STM row;
/// RUNBOOK.md:59 lists no TVar/TQueue fact). The two fields whose image is
/// not the literal one:
///   * `envDbSchemaCache :: MVar DbSchemaCache` -> `Mutex<DbSchemaCache>`
///     (the MVar is used as a lock: `readMVar` at Schema.hs:324,
///     `modifyMVar_` at Schema.hs:335 and Open.hs:598);
///   * `envSchemaUpdateSignal :: TMVar ()` -> `TMVar<()>`.
pub struct Env {
    /// `envEventBase :: EventBaseDataplane` (:245). SUSPECTED cause of
    /// `Env`'s `!Send`: the kit type's Send-ness is unknown (see the TODO
    /// above).
    pub env_event_base: EventBaseDataplane,

    /// `envServerLogger :: Some GleanServerLogger` (:246) — inventory
    /// row 136's OPEN logger carve-out, hence `Box<dyn>`; `Send + Sync`
    /// is the Rust thread-boundary constraint (the source class has none;
    /// Env is shared), matching config.rs's `cfg_server_logger`.
    pub env_server_logger: Box<dyn GleanServerLogger + Send + Sync>,

    /// `envDatabaseLogger :: Some GleanDatabaseLogger` (:247).
    pub env_database_logger: Box<dyn GleanDatabaseLogger + Send + Sync>,

    /// `envBatchLocationParser :: Some BatchLocation.Parser` (:248).
    pub env_batch_location_parser: SomeParser,

    /// `envLoggerRateLimit :: RateLimiterMap Text` (:249).
    pub env_logger_rate_limit: RateLimiterMap<String>,

    /// `envCatalog :: Catalog` (:250).
    pub env_catalog: Catalog,

    /// `envStorage :: HashMap StorageName (Some Storage)` (:251).
    pub env_storage: HashMap<StorageName, Storage>,

    /// `envDefaultStorage :: (StorageName, Bool)` (:252) — the name and
    /// whether the CLI forced it (the Bool read at :327).
    pub env_default_storage: (StorageName, bool),

    /// `envSchemaSource :: Observed SchemaIndex` (:253) — the schema source;
    /// its parsed/resolved form is cached here too. `Arc` per DIVERGENCES 1.
    pub env_schema_source: Observed<Arc<SchemaIndex>>,

    /// `envDbSchemaCache :: MVar DbSchemaCache` (:255) — a lock, not a
    /// slot (`readMVar` at Schema.hs:324; `modifyMVar_` at Schema.hs:335
    /// and Open.hs:598).
    pub env_db_schema_cache: Mutex<DbSchemaCache>,

    /// `envUpdateSchema :: Bool` (:256).
    pub env_update_schema: bool,

    /// `envSchemaUpdateSignal :: TMVar ()` (:257) — the one-shot latch
    /// (`newEmptyTMVarIO`, Env.hs:138; `takeTMVar`/`tryPutTMVar`,
    /// Env.hs:238/:241).
    pub env_schema_update_signal: TMVar<()>,

    /// `envSchemaId :: Maybe Thrift.SchemaId` (:258) — the session schema
    /// id for the local backend.
    pub env_schema_id: Option<thrift::SchemaId>,

    /// `envServerConfig :: Observed ServerConfig.Config` (:260).
    pub env_server_config: Observed<ServerConfig>,

    /// `envBackupBackends :: Backup.Backends` (:261).
    pub env_backup_backends: Backends,

    /// `envActive :: TVar (HashMap Thrift.Repo DB)` (:262). `Arc<DB>`
    /// because the bracket hands the DB out of the transaction
    /// (Open.hs:455-478).
    pub env_active: Mutex<HashMap<thrift::Repo, Arc<DB>>>,

    /// `envDeleting :: TVar (HashMap Thrift.Repo (Async ()))` (:263).
    pub env_deleting: Mutex<HashMap<thrift::Repo, Task<()>>>,

    /// `envCompleting :: TVar (HashMap Thrift.Repo (Async ()))` (:264).
    pub env_completing: Mutex<HashMap<thrift::Repo, Task<()>>>,

    /// `envCompletingDerived :: TVar (HashMap Thrift.Repo (HashMap
    /// PredicateId (Async ())))` (:265-266).
    pub env_completing_derived: Mutex<HashMap<thrift::Repo, HashMap<PredicateId, Task<()>>>>,

    /// `envShuttingDown :: TVar Bool` (:267) — set on SIGTERM to reject new
    /// writes and quiesce the writer threads while Incomplete DBs are
    /// backed up (Server/Sharding.hs:237; see
    /// `Glean.Database.Backup.Incomplete`).
    pub env_shutting_down: Mutex<bool>,

    /// Rust-only companion to `env_shutting_down` (row 36's queues-closed
    /// wait): the Condvar the writer's `dequeueLoop` blocks on when
    /// `env_shutting_down` is true (Writes.hs:165-190 parks via STM
    /// `retry`). The shutdown setter MUST set the bool and notify this
    /// Condvar while holding the paired `env_shutting_down` lock. Not a
    /// source field.
    pub env_shutting_down_changed: Condvar,

    /// `envReadOnly :: Bool` (:271).
    pub env_read_only: bool,

    /// `envMockWrites :: Bool` (:272).
    pub env_mock_writes: bool,

    /// `envStats :: Stats` (:273) — glean-core's write Stats.
    pub env_stats: Stats,

    /// `envLookupCacheStats :: LookupCache.Stats` (:274).
    pub env_lookup_cache_stats: LookupCacheStats,

    /// `envWarden :: Warden` (:275).
    pub env_warden: Warden,

    /// `envDatabaseJanitor :: TVar (Maybe (UTCTime, JanitorRunResult))`
    /// (:276). `Arc` because Server.hs:136-141/:158-166 read the tuple and
    /// match on it after releasing the TVar.
    pub env_database_janitor: Mutex<Option<(SystemTime, Arc<JanitorRunResult>)>>,

    /// Rust-only companion (row 95's STM-retry image): the Condvar the
    /// server waits on for a new janitor result. Not a source field.
    pub env_database_janitor_changed: Condvar,

    /// `envDatabaseJanitorPublishedCounters :: TVar (HashSet ByteString)`
    /// (:277).
    pub env_database_janitor_published_counters: Mutex<HashSet<Vec<u8>>>,

    /// `envCachedRestorableDBs :: TVar (Maybe (UTCTime, [(Thrift.Repo,
    /// Meta)]))` (:278). SUSPECTED cause of `Env`'s `!Send`: the snapshot
    /// needs `Meta: Clone`, and `Meta` lives in the unshipped
    /// `crate::internal::types` (see the TODO above).
    pub env_cached_restorable_dbs: Mutex<Option<(SystemTime, Vec<(thrift::Repo, Meta)>)>>,

    /// `envCachedAvailableDBs :: TVar (HashSet Thrift.Repo)` (:279).
    pub env_cached_available_dbs: Mutex<HashSet<thrift::Repo>>,

    /// `envWrites :: TVar (HashMap Text Write)` (:280). `Arc<Write>`
    /// because the map is read and the `write_wait` slot used after the
    /// transaction (Writes.hs:334-342, :394-412).
    pub env_writes: Mutex<HashMap<String, Arc<Write>>>,

    /// `envDerivations :: TVar (HashMap (Thrift.Repo, PredicateId)
    /// Derivation)` (:281). `Arc<Derivation>` because the record is read,
    /// used across IO, then re-locked (Derive.hs:494-503, :560-566).
    pub env_derivations: Mutex<HashMap<(thrift::Repo, PredicateId), Arc<Derivation>>>,

    /// `envWriteQueues :: WriteQueues` (:282).
    pub env_write_queues: WriteQueues,

    /// `envListener :: Listener` (:283) — testing support only.
    pub env_listener: Listener,

    /// `envGetCurrentTime :: IO UTCTime` (:286) — yield the current time;
    /// normally `getCurrentTime`, replaceable for testing. `UTCTime` ->
    /// `SystemTime` (RULE §1 B3). Arc because Env is shared.
    pub env_get_current_time: Arc<dyn Fn() -> SystemTime + Send + Sync>,

    /// `envShardManager :: SomeShardManager` (:289).
    pub env_shard_manager: SomeShardManager,

    /// `envEnableRecursion :: EnableRecursion` (:290).
    pub env_enable_recursion: EnableRecursion,

    /// `envFilterAvailableDBs :: [Thrift.Repo] -> IO [Thrift.Repo]`
    /// (:292) — filter out DBs not currently available in the server tier.
    /// The `ConfigError` image matches config.rs's
    /// `cfg_filter_available_dbs` exactly (Env.hs:152 copies the field).
    pub env_filter_available_dbs:
        Arc<dyn Fn(&[thrift::Repo]) -> Result<Vec<thrift::Repo>, ConfigError> + Send + Sync>,

    /// `envResolveAclGroups :: [Text] -> IO [Text]` (:294) — given the
    /// candidate ACL group names of the DB layer being queried, resolve
    /// the subset the current request's caller is a member of.
    pub env_resolve_acl_groups:
        Arc<dyn Fn(&[String]) -> Result<Vec<String>, ConfigError> + Send + Sync>,

    /// `envAclCalculateEnabled :: IO Bool` (:297) — whether completion
    /// should calculate ACL ownership.
    pub env_acl_calculate_enabled: Arc<dyn Fn() -> Result<bool, ConfigError> + Send + Sync>,

    /// `envAclCheckEnabled :: IO Bool` (:299) — whether queries should
    /// filter facts by ACL ownership.
    pub env_acl_check_enabled: Arc<dyn Fn() -> Result<bool, ConfigError> + Send + Sync>,

    /// `envTracer :: Tracer GleanTrace` (:301) — config.rs's Tracer sketch
    /// (Control.Trace has no manifest unit).
    pub env_tracer: Tracer<GleanTrace>,

    /// `envDebug :: DebugFlags` (:302) — re-exported from Config (:17).
    pub env_debug: DebugFlags,
}

// ---------------------------------------------------------------------------
// Storage dispatchers (:305-335)
// ---------------------------------------------------------------------------

/// 'withStorageFor' (:305-318):
///
/// ```haskell
/// withStorageFor :: Env -> Thrift.Repo -> Meta
///                -> (forall s. Storage s => s -> IO a) -> IO a
/// ```
///
/// The rank-2 callback is imaged by handing it the closed [`Storage`]
/// enum, which implements [`StorageTrait`] (SIBLING-UNIT CONTRACTS). The
/// generic `E` is the image of the callback's unconstrained `IO`
/// exceptions (DIVERGENCES 2).
///
/// Source behaviour: an empty `metaStorage` means the legacy RocksDB name
/// (:312-313), and an unknown storage is a `dbError repo` (:316-317).
pub fn with_storage_for<A, E, F>(env: &Env, repo: &thrift::Repo, meta: &Meta, f: F) -> Result<A, E>
where
    E: From<DbError>,
    F: FnOnce(&Storage) -> Result<A, E>,
{
    let n = &meta.meta_storage;
    let name = if n.is_empty() {
        // `Text.null (unStorageName n)` (:312): backwards compat.
        rocksdb_name()
    } else {
        n.clone()
    };
    match env.env_storage.get(&name) {
        None => db_error(repo, &format!("unknown storage: {}", name)).map_err(E::from),
        Some(storage) => f(storage),
    }
}

/// 'withDefaultStorage' (:320-335):
///
/// ```haskell
/// withDefaultStorage :: Env
///                    -> (forall s. Storage s => StorageName -> s -> IO a)
///                    -> IO a
/// ```
///
/// The name is the CLI-forced default when `snd envDefaultStorage` is True,
/// otherwise `ServerConfig.config_db_create_storage` if set, otherwise the
/// default (:325-331). An unknown storage is a `Thrift.Exception`
/// (:333-334), deliberately not the `DbError` channel of
/// `withStorageFor` (SIBLING-UNIT CONTRACTS).
pub fn with_default_storage<A, E, F>(env: &Env, f: F) -> Result<A, E>
where
    E: From<thrift::Exception>,
    F: FnOnce(&StorageName, &Storage) -> Result<A, E>,
{
    let name = match &env.env_default_storage {
        // `(name, cli) | cli -> return name` (:327).
        (name, true) => name.clone(),
        // `cfg <- Observed.get (envServerConfig env)` (:329) and
        // `fromMaybe name $ fmap StorageName $ config_db_create_storage cfg`
        // (:330-331).
        (name, false) => env
            .env_server_config
            .get()
            .config_db_create_storage
            .unwrap_or_else(|| name.clone()),
    };
    match env.env_storage.get(&name) {
        None => Err(E::from(thrift::Exception {
            message: format!("unknown storage: {}", name),
        })),
        Some(storage) => f(&name, storage),
    }
}

// ---------------------------------------------------------------------------
// Rust-only slot images (RULE §2's MVar / TMVar rows)
// ---------------------------------------------------------------------------

/// `MVar a` — the A3 communication-slot image (RULE §2): a BLOCKING `put`
/// (blocks when full), a consuming `take`, and a repeated-read `read`
/// (blocking, leaves the value). The source uses `readMVar`/`tryReadMVar`
/// on `writeWait` (Writes.hs:342/:412), so `read`/`read_timeout`/`try_read`
/// take a closure and inspect the value in place: `Subst` is not `Clone`
/// and `readMVar` shares the value rather than copying it.
///
/// Haskell's `MVar` is a SHARED MUTABLE REFERENCE, not an owned slot: one
/// `newEmptyMVar` value is stored in `WriteJob.writeDone` and the SAME
/// value is stored in `Write.writeWait` (Queue.hs:91/:114/:132,
/// Writes.hs:431-436), so the writer thread and the poller name one slot.
/// This image is therefore an `Arc`-backed handle and `Clone` copies the
/// handle, not the contents. The `Clone` impl is manual because a derive
/// would wrongly require `T: Clone`, and `Subst` is not `Clone`.
///
/// Every notify is issued while the paired `Mutex` guard is held, so the
/// state change and the wakeup are one critical section (no lost wakeups).
/// `filled` is signalled with `notify_all`, not `notify_one`: both `take`
/// and `read` wait on it, and a single wakeup could land on the wrong
/// waiter.
///
/// This is NOT the mutex reading of MVar (that is `std::sync::Mutex`;
/// unit 102). `envDbSchemaCache` uses the mutex reading and is a plain
/// `Mutex`.
pub struct MVar<T>(Arc<MVarInner<T>>);

/// The shared body of [`MVar`]. Private: the handle is the API.
struct MVarInner<T> {
    state: Mutex<Option<T>>,
    /// Signalled when a value is put.
    filled: Condvar,
    /// Signalled when a value is taken.
    empty: Condvar,
}

impl<T> Clone for MVar<T> {
    fn clone(&self) -> Self {
        MVar(Arc::clone(&self.0))
    }
}

impl<T> MVar<T> {
    /// 'newEmptyMVar' — the `writeWait`/`writeDone` initialization.
    pub fn new_empty() -> MVar<T> {
        MVar(Arc::new(MVarInner {
            state: Mutex::new(None),
            filled: Condvar::new(),
            empty: Condvar::new(),
        }))
    }

    /// 'newMVar x' — a slot that starts full.
    pub fn new(value: T) -> MVar<T> {
        MVar(Arc::new(MVarInner {
            state: Mutex::new(Some(value)),
            filled: Condvar::new(),
            empty: Condvar::new(),
        }))
    }

    /// 'putMVar' — block while the slot is full, then store.
    pub fn put(&self, value: T) {
        let mut state = lock(&self.0.state);
        while state.is_some() {
            state = self
                .0
                .empty
                .wait(state)
                .unwrap_or_else(PoisonError::into_inner);
        }
        *state = Some(value);
        self.0.filled.notify_all();
    }

    /// 'tryPutMVar' — non-blocking; `true` iff the value was stored. A
    /// contended lock yields `false` (the try-lock contract: never waits).
    pub fn try_put(&self, value: T) -> bool {
        let mut state = match self.0.state.try_lock() {
            Ok(guard) => guard,
            Err(TryLockError::Poisoned(e)) => e.into_inner(),
            Err(TryLockError::WouldBlock) => return false,
        };
        if state.is_some() {
            false
        } else {
            *state = Some(value);
            self.0.filled.notify_all();
            true
        }
    }

    /// 'takeMVar' — block while empty, then consume the value.
    pub fn take(&self) -> T {
        let mut state = lock(&self.0.state);
        loop {
            if let Some(value) = state.take() {
                self.0.empty.notify_one();
                return value;
            }
            state = self
                .0
                .filled
                .wait(state)
                .unwrap_or_else(PoisonError::into_inner);
        }
    }

    /// 'readMVar' — block while empty, then inspect the value in place
    /// (the value stays in the slot).
    pub fn read<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        let mut state = lock(&self.0.state);
        loop {
            if let Some(value) = state.as_ref() {
                return f(value);
            }
            state = self
                .0
                .filled
                .wait(state)
                .unwrap_or_else(PoisonError::into_inner);
        }
    }

    /// `timeout d $ readMVar` (Writes.hs:342's 100 ms poll): `None` if no
    /// value appeared within `dur`. The slot is unchanged either way.
    pub fn read_timeout<R>(&self, dur: Duration, f: impl FnOnce(&T) -> R) -> Option<R> {
        let deadline = Instant::now() + dur;
        let mut state = lock(&self.0.state);
        loop {
            if let Some(value) = state.as_ref() {
                return Some(f(value));
            }
            let now = Instant::now();
            if now >= deadline {
                return None;
            }
            let (guard, _) = self
                .0
                .filled
                .wait_timeout(state, deadline - now)
                .unwrap_or_else(PoisonError::into_inner);
            state = guard;
        }
    }

    /// 'tryReadMVar' — non-blocking inspect. `None` if the slot is empty OR
    /// the lock is contended (a try-lock: it never waits, the
    /// `tryReadMVar` contract). The two `None` cases are deliberately
    /// indistinguishable, as in the source's retry-on-nothing poll.
    pub fn try_read<R>(&self, f: impl FnOnce(&T) -> R) -> Option<R> {
        let state = match self.0.state.try_lock() {
            Ok(guard) => guard,
            Err(TryLockError::Poisoned(e)) => e.into_inner(),
            Err(TryLockError::WouldBlock) => return None,
        };
        state.as_ref().map(f)
    }
}

/// `TMVar a` — the RULE §2 image: `Mutex<Option<a>>` + a Condvar latch,
/// preserving the full/empty state and `tryPut`'s Bool. The only site in
/// this file is `envSchemaUpdateSignal :: TMVar ()` (:257).
///
/// One Condvar serves both the "wait for empty" (`put`) and "wait for
/// full" (`take`) sides, so every transition uses `notify_all`: waking a
/// waiter on the wrong side with `notify_one` could leave the right waiter
/// asleep. Every notify is issued while the paired `Mutex` guard is held.
pub struct TMVar<T> {
    state: Mutex<Option<T>>,
    changed: Condvar,
}

impl<T> TMVar<T> {
    /// 'newEmptyTMVarIO' (Env.hs:138).
    pub fn new_empty() -> TMVar<T> {
        TMVar {
            state: Mutex::new(None),
            changed: Condvar::new(),
        }
    }

    /// 'newTMVarIO x'.
    pub fn new(value: T) -> TMVar<T> {
        TMVar {
            state: Mutex::new(Some(value)),
            changed: Condvar::new(),
        }
    }

    /// 'putTMVar' — block while full, then store.
    pub fn put(&self, value: T) {
        let mut state = lock(&self.state);
        while state.is_some() {
            state = self
                .changed
                .wait(state)
                .unwrap_or_else(PoisonError::into_inner);
        }
        *state = Some(value);
        self.changed.notify_all();
    }

    /// 'tryPutTMVar' — non-blocking; `true` iff the value was stored
    /// (Env.hs:241's signal).
    pub fn try_put(&self, value: T) -> bool {
        let mut state = lock(&self.state);
        if state.is_some() {
            false
        } else {
            *state = Some(value);
            self.changed.notify_all();
            true
        }
    }

    /// 'takeTMVar' — block while empty, then consume (Env.hs:238).
    pub fn take(&self) -> T {
        let mut state = lock(&self.state);
        loop {
            if let Some(value) = state.take() {
                self.changed.notify_all();
                return value;
            }
            state = self
                .changed
                .wait(state)
                .unwrap_or_else(PoisonError::into_inner);
        }
    }

    /// 'tryTakeTMVar' — non-blocking consume.
    pub fn try_take(&self) -> Option<T> {
        let mut state = lock(&self.state);
        let value = state.take();
        if value.is_some() {
            self.changed.notify_all();
        }
        value
    }

    /// 'isEmptyTMVar'.
    pub fn is_empty(&self) -> bool {
        lock(&self.state).is_none()
    }
}

/// The crate's poison policy (catalog.rs:252-254, with the one `fn lock`
/// at catalog.rs:706-708): a poisoned lock still exposes its data, and
/// every consumer of these slots treats poisoning as unreachable in a
/// no-panic pipeline. Recover rather than propagate.
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// `Data.RateLimiterMap` — ASSUMED-IMPORT (RULE §4 B4). See the header's
/// ASSUMED-IMPORTS block for the pinned Haskell shape.
///
/// TODO(port): this is an opaque placeholder that fixes only the `Env`
/// field type (`envLoggerRateLimit`, :249). The functional port — the
/// moving-average limiter table, `newRateLimiterMap` and `whenAllowed` —
/// is owed by two owners, one per half: the CONSTRUCTOR by `Env`'s
/// `newRateLimiterMap` (Env.hs:129-130; manifest row 132,
/// src/database/env.rs), and the first METHOD consumer `whenAllowed` by
/// Logger.hs:39 (manifest row 106, src/logger.rs). The Step-4 kit may take
/// both. `Data.RateLimiterMap` and `Data.MovingAverageRateLimiter` have no
/// manifest unit. The placeholder is `Send + Sync` for any `Send + Sync`
/// K, so it does not add a second cause to Env's Send problem.
pub struct RateLimiterMap<K> {
    _marker: std::marker::PhantomData<K>,
}

// PORT STATUS: confidence=medium todos=2
