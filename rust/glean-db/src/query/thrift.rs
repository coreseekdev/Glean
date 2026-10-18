// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// Ported from: glean/backend-api/Glean/Query/Thrift.hs (153 lines)
// Manifest unit: rust/glean-db/src/query/thrift.rs
// (migration/manifest.tsv line 47, crate glean-db). RULE §4: one source
// file -> one target file; the manifest path is authoritative.
//
// WHAT THIS FILE IS: the query facade — the module the source tells
// consumers to use instead of the internals (Internal.hs:14 "please import
// Glean.Query.Thrift instead"). It has four parts:
//   * the six query-running functions (:50-153): the one-page entry
//     (runQueryPage, :89-105), the fetch-all pair (runQuery_/runQuery,
//     :50-75), and the streaming trio (runQueryMapPages_/runQueryEach/
//     runQueryEachBatch, :110-153) — the pipelined loop is the migration's
//     first `Async.concurrently` site (B2; marker at the site);
//   * the 12 re-exports (:21-33) that resolve to row 45's
//     crate::query::thrift::internal (the combinators `query`...`store`,
//     `all_facts`, `display_query`), plus the `Query` type re-export
//     (:11);
//   * the error type this module owns (the source throws untyped through
//     IO; the port's IO channel is `Result<_, RunQueryError>`);
//   * the deferred page action (`pub type Page<Q>`), the Rust image of the
//     source's inner `IO [q]` — the source binds `decodeResults` as a lazy
//     action and hands it out (:104-105); the port boxes the same action.
//
// MODULE WIRING (RULE §4): this unit finishes the crate's Query-area
// wiring. It declares `pub mod internal;` (below) and appends
// `pub mod thrift;` at the top of src/query.rs — those two declarations
// bind `crate::query::thrift` and `crate::query::thrift::internal` (row
// 45's file, shipped) to the crate root; src/lib.rs has carried
// `pub mod query;` since row 45. Both prose records of the pending append
// (src/query.rs's contract block, src/lib.rs's Query/ area block and the
// MODULE WIRING paragraph in src/query/thrift/internal.rs) were rewritten
// to their consumed form by this unit.
//
// ---------------------------------------------------------------------------
// EXPORT MAPPING (Thrift.hs:9-34 — 19 exported names; source order:
// Types :11, perform-query :13-15, streaming :17-19, combinators :21-31,
// support :33)
// ---------------------------------------------------------------------------
//
//    #  Thrift.hs                        Rust image
//   --  --------------------------------  ---------------------------------
//    1  Query                    :11     pub use crate::backend::types::Query
//                                         (the owner re-export; Internal.hs
//                                         carries the same path at :17 —
//                                         the source obtains Query through
//                                         Internal.hs's re-export, the port
//                                         names the owner once, documented
//                                         at the code block)
//    2  runQuery                 :13     run_query
//    3  runQuery_                :14     run_query_
//    4  runQueryPage             :15     run_query_page
//    5  runQueryMapPages_        :17     run_query_map_pages_
//    6  runQueryEach             :18     run_query_each
//    7  runQueryEachBatch        :19     run_query_each_batch
//    8  query                    :21     (re-export; Internal.hs:19)
//    9  angle                    :22     angle (Internal.hs:20)
//   10  angleData                :23     angle_data (Internal.hs:21)
//   11  keys                     :24     keys (Internal.hs:22)
//   12  recursive                :25     recursive (Internal.hs:23)
//   13  limit                    :26     limit (Internal.hs:24)
//   14  limitBytes               :27     limit_bytes (Internal.hs:25)
//   15  limitTime                :28     limit_time (Internal.hs:26)
//   16  expanding                :29     expanding (Internal.hs:29)
//   17  store                    :30     store (Internal.hs:27)
//   18  allFacts                 :31     all_facts (Internal.hs:28)
//   19  displayQuery             :33     display_query (Internal.hs:33)
//
// The facade deliberately does NOT re-export Internal.hs's other five
// exported names (`justCheck`/`dbgPredHasFacts`/`reportUserQueryStats`/
// `showUserQueryStats`/`decodeResults` — the source's export list omits
// them; Internal.hs:16-37 lists all 18, the facade's :9-34 picks 13) nor
// its two private definitions (`angleQuery`, `type ResultDecoder`).
// Nothing else is exported here.
//
// ---------------------------------------------------------------------------
// THE CONCURRENCY BLOCK (B2) AND THE BOUNDS IT FORCES
// ---------------------------------------------------------------------------
//
// runQueryEachBatch's second and later iterations run the next fetch and
// the current page's processing concurrently (source :149-152). The
// source uses `Async.concurrently`, which cancels the losing side on the
// first exception; RULEBOOK §2's B2 default is let-drain with the marker
// at the site. The port spawns the fetch on a scoped thread, processes the
// current page on the calling thread, and joins before continuing (the
// marker text at the site spells out the divergence and the scope-join
// properties).
//
// Rust-only compile-time requirements (not observable semantics; no source
// clause corresponds):
//   * `B: Backend + Sync` on run_query_each_batch / run_query_each /
//     run_query_map_pages_ / run_query_ — the fetch closure captures
//     `&B`, and `&B: Send` holds exactly when `B: Sync`. run_query_page and
//     run_query do NOT need Sync (they never spawn).
//   * `&Query<Q>: Send` needs no bound on `Q`: Query's marker field is
//     `PhantomData<fn() -> A>` (types.rs:1397-1401), which is Send + Sync
//     for every A, and the assumed `thrift::UserQuery` image is plain data
//     (assumed-import block). For the same reason no `Q: Send` exists
//     anywhere: Q crosses no thread boundary — only the RESULT
//     `Result<(Page<Q>, Option<UserQueryCont>), RunQueryError>` does, and
//     every component of it is Send by construction (the Page trait object
//     carries `+ Send`; the assumed thrift members are Send + Sync).
//   * no `S: Send` and no `F: Send` either: the state and the callback are
//     used only on the calling thread; they never enter the scoped thread.
//
// ---------------------------------------------------------------------------
// RUST-ONLY ADDITIONS AND MAPPING CHOICES
// ---------------------------------------------------------------------------
//
//   * `RunQueryError` — no source counterpart: the source's callbacks and
//     backend calls run in `IO` and throw dynamically (`SomeException` is
//     the only common supertype). RULE §2's dynamic-exceptions row pins
//     "typed error enum per crate"; this is the facade's union image:
//     BackendError (row 41's channel, provable from userQuery/waitBatch
//     sites), DecodeError (row 45's decodeResults ErrorCall arm), and the
//     open leg for caller-supplied callback errors. Display is
//     constructor-shaped per RULE §2's deriving-Show default — there is no
//     Show to replicate (the type is Rust-only), so the default naming
//     (`Backend(...)`/`Decode(...)`/`Other(...)`) applies; the Backend leg
//     renders with `{:?}` because BackendError has no Display yet
//     (types.rs:407-414 defers it to Step 4's catch-taxonomy table).
//   * `pub type Page<Q>` — the deferred page action, the image of the
//     source's inner `IO [q]` (:87: `IO (IO [q], Maybe UserQueryCont)`).
//     The `+ Send` is load-bearing: the action is built inside
//     run_query_page and RETURNED from the scoped fetch thread through the
//     join.
//   * runQuery_'s accumulator: the source threads a difference list —
//     `id` initial, `\acc page -> acc . (page++)` per batch (:57-58),
//     applied once to `[]` (:59). The port threads a `Vec` and `extend`s
//     per page: order- and element-identical (GHC probe
//     /tmp/opencode/db47-probe/ghc_acc.hs: `id . (p1++) . (p2++) . ...`
//     applied to [] equals `concat [p1, p2, ...]`), same asymptotics (each
//     element copied once either way) — so no performance deferral marker.
//   * runQueryEach's per-element step: `foldM fn` (:133) threads
//     left-to-right; the port's `for` loop is that fold (same probe).
//   * Effects order in run_query_page is the source's: report stats
//     (:101), then log diagnostics (:102), then waitBatch (:103), and the
//     decode (:104) is deferred into the returned action. `IO` returns
//     become `Result<_, RunQueryError>`; `?` is `=<<`-then-throw.
//   * Signature notes: `runQueryMapPages_` keeps the source's argument
//     order `be repo fn query` (:110-118); the `Maybe UserQueryCont`
//     continuation is `Option<thrift::UserQueryCont>` by value (the
//     generated struct is plain data; the request builder moves it in).
//   * `Data.Default` -> `Default::default()`; `Data.Maybe`'s
//     `fromMaybe`/`isJust` -> `Option::take().unwrap_or_default()` /
//     `is_some()`.
//
// ---------------------------------------------------------------------------
// ASSUMED-IMPORT (RULE §4 B4) EXTENSION: the `glean_thrift` crate (generated)
// ---------------------------------------------------------------------------
//
// SKETCH ANCHORS, consumed unchanged — each first sketch cited at its OWNER
// (RULEBOOK.md:116-126: a consumer extending an assumed surface records the
// extension in its own block and cross-references the first sketch; never
// fork a hand-translation):
//   * crate-level image — first sketch owned by
//     rust/glean-db/src/backend/types.rs:87-226: flat crate root, IDL field
//     names in snake_case, `optional` -> Option, `list` -> Vec,
//     `bytestring` -> Vec<u8>, thrift `map` -> BTreeMap, struct
//     Default = hsthrift `def` (types.rs:149-156).
//   * `Handle` — backend/types.rs:168 (`pub type Handle = String`, :351).
//   * `Repo` — backend/types.rs:171-175 (:106).
//   * `UserQuery` — internal.rs:141-149 (the request record this file
//     clones and updates).
//   * `UserQueryStats` — internal.rs:151-155.
//   * `UserQueryEncodedResults` — internal.rs:168-184 (the union the
//     deferred decode consumes).
//
// THIS UNIT'S EXTENSION: the members the record update (:90-99) and the
// response read (:100-105) need. The IDL is glean/if/glean.thrift; gen-hs2
// cites are ~/glean-build/Glean/glean/if/gen-hs2/Glean/Types.hs (the
// pinned compiler's mapping).
//
// EXTENSION FLOOR for UserQueryOptions — internal.rs:122-136 sketches:
//
//     struct UserQueryOptions {  // :408-474; Default == hsthrift `def`
//         pub expand_results: bool, ...
//         /* no_base64_binary, continuation, collect_facts_searched,
//            omit_results, ... */ }
//
// i.e. the sketch already LISTED `continuation` as an unnamed slot; this
// unit types that one slot and consumes nothing else of the struct:
//
//     struct UserQueryOptions {           // :408-474
//         /* the internal.rs:122-136 fields unchanged */
//         pub continuation: Option<UserQueryCont>,   // :448 (field 5)
//     }
//         // gen-hs2 :3784-3785 `userQueryOptions_continuation ::
//         // Prelude.Maybe UserQueryCont`; this unit SETS it (:94) and the
//         // rest of the struct rides the hsthrift `def` image.
//
//     #[derive(Clone, Debug, Default, PartialEq, Eq)]
//     pub struct UserQueryCont {           // :393-400; gen-hs2 :3545-3552
//         pub continuation: Vec<u8>,       // :394 (field 3; binary)
//         pub next_id: i64,                // :395 (field 4)
//         pub version: i32,                // :396 (field 5; i32, not
//                                          // hs.newtype)
//         pub hash: i64,                   // :397 (field 6)
//         pub return_type: Option<Vec<u8>>, // :398 (field 7; optional)
//         pub pids: Vec<i64>,              // :399 (field 8)
//     }
//         // the IDL numbers these 3-8 (no fields 1/2); Default = hsthrift
//         // `def` = gen-hs2 :3713 (`""` / 0 / 0 / 0 / Nothing / []).
//
//     pub enum UserQueryEncoding {          // union :532-537; NOT
//                                           // hs.nonempty: gen-hs2
//                                           // :4531-4535 HAS
//                                           // UserQueryEncoding_EMPTY
//         Empty,                            // the unset arm (never
//                                           // constructed here)
//         Bin(UserQueryEncodingBin),        // :533
//         Json(UserQueryEncodingJSON),      // :534
//         Compact(UserQueryEncodingCompact), // :535
//         Listbin(UserQueryEncodingListBin), // :536
//     }
//         // arm spellings = UpperCamel of the thrift field names, the same
//         // convention as internal.rs's UserQueryEncodedResults sketch
//         // (:168-184); this unit only CONSTRUCTS Bin and Listbin.
//
//     #[derive(Clone, Debug, Default, PartialEq, Eq)]
//     pub struct UserQueryEncodingBin {}    // :489; gen-hs2 :4311,
//                                           // Default :4341
//     #[derive(Clone, Debug, Default, PartialEq, Eq)]
//     pub struct UserQueryEncodingListBin {} // :492; gen-hs2 :4347,
//                                            // Default :4377
//         // empty structs; hsthrift `def` is the empty value, so
//         // `::default()` is the construction image of
//         // `Thrift.UserQueryEncoding_listbin def` (:96-97).
//
//     pub struct UserQueryEncodingJSON { /* opaque */ }    // :495-523
//     pub struct UserQueryEncodingCompact { /* opaque */ } // :526-529,
//                                                          // gen-hs2 :4472
//                                                          // (Default :4524)
//         // payloads named by the union arms only; never inspected here
//         // (gen-hs2 :4383 JSON carries expand_results/no_base64_binary —
//         // not this unit's business).
//
//     #[derive(Clone, Debug, Default, PartialEq, Eq)]
//     pub struct UserQueryResults {        // :768-802; gen-hs2 :7192-7204
//         pub stats: Option<UserQueryStats>,       // :772
//         pub continuation: Option<UserQueryCont>, // :778
//         pub diagnostics: Vec<String>,            // :782 (list<string>;
//                                                  // Text -> String per
//                                                  // RULE §1)
//         pub results: UserQueryEncodedResults,    // :786
//         pub handle: Option<Handle>,              // :789
//         /* facts :769 and nested_facts :775 (both deprecated),
//            type :794, auth_status :800, auth_message :801 */
//     }
//         // Default = hsthrift `def` = gen-hs2 :7511.
//
// Send + Sync claim: every member above is plain data (String / Vec<u8> /
// i64 / i32 / Option / Vec), so the generated structs and unions are
// Send + Sync; that is what lets the scoped fetch thread's
// `Result<(Page<Q>, Option<UserQueryCont>), RunQueryError>` cross the
// `thread::scope` join, and why no `Q: Send` bound appears (see the bounds
// block).
//
// Step-4 fallback (RULE §4 B4): if the generated crate ships different
// spellings (variant names, field names, derive sets), only this file's
// field reads and constructions change — a compile error, never silent
// behavior.
//
// ---------------------------------------------------------------------------
// ASSUMED-IMPORT (RULE §1 logging row (A10)): the `glean_util::log` seam
// ---------------------------------------------------------------------------
//
// `import Util.Log.Text` (Thrift.hs:41, for the :102 `mapM_ (vlog 1)`) has
// NO manifest unit: Util.Log.Text is hsthrift's Text-typed glog shim
// (~/glean-build/Glean/hsthrift/common/util/Util/Log/Text.hs:38-41 —
// `vlog :: Int -> Text -> m ()`; its "we only evaluate the message if
// verbosity at the desired level is enabled" wording is the twin of
// Util.Log.String.hs:36-39; the message is forced within `vlog` iff
// `vlogIsOn level`). RULEBOOK §1's logging row (A10) pins the image:
//
//     pub fn vlog(level: i64, msg: impl FnOnce() -> String);
//
// First sketch: internal.rs:190-213 (unit 45); unit 46 extended it with
// `log_info` (send_batch.rs:150-171); this unit consumes the same pinned
// `vlog`. The DECISIONS.md 2026-10-18 log-seam ruling: the direct
// final-call-site form is the convention — no private stand-in, no port
// todo; the module is consumed as a forward reference until glean-util's
// log unit lands, at which point the swap is import resolution only.
// Text -> String per RULE §1's strings row, so the one pinned `vlog`
// covers both hsthrift variants. The closures below capture their
// diagnostic strings by reference and `vlog` consumes them within the
// call, so no `'static` demand arises.
//
// ---------------------------------------------------------------------------
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, module-level)
// ---------------------------------------------------------------------------
//
// Rows naming glean/backend-api/Glean/Query/Thrift.hs (grepped):
//   OUTBOUND — exactly four (:21-:24):
//     :21 -> glean/backend-api/Glean/Backend/Types.hs  (row 41, SHIPPED:
//            Backend/BackendError + the Query owner),
//     :22 -> glean/backend-api/Glean/Query/Thrift/Internal.hs (row 45,
//            SHIPPED: decode_results/report_user_query_stats + the 12
//            re-exports),
//     :23 -> glean/backend-api/Glean/Write/SendBatch.hs (row 46, SHIPPED:
//            wait_batch),
//     :24 -> glean/typed/Glean/Typed/Binary.hs (SHIPPED: the `Type` trait
//            and its decode_as_fact, binary.rs:235-268).
//   INBOUND — two, both consumers (they import the facade's names; no
//   impact on this translation):
//     :73 glean/client/hs/Glean.hs (manifest 161) re-exports Query,
//         runQuery, runQuery_, runQueryEach, runQueryEachBatch,
//         runQueryMapPages_ and the combinator names (:74-100);
//     :89 glean/client/hs/Glean/Haxl.hs uses the facade.
// `Util.Log.Text` (:41) and `Glean.Types` (:44) have NO edges row:
// Util.Log.Text is hsthrift (external package) and Glean.Types is
// generated — the assumed-import class above.
//
// Import disposition (Thrift.hs:36-47), every import:
//   * Control.Concurrent.Async (:36; `concurrently` :150-152) -> the B2
//     `std::thread::scope` block in run_query_each_batch (the migration's
//     first `concurrently` site; marker there).
//   * Control.Monad (:37; `foldM` :133) -> the `for` loop in
//     run_query_each.
//   * Data.Default (:38; `def` :93/:96/:97) -> `Default::default()` (the
//     hsthrift-def pin, types.rs:156).
//   * Data.Maybe (:39; `fromMaybe` :93, `isJust` :75) ->
//     `Option::take().unwrap_or_default()` / `is_some()`.
//   * Util.Log.Text (:41; `vlog` :102) -> the assumed `glean_util::log`
//     seam above.
//   * Glean.Typed.Binary (:43; `decodeAsFact` :104) -> `Q::decode_as_fact`
//     (binary.rs:253-261; the D bound internal.rs:470-485 pins, and
//     internal.rs:296-298 records it as directly passable — re-proven by
//     this unit's probe /tmp/opencode/db47-probe/typecheck.rs).
//   * Glean.Types as Thrift (:44) -> `use glean_thrift as thrift;`
//     (generated; extension block above).
//   * Glean.Backend.Types (Backend(..)) (:45) ->
//     `crate::backend::types::{Backend, BackendError}` (row 41).
//   * Glean.Query.Thrift.Internal (:46) -> `crate::query::thrift::internal`
//     (row 45): the internal consumers + the 12-name re-export block.
//   * Glean.Write.SendBatch (:47) -> `crate::write::send_batch::wait_batch`
//     (row 46).
//
// ---------------------------------------------------------------------------
// INVENTORY (RULE §5; migration/inventory*.tsv)
// ---------------------------------------------------------------------------
//
// Greps across migration/inventory.tsv, inventory.stm.tsv,
// inventory.codeflow.tsv, inventory.ffi.tsv, inventory.protocol.tsv,
// inventory.sites.tsv for "Query/Thrift", "runQuery" and "concurrently":
//   * "Query/Thrift" (2 hits): inventory.tsv:56 and its twin
//     inventory.stm.tsv:35 (the haxl StateStore-caches BATCHING row). Both
//     hits are evidence text ("Thrift/Internal.hs:98,106 cacheRef
//     newIORef IntMap.empty") governing row 45's decode caches — no
//     governing row for THIS file.
//   * "runQuery": zero hits.
//   * "concurrently" (2 hits): inventory.tsv:114 and its twin
//     inventory.stm.tsv:15 (the Observed.hs TVar/update-callback row,
//     where "callbacks may run concurrently" is prose about callback
//     placement). Neither governs this file's B2 site. A case-insensitive
//     sweep additionally hits the Warden and Write.Async rows
//     (inventory.tsv:117-118, inventory.stm.tsv:16/:31), whose
//     `mapConcurrently_`/`forConcurrently_` evidence describes thread
//     fleets and the writer-flush protocol — constructs this site does
//     not use; no governing row there either.
// No inventory-divergence flag is raised; the B2 site's obligation is the
// marker itself, not an inventory row.

