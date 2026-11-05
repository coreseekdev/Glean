// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// Ported from: glean/util/Glean/Util/Warden.hs (104 lines)
// Manifest unit: rust/glean-util/src/util/warden.rs
// (migration/manifest.tsv line 103, crate glean-util). RULE §4: one source
// file -> one target file; the manifest path is authoritative.

//! WHAT THIS FILE IS: the port record of `Glean.Util.Warden`, the
//! thread-ownership registry — and the migration's FIRST owner of an
//! `Async`-shaped abstraction.
//!
//! The source (RULE §0: the old code is the spec):
//!
//! ```haskell
//! newtype Warden = Warden (MVar (Maybe (HashSet (Async ()))))      -- :37
//! withWarden = bracket create shutdown                             -- :41-42
//! create = Warden <$> newMVar (Just mempty)                        -- :45-46
//! shutdown (Warden v) = do                                         -- :50-53
//!   r <- swapMVar v Nothing
//!   mapM_ (Async.mapConcurrently_ Async.cancel) r
//! forget (Warden v) async = modifyMVar_ v $ ...                    -- :55-58
//! spawnMask (Warden v) action = modifyMVar v $ ...                 -- :62-70
//! spawn warden action = spawnMask warden $ \unmask -> unmask action -- :73-74
//! spawn_ w = void . spawn w                                        -- :77-78
//! spawnDaemon w name io = void $ spawnMask w $ \restore ->         -- :83-90
//!   forever $ handle (\ex -> ...) $ restore io
//! adopt (Warden v) async = modifyMVar_ v $ ...                     -- :95-104
//! ```
//!
//! `Control.Concurrent.Async.Async a` is a cancellable thread handle:
//! `wait` blocks for the result (rethrowing the thread's exception),
//! `waitCatch` returns `Either SomeException a`, and `cancel` throws
//! `AsyncCancelled` into the thread and waits for it to die. Rust std has
//! NO thread cancellation and no exception hierarchy; this unit chooses the
//! image every later consumer inherits (see TRANSLATION NOTES):
//!
//!   * `Task<A>` — the `Async a` image: a cloneable handle over a
//!     `JoinHandle<()>`, a shared result slot (`Mutex<Option<Result<A,
//!     TaskError>>>` + `Condvar`), and a cooperative cancel flag.
//!   * `CancelToken` — the image of `spawnMask`'s rank-2 `unmask` argument:
//!     the capability the action polls for cancellation. `spawn` hands the
//!     token to its action too: the source's action takes no argument
//!     because `throwTo` cancels preemptively, but Rust cancellation is
//!     cooperative, so every `spawn`-owned action must be able to observe
//!     the request (an action may ignore it, like the source's could).
//!   * `TaskError` — the `SomeException` image: a message-carrying value
//!     (the panic payload is rendered at catch time; see the show-tag
//!     marker).
//!
//! The registry is a `Mutex` over `Option<HashMap<ThreadId, Arc<dyn
//! TaskControl>>>` (wrapped in the private `WardenState`) — the source's
//! `MVar (Maybe (HashSet (Async ())))`: the MVar is the mutex, `Maybe` is
//! the `Option`, and the `HashSet` is the `HashMap` keyed by the task's
//! thread identity (see INVENTORY and TRANSLATION NOTES).

// ---------------------------------------------------------------------------
// INVENTORY (RULE §5; migration/inventory*.tsv)
// ---------------------------------------------------------------------------
//
// ONE row governs this file — inventory.tsv:117 == inventory.stm.tsv:16
// (STM_GROUP, status "unknown"):
//   "Warden — MVar (Maybe (HashSet (Async ()))) thread-ownership registry;
//    shutdown cancels all. UNKNOWN. std::thread has no cancellation:
//    Mutex<Option<HashMap<ThreadId,JoinHandle>>> + per-thread
//    AtomicBool/Condvar stop flag for COOPERATIVE shutdown. The row's open
//    question: Async.cancel is abortive mid-FFI — define which loops get
//    check-points (janitor/backuper/writers) and whether shutdown waits
//    (join) or detaches; source cancel-leaks ( forgotten threads) vs
//    join-deadlock tradeoff — q: is drain-with-timeout acceptable as the
//    mapping of mapConcurrently_ cancel?"
//   evidence column: Warden.hs:37 newtype; :46 create; :50-53 shutdown
//   swapMVar + mapConcurrently_ cancel; :63-70 spawnMask (fixIO
//   self-removing async); :83-90 spawnDaemon catch-log loop; :95-104
//   adopt; used Env.hs:123,199,230,232-241 (grep-verified; the evidence
//   column omits the five other importers listed under SIBLING-UNIT
//   CONTRACTS — CompletePredicates.hs, Delete.hs, Open.hs, Types.hs,
//   Derive.hs).
//
// FOLLOWED in shape: `Mutex<Option<HashMap<...>>>` + a per-task
// `AtomicBool`/`Condvar` stop flag, exactly the row's sketch. The row's
// status is "unknown" and its own text invites the open decisions, so the
// two refinements below are recorded here (the value refinement carries
// the row-117 flag):
//   * the map key is the task's `std::thread::ThreadId`. The source's
//     `HashSet (Async ())` keys on the async package's `Eq`/`Hashable`,
//     both defined by `asyncThreadId` (async-2.2.5,
//     Control/Concurrent/Async/Internal.hs:76-83), so `ThreadId` is the
//     source's identity and the row's key. `Task` itself cannot derive
//     `Hash`/`Eq`: it owns a `JoinHandle` (neither trait) and is generic
//     over `A`; the registry must be type-erased and non-generic.
//   * the map value is `Arc<dyn TaskControl>` (a type-erased control block
//     carrying the `JoinHandle` inside a `Mutex`), not a bare
//     `JoinHandle`: shutdown must reach each task to cancel AND join it,
//     and a `JoinHandle` alone cannot be taken by two waiters.
// INVENTORY-BUG-CANDIDATE row 117: the row prescribes `Mutex<Option<HashMap<ThreadId,JoinHandle>>>`, but the stored value is `Arc<dyn TaskControl>` because shutdown must cancel AND join each task and a bare `JoinHandle` cannot be taken by two waiters.
//
// The row's TODO is carried at the `shutdown`/`CancelToken` site (the
// cooperative-cancellation marker). The other inventory files
// (inventory.ffi.tsv, inventory.codeflow.tsv, inventory.protocol.tsv,
// inventory.sites.tsv) have NO row for this file: grep "Warden" and
// "Util/Warden" hits only inventory.tsv, inventory.stm.tsv, manifest.tsv,
// depmap/order.txt and depmap/edges.tsv.

