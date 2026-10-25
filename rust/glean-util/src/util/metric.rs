// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// Ported from: glean/util/Glean/Util/Metric.hs (94 lines).
// Manifest unit: rust/glean-util/src/util/metric.rs
// (migration/manifest.tsv line 99, crate glean-util). RULE §4: one source
// file -> one target file; the manifest path is authoritative.
//
// WHAT THIS FILE IS: the metric-formatting utilities behind
// Stats.hs's "mut_lat"/"mut_thp"/"lch_mem" tags. Two timing records
// (Tick — a value plus a monotonic-clock span, with the Semigroup/Monoid
// used to accumulate them; Point — a value plus the tick's start), the
// begin/end tick family over two monotonic reads, and the five text
// surfaces (showLatency, showThroughput, showMemory, showCount and the
// shared showScaled/pad plumbing) whose output text is exact: every
// threshold, truncation and padding byte below was scratch-probed against
// the pinned GHC 9.4.7 running the real Glean.Util.Metric (see
// TRANSLATION NOTES / DIVERGENCES for the boundary matrix).
//
// ---------------------------------------------------------------------------
// INVENTORY (RULE §5; migration/inventory*.tsv)
// ---------------------------------------------------------------------------
//
// ZERO rows name this source file. Exact commands at the repo root:
//   $ grep -n 'Util/Metric' migration/inventory*.tsv
//   $ grep -n 'Metric.hs' migration/inventory*.tsv
//   (no output; exit status 1 for both)
// A construct-level search is likewise empty — nothing here is an
// unsafePerformIO site, a CAF, STM, laziness-at-a-boundary or an FFI site:
//   $ grep -n 'showLatency\|showThroughput\|showScaled\|beginTick' migration/inventory*.tsv
//   (no output)
// so no inventory decision governs any line of this file; the mappings
// below are this file's own calls under RULE §1/§2 (Time row, numeric-
// conversions row). No INVENTORY-BUG-CANDIDATE applies.
//
// ---------------------------------------------------------------------------
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, checked not assumed)
// ---------------------------------------------------------------------------
//
//   * OUTBOUND: none in-repo. The source's imports are Data.List (find),
//     Data.Word (boot libraries) and System.Clock — the external `clock`
//     package (clock-0.8.4 in the pinned build's cabal store,
//     ~/.cabal/store/ghc-9.4.7/package.db), which has NO manifest unit;
//     RULE §1's Time row maps it onto std::time (see THE CLOCK IMAGE).
//     edges.tsv has no outbound row for Metric.hs — consistent: no
//     in-repo unit is imported.
//   * INBOUND (6), all FUTURE units (none shipped as of this file):
//       edges.tsv:463  Database/Types.hs imports `Glean.Util.Metric (Point)`
//         (:67) and holds `writeQueueLatency :: TVar TimeSpec` (:194).
//       edges.tsv:494  Database/Write/Batch.hs imports unqualified (:54);
//         beginTick at :69, endTick at :140 (feeds Stats.mutatorLatency).
//       edges.tsv:506  Database/Write/Queue.hs imports `beginTick` (:43);
//         beginTick 1 at :88; also `import System.Clock (TimeSpec,
//         toNanoSecs)` (:25) and `Maybe TimeSpec` (:173).
//       edges.tsv:520  Database/Writes.hs imports unqualified (:82).
//       edges.tsv:1307 Write/Stats.hs imports unqualified (:32) — the
//         heaviest consumer (see SIBLING-UNIT CONTRACTS).
//       edges.tsv:1884 Write/SendAndRebaseQueue.hs imports unqualified
//         (:43); beginTick at :315/:473, endTick at :337/:447.
//
// ---------------------------------------------------------------------------
// ASSUMED IMPORTS (RULE §4 B4)
// ---------------------------------------------------------------------------
//
//   * NONE — std::time is std; nothing generated or cross-crate is
//     involved. The `clock` package image is std::time per RULE §1, not an
//     assumed generated surface.
//
// ---------------------------------------------------------------------------
// SIBLING-UNIT CONTRACTS (what the six consumers call; all grep-verified)
// ---------------------------------------------------------------------------
//
//   * Write/Stats.hs:32 imports the module unqualified and uses:
//       tickValue        (:93-:124 contramap tickValue)  -> Tick.tick_value
//       tickMillis       (:110/:117/:124)                  -> tick_millis
//       tickLatencyNanoSecs (:142 — `fromIntegral (tickLatencyNanoSecs
//         tick `div` 1000000)`)                      -> tick_latency_nano_secs
//         Stats.hs then narrows to its own counter width; the u128 -> ? cast
//         is THAT unit's obligation, not this file's.
//       showLatency / showThroughput (:157-:162)           -> show_latency /
//         show_throughput
//       showMemory / showCount (:173/:176)                 -> show_memory /
//         show_count
//       pad 5 (:186, showMissRate)                         -> pad
//       beginTick/endTick (:228-:230)                      -> begin_tick /
//         end_tick
//       Tick's <> (bumper, :66-68 `Stat (new <> x) (all <> x)`)
//                                                    -> Tick::combine
//   * Database/Types.hs:67 imports ONLY Point (the write-queue latency
//     tracker); its `TVar TimeSpec` (:194) becomes `TVar Duration` in that
//     unit — TimeSpec has no manifest unit of its own, Duration is the
//     sanctioned image (RULE §1 Time row; same for Queue.hs:173's
//     `Maybe TimeSpec` and Stats.hs:211's `new :: TimeSpec -> IO Stats`).
//   * Database/Write/Batch.hs:54, Database/Writes.hs:82,
//     Write/SendAndRebaseQueue.hs:43 import unqualified and use only the
//     begin/end family (cites in DEPENDENCY TRUTH) plus the Tick Monoid.
//   * All latency call sites begin with value 1 (Queue.hs:88, Batch.hs:69,
//     SendAndRebaseQueue.hs:315) or a real byte size (:473) — see the
//     divide-by-zero note in TRANSLATION NOTES.
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
//   Source (System.Clock, clock-0.8.4 — the .hsc is the exact package in
//   the pinned store)        Target                       Evidence
//   -----------------------  ---------------------------  -----------------
//   `getTime Monotonic`      `std::time::Instant::now()`  clock-0.8.4
//       (:206-219 clock_gettime CLOCK_MONOTONIC)          System/Clock.hsc
//                                                         :219; Instant is
//                                                         the monotonic
//                                                         image per the
//                                                         row above
//   `now - start`            `now.duration_since(start)`  both operands are
//       (:80, :85 — Num.- on TimeSpec,                    monotonic reads;
//     normalizing via divMod)                             see DIVERGENCES 1
//                                                         for inversion
//                                                         semantics
//   `TimeSpec { sec, nsec }` fields of `Duration`        Duration is
//     (Int64; hsc :232-235)   (as_secs()/subsec_nanos())  always normalized
//                                                         like TimeSpec's
//                                                         `normalize`
//                                                         (hsc :266-269)
//   TimeSpec `+`/`-` (Num,   Duration's Add/Sub          exact on
//     hsc :271-274)                                       u64-secs/u32-nanos
//                                                         for non-negative
//                                                         operands
//   `toNanoSecs` (hsc :322-  `Duration::as_nanos()`      u128 — the image
//     324, Integer result)                                 of the source's
//                                                          Integer; see the
//                                                          carry decision in
//                                                          TRANSLATION NOTES
//   `sec time` / `nsec time` `as_secs()` /                 tickMillis (:65-67)
//     (hsc :232-235)          `subsec_nanos()`            arithmetic exact
//                                                         on u64
//   `mempty = Tick 0 0`      `impl Default for Tick`      crate convention
//     (:28-29)                (Duration::ZERO)            (util/trace.rs
//                                                         DIVERGENCES 2,
//                                                         glean-db logger/
//                                                         database.rs:77)
//   `instance Semigroup Tick`  `pub fn combine(self,     fieldwise; Word64
//     (:25-26)                  other) -> Tick`           add wraps (see
//                                                           DIVERGENCES 3)
//
//   System.Clock exposes NO Semigroup/Monoid instance for TimeSpec in
//   clock-0.8.4 (Show IS derived — Clock.hsc:235 `deriving (Generic,
//   Read, Show, Typeable)` — but Metric.hs never shows a TimeSpec) — Metric.hs's
//   `d1+d2` (:26) is the Num.+ above and `0` (:29) is fromInteger 0 =
//   TimeSpec 0 0 (hsc :280).
//
// ---------------------------------------------------------------------------
// TRANSLATION NOTES
// ---------------------------------------------------------------------------
//
//   * Record strictness: `!Word64`/`!TimeSpec` (:21-22, :70-71) are strict
//     fields — Rust fields are already strict; nothing to stage.
//   * showLatency (:31-39): `pad 5 . f`; `f (Tick 0 _) = "-"` guards on the
//     VALUE field only — a non-zero value over a zero duration renders
//     "0ns" (probed: GHC "  0ns", port identical); the ladder is
//     `n <= 2000` -> ns; `<= 2000000` -> us (n div 1000); `<= 2000000000`
//     -> ms (n div 1000000); otherwise s (n div 1000000000) — INTEGER
//     divisions, truncating (probed: 2001 -> "2us", 2001000 -> "2ms",
//     2000000001 -> "2s"). The ladder runs on u128, the carry decision
//     below.
//   * Carry type of tickLatencyNanoSecs (:41-43): the source computes
//     `toNanoSecs time div fromIntegral value` in Integer (unbounded).
//     The port returns u128: `as_nanos()` is u128 and is EXACT for every
//     representable Duration (Duration's range is u64 secs + u32 nanos =
//     < 2^104 ns, inside u128), so no Duration input can lose bits, and
//     u128 floor division by value > 0 equals Integer floor division
//     exactly. A u64 carry would truncate for single ticks spanning over
//     2^64 ns (~584 years) — representable by Duration, so truncation
//     would be reachable in principle; u128 closes that. showLatency's
//     ladder consumes it in u128; consumers (Stats.hs:142) cast down
//     explicitly at their site.
//   * showScaled (:45-51), private (not in the source export list :9-14):
//     `find ((value >=) . fst) scale` = FIRST key the value is >= —
//     deciscale is descending [(1e9,"G"),(1e6,"M"),(1e3,"K")] (:62-63), so
//     find picks the LARGEST key <= value; below all keys it is `show
//     value`. The two-digit form is ONE fraction digit, truncated not
//     rounded: `n = (value*10) div k`, rendered `show (n div 10) ++ "." ++
//     show (n mod 10) ++ sfx` (probed: 1234 -> "1.2K", 1500 -> "1.5K",
//     999999 -> "999.9K", 1001 -> "1.0K" — no rounding anywhere). `value *
//     10` is Word64 arithmetic and wraps silently in the source (probed:
//     value = maxBound div 10 + 1 -> "0.0G"); reproduced with
//     wrapping_mul (DIVERGENCES 3).
//   * showThroughput (:53-60): `ms == 0` renders "-" — value 0 with ms > 0
//     renders "0B/s" (probed: Tick 0 / 1s -> "0B/s", Tick 100 / 0ms ->
//     "-"; the guard asymmetry with showLatency is the source's, kept).
//     `rate = (tickValue * 1000) div ms` — Word64 wrap reproduced with
//     wrapping_mul (probed: Tick maxBound / 1ms -> "1844674407.3GB/s" on
//     both sides). The `++ "B/s"` suffix (:57) and showMemory's
//     `++ "B"` (:88) are exact.
//   * tickMillis (:65-67): `sec time * 1000 + nsec time div 1000000` then
//     fromIntegral to Word64 — `as_secs() * 1000 + (subsec_nanos() /
//     1_000_000) as u64` (probed: 1999999 ns -> 1 ms, 999999 ns -> 0 ms;
//     truncating, exact on u64). The source computes the intermediate in
//     Int64 (TimeSpec's field type): a span over 2^63/1000 seconds would
//     wrap negative there and fromIntegral would reinterpret it as a huge
//     Word64; u64 arithmetic stays exact up to 2^64/1000 s (~584 million
//     years). Unreachable either way — see DIVERGENCES 4.
//   * pad (:93-94): `replicate (n - length s) ' ' <> s`. Haskell's
//     replicate with a negative count is "" (probed against GHC 9.4.7:
//     pad 2 "abc" -> "abc", no error, no truncation) — `saturating_sub`
//     reproduces exactly. `length` counts code points; all current inputs
//     are ASCII (digits, unit suffixes, Stats.hs:186's percent strings),
//     `s.chars().count()` keeps even non-ASCII inputs faithful. The source
//     takes Int; all call sites pass positive literals (5 here, 9 here,
//     5 at Stats.hs:186), so usize loses nothing (a negative n would
//     behave like 0 — same output as Haskell's replicate-empty).
//   * beginTick/endTick/endTicks (:74-85): IO is dropped — Instant::now()
//     is total and non-failing, no Result channel exists in the source
//     either. endTicks reads ONE `now` for the whole list (:84) — the port
//     does the same, so a list of points shares one read (probed: length
//     and value preservation only; spans are clock-dependent).
//   * `error` sites: none in the source; the one partial operation is the
//     division in tickLatencyNanoSecs (:43). A Tick with value 0 makes the
//     source throw ArithException "divide by zero" (probed), and the port
//     panics with Rust's "attempt to divide by zero" — same kind, on the
//     same input, different message text (GHC's message is the runtime's,
//     not an `error` call with an author-chosen string, so the
//     panic-with-same-message rule has nothing to reproduce verbatim).
//     showLatency guards Tick 0 _ before dividing (:33), and every current
//     latency call site begins with value 1, so the throw is latent —
//     recorded here rather than emulated away or "fixed" (bug-for-bug).
//   * Point/Tick derive Clone/Copy/Debug (Haskell records are freely
//     copied; Debug is Rust diagnostics only, not a Show mapping — the
//     source has NO Show on either record, so none is exposed here).

