// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// Ported from: glean/backend-api/Glean/Write/SendBatch.hs (136 lines)
// Manifest unit: rust/glean-db/src/write/send_batch.rs (migration/manifest.tsv
// line 46, crate glean-db). RULE §4: one source file -> one target file.
//
// Purpose (module haddock :9-11): "This layer provides synchronous writing on
// top of the lower-level asynchronous write API, by doing polling and waiting
// as necessary." Six exports (:13-20), all pub here: send_batch_and_wait
// (:31-39), send_batch (:41-53), send_json_batch_and_wait (:55-68),
// send_json_batch (:70-90), send_batch_descriptor (:92-115), wait_batch
// (:122-136); the local helper `retry` (:117-120) is private.
//
// ---------------------------------------------------------------------------
// THE TRY-TYPE-DIRECTEDNESS TABLE (the central fidelity point)
// ---------------------------------------------------------------------------
//
// Haskell `try` is TYPE-DIRECTED: each `try` annotation fixes which exception
// class is caught, and every other exception is rethrown. The port must
// therefore reproduce a DIFFERENT catch set per function:
//
//   * send_batch (:41-53): NO `try` AT ALL. `enqueueBatch` errors propagate
//     untouched — including a declared `Retry` exception. The only retry
//     signal is the union arm `SendResponse_retry (BatchRetry r)` (:52-53):
//     sleep, then tail-recurse. Rust: `backend.enqueue_batch(...)?` — the `?`
//     propagates `BackendError::Retry` too. This is deliberate and
//     non-obvious: adding a Retry catch here would diverge from the source.
//   * send_json_batch (:70-90): `try @Retry` — the case at :85-88 fixes the
//     try's type to `Either Thrift.Retry _`, so it catches ONLY the declared
//     `Retry` and reads its `retry_seconds`; every other error (Exception,
//     UnknownDatabase, channel/protocol, BatchAlreadyProcessed) propagates.
//     Rust: match `Err(BackendError::Retry(r))`; `Err(e) => return Err(e)`.
//   * wait_batch (:122-136): `try @Retry` — catches ONLY `Retry`; on success
//     the union `FinishResponse` is matched: `Subst subst` is returned,
//     `Retry (BatchRetry r)` sleeps and tail-recurses. BOTH retry sources
//     (exception :131-132, union arm :135-136) sleep then recurse;
//     non-Retry errors propagate.
//   * send_batch_descriptor (:92-115): `try @SomeException` + `fromException`
//     classification IN GUARD ORDER (:108-115): (1) `BatchAlreadyProcessed`
//     -> log the message (:110) and `return ""` (an EMPTY Handle, :111);
//     (2) `Retry` -> sleep + tail-recurse (:112-114); (3) otherwise
//     `throwIO e` (:115). Rust: the three-way match below, same order.
//   * send_batch_and_wait (:31-39) / send_json_batch_and_wait (:55-68): no
//     `try` of their own; they inherit their callees' catch sets.
//   * send_json_batch_and_wait's empty-handle branch (:64-68): `Text.null
//     handle` -> `handle.is_empty()`; the old-server fallback returns `def`
//     (Data.Default) -> `thrift::Subst::default()`.
//
// ---------------------------------------------------------------------------
// Loops, recursion, per-attempt clones
// ---------------------------------------------------------------------------
//
// Every recursive call is a TAIL call (`retry r $ sendBatch ...` :53;
// `retry retry_seconds $ ...` :89-90, :113-114, :131-132; `retry r $
// waitBatch ...` :135-136) -> Rust `loop`, matching the house `until_done`
// shape (backend/types.rs:1350-1365): no stack growth.
//
// Owned thrift arguments are rebuilt/cloned per attempt — `repo`, `batch`,
// `batches`, `opts`, `descriptor`, `handle` — exactly as the source
// re-passes the same pure value through its recursion, and exactly the
// clone-per-attempt precedent at backend/types.rs:1334-1338 (the generated
// call convention takes thrift arguments by value, types.rs:437-438).
//
// ---------------------------------------------------------------------------
// The sleep image (System.Time.Extra) and the NaN note
// ---------------------------------------------------------------------------
//
// `retry secs action = sleep (max secs 1) >> action` (:117-120). `sleep` is
// System.Time.Extra.sleep (pinned source, extra-1.8/src/System/Time/
// Extra.hs:26 `type Seconds = Double`, :31-41):
//
//     loopM $ \s -> if s < 0 then return () else
//                   if s > 2000 then threadDelay 2000000000 >> loop (s - 2000)
//                               else threadDelay (ceiling (s * 1000000))
//
// `threadDelay` takes MICROseconds, so 2000000000 = 2000 s per chunk; the
// chunking of large values is exact. The Rust helpers `sleep`/`retry` below
// implement this directly and deliberately do NOT use
// `Duration::from_secs_f64` (it panics on NaN and huge/inf inputs).
//
// `max secs 1` subtlety: Haskell's `max` is the Ord method, i.e. `if x <= y
// then y else x`; for `max NaN 1` that is NaN (GHC probe planprobe.hs,
// `max:nan NaN`). Rust `f64::max(NaN, 1.0)` returns 1.0 — a divergence — so
// the port ships the `if secs <= 1.0 { 1.0 } else { secs }` form, which
// reproduces Haskell `max` for ALL inputs, NaN included.
//
// NaN note (NO observable divergence): at the source's call site the
// instantiation is `ceiling :: Double -> Int` (`threadDelay :: Int -> IO
// ()`), and for NaN GHC's `ceilingDoubleInt` yields minBound on the pinned
// 64-bit compiler (runtime probe nan_runtime: -9223372036854775808, no
// exception — base-4.17.2.0 GHC/Float/RealFracMethods.hs:209-213); a
// non-positive `threadDelay` argument runs the callback IMMEDIATELY
// (GHC/Event/TimerManager.hs:218 `if us <= 0 then cb`; measured in
// single-digit microseconds), and Rust's saturating `as u64` cast yields 0
// — an immediate return too. Both sides proceed to the next attempt at
// once; only a NaN wire value can reach it. (The ~400-digit garbage
// integer the earlier probes quoted is the `:: Integer` instantiation,
// which the source's call site never uses.) The conversion is annotated
// at the site below.
//
// ---------------------------------------------------------------------------
// BatchAlreadyProcessed: the `fromException` image
// ---------------------------------------------------------------------------
//
// send_batch_descriptor catches the IDL-declared `BatchAlreadyProcessed`
// exception via `fromException e :: Maybe BatchAlreadyProcessed` (:109).
// Declared exceptions ride the generated crate's OPEN leg,
// `thrift::ThriftError::Service(Box<dyn Error + Send + Sync>)` — the
// encoding shipped by backend/retry.rs (:334-353: a declared exception that
// is not the Channel leg is carried inside a `Service` box and restored
// unchanged on the way back), against the first sketch of `ThriftError`'s
// `Service` arm in glean-util/src/util/thrift_service.rs:264-292. So the
// image is a downcast helper scoped to that one carrier:
//
//   * `as_batch_already_processed` below is the ONLY site that changes if
//     Step 4's catch-taxonomy table (backend/types.rs:407-414, whose
//     provisional note says which carriers ever hold declared exceptions is
//     a Step-4 decision) pins a different carrier for it.
//   * the declared `Retry` exception, by contrast, already has the
//     dedicated `BackendError::Retry` variant (types.rs:403-405) — that
//     asymmetry is why the two classifications below differ in shape.
//   * `location :: Maybe Text` (glean.thrift:257-259) -> `Option<String>`;
//     the log message is `"Batch already processed, skipping: " <> show loc`
//     (:110) — hence the private GHC-`show` helpers.
//
// ---------------------------------------------------------------------------
// The `show (loc :: Maybe Text)` helpers
// ---------------------------------------------------------------------------
//
// GHC's `show (Just t)` for Text renders `Just "` <> showLitString t <> `"`,
// and `showLitString`/`showLitChar` (base-4.17.2.0 GHC/Show.hs:389-398
// showLitString incl. the quote branch :397 + showLitChar :373-385) is
// reimplemented privately as `show_lit_string` +
// `show_maybe_text`: `"` -> `\"`; `\` -> `\\`; DEL -> the DEL escape;
// ord > 127 -> backslash + decimal Unicode scalar, with `protectEsc isDec`
// appending `\&` iff the next output char would be a decimal digit (only a
// bare ASCII digit can start one); printable ASCII -> itself; the named
// escapes \a \b \f \n \r \t \v and SO (with `protectEsc (== 'H')`); every
// other control char -> backslash + its asciiTab name (NUL SOH ... US).
// Verified by a differential battery against GHC 9.4.7: 185 cases (each of
// U+0000..U+001F, `"`, `\`, U+007F, and U+0080..U+00FF singly; é/中/😀;
// digit- and H-adjacency after every escape kind; the full printable run)
// — byte-identical (probe /tmp/opencode/db46-probe/showbattery.{hs,rs}).
//
// ---------------------------------------------------------------------------
// ASSUMED-IMPORT (RULE §1 logging row (A10)): the `glean_util::log` seam
// ---------------------------------------------------------------------------
//
// `import Util.Log` (SendBatch.hs:26, for the :110 `logInfo`) has NO manifest
// unit. Unit 45's convention (recorded; a known divergence from unit 43's
// private stand-in-plus-note): this file writes the FINAL call site against
// the pinned shape with a direct import. Unit 45's block
// (rust/glean-db/src/query/thrift/internal.rs:190-213) is the FIRST SKETCH
// of this seam; this file EXTENDS it with `logInfo`:
//
//     pub fn log_info(msg: impl FnOnce() -> String);
//
// Util.Log.String.hs:32-35 — the "message never forced unless verbose"
// laziness is `vlog`-only (:36-39 checks `vlogIsOn` before touching the
// message); `log_info` (:50-53) ALWAYS forces its message, so the closure is
// forced immediately inside the call. It captures `bap` by reference and is
// consumed within the call, so no `'static` demand arises. The module does
// not exist on disk yet; that is the same forward-reference form
// `glean_thrift` is consumed by (and `crate::backend::retry` was, until
// unit 43 landed it), so the swap when glean-util's log unit lands is
// import resolution only, never a call-site edit. No stand-in is written
// here.
//
// ---------------------------------------------------------------------------
// ASSUMED-IMPORT (RULE §4 B4): the glean_thrift extensions this file needs
// ---------------------------------------------------------------------------
//
// rust/glean-db/src/backend/types.rs is the FIRST SKETCH (its member list
// :99-147 and code sketches :149-226): it pins `Repo` (:171-175), `Handle`
// (:168, `:351`), `Retry` (:176-178, `:239`) and lists the batch/response
// members this file uses as OPAQUE (`ComputedBatch`/`SendResponse`/
// `FinishResponse` :110-111, `:354/:369/:374`; `SendJsonBatch`/
// `SendJsonBatchResponse`/`EnqueueBatch`/`EnqueueBatchWaitPolicy`/
// `EnqueueBatchResponse` :126-130, `:890/:902/:920/:924/:933`) — do not
// re-sketch those. THIS unit extends the block with the field shapes and
// union arms below (RULE §4 consumer-extension). Every name is generated by
// build.rs from glean/if/glean.thrift into the pinned `glean_thrift` crate;
// gen-hs2 cites are ~/glean-build/Glean/glean/if/gen-hs2/Glean/Types.hs.
//
// IDL RPC exception sets this file's classification runs against (prose
// context for the try table): sendBatch throws UnknownDatabase
// (glean.thrift:1146); enqueueBatch throws Exception/Retry/UnknownDatabase/
// BatchAlreadyProcessed (:1156); finishBatch throws UnknownBatchHandle
// (:1172); sendJsonBatch throws Exception/Retry/UnknownDatabase (:1184).
//
// The extensions:
//
//     #[derive(Clone, Debug, Default, PartialEq, Eq)]
//     pub struct ComputedBatch {            // :354-363; gen-hs2 :3091
//         pub repo: Repo,                   // constructor ComputedBatch
//         pub remember: bool,               // repo remember batch, :355,
//                                           // :360, :362
//         pub batch: Batch,
//     }   // field 2 does not exist in the IDL, fields 1/3/4 do
//
//     pub struct Batch { /* opaque */ }     // :159; gen-hs2 :1246
//
//     #[derive(Clone, Debug, Default, PartialEq, Eq)]
//     pub struct Subst { /* opaque */ }     // :216-219; gen-hs2 :1555
//         // Default = gen-hs2 :1626 (the `def` at :67); Clone per the
//         // generated-struct convention (types.rs:221-224)
//
//     pub struct JsonFactBatch { /* opaque */ }       // :882; gen-hs2 :8685
//     pub struct SendJsonBatchOptions { /* opaque */ } // :111; gen-hs2 :697
//
//     #[derive(Clone, Debug, Default, PartialEq, Eq)]
//     pub struct SendJsonBatch {            // :890-900; gen-hs2 :8794
//         pub batches: Vec<JsonFactBatch>,
//         pub options: Option<SendJsonBatchOptions>,
//         pub remember: bool,
//     }
//
//     #[derive(Clone, Debug, Default, PartialEq, Eq)]
//     pub struct SendJsonBatchResponse {    // :902-907; gen-hs2 :8909
//         pub handle: String,               // + auth_status/auth_message
//     }
//
//     #[derive(Clone, Debug, PartialEq, Eq)]
//     pub enum SendResponse {               // :369-372, (hs.nonempty);
//         Empty,                            // gen-hs2 :3239-3240 has NO
//         Handle(Handle),                   // Empty constructor; fbthrift's
//         Retry(BatchRetry),                // Rust union image adds the
//     }                                     // unset `Empty` arm first, the
//                                           // SelectSchema convention
//                                           // (types.rs:180-186)
//
//     #[derive(Clone, Debug, PartialEq, Eq)]
//     pub enum FinishResponse {             // :374-377, (hs.nonempty);
//         Empty,                            // gen-hs2 :3291
//         Subst(Subst),
//         Retry(BatchRetry),
//     }
//
//     #[derive(Clone, Debug, PartialEq, PartialOrd)]
//     pub struct BatchRetry { pub seconds: f64 }  // :365-367; gen-hs2 :3184
//         // newtype BatchRetry{batchRetry_seconds::Double}; the derive
//         // list matches the sibling Retry sketch (types.rs:176-178)
//
//     #[derive(Clone, Debug, PartialEq)]
//     pub struct BatchAlreadyProcessed {     // :257-259; gen-hs2 :2130-2132
//         pub location: Option<String>,      // newtype over Maybe Text
//     }
//         // Display is constructor-shaped per RULE §2's deriving-Show row;
//         // Debug + Display + std::error::Error + Send + Sync are required
//         // for the value to ride ThriftError::Service's box
//
//     #[derive(Clone, Debug, PartialEq, Eq)]
//     pub enum EnqueueBatch {               // :920-922 (NOT hs.nonempty:
//         Empty,                            // gen-hs2 :9151-9152 carries
//         Descriptor(BatchDescriptor),      // EnqueueBatch_EMPTY); this
//     }                                     // port only CONSTRUCTS Descriptor
//
//     // :915-918, gen-hs2 :9071 — passed through, never inspected:
//     pub struct BatchDescriptor { pub location: String, pub format: BatchFormat }
//     // :909-912, gen-hs2 :9038 (JSON=0, Binary=1)
//     pub enum BatchFormat { Json, Binary }
//
//     #[derive(Clone, Copy, Debug, PartialEq, Eq)]
//     pub enum EnqueueBatchWaitPolicy {     // :924-931; gen-hs2 :9200
//         None,                             // ASSUMED-image mark: Step 4
//         Remember,                         // verifies these spellings
//     }                                     // against the generated crate;
//                                           // a mismatch is a compile error,
//                                           // never silent behavior
//
//     #[derive(Clone, Debug, Default, PartialEq, Eq)]
//     pub struct EnqueueBatchResponse {     // :933-938; gen-hs2 :9236
//         pub handle: String,               // + auth_status/auth_message
//     }
//
// Step-4 fallback (RULE §4 B4): if the generated crate ships different
// spellings (variant names, field names, derive sets), only this file's
// match arms and field reads change — a compile error, never silent.
//
// ---------------------------------------------------------------------------
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, module-level)
// ---------------------------------------------------------------------------
//
// Rows naming glean/backend-api/Glean/Write/SendBatch.hs (grepped):
//   OUTBOUND — exactly one:
//     :28  -> glean/backend-api/Glean/Backend/Types.hs
//         = `crate::backend::types::{Backend, BackendError}` (same crate;
//           the trait whose enqueue_batch/enqueue_json_batch/
//           enqueue_batch_descriptor/poll_batch methods this file drives,
//           and the error channel).
//   INBOUND — exactly three, all consumers (no impact on this translation;
//   they import the six functions by the same names):
//     :23   glean/backend-api/Glean/Query/Thrift.hs (manifest 47 ->
//           rust/glean-db/src/query/thrift.rs) imports this module (:47);
//     :75   glean/client/hs/Glean.hs (manifest 161) re-exports
//           sendBatchAndWait/sendJsonBatchAndWait/sendBatchDescriptor
//           (:186-194);
//     :1888 glean/write/Glean/Write/SendQueue.hs (manifest 157 ->
//           rust/glean-client/src/write/send_queue.rs) calls sendBatch
//           (:286), sendJsonBatch (:287) and sendBatchDescriptor (:289),
//           and reads the empty-handle convention (:290-292).
//
// ---------------------------------------------------------------------------
// Import disposition (SendBatch.hs:22-29)
// ---------------------------------------------------------------------------
//
//   * Control.Exception (:22; try/fromException/throwIO at :79/:104/:115)
//     -> the Result<_, BackendError> returns + the per-function catch sets
//     of the try table above; no Rust import.
//   * Data.Default (:23; `def` :67) -> `Default::default()`
//     (`thrift::Subst::default()`).
//   * qualified Data.Text as Text (:24; `Text.null` :66) -> `str::is_empty`
//     on the `String` handle; no import.
//   * System.Time.Extra (:25; `sleep` :119) -> the private `sleep` helper
//     (no manifest unit — external package; pinned source cited above).
//   * Util.Log (:26; `logInfo` :110) -> the assumed `glean_util::log` seam
//     (block above).
//   * Glean.Backend.Types (:28; the class and its methods) ->
//     `crate::backend::types::{Backend, BackendError}` (row 41).
//   * qualified Glean.Types as Thrift (:29) -> `use glean_thrift as thrift;`
//     (the types.rs:361 alias convention; generated, no manifest row).
//
// ---------------------------------------------------------------------------
// INVENTORY (RULE §5)
// ---------------------------------------------------------------------------
//
// Grep "SendBatch" (and "send batch", case-insensitive) across
// migration/inventory*.tsv: ZERO hits — no inventory row names this file,
// so there is no row to follow or diverge from. The nearest rows do not
// govern it: inventory.tsv:7 (wrapper initGlobalState pass-throughs — this
// is not a Backend wrapper), :13 (EXC_TAXONOMY, the Haxl consumer), :118-121
// (the write-pipeline state/taxonomy rows: Async.hs, SendAndRebaseQueue.hs,
// SendQueue.hs — SendQueue.hs is an INBOUND consumer of this module, and its
// rows govern its queue state, not these six functions). No
// inventory-divergence flag is raised.