// ---------------------------------------------------------------------------
// DEPENDENCY TRUTH (every import, Warden.hs:22-32)
// ---------------------------------------------------------------------------
//
//   :22 import Control.Concurrent (forkIO)
//        -> std::thread::spawn (the `adopt` watcher and every task worker).
//   :23 import Control.Concurrent.Async (Async)
//   :24 import qualified Control.Concurrent.Async as Async
//        -> NO Rust image exists yet. This unit owns it: `Task<A>` below
//           (`wait`/`wait_catch`/`cancel`), `CancelToken`, `TaskError`.
//           The async-2.2.5 semantics are read from the package source
//           (Control/Concurrent/Async.hs; Internal.hs:69-83, :261-263,
//           :274-281): `Async` is `ThreadId + STM (Either SomeException a)`;
//           `cancel = throwTo t AsyncCancelled <* waitCatch a` (:262-263,
//           throwTo + wait); `AsyncCancelled` and its `Exception` instance
//           are :274-281.
//   :25 import Control.Concurrent.MVar
//        -> std::sync::Mutex + Option: `newMVar (Just mempty)` is
//           `Mutex::new(Some(HashMap::new()))`; `modifyMVar`/`modifyMVar_`
//           are lock-mutate-unlock; `swapMVar` is `Option::take` under the
//           lock. The task completion latch adds a `Condvar`.
//   :26 import Control.Exception
//        -> std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe}:
//           the source's sync-exception image. `finally` is the worker
//           wrapper's unconditional `forget`; `handle` is `catch_unwind`
//           in `spawn_daemon`; `mask_` has no image (no async-exception
//           mask — catalog.rs:1275-1277's `mask_ $ async` ruling);
//           `throwIO AsyncCancelled` is `TaskError::cancelled()`.
//   :27 import Control.Monad
//        -> `void`/`forever`: `let _ = ...` and the `while` loop in
//           `spawn_daemon`.
//   :28 import Data.HashSet (HashSet)
//   :29 import qualified Data.HashSet as HashSet
//        -> std::collections::HashMap<ThreadId, Arc<dyn TaskControl>>
//           (membership is the thread identity; a map, not a set, because
//           shutdown must reach each handle to cancel/join it).
//   :30 import System.IO (fixIO)
//        -> NOT NEEDED: the source's `fixIO` ties the async handle into
//           its own thread body so the body can `forget` itself. The port
//           reads `thread::current().id()` inside the worker, so there is
//           no knot. The same dissolution as catalog.rs:1275-1277
//           (`fixIO` unnecessary because the thread only needs state that
//           already exists).
//   :32 import Util.Log
//        -> `crate::log::log_error` (ASSUMED-IMPORT below; the seam has no
//           manifest unit).
//   `Data.Set` is NOT imported; the source uses `Data.HashSet`.

// ---------------------------------------------------------------------------
// ASSUMED-IMPORT (RULE §1 logging row (A10); RULE §4 B4): the
// `glean_util::log` seam
// ---------------------------------------------------------------------------
//
// `import Util.Log` (Warden.hs:32) has NO manifest unit. The seam's first
// sketch is rust/glean-db/src/query/thrift/internal.rs:190-213 (`vlog`);
// unit 46 added `log_info` (write/send_batch.rs:150-172); the `log_error`
// member is pinned by rust/glean-db/src/database/catalog.rs:481-505, which
// also records the `logError` forcing semantics (Util.Log.String.hs:61-63
// feeds `logCommon` -> `withCString`, so the message is always forced):
//
//     pub fn log_error(msg: impl FnOnce() -> String);
//
// The module does not exist on disk yet (the future owner is
// rust/glean-util/src/log.rs; a later unit adds `pub mod log;` to lib.rs).
// THIS FILE IS INSIDE glean-util, so the import is the in-crate path, not
// the cross-crate `glean_util::log` spelling the brief suggested:
//
//     use crate::log::log_error;
//
// That is the established in-crate convention (util/io.rs:62-74 and :217;
// util/retry_channel_exception.rs:305-330 records the same seam with a
// stand-in). A rustc probe (/tmp/warden-probe/self_import.rs, edition
// 2021) confirms `use glean_util::log::log_error;` does NOT resolve inside
// the crate named `glean_util` (E0433: "cannot find module or crate
// `glean_util` in this scope"), so the cross-crate spelling is not an
// option here. The swap when the log unit lands is import resolution only.

