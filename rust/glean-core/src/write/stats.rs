// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// Ported from: glean/hs/Glean/Write/Stats.hs (232 lines; module
// Glean.Write.Stats).
// Manifest unit: rust/glean-core/src/write/stats.rs
// (migration/manifest.tsv line 101, crate glean-core). RULE §4: one
// source file -> one target file; the manifest path is authoritative.
//
// WHAT THIS FILE IS: the write-path statistics accumulator — eight
// monoid-accumulating counters over a shared state, a periodic
// log-and-reset gate (the Throttle consumer: the gate fires at most once
// per period, and when it fires it reads all counters, emits ONE log
// line, and zeroes the "current window" halves), and the fb303
// service-counter pushes that ride along with each bump. The write
// pipeline's `mut_lat`/`mut_thp`/... log line and the
// "glean.db.write.*" fb303 counters both come from here.
//
// ---------------------------------------------------------------------------
// INVENTORY (RULE §5; migration/inventory.tsv line 82 == inventory.stm.tsv
// line 33 — read verbatim from both files, byte-identical)
// ---------------------------------------------------------------------------
//
//   source_file:  glean/hs/Glean/Write/Stats.hs
//   site:         Stats
//   summary:      8 IORef (Stat a) counters, atomicModifyIORef' bump+
//                 accumulate, periodic log-and-reset gated by Throttle
//   class:        STM_GROUP
//   target_translation (VERBATIM):
//     "struct StatsGroup { Mutex<[Stat; 8]> } (or per-counter Mutex —
//      single Mutex is the faithful grouping since bump(); log+reset at
//      215-224 reads ALL then zeroes ALL, which must be atomic vs bumps).
//      Bump = lock + monoid-extend. The statsLogger atMostEvery gate
//      (Throttle row) decides whether the taker does the full read+reset.
//      Service counters (bumpServiceCounter) go to the fb303 stats sink —
//      out of scope here. Also exposed as envStats/envLookupCacheStats
//      (Env row 8)"
//   evidence:     Stats.hs:40-50 data Stats (8 IORef (Stat a) fields);
//                 :54-64 refs list; :66-68 bumper atomicModifyIORef'
//                 (new <> x, all <> x); :211-212 new + atMostEvery;
//                 :215-224 bump: runBump then statsLogger read-all +
//                 reset statNew = mempty; :226-232 tick wrap
//   status:       confirmed
//
// Followed EXACTLY, clause by clause:
//   * ONE Mutex over a StatsGroup holding all 8 counters — never
//     per-counter locks. The row's "reads ALL then zeroes ALL ... must be
//     atomic vs bumps" is why: bump() (:215-224) runs the gate on EVERY
//     bump, and when the gate fires, the read-all (allStats, :155-178)
//     and the zero-all (refs loop, :223-224) sit in ONE critical section
//     here, so no bump can land between a logged value and its reset.
//     (The source's eight separate IORefs make read/reset individually
//     atomic only — the row explicitly upgrades the grouping; the log
//     line is emitted inside the section in the source's own statement
//     order read -> log -> zero.)
//   * "Bump = lock + monoid-extend": bumper (:66-68) takes the group
//     lock, monoid-extends stat_new and stat_all with the bump value,
//     and releases — the atomicModifyIORef' step, widened to the group.
//   * "The statsLogger atMostEvery gate (Throttle row) decides whether
//     the taker does the full read+reset": the gate is CALLED on every
//     bump (:218) but runs the log-and-reset action at most once per
//     period — exactly the sibling throttle.rs gate semantics.
//   * "Service counters (bumpServiceCounter) go to the fb303 stats sink —
//     out of scope here": the service-counter bumps NEVER touch the
//     Mutex — the source arms ignore the Stats entirely
//     (`Bump $ \_ tick ->` / `Bump $ \_ value ->`, :141/:147) — they
//     push to the glean-fb303 shim only.
//   * The row's "[Stat; 8]" sketch is homogeneous and cannot hold the
//     source's heterogeneous counters (7 x Stat Tick + 1 x
//     Stat LookupCache.StatValues); the row's substantive mandate is the
//     single-Mutex grouping, realized here as the typed StatsGroup
//     struct fields under one Mutex (the row's own "or" alternative
//     wording acknowledges the sketch latitude; the grouping — ONE lock
//     — is followed exactly).
//   * "Also exposed as envStats/envLookupCacheStats (Env row 8)":
//     Database/Types.hs:273 `envStats :: Stats` — the Env consumer is
//     manifest row 104's unit; `Stats` is Send + Sync here by
//     construction (Mutex<StatsGroup> + the Send+Sync Execute gate),
//     which is what sharing it across writer threads needs.
//
// ---------------------------------------------------------------------------
// THE DISSOLUTIONS (source machinery with no Rust image; each documented,
// none silent)
// ---------------------------------------------------------------------------
//
//   1. The `Ref` existential (:52-64): `data Ref = forall a. Monoid a =>
//      Ref (Stats -> IORef (Stat a))` with `refs :: [Ref]` — a list that
//      uniformly reaches all 8 IORefs so bump() can zero every
//      stat_new. The closed-set truth: exactly TWO monoid types occur
//      across the 8 refs (Tick x7, LookupCache.StatValues x1), so the
//      existential carries no information the struct doesn't. The Rust
//      image dissolves it: StatsGroup's typed fields ARE the refs, and
//      the :223-224 reset loop becomes `reset_all`'s eight field
//      assignments (one per ref, source order :56-63). The `Monoid a`
//      constraint becomes the private `Monoid` trait below with exactly
//      those two impls — the closed-enum treatment of the §2
//      GADTs/existential row ("enum where the variant set is closed"),
//      realized as a two-impl trait instead of an enum because the type
//      index never needs to be inspected at runtime (allStats formats
//      each typed field directly; see dissolution 3).
//   2. The `Initial` class (:200-209): `class Initial a where
//      initial :: IO a -> IO Stats` with the recursive instance
//      sequencing the 8 `newIORef (Stat mempty mempty)` allocations
//      behind the partially applied `Stats` constructor — an applicative
//      builder threading the IORefs into `Stats <$> atMostEvery period`
//      (:212). Rust needs no class: `new` is a plain constructor that
//      builds the Execute (at_most_every) and the Mutex<StatsGroup> with
//      every counter at mempty in one expression (the 8 IORef
//      allocations become the 8 struct fields of one StatsGroup value).
//      The `Monoid a` context of the second instance is dissolution 1's
//      trait; `initial = id` for `Stats` needs no counterpart.
//   3. The `allStats`/`counter`/`value` table (:155-196): the source
//      stores `(String, Stats -> IO String)` pairs whose closures pick
//      the formatter by value type; the private `counter`/`value` where-
//      fns are the two row shapes. Rust keeps the table as `log_line`'s
//      eleven entries in source order, with `counter_row`/`value_row`
//      the two shape helpers and the formatter a closure over the typed
//      field — the existential-free version of the same table (the
//      formatter's input type is static per row).
//   4. `mconcat` over `lookupCacheStats`'s list (:127): foldr (<>)
//      mempty of [bumper, 13 contramap'd counter bumps]; each `<>` arm
//      runs left-then-right and the arms are independent effects, so a
//      left fold with `Bump::combine` preserves the execution order
//      (bumper first, then the 13 counters in `ALL_STATS` discriminant
//      order = `[minBound .. maxBound]`).
//
// ---------------------------------------------------------------------------
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, grep-verified)
// ---------------------------------------------------------------------------
//
//   * OUTBOUND (5 in-repo rows, all consumed below):
//       edges.tsv:1304 Stats.hs -> github/ServiceData/GlobalStats.hs
//         (import :28; addStatValueType :143/:148, setCounter :153)
//       edges.tsv:1305 Stats.hs -> github/ServiceData/Types.hs
//         (import :29; Avg :143, Sum :148)
//       edges.tsv:1306 Stats.hs -> RTS/Foreign/LookupCache.hs
//         (qualified import :26; StatValues/Stat/getStat/isCounter/
//         the `show c` metric names — see SIBLING-UNIT CONTRACTS)
//       edges.tsv:1307 Stats.hs -> Util/Metric.hs (import :32)
//       edges.tsv:1308 Stats.hs -> Util/Throttle.hs (import :33;
//         the sole atMostEvery construction site in the tree)
//     plus `import Util.Log` (:30) — hsthrift's glog shim, NO manifest
//     unit (ASSUMED-IMPORT below).
//   * INBOUND (5 in-repo rows, all FUTURE units): edges.tsv:279
//     Database/Env.hs (envStats), :337 Database/Open.hs, :462
//     Database/Types.hs (envStats :: Stats at :273 — the row-82 Env
//     clause), :493 Database/Write/Batch.hs, :1883
//     Write/SendAndRebaseQueue.hs.
//
// ---------------------------------------------------------------------------
// ASSUMED-IMPORT (RULE §1 logging row (A10)): the `glean_util::log` seam
// ---------------------------------------------------------------------------
//
// `import Util.Log` (Stats.hs:30; `logInfo` :222) has NO manifest unit:
// Util.Log is hsthrift's glog shim
// (~/glean-build/Glean/hsthrift/common/util/Util/Log.hs:9-16,
// re-exporting Util.Log.String), where
// `logInfo :: (MonadIO m, HasCallStack) => String -> m ()`
// (Util/Log/String.hs:51-53) ALWAYS forces its message — unlike `vlog`
// there is no laziness to preserve, but the seam's pinned shape is the
// lazy-thunk form regardless:
//
//     pub fn log_info(msg: impl FnOnce() -> String);
//
// The module does not exist on disk yet. This file writes the FINAL call
// site against the pinned shape per DECISIONS 2026-10-18 (the
// units-45/46 direct call-site convention; no stand-in, no port todo):
// the first sketch is glean-db/src/query/thrift/internal.rs:190-213
// (unit 45, `vlog`), EXTENDED with `log_info` by
// glean-db/src/write/send_batch.rs:150-166 (unit 46) — this file only
// CONSUMES `log_info`, so no extension of the sketch is owed. The swap
// when glean-util's log unit lands is the import resolution only.
//
// Cross-crate dep edges (Step-4 Cargo.toml wiring — glean-core has no
// Cargo.toml yet, HANDOFF §2/§4.2): this file anticipates deps on
// glean-util (metric/throttle/log), glean-fb303 (service_data) and
// once_cell (the CAF statics below — whitelisted, RULE §1), the
// same comment-only anticipated-dep form glean-db's Cargo.toml records.
//
// ---------------------------------------------------------------------------
// SIBLING-UNIT CONTRACTS (DONE units' real APIs read, never re-declared;
// every signature below grep-verified in the shipped file)
// ---------------------------------------------------------------------------
//
//   * rust/glean-util/src/util/metric.rs (unit 99, row 99) — whose own
//     header pins THIS unit as the heaviest consumer (Stats.hs uses:
//     tickValue/tickMillis/tickLatencyNanoSecs/showLatency/showThroughput/
//     showMemory/showCount/pad/beginTick/endTick/Tick's <>):
//       Tick { pub tick_value: u64, pub tick_duration: Duration } +
//         `Tick::combine(self, other: Tick) -> Tick` (the Semigroup) +
//         `Default` (the Monoid; used for mempty at :67-68/:209/:224)
//       begin_tick(value: u64) -> Point / end_tick(Point) -> Tick (:227/:230)
//       tick_latency_nano_secs(&Tick) -> u128 (:142; u128 carry — the
//         fromIntegral narrowing below is THIS file's obligation, as
//         metric.rs's contract says)
//       tick_millis(&Tick) -> u64 (:110/:117/:124)
//       show_latency(&Tick) -> String / show_throughput(&Tick) -> String
//         (:157-:162; pad 5 / pad 9 baked in)
//       show_memory(u64) -> String / show_count(u64) -> String (:173/:176)
//       pad(usize, &str) -> String (:186, showMissRate's pad 5)
//     The `TimeSpec -> Duration` image for :211's period is pinned by
//     metric.rs's sibling contract for exactly this line.
//   * rust/glean-util/src/util/throttle.rs (unit 100, row 100) — whose
//     header pins THIS unit as the sole consumer:
//       `pub type Execute = Arc<dyn Fn(&mut dyn FnMut()) + Send + Sync>`
//         (the `statsLogger :: Execute` field image, :41)
//       `at_most_every(Duration) -> Execute` (:212)
//       the call-site shape `(stats.stats_logger)(&mut || { ... })`
//         (:218) — the inline closure-literal form that file
//         compile-probes.
//   * rust/glean-core/src/rts/foreign/lookup_cache.rs (unit 29, manifest row 29;
//     qualified as `lookup_cache` below, mirroring the source's
//     `qualified ... as LookupCache` import :26):
//       `StatValues` (private Vec<u64>, len ALWAYS STAT_COUNT) with
//         `zero()` (the Monoid mempty, :209/:224) and
//         `combine(&self, &StatValues) -> StatValues` (the Monoid <>,
//         :67-68) — that file's own header notes the Write/Stats
//         consumer.
//       `Stat` — the 13-variant closed enum whose `Display` renders the
//         Haskell constructor tags verbatim ("IdByKey_hits" ...),
//         BECAUSE this module builds the fb303 metric names from
//         `map toLower (show c)` (:132) — consumed here as
//         `format!("glean.db.write.cache.{}", c).to_ascii_lowercase()`
//         (the tags are ASCII, so per-char toLower == ascii lowercase).
//       `Stat::{IdByKeyHits, IdByKeyMisses, TypeByIdHits,
//         TypeByIdMisses, FactByIdHits, FactByIdMisses, FactBytes,
//         FactCount}` (the :163-177 showMissRate/`using` selectors)
//       `ALL_STATS: [Stat; STAT_COUNT]` (the `[minBound .. maxBound]`
//         image, :133)
//       `is_counter(Stat) -> bool` (:136 — add vs set)
//       `get_stat(&StatValues, Stat) -> u64` (:128/:192-193/:198)
//   * rust/glean-fb303/src/service_data/global_stats.rs (unit 97, row 97)
//     + types.rs (unit 96, row 96) — the OSS no-op shim, whose header
//     names this file among its consumers:
//       `add_stat_value_type(_name: &[u8], _value: i64, _ty: ExportType)`
//         (:143/:148 — the names are `Char8.pack`ed ByteStrings ->
//         &[u8] at the call; the shim's Int is i64)
//       `set_counter(_name: &[u8], _value: i64)` (:153) — the shim
//         source is `setCounter :: ByteString -> Int -> IO ()`, so the
//         source's `void $` wrapper (:153) discards nothing here.
//       `ExportType::{Sum, Avg}` (:143/:148)
//     Per the row-82 clause, these calls are OUT of the Mutex scope (the
//     arms never touch the group).
//
// ---------------------------------------------------------------------------
// NUMERIC NOTES (fromIntegral/wrap images; each grep- or probe-pinned)
// ---------------------------------------------------------------------------
//
//   * `ms = fromIntegral (tickLatencyNanoSecs tick `div` 1000000)` (:142):
//     the shipped tick_latency_nano_secs returns u128; the source's
//     Integer `div 1000000` truncates identically on u128, and
//     `as i64` narrows u128 -> i64 by truncating to the low 64 bits and
//     reinterpreting — fromIntegral's Integer->Int wrap semantics.
//   * `fromIntegral value` (:148/:153): Word64 -> Int — `u64 as i64` is
//     the bit-preserving two's-complement reinterpret (values above
//     i64::MAX go negative on both sides).
//   * `hits+misses` and `misses * 1000` (:186-189) are Word64 ring
//     arithmetic — `wrapping_add`/`wrapping_mul` reproduce the silent
//     wrap bug-for-bug. GHC-probed: showMissRate maxBound 2 renders
//     "200.0%" (hits+misses wrapped to 1 -> rate 2000), and
//     showMissRate (maxBound div 1000 + 1) 0 renders " 0.0%" (the wrap
//     row is (0, maxBound div 1000 + 1): misses*1000 wraps to 384 ->
//     rate 0; with misses=0 there is no *1000 wrap at all) — see the
//     probe record below. Same
//     disposition as metric.rs's Word64-wrap rows (reproduced exactly as
//     written; no defect stamp).
//   * showMissRate's percent form (:190): `show (rate div 10) ++ "." ++
//     show (rate mod 10) ++ "%"` — ONE fraction digit, truncated not
//     rounded (66.66..% renders "66.6%"); "-" on zero, pad 5.
//
// ---------------------------------------------------------------------------
// PROBE RECORD (pinned-toolchain evidence; scratch, never in-tree)
// ---------------------------------------------------------------------------
//
//   * GHC (the format oracle): /tmp/write-stats-probe/Probe.hs compiled
//     with the pinned GHC 9.4.7 against the REAL Glean.Util.Metric
//     (exported fns; -i ~/glean-build/Glean/glean/util -package-db
//     ~/.cabal/store/ghc-9.4.7/package.db -package clock-0.8.4), with
//     showMissRate copied VERBATIM from Stats.hs:186-193. Ground truth:
//     the 11-tag log line, the counter "new [all]" / value "new" shapes,
//     the miss-rate truncation + both Word64 wrap rows, and the 13
//     lowercased metric names ("glean.db.write.cache.idbykey_hits" ...
//     "glean.db.write.cache.fact_bytes"/"fact_count").
//   * Rust: /tmp/write-stats-probe/probe.rs — this file's private
//     formatting fns (show_miss_rate/counter_row/value_row/log_line
//     over a StatsGroup stand-in) + verbatim copies of the shipped
//     metric.rs fns, run over the SAME inputs: byte-identical to the GHC
//     output (diff empty; expected.txt retained beside it).
//   * Type check: /tmp/write-stats-probe/typecheck/probe_lib.rs — a /tmp
//     crate-root stubbing log/fb303/lookup_cache/once_cell (signature-
//     faithful stand-ins) and include!-ing the REAL shipped metric.rs and
//     throttle.rs, then this file: rustc --crate-type lib --emit=metadata
//     -D warnings is clean. It caught one real defect pre-review: the
//     Bump impl's missing 'static type-param bounds (fixed; see the impl
//     note). The repo file itself is never compiled (RULE §0).