use std::thread;
use std::time::Duration;

use glean_thrift as thrift;
// ASSUMED-IMPORT (RULE §1 logging row (A10)): see the header block — the
// future `glean_util::log` surface, consumed at its pinned shape (the
// unit-45 first sketch extended with log_info).
use glean_util::log::log_info;

use crate::backend::types::{Backend, BackendError};

// ---------------------------------------------------------------------------
// sleep / retry (SendBatch.hs:117-120; System.Time.Extra:31-41)
// ---------------------------------------------------------------------------

/// `sleep :: Seconds -> IO ()` (System.Time.Extra:31-41): `loopM` over
/// 2000-second `threadDelay 2000000000` chunks; a negative value returns
/// immediately; the final chunk sleeps `ceiling (s * 1e6)` microseconds.
///
/// NaN note (no observable divergence — header block): GHC's
/// `ceiling :: Int` yields minBound for NaN, which `threadDelay` treats as
/// an immediate return; this saturating cast yields 0 — both proceed at
/// once.
fn sleep(secs: f64) {
    let mut s = secs;
    loop {
        if s < 0.0 {
            return;
        } else if s > 2000.0 {
            thread::sleep(Duration::from_micros(2_000_000_000));
            s -= 2000.0;
        } else {
            // NaN: `as u64` saturates to 0, and the source's `ceiling ::
            // Int` minBound is an immediate threadDelay — both proceed at
            // once (header block).
            thread::sleep(Duration::from_micros((s * 1_000_000.0).ceil() as u64));
            return;
        }
    }
}