// ---------------------------------------------------------------------------
// SIBLING-UNIT CONTRACTS (the exact public surface later units consume)
// ---------------------------------------------------------------------------
//
// Export parity with Warden.hs:10-20, INCLUDING the abstraction:
//
//   Warden            -> pub struct Warden (field PRIVATE; Clone, Send, Sync)
//   withWarden        -> pub fn with_warden<F, A>(f: F) -> A
//                        where F: FnOnce(&Warden) -> A
//   create            -> pub fn create() -> Warden
//   shutdown          -> pub fn shutdown(warden: &Warden)
//   spawn             -> pub fn spawn<F, A>(warden: &Warden, action: F) -> Task<A>
//                        where F: FnOnce(&CancelToken) -> A + Send + 'static,
//                              A: Send + 'static
//   spawn_            -> pub fn spawn_<F>(warden: &Warden, action: F)
//                        where F: FnOnce(&CancelToken) + Send + 'static
//   spawnMask         -> pub fn spawn_mask<F, A>(warden: &Warden, action: F)
//                        -> Task<A>
//                        where F: FnOnce(&CancelToken) -> A + Send + 'static,
//                              A: Send + 'static
//   spawnDaemon       -> pub fn spawn_daemon<F>(warden: &Warden, name: &str, io: F)
//                        where F: FnMut(&CancelToken) + Send + 'static
//   adopt             -> pub fn adopt<A>(warden: &Warden, task: Task<A>)
//                        where A: Send + 'static
//
// `forget` is PRIVATE (Warden.hs:55-58; not in the export list).
//
// Rust-only public types (documented, not source exports — the same
// disposition catalog.rs:552-554 records for its Rust-only items):
//
//   pub struct Task<A> { /* private */ }
//   impl<A> Clone for Task<A>
//   impl<A: Clone + Send + 'static> Task<A> {
//       pub fn wait(&self) -> A;                          // Async.wait
//       pub fn wait_catch(&self) -> Result<A, TaskError>; // Async.waitCatch
//       pub fn wait_timeout(&self, dur: Duration) -> Option<A>;
//           // Rust-only image of a consumer-side `timeout d $ wait a`
//           // (CompletePredicates.hs:374-379); None on timeout.
//   }
//   impl<A: Send + 'static> Task<A> {
//       pub fn wait_once(self) -> A;  // one-shot, no A: Clone
//       pub fn cancel(&self);         // Async.cancel
//   }
//   pub struct CancelToken { /* private */ }  // the unmask image
//   impl Clone for CancelToken
//   impl CancelToken {
//       pub fn is_cancelled(&self) -> bool;
//       pub fn wait_timeout(&self, dur: Duration) -> bool;
//   }
//   #[derive(Clone, Debug)]
//   pub struct TaskError { /* private */ }
//   impl TaskError {
//       pub fn message(&self) -> &str;
//       pub fn is_cancelled(&self) -> bool;  // AsyncCancelled test
//   }
//   impl Display for TaskError
//
// CONSUMERS (grep-verified; depmap/edges.tsv line numbers):
//   * Database/Types.hs:275 `envWarden :: Warden` (edge :469) — the Env
//     field. `Warden` is Clone + Send + Sync, so the Rust Env stores a
//     plain `Warden` (or a clone) with no extra Arc.
//   * Database/Env.hs (edge :286): :123 create -> create(); :199/:230/
//     :233/:244/:259 spawn_ -> spawn_ (each action becomes `|_| ...`, or
//     `|token| ...` when it polls); :237 spawnDaemon "schema updater"
//     -> spawn_daemon(.., "schema updater", |_| ..); :269 shutdown -> shutdown.
//   * Database/CompletePredicates.hs:331 (edge :210) spawn -> spawn (the
//     action becomes `|_| atomically ... >> doCompletion`); the returned
//     `Task<()>` is cancelled with `task.cancel()` (:337/:342/:345/:351)
//     and waited with `task.wait()` (:352/:356). `waitFor` :374-379 is a
//     SINGLE `timeout 1000000 $ wait async` (1s, then `throwIO (Retry 1)`),
//     i.e. `task.wait_timeout(Duration::from_secs(1))` + `None` -> retry
//     (A = (), Clone).
//   * Database/Delete.hs:111 (edge :267) spawnMask -> spawn_mask (the
//     `\_ -> ...` action ignores the token, i.e. `|_| ...`); the `Task<()>`
//     is stored in envDeleting (:115) and waited at :139 (`Async.wait` ->
//     `wait()`).
//   * Database/Open.hs:690 (edge :341) spawnMask -> spawn_mask; the
//     source's `\restore -> ...` becomes `|token| ...`; :704's
//     `restore $ do ...` is a plain block in Rust (no mask to restore) or
//     a token-checkpoint site for the Open unit. The `Async OpenDB` return
//     (:686) becomes `Task<OpenDB>`; Create.hs:166's `Async.wait opener`
//     is `opener.wait_once()` (OpenDB is not Clone; `wait` would demand
//     `OpenDB: Clone`).
//   * Query/Derive.hs:136 (edge :572) spawn_ -> spawn_ (action `|_| ...`).
//   * Env.hs:230 passes the env listener into `spawn_` (trace.rs:88-91
//     records the same site); the listener's `Arc<dyn Fn + Send + Sync>`
//     (trace.rs:242) satisfies `spawn_`'s `F: FnOnce(&CancelToken) + Send +
//     'static` (the wrapper becomes `|_| listener()`).
//   * util/tests/WardenTest.hs:21-36 is the only withWarden/adopt caller
//     (`withWarden`, `spawn` with action `|_| ...`, `wait`); RULE §4 does
//     not translate tests. No non-test caller of `withWarden` or `adopt`
//     exists in this tree.
//   * Database/Backup.hs and Server/Sharding.hs are NOT Warden importers:
//     grep "Glean.Util.Warden" across glean/ returns exactly the six files
//     above plus WardenTest.hs. The brief's consumer list named them; the
//     tree says otherwise (recorded in the report, not silently followed).