pub mod internal;

use std::fmt;

use glean_thrift as thrift;
use glean_typed::typed::build::DecodeError;
use glean_typed::typed::Type;
// ASSUMED-IMPORT (RULE §1 logging row (A10)): see the header block — the
// future `glean_util::log` surface, consumed at its pinned shape (the
// unit-45 first sketch extended by this unit's call site).
use glean_util::log::vlog;

use crate::backend::types::{Backend, BackendError};
use crate::query::thrift::internal::{decode_results, report_user_query_stats};
use crate::write::send_batch::wait_batch;

// The source's `Query` export (:11): one public path, re-exported from the
// OWNER (backend/types.rs:1397-1422) — the same path Internal.hs's export
// carries (internal.rs:440). The source actually obtains Query through
// Internal.hs's re-export; naming the owner here keeps one owner per type
// while the export surface is identical (both `crate::query::thrift::Query`
// and `crate::query::thrift::internal::Query` resolve to it, exactly as the
// two Haskell modules re-exported it).
pub use crate::backend::types::Query;

// The 12 combinator/support re-exports of the source's export list
// (:21-33). NAMED re-exports, no glob (RULE §4); the five Internal names
// the source's export list omits are NOT here (export mapping above).
pub use crate::query::thrift::internal::{
    all_facts, angle, angle_data, display_query, expanding, keys, limit, limit_bytes, limit_time,
    query, recursive, store,
};

