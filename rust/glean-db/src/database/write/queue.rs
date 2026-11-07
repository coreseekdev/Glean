// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// Ported from: glean/db/Glean/Database/Write/Queue.hs (211 lines).
// Manifest unit: rust/glean-db/src/database/write/queue.rs
// (migration/manifest.tsv line 105, crate glean-db). RULE §4: one source
// file -> one target file; the manifest path is authoritative.
//
// WHAT THIS FILE IS: the PRODUCER half of the server-side write pipeline —
// the five exports of Queue.hs (`enqueueWrite`, the single funnel every
// enqueue path goes through; `enqueueBatchDescriptorWithResultHandling`, the
// async descriptor-write wrapper that marks the DB broken on failure;
// `addWriteJobToQueue`, the push primitive with the round-robin registration;
// `reportQueueSizes` and `writeContentFromBatch`, the stats/record helpers)
// plus the private `rejectWrite` / `checkMemoryAvailable` /
// `downloadBatchFromLocation`. The CONSUMER half is Writes.hs's writerThread/
// dequeueLoop (manifest row 116, src/database/writes.rs) and the completion
// call sites Batch.hs (row 112) / Finish.hs (row 133) / Close.hs (row 121).
//
// ---------------------------------------------------------------------------
// INVENTORY (RULE §5; migration/inventory.tsv row 35 — the governing row)
// ---------------------------------------------------------------------------
//
//   $ sed -n '35p' migration/inventory.tsv
//   glean/db/Glean/Database/Write/Queue.hs   WriteQueue   STM queue + batch
//   coalescing   STM_GROUP   mpsc channel + Mutex<BatchState>; commit
//   boundary preserves wrNextId monotonicity   Queue.hs:186-196 (Types.hs);
//   round-robin per-repo; flush-on-finish ordering preserved (Write.hs finish
//   waits)   confirmed
//
// Disposition, clause by clause:
//
//   * "mpsc channel + Mutex<BatchState>" — NOT FOLLOWED.
//     INVENTORY-BUG-CANDIDATE row 35: RULEBOOK §2's STM row mandates one
//     `Mutex<State>` per multi-variable transaction, and every site that
//     touches this queue reads the queue TOGETHER with its counters in one
//     `atomically` — enqueue (Queue.hs:100-120), the writer's done block
//     (Writes.hs:128-137), dequeueLoop (Writes.hs:165-190) and finishDatabase
//     (Finish.hs:65-84, active + emptiness). An mpsc channel cannot compose
//     with those counter reads. Row-35 evidence-string cite error, for the
//     record (fixable in inventory.tsv at the next amendment batch): the
//     row's evidence "Queue.hs:186-196 (Types.hs)" actually points at
//     writeContentFromBatch here (Queue.hs:186-190); the WriteQueue type it
//     means is Types.hs:186-196. The shipped definition (types.rs, unit 104)
//     already carries this flag at `WriteQueueState`; this unit consumes
//     those shapes unchanged: `Mutex<WriteQueueState>` +
//     `Mutex<WriteQueuesState>`, each with its Condvar beside it.
//   * "commit boundary preserves wrNextId monotonicity" — owned by row 34's
//     `WriteState` (Batch.hs:237-249: wrCommit CAS + wrNextId bump); Queue.hs
//     never touches wrNextId, and neither does this file.
//   * "round-robin per-repo" — preserved below: a queue is registered in the
//     round-robin deque exactly when it transitions empty -> non-empty
//     (addWriteJobToQueue, Queue.hs:143-150), and the writer re-registers
//     only while still non-empty (Writes.hs:177-180).
//   * "flush-on-finish ordering preserved" — preserved by the ONE
//     WRITE_QUEUE_STATE lock: finishDatabase reads `writeQueueActive` and the
//     queue's emptiness in one transaction (Finish.hs:73-80); with the
//     counters under the same lock as the queue, the port's finish (row 133)
//     reads both atomically.
//   * "Write.hs finish waits" — the row's evidence names Write.hs; the
//     finish path lives in glean/db/Glean/Database/Finish.hs (there is no
//     Write.hs under db/Glean/Database/), cited here as Finish.hs:65-84.
//
// ---------------------------------------------------------------------------
// TRANSACTION MAP (the atomicity the call sites dictate — read before
// touching the lock structure)
// ---------------------------------------------------------------------------
//
// Queue.hs's `immediately` (Util/Defer.hs:41-45: `runStateT` under ONE
// `atomically`; every `now` composes into the same transaction, every
// `later` is deferred IO run after commit) makes the WHOLE enqueue path one
// atomic block:
//
//   readTVar envShuttingDown                       Queue.hs:123
//   checkMemoryAvailable (reserve writeQueuesSize) Queue.hs:127/:158-171
//   enqueueIt: re-read writeQueuesSize             Queue.hs:104-106
//              addWriteJobToQueue (push + register) Queue.hs:106-116/:141-150
//              updateTVar writeQueueCount (+1)      Queue.hs:117
//              updateTVar writeQueueSize (+ size)   Queue.hs:118
//   reject:    readTVar writeQueueLatency          Queue.hs:130
//
// RULEBOOK §2's STM row images this as ONE critical section. The guards
// held, in acquisition order: `env_shutting_down` -> WRITE_QUEUES
// (`WriteQueues.write_queues_state`) -> WRITE_QUEUE_STATE
// (`WriteQueue.write_queue_state`) — the types.rs TOTAL LOCK ORDER's
// WRITE_QUEUES -> WRITE_QUEUE_STATE suffix, with the shutdown flag taken
// FIRST (the source's first read, Queue.hs:123). No cycle is possible: the
// only other takers of `env_shutting_down` are the shutdown setter
// (Server/Sharding.hs:237 — a solo `writeTVar ... True`), the writer's
// dequeueLoop (Writes.hs:165-190, which reads the flag BEFORE taking
// WRITE_QUEUES — the same prefix) and the shard-updater poll handed to
// withShardsUpdater (Server.hs:117 — a solo `readTVar`, the "is the server
// gracefully shutting down?" STM Bool consumed by dbUpdateNotifierThread;
// Sharding.hs:231-236 names it as the reader of this same flag — no nested
// locks, no cycle), and no queue-lock holder ever waits for
// `env_shutting_down`.
//
// The deferred blocks (`later`) run AFTER the critical section, OUTSIDE the
// locks (RULE §2 B1: callbacks after the state mutation, outside the lock):
// the enqueue stats (Queue.hs:119-121) and the reject stats + throw
// (Queue.hs:130-132/:148-155).
//
// Other call sites, for the record (their ports own them):
//   * Writes.hs:128-137 `done` — the four counters AND writeQueuesSize in
//     one transaction; stats/`active` updated BEFORE `try_put(write_done)`
//     (the source's own race note, Writes.hs:134-136).
//   * Writes.hs:165-190 dequeueLoop — round-robin + checkpoint protocol,
//     parks on the shutdown flag via STM `retry`.
//   * Finish.hs:65-84 — see the flush-on-finish clause above.
//   * Close.hs:44-49/:82-87 — eviction/`deleteWriteQueues` take the DB lock
//     first (the documented DB -> WRITE_QUEUE_STATE edge). NOTE:
//     `deleteWriteQueues` ALSO mutates the global writeQueuesSize
//     (Writes.hs:441-449 — `modifyTVar' (writeQueuesSize (envWriteQueues
//     env)) (subtract size)`), so the close path exercises the DB ->
//     WRITE_QUEUES edge too; types.rs's total order (DB -> WRITE_QUEUES ->
//     WRITE_QUEUE_STATE) subsumes both edges (doc precision for row 121's
//     port).
//
// ---------------------------------------------------------------------------
// DEPENDENCY TRUTH (source import line -> shipped Rust path)
// ---------------------------------------------------------------------------
//
//   :18 Control.Concurrent (MVar, newEmptyMVar)
//                                       -> crate::database::types::MVar
//                                          (the A3 slot image; `new_empty`)
//   :19 Control.Exception (SomeException, throwIO)
//                                       -> Result + the typed
//                                          `EnqueueWriteError` below (RULE §2
//                                          dynamic-exceptions row); the
//                                          materializer's throwIO image is
//                                          `panic_any` (see the marker at
//                                          `enqueue_batch_descriptor_with_
//                                          result_handling`)
//   :21-23 Data.Map / Data.Text / Data.Text.Encoding
//                                       -> BTreeMap (DbSchema.schema_envs) /
//                                          String / byte strings (RULE §1
//                                          strings row: Vec<u8> for
//                                          ByteString)
//   :24 Text.Printf                     -> format!
//   :25 System.Clock (TimeSpec, toNanoSecs)
//                                       -> std::time::Duration (RULE §1 B3
//                                          time row; types.rs pinned the
//                                          WriteQueueState.latency image)
//   :27-28 ServiceData.GlobalStats / ServiceData.Types (ExportType)
//                                       -> glean_fb303::service_data::
//                                          global_stats::add_stat_value_type /
//                                          glean_fb303::service_data::types::
//                                          ExportType (the seven no-op
//                                          signatures; global_stats.rs cites
//                                          Queue.hs:120,152-184 as consumer
//                                          evidence)
//   :29 Util.Defer (immediately, now, later)
//                                       -> dissolves: one critical section
//                                          for the `now`s, deferred block
//                                          after it for the `later`s (the
//                                          TRANSACTION MAP above). External
//                                          hsthrift package, no manifest
//                                          unit; pinned semantics at
//                                          ~/glean-build/hsthrift/common/
//                                          util/Util/Defer.hs:26-45.
//   :30 Util.STM                        -> std::sync Mutex/Condvar images
//                                          (RULEBOOK §2 STM row); the pinned
//                                          `updateTVar` (Util/STM.hs:25-30)
//                                          is the read-modify-write on the
//                                          guard's field
//   :32 Glean.Database.Catalog          -> crate::database::catalog::Catalog
//                                          ::modify_meta (catalog.rs:1565;
//                                          the source's pure callback
//                                          `\meta -> pure meta{...}` matches
//                                          its `Fn(Meta) -> Meta` exactly)
//   :33 Glean.Database.BatchLocation    -> crate::database::batch_location::
//                                          {Parser, SomeParser}
//   :34 Glean.Database.Exception (dbError)
//                                       -> crate::database::exception::
//                                          db_exception (the DBError
//                                          constructor image; exception.rs's
//                                          single-funnel contract)
//   :35 Glean.Database.Storage          -> crate::database::storage::
//                                          DatabaseOps::
//                                          mark_batch_descriptor_as_written
//                                          (dispatched on SomeDatabaseOps,
//                                          types.rs:895-901)
//   :36 Glean.Database.Types            -> crate::database::types (Env,
//                                          WriteQueue, WriteQueues, WriteJob,
//                                          WriteContent, MVar)
//   :37 Glean.Database.Schema.Types     -> crate::database::schema::types::
//                                          DbSchema (schema_envs:
//                                          BTreeMap<SchemaId, _>)
//   :38 Glean.Internal.Types (Thrift.BatchDescriptor)
//                                       -> glean_thrift (ASSUMED-IMPORT)
//   :39 Glean.Types (Repo)              -> glean_thrift (ASSUMED-IMPORT;
//                                          backend/types.rs:171-175 pinned
//                                          `Repo { name, hash }`)
//   :40 Glean.RTS.Foreign.Subst         -> glean_core::rts::foreign::subst::
//                                          Subst
//   :41 Glean.ServerConfig.Types        -> glean_thrift::Config (the
//                                          `ServerConfig` alias, types.rs:654)
//   :42 Glean.Util.Observed             -> glean_util::util::observed::
//                                          Observed (`get` returns A by value)
//   :43 Glean.Util.Metric (beginTick)   -> glean_util::util::metric::
//                                          begin_tick (Point; :328)
//   :44 Glean.Util.Some                 -> crate::database::types::
//                                          SomeDatabaseOps (the closed-enum
//                                          image, row 136/137)
//
// ---------------------------------------------------------------------------
// ASSUMED-IMPORTS (RULE §4 B4) — generated thrift shapes pinned to the IDL
// ---------------------------------------------------------------------------
//
//   * `glean_thrift::Retry { seconds: f64 }` — glean.thrift:239-241
//     (`exception Retry { 1: double seconds; }`); the Haskell generated
//     record's field is `retry_seconds` (Writes.hs:350/:355 pattern-match
//     evidence), the Rust field is the bare IDL name.
//   * `glean_thrift::BatchDescriptor { location: String, format:
//     BatchFormat }` — glean.thrift:915-918 (`BatchDescriptor_location`/
//     `BatchDescriptor_format` at Queue.hs:61/:198-199).
//   * `glean_thrift::BatchFormat` — glean.thrift:909; Copy, the fbthrift
//     generated-enum convention (batch_location.rs:144's trait already takes
//     it by value).
//   * `glean_thrift::DatabaseBroken { task, reason }` — pinned by meta.rs:615
//     (`Completeness::Broken(thrift::DatabaseBroken { task, reason })`);
//     Queue.hs:66-67 passes `"write"` and `show exc` positionally, i.e.
//     task = "write", reason = the shown exception.
//   * `crate::internal::types::Completeness` — the generated
//     Meta/Completeness home (meta.rs:522 precedent; the module is row 142's
//     generated `StoredSchema`/Meta surface, not a hand unit).
//   * `glean_thrift::Config`'s `config_db_write_queue_limit_mb: i32`
//     (server_config.thrift:240) and `config_check_write_schema_id: bool`
//     (:318) — the generated field keeps the `config_` prefix
//     (types.rs:268's pin).
//   * `glean_thrift::Exception { message: String }` — glean.thrift:225-227
//     (one field, `1: string message`). This file's uses lean on the
//     generated image carrying Display (the derived-Show image),
//     std::error::Error and Send + Sync + 'static: `download_batch_from_
//     location`'s `?` propagates it as that fn's error type, and the
//     materializer's `panic_any` arm boxes it into
//     `Box<dyn Error + Send + Sync>` as the panic payload. Display + Error
//     are batch_location.rs:66-69's "Step-4 generator obligations" pin (the
//     first sketch); the Send + Sync + 'static leg is this unit's extension
//     of that sketch, recorded here per RULE §4 B4 (consumer records the
//     extension, cross-references the first sketch).
//
// ---------------------------------------------------------------------------
// SIBLING-UNIT CONTRACTS (consumed, never re-declared)
// ---------------------------------------------------------------------------
//
//   * crate::database::types (unit 104, SHIPPED): `WriteQueue` /
//     `WriteQueueState` / `WriteQueues` / `WriteQueuesState` / `WriteJob` /
//     `WriteContent` / `MVar` / `Env` are USED here, not re-defined. The
//     lock order is that file's TOTAL LOCK ORDER
//     (ENV_ACTIVE -> DB -> WRITING -> WRITE_QUEUES -> WRITE_QUEUE_STATE);
//     this file takes the WRITE_QUEUES -> WRITE_QUEUE_STATE suffix (with the
//     `env_shutting_down` prefix — TRANSACTION MAP).
//   * crate::database::exception (row 53, SHIPPED): the schema-check throw
//     funnels through `db_exception` (single-funnel contract).
//   * crate::database::catalog (row 54, SHIPPED): `modify_meta`'s infallible
//     `Fn(Meta) -> Meta` callback is exactly the source's pure `\meta ->
//     pure meta{...}` (Queue.hs:64-68); the source's `void` discards only
//     the returned Meta value — a FAILED transaction (the `Result`'s Err
//     leg, CatalogError) is NOT swallowed here: it is logged and re-thrown
//     at the `on_complete` closure, mirroring Writes.hs:126-149.
//   * glean_fb303 service_data (SHIPPED): the stats calls compile against
//     the seven no-op signatures (global_stats.rs's row-105 contract note).
//   * crate::write (src/write.rs, row 46 area) is the CLIENT-side
//     SendBatch/SendQueue layer — a different pipeline; nothing here.
//
// ---------------------------------------------------------------------------
// DIVERGENCES (Rust-only images; no inventory row governs them)
// ---------------------------------------------------------------------------
//
//   1. `EnqueueWriteError` — Rust-only enum carrying the two exception types
//      that escape `enqueueWrite`'s frame (RULE §2 dynamic-exceptions row:
//      typed error per frame): the schema-check `DbError` (Queue.hs:94-99)
//      and `rejectWrite`'s thrift `Retry` (Queue.hs:155). Haskell's caller
//      catches ONLY the Retry leg (`try` specialized at Writes.hs:231-242);
//      the DBError leg propagates to the thrift boundary.
//   2. `is_async` — the source's parameter name `async` is a Rust keyword.
//      Naming-only image.
//   3. The materializer's `throwIO` image is `panic_any` carrying the boxed
//      error (see the marker at `enqueue_batch_descriptor_with_result_
//      handling`) — interim pending the WriteJob field-image decision. The
//      same image carries the writeOnComplete throw (Writes.hs:126) in the
//      `on_complete` closure (see its BUG reproduction).
//   4. `stats_name` / `quote_text` / `round_half_to_even` — private helpers
//      imaging ByteString `<>`, `show :: [Text] -> String` and Haskell's
//      banker's `round`; none is a source export.
//
// WIRING — RESOLVED. The two `pub mod` lines this unit owed at ship time by
// the database.rs convention (rows 49/56/58 precedent; HANDOFF.md §2.4's
// direct-child wiring rule mandates them with the shipped file, the row-56
// Backup/ shape) have landed: `pub mod write;` is declared in
// rust/glean-db/src/database.rs (binding the directory module) and
// rust/glean-db/src/database/write.rs exists declaring `pub mod queue;` (the
// RULE §4 directory module for the Write/ subdirectory, mirroring Backup/ ->
// backup.rs + backup/backend.rs, row 56). `crate::database::write::queue`
// resolves; siblings may import it.
//
// ---------------------------------------------------------------------------
// MARKERS: two port todos (the WriteJob-construction Send/ownership decision,
// inherited from types.rs's Env TODO; the materializer failure channel),
// one inventory-bug candidate (row 35, first flagged at the types.rs
// definition), two source-defect reproductions (the enqueue-path transient
// over-report at `enqueue_write` and the onComplete exception-path leak at
// `enqueue_batch_descriptor_with_result_handling` — both reproduced
// bug-for-bug under the BUG rule, fix only as flagged post-parity), zero
// performance deferrals, zero SAFETY notes (no unsafe in this file). The
// TODO sites are marked once each in the body; the tally above avoids the
// literal marker spellings.
// ---------------------------------------------------------------------------