// ---------------------------------------------------------------------------
// TRANSLATION NOTES (the hard decisions)
// ---------------------------------------------------------------------------
//
//   1. THE ASYNC IMAGE. `Task<A>` is a cloneable handle (Haskell's `Async`
//      is an immutable value that can be shared freely) over:
//        - `Arc<TaskInner<A>>`, whose `state` holds the result and the
//          `JoinHandle` (taken by the first waiter/canceller);
//        - `done: Condvar` for `wait`/`waitCatch`/join;
//        - `cancelled: Arc<AtomicBool>` + `wake: Arc<CancelWake>` for the
//          cooperative cancel.
//      The worker wrapper catches the action's panic and stores
//      `Result<A, TaskError>` BEFORE marking `finished`, so a waiter can
//      never observe `finished` without a result. `wait`/`wait_catch` are
//      repeatable (the source's `wait` is), which is why they require
//      `A: Clone`; `wait_once(self)` moves the result out for a non-Clone
//      `A` and consumes one handle (see note 10).
//
//   2. THE CANCEL IMAGE. Haskell `cancel` is PREEMPTIVE (throwTo) and
//      waits. Rust std has none, so cancellation is COOPERATIVE:
//      `signal_cancel` stores the flag and wakes `CancelToken` waiters;
//      `cancel`/`shutdown` then JOIN. A task that never polls the token
//      runs to completion. This is the inventory row's stop-flag image and
//      the marker at `shutdown` below; the row's open questions (which
//      loops get check-points; join vs detach vs drain-with-timeout) are
//      reopened at the consumer units and Step 4. The divergence-marking
//      discipline follows the shipped B2 precedent (query/thrift.rs:
//      709-727): one canonical marker at the site, the real-thread image
//      spelled out, and the reopen condition named.
//
//   3. THE `unmask` PARAMETER. `spawnMask`'s rank-2 `forall b. IO b -> IO b`
//      has no Rust image: there is no async-exception mask to restore, so
//      `unmask action` is the identity on actions. The parameter becomes
//      `&CancelToken` — the only interruptibility capability a Rust thread
//      has. `spawn` hands the token to its action too: the source's
//      `spawn :: Warden -> IO a -> IO (Async a)` takes no action argument
//      because `cancel` is PREEMPTIVE (the exception lands wherever the
//      thread is), whereas Rust cancellation is COOPERATIVE and the action
//      must be able to observe the flag. The source's action could ignore
//      cancellation, and so may a Rust action ignore the token.
//      `spawn_mask` keeps its `FnOnce(&CancelToken)` shape (the source's
//      action receives `unmask`), so the two now have the same shape;
//      `spawn_daemon`'s `restore` is the same token and `io` receives it
//      on every iteration.
//
//   4. THE REGISTRY KEY. The source's `HashSet (Async ())` keys on the
//      async package's `Eq`/`Hashable`, both `asyncThreadId`
//      (async-2.2.5, Internal.hs:76-83). The port keys by
//      `std::thread::ThreadId` — the same identity — because `Task`
//      cannot derive `Hash`/`Eq` (a `JoinHandle` field, a generic `A`) and
//      the registry must be type-erased. The map value is
//      `Arc<dyn TaskControl>`; the `JoinHandle` lives inside it.
//
//   5. `fixIO` / `mask_`. The source's `fixIO` (:67-68) exists only to
//      pass the new async handle into its own thread body for the
//      self-`forget`. The port has no knot: the worker reads its own
//      `thread::current().id()`. `mask_` (:67) is a plain spawn (no
//      async-exception mask; catalog.rs:1275-1277). `modifyMVar` holds the
//      warden MVar across the spawn; the port holds the warden Mutex
//      across the spawn, so the worker's `forget` blocks until the insert
//      completes — the same ordering.
//
//   6. SHUTDOWN ORDER. `mapM_ (Async.mapConcurrently_ Async.cancel)` (:53)
//      starts every cancel concurrently and waits for all. The source's
//      HashSet order is unspecified, and so is the port's HashMap order.
//      The port uses two phases — request every cancel, then wait for
//      every task — so no task's join can delay another task's cancel
//      request. Order is otherwise immaterial: `swapMVar` empties the set
//      before any cancel starts, so a concurrent `spawn` sees `None`.
//
//   7. SPAWN AFTER SHUTDOWN. `Async.async (throwIO AsyncCancelled)` (:70)
//      returns a real (immediately failing) async. The port returns a
//      `Task` whose worker immediately stores `Err(TaskError::cancelled())`
//      ("AsyncCancelled"), so `wait` rethrows the same message and
//      `waitCatch` returns the same `Left`. The task is not registered.
//      The source's `modifyMVar` holds the warden MVar across this
//      throwaway spawn; the port drops the guard first — the state is
//      already `None`, so no mutation is possible and the only observable
//      effect is the same already-failed handle.
//
//   8. `adopt`. `forkIO (waitCatch >> forget)` (:98-100) becomes a detached
//      watcher thread that calls the result-free `join_finished` and then
//      `forget`; it does not inspect the result, so it is unaffected by a
//      `wait_once`. The already-shutdown branch (`Async.cancel` :103)
//      drops the warden lock before cancelling (marker at the site).
//
//   9. `spawn_daemon`. `forever $ handle (...) $ restore io` (:85-90):
//      the loop checks the `CancelToken` at the top of each iteration,
//      passes it to every `io(token)` call (`restore` is `unmask`, the
//      identity in Rust), and `catch_unwind`s each call. There is no
//      `AsyncCancelled` panic to rethrow — cancellation is the token — so
//      a cancel-aware action should return, not panic; a panic is always a
//      sync failure and is logged. `show ex` has no faithful constructor
//      tag (marker at the log site). A long-blocking `io` cannot be
//      interrupted mid-call.
//
//  10. WAIT REPEATABILITY. Haskell's `wait` can be called any number of
//      times on the same `Async`. `Task::wait`/`wait_catch` clone the
//      stored result, hence `A: Clone`. `Task::wait_once(self)` is the
//      Rust-only one-shot form for a non-Clone `A`: it moves the result
//      out, so any other handle must not call `wait` afterwards (a second
//      `wait_once` panics "task result already taken"; a `wait` panics
//      "task finished without a result"). `adopt`'s watcher and
//      `shutdown` use the result-free `join_finished`, so they are safe.
//
//  11. POISONING. Every lock goes through `lock`, the catalog.rs:707-709
//      house helper (`unwrap_or_else(PoisonError::into_inner)`). A task
//      action's panic is caught before it can poison the task state, but a
//      panic while a guard is held must not turn a later `shutdown` into a
//      panic: the source's MVar has no poison concept.
//
//  12. `withWarden = bracket create shutdown` (:41-42). The port uses a
//      private `WardenGuard` whose `Drop` calls `shutdown`; it fires on
//      normal return and during unwinding, exactly `bracket`'s release. No
//      `catch_unwind` is used, so the callback's panic payload is
//      preserved untouched.

use std::any::Any;
use std::collections::HashMap;
use std::fmt;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle, ThreadId};
use std::time::{Duration, Instant};

// ASSUMED-IMPORT (RULE §1 logging row (A10)): see the header block. The
// in-crate spelling of the future `glean_util::log` seam (util/io.rs:217).
use crate::log::log_error;

