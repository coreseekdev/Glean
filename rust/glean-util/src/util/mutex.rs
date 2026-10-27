// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// Ported from: glean/util/Glean/Util/Mutex.hs (39 lines)
// Manifest unit: rust/glean-util/src/util/mutex.rs
// (migration/manifest.tsv line 102, crate glean-util). RULE §4: one source
// file -> one target file; the manifest path is authoritative.

//! WHAT THIS FILE IS: the port record of `Glean.Util.Mutex`, the MVar-as-
//! mutex wrapper — and why it ships ZERO items (the same disposition as
//! util/some.rs:105-113): every one of its 7 exports dissolves into
//! `std::sync::Mutex`, exactly as the governing inventory row prescribes.
//! The path is kept so this decision record sits where consumers and
//! reviewers look for it (RULE §4: the manifest is authoritative).
//!
//! The source (RULE §0: the old code is the spec — full surface, 7 names,
//! Mutex.hs:9-12):
//!
//! ```haskell
//! newtype Mutex a = Mutex (MVar a)                                -- :17
//! newMutex       :: a -> IO (Mutex a);  newMutex x = Mutex <$> newMVar x
//!                                                                 -- :19-20
//! withMutex       :: Mutex a -> (a -> IO b) -> IO b
//! withMutex (Mutex v) = withMVar v                                -- :22-23
//! withMutexSafe   :: Mutex (f a) -> (forall s . f s -> IO b) -> IO b
//! withMutexSafe (Mutex v) = withMVar v                            -- :25-26
//! withMutex_      :: Mutex a -> IO b -> IO b;  withMutex_ m = withMutex m . const
//!                                                                 -- :28-29
//! tryWithMutex    :: Mutex a -> (a -> IO b) -> IO (Maybe b)
//! tryWithMutex (Mutex v) f =                                        -- :31-36
//!   bracket (tryTakeMVar v) (maybe (return ()) $ putMVar v) (traverse f)
//! tryWithMutexSafe = tryWithMutex                                 -- :38-39
//! ```
//!
//! The idiom is the standard MVar-as-lock: the cell starts full
//! (`newMVar x`); `withMVar` = `bracket takeMVar putMVar` (blocking take,
//! exception-safe put-back, the callback holds the empty cell so every
//! other taker blocks); `tryTakeMVar` = the non-blocking try. This is the
//! MUTEX reading of MVar, not the RULE §2 (A3) communication-slot reading.

// ---------------------------------------------------------------------------
// INVENTORY (RULE §5; migration/inventory*.tsv)
// ---------------------------------------------------------------------------
//
// ONE row governs this file — inventory.tsv:113 == inventory.stm.tsv:18
// (MECHANICAL, status "confirmed"):
//   "direct std::sync::Mutex; withMutex f -> lock().map(f) (guard drop =
//   MVar put-back); tryWithMutex -> try_lock. NOTE: withMutexSafe (forall s
//   rank-2) exists because Haskell lacks ownership — in Rust the &mut guard
//   IS the safety mechanism; drop the _Safe variants. Any code that holds
//   the MVar across a blocking call (e.g. Async.hs maybeFlush) needs an
//   explicit lock-ordering note at the call site"
// (The row's evidence column cites Mutex.hs:17, :19-20, :22-23, :25-26,
// :31-36 and :38-39 — every line of the surface below; grep-verified.)
//
// Followed to the letter — the row's target_translation is a call-site
// dissolution, so NOTHING generic remains for this module to export and no
// INVENTORY-BUG-CANDIDATE arises under RULE §5. One imprecision in the
// row's NOTE is recorded, not diverged from: its illustrative example
// "Async.hs maybeFlush" does not import this module (grep
// "Glean\.Util\.Mutex" across the tree: exactly 7 importer files, listed
// under SIBLING-UNIT CONTRACTS; glean/write/Glean/Write/Async.hs is not
// one of them). The real lock-held-across-blocking-call sites the NOTE
// targets are Batch.hs:148/:309, Derive.hs:552 (read below) and
// HackDeclarationTarget.hs:203 (read below) — the
// explicit lock-ordering note obligation lands on those consumer units.