use std::error::Error;
use std::fmt;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use glean_core::rts::foreign::subst::Subst;
use glean_fb303::service_data::global_stats::add_stat_value_type;
use glean_fb303::service_data::types::ExportType;
use glean_thrift as thrift;
use glean_thrift::Config as ServerConfig;
// The writer's handler log channel (Writes.hs:149 `logError (show e)`;
// Util.Log -> the pinned `glean_util::log` seam, the data.rs:75-79 sketch).
// Queue.hs itself does not import Util.Log — the calls inside `on_complete`
// front-load the handler's log to the throw site, because the catch that
// logs in the source (the writer's tryAll, Writes.hs:141-149) is row 116's
// file.
use glean_util::log::log_error;
use glean_util::util::metric::{begin_tick, Point};

use crate::database::batch_location::Parser;
use crate::database::exception::{db_exception, DbError};
use crate::database::schema::types::DbSchema;
use crate::database::storage::DatabaseOps;
use crate::database::types::{
    Env, MVar, SomeDatabaseOps, WriteContent, WriteJob, WriteQueue, WriteQueueState, WriteQueues,
    WriteQueuesState,
};
use crate::internal::types::Completeness;

/// `k = 1024` (:204-205). `Int` -> `i64` (LP64 standing fact, types.rs
/// TRANSLATION NOTES 4).
const K: i64 = 1024;

