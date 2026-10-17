// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// A configurable wrapper for retrying Thrift requests that may fail with
// ChannelException (the source module haddock, RetryChannelException.hs:9-12).
//
// Ported from: glean/util/Glean/Util/RetryChannelException.hs (75 lines)
//
// Manifest unit: rust/glean-util/src/util/retry_channel_exception.rs
// (migration/manifest.tsv line 42, crate glean-util). RULE §4: one source
// file -> one target file; the manifest path is authoritative.
//
// Export surface (the source export list, :13-17, is exactly
// `RetryPolicy(..)`, `defaultRetryPolicy`, `retryChannelExceptions`;
// logRetry (:47-52) and retryDuration (:71-75) are internal):
//   RetryPolicy(..)          -> `pub struct RetryPolicy` with pub fields
//                               (record selectors are exported by `(..)`);
//   defaultRetryPolicy       -> `pub fn default_retry_policy()` (the
//                               source's top-level VALUE; a function, not
//                               an `impl Default`, because the source has
//                               no Data.Default instance here and the
//                               callback field is a boxed closure — no
//                               const constructor);
//   retryChannelExceptions   -> `pub fn retry_channel_exceptions`.
//
// INVENTORY (RULE §5): NO inventory row governs this file — grep
// "RetryChannelException" across all six migration/inventory*.tsv returns
// ZERO rows whose subject is this module. The nearest rows constrain the
// taxonomy this file participates in but govern OTHER files:
//   * inventory.tsv:13 == inventory.protocol.tsv:35 (EXC_TAXONOMY,
//     subject Glean/Haxl.hs): "retryChannelExceptions re-runs the WHOLE
//     runHaxl on ChannelException only"; target "an outer retry policy
//     scoped to a distinct ChannelError variant". The distinct-variant
//     half is satisfied by thrift_service.rs's `ThriftError::Channel`
//     leg; the outer-policy half is row 154's (Haxl.rs) to implement —
//     cited, not implemented here (see SIBLING-UNIT CONTRACTS).
//   * inventory.stm.tsv:32 == inventory.tsv:120 (SendQueue state) mentions
//     retry-on-failure requeue at the queue layer — a different mechanism
//     (in-process requeue, not this RPC retry wrapper).
// No INVENTORY-BUG-CANDIDATE: no row can diverge from a file no row
// governs. The source is the spec (RULE §0).
//
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, module-level): ZERO
// outbound edges (the source's imports are all external packages or
// no-manifest modules — disposed below); inbound rows 16
// (Glean/Backend/Retry.hs, manifest 43), 92 (Glean/Haxl.hs, manifest
// 154), 105 (Glean/Remote.hs, manifest 167), 1867 (Glean/Write/Options.hs,
// manifest 160), 1889 (Glean/Write/SendQueue.hs, manifest 157). Every
// source import has a disposition (RetryChannelException.hs:19-25):
//   * Control.Exception (:19; `try` :58, pattern :61, `throwIO` :64)
//     -> the Result channel: the retry loop returns
//     `Result<A, ThriftError>`; the catch is a match on ONE variant, the
//     rethrow is an Err return (deliberate — see the loop's comments and
//     RULE §2's dynamic-exceptions row);
//   * Text.Printf (:20) -> `format!` (the message TEXT is parity-bearing:
//     RULE §1 logging row + §2 Show-tag row; mapped at `log_retry`);
//   * System.Random (:21; `randomRIO` :73) -> an IN-FILE splitmix64 (RULE
//     §1 third-party row: whitelist has no rand crate) — see the
//     TODO(port) at the PRNG;
//   * System.Time.Extra (:22; `Seconds` :29-30, `sleep` :68) -> the
//     library-cited fact `type Seconds = Double` (extra-1.8,
//     src/System/Time/Extra.hs:26, from this build tree's cabal cache)
//     -> plain `f64` fields
//     (no alias is forked here: no sibling has sketched the
//     System.Time.Extra surface — grep over rust/ returns nothing — and
//     f64 is the whole type); `sleep delay` ->
//     `std::thread::sleep(Duration::from_secs_f64(delay))`;
//   * Util.Log (:23; `logWarning` :50) -> ASSUMED-IMPORT #2 below (the
//     future `glean_util::log` owner, A10) — consumed through a local
//     stand-in with the pinned signature;
//   * Thrift.Channel (:25; `ChannelException` :32, :61) ->
//     ASSUMED-IMPORT #1 below (the `glean_thrift` sketch, first drawn by
//     unit 40) — consumed, not re-sketched.
//
// ---------------------------------------------------------------------------
// ASSUMED-IMPORT (RULE §4 B4) #1: the `glean_thrift` error surface.
//
// FIRST SKETCH OWNER: rust/glean-util/src/util/thrift_service.rs:260-309
// (manifest row 40). This file is the sketch's named consumer — its
// comment at :268-270 says "Remote.hs:307-312 pattern-matches it and
// RetryChannelException.hs:54-69 retries on it only", which the source
// confirms: :61 matches `Left ex@ChannelException{}` and no other
// exception type appears anywhere in the module. No extension of the
// surface is made here: exactly two items are consumed,
//     pub enum ThriftError { Channel(ChannelException), Protocol(..),
//                            Application(..), Service(..) }   // :264-292
//     pub struct ChannelException(pub String)                  // :293-294
// imported flat at the crate root per the sketch's convention
// (thrift_service.rs:215-216, "same as service.rs:153"). Pinned evidence
// recorded there, restated for this file's load-bearing parts:
//   * Channel.hs:81-83 `newtype ChannelException = ChannelException Text
//     deriving (Show, Eq)` + `instance Exception` (sketch :201-202);
//   * the Show text is parity-bearing HERE: logRetry formats `show ex`
//     (:49), and GHC renders the newtype as `ChannelException "msg"`
//     (probe quoted at thrift_service.rs:270-272). The sketch's
//     constructor-shaped Display (:302-306, `ChannelException {:?}`)
//     produces exactly that text for printable payloads (probed
//     identical, including quote/backslash escaping and the %.2f vs
//     `{:.2}` tie rounding — both half-even), so `format!("{}", ex)` is
//     the `show ex` image; control-character payloads can diverge
//     (Haskell `\SOH`-style vs Rust `\u{...}` escapes) — cosmetic for
//     transport strings. No re-derivation of Display in this file.
// The catch/rethrow semantics against this enum are the loop's own and
// are documented at the match.
//
// ---------------------------------------------------------------------------
// ASSUMED-IMPORT (RULE §4 B4 + A10) #2: the `Util.Log` surface.
//
// `import Util.Log` (RetryChannelException.hs:23) has NO manifest unit:
// Util.Log is hsthrift's glog shim
// (~/glean-build/Glean/hsthrift/common/util/Util/Log.hs:9-16, re-exporting
// Util.Log.String). A10's pinned ruling (RULEBOOK §1 logging row) owns the
// image as a `glean_util::log` module "with LAZY message thunks (message
// never forced unless verbose)"; that module does not exist yet — this
// file is its FIRST CONSUMER, and per the unit brief's sanctioned form the
// surface is recorded HERE and consumed through a private stand-in
// (`log_warning`, at the call site below) with A10's pinned signature
// `impl FnOnce() -> String`, NOT through a phantom import (the module is
// not on disk; RULE §4's UNKNOWN rule). When the owner lands
// (rust/glean-util/src/log.rs), the stand-in is deleted and replaced by
// `use crate::log::log_warning;` — a one-line swap BECAUSE the signature
// is already the pinned one. Pinned evidence for the severity's behavior:
// Util.Log.String.hs:56-58 `logWarning msg = liftIO $ logCommon
// c_glog_warning (getCaller ...) msg` — glog WARNING to stderr
// (String.hs:12 module comment), and the message is forced on EVERY call
// (logCommon runs unconditionally; String.hs:32-35 scopes message
// laziness to `vlog` only), so the thunk is semantically inert at WARNING
// severity — the parity-bearing part is the TEXT, which `log_retry`
// builds.
//
// ---------------------------------------------------------------------------
// SIBLING-UNIT CONTRACTS
//
//   * thrift_service.rs (row 40, SHIPPED): owns the ThriftError/
//     ChannelException sketch this file consumes (ASSUMED-IMPORT #1). Its
//     transport/IO-error-leg TODO(port) (thrift_service.rs:311-322) notes
//     the retry policy is ChannelException-scoped — this file is the
//     mechanism that makes that scoping observable: non-Channel legs are
//     never retried, here or anywhere.
//   * rust/glean-db/src/backend/retry.rs (row 43, NEXT unit): the
//     heaviest consumer — Backend/Retry.hs:39-103 wraps every backend
//     operation in `retryChannelExceptions policy $ ...` (grep: 21 call
//     sites). It supplies its own RetryPolicy; nothing here changes for
//     it.
//   * rust/glean-client/src/haxl.rs (row 154) + the row-146 data-source
//     cluster (Haxl/DataSource/Glean/Common.hs -> rust/glean-db/src/
//     haxl/data_source/glean/common.rs): the Haxl consumer — Haxl.hs:55
//     imports (RetryPolicy, retryChannelExceptions), Haxl.hs:104-105
//     `runHaxlWithRetry ... = retryChannelExceptions policy $ runHaxl
//     backend u h`, re-running the WHOLE haxl run per attempt. The
//     BatchCtx error model it nests inside is inventory.tsv:13 ==
//     inventory.protocol.tsv:35's EXC_TAXONOMY row — that row's Rust half
//     is theirs; the distinct Channel variant it scopes on is
//     `ThriftError::Channel` (sketch, above).
//   * rust/glean-client/src/remote.rs (row 167): re-exports
//     defaultRetryPolicy (Remote.hs:39) and feeds it to
//     backendRetryReads/backendRetryWrites; its ChannelException fallback
//     (Remote.hs:303-312, `try sharded` + `show e`) is the caller
//     thrift_service.rs's header cites — a DIFFERENT catch site from this
//     file's retry loop (Remote catches once and falls back or rethrows;
//     this file retries N times first).
//   * rust/glean-client/src/write/options.rs (row 160):
//     Write/Options.hs:66 `toRetryPolicy n = defaultRetryPolicy
//     { maxRetries = n }` — the record-update image is struct update
//     syntax, `RetryPolicy { max_retries: n, ..default_retry_policy() }`
//     (works with the boxed callback: the Box moves).
//   * rust/glean-client/src/write/send_queue.rs (row 157):
//     SendQueue.hs:342 stores `defaultRetryPolicy` verbatim as
//     sendQueueRetry.
//
// ---------------------------------------------------------------------------