// ---------------------------------------------------------------------------
// THE NEWTYPE DISPOSITION: `newtype Mutex a = Mutex (MVar a)` dissolves
// ---------------------------------------------------------------------------
//
// In the Haskell tree the newtype exists to put a DISTINGUISHABLE TYPE on
// mutex-MVars (so `Mutex (WriteLock ())` cannot be confused with an MVar
// used as a communication cell). In Rust that distinction is already
// carried by the TYPES THEMSELVES: a mutex is `std::sync::Mutex<T>`; an
// A3 blocking communication slot is the slot struct with blocking
// put/consuming take/repeated-read read (RULE §2 MVar row). No wrapper
// struct is added on top of std::sync::Mutex:
//
//   * The house precedent is unanimous — every shipped MVar-as-mutex image
//     uses `std::sync::Mutex` DIRECTLY, none defines a Mutex newtype
//     (grep "struct Mutex" across rust/ minus target: zero definitions):
//     - rust/glean-typed/src/typed/predicate.rs:63,210,434-441 (the CAF
//       mapping; the poisoning note);
//     - rust/glean-db/src/database/schema.rs:429-433,534,935 ("the
//       source's cache ops are readMVar/modifyMVar_ over an always-full
//       cell ... A3's blocking-slot image is for oneshot channels, not
//       this" — the exact mutex-vs-communication distinction this
//       newtype was drawing);
//     - rust/glean-db/src/database/storage/memory.rs:120,271;
//       rust/glean-db/src/database/catalog.rs:708;
//       rust/glean-db/src/database/catalog/local/memory.rs:184;
//       rust/glean-util/src/util/observed.rs:188-250;
//       rust/glean-util/src/util/throttle.rs:299;
//       rust/glean-core/src/write/stats.rs:513,674.
//   * The governing row says "direct std::sync::Mutex" — a wrapper struct
//     would diverge from it and fork the house idiom in the same stroke.
//   * The type-level distinction survives the dissolution: consumers
//     declare `std::sync::Mutex<WriteLock<()>>` (a mutex) and the RULE §2
//     A3 slot struct (a channel) stays a different type. The newtype's job
//     is done by the type system without a second name.
//
// No shipped A3 slot struct exists yet to cite (no MVar-as-channel unit
// has shipped — grep "A3" hits only schema.rs:431), so the channel-vs-
// mutex distinction rests on the RULE §2 row itself plus the schema.rs
// precedent above.

// ---------------------------------------------------------------------------
// EXPORT MAPPING (source export list Mutex.hs:9-12, all 7 names)
// ---------------------------------------------------------------------------
//
//   Mutex a                   -> std::sync::Mutex<T> (the newtype
//                                dissolves; consumers name the std type —
//                                see THE NEWTYPE DISPOSITION).
//   newMutex x (:19-20)       -> Mutex::new(x). The `IO` in newMutex is
//                                allocation bookkeeping, not an effect:
//                                newMVar x is a pure value creation
//                                (always-full cell), so the IO wrapper
//                                dissolves like any pure-allocation
//                                constructor. Open.hs:653
//                                `newMutex (Storage.WriteLock ())` ->
//                                `Mutex::new(WriteLock(()))`;
//                                HackDeclarationTarget.hs:200
//                                `newMutex ()` -> `Mutex::new(())`.
//   withMutex m f (:22-23)    -> `let guard = m.lock().unwrap_or_else(
//                                |poisoned| poisoned.into_inner());
//                                f(&mut *guard)` — the callback gets the
//                                exclusive lease exactly as withMVar
//                                handed it the cell's value while every
//                                other taker blocked; the guard's drop at
//                                scope end IS the bracket's putMVar. The
//                                row writes this as lock().map(f); the
//                                poisoning recovery between lock() and
//                                the callback is the predicate.rs image
//                                (see EXCEPTION SAFETY).
//   withMutex_ m k (:28-29)   -> `let _guard = m.lock().unwrap_or_else(
//                                |poisoned| poisoned.into_inner()); k()`
//                                — the `const`-discard becomes the
//                                discarded binding; the guard still
//                                delimits the critical section.
//   withMutexSafe (:25-26)    -> the SAME image as withMutex — DROPPED as
//                                a separate name per the row. The rank-2
//                                `forall s . f s -> IO b` existed because
//                                Haskell has no ownership: it pinned the
//                                callback's argument to a FRESH scope
//                                variable so the callback could not
//                                extract the value out of the mutex's
//                                extent (an ST-style guarantee — the
//                                value stored is `f a`, the callback only
//                                ever sees `f s`). In Rust the guarantee
//                                is STRUCTURAL: `lock()` returns a
//                                `MutexGuard` whose borrow ends with the
//                                guard — nothing derived from
//                                `&mut *guard` can outlive the critical
//                                section. The two Safe variants are the
//                                same function at their types' instances
//                                (the source says so literally:
//                                withMutexSafe (Mutex v) = withMVar v is
//                                withMutex's right-hand side; :38-39
//                                tryWithMutexSafe = tryWithMutex). The
//                                concrete `s` of the one live
//                                instantiation is the WriteLock token:
//                                `f := Storage.WriteLock`, stored as
//                                `WriteLock ()` (Open.hs:653), callback
//                                sees `WriteLock w` — the token survives
//                                as the SHIPPED generic
//                                `WriteLock<W>(pub W)`
//                                (rust/glean-db/src/database/storage.rs:
//                                494-497, with the :16-20 imaging note
//                                "the only instantiations are at
//                                WriteLock () ... but the type parameter
//                                stays for parity with the source's w"),
//                                and the DatabaseOps methods stay
//                                generic over W (storage/db.rs:518,559,
//                                625,743,761 `_lock: &WriteLock<W>`) —
//                                ordinary Rust generics, no rank-2.
//   tryWithMutex (:31-36)     -> `match m.try_lock() { Ok(guard) => {
//                                let v = f(&mut *guard); Some(v) }
//                                None => None }` — tryTakeMVar's empty
//                                result is try_lock's None (the busy
//                                path: the source's release bracket is
//                                `maybe (return ()) $ putMVar v`, i.e. a
//                                lock never taken is never re-put —
//                                Rust's None carries no guard to drop,
//                                the same no-op); std's try_lock has TWO
//                                failure modes — Err(WouldBlock) is the
//                                busy None, Err(Poisoned) does not exist
//                                for an MVar: the house form recovers via
//                                into_inner() and proceeds as the taken
//                                path (predicate.rs:435-442). The taken
//                                path is `traverse f` (run the callback,
//                                then restore). The consumer passes the
//                                token
//                                ONWARD into a blocking write
//                                (Batch.hs:148 `tryWithMutexSafe
//                                (wrLock writing) $ \lock ->
//                                reallyWriteBatch ...` — the guard is
//                                held across the whole write; that is
//                                the source's intent, the write lock
//                                serializes batches), so the consumer
//                                unit binds the guard in a `let` rather
//                                than a temporary — same semantics, the
//                                lock-ordering note obligation of the
//                                row's NOTE lands there.
//   tryWithMutexSafe (:38-39) -> the SAME image as tryWithMutex — DROPPED
//                                as a separate name per the row (the
//                                source itself defines it as
//                                `tryWithMutexSafe = tryWithMutex`).