/// `mb = 1024*1024` (:207-208).
const MB: i64 = 1024 * 1024;

/// `roundUp :: Int -> Int -> Int` (:210-211) — `(x + unit-1) `div` unit`.
/// `div` is floor division; `div_euclid` is its exact image for the positive
/// `unit` used here.
fn round_up(unit: i64, x: i64) -> i64 {
    (x + unit - 1).div_euclid(unit)
}

/// The crate's poison policy (types.rs:2038-2040, citing catalog.rs:252-254):
/// a poisoned lock still exposes its data; recover rather than propagate.
fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Haskell `round` (RealFrac): round-half-to-even. Rust's `f64::round` rounds
/// halves AWAY from zero, so exact halves are adjusted to the even neighbour
/// (2.5 -> 2, 3.5 -> 4, as `round` does). DIVERGENCES 4.
fn round_half_to_even(x: f64) -> i64 {
    let r = x.round();
    if (x - x.trunc()).abs() == 0.5 && r % 2.0 != 0.0 {
        (r - x.signum()) as i64
    } else {
        r as i64
    }
}

/// ByteString `<>` on the stat-name prefixes (RULE §1 strings row: Vec<u8>
/// for ByteString; the source builds `"glean.db.write.queue_count." <>
/// Text.encodeUtf8 (repo_name repo)`, Queue.hs:176).
fn stats_name(prefix: &[u8], repo_b: &[u8]) -> Vec<u8> {
    let mut name = prefix.to_vec();
    name.extend_from_slice(repo_b);
    name
}