use std::cell::RefCell;
use std::thread_local;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use glean_thrift::{ChannelException, ThriftError};

// ---------------------------------------------------------------------------
// RetryPolicy (RetryChannelException.hs:27-33)
// ---------------------------------------------------------------------------

/// The configurable retry policy (RetryChannelException.hs:27-33).
///
/// Field mapping: `maxRetries :: Int` -> `i64` (LP64); `minRetryDelay ::
/// Seconds` / `maxRetryDelay :: Seconds` -> `f64` (`type Seconds = Double`,
/// extra-1.8 src/System/Time/Extra.hs:26 — the library cite; NOTE these
/// are Seconds fields,
/// NOT Int); `retryJitter :: Double` -> `f64` ("fraction of delay", :31);
/// `onError :: ChannelException -> Int -> Int -> Maybe Double -> IO ()` ->
/// `Box<dyn FnMut(&ChannelException, i64, i64, Option<f64>)>` — the
/// argument order is the source's (:32, called at :63/:67). `Option<f64>`
/// per the sibling's Maybe-Double image (thrift_service.rs:72-74,
/// decision (a)).
///
/// Why a boxed callback and not a generic parameter: the source is a
/// MONOMORPHIC record type whose exported value `defaultRetryPolicy` has
/// exactly this type (:38-45) — a generic parameter would infect the whole
/// surface; and the callback set is genuinely open (any `IO ()` closure —
/// RULE §2's GADT row permits `Box<dyn Trait>` only where genuinely open,
/// which a function field is). `FnMut` rather than `Fn`: an `IO ()`
/// closure may capture mutable state (IORef); `log_retry` is a plain `Fn`
/// and every `Fn` implements `FnMut`, so the default satisfies the bound.
/// The callback takes `&ChannelException` (Rust borrow plumbing, as the
/// sibling's `&Service` receivers); the exhaustion path still carries the
/// ORIGINAL value out via the return (see the loop). The object type
/// keeps the default `+ 'static` bound, and that is unobservable against
/// the source: the ONLY `onError` value in the tree is the capture-free
/// top-level `logRetry` (grep over glean/ + glass/: no site constructs
/// `RetryPolicy` from scratch and no record update touches `onError` —
/// the sole construction site is Write/Options.hs:66, which changes
/// `maxRetries` only), so no Haskell closure ever captures a callback
/// environment that a `'static` bound would have to own.
pub struct RetryPolicy {
    /// Give up after this many failed attempts (:28). The loop makes
    /// exactly `maxRetries + 1` attempts (the source's log denominator,
    /// :49).
    pub max_retries: i64,
    /// Lower delay bound, in seconds (:29).
    pub min_retry_delay: f64,
    /// Upper delay bound, in seconds (:30).
    pub max_retry_delay: f64,
    /// Fraction of delay the jitter may swing either way (:31).
    pub retry_jitter: f64,
    /// Invoked on every failure (:32): the exception, the attempt number
    /// (1-based), `max_retries`, and `Some(delay)` when a retry follows /
    /// `None` when this failure is final.
    pub on_error: Box<dyn FnMut(&ChannelException, i64, i64, Option<f64>)>,
}