/// The one lock helper (catalog.rs:707-709): a poisoned mutex is recovered,
/// not propagated — the source's MVar has no poison concept.
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

// ===========================================================================
// The `Async a` image: Task / CancelToken / TaskError
// ===========================================================================

/// The `SomeException` image: the message a task ended with. Haskell's
/// `waitCatch` carries the original exception value; a Rust panic payload
/// is `Box<dyn Any + Send>`, which cannot be cloned or inspected portably,
/// so the worker renders it to a message at catch time (`panic_message`).
/// `TaskError::cancelled()` is the `AsyncCancelled` image and the only
/// error with `is_cancelled() == true` (the source's `fromException` test).
#[derive(Clone, Debug)]
pub struct TaskError {
    message: String,
    cancelled: bool,
}

impl TaskError {
    fn from_panic(payload: Box<dyn Any + Send>) -> Self {
        TaskError {
            message: panic_message(&*payload),
            cancelled: false,
        }
    }

    /// `Async.async (throwIO AsyncCancelled)` (:70): the immediate-failure
    /// image. The message matches `show AsyncCancelled`.
    fn cancelled() -> Self {
        TaskError {
            message: "AsyncCancelled".to_string(),
            cancelled: true,
        }
    }

    /// The rendered message (the `show ex` text the source logs).
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Is this the `AsyncCancelled` image? The `fromException` test a
    /// `waitCatch` caller (Backup.hs:486) performs on the source side.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled
    }
}

impl fmt::Display for TaskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

/// Render a panic payload the way `show ex` renders the message half. The
/// constructor tag half has no Rust image (see the marker at
/// `spawn_daemon`'s log site).
fn panic_message(payload: &(dyn Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "<non-string panic payload>".to_string()
    }
}

/// The wake half of the cooperative cancel: `signal_cancel` notifies this
/// condvar so a `CancelToken::wait_timeout` wakes immediately instead of
/// sleeping out its timeout (the interruptible-wait image of an async
/// exception landing in `threadDelay`).
struct CancelWake {
    lock: Mutex<()>,
    cv: Condvar,
}

/// The image of `spawnMask`'s rank-2 `unmask` argument: the capability the
/// action is handed to observe (and wait for) cancellation. Rust has no
/// async-exception mask, so this is a token, not a function.
#[derive(Clone)]
pub struct CancelToken {
    cancelled: Arc<AtomicBool>,
    wake: Arc<CancelWake>,
}

impl CancelToken {
    fn new(cancelled: Arc<AtomicBool>, wake: Arc<CancelWake>) -> Self {
        CancelToken { cancelled, wake }
    }

    /// Has cancellation been requested? Poll this at a task's check-points.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    /// Block for at most `dur`, or until cancellation is signalled. Returns
    /// true if cancelled. This is the Rust-only interruptible wait a
    /// cancel-aware loop should use instead of a bare sleep.
    pub fn wait_timeout(&self, dur: Duration) -> bool {
        if self.is_cancelled() {
            return true;
        }
        let guard = lock(&self.wake.lock);
        if self.is_cancelled() {
            return true;
        }
        let (guard, _timeout) = self
            .wake
            .cv
            .wait_timeout(guard, dur)
            .unwrap_or_else(PoisonError::into_inner);
        drop(guard);
        self.is_cancelled()
    }
}

/// Per-task state, guarded by `TaskInner::state`. `result` is `None` until
/// the worker finishes; `finished` is the result-free completion flag (so
/// `wait_once` can move the result out without hanging a watcher); `join`
/// is taken by the first waiter/canceller; `worker` is the id of the
/// thread that owns the action (the self-wait guard).
struct TaskState<A> {
    result: Option<Result<A, TaskError>>,
    finished: bool,
    join: Option<JoinHandle<()>>,
    worker: Option<ThreadId>,
}

/// The non-generic-in-the-registry control block. `Task<A>` and the
/// warden's `Arc<dyn TaskControl>` point at the same allocation.
struct TaskInner<A> {
    cancelled: Arc<AtomicBool>,
    wake: Arc<CancelWake>,
    state: Mutex<TaskState<A>>,
    done: Condvar,
}

impl<A> TaskInner<A> {
    fn new() -> Self {
        TaskInner {
            cancelled: Arc::new(AtomicBool::new(false)),
            wake: Arc::new(CancelWake {
                lock: Mutex::new(()),
                cv: Condvar::new(),
            }),
            state: Mutex::new(TaskState {
                result: None,
                finished: false,
                join: None,
                worker: None,
            }),
            done: Condvar::new(),
        }
    }

    fn token(&self) -> CancelToken {
        CancelToken::new(Arc::clone(&self.cancelled), Arc::clone(&self.wake))
    }

    fn set_join(&self, handle: JoinHandle<()>) {
        let mut state = lock(&self.state);
        state.worker = Some(handle.thread().id());
        state.join = Some(handle);
    }

    /// Is the caller the task's own worker thread? `cancel`/`shutdown`
    /// from inside the action must not wait for a completion that cannot
    /// happen while the action is running. This is a deadlock guard, not a
    /// reproduction of the source's `throwTo`-into-self (which the action
    /// would observe at its next interruptible point).
    fn is_worker_thread(&self) -> bool {
        lock(&self.state).worker == Some(thread::current().id())
    }

    /// The worker's tail: store the result, then mark finished and wake
    /// every waiter. The result is visible before `finished` (same lock).
    fn finish(&self, result: Result<A, TaskError>) {
        {
            let mut state = lock(&self.state);
            state.result = Some(result);
            state.finished = true;
        }
        self.done.notify_all();
    }

    /// `signal_cancel`: set the flag, then wake a `wait_timeout` waiter.
    /// Store-before-notify is what makes the lost-wakeup race impossible.
    fn signal_cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        let _guard = lock(&self.wake.lock);
        self.wake.cv.notify_all();
    }

    /// Wait for the worker to finish, then join it if this caller wins the
    /// `JoinHandle`. Result-free, so `adopt`'s watcher and `shutdown` do
    /// not depend on the result still being present. A caller that IS the
    /// worker returns immediately (self-cancel/self-shutdown deadlock
    /// guard).
    fn join_finished(&self) {
        if self.is_worker_thread() {
            return;
        }
        let mut state = lock(&self.state);
        while !state.finished {
            state = self
                .done
                .wait(state)
                .unwrap_or_else(PoisonError::into_inner);
        }
        if let Some(handle) = state.join.take() {
            drop(state);
            let _ = handle.join();
        }
    }
}