use std::sync::{Arc, Mutex};
use std::time::Duration;

use glean_fb303::service_data::global_stats::{add_stat_value_type, set_counter};
use glean_fb303::service_data::types::ExportType;
// ASSUMED-IMPORT: see the header block — the `glean_util::log` seam,
// consumed at its pinned shape (the units-45/46 convention).
use glean_util::log::log_info;
use glean_util::util::metric::{
    begin_tick, end_tick, pad, show_count, show_latency, show_memory, show_throughput,
    tick_latency_nano_secs, tick_millis, Tick,
};
use glean_util::util::throttle::{at_most_every, Execute};
use once_cell::sync::Lazy;

use crate::rts::foreign::lookup_cache;

// ---------------------------------------------------------------------------
// Stat (Stats.hs:35-38) — private, like the source's unexported record
// ---------------------------------------------------------------------------

/// `data Stat a = Stat { statNew :: !a, statAll :: !a }` (:35-38) — the
/// current-window and since-start accumulators. NOT exported (the source
/// export list :9-15 names only Stats/new/Bump/bump/tick and the eight
/// bump constants); the fields are private for the same reason.
struct Stat<A> {
    stat_new: A,
    stat_all: A,
}

impl<A: Monoid> Stat<A> {
    /// `Stat mempty mempty` (:209, the Initial builder's seed).
    fn empty() -> Stat<A> {
        Stat {
            stat_new: A::mempty(),
            stat_all: A::mempty(),
        }
    }
}