/// Sized to survive a write-server restart (host move + multi-GB database
/// download before it can serve again): 30 retries with a 60s cap
/// (1,2,4,8,16,32 then 60s) ~= 25 min. (RetryChannelException.hs:35-45,
/// transcribed; the arithmetic is the spec and checks out — rustc probe
/// /tmp/opencode/rce-probe/retry_probe.rs: attempts 1..6 sleep
/// 1,2,4,8,16,32s at zero jitter, attempts 7..30 ride the 60s cap, total
/// 1503s = 25.05 min ~= "25 min".)
///
/// The source exports this as a top-level VALUE (:38-45); the image is a
/// function because the boxed-callback field has no const constructor
/// (see the struct's docs for why the field is boxed at all).
pub fn default_retry_policy() -> RetryPolicy {
    RetryPolicy {
        max_retries: 30,
        min_retry_delay: 1.0,
        max_retry_delay: 60.0,
        retry_jitter: 0.2, // 20% (:43)
        on_error: Box::new(log_retry),
    }
}

// ---------------------------------------------------------------------------
// logRetry (RetryChannelException.hs:47-52) — internal, not exported
// ---------------------------------------------------------------------------

/// The default `on_error`: a printf-formatted warning (:47-52).
///
/// Text is parity-bearing (RULE §1 logging row + §2 Show-tag row):
/// :49 `printf "failed (%s) (try %d/%d)" (show ex) n (maxRetries+1)` ->
/// `format!("failed ({}) (try {}/{})", ex, n, max_retries + 1)`, where
/// `format!("{}", ex)` reproduces `show ex` via the sketch's
/// constructor-shaped Display (`ChannelException "msg"`, ASSUMED-IMPORT
/// #1); :52 `printf ", retry after %.2fs" delay` ->
/// `format!(", retry after {:.2}s", delay)` (both sides correctly round
/// the same binary64, so the two-decimal text matches). The suffix is
/// appended after the main message (:50-52), empty when the failure is
/// final.
fn log_retry(ex: &ChannelException, n: i64, max_retries: i64, maybe_delay: Option<f64>) {
    let mut msg = format!("failed ({}) (try {}/{})", ex, n, max_retries + 1);
    match maybe_delay {
        None => {}
        Some(delay) => {
            msg.push_str(&format!(", retry after {:.2}s", delay));
        }
    }
    log_warning(move || msg);
}