// ---------------------------------------------------------------------------
// DIVERGENCES (numbered; none observable on reachable inputs)
// ---------------------------------------------------------------------------
//
//   1. `endTick`'s `now - start` (:80): TimeSpec subtraction normalizes
//      with floor division (hsc :272-273) and could represent a NEGATIVE
//      span if the clock jumped backwards; the port uses
//      `Instant::duration_since`, which saturates to zero on inversion
//      (std since 1.60; it panicked before). Unreachable: both operands
//      are CLOCK_MONOTONIC reads, which POSIX defines as never going
//      backwards ("cannot have negative clock jumps", clock-0.8.4
//      System/Clock.hsc :60-66) — so where the port saturates the source
//      would compute a positive normalized span anyway. The saturating
//      form is the house-safe one (no panic on a platform-clock bug).
//   2. Accumulated-span overflow: Tick::combine's duration add panics on
//      Duration overflow (> u64 secs) where TimeSpec's Num.+ wraps its
//      Int64 fields silently (> 2^63 ns, hsc :271-272) — the source would
//      keep counting, silently wrong; the port aborts. Unreachable: a
//      the wrap is the SEC FIELD (hsc :272 `TimeSpec (xs + ys) (xn + yn)`
//      — seconds, not nanoseconds), so the source wraps past 2^63 SECONDS
//      (~292 billion years); the port's Duration Add panics past u64 secs
//      (~2^64 s ≈ 584 billion years) — in the 2^63s-2^64s window the port
//      is EXACT where the source wraps. Unreachable either way.
//   3. Word64 ring arithmetic: the source's `v1+v2` (:26), `value * 10`
//      (:48) and `tickValue * 1000` (:60) wrap silently at 2^64. The port
//      uses wrapping_add/wrapping_mul at exactly those three sites to
//      reproduce the wrap bug-for-bug (probed rows: value*10 at
//      maxBound div 10 + 1 -> "0.0G"; throughput wrap row -> identical
//      text on both sides). Plain `*`/`+` would panic in debug builds
//      where the source renders a number.
//   4. tickMillis' intermediate width: the source multiplies in Int64
//      (TimeSpec's sec field) and fromIntegrals the possibly-negative
//      result to Word64; the port multiplies in u64. Identical for every
//      span under 2^63/1000 s (~292 million years); beyond that the
//      two wrap differently. Emulating a signed intermediate to reproduce
//      behavior 292 million years out is not worth the code — the simpler
//      total path is kept (precedent: util/disk.rs DIVERGENCES 1).
//
// The boundary matrix itself (78 deterministic rows: the ladder at
// 2000/2001, 2000000/2000001, 2000000000/2000000001, 2^64 ns; showScaled
// at 0/1/999/1000/1001/1234/1500/999999/1000000/1234567/999999999/1e9/
// 1e9+1 and the three wrap rows; throughput's zero-value/zero-dur
// asymmetry; tickMillis' sub-millisecond truncation; pad's
// negative-replication; the Monoid rows; divide-by-zero on value 0) was
// run against GHC 9.4.7 compiling the REAL Glean.Util.Metric (+ a verbatim
// copy of the private :45-51/:62-63 for showScaled/deciscale, which are
// not exported) and against this module's probe copy: byte-identical
// output, diff empty.