/// The `SomeException` image of this facade: backend errors from
/// `userQuery`/`waitBatch` (row 46's `wait_batch`), decode errors from
/// `decodeResults`' ErrorCall arm (row 45), and the open leg for
/// caller-supplied callback errors (the source's `IO` callbacks can throw
/// anything). The source throws all three untyped through `IO`; the port
/// collapses them into this enum at every `?`. See RUST-ONLY ADDITIONS for
/// the Display/Error derivation.
#[derive(Debug)]
pub enum RunQueryError {
    /// `BackendError` from `crate::backend::types` (row 41's channel —
    /// provable from the `userQuery`/`waitBatch` calls).
    Backend(BackendError),
    /// `DecodeError` from `crate::query::thrift::internal::decode_results`
    /// (row 45; the source's `result <- io` can throw it when the action
    /// runs).
    Decode(DecodeError),
    /// The open leg: anything a caller-supplied callback throws. The
    /// source's callbacks are `IO` and can raise `SomeException`; Rust
    /// callbacks return `Result<_, RunQueryError>` and put their own
    /// error type here.
    Other(Box<dyn std::error::Error + Send + Sync>),
}

impl From<BackendError> for RunQueryError {
    fn from(e: BackendError) -> Self {
        RunQueryError::Backend(e)
    }
}