/// The source's `Monoid a` constraint (:52/:66/:180) over exactly the two
/// element types that occur across the 8 refs (dissolution 1):
/// `Tick` x7 and `LookupCache.StatValues` x1.
trait Monoid: Sized {
    /// `mempty`.
    fn mempty() -> Self;
    /// `<>`.
    fn mappend(&self, other: &Self) -> Self;
}

impl Monoid for Tick {
    /// `instance Monoid Tick` (Metric.hs:28-29, mempty = Tick 0 0).
    fn mempty() -> Tick {
        Tick::default()
    }
    /// `instance Semigroup Tick` (Metric.hs:25-26) — Tick::combine.
    fn mappend(&self, other: &Tick) -> Tick {
        self.combine(*other)
    }
}

impl Monoid for lookup_cache::StatValues {
    /// `instance Monoid StatValues` (LookupCache.hs:84-85,
    /// mempty = V.replicate sTAT_COUNT 0).
    fn mempty() -> lookup_cache::StatValues {
        lookup_cache::StatValues::zero()
    }
    /// `instance Semigroup StatValues` (LookupCache.hs:80-82) —
    /// per-index add for counters, right-take for the sums.
    fn mappend(&self, other: &lookup_cache::StatValues) -> lookup_cache::StatValues {
        self.combine(other)
    }
}