use std::time::{Duration, Instant};

/// 'data Tick' (:20-23): a measured value (bytes, facts, …) together with
/// the monotonic span it was measured over.
#[derive(Clone, Copy, Debug)]
pub struct Tick {
    pub tick_value: u64,
    pub tick_duration: Duration,
}

impl Tick {
    /// 'instance Semigroup Tick' (:25-26): fieldwise addition. The value
    /// add wraps like Word64 (DIVERGENCES 3); the duration add is
    /// Duration's own Add (exact for non-negative operands; DIVERGENCES 2
    /// for the overflow story).
    pub fn combine(self, other: Tick) -> Tick {
        Tick {
            tick_value: self.tick_value.wrapping_add(other.tick_value),
            tick_duration: self.tick_duration + other.tick_duration,
        }
    }
}

impl Default for Tick {
    /// 'instance Monoid Tick' (:28-29): mempty = Tick 0 0.
    fn default() -> Tick {
        Tick {
            tick_value: 0,
            tick_duration: Duration::ZERO,
        }
    }
}

/// 'data Point' (:69-72): a value plus the tick's start instant.
#[derive(Clone, Copy, Debug)]
pub struct Point {
    pub point_value: u64,
    pub point_start: Instant,
}

/// 'beginTick :: Word64 -> IO Point' (:74-75).
pub fn begin_tick(value: u64) -> Point {
    Point {
        point_value: value,
        point_start: Instant::now(),
    }
}