/// GHC's `show` for a LIST of Text (Queue.hs:99): elements quoted,
/// comma-separated, no spaces (`["a","b"]`). Data.Text's Show escapes per
/// Haskell string-literal rules; the backslash and quote escapes are
/// reproduced single-pass. Control-character escapes (\n, \t, ...) are not
/// reproduced: schema IDs are hash strings (ASSUMED-IMPORT: `typedef string
/// SchemaId`), so no non-printable ever appears — extend this helper if one
/// does. DIVERGENCES 4.
fn quote_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

// ---------------------------------------------------------------------------
// The two exception channels of `enqueueWrite` (DIVERGENCES 1)
// ---------------------------------------------------------------------------

/// The exceptions that escape [`enqueue_write`]'s frame:
///
///   * `Db` — the schema-ID check's `dbError repo` (Queue.hs:94-99), a
///     `DBError` built by [`db_exception`] (the exception.rs single-funnel
///     image).
///   * `Retry` — `rejectWrite`'s `throwIO (Retry clamped)` (Queue.hs:155),
///     the thrift `Retry` exception (glean.thrift:239-241). The source's
///     caller catches ONLY this leg: Writes.hs:231-242 `try`-matches
///     `Left (Retry n) -> SendResponse_retry (BatchRetry n)`; the `Db` leg
///     propagates to the write API's declared exceptions.
pub enum EnqueueWriteError {
    /// The `DBError` channel (Queue.hs:94-99).
    Db(DbError),
    /// The `Retry` channel (Queue.hs:148-155).
    Retry(thrift::Retry),
}

impl fmt::Display for EnqueueWriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // `show (DBError s) = s` (Exception.hs:27-29): the DbError
            // Display already renders the bare message.
            EnqueueWriteError::Db(e) => fmt::Display::fmt(e, f),
            // The generated record's derived-Show shape
            // (`Retry {retry_seconds = ...}`); `{:?}` renders 1.0 as "1.0"
            // like GHC's Double show. Nothing in-tree shows a Retry value,
            // so this is the convention-only image.
            EnqueueWriteError::Retry(r) => write!(f, "Retry {{retry_seconds = {:?}}}", r.seconds),
        }
    }
}

impl fmt::Debug for EnqueueWriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl Error for EnqueueWriteError {}

// ---------------------------------------------------------------------------
// enqueueBatchDescriptorWithResultHandling (:50-72)
// ---------------------------------------------------------------------------