// ---------------------------------------------------------------------------
// EXCEPTION SAFETY (the bracket story, both images)
// ---------------------------------------------------------------------------
//
// Haskell: withMVar/bracket restores the MVar (putMVar in the release
// handler) even when the callback throws; the next withMutex then works —
// an MVar has no poison. Rust: the MutexGuard unlocks on Drop, including
// during unwinding — the same restoration guarantee. The one Rust-only
// wrinkle is POISONING (a panicked holder marks the mutex), which the
// source has no counterpart for. House image, fixed by precedent at
// rust/glean-typed/src/typed/predicate.rs:435-442 ("An MVar cannot poison
// — modifyMVar's cleanup restores it on exception ... The Rust image:
// recover the guard through the poisoning the same way"): recover with
// `lock().unwrap_or_else(|poisoned| poisoned.into_inner())` rather than
// panic — matching the sibling `unwrap_or_else(PoisonError::into_inner)`
// sites (storage/memory.rs:271, catalog.rs:708, catalog/local/memory.rs:
// 184). Consumer units copying these images inline use that form.

// ---------------------------------------------------------------------------
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, checked not assumed)
// ---------------------------------------------------------------------------
//
//   * OUTBOUND: none. edges.tsv has no row with
//     glean/util/Glean/Util/Mutex.hs as source (grep-verified) — the
//     module's two imports are base-only (Control.Concurrent.MVar,
//     Control.Exception (bracket), Mutex.hs:14-15), both dissolved into
//     std::sync above. A leaf module.
//   * INBOUND: 7 rows, exactly the 7 importer files (the two greps agree
//     — see SIBLING-UNIT CONTRACTS): edges.tsv:191 (Close.hs), :209
//     (CompletePredicates.hs), :338 (Open.hs), :464 (Database/Types.hs),
//     :495 (Write/Batch.hs), :570 (Query/Derive.hs), :1435
//     (HackDeclarationTarget.hs).