impl<A: Clone> TaskInner<A> {
    /// The repeatable `wait`/`waitCatch` path: wait for `finished`, clone
    /// the stored result, then join. On the worker thread itself there is
    /// no completion to wait for, so it returns the `AsyncCancelled` image
    /// instead of deadlocking (self-wait guard).
    fn wait_result(&self) -> Result<A, TaskError> {
        let self_call = self.is_worker_thread();
        let mut state = lock(&self.state);
        loop {
            if state.finished {
                let result = state
                    .result
                    .as_ref()
                    .expect("task finished without a result")
                    .clone();
                if !self_call {
                    if let Some(handle) = state.join.take() {
                        drop(state);
                        let _ = handle.join();
                    }
                }
                return result;
            }
            if self_call {
                return Err(TaskError::cancelled());
            }
            state = self
                .done
                .wait(state)
                .unwrap_or_else(PoisonError::into_inner);
        }
    }

    /// The Rust-only timed wait: `Some(result)` if the task finishes within
    /// `dur`, `None` on timeout. Does not cancel the task. The deadline is
    /// recomputed after every (spurious) wakeup.
    fn wait_result_timeout(&self, dur: Duration) -> Option<Result<A, TaskError>> {
        let deadline = Instant::now() + dur;
        let mut state = lock(&self.state);
        loop {
            if state.finished {
                let result = state
                    .result
                    .as_ref()
                    .expect("task finished without a result")
                    .clone();
                if let Some(handle) = state.join.take() {
                    drop(state);
                    let _ = handle.join();
                }
                return Some(result);
            }
            let now = Instant::now();
            if now >= deadline {
                return None;
            }
            let (guard, _timeout) = self
                .done
                .wait_timeout(state, deadline - now)
                .unwrap_or_else(PoisonError::into_inner);
            state = guard;
        }
    }
}

impl<A> TaskInner<A> {
    /// The one-shot path: move the result out. A second `wait_once` (or a
    /// `wait` afterwards) panics rather than hanging. On the worker thread
    /// itself it returns the `AsyncCancelled` image instead of deadlocking
    /// (self-wait guard).
    fn take_result(&self) -> Result<A, TaskError> {
        let self_call = self.is_worker_thread();
        let mut state = lock(&self.state);
        loop {
            if state.finished {
                let result = state.result.take().expect("task result already taken");
                if !self_call {
                    if let Some(handle) = state.join.take() {
                        drop(state);
                        let _ = handle.join();
                    }
                }
                return result;
            }
            if self_call {
                return Err(TaskError::cancelled());
            }
            state = self
                .done
                .wait(state)
                .unwrap_or_else(PoisonError::into_inner);
        }
    }
}

/// The type-erased half the warden stores: cancellation and joining do not
/// need `A`.
trait TaskControl: Send + Sync {
    fn request_cancel(&self);
    fn wait_done(&self);
}

impl<A: Send + 'static> TaskControl for TaskInner<A> {
    fn request_cancel(&self) {
        self.signal_cancel();
    }

    fn wait_done(&self) {
        self.join_finished();
    }
}

/// The `Async a` image. Cloneable (Haskell shares the immutable handle);
/// `Send + Sync` when `A: Send`. The result slot is shared through the
/// `Arc`, so every clone observes the same task.
pub struct Task<A> {
    inner: Arc<TaskInner<A>>,
    thread_id: ThreadId,
}

impl<A> Clone for Task<A> {
    fn clone(&self) -> Self {
        Task {
            inner: Arc::clone(&self.inner),
            thread_id: self.thread_id,
        }
    }
}

impl<A: Send + 'static> Task<A> {
    fn control(&self) -> Arc<dyn TaskControl> {
        Arc::clone(&self.inner) as Arc<dyn TaskControl>
    }

    /// `Async.cancel`: request cancellation, then wait for the thread to
    /// finish. Cooperative — see the marker at `shutdown`.
    pub fn cancel(&self) {
        self.inner.signal_cancel();
        self.inner.join_finished();
    }

    /// One-shot `Async.wait` for a non-Clone `A`: consumes this handle and
    /// moves the result out. Do not call `wait` on another clone
    /// afterwards (it panics "task finished without a result").
    pub fn wait_once(self) -> A {
        match self.inner.take_result() {
            Ok(a) => a,
            Err(e) => resume_unwind(Box::new(e.message)),
        }
    }
}

impl<A: Clone + Send + 'static> Task<A> {
    /// `Async.wait`: block for the result and rethrow the thread's failure
    /// (a panic with the same message). Repeatable, hence `A: Clone`.
    pub fn wait(&self) -> A {
        match self.inner.wait_result() {
            Ok(a) => a,
            Err(e) => resume_unwind(Box::new(e.message)),
        }
    }

    /// `Async.waitCatch`: block for the result, returning the failure
    /// instead of rethrowing it. Repeatable, hence `A: Clone`.
    pub fn wait_catch(&self) -> Result<A, TaskError> {
        self.inner.wait_result()
    }

    /// The Rust-only image of a consumer-side `timeout d $ wait a`
    /// (CompletePredicates.hs:374-379's single 1s timeout): wait up to
    /// `dur` for the task to finish. Returns `Some(result)` — rethrowing a
    /// failure like `wait` — if it finished in time, `None` on timeout.
    /// The task keeps running on timeout; this is not `cancel`.
    pub fn wait_timeout(&self, dur: Duration) -> Option<A> {
        match self.inner.wait_result_timeout(dur) {
            Some(Ok(a)) => Some(a),
            Some(Err(e)) => resume_unwind(Box::new(e.message)),
            None => None,
        }
    }
}