/// `retry secs action = sleep (max secs 1) >> action` (:117-120) — the
/// sleep half only; the action half is each caller's `loop`. The `if
/// secs <= 1.0` form is Haskell `max` on Ord Double (`if x <= y then y else
/// x`) reproduced for every input, NaN included; `f64::max` would return 1.0
/// for NaN (divergent — header block).
fn retry(secs: f64) {
    sleep(if secs <= 1.0 { 1.0 } else { secs });
}

// ---------------------------------------------------------------------------
// show helpers (SendBatch.hs:110 `show loc`, loc :: Maybe Text)
// ---------------------------------------------------------------------------

/// GHC `showLitString` (base-4.17.2.0 GHC/Show.hs:389-398, quote branch
/// :397 + showLitChar :373-385): the escaped body of a string, without
/// surrounding quotes.
fn show_lit_string(s: &str) -> String {
    // asciiTab (GHC/Show.hs:428-434), the named escapes for c < ' '; the
    // SP entry is unreachable (`c >= ' '` fires first).
    const ASCII_TAB: [&str; 32] = [
        "NUL", "SOH", "STX", "ETX", "EOT", "ENQ", "ACK", "BEL", "BS", "HT", "LF", "VT", "FF", "CR",
        "SO", "SI", "DLE", "DC1", "DC2", "DC3", "DC4", "NAK", "SYN", "ETB", "CAN", "EM", "SUB",
        "ESC", "FS", "GS", "RS", "US",
    ];
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        // showLitString quotes branch (:397).
        if c == '"' {
            out.push_str("\\\"");
        // showLitChar echo `c > '\DEL'` (:373): decimal Unicode scalar +
        // `protectEsc isDec` (`\&` iff the next output char is a decimal
        // digit — only a bare printable ASCII digit can start one).
        } else if c as u32 > 0x7F {
            out.push('\\');
            out.push_str(&(c as u32).to_string());
            if matches!(chars.peek(), Some(d) if d.is_ascii_digit()) {
                out.push_str("\\&");
            }
        // showLitChar '\DEL' (:374).
        } else if c == '\u{7f}' {
            out.push_str("\\DEL");
        // showLitChar '\\' (:375).
        } else if c == '\\' {
            out.push_str("\\\\");
        // showLitChar echo `c >= ' '` (:376).
        } else if c >= ' ' {
            out.push(c);
        // showLitChar named escapes (:377-383).
        } else {
            match c {
                '\u{7}' => out.push_str("\\a"),
                '\u{8}' => out.push_str("\\b"),
                '\u{c}' => out.push_str("\\f"),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                '\u{b}' => out.push_str("\\v"),
                // '\SO' (:384): `protectEsc (== 'H')` — `\&` iff the next
                // output char is 'H' (only a bare 'H' can start one).
                '\u{e}' => {
                    out.push_str("\\SO");
                    if matches!(chars.peek(), Some('H')) {
                        out.push_str("\\&");
                    }
                }
                // showLitChar fallback (:385): backslash + asciiTab name.
                _ => {
                    out.push('\\');
                    out.push_str(ASCII_TAB[c as usize]);
                }
            }
        }
    }
    out
}