/// 'endTick :: Point -> IO Tick' (:77-80).
pub fn end_tick(point: Point) -> Tick {
    let now = Instant::now();
    Tick {
        tick_value: point.point_value,
        tick_duration: now.duration_since(point.point_start),
    }
}

/// 'endTicks :: [Point] -> IO [Tick]' (:82-85): ONE monotonic read for the
/// whole list (:83), each span taken against it.
pub fn end_ticks(ticks: Vec<Point>) -> Vec<Tick> {
    let now = Instant::now();
    ticks
        .into_iter()
        .map(|point| Tick {
            tick_value: point.point_value,
            tick_duration: now.duration_since(point.point_start),
        })
        .collect()
}

/// 'tickLatencyNanoSecs :: Tick -> Integer' (:41-43): the mean nanoseconds
/// per unit, Integer division (truncating) on both sides. Panics on value
/// 0 exactly where the source throws "divide by zero" (TRANSLATION NOTES).
pub fn tick_latency_nano_secs(tick: &Tick) -> u128 {
    tick.tick_duration.as_nanos() / tick.tick_value as u128
}

/// 'showLatency :: Tick -> String' (:31-39).
pub fn show_latency(tick: &Tick) -> String {
    pad(5, &show_latency_unpadded(tick))
}

/// 'f' (:33-34): the "-" guard is on the VALUE field only — a non-zero
/// value over a zero duration falls through to the ladder ("0ns").
fn show_latency_unpadded(tick: &Tick) -> String {
    if tick.tick_value == 0 {
        return "-".to_string();
    }
    display_latency(tick_latency_nano_secs(tick))
}