/// 'Enqueue a batch descriptor with result handling for asynchornous writes
/// only / Mark the DB as broken on failure / Mark the batch descriptor as
/// written on success' (source doc comment, :46-49 — typo verbatim).
///
/// `onComplete` (:62-70) runs on the WRITER thread when the job completes
/// (Writes.hs:126): on failure, when `async`, flip the DB's completeness to
/// `Broken`; on success, mark the batch descriptor as written in storage.
/// The two borrows it needs (`envCatalog`, `odbHandle`) are shared from the
/// caller's `withWritableDatabase` bracket (Open.hs:205-212) — see the
/// construction marker at [`enqueue_write`].
pub fn enqueue_batch_descriptor_with_result_handling(
    env: &Env,
    repo: &thrift::Repo,
    queue: &Arc<WriteQueue>,
    odb_schema: &DbSchema,
    odb_handle: &SomeDatabaseOps,
    descriptor: &thrift::BatchDescriptor,
    // `async` (:57) — a Rust keyword, hence `is_async` (DIVERGENCES 2).
    is_async: bool,
) -> Result<MVar<Result<Subst, Box<dyn Error + Send + Sync>>>, EnqueueWriteError> {
    // `onComplete` (:62-70). The result type is the WriteJob completion
    // image: `Either SomeException Subst` -> `Result<Subst, Box<dyn Error +
    // Send + Sync>>` (types.rs TYPE-MAPPING TRUTH).
    let on_complete = {
        // `location = Thrift.batchDescriptor_location descriptor` (:61);
        // the closure owns its copy (the source shares the binding).
        let location = descriptor.location.clone();
        move |result: Result<Subst, Box<dyn Error + Send + Sync>>| {
            if is_async {
                match result {
                    Err(exc) => {
                        // :63-68 — `void $ atomically $ Catalog.modifyMeta
                        // envCatalog repo $ \meta -> pure meta {
                        // metaCompleteness = Broken (DatabaseBroken "write"
                        // (Text.pack (show exc))) }`. `show exc` -> Display
                        // (the pinned SomeException image, exception.rs
                        // FIDELITY). The `void` discards the RESULT value,
                        // but the transaction itself can still FAIL (the
                        // getEntry race with a concurrent repo deletion,
                        // Catalog.hs:456-464): the source's `atomically`
                        // THROWS out of writeOnComplete (Writes.hs:126)
                        // through tryBracket into the writer's tryAll/
                        // logError (Writes.hs:141-149) — it is NOT swallowed.
                        if let Err(err) = env.env_catalog.modify_meta(repo, |mut meta| {
                            meta.meta_completeness = Completeness::Broken(thrift::DatabaseBroken {
                                task: "write".to_string(),
                                reason: format!("{}", exc),
                            });
                            meta
                        }) {
                            // The failure log the writer's handler emits for
                            // any exception crossing tryBracket (Writes.hs:
                            // 149), on the writer's own channel; then
                            // re-throw via the file's throwIO image so the
                            // source's throw path (and its wart) reproduces
                            // below.
                            log_error(|| format!("{}", err));
                            std::panic::panic_any(Box::new(err) as Box<dyn Error + Send + Sync>);
                        }
                    }
                    Ok(_) => {
                        // :70 — `Storage.markBatchDescriptorAsWritten
                        // odbHandle location`. The source's `IO ()` declares
                        // no failure channel but IO still THROWS (storage
                        // I/O, Storage.hs:206-207), and the throw takes the
                        // same path as the modifyMeta failure above:
                        // Writes.hs:126 -> tryBracket -> Writes.hs:141-149.
                        if let Err(err) = odb_handle.mark_batch_descriptor_as_written(&location) {
                            log_error(|| format!("{}", err));
                            std::panic::panic_any(Box::new(err) as Box<dyn Error + Send + Sync>);
                        }
                    }
                }
            }
        }
    };

    // BUG(port): source defect, reproduced bug-for-bug — the throw path
    // above LEAKS the queue counters and never completes the wait slot. In
    // the source the exception leaves `done` at Writes.hs:126 BEFORE the
    // counter decrement (the `immediately` block, Writes.hs:127-133) and
    // BEFORE `tryPutMVar writeDone` (Writes.hs:137), so
    // writeQueueActive/writeQueueCount/writeQueueSize/writeQueuesSize stay
    // elevated forever and the job's poll waits on an MVar that is never
    // put; the exception is caught only by the writer's tryAll (Writes.hs:
    // 141-149), which logs it and moves on to the next job. The `panic_any`
    // sites above leave the closure exactly where the source's throwIO
    // leaves writeOnComplete, so the faithful writer done-block (row 116) —
    // call write_on_complete FIRST, before the counters and the slot, with
    // its tryAll OUTSIDE tryBracket (Writes.hs:111-116/:140-150) — makes the
    // skip fall out as in the source. Fix only as flagged post-parity.

    // :71-72 — `enqueueWrite env repo queue odbSchema 0 Nothing False
    // onComplete $ downloadBatchFromLocation env descriptor` (size 0, no
    // schema ID, no queue-size check).
    let write_content = {
        // downloadBatchFromLocation (:192-202) is DEFERRED: the download
        // happens on the writer thread, inside writeDatabase
        // (Batch.hs:135-138 materializes after readDatabase confirmed the DB
        // is open and writable). Its parse error must reach the job's `Left`
        // result via writerThread's tryBracket (`done` sees `Left ex`,
        // Writes.hs:111-137; Util/Control/Exception.hs:84-96).
        move || match download_batch_from_location(env, descriptor) {
            Ok(content) => content,
            // TODO(port): materializer failure channel. `writeContentIO ::
            // IO WriteContent` THROWS (the parse error must reach the job's
            // `Left` result: Batch.hs:135-138 materializes inside
            // writeDatabase; writerThread's tryBracket hands the exception
            // to `done` as `Left`, Writes.hs:111-137; Util/Control/
            // Exception.hs:84-96), but the pinned `WriteJob.write_content_io`
            // image (types.rs, unit 104) is infallible: `Box<dyn FnOnce() ->
            // WriteContent + Send>`. Interim image: `throwIO` = `panic_any`
            // carrying the boxed error as the payload; the writer's tryAll
            // image (catch_unwind + payload downcast to `Box<dyn Error +
            // Send + Sync>`) rebuilds the job's `Left`. Settle the field
            // image (fallible closure, or a catch protocol at the writer,
            // row 116) together with rows 71/104 when WriteJob becomes
            // constructible. The OSS DefaultParser rejects every location,
            // so this arm is the COMMON path for async descriptor writes.
            Err(exc) => std::panic::panic_any(Box::new(exc) as Box<dyn Error + Send + Sync>),
        }
    };

    enqueue_write(env, repo, queue, odb_schema, 0, None, false, on_complete, write_content)
}