// ---------------------------------------------------------------------------
// The log seam — stand-in pending the future glean_util::log owner
// (ASSUMED-IMPORT #2; the unit-brief-sanctioned form (a): the call site
// reads naturally with a stand-in and the signature is already A10's
// pinned surface, so the swap is a one-line import change)
// ---------------------------------------------------------------------------

// TODO(port): stand-in for Util.Log's logWarning — the owner is the future
// `glean_util::log` module (A10, RULE §1 logging row; no manifest unit yet;
// this file is its first consumer). When it lands at
// rust/glean-util/src/log.rs, delete this fn and add
// `use crate::log::log_warning;` — the signature is already the pinned
// one. The stand-in has no glog format or verbosity gate (the real one
// emits glog WARNING to stderr with caller file/line,
// Util.Log.String.hs:48-50); it forces the message exactly as the Haskell
// logWarning does (String.hs:33-35 scopes laziness to `vlog`, WARNING
// always forces), so no semantics are deferred — only the sink/format,
// which is the log module's own parity obligation, not this file's.
fn log_warning(msg: impl FnOnce() -> String) {
    eprintln!("{}", msg());
}

// ---------------------------------------------------------------------------
// The retry loop (RetryChannelException.hs:54-69)
// ---------------------------------------------------------------------------