// ---------------------------------------------------------------------------
// Stats + StatsGroup (Stats.hs:40-50) — the row-82 single-Mutex image
// ---------------------------------------------------------------------------

/// `data Stats` (:40-50). The eight `IORef (Stat a)` fields are the
/// inventory row's ONE `Mutex`-guarded StatsGroup: heterogeneous typed
/// counters (7 x `Stat<Tick>` + 1 x `Stat<StatValues>`) under a single
/// lock, so the gated log-and-reset is atomic vs bumps (the row's
/// "reads ALL then zeroes ALL, which must be atomic vs bumps").
///
/// `Stats` is `Send + Sync` by construction (a `Mutex` over plain data +
/// the `Send + Sync` [`Execute`] gate) — the shape sharing it as
/// `envStats :: Stats` (Database/Types.hs:273, Env row 8) needs; no
/// manual impls.
pub struct Stats {
    /// `statsLogger :: Execute` (:41) — the at-most-once-per-period gate.
    stats_logger: Execute,
    /// The 8 counters, one lock (inventory.tsv:82 == inventory.stm.tsv:33).
    group: Mutex<StatsGroup>,
}

/// The eight counters, in the source's field order (:42-49). Private.
struct StatsGroup {
    mutator_latency: Stat<Tick>,
    mutator_input: Stat<Tick>,
    mutator_throughput: Stat<Tick>,
    mutator_deduped_throughput: Stat<Tick>,
    mutator_dup_throughput: Stat<Tick>,
    rename_throughput: Stat<Tick>,
    commit_throughput: Stat<Tick>,
    lookup_cache: Stat<lookup_cache::StatValues>,
}