/// 'downloadBatchFromLocation' (:192-202) — resolve a batch descriptor's
/// location into a [`WriteContent`]. The parser's `thrift::Exception` is the
/// failure channel (batch_location.rs `Parser::from_string`); the OSS
/// `DefaultParser` rejects every location, so async descriptor writes always
/// fail here.
fn download_batch_from_location(
    env: &Env,
    batch_descriptor: &thrift::BatchDescriptor,
) -> Result<WriteContent, thrift::Exception> {
    let batch_format = batch_descriptor.format;
    let location_string = &batch_descriptor.location;
    let batch = env
        .env_batch_location_parser
        .from_string(location_string, batch_format)?;
    Ok(write_content_from_batch(batch))
}

// ---------------------------------------------------------------------------
// enqueueWrite (:74-133)
// ---------------------------------------------------------------------------

/// 'Add a write job to the queue, or throw Retry if no memory available'
/// (source doc, :74).
///
/// Everything from the shutdown check to the counter updates is ONE
/// transaction (the TRANSACTION MAP above): one critical section holding
/// `env_shutting_down` -> WRITE_QUEUES -> WRITE_QUEUE_STATE. The deferred
/// stats blocks run after the guards are dropped (RULE §2 B1). The completion
/// slot is returned whether or not the write is later accepted by the
/// writer — the caller polls it or remembers it (`rememberWrite`,
/// Writes.hs:421-433), exactly as the source returns `mvar` (:133).
///
/// `+ Send` on the two callbacks is the writer-thread boundary (the job
/// crosses the queue to writerThread); it cannot be satisfied for
/// env-capturing closures until the Env Send decision lands — see the
/// marker at the WriteJob construction below.
pub fn enqueue_write(
    env: &Env,
    repo: &thrift::Repo,
    queue: &Arc<WriteQueue>,
    odb_schema: &DbSchema,
    size: i64,
    opt_schema_id: Option<&thrift::SchemaId>,
    check_queue_size: bool,
    on_complete: impl FnOnce(Result<Subst, Box<dyn Error + Send + Sync>>) + Send,
    write_content: impl FnOnce() -> WriteContent + Send,
) -> Result<MVar<Result<Subst, Box<dyn Error + Send + Sync>>>, EnqueueWriteError> {
    // `start <- beginTick 1` (:88) — the latency anchor the writer subtracts
    // from its end time (Writes.hs:119-121).
    let start: Point = begin_tick(1);

    // `config <- Observed.get envServerConfig` (:89) — before the
    // transaction, as in the source.
    let config = env.env_server_config.get();

    // `mvar <- newEmptyMVar` (:90) — the completion slot; the SAME handle is
    // the job's `writeDone` (RULE §2 MVar row: the slot is a shared,
    // Clone-able reference, types.rs's MVar contract).
    let mvar = MVar::new_empty();

    // :92-100 — check the schema ID in the batch matches the DB (plain IO,
    // BEFORE the transaction). `Map.member schemaId (schemaEnvs odbSchema)`
    // -> `BTreeMap::contains_key`; the error is `dbError repo $ printf ...`.
    if let Some(schema_id) = opt_schema_id {
        if config.config_check_write_schema_id && !odb_schema.schema_envs.contains_key(schema_id) {
            // :96-99 — "schema ID in batch (%s) does not match schema ID of
            // DB (%s)" with `unSchemaId schemaId` (the raw text) and
            // `show (Map.keys (schemaEnvs odbSchema))`.
            let keys: Vec<String> = odb_schema
                .schema_envs
                .keys()
                .map(|id| quote_text(id))
                .collect();
            return Err(EnqueueWriteError::Db(db_exception(
                repo,
                &format!(
                    "schema ID in batch ({}) does not match schema ID of DB ({})",
                    schema_id,
                    format!("[{}]", keys.join(","))
                ),
            )));
        }
    }

    // :122-133 — the `immediately` block: ONE transaction (TRANSACTION MAP).
    // Guard 1: `env_shutting_down` (the source's first read, :123).
    let sd_guard = lock(&env.env_shutting_down);
    let shutting_down = *sd_guard;

    // Guard 2: WRITE_QUEUES — writeQueuesSize is read by checkMemoryAvailable
    // (:165), re-read by enqueueIt (:104) and mutated by the registration
    // (:145); all inside this transaction, so the guard spans the whole
    // critical section.
    let mut write_queues_guard = lock(&env.env_write_queues.write_queues_state);

    // :124-128 — the check. `shuttingDown -> reject new writes: client backs
    // off and resends` (:125 comment).
    let check = if shutting_down {
        false
    } else if check_queue_size {
        // :127 — `now $ checkMemoryAvailable env config size`.
        check_memory_available(&mut write_queues_guard, &config, size)
    } else {
        true
    };

    if check {
        // enqueueIt (:103-121).
        // Guard 3: WRITE_QUEUE_STATE (after WRITE_QUEUES — the types.rs order).
        let mut queue_guard = lock(&queue.write_queue_state);

        // :104-106 — `pending <- readTVar writeQueuesSize; newSize = pending
        // + size`. NOTE: when checkQueueSize reserved the space above,
        // writeQueuesSize ALREADY includes `size`, so `new_size` counts this
        // batch TWICE in the "glean.db.write.queue" stat. That is the
        // source's own arithmetic (the reserve at :169 and this re-read at
        // :104 are two statements of the same transaction) — reproduced
        // as-is (RULE §0: the old code is the spec; the counter itself is
        // updated correctly).
        // BUG(port): transient over-report in the source gauge. Repro: the
        // checkQueueSize=True callers (enqueueBatch, Writes.hs:231;
        // enqueueJsonBatch, Writes.hs:282) reserve the space first —
        // checkMemoryAvailable sets writeQueuesSize = pending + size at
        // Queue.hs:169 — then THIS re-read at Queue.hs:104 computes
        // pending + 2*size, and reportQueueSizes emits it on
        // "glean.db.write.queue" at Queue.hs:121: the batch is counted twice
        // until `done` subtracts writeSize once (Writes.hs:132). Reproduced
        // bug-for-bug; fix only as flagged post-parity.
        let pending = write_queues_guard.size;
        let new_size = pending + size;

        // :106-116 — build the job and push it (addWriteJobToQueue,
        // :141-150, inside the same transaction).
        //
        // TODO(port): WriteJob construction is blocked on the Env
        // Send/ownership decision (types.rs TODO at `Env`; rows 71/104 own
        // it). The pinned `WriteJob` field images are `Box<dyn FnOnce ... +
        // Send>` ('static) and `Env` is `!Send`, but these closures borrow
        // `env`/`odbHandle` from the caller's bracket (the source shares
        // them by reference; Open.hs:205-212 hands them out of OpenDB, and
        // Queue.hs:58-70 is the capture evidence types.rs names). The
        // construction below is written against the pinned shapes verbatim;
        // it binds only once the shared-handle posture lands (Arc-carried
        // captures). No `unsafe impl Send` is added (RULE §3).
        let job = WriteJob::WriteJob {
            // `writeSize` (:112).
            write_size: size,
            // `writeContentIO` (:113).
            write_content_io: Box::new(write_content),
            // `writeDone` (:114) — the same slot the caller receives.
            write_done: mvar.clone(),
            // `writeStart` (:115).
            write_start: start,
            // `writeOnComplete` (:116).
            write_on_complete: Box::new(on_complete),
        };

        // :106-116 + :141-150 — push + round-robin registration. The
        // per-queue push signals `write_queue_changed` and the registration
        // signals `write_queues_changed` (the `writeTQueue` wake images;
        // types.rs's "signal after every push" contract). A broadcast, not
        // `notify_one`: the STM commit wakes EVERY blocked `readTQueue` and
        // each loser re-checks its predicate — the Condvar image of that is
        // `notify_all` (RULEBOOK §2 STM row).
        add_write_job_to_queue(
            repo,
            (queue, &mut queue_guard),
            (&env.env_write_queues, &mut write_queues_guard),
            job,
        );

        // :117-118 — `queueCount <- updateTVar writeQueueCount (+1);
        // queueSize <- updateTVar writeQueueSize (+ size)` (the pinned
        // `updateTVar`: read, write, RETURN the new value — Util/STM.hs:25-30).
        queue_guard.count += 1;
        queue_guard.size += size;
        let queue_count = queue_guard.count;
        let queue_size = queue_guard.size;

        // Transaction commit edge: drop the guards, then run the deferred
        // block (:119-121) OUTSIDE the locks (RULE §2 B1). Reverse
        // acquisition order.
        drop(queue_guard);
        drop(write_queues_guard);
        drop(sd_guard);

        // `later $ do addStatValueType "glean.db.write.enqueued" (size div k)
        // Sum; reportQueueSizes repo queueCount queueSize newSize Nothing`
        // (:119-121).
        add_stat_value_type(b"glean.db.write.enqueued", size.div_euclid(K), ExportType::Sum);
        report_queue_sizes(repo, queue_count, queue_size, new_size, None);

        Ok(mvar)
    } else {
        // :129-132 — the reject path. `latency <- readTVar
        // writeQueueLatency` (:130) is the PER-QUEUE latency, so
        // WRITE_QUEUE_STATE is taken here too (after WRITE_QUEUES).
        let queue_guard = lock(&queue.write_queue_state);
        let latency = queue_guard.latency;
        drop(queue_guard);
        drop(write_queues_guard);
        drop(sd_guard);

        // `let elapsed = fromIntegral (toNanoSecs latency) / 1000000000.0`
        // (:131).
        let elapsed = latency.as_nanos() as f64 / 1_000_000_000.0;
        // `later $ rejectWrite repo size elapsed` (:132) — the deferred
        // block emits the stats, then throws (the `Err` return is the
        // throwIO image).
        Err(reject_write(repo, size, elapsed))
    }
}