/// 'display n' (:35-39): the threshold ladder with integer divisions
/// (truncating), u128 so no representable span truncates on the way in.
fn display_latency(n: u128) -> String {
    if n <= 2000 {
        format!("{}ns", n)
    } else if n <= 2000000 {
        format!("{}us", n / 1000)
    } else if n <= 2000000000 {
        format!("{}ms", n / 1000000)
    } else {
        format!("{}s", n / 1000000000)
    }
}

/// 'showScaled :: [(Word64, String)] -> Word64 -> String' (:45-51).
/// Private: not in the source's export list (:9-14).
fn show_scaled(scale: &[(u64, &str)], value: u64) -> String {
    match scale.iter().find(|(k, _)| value >= *k) {
        Some((k, sfx)) => {
            // n = (value * 10) div k; rendered as one truncated fraction
            // digit: show (n div 10) ++ "." ++ show (n mod 10) ++ sfx.
            let n = value.wrapping_mul(10) / k;
            format!("{}.{}{}", n / 10, n % 10, sfx)
        }
        None => value.to_string(),
    }
}

/// 'showThroughput :: Tick -> String' (:53-60).
pub fn show_throughput(tick: &Tick) -> String {
    pad(9, &show_throughput_unpadded(tick))
}

/// 'f' (:55-60) with its where-bindings ms/rate inlined: "-" is ms == 0
/// (NOT value == 0); the value*1000 wrap is the source's Word64 ring
/// (DIVERGENCES 3).
fn show_throughput_unpadded(tick: &Tick) -> String {
    let ms = tick_millis(tick);
    if ms == 0 {
        return "-".to_string();
    }
    let rate = tick.tick_value.wrapping_mul(1000) / ms;
    format!("{}B/s", show_scaled(DECISCALE, rate))
}