/// Retry `act` per `r`'s policy while it fails with a ChannelException
/// (:54-69).
///
/// Signature image: `RetryPolicy -> IO a -> IO a` where the `IO` THROWS —
/// under RULE §2's dynamic-exceptions row the typed enum replaces throws
/// with `Err` returns (the unit-41 `try_bracket` precedent,
/// rust/glean-db/src/backend/types.rs:725, and the sibling's runner
/// decision, thrift_service.rs:121-127), so the surface is
/// `Result<A, ThriftError>` and no caller can observe "returned vs threw".
///
/// The action image: `act :: IO a` is EXECUTED once per attempt (`try act`
/// at :58 runs it afresh every time `go` fires), so the Rust parameter is
/// `impl FnMut() -> Result<A, ThriftError>` — the caller closes over
/// whatever makes the attempt repeatable. A caller holding a single-use
/// `ThriftAction` (thrift_service.rs:223-232's CONTRACT) wraps it as
/// `|| action.clone().run(env)` and clones once per attempt at ITS call
/// site — the same clone-once discipline that contract already pins.
pub fn retry_channel_exceptions<A>(
    r: RetryPolicy,
    mut act: impl FnMut() -> Result<A, ThriftError>,
) -> Result<A, ThriftError> {
    // `r@RetryPolicy{..}` (:55): the whole record AND its fields. The
    // destructure moves the boxed callback out; `act` stays a disjoint
    // local, so both are callable inside the loop.
    let RetryPolicy {
        max_retries,
        min_retry_delay,
        max_retry_delay,
        retry_jitter,
        mut on_error,
    } = r;

    // `go 1` (:55): n is the 1-based attempt number.
    let mut n: i64 = 1;
    loop {
        // `e <- try act` (:58): `try` with the :61 pattern catches ONLY
        // ChannelException — ANY other exception propagates immediately,
        // no retry, no onError. The match mirrors that exactly: only the
        // `ThriftError::Channel` leg is acted on; every other leg returns
        // UNTOUCHED (same variant, same payload), so upstream classifiers
        // (inventory.tsv:13's BatchCtx model, Remote.hs:303-312's
        // fallback) still see the full error.
        match act() {
            Ok(x) => return Ok(x),
            Err(ThriftError::Channel(ex)) => {
                if n > max_retries {
                    // Exhausted (:62-64): onError with NO delay, THEN
                    // rethrow. The source's `throwIO ex` rethrows the
                    // ORIGINAL exception object; under the typed-enum
                    // rule the rethrow IS this Err return, carrying the
                    // original ChannelException value — deliberate, not a
                    // divergence.
                    on_error(&ex, n, max_retries, None);
                    return Err(ThriftError::Channel(ex));
                } else {
                    // Retry (:65-69) — ORDER IS SEMANTIC: compute the
                    // delay FIRST, then onError with Some(delay), then
                    // sleep, then the next attempt.
                    let delay = retry_duration(
                        min_retry_delay,
                        max_retry_delay,
                        retry_jitter,
                        n,
                    );
                    on_error(&ex, n, max_retries, Some(delay));
                    // `sleep delay` (:68); Seconds = f64 ->
                    // Duration::from_secs_f64.
                    std::thread::sleep(Duration::from_secs_f64(delay));
                    n += 1;
                }
            }
            Err(other) => return Err(other),
        }
    }
}

