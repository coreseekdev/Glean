// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// Ported from: glean/util/Glean/Util/Throttle.hs (29 lines).
// Manifest unit: rust/glean-util/src/util/throttle.rs
// (migration/manifest.tsv line 100, crate glean-util). RULE §4: one source
// file -> one target file; the manifest path is authoritative.
//
// WHAT THIS FILE IS: the heartbeat throttle behind Write/Stats.hs's
// periodic logger — `atMostEvery tm` returns a gate closure which, applied
// to an action, runs that action only if at least `tm` has elapsed since
// the last run (or since the gate's construction). Two exports, exactly
// the source's export list (:10 `Execute, atMostEvery`).
//
// ---------------------------------------------------------------------------
// INVENTORY (RULE §5; migration/inventory*.tsv)
// ---------------------------------------------------------------------------
//
// THE GOVERNING ROW pins the gate's image. inventory.stm.tsv:19, quoted
// verbatim (tab-separated columns file/symbol/source_construct/
// classification/target_translation/evidence/status):
//
//   glean/util/Glean/Util/Throttle.hs	atMostEvery	IORef
//   next-allowed-time gate via atomicModifyIORef'	MECHANICAL	AtomicU64
//   storing next-allowed monotonic instant (nsecs since Instant anchor) —
//   compare_exchange maps atomicModifyIORef' exactly; or Mutex<Instant>.
//   Consumed as Stats logger gate (Stats.hs:212)	Throttle.hs:19-29
//   atMostEvery: newIORef now; atomicModifyIORef' decide-then-bump
//   (now + tm, True)/(next, False)	confirmed
//
// (The same row text appears a second time in the main shard,
// inventory.tsv:115 — both greps verified.) The consumer rows naming this
// gate are inventory.stm.tsv:33 / inventory.tsv:82 (Write/Stats.hs; see
// SIBLING-UNIT CONTRACTS).
//
// THE GATE IMAGE — the row offers TWO sanctioned images; this file ships
// (b) `Mutex<Instant>`, the row's own "or" alternative, so this follows
// the row's letter rather than diverging from it (no
// INVENTORY-BUG-CANDIDATE under RULE §5). Why (b) over the row's
// first-listed (a) `AtomicU64` — every claim below scratch-probed
// (/tmp/probe-throttle/probe.rs, rustc 1.98.1 -O, std only; both images
// plus a naive load-then-store anti-image under identical races):
//
//   * EXACTNESS is a property of the ATOMIC DECIDE+BUMP STEP, and both
//     images have it. Fresh-gate 8-thread race x200: max fires = 1 for
//     BOTH (a) and (b) — exactly one thread fires, the losers read the
//     bumped `next`. 2-thread tight-loop boundary window (300 ms over a
//     5 ms period): 60 fires for BOTH, inside the correct bound
//     ceil(300/5)+1 = 61. The naive load-then-store form the row's
//     "compare_exchange maps atomicModifyIORef' exactly" clause exists to
//     forbid demonstrably over-fires under the same harness: max fires 2
//     (total 214 (single-run sample; re-runs 208-214) over 200 fresh gates — double fires) and 106 > 61 at the
//     boundary. A plain load-then-store race would let two threads both
//     fire; one atomic step does not — whichever library holds it.
//   * (a) costs a process-global anchor: `Instant` has no portable
//     to-nanos, so the u64 store needs a static `once_cell Lazy<Instant>`
//     (whitelisted — glean-typed/src/typed/predicate.rs:212 precedent)
//     plus two conversion surfaces (uptime Instant -> u64 ns, and
//     `tm` Duration -> u64 ns — each exact only below ~2^64 ns ≈ 584
//     years, and neither wrap matches the source's, see DIVERGENCES 1).
//     (b) compares Instants directly — zero conversion surface, no
//     global — and its critical section IS the transaction, so the
//     decide-then-bump needs no auxiliary exactness argument (the (a)
//     CAS loop additionally needs the same-value-store elision note in
//     TRANSLATION NOTES to be exact).
//   * CONTENTION cannot matter: the gate's sole construction site fires
//     it at most once per 10 s (Stats.hs:212, via Database/Env.hs:121
//     `envStats <- Stats.new (TimeSpec 10 0)`); an uncontended Mutex
//     lock is one CAS — the same primitive the (a) loop's first
//     compare_exchange issues.
//
// ---------------------------------------------------------------------------
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, checked not assumed)
// ---------------------------------------------------------------------------
//
//   * OUTBOUND: none in-repo. The source's imports are Control.Monad
//     (`when`, :13) and Data.IORef (:14) — boot libraries — and
//     System.Clock (:15), the external `clock` package (clock-0.8.4 in
//     the pinned build's cabal store, ~/.cabal/store/ghc-9.4.7/, source
//     tarball ~/.cabal/packages/hackage.haskell.org/clock/0.8.4/), which
//     has NO manifest unit; RULE §1's Time row maps it onto std::time
//     (THE CLOCK IMAGE; same disposition as util/metric.rs). edges.tsv
//     has no outbound row for Throttle.hs:
//     $ grep -n 'Throttle' migration/depmap/edges.tsv  -> line 1308 only.
//   * INBOUND (1), a FUTURE unit (not shipped as of this file):
//       edges.tsv:1308  Write/Stats.hs imports the module unqualified
//         (:33); holds `statsLogger :: Execute` (:41), constructs
//         `atMostEvery period` (:212), applies it (:218) — the sole
//         consumer AND the sole construction site in the tree (its own
//         sole caller is Database/Env.hs:121, period `TimeSpec 10 0`).
//
// ---------------------------------------------------------------------------
// ASSUMED IMPORTS (RULE §4 B4)
// ---------------------------------------------------------------------------
//
//   * NONE — std::sync and std::time only; nothing generated or
//     cross-crate is involved.
//
// ---------------------------------------------------------------------------
// SIBLING-UNIT CONTRACTS (what the one consumer calls; grep-verified)
// ---------------------------------------------------------------------------
//
//   * glean/hs/Glean/Write/Stats.hs (manifest row 101 ->
//     rust/glean-core/src/write/stats.rs, FUTURE): imports
//     `import Glean.Util.Throttle` (:33) and uses:
//       Execute            (:41 field `statsLogger :: Execute`)
//                          -> `pub stats_logger: Execute` (an Arc: cheap
//                             to clone into whatever the Env row-8 image
//                             shares envStats across writer threads)
//       atMostEvery period (:211-212 `new period = initial $ Stats <$>
//                          atMostEvery period`)
//                          -> `at_most_every(period)`; the period arrives
//                             as Stats.hs:211's `TimeSpec`, whose
//                             TimeSpec->Duration conversion is ALREADY
//                             ruled by util/metric.rs's sibling contract
//                             for exactly this line (:95-96 there).
//       statsLogger stats $ do … (:218, inside bump :215-224 — the
//                          read-all-then-reset the gate admits at most
//                          once per period, per inventory.stm.tsv:33 /
//                          inventory.tsv:82)
//                          -> `(stats.stats_logger)(&mut || { … })`; the
//                             inline closure-literal form compile-probes.
//     Cross-crate import per RULE §4:
//     `use glean_util::util::throttle::{Execute, at_most_every};`
//     (lib.rs exposes `pub mod util` at the crate root; no re-exports).
//
// ---------------------------------------------------------------------------
// THE CLOCK IMAGE (RULE §1 Time row — cited verbatim; B3-corrected)
// ---------------------------------------------------------------------------
//
//   "Time: NO time crate; `TimePoint`/monotonic → `std::time::Instant`,
//    wall clock → `std::time::SystemTime`, durations `std::time::Duration`
//    (B3 corrects A10's wording, which caused a wall-clock infidelity in
//    round 2)."
//
//   Source (System.Clock, clock-0.8.4)      Target                  Evidence
//   --------------------------------------  ----------------------  ---------
//   `getTime Monotonic` (:21, :24)          `Instant::now()`        RULE §1
//                                                                   row above;
//                                                                   util/
//                                                                   metric.rs
//                                                                   table row 1
//   `tm :: TimeSpec` (:19)                 `Duration`              RULE §1
//                                                                   durations;
//                                                                   metric.rs
//                                                                   :95-96 for
//                                                                   Stats.hs:211
//   `now + tm` (:27)                        `Instant + Duration`    TimeSpec's
//                                                                   Num.+
//                                                                   normalizes
//                                                                   then wraps
//                                                                   its Int64
//                                                                   fields
//                                                                   (Clock.hsc
//                                                                   :271-272);
//                                                                   Instant Add
//                                                                   panics on
//                                                                   platform
//                                                                   overflow —
//                                                                   DIVERGENCES
//                                                                   1
//   `now >= next` (:26)                     `Instant` Ord           TimeSpec's
//                                                                   Eq/Ord
//                                                                   NORMALIZE
//                                                                   both
//                                                                   operands
//                                                                   before
//                                                                   comparing
//                                                                   (Clock.hsc
//                                                                   :302-310)
//                                                                   — a total
//                                                                   order on
//                                                                   the
//                                                                   represented
//                                                                   time; the
//                                                                   stored
//                                                                   `next` is
//                                                                   normalized
//                                                                   too (the
//                                                                   `+` above
//                                                                   normalizes
//                                                                   inside
//                                                                   Num.+), so
//                                                                   Instant
//                                                                   (always
//                                                                   normalized)
//                                                                   compares
//                                                                   exactly
//   `newIORef now` (:22)                    `Mutex<Instant>`        the row's
//                                                                   image (b);
//                                                                   THE GATE
//                                                                   IMAGE
//
// ---------------------------------------------------------------------------
// TRANSLATION NOTES
// ---------------------------------------------------------------------------
//
//   * `Execute` (:17) is a TYPE ALIAS for a function type, so the port
//     stays an alias rather than growing a wrapper struct: an erased
//     shared callable `Arc<dyn Fn(&mut dyn FnMut()) + Send + Sync>` —
//     the same shape as util/observed.rs:147's `pub type UpdateFn<A> =
//     Arc<dyn Fn(&dyn Fn(Option<&A>) -> A) + Send + Sync>` (a Haskell
//     closure-valued field) — with the action passed as
//     `&mut dyn FnMut()` per util/thrift_source.rs:160's `with_value`
//     family. The source applies the action at most once
//     (`when go action`, :29), so FnMut called once is the exact
//     discipline (a Box<dyn FnOnce()> would add a per-call allocation to
//     say the same thing). The ACTION carries no Send bound — it runs on
//     the CALLER's thread inside the gate — while the GATE is Send+Sync
//     because Stats stores it in the shared Env (Env.hs:121). Haskell
//     closures are GC values copied freely; Arc-clone is the Rust image.
//   * Construction (:21-22): `next` starts at the construction instant,
//     so the FIRST call always fires (its now >= the construction now).
//     Mirrored by `Mutex::new(Instant::now())`.
//   * Statement order (:24-:25): the clock is read BEFORE the atomic
//     step. Mirrored — `Instant::now()` precedes the lock. Interleaving
//     parity: a thread whose `now` was read before another thread's bump
//     still compares it against the BUMPED `next` once it gets the lock
//     (an earlier `now` cannot pass `now >= next'`), exactly as a late
//     atomicModifyIORef' sees the bumped IORef.
//   * The False branch: the source's `atomicModifyIORef'` always writes
//     the IORef back, writing the SAME `next` it read on `(next, False)`.
//     A same-value store cannot change any other thread's read outcome,
//     so eliding it — as the locked section does by leaving `*next`
//     untouched — is exact. (Trivial inside the Mutex; recorded because
//     the row's (a) image needs the same argument at its
//     skip-vs-compare_exchange decision.)
//   * `when go action` (:29) runs AFTER the state mutation, OUTSIDE the
//     critical section — RULE §2's STM-invoked-callbacks row (B1)
//     discipline; an in-lock call would deadlock on re-entry. Probed: a
//     re-entrant gate call inside the action neither deadlocks nor fires
//     (the bump already moved `next` past the action's `now`).
//   * `tm == Duration::ZERO` admits every call (`next := now`; the next
//     call's now >= it) — same as the source with `TimeSpec 0 0`. A
//     NEGATIVE `tm` (TimeSpec admits it; `now + tm` lands in the past →
//     admit-every-call) has no Duration image: the signed→unsigned change
//     is RULE §1's own TimeSpec→Duration mapping at this boundary, the
//     sole caller passes `TimeSpec 10 0`, and ZERO is the
//     admit-every-call image if a future caller wants one.
//   * `lock().unwrap()`: the only panic reachable while holding the lock
//     is DIVERGENCES 1's (unreachable) overflow; house convention
//     (util/observed.rs).
//
// ---------------------------------------------------------------------------
// DIVERGENCES (numbered; none observable on reachable inputs)
// ---------------------------------------------------------------------------
//
//   1. `now + tm` overflow (:27): the source's TimeSpec Num.+ is
//      `normalize $! TimeSpec (xs + ys) (xn + yn)` (Clock.hsc:271-272) —
//      the Int64 SECOND field wraps silently past 2^63 s (~292 billion
//      years) and the gate keeps operating on wrapped values (silently
//      wrong but running). The port's `Instant + Duration` PANICS when
//      the underlying platform timespec (i64 tv_sec on Linux) overflows:
//      std offers `checked_add` but NO `saturating_add` for Instant
//      (compile-probed: E0599 "no method named `saturating_add`"; there
//      is no portable platform-max Instant to saturate to), so aborting
//      is the only total-checkable form. Same class and discipline as
//      util/metric.rs DIVERGENCES 2 ("the source would keep counting,
//      silently wrong; the port aborts. Unreachable"): the sole
//      construction site passes `TimeSpec 10 0` — 10 s — seventeen
//      orders of magnitude below the wrap. If the panic ever fired it
//      would fire holding the lock, poisoning the Mutex; every later
//      call then panics on the unwrap — collapse to a deterministic
//      abort rather than the source's silently-wrong-but-running gate
//      (same unreachable input).
//
// The probe record (/tmp/probe-throttle/probe.rs — scratch, never
// in-tree): sequential rows (first fires / immediate second does not /
// post-period fires) identical for images (a) and (b); fresh-gate race
// x200 max fires 1/1 vs naive 2; boundary 300ms/5ms fires 60/60 (bound
// 61) vs naive 106; re-entrant inner call fires 0, no deadlock; inline
// `(gate)(&mut || {..})` compiles.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// 'type Execute = IO () -> IO ()' (:17): an action gate — apply it to an
/// action and the action runs only if at least the throttle period has
/// elapsed since the last run (or since construction). See TRANSLATION
/// NOTES for the alias-over-Arc shape.
pub type Execute = Arc<dyn Fn(&mut dyn FnMut()) + Send + Sync>;