/// GHC `show (t :: Maybe Text)`: `Nothing` -> `Nothing`; `Just t` ->
/// `Just "<escaped>"` (Show Maybe + showLitString's quotes).
fn show_maybe_text(t: Option<&str>) -> String {
    match t {
        None => "Nothing".to_string(),
        Some(t) => format!("Just \"{}\"", show_lit_string(t)),
    }
}

// ---------------------------------------------------------------------------
// BatchAlreadyProcessed downcast (SendBatch.hs:109)
// ---------------------------------------------------------------------------

/// Rust image of `fromException e :: Maybe BatchAlreadyProcessed` (:109):
/// declared exceptions ride the generated crate's open `ThriftError::Service`
/// leg (see the header block; retry.rs:334-353's encoding). The ONLY site
/// that changes if Step 4's catch-taxonomy table (types.rs:407-414) pins a
/// different carrier.
fn as_batch_already_processed(e: &BackendError) -> Option<&thrift::BatchAlreadyProcessed> {
    match e {
        BackendError::Thrift(thrift::ThriftError::Service(boxed)) => {
            boxed.downcast_ref::<thrift::BatchAlreadyProcessed>()
        }
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// sendBatchAndWait (SendBatch.hs:31-39)
// ---------------------------------------------------------------------------

/// `sendBatchAndWait :: Backend be => be -> Repo -> Batch -> IO Subst`
/// (:31-36). No `try` of its own (:37-39).
pub fn send_batch_and_wait<B: Backend>(
    backend: &B,
    repo: &thrift::Repo,
    batch: thrift::Batch,
) -> Result<thrift::Subst, BackendError> {
    let handle = send_batch(backend, repo, batch, true)?;
    wait_batch(backend, repo, handle)
}

// ---------------------------------------------------------------------------
// sendBatch (SendBatch.hs:41-53)
// ---------------------------------------------------------------------------

/// `sendBatch :: Backend be => be -> Repo -> Batch -> Bool -> IO Handle`
/// (:41-47). NO `try` (:48-53): `enqueueBatch` errors — including a declared
/// `Retry` — PROPAGATE through the `?`; the union's `retry` arm is the only
/// retry signal.
pub fn send_batch<B: Backend>(
    backend: &B,
    repo: &thrift::Repo,
    batch: thrift::Batch,
    remember: bool,
) -> Result<thrift::Handle, BackendError> {
    loop {
        let r = backend.enqueue_batch(thrift::ComputedBatch {
            repo: repo.clone(),
            remember,
            batch: batch.clone(),
        })?;
        match r {
            thrift::SendResponse::Handle(h) => return Ok(h),
            thrift::SendResponse::Retry(br) => {
                retry(br.seconds);
            }
            // (hs.nonempty) — the Haskell type has no EMPTY constructor
            // (gen-hs2 :3239-3240); only the wire-level deserializer could
            // produce it, and it rejects an unset union.
            thrift::SendResponse::Empty => {
                unreachable!("SendResponse::Empty has no Haskell image (hs.nonempty)")
            }
        }
    }
}

// ---------------------------------------------------------------------------
// sendJsonBatchAndWait (SendBatch.hs:55-68)
// ---------------------------------------------------------------------------

/// `sendJsonBatchAndWait :: Backend be => be -> Repo -> [JsonFactBatch] ->
/// Maybe SendJsonBatchOptions -> IO Subst` (:55-61). The empty handle from
/// an old synchronous server means "already done" -> `def` (:64-67).
pub fn send_json_batch_and_wait<B: Backend>(
    backend: &B,
    repo: &thrift::Repo,
    batches: Vec<thrift::JsonFactBatch>,
    opts: Option<thrift::SendJsonBatchOptions>,
) -> Result<thrift::Subst, BackendError> {
    let handle = send_json_batch(backend, repo, batches, opts, true)?;
    // Cope with talking to an old server where this API was synchronous
    // and didn't return a handle. (source comment :64-65)
    if handle.is_empty() {
        return Ok(thrift::Subst::default());
    }
    wait_batch(backend, repo, handle)
}

// ---------------------------------------------------------------------------
// sendJsonBatch (SendBatch.hs:70-90)
// ---------------------------------------------------------------------------

/// `sendJsonBatch :: Backend be => be -> Repo -> [JsonFactBatch] -> Maybe
/// SendJsonBatchOptions -> Bool -> IO Handle` (:70-77). `try @Retry`
/// (:79-88): catches ONLY the declared `Retry`; every other error
/// propagates.
pub fn send_json_batch<B: Backend>(
    backend: &B,
    repo: &thrift::Repo,
    batches: Vec<thrift::JsonFactBatch>,
    opts: Option<thrift::SendJsonBatchOptions>,
    remember: bool,
) -> Result<thrift::Handle, BackendError> {
    loop {
        let r = backend.enqueue_json_batch(
            repo,
            thrift::SendJsonBatch {
                batches: batches.clone(),
                options: opts.clone(),
                remember,
            },
        );
        match r {
            Ok(thrift::SendJsonBatchResponse { handle, .. }) => return Ok(handle),
            Err(BackendError::Retry(r)) => {
                retry(r.seconds);
            }
            Err(e) => return Err(e),
        }
    }
}

// ---------------------------------------------------------------------------
// sendBatchDescriptor (SendBatch.hs:92-115)
// ---------------------------------------------------------------------------

/// `sendBatchDescriptor :: Backend be => be -> Repo -> BatchDescriptor ->
/// Bool -> IO Handle` (:92-98). `try @SomeException` + `fromException`
/// classification in guard order (:104-115): BatchAlreadyProcessed -> log and
/// return the empty handle; Retry -> sleep and recurse; otherwise rethrow.
pub fn send_batch_descriptor<B: Backend>(
    backend: &B,
    repo: &thrift::Repo,
    descriptor: thrift::BatchDescriptor,
    remember: bool,
) -> Result<thrift::Handle, BackendError> {
    loop {
        let batch = thrift::EnqueueBatch::Descriptor(descriptor.clone());
        let policy = if remember {
            thrift::EnqueueBatchWaitPolicy::Remember
        } else {
            thrift::EnqueueBatchWaitPolicy::None
        };
        match backend.enqueue_batch_descriptor(repo, batch, policy) {
            Ok(thrift::EnqueueBatchResponse { handle, .. }) => return Ok(handle),
            Err(e) => {
                if let Some(bap) = as_batch_already_processed(&e) {
                    // `logInfo $ "Batch already processed, skipping: " <>
                    // show loc` (:110); the message is ALWAYS forced.
                    log_info(|| {
                        format!(
                            "Batch already processed, skipping: {}",
                            show_maybe_text(bap.location.as_deref())
                        )
                    });
                    return Ok(String::new());
                } else if let BackendError::Retry(r) = &e {
                    retry(r.seconds);
                } else {
                    return Err(e);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// waitBatch (SendBatch.hs:122-136)
// ---------------------------------------------------------------------------

/// `waitBatch :: Backend be => be -> Repo -> Handle -> IO Subst` (:122-127).
/// `try @Retry` (:129-132): catches ONLY the declared `Retry`; the success
/// union `FinishResponse` (:133-136) returns `subst` or retries on its
/// `retry` arm. Non-Retry errors propagate.
pub fn wait_batch<B: Backend>(
    backend: &B,
    repo: &thrift::Repo,
    handle: thrift::Handle,
) -> Result<thrift::Subst, BackendError> {
    loop {
        match backend.poll_batch(Some(repo), handle.clone()) {
            Err(BackendError::Retry(r)) => {
                retry(r.seconds);
            }
            Ok(r) => match r {
                thrift::FinishResponse::Subst(subst) => return Ok(subst),
                thrift::FinishResponse::Retry(br) => {
                    retry(br.seconds);
                }
                // (hs.nonempty) — no EMPTY constructor (gen-hs2 :3291).
                thrift::FinishResponse::Empty => {
                    unreachable!("FinishResponse::Empty has no Haskell image (hs.nonempty)")
                }
            },
            Err(e) => return Err(e),
        }
    }
}

// ===========================================================================
// Coverage / markers / status
// ===========================================================================
//
// Coverage: the 136-line source is fully accounted for. All six exports
// (:31-39, :41-53, :55-68, :70-90, :92-115, :122-136) -> the six pub fns,
// signature-for-signature; the local `retry` helper (:117-120) -> the
// private `sleep`/`retry` pair (the action half is the callers' `loop`);
// both source comments are kept at their sites (:64-65 old-server note,
// :110 log message); the empty-handle convention (:66-68) -> `is_empty` +
// `Subst::default()`. The header's try table disposes every `try` site
// (:79, :104, :129) and every union arm (:50-53, :85-90, :106-107, :133-136).
// Nothing is dropped, stubbed, or deferred.
//
// Marker breakdown (closed set — zero of every class):
//   * port todos: zero. The two novel mechanisms — the GHC-show replicas
//     and the Service-box downcast — are decided, probe-verified images of
//     source semantics, not open questions; the NaN conversion carries a
//     no-divergence note at its site, and the taxonomy question that could
//     touch the downcast carrier belongs to types.rs:407-414 and is cited,
//     not duplicated.
//   * source-defect reproductions: zero — the source is total (no partial
//     functions, no `error`; the only throw is the descriptor branch's
//     rethrow of an unclassified exception, ported as `return Err(e)`).
//   * performance deferrals: zero — each retry attempt pays one closure-free
//     match, the per-attempt clone the source's re-executed call pays
//     semantically, and the same thread sleeps.
//   * safety comments: zero — no unsafe.
//   * inventory divergence: zero — no inventory row names this file
//     (INVENTORY block above).
//
// PORT STATUS: confidence=high todos=0