impl From<DecodeError> for RunQueryError {
    fn from(e: DecodeError) -> Self {
        RunQueryError::Decode(e)
    }
}

/// Constructor-shaped, per RULE §2's deriving-Show default (the type has
/// no source counterpart, so there is no Show text to replicate). The
/// Backend leg renders structurally with `{:?}` because `BackendError` has
/// no Display yet (types.rs:407-414 defers Display/Error impls to Step 4's
/// catch-taxonomy table); the other two legs use Display.
impl fmt::Display for RunQueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RunQueryError::Backend(e) => write!(f, "Backend({:?})", e),
            RunQueryError::Decode(e) => write!(f, "Decode({})", e),
            RunQueryError::Other(e) => write!(f, "Other({})", e),
        }
    }
}

/// `source()` stays the default (no source counterpart; a taxonomy of
/// causes belongs to Step 4's table alongside BackendError's).
impl std::error::Error for RunQueryError {}

/// The deferred page action — the image of the source's inner `IO [q]`
/// (:87: `runQueryPage` returns `IO (IO [q], Maybe UserQueryCont)`; the
/// decode is bound with `let` at :104 and only performed when the action
/// is run). Built in `run_query_page`, invoked by `run_query`/the loop's
/// `go` (:148/:152).
///
/// The `+ Send` is load-bearing: in `run_query_each_batch` the action is
/// produced on the scoped fetch thread and crosses back through the join
/// (see the bounds block); the boxed closure owns the page's encoded
/// results, so `Send` is the only extra capability needed.
///
/// The port boxes the action as `FnOnce` — NARROWER than the source's
/// re-runnable `IO [q]` (an `IO` value may be executed repeatedly; the
/// source's inner action would re-run `decodeResults`, a pure function of
/// the captured encoded results, producing identical output). Recorded
/// choice, not a silent one: every in-tree consumer runs the action
/// exactly once (`Thrift.hs:74`/`:148`/`:152`;
/// test/tests/ContinuationTest.hs runs each once), and the closure
/// consumes the owned encoded results, so a re-runnable image would have
/// to clone them per run. A future consumer that needs re-execution calls
/// `run_query_page` again; the narrowing is a compile error, never a
/// silent behavior change.
pub type Page<Q> = Box<dyn FnOnce() -> Result<Vec<Q>, RunQueryError> + Send>;