/// Spawn a worker that immediately fails with `AsyncCancelled` — the
/// `Async.async (throwIO AsyncCancelled)` image (:70). It is NOT registered
/// with any warden (the warden that refused the spawn is shut down).
fn spawn_cancelled<A: Send + 'static>() -> Task<A> {
    let inner = Arc::new(TaskInner::new());
    let worker_inner = Arc::clone(&inner);
    let handle = thread::spawn(move || {
        worker_inner.finish(Err(TaskError::cancelled()));
    });
    let thread_id = handle.thread().id();
    inner.set_join(handle);
    Task { inner, thread_id }
}

// ===========================================================================
// Warden
// ===========================================================================

/// The warden's mutable state: `None` is the source's `Nothing` (shut
/// down); `Some(map)` owns the live tasks. The MVar is the mutex; the
/// HashSet is the map keyed by thread identity.
struct WardenState {
    tasks: Option<HashMap<ThreadId, Arc<dyn TaskControl>>>,
}

struct WardenInner {
    state: Mutex<WardenState>,
}

/// `newtype Warden = Warden (MVar (Maybe (HashSet (Async ()))))` (:37).
/// The field is PRIVATE, matching the source's abstract export (:11).
#[derive(Clone)]
pub struct Warden {
    inner: Arc<WardenInner>,
}

/// `withWarden = bracket create shutdown` (:41-42). The guard's `Drop` is
/// the bracket release: it runs on normal return and during unwinding.
pub fn with_warden<F, A>(f: F) -> A
where
    F: FnOnce(&Warden) -> A,
{
    let warden = create();
    let _guard = WardenGuard(warden.clone());
    f(&warden)
}

struct WardenGuard(Warden);

impl Drop for WardenGuard {
    fn drop(&mut self) {
        shutdown(&self.0);
    }
}

/// `create = Warden <$> newMVar (Just mempty)` (:45-46). The `IO` in the
/// source is allocation bookkeeping, not an effect (the mutex.rs:118-120
/// `newMutex` precedent), so `create` is a plain function.
pub fn create() -> Warden {
    Warden {
        inner: Arc::new(WardenInner {
            state: Mutex::new(WardenState {
                tasks: Some(HashMap::new()),
            }),
        }),
    }
}

/// `shutdown (Warden v) = do r <- swapMVar v Nothing; mapM_ (...) r`
/// (:50-53). `Option::take` is the `swapMVar`; the two-phase loop is the
/// `mapConcurrently_ cancel` image (see TRANSLATION NOTES 6). Subsequent
/// calls to `spawn` and `shutdown` will be no-ops (Warden.hs:48-49).
pub fn shutdown(warden: &Warden) {
    // TODO(port): async-cancellation divergence — `Async.cancel` (:53,
    // :103) is PREEMPTIVE: it throws `AsyncCancelled` into the target
    // thread (throwTo) and waits for it to die. Rust std has no thread
    // cancellation, so this port is COOPERATIVE (inventory.tsv:117's
    // stop-flag image): `cancel`/`shutdown` set the task's `AtomicBool`,
    // wake its `CancelToken` waiters, and then JOIN. A task that never
    // polls the token runs to completion, so `shutdown` can block
    // indefinitely where the source would have interrupted it, and `wait`
    // after such a cancel returns the normal result instead of rethrowing
    // AsyncCancelled. Reopen at the consumer units named by the row
    // (Env.hs's janitor/backuper/writers place the check-points; note
    // Backup.hs and Sharding.hs are NOT Warden importers) and at the
    // Step-4 decision on whether shutdown joins, detaches, or drains with
    // a timeout (the row's open question).
    let tasks: Vec<Arc<dyn TaskControl>> = {
        let mut state = lock(&warden.inner.state);
        match state.tasks.take() {
            Some(tasks) => tasks.into_values().collect(),
            None => Vec::new(),
        }
    };
    for task in &tasks {
        task.request_cancel();
    }
    for task in &tasks {
        task.wait_done();
    }
}

/// `forget (Warden v) async = modifyMVar_ v $ ...` (:55-58). Private, as
/// in the source. The worker calls it on exit (the `finally` image).
fn forget(warden: &Warden, thread_id: ThreadId) {
    let mut state = lock(&warden.inner.state);
    if let Some(tasks) = &mut state.tasks {
        tasks.remove(&thread_id);
    }
}

/// `spawnMask (Warden v) action = modifyMVar v $ ...` (:62-70). The
/// warden lock is held across the spawn and the insert, so the worker's
/// `forget` cannot run before the task is registered — the ordering
/// `fixIO` + the held MVar gave the source (TRANSLATION NOTES 5).
pub fn spawn_mask<F, A>(warden: &Warden, action: F) -> Task<A>
where
    F: FnOnce(&CancelToken) -> A + Send + 'static,
    A: Send + 'static,
{
    let mut state = lock(&warden.inner.state);
    if state.tasks.is_none() {
        drop(state);
        return spawn_cancelled();
    }

    let inner = Arc::new(TaskInner::new());
    let token = inner.token();
    let worker_inner = Arc::clone(&inner);
    let worker_warden = warden.clone();
    let handle = thread::spawn(move || {
        let thread_id = thread::current().id();
        let result = catch_unwind(AssertUnwindSafe(|| action(&token)));
        // `finally forget` (:68): unconditional, including on a caught
        // panic. It runs BEFORE the result is stored, as async's
        // `asyncUsing` completes the source's `finally` before publishing
        // the result. It blocks on the warden lock until the insert below,
        // so a waiter simply observes completion slightly later.
        forget(&worker_warden, thread_id);
        worker_inner.finish(result.map_err(TaskError::from_panic));
    });
    let thread_id = handle.thread().id();
    inner.set_join(handle);
    state
        .tasks
        .as_mut()
        .expect("checked above")
        .insert(thread_id, Arc::clone(&inner) as Arc<dyn TaskControl>);
    Task { inner, thread_id }
}

/// `spawn warden action = spawnMask warden $ \unmask -> unmask action`
/// (:73-74). `unmask` is the identity in Rust (no async-exception mask),
/// but the token is still handed to the action: the source's action takes
/// no argument only because `throwTo` is preemptive, while Rust
/// cancellation is cooperative and the action must be able to observe the
/// request (it may ignore the token, like the source's action could).
pub fn spawn<F, A>(warden: &Warden, action: F) -> Task<A>
where
    F: FnOnce(&CancelToken) -> A + Send + 'static,
    A: Send + 'static,
{
    spawn_mask(warden, action)
}