/// 'addWriteJobToQueue' (:135-150) — the push primitive.
///
/// `STM ()` -> a function over guards the CALLER holds (RULEBOOK §2 STM row:
/// the transaction is the caller's critical section). Both this unit's
/// [`enqueue_write`] and the future enqueueCheckpoint (Writes.hs:210-211,
/// row 116) compose it into a larger transaction; taking the locks here
/// would split that transaction and deadlock on the held guards.
///
/// Parameters mirror the source order `(repo, queue, writeQueues, job)`; the
/// guards ride with their handles. The round-robin element is
/// `(Repo, Arc<WriteQueue>)` (types.rs): the SAME queue as
/// `Writing.wr_queue`, so the handle parameter is the caller's
/// `&Arc<WriteQueue>` (an `Arc` cannot be recovered from a bare
/// `&WriteQueue`).
pub fn add_write_job_to_queue(
    repo: &thrift::Repo,
    // `queue :: WriteQueue` (:137) — the shared handle (for the Condvar)
    // plus its held WRITE_QUEUE_STATE guard.
    queue: (&Arc<WriteQueue>, &mut WriteQueueState),
    // `WriteQueues` (:138) — the handle plus its held WRITE_QUEUES guard.
    write_queues: (&WriteQueues, &mut WriteQueuesState),
    job: WriteJob,
) {
    let (queue, queue_state) = queue;
    let (write_queues, write_queues_state) = write_queues;

    // :142 — `wasEmpty <- isEmptyTQueue writeQueue`.
    let was_empty = queue_state.queue.is_empty();

    // :143 — `writeTQueue writeQueue job`; the wake image per the types.rs
    // contract ("signal write_queue_changed after every push").
    queue_state.queue.push_back(job);
    queue.write_queue_changed.notify_all();

    // :144-145 — "-- if this repo previously had no writes, add it to the
    // round-robin: when wasEmpty $ writeTQueue writeQueues (repo, queue)".
    // The invariant this maintains with the writer's requeue
    // (Writes.hs:177-180): a queue is on the round-robin iff it is
    // non-empty. The registration wakes the writer parked on the empty
    // round-robin (the `readTQueue writeQueues` image).
    if was_empty {
        write_queues_state
            .queues
            .push_back((repo.clone(), Arc::clone(queue)));
        write_queues.write_queues_changed.notify_all();
    }
}