// ---------------------------------------------------------------------------
// runQueryPage (Thrift.hs:81-105)
// ---------------------------------------------------------------------------

/// Perform a query, fetching results up to the limits specified by the
/// query ('limit', 'limitBytes', 'limitTime'), and returning a
/// continuation to fetch more results if a limit was hit (:78-80).
///
/// The returned action and continuation are the source's `IO (IO [q],
/// Maybe UserQueryCont)` : the decode (`decodeResults userQueryResults_results
/// decodeAsFact`, :104) is DEFERRED into the action, while the three
/// side-effecting steps (:101-103) run eagerly here, in source order.
///
/// Bounds: `Q: Type<Q> + 'static` is what let the deferred action name
/// `Q::decode_as_fact` and be boxed as a `'static` `Page<Q>` (the
/// constructor-context image: the source's datatype context `Type a =>`
/// rides every construction — the same class internal.rs:384-394 maps);
/// `B: Backend` alone suffices — this function never spawns.
pub fn run_query_page<Q, B>(
    backend: &B,
    repo: &thrift::Repo,
    cont: Option<thrift::UserQueryCont>,
    query: &Query<Q>,
) -> Result<(Page<Q>, Option<thrift::UserQueryCont>), RunQueryError>
where
    Q: Type<Q> + 'static,
    B: Backend,
{
    // `runQueryPage be repo cont (Query query) = do let query' = query {..}`
    // (:89-99): the pattern unwraps to the spec; clone it because the
    // caller keeps its Query (do NOT into_spec — that consumes).
    let mut spec = query.query_spec.clone();
    // `Just (fromMaybe def (userQuery_options query))` (:92-93): take() so
    // the cloned record is moved, not re-cloned; None -> hsthrift def
    // (`UserQueryOptions::default()`, types.rs:156).
    let mut options = spec.options.take().unwrap_or_default();
    // `{ userQueryOptions_continuation = cont }` (:94; IDL :448).
    options.continuation = cont;
    // The source always re-wraps in Just (even for cont = Nothing) — the
    // GHC probe ghc_recexpr.hs shows `Just Nothing` for that case.
    spec.options = Some(options);
    // `userQuery_encodings = [ UserQueryEncoding_listbin def,
    // UserQueryEncoding_bin def ]` (:95-99): REPLACES the whole list,
    // listbin first — order preserved (probe: ghc_recexpr.hs).
    spec.encodings = vec![
        thrift::UserQueryEncoding::Listbin(thrift::UserQueryEncodingListBin::default()),
        thrift::UserQueryEncoding::Bin(thrift::UserQueryEncodingBin::default()),
    ];
    // `UserQueryResults{..} <- userQuery be repo query'` (:100): the trait
    // takes the request by value (types.rs:495-499).
    let results = backend.user_query(repo, spec)?;
    // `mapM_ reportUserQueryStats userQueryResults_stats` (:101). The
    // stats log is lazily forced inside report_user_query_stats's vlog.
    if let Some(stats) = &results.stats {
        report_user_query_stats(stats);
    }
    // `mapM_ (vlog 1) userQueryResults_diagnostics` (:102): each String is
    // cloned into the lazy thunk only because the loop borrows the vector
    // (the source passes each Text itself; same laziness, one copy).
    for diag in &results.diagnostics {
        vlog(1, || diag.clone());
    }
    // `mapM_ (waitBatch be repo) userQueryResults_handle` (:103): only for
    // `store`-ed queries; the handle is optional (`Maybe Handle`, IDL :789).
    if let Some(handle) = &results.handle {
        wait_batch(backend, repo, handle.clone())?;
    }
    // `let results = decodeResults userQueryResults_results decodeAsFact`
    // (:104): the source binds the action lazily and the decode runs when
    // the returned IO action is invoked (in `runQuery`'s `results <- io`,
    // :74, or the loop's `page` applications, :148/:152) — the port boxes
    // exactly that. The `move` closure's precise capture takes only
    // `results.results` (edition 2021), leaving `results.continuation`
    // readable for the return below.
    let page: Page<Q> = Box::new(move || Ok(decode_results(results.results, Q::decode_as_fact)?));
    // `return (results, userQueryResults_continuation)` (:105).
    Ok((page, results.continuation))
}