/// `spawn_ w = void . spawn w` (:77-78). The action receives the cancel
/// token for the same cooperative-cancellation reason as `spawn`.
pub fn spawn_<F>(warden: &Warden, action: F)
where
    F: FnOnce(&CancelToken) + Send + 'static,
{
    let _ = spawn(warden, action);
}

/// `spawnDaemon w name io = void $ spawnMask w $ \restore -> forever $
/// handle (...) $ restore io` (:83-90). The loop checks the token between
/// iterations and passes it to every `io(token)`; each call is
/// `catch_unwind`ed (the sync-exception image). Cancellation is not a
/// panic, so there is no `AsyncCancelled` rethrow.
pub fn spawn_daemon<F>(warden: &Warden, name: &str, mut io: F)
where
    F: FnMut(&CancelToken) + Send + 'static,
{
    let name = name.to_string();
    let _ = spawn_mask(warden, move |token| {
        while !token.is_cancelled() {
            match catch_unwind(AssertUnwindSafe(|| io(token))) {
                Ok(()) => {}
                Err(payload) => {
                    // TODO(port): show-tag parity — the source logs
                    // `name <> ": " <> show ex` (:89), where `show`
                    // renders the exception's constructor tag plus its
                    // message (e.g. `IOException: ...`). A Rust panic
                    // payload carries only a message (`&str`/`String`;
                    // `panic_message` falls back to a placeholder), so the
                    // tag cannot be rendered. The default panic hook also
                    // prints the payload to stderr before `catch_unwind`
                    // sees it; the source prints nothing (only logError).
                    // Reopen when the Step-4 error model replaces panics
                    // with a typed error enum (RULE §2 dynamic-exceptions
                    // row): `Display` can emit the variant tag, and a
                    // crate-level panic hook or a Result-returning action
                    // can remove the extra stderr line.
                    log_error(|| format!("{}: {}", name, panic_message(&*payload)));
                }
            }
        }
    });
}

/// `adopt (Warden v) async = modifyMVar_ v $ ...` (:95-104). The active
/// branch inserts the task and starts a detached watcher (`forkIO
/// (waitCatch >> forget)`, :98-100); the watcher waits without reading the
/// result, then forgets.
pub fn adopt<A>(warden: &Warden, task: Task<A>)
where
    A: Send + 'static,
{
    let mut state = lock(&warden.inner.state);
    if state.tasks.is_none() {
        drop(state);
        // TODO(port): adopt lock-scope divergence — the source's `Nothing`
        // branch runs `Async.cancel async` INSIDE `modifyMVar_` (:96-104),
        // i.e. holding the warden MVar. The port drops the guard first: a
        // task that self-forgets on THIS warden re-enters `forget` (which
        // takes the same lock) while `cancel` waits for it — a latent
        // source deadlock the port does not reproduce. The state is
        // already `None` (shut down), so no mutation is lost; the only
        // observable difference is that a concurrent `spawn`/`shutdown`
        // can interleave between the observation and the cancel (both are
        // no-ops in that state). Reopen if a consumer depends on
        // adopt-vs-shutdown serialization or the Step-4 judge reproduces
        // the source deadlock. The immediate-cancel contract cannot be met
        // cooperatively: the source's `Async.cancel` throws and returns at
        // the next interruptible point (Warden.hs:92-94 promises "cancelled
        // immediately"), while `cancel` here signals and JOINS, so a task
        // that never polls the token makes this branch block unboundedly.
        task.cancel();
        return;
    }

    let thread_id = task.thread_id;
    let control = task.control();
    state
        .tasks
        .as_mut()
        .expect("checked above")
        .insert(thread_id, control);

    let watcher_warden = warden.clone();
    let watcher = task.clone();
    thread::spawn(move || {
        watcher.inner.join_finished();
        forget(&watcher_warden, thread_id);
    });
}

// ===========================================================================
// Coverage / markers / status
// ===========================================================================
//
// Coverage: all 104 source lines are accounted for. The export list
// (:10-20, 9 names) maps one-to-one per the SIBLING-UNIT CONTRACTS above;
// the private `forget` (:55-58) is `forget` below; every import of :22-32 is
// disposed in DEPENDENCY TRUTH; `Task`/`CancelToken`/`TaskError` are the
// Rust-only surface the external `async` package would otherwise supply.
// The source's `WardenTest.hs` is a test and is not translated (RULE §4).
//
// Marker breakdown (closed set — three port todos, one row-117 inventory
// flag, zero of every other class):
//   * port todos: three — (1) the cooperative-cancellation divergence at
//     `shutdown` (the row-117 obligation: cancel is a flag + join, not a
//     throwTo), (2) the show-tag parity gap at `spawn_daemon`'s log line
//     (a panic payload has no constructor tag), (3) the adopt lock-scope
//     divergence (the port releases the warden lock before cancelling a
//     task on an already-shut-down warden; it also records that the
//     source's immediate-cancel promise cannot be met cooperatively).
//   * source-defect reproductions: zero — no `error`/`head`/`fromJust`;
//     the source's only partial-looking paths (`Async.cancel` on a dead
//     handle; the `Nothing` branches) are total in the source and here.
//   * performance deferrals: zero — the registry is a HashMap keyed by
//     ThreadId, the same O(1) membership the source's HashSet gives; the
//     result clone on `wait` is a Rust ownership cost, not a deferral.
//   * safety comments: zero — no `unsafe`. `AssertUnwindSafe` is safe
//     code; it asserts the task state is not left torn, which the
//     poison-recovering lock and the caught-panic wrapper guarantee.
//   * inventory divergence: one row-117 inventory flag — the row's
//     `unknown` sketch is followed in shape, but the stored map value is
//     `Arc<dyn TaskControl>` rather than the row's bare `JoinHandle`
//     (see INVENTORY).
//
// PORT STATUS: confidence=medium todos=3