/// `new :: TimeSpec -> IO Stats` (:211-212) — `Duration` is the pinned
/// TimeSpec image (metric.rs's sibling contract names this exact line;
/// the sole construction site is Env.hs:121's `TimeSpec 10 0`).
///
/// The `Initial` class dissolves here (dissolution 2): the source's
/// applicative builder sequences the 8 `newIORef (Stat mempty mempty)`
/// allocations behind the partially applied constructor; the Rust
/// constructor builds the gate and the seeded group in one expression.
/// `IO` is dropped — no failure channel exists in the source.
pub fn new(period: Duration) -> Stats {
    Stats {
        stats_logger: at_most_every(period),
        group: Mutex::new(StatsGroup {
            mutator_latency: Stat::empty(),
            mutator_input: Stat::empty(),
            mutator_throughput: Stat::empty(),
            mutator_deduped_throughput: Stat::empty(),
            mutator_dup_throughput: Stat::empty(),
            rename_throughput: Stat::empty(),
            commit_throughput: Stat::empty(),
            lookup_cache: Stat::empty(),
        }),
    }
}

// ---------------------------------------------------------------------------
// Bump (Stats.hs:66-153) — the contravariant consumer handle
// ---------------------------------------------------------------------------

/// `newtype Bump a = Bump { runBump :: Stats -> a -> IO () }` (:72) —
/// "A metric that can be bumped using 'bump'. Metrics of the same type
/// can be bumped together using the `Monoid` instance." (:70-71)
///
/// The field is private: the source exports `Bump` without its
/// constructor (:9-15) — values are built by this module's four private
/// constructors (bumper + the three service-counter arms) feeding the
/// eight constants, and the Semigroup/Monoid/Contravariant instances
/// below are the only public composition surface.
///
/// The run closure is an `Arc` (not the `Box` a plain field would
/// suggest) because the eight constants are top-level CAFs shared across
/// writer threads: the Rust image is `once_cell::sync::Lazy` statics
/// (the subst.rs EMPTY / predicate.rs PREDICATE_INDICES CAF precedent),
/// which need `Sync` (forcing `+ Send + Sync` on any boxed form — probed
/// E0277), and the Env row-8 sharing gives the same Send+Sync demand the
/// throttle.rs contract records for the gate ("the GATE is Send+Sync
/// because Stats stores it in the shared Env"). Arc specifically is the
/// house closure image (throttle.rs/observed.rs), not forced by Sync.
/// Each combinator
/// closure captures only `Send + Sync` state (counter names, `Stat`
/// variants, other Arcs), so every `Bump<A>` is `Send + Sync` for every
/// `A` with no bounds.
pub struct Bump<A> {
    run_bump: Arc<dyn Fn(&Stats, &A) + Send + Sync>,
}

// `'static` on the impl's type params (A, and contramap's B): the boxed
// closure trait object is `+ 'static` (elided), so every type the closure
// mentions must outlive it — the HANDOFF unit-15 ruling ("boxed closure
// trait objects propagate 'static ... 'static bounds on every type
// param"). Every A in use is plain data (Tick/u64/StatValues).
impl<A: 'static> Bump<A> {
    /// `instance Semigroup (Bump a)` (:74-75): `Bump p <> Bump q = Bump $
    /// \stats a -> p stats a >> q stats a` — sequential composition;
    /// both arms see the same `a` (hence the `&A` threading below).
    pub fn combine(self, other: Bump<A>) -> Bump<A> {
        let p = self.run_bump;
        let q = other.run_bump;
        Bump {
            run_bump: Arc::new(move |stats: &Stats, a: &A| {
                p(stats, a);
                q(stats, a);
            }),
        }
    }

    /// `instance Monoid (Bump a)` (:77-78): `mempty = Bump $ const $
    /// const $ return ()`.
    pub fn empty() -> Bump<A> {
        Bump {
            run_bump: Arc::new(|_: &Stats, _: &A| {}),
        }
    }

    /// `instance Contravariant Bump` (:80-81): `contramap f (Bump g) =
    /// Bump $ \stats a -> g stats (f a)` — pre-compose the input
    /// projection (the inherent-method form; Rust has no Contravariant
    /// class to implement).
    pub fn contramap<B: 'static>(self, f: impl Fn(&B) -> A + Send + Sync + 'static) -> Bump<B> {
        let g = self.run_bump;
        Bump {
            run_bump: Arc::new(move |stats: &Stats, b: &B| g(stats, &f(b))),
        }
    }
}