// ---------------------------------------------------------------------------
// runQuery (Thrift.hs:63-75)
// ---------------------------------------------------------------------------

/// Perform a query, fetching results up to the limits specified by the
/// query (:61-62). The returned Bool is True if the query results were
/// truncated by a limit (:68-71): `isJust cont` (:75).
pub fn run_query<Q, B>(
    backend: &B,
    repo: &thrift::Repo,
    query: &Query<Q>,
) -> Result<(Vec<Q>, bool), RunQueryError>
where
    Q: Type<Q> + 'static,
    B: Backend,
{
    // `(io, cont) <- runQueryPage backend repo Nothing query` (:73).
    let (page, cont) = run_query_page(backend, repo, None, query)?;
    // `results <- io` (:74) — the deferred decode runs here.
    let results = page()?;
    // `return (results, isJust cont)` (:75).
    Ok((results, cont.is_some()))
}

// ---------------------------------------------------------------------------
// runQuery_ (Thrift.hs:50-59)
// ---------------------------------------------------------------------------

/// Perform a query, fetching all results (:49). The source's accumulator
/// is a difference list — `id` then `\acc page -> acc . (page++)` per call
/// (:57-58), applied once to `[]` (:59); the port's Vec-and-extend loop is
/// the same concatenation in fetch order (GHC probe ghc_acc.hs; see
/// RUST-ONLY ADDITIONS — same asymptotics, no performance marker).
pub fn run_query_<Q, B>(
    backend: &B,
    repo: &thrift::Repo,
    query: &Query<Q>,
) -> Result<Vec<Q>, RunQueryError>
where
    Q: Type<Q> + 'static,
    B: Backend + Sync,
{
    let r = run_query_each_batch(backend, repo, query, Vec::new(), |mut acc, page| {
        acc.extend(page);
        Ok(acc)
    })?;
    // `return (r [])` (:59).
    Ok(r)
}