/// 'atMostEvery :: TimeSpec -> IO Execute' (:19-29). The gate: `next`
/// starts at the construction instant (:21-22 — the first call always
/// fires); each call reads the clock (:24) before entering the critical
/// section (:25 — the source's statement order), decides-and-bumps in ONE
/// step (atomicModifyIORef' as a single locked section: fire and bump to
/// `now + tm` iff `now >= next`, else leave `next` untouched), and runs
/// the action only when firing, OUTSIDE the section ('when go action',
/// :29; B1 discipline).
pub fn at_most_every(tm: Duration) -> Execute {
    let next = Mutex::new(Instant::now());
    Arc::new(move |action: &mut dyn FnMut()| {
        let now = Instant::now();
        let go = {
            let mut next = next.lock().unwrap();
            if now >= *next {
                *next = now + tm; // overflow: DIVERGENCES 1
                true
            } else {
                false
            }
        }; // guard dropped — the action runs outside the section
        if go {
            action();
        }
    })
}

// Marker breakdown (closed set): zero port todos (both exports fully
// translated above), zero BUG(port) reproductions (the one hazard
// behavior — TimeSpec's silent Int64-field wrap in `now + tm` — is a
// Num-instance width property neither sanctioned image can represent
// (u64-nanos and Instant wrap/panic at different moduli), so it is
// recorded in DIVERGENCES 1 per util/metric.rs's identical treatment of
// its wrap rows; flagged for reviewers in the unit report), zero PERF
// deferrals (the locked section is the direct form — see THE GATE IMAGE
// on contention), zero inventory divergences (image (b) is the row's own
// "or" alternative), zero unsafe blocks.
//
// PORT STATUS: confidence=high todos=0