/// 'rejectWrite :: Repo -> Int -> Double -> IO retry' (:148-155) — 'Update
/// stats and throw a Retry exception'. Private. The `throwIO` is the `Err`
/// return; the stats run before it, as the source's deferred block does
/// (Queue.hs:132 -> :148-155).
fn reject_write(repo: &thrift::Repo, size: i64, elapsed: f64) -> EnqueueWriteError {
    // :150 — `clamped = min 1000.0 $ max 1.0 elapsed`.
    let clamped = 1000.0_f64.min(1.0_f64.max(elapsed));
    let repo_b = repo.name.as_bytes();

    // :152 — `addStatValueType "glean.db.write.rejected" (size div k) Sum`.
    add_stat_value_type(b"glean.db.write.rejected", size.div_euclid(K), ExportType::Sum);
    // :153-154 — `addStatValueType ("glean.db.write.retry_ms." <> repoB)
    // (round (elapsed*1000)) Avg` — `round` is banker's rounding, hence
    // [`round_half_to_even`]. The UNclamped elapsed feeds this stat.
    add_stat_value_type(
        &stats_name(b"glean.db.write.retry_ms.", repo_b),
        round_half_to_even(elapsed * 1000.0),
        ExportType::Avg,
    );
    // :155 — `throwIO (Retry clamped)` — the thrift Retry exception
    // (glean.thrift:239-241); Writes.hs:238-239 renders it as
    // `SendResponse_retry (BatchRetry n)`.
    EnqueueWriteError::Retry(thrift::Retry { seconds: clamped })
}

/// 'checkMemoryAvailable :: Env -> ServerConfig.Config -> Int -> STM Bool'
/// (:158-171) — 'Check and update the in-memory write queue size'. Private.
///
/// `STM Bool` -> a function over the held WRITE_QUEUES guard (the
/// transaction is the caller's — the reserve must be atomic with the push,
/// per the TRANSACTION MAP). The `Env` unpack of the source signature
/// dissolves into the guard parameter. `config_db_write_queue_limit_mb` is
/// `i32` in the IDL (server_config.thrift:240); `fromIntegral` -> `as i64`.
fn check_memory_available(
    write_queues_state: &mut WriteQueuesState,
    config: &ServerConfig,
    size: i64,
) -> bool {
    // :165 — `pending <- readTVar writeQueuesSize`; :166 — `newSize =
    // pending + size`.
    let pending = write_queues_state.size;
    let new_size = pending + size;
    // :167-171 — `roundUp mb newSize <= fromIntegral
    // config_db_write_queue_limit_mb` reserves the space; otherwise leave
    // the counter untouched.
    if round_up(MB, new_size) <= config.config_db_write_queue_limit_mb as i64 {
        write_queues_state.size = new_size;
        true
    } else {
        false
    }
}

// ---------------------------------------------------------------------------
// reportQueueSizes (:173-184)
// ---------------------------------------------------------------------------

/// 'reportQueueSizes' (:173-184) — the four queue gauges, plus the latency
/// gauge when the caller has one (the writer's `done` block passes `Just
/// latency`; the enqueue path passes `Nothing`, Queue.hs:121/:133).
///
/// `Maybe TimeSpec` -> `Option<Duration>` (RULE §1 B3; the value is the
/// writer's monotonic difference, Writes.hs:119-121).
pub fn report_queue_sizes(
    repo: &thrift::Repo,
    repo_queue_count: i64,
    repo_queue_size: i64,
    total_queue_size: i64,
    m_latency: Option<Duration>,
) {
    // :175 — `repoB = Text.encodeUtf8 (repo_name repo)`.
    let repo_b = repo.name.as_bytes();

    // :176-179 — the three Avg gauges; `div k` on both size stats.
    add_stat_value_type(
        &stats_name(b"glean.db.write.queue_count.", repo_b),
        repo_queue_count,
        ExportType::Avg,
    );
    add_stat_value_type(
        &stats_name(b"glean.db.write.queue.", repo_b),
        repo_queue_size.div_euclid(K),
        ExportType::Avg,
    );
    add_stat_value_type(
        b"glean.db.write.queue",
        total_queue_size.div_euclid(K),
        ExportType::Avg,
    );

    // :180-184 — `forM_ mLatency $ \latency -> ... when (elapsedMilliSeconds
    // > 0) $ addStatValueType ("glean.db.write.queue_ms." <> repoB)
    // elapsedMilliSeconds Avg`. `toNanoSecs latency div 1000000` is integer
    // division of the nanoseconds.
    if let Some(latency) = m_latency {
        let elapsed_milli_seconds = (latency.as_nanos() / 1_000_000) as i64;
        if elapsed_milli_seconds > 0 {
            add_stat_value_type(
                &stats_name(b"glean.db.write.queue_ms.", repo_b),
                elapsed_milli_seconds,
                ExportType::Avg,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// writeContentFromBatch (:186-190)
// ---------------------------------------------------------------------------

/// 'writeContentFromBatch :: Thrift.Batch -> WriteContent' (:186-190) — the
/// plain-batch record constructor (no ownership).
pub fn write_content_from_batch(write_batch: thrift::Batch) -> WriteContent {
    WriteContent {
        write_batch,
        write_ownership: None,
    }
}

// PORT STATUS: confidence=medium todos=2