// ---------------------------------------------------------------------------
// retryDuration (RetryChannelException.hs:71-75) — internal, not exported
// ---------------------------------------------------------------------------

/// Exponential backoff with multiplicative jitter (:71-75):
/// `exp randomJitter * minRetryDelay * 2^(n-1)`, clamped to
/// `[minRetryDelay, maxRetryDelay]`.
///
/// Fidelity notes: :74 is left-associative — `((exp j) * min) *
/// 2^(n-1)` — and the Rust chain keeps that association. :75 clamps AFTER
/// the multiply (`max minRetryDelay $ min maxRetryDelay dur`): the bounds
/// are SECONDS, the jitter is a FRACTION (exp(±jitter) ∈ [0.819, 1.221]
/// at the default 0.2), and BOTH clamp directions are load-bearing — the
/// rustc probe (/tmp/opencode/rce-probe/retry_probe.rs) shows attempt 1
/// clamping UP from 0.819 and attempts 7+ clamping DOWN from up to
/// 78.17s. The `$!` on the return is strictness that is immaterial for a
/// non-bottom f64. `2^(n-1)`: the base is the Double literal 2, the
/// exponent Haskell `Int` -> `f64::powi((n - 1) as i32)`; with base 2.0
/// every intermediate product is exactly representable, so powi is exact
/// here (n - 1 <= maxRetries << 1024).
///
/// The source takes the record by value and destructures it (:72); the
/// image takes the three fields it actually uses — the policy is shared,
/// not consumed (the caller keeps using it in the loop above), so the
/// borrow costs nothing the source doesn't.
fn retry_duration(min_retry_delay: f64, max_retry_delay: f64, retry_jitter: f64, n: i64) -> f64 {
    let random_jitter = random_rio(-retry_jitter, retry_jitter);
    // 2^(n-1) :: Double overflows to Infinity for n-1 >= 1024, where the
    // source's clamp pins 60; powi's i32 exponent would WRAP at n-1 >=
    // 2^31 (probed: 2^32 -> 0), so the exponent is pinned at the overflow
    // boundary — powi(1023) finite, powi(1024) = inf, both -> 60 after
    // the clamp below.
    let dur = random_jitter.exp() * min_retry_delay * 2f64.powi(((n - 1).min(1023)) as i32);
    dur.min(max_retry_delay).max(min_retry_delay)
}

// ---------------------------------------------------------------------------
// The PRNG image of System.Random (:21, `randomRIO` :73)
// ---------------------------------------------------------------------------

// TODO(port): System.Random's global system-seeded StdGen replaced by an
// in-file splitmix64 (whitelist has no rand crate); jitter-distribution
// equivalence is the claim, exact stream parity is NOT.
//
// Notes: the source's generator is AMBIENT (`randomRIO` takes no
// generator argument — the global StdGen), so the image is an ambient
// thread-local whose state advances per call, seeded once from the wall
// clock. Thread-local rather than process-global-Mutex: only the
// statistical distribution is observable here (one jitter per backoff
// delay), and the pinned TODO already disclaims stream parity; std-only
// keeps RULE §1's whitelist intact (no once_cell needed — thread_local
// is std).