/// `bumper :: Monoid a => (Stats -> IORef (Stat a)) -> Bump a` (:66-68) —
/// "lock + monoid-extend" per the row: take the group lock, extend BOTH
/// halves (`(Stat (new <> x) (all <> x), ())`), release. The field
/// selector stands in for the source's `Stats -> IORef (Stat a)`
/// accessor; it runs under the lock (the row's atomicity upgrade), so
/// the two-half extend is one critical section exactly as the source's
/// single `atomicModifyIORef'` was.
fn bumper<A: Monoid + 'static>(get: fn(&mut StatsGroup) -> &mut Stat<A>) -> Bump<A> {
    Bump {
        run_bump: Arc::new(move |stats: &Stats, x: &A| {
            let mut group = stats.group.lock().unwrap();
            let stat = get(&mut group);
            stat.stat_new = stat.stat_new.mappend(x);
            stat.stat_all = stat.stat_all.mappend(x);
        }),
    }
}

/// `bumpServiceCounterLatency :: ByteString -> Bump Tick` (:139-143) —
/// "Bump an fb303 counter representing latency". Ignores the Stats
/// entirely (`Bump $ \_ tick ->`): OUT of the Mutex scope (row-82
/// clause). `ms = fromIntegral (tickLatencyNanoSecs tick `div` 1000000)`
/// then `addStatValueType counter ms Avg`.
fn bump_service_counter_latency(counter: String) -> Bump<Tick> {
    Bump {
        run_bump: Arc::new(move |_stats: &Stats, tick: &Tick| {
            let ms = (tick_latency_nano_secs(tick) / 1_000_000) as i64;
            add_stat_value_type(counter.as_bytes(), ms, ExportType::Avg);
        }),
    }
}

/// `bumpServiceCounter :: ByteString -> Bump Word64` (:145-148) — "Bump
/// an fb303 counter": `addStatValueType counter (fromIntegral value) Sum`.
fn bump_service_counter(counter: String) -> Bump<u64> {
    Bump {
        run_bump: Arc::new(move |_stats: &Stats, value: &u64| {
            add_stat_value_type(counter.as_bytes(), *value as i64, ExportType::Sum);
        }),
    }
}

/// `setServiceCounter :: ByteString -> Bump Word64` (:150-153) — "Set an
/// fb303 counter": `void $ setCounter counter (fromIntegral value)`. The
/// OSS shim's `setCounter :: ByteString -> Int -> IO ()` discards
/// nothing, so there is no `void` counterpart.
fn set_service_counter(counter: String) -> Bump<u64> {
    Bump {
        run_bump: Arc::new(move |_stats: &Stats, value: &u64| {
            set_counter(counter.as_bytes(), *value as i64);
        }),
    }
}

// The eight exported constants (:84-127) as `Lazy` statics — the source's
// top-level CAFs (allocated once, shared; the subst.rs/predicate.rs
// image). Static case is the Rust convention (SCHEMA_SOURCE_DIR
// precedent); names otherwise match the source exports.

/// `mutatorLatency :: Bump Tick` (:83-87) — "The latency between a write
/// request being received, and it being completed."
pub static MUTATOR_LATENCY: Lazy<Bump<Tick>> = Lazy::new(|| {
    bumper(|g| &mut g.mutator_latency).combine(bump_service_counter_latency(
        "glean.db.write.latency_ms".to_string(),
    ))
});

/// `mutatorInput :: Bump Tick` (:95-97) — the accumulator only; no
/// service counter rides along.
pub static MUTATOR_INPUT: Lazy<Bump<Tick>> = Lazy::new(|| bumper(|g| &mut g.mutator_input));

/// `mutatorThroughput :: Bump Tick` (:89-93) — "The throughput of writes
/// in bytes per unit time".
pub static MUTATOR_THROUGHPUT: Lazy<Bump<Tick>> = Lazy::new(|| {
    bumper(|g| &mut g.mutator_throughput).combine(
        bump_service_counter("glean.db.write.throughput".to_string())
            .contramap(|tick: &Tick| tick.tick_value),
    )
});

/// `mutatorDedupedThroughput :: Bump Tick` (:99-103).
pub static MUTATOR_DEDUPED_THROUGHPUT: Lazy<Bump<Tick>> = Lazy::new(|| {
    bumper(|g| &mut g.mutator_deduped_throughput).combine(
        bump_service_counter("glean.db.write.deduped.bytes".to_string())
            .contramap(|tick: &Tick| tick.tick_value),
    )
});

/// `mutatorDupThroughput :: Bump Tick` (:105-110) — TWO service
/// counters: the undeduped bytes AND the elapsed ms.
pub static MUTATOR_DUP_THROUGHPUT: Lazy<Bump<Tick>> = Lazy::new(|| {
    bumper(|g| &mut g.mutator_dup_throughput)
        .combine(
            bump_service_counter("glean.db.write.undeduped.bytes".to_string())
                .contramap(|tick: &Tick| tick.tick_value),
        )
        .combine(
            bump_service_counter("glean.db.write.undeduped.ms".to_string()).contramap(tick_millis),
        )
});

/// `renameThroughput :: Bump Tick` (:112-117) — "The throughput of
/// renames in bytes per unit time".
pub static RENAME_THROUGHPUT: Lazy<Bump<Tick>> = Lazy::new(|| {
    bumper(|g| &mut g.rename_throughput)
        .combine(
            bump_service_counter("glean.db.write.rename.bytes".to_string())
                .contramap(|tick: &Tick| tick.tick_value),
        )
        .combine(
            bump_service_counter("glean.db.write.rename.ms".to_string()).contramap(tick_millis),
        )
});