// ---------------------------------------------------------------------------
// runQueryMapPages_ (Thrift.hs:110-118)
// ---------------------------------------------------------------------------

/// Perform a query and map a function over the pages of results, running
/// the query over multiple pages if necessary (:108-109).
///
/// `runQueryEachBatch be repo query () (const fn)` (:117-118): the state
/// is the unit and the callback ignores it, so the port's closure discards
/// the first argument. Argument order is the source's: `be repo fn query`
/// — the function value comes BEFORE the query, unlike the Each functions.
pub fn run_query_map_pages_<Q, B, F>(
    backend: &B,
    repo: &thrift::Repo,
    mut f: F,
    query: &Query<Q>,
) -> Result<(), RunQueryError>
where
    Q: Type<Q> + 'static,
    B: Backend + Sync,
    F: FnMut(Vec<Q>) -> Result<(), RunQueryError>,
{
    run_query_each_batch(backend, repo, query, (), |_, page| f(page))
}

// ---------------------------------------------------------------------------
// runQueryEach (Thrift.hs:124-133)
// ---------------------------------------------------------------------------

/// Perform a query by pages, mapping a state function over the results in
/// each page; pages are pipelined (fetch of page N+1 concurrent with the
/// processing of page N, via run_query_each_batch) (:120-123).
///
/// `runQueryEachBatch be repo query s (foldM fn)` (:132-133): `foldM fn`
/// threads `s` left-to-right over the elements of one page (GHC probe
/// ghc_acc.hs); the `for` loop is that fold. `f` is borrowed mutably so
/// the same closure serves every element of every page.
pub fn run_query_each<Q, B, S, F>(
    backend: &B,
    repo: &thrift::Repo,
    query: &Query<Q>,
    init: S,
    mut f: F,
) -> Result<S, RunQueryError>
where
    Q: Type<Q> + 'static,
    B: Backend + Sync,
    F: FnMut(S, Q) -> Result<S, RunQueryError>,
{
    run_query_each_batch(backend, repo, query, init, |mut s, page| {
        for x in page {
            s = f(s, x)?;
        }
        Ok(s)
    })
}

// ---------------------------------------------------------------------------
// runQueryEachBatch (Thrift.hs:136-153)
// ---------------------------------------------------------------------------