/// Splitmix64 — self-contained, 64-bit state, one multiply-xor chain per
/// output. Seeded from SystemTime's nanos (the image of System.Random's
/// system-seeded global StdGen).
struct SplitMix64(u64);

impl SplitMix64 {
    fn new_from_time() -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        SplitMix64(nanos)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in [0, 1): the top 53 mantissa bits over 2^53 — exactly
    /// representable steps, no skew toward 1.
    fn next_unit(&mut self) -> f64 {
        ((self.next_u64() >> 11) as f64) / ((1u64 << 53) as f64)
    }
}

thread_local! {
    /// The ambient generator (the global-StdGen image; see the TODO(port)
    /// above). Initialized on the thread's first jitter draw.
    static RNG: RefCell<SplitMix64> = RefCell::new(SplitMix64::new_from_time());
}

/// `randomRIO (lo, hi)` (:73) — uniform over the CLOSED-OPEN range
/// [lo, hi): the unit draw is mapped onto the signed range as
/// `lo + u * (hi - lo)`, the honest [0,1) -> (-j, +j) affine map (no
/// range-subset pitfalls: every interior value is reachable; the support
/// is [lo,hi) where System.Random's `randomR` is closed [lo,hi] — the
/// upper endpoint needs a unit draw of exactly 1.0, probability 2^-64 —
/// and per-draw rounding differs at the ulp level; both covered by the
/// disclaimed stream parity). Called as
/// `randomRIO (-retryJitter, retryJitter)`, i.e. lo < hi always.
fn random_rio(lo: f64, hi: f64) -> f64 {
    RNG.with(|rng| lo + rng.borrow_mut().next_unit() * (hi - lo))
}

// ===========================================================================
// Coverage / markers / status
// ===========================================================================
//
// Coverage: the 75-line source is fully accounted for — the record
// (:27-33) -> `RetryPolicy` (all five fields, pub per `(..)`); the default
// value + its doc comment (:35-45) -> `default_retry_policy` (arithmetic
// probe-verified); logRetry (:47-52) -> `log_retry` with the
// printf-to-format! text mapping; the retry loop (:54-69) ->
// `retry_channel_exceptions` (catch-only-Channel, ordered
// onError/sleep/next, exhaustion onError-then-rethrow); retryDuration
// (:71-75) -> `retry_duration` (association, clamp order, powi image).
// The import block (:19-25) is disposed in DEPENDENCY TRUTH; nothing is
// dropped, stubbed, or deferred beyond the two flagged surfaces (PRNG,
// log sink) whose owners are named at their TODOs.
//
// Marker breakdown (closed set — exactly two open port todos, zero
// others):
//   * TODO(port) x2: (1) the PRNG stand-in (splitmix64 for
//     System.Random's global StdGen — RULE §1 whitelist has no rand
//     crate; pinned wording at the PRNG block); (2) the log sink stand-in
//     (the future `glean_util::log` owner per A10; signature already
//     pinned, swap is one line). Both name their owners.
//   * no source-defect reproductions: the source is total — no partial
//     functions, no `error`, no lazy-IO; the only throw is the caught-
//     and-rethrown ChannelException (:64), which is the specified
//     behavior (the exhaustion path), not a defect.
//   * no performance deferrals: one splitmix64 advance per delay is the
//     source's own cost shape (one StdGen draw); the sleep dominates by
//     design.
//   * no inventory divergence: no row governs this file (INVENTORY
//     block); inventory.tsv:13 == inventory.protocol.tsv:35 and
//     inventory.tsv:120 == inventory.stm.tsv:32 are cited as neighboring
//     constraints on OTHER files and are followed where they touch this
//     one (the distinct-variant scoping is `ThriftError::Channel`).
//
// PORT STATUS: confidence=high todos=2