/// `commitThroughput :: Bump Tick` (:119-124) — "The throughput of
/// commits in bytes per unit time".
pub static COMMIT_THROUGHPUT: Lazy<Bump<Tick>> = Lazy::new(|| {
    bumper(|g| &mut g.commit_throughput)
        .combine(
            bump_service_counter("glean.db.write.commit.bytes".to_string())
                .contramap(|tick: &Tick| tick.tick_value),
        )
        .combine(
            bump_service_counter("glean.db.write.commit.ms".to_string()).contramap(tick_millis),
        )
});

/// `lookupCacheStats :: Bump LookupCache.StatValues` (:126-128) — the
/// bumper PLUS one contramap'd fb303 push per `Stat` constructor
/// (`[minBound .. maxBound]` = `ALL_STATS`, :131-137): counters are
/// ADDED (`bumpServiceCounter`, `isCounter c`), the two sums are SET
/// (`setServiceCounter`); each name is
/// `"glean.db.write.cache." ++ map toLower (show c)` (:132). Left fold
/// with `combine` preserves `mconcat`'s execution order (dissolution 4).
pub static LOOKUP_CACHE_STATS: Lazy<Bump<lookup_cache::StatValues>> = Lazy::new(|| {
    let mut acc = bumper(|g| &mut g.lookup_cache);
    for &c in lookup_cache::ALL_STATS.iter() {
        // `bump c` (:135-137): the add-vs-set selector.
        let name = format!("glean.db.write.cache.{}", c).to_ascii_lowercase();
        let push = if lookup_cache::is_counter(c) {
            bump_service_counter(name)
        } else {
            set_service_counter(name)
        };
        acc =
            acc.combine(push.contramap(move |values: &lookup_cache::StatValues| {
                lookup_cache::get_stat(values, c)
            }));
    }
    acc
});

// ---------------------------------------------------------------------------
// bump / tick (Stats.hs:215-232) — the write path
// ---------------------------------------------------------------------------

/// `bump :: Stats -> Bump a -> a -> IO ()` (:214-224) — "Bump a metric":
/// run the bump (lock + monoid-extend, or the out-of-lock service-counter
/// push), then hand the gate the log-and-reset action (:218). The gate
/// decides whether the action runs (at most once per period); when it
/// runs, ONE critical section reads all eleven formatted entries
/// (allStats), emits the log line, and zeroes every `stat_new` — the
/// row's "reads ALL then zeroes ALL, which must be atomic vs bumps", in
/// the source's own statement order read -> log -> zero.
///
/// `a` is taken by reference: the Semigroup chain feeds the SAME value
/// to every arm (`p stats a >> q stats a`), and both `Tick` and
/// `StatValues` consumers only read it.
pub fn bump<A>(stats: &Stats, b: &Bump<A>, a: &A) {
    (b.run_bump)(stats, a);
    (stats.stats_logger)(&mut || {
        let mut group = stats.group.lock().unwrap();
        let line = log_line(&group);
        log_info(|| line);
        reset_all(&mut group);
    });
}

/// `tick :: Stats -> Bump Tick -> Word64 -> IO a -> IO a` (:226-232) —
/// bracket the action with [`begin_tick`]/[`end_tick`] and bump the
/// resulting `Tick`. `IO` is dropped (both clock reads are total; the
/// metric.rs precedent), `io` is a `FnOnce`.
pub fn tick<R>(stats: &Stats, b: &Bump<Tick>, val: u64, io: impl FnOnce() -> R) -> R {
    let point = begin_tick(val);
    let x = io();
    let tick = end_tick(point);
    bump(stats, b, &tick);
    x
}

// ---------------------------------------------------------------------------
// allStats + reset (Stats.hs:155-198, :223-224) — the log line
// ---------------------------------------------------------------------------

/// `allStats :: [(String, Stats -> IO String)]` (:155-178) — the eleven
/// entries in source order, formatted against the unlocked group.
///
/// The `counter` where-fn (:180-184) renders `"new [all]"`; the `value`
/// where-fn (:195-196) renders `"new"` only. The lookupCache rows reuse
/// ONE `Stat<StatValues>` field with different selectors — ibk/tbi/fbi
/// miss rates and the two `using`-projected sums (:198).
fn log_line(group: &StatsGroup) -> String {
    let entries = [
        counter_row("mut_lat", &group.mutator_latency, show_latency),
        counter_row("mut_thp", &group.mutator_throughput, show_throughput),
        counter_row(
            "ded_thp",
            &group.mutator_deduped_throughput,
            show_throughput,
        ),
        counter_row("dup_thp", &group.mutator_dup_throughput, show_throughput),
        counter_row("rnm_thp", &group.rename_throughput, show_throughput),
        counter_row("cmt_thp", &group.commit_throughput, show_throughput),
        counter_row("ibk_mis", &group.lookup_cache, |values| {
            show_miss_rate(
                values,
                lookup_cache::Stat::IdByKeyHits,
                lookup_cache::Stat::IdByKeyMisses,
            )
        }),
        counter_row("tbi_mis", &group.lookup_cache, |values| {
            show_miss_rate(
                values,
                lookup_cache::Stat::TypeByIdHits,
                lookup_cache::Stat::TypeByIdMisses,
            )
        }),
        counter_row("fbi_mis", &group.lookup_cache, |values| {
            show_miss_rate(
                values,
                lookup_cache::Stat::FactByIdHits,
                lookup_cache::Stat::FactByIdMisses,
            )
        }),
        value_row("lch_mem", &group.lookup_cache, |values| {
            show_memory(lookup_cache::get_stat(
                values,
                lookup_cache::Stat::FactBytes,
            ))
        }),
        value_row("lch_cnt", &group.lookup_cache, |values| {
            show_count(lookup_cache::get_stat(
                values,
                lookup_cache::Stat::FactCount,
            ))
        }),
    ];
    entries.join(" ")
}