/// 'deciscale :: [(Word64, String)]' (:62-63), descending; find picks the
/// FIRST (largest) key the value is >=. Private: not exported.
const DECISCALE: &[(u64, &str)] =
    &[(1000 * 1000 * 1000, "G"), (1000 * 1000, "M"), (1000, "K")];

/// 'tickMillis :: Tick -> Word64' (:65-67).
pub fn tick_millis(tick: &Tick) -> u64 {
    tick.tick_duration.as_secs() * 1000 + (tick.tick_duration.subsec_nanos() / 1_000_000) as u64
}

/// 'showMemory :: Word64 -> String' (:87-88).
pub fn show_memory(n: u64) -> String {
    format!("{}B", show_scaled(DECISCALE, n))
}

/// 'showCount :: Word64 -> String' (:90-91).
pub fn show_count(n: u64) -> String {
    show_scaled(DECISCALE, n)
}

/// 'pad :: Int -> String -> String' (:93-94): LEFT-pad with spaces to n
/// code points; never truncates — Haskell's replicate of a negative count
/// is "" (probed), so saturating_sub is the faithful form.
pub fn pad(n: usize, s: &str) -> String {
    format!("{}{}", " ".repeat(n.saturating_sub(s.chars().count())), s)
}

// Marker breakdown (closed set): zero port todos (no deferred surface —
// every export is translated above), zero BUG(port) reproductions (the
// three hazard behaviors — Word64 wraps, divide-by-zero on value 0, the
// "-"-guard asymmetry — are recorded in TRANSLATION NOTES/DIVERGENCES with
// repros; none stamps BUG because each is reproduced exactly as written
// and none is an author-defect, a judgment call flagged for reviewers in
// the unit report), zero PERF deferrals (the probe-verified form IS the
// direct form), zero inventory divergences (zero inventory rows), zero
// unsafe blocks.
//
// PORT STATUS: confidence=high todos=0