// ---------------------------------------------------------------------------
// SIBLING-UNIT CONTRACTS (what each consumer calls; ALL UNSHIPPED — the
// rows/futures below are their obligation, not this file's)
// ---------------------------------------------------------------------------
//
//   * glean/db/Glean/Database/Types.hs (manifest line 104 ->
//     rust/glean-db/src/database/types.rs, FUTURE): imports (:68) and
//     DECLARES the field this module exists for —
//     `wrLock :: Mutex (WriteLock ())` (:80, inside `data Writing`) ->
//     `pub wr_lock: Mutex<WriteLock<()>>` in the Writing image
//     (std::sync::Mutex; WriteLock<W> from the SHIPPED
//     glean-db::database::storage unit). The other Writing fields
//     (wrNextId, wrLookupCache, wrQueue, wrCommit ...) are that unit's
//     own STM_GROUP rows.
//   * glean/db/Glean/Database/Open.hs (manifest line 111 ->
//     rust/glean-db/src/database/open.rs, FUTURE): imports (:80);
//     `mutex <- newMutex (Storage.WriteLock ())` (:653, the Writing
//     constructor) -> `Mutex::new(WriteLock(()))` feeding `wr_lock`.
//   * glean/db/Glean/Database/Close.hs (manifest line 121 ->
//     rust/glean-db/src/database/close.rs, FUTURE): imports (:33);
//     `withMutex wrLock $ const $ LookupCache.clear wrLookupCache`
//     (:140) -> bind the guard, clear the lookup cache, drop the guard —
//     the discarded-value (withMutex_) shape over a named field.
//   * glean/db/Glean/Database/CompletePredicates.hs (manifest line 117
//     -> rust/glean-db/src/database/complete_predicates.rs, FUTURE):
//     imports (:61); `withMutexSafe (wrLock writing) f` (:269) is the
//     body of its own `withWriteLock :: Maybe Writing -> (forall w .
//     Storage.WriteLock w -> IO b) -> IO b` (:259-269) — the Nothing
//     branch passes `undefined :: WriteLock ()` (the already-read-only
//     finalization phase) -> in Rust the helper takes
//     `Option<&Mutex<WriteLock<()>>>` and the no-lock branch fakes the
//     token (`WriteLock(())`) exactly as the source does; the forall-w
//     dissolution is the withMutexSafe row above.
//   * glean/db/Glean/Database/Write/Batch.hs (manifest line 112 ->
//     rust/glean-db/src/database/write/batch.rs, FUTURE): imports (:55);
//     `tryWithMutexSafe (wrLock writing) $ \lock -> reallyWriteBatch ...`
//     (:148 — the busy None degrades to deDupBatch) and two
//     `withMutexSafe` sites (:246 `const $ release facts`; :309 whose
//     callback RETURNS a continuation `cont` that runs after the lock is
//     released — `wsubst <- cont` at :316) -> try_lock/lock images per
//     the mapping; the row's NOTE obligation (explicit lock-ordering
//     note: the write lock is held across reallyWriteBatch's blocking
//     fact writes) lands on this unit.
//   * glean/db/Glean/Query/Derive.hs (manifest line 145 ->
//     rust/glean-db/src/query/derive.rs, FUTURE): imports (:61);
//     `withMutexSafe (wrLock writing) $ \lock ->` (:552) wraps
//     computeDerivedOwnership + storeOwnership (both taking the WriteLock
//     token, both blocking) -> lock image; same NOTE obligation (lock
//     held across blocking ownership computation).
//   * glean/lang/hack/Derive/HackDeclarationTarget.hs (manifest line 330
//     -> rust/glean-indexer-hack/src/derive/hack_declaration_target.rs,
//     FUTURE; the crate does not exist yet): imports (:35);
//     `mutex <- newMutex ()` (:200) + `ctx <- withMutex_ mutex $
//     chunkCtx (i, chunk)` (:203, under forConcurrently_ :201) ->
//     Mutex::new(()) / the withMutex_ image. NOTE-TARGET: the lock IS
//     held across blocking backend calls — chunkCtx = contextFor
//     (:228-233) runs concurrently (getSources/getTargets …), real
//     Glean queries (getSourcesQuery :59, getTargetsQuery :78; the
//     source's own comment :202: "avoid many queries in flight at
//     once") — NOT a pure throttle: the row's lock-ordering-note
//     obligation lands HERE too (read below).

// Coverage: the 39-line source is fully accounted for — the newtype and
// all 7 exports dissolve into std::sync::Mutex per the governing row
// (inventory.tsv:113 == inventory.stm.tsv:18), and nothing in the source
// is left unported (the module's body IS the 7 functions mapped above;
// there are no instances, no CAFs, no other declarations).
//
// Marker breakdown (closed set — zero of every class): no TODO(port)
// (nothing is open in THIS file — every consumer obligation above is
// contract text owed by a named future unit, and the consumer survey is
// complete: the 7 importers match edges.tsv's 7 inbound rows exactly);
// no BUG(port) (the source has no defect here to reproduce); no
// PERF(port) (no code); no INVENTORY-BUG-CANDIDATE (the row's
// target_translation is followed exactly — the NOTE's "Async.hs
// maybeFlush" example is recorded as imprecise above, the mapping itself
// is untouched); no SAFETY comments (no unsafe — the escape hatch is not
// invoked).
//
// PORT STATUS: confidence=high todos=0