/// `counter shw get stats` (:180-184):
/// `concat [shw $ statNew stat, " [", shw $ statAll stat, "]"]`, prefixed
/// by the tag (`unwords`'s element form, :221 — `tag ++ ": " ++ s`).
fn counter_row<A, F>(tag: &str, stat: &Stat<A>, shw: F) -> String
where
    F: Fn(&A) -> String,
{
    format!("{}: {} [{}]", tag, shw(&stat.stat_new), shw(&stat.stat_all))
}

/// `value shw get stats` (:195-196): `shw . statNew` — no bracket.
fn value_row<A, F>(tag: &str, stat: &Stat<A>, shw: F) -> String
where
    F: Fn(&A) -> String,
{
    format!("{}: {}", tag, shw(&stat.stat_new))
}

/// `showMissRate h m` (:186-193) over one `StatValues` snapshot:
/// `pad 5 $ case hits+misses of 0 -> "-"; n -> ...` with the one-digit
/// truncated percent (`show (rate div 10) ++ "." ++ show (rate mod 10) ++
/// "%"`). The `where`-bound hits/misses read the snapshot by selector
/// (:192-193). Both Word64 ring sites wrap (NUMERIC NOTES; probed).
fn show_miss_rate(
    values: &lookup_cache::StatValues,
    h: lookup_cache::Stat,
    m: lookup_cache::Stat,
) -> String {
    let hits = lookup_cache::get_stat(values, h);
    let misses = lookup_cache::get_stat(values, m);
    let padded = match hits.wrapping_add(misses) {
        0 => "-".to_string(),
        n => {
            let rate = misses.wrapping_mul(1000) / n;
            format!("{}.{}%", rate / 10, rate % 10)
        }
    };
    pad(5, &padded)
}

/// The `refs` reset (:223-224) with the `Ref` existential dissolved
/// (dissolution 1): one assignment per ref, source order (:56-63), each
/// `statNew = mempty`. `stat_all` is NEVER reset (since-start), and
/// `mutator_input` is reset although it has no allStats row — the reset
/// is driven by the refs list (:54-64), not by the log table.
fn reset_all(group: &mut StatsGroup) {
    group.mutator_latency.stat_new = Tick::default();
    group.mutator_input.stat_new = Tick::default();
    group.mutator_throughput.stat_new = Tick::default();
    group.mutator_deduped_throughput.stat_new = Tick::default();
    group.mutator_dup_throughput.stat_new = Tick::default();
    group.rename_throughput.stat_new = Tick::default();
    group.commit_throughput.stat_new = Tick::default();
    group.lookup_cache.stat_new = lookup_cache::StatValues::zero();
}

// Marker breakdown (closed set — zero of every class):
//   * no open port TODOs: every source export is translated (export
//     parity below); the one absent sibling surface (glean_util::log) is
//     an ASSUMED-IMPORT consumed at its pinned shape per DECISIONS
//     2026-10-18 — no stand-in, no todo; the crate-root `pub mod write;`
//     line is write.rs's own Step-4 contract, not this file's.
//   * no BUG(port) reproductions: the Word64 ring wraps (hits+misses,
//     misses*1000) and the divide-by-zero latent in
//     tick_latency_nano_secs are the shipped metric.rs/lookup_cache.rs
//     surfaces' own probe-verified behaviours, reproduced exactly as
//     written (the metric.rs precedent declines the defect stamp for
//     ring arithmetic).
//   * no PERF deferrals: every path is the direct form; the Lazy statics
//     are the CAF-faithful allocation-once shape, not a deferral.
//   * no unsafe (RULE §3: none needed — the state is a Mutex over plain
//     data; no FFI in this unit) and no inventory divergence (row 82 ==
//     stm:33 followed clause by clause; the homogeneous "[Stat; 8]"
//     sketch detail is realized as the row's own StatsGroup-under-one-
//     Mutex mandate with typed fields — see the INVENTORY block).
//
// Export parity (Stats.hs:9-15 -> this file): Stats (pub, opaque),
// new, Bump (pub, opaque), bump, tick, mutatorLatency -> MUTATOR_LATENCY,
// mutatorInput -> MUTATOR_INPUT, mutatorThroughput -> MUTATOR_THROUGHPUT,
// mutatorDedupedThroughput -> MUTATOR_DEDUPED_THROUGHPUT,
// mutatorDupThroughput -> MUTATOR_DUP_THROUGHPUT,
// renameThroughput -> RENAME_THROUGHPUT, commitThroughput ->
// COMMIT_THROUGHPUT, lookupCacheStats -> LOOKUP_CACHE_STATS (13/13; the
// eight constants are statics, the static-case image of top-level CAFs).
// The Semigroup/Monoid/Contravariant instances on Bump ship as the pub
// combine/empty/contramap methods — instances are part of the type's
// public surface in Haskell even though the constructor is not exported.
// PORT STATUS: confidence=high todos=0