/// Like runQueryEach, but process results in batches (:135).
///
/// The source's `go` (:147-153):
///
///     go page Nothing s = f s =<< page
///     go page cont@Just{} s = do
///       ((nextPage, nextCont), s2) <- concurrently
///         (runQueryPage be repo cont query)
///         (f s =<< page)
///       go nextPage nextCont s2
///
/// The port runs the fetch on a scoped thread and the processing on the
/// calling thread; each iteration consumes the current page action, joins
/// the fetch, then continues with (next_page, next_cont, s2) — the same
/// page chain and the same state threading as `go`.
pub fn run_query_each_batch<Q, B, S, F>(
    backend: &B,
    repo: &thrift::Repo,
    query: &Query<Q>,
    init: S,
    mut f: F,
) -> Result<S, RunQueryError>
where
    Q: Type<Q> + 'static,
    B: Backend + Sync,
    F: FnMut(S, Vec<Q>) -> Result<S, RunQueryError>,
{
    // `(page, cont) <- runQueryPage be repo Nothing query` (:145).
    let (mut page, mut cont) = run_query_page(backend, repo, None, query)?;
    let mut s = init;
    loop {
        match cont {
            // `go page Nothing s = f s =<< page` (:148).
            None => {
                let results = page()?;
                return f(s, results);
            }
            // `go page cont@Just{} s = ...` (:149-153).
            Some(c) => {
                // TODO(port): async-cancellation divergence — the source's
                // `concurrently` (:150-152) cancels the losing side as soon
                // as either side throws; this port is the RULEBOOK §2 B2
                // default, let-drain. Both sides run to completion and the
                // PROCESSING side's error wins when both fail (an error
                // from the fetch propagates only when processing
                // succeeded). `thread::scope` joins the fetch thread on
                // closure exit even through the early `?` returns below —
                // that is the let-drain property: the in-flight request
                // completes with real results instead of being abandoned.
                // A panicked fetch resumes its payload on this thread at
                // the explicit join; when the process side `?`-returned
                // first, `thread::scope` instead raises its OWN "a scoped
                // thread panicked" panic while joining the unjoined fetch
                // (std's behavior — the fetch payload is not unwrapped
                // then, and the process-side error is discarded; probes
                // /tmp/opencode/db47-probe/scope_panic.rs and
                // /tmp/opencode/db47-reviewA/pipeline_probe.rs scenarios
                // 7-8).
                let (next_page, next_cont, s2) =
                    std::thread::scope(|scope| -> Result<_, RunQueryError> {
                        let fetch = scope.spawn(|| run_query_page(backend, repo, Some(c), query));
                        // Process the current page on this thread,
                        // concurrently with the fetch (:152).
                        let current = page()?;
                        let s2 = f(s, current)?;
                        let (next_page, next_cont) = match fetch.join() {
                            Ok(fetched) => fetched?,
                            // A panicked fetch resumes on this thread — the
                            // closest image of the source's exception
                            // propagation out of `concurrently`.
                            Err(payload) => std::panic::resume_unwind(payload),
                        };
                        Ok((next_page, next_cont, s2))
                    })?;
                page = next_page;
                cont = next_cont;
                s = s2;
            }
        }
    }
}

// ===========================================================================
// Coverage / markers / status
// ===========================================================================
//
// Coverage: the 153-line source is fully accounted for. The export list
// (:9-34, 19 names) maps one-to-one per the EXPORT MAPPING table: six new
// functions (run_query_/run_query/run_query_page/run_query_map_pages_/
// run_query_each/run_query_each_batch) plus 13 re-exports (Query from the
// owner; the 12 Internal names). All six doc comments survive at their
// functions (:49, :61-62/:68-71, :78-80, :108-109, :120-123, :135), every
// import of :36-47 is disposed in the import block, and all four outbound
// edges are consumed. The two Rust-only additions (RunQueryError, Page)
// are documented above. Nothing is dropped, stubbed, or deferred.
//
// Marker breakdown (closed set — one port todo, zero of every other
// class):
//   * port todos: one — the B2 async-cancellation divergence marker at
//     the run_query_each_batch concurrency site (the RULE §2 B2 obligation;
//     real threads, let-drain with scope-join, processing error first, join
//     panic resumed). It is the only marker token in the file.
//   * source-defect reproductions: zero — the source is total (no `error`,
//     no `head`/`fromJust`; the wrong-encoding partial path lives in row
//     45's decode_results and is already a typed Err there).
//   * performance deferrals: zero — the difference-list-to-Vec mapping has
//     the same asymptotics and is quotient-equal (GHC probe), not a
//     deferral; the scoped-thread pipeline is the required B2 shape, not
//     an optimization escape.
//   * safety comments: zero — no unsafe. The concurrency is 100% safe
//     `std::thread::scope`; no `'static` laundering.
//   * inventory divergence: zero — no inventory row names this file (the
//     INVENTORY block above records the greps).
//
// PORT STATUS: confidence=high todos=1
