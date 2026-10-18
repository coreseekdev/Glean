// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// Ported from: glean/backend-api/Glean/Query/Thrift/Internal.hs (253 lines)
// Manifest unit: rust/glean-db/src/query/thrift/internal.rs
// (migration/manifest.tsv line 45, crate glean-db). RULE §4: one source
// file -> one target file; the manifest path is authoritative (the
// `query/thrift/` subdirectory is generated from the module path
// `Glean.Query.Thrift.Internal`, exactly as src/backend/types.rs was from
// `Glean.Backend.Types`).
//
// WHAT THIS FILE IS: the internal half of the query facade
// (`Glean.Query.Thrift` body, Internal.hs:14 "Define internal pieces,
// please import Glean.Query.Thrift instead"). It has three parts:
//   * the query combinators (:64-242): construction over the generated
//     `thrift::UserQuery` spec (`query`, `angle`, `angleData`, `allFacts`),
//     retyping (`keys`), and flags-only option updates (`recursive`,
//     `expanding`, `limit`, `limitBytes`, `limitTime`, `store`, `justCheck`,
//     `dbgPredHasFacts`);
//   * result decoding (:87-117): the `UserQueryEncodedResults` union ->
//     `Vec<a>` through a caller-supplied decoder;
//   * support (:119-121, :244-253): spec display and the `UserQueryStats`
//     formatter/logger.
//
// MODULE WIRING (RULE §4): this unit also adds the crate's Query-area
// scaffolding — src/query.rs (Rust-only, prose-only: Haskell has no
// `Glean.Query` root module) and `pub mod query;` in src/lib.rs. The
// remaining link is row 47's (src/query/thrift.rs, manifest.tsv line 47):
// it appends `pub mod thrift;` at the top of src/query.rs and declares
// `pub mod internal;` in src/query/thrift.rs, which is what binds this
// file's path as `crate::query::thrift::internal`. Until row 47 lands the
// path is an ordinary forward reference (the same class as
// `crate::database::types::Env` / `crate::haxl::engine` named in
// src/lib.rs:28-37), documented in src/query.rs's own contract block.
//
// ---------------------------------------------------------------------------
// INVENTORY (RULE §5; migration/inventory.tsv)
// ---------------------------------------------------------------------------
//
// NO row's source file is this file. Three rows govern the constructs used
// here, and none is followed silently:
//
//   * inventory.tsv:56 (twin row: inventory.stm.tsv:35) — source
//     glean/haxl/Haxl/DataSource/Glean/Common.hs, construct "StateStore
//     caches", class BATCHING. It is the ONLY row in inventory.tsv whose
//     evidence names this file: "Thrift/Internal.hs:98,106 cacheRef
//     newIORef IntMap.empty" — exactly the two per-arm caches
//     decode_results allocates. Its ruling ("dissolve into BatchCtx ...
//     haxl's scheduler
//     is the only writer") is scoped to the Common.hs StateStore caches;
//     the haxl half is owned by row 146's unit. Here both caches are
//     synchronous per-CALL locals with no cross-request scope to dissolve.
//     The port keeps the locals and does NOT invent a BatchCtx dependency;
//     divergence flagged at the site (the file's one inventory flag).
//     NOTE: the stm variant's cross-reference "per inventory.tsv rows
//     17-18" is stale — inventory.tsv:17-18 are the ShellPrint lazy-pipe
//     row and the Write.hs Template-Haskell row; the haxl-area BATCHING
//     rows are inventory.tsv:8-9, :11-12, :14, :16, :52 and :55-57.
//   * inventory.tsv:43 — source glean/db/Glean/Write/JSON.hs, construct
//     "JSON FactBuilder": its disposition sketch explicitly covers the
//     decode cache this file carries — "Build.hs decode cache (newIORef
//     IntMap per decode call, Build.hs:84-88) -> local HashMap<u64,T> &mut.
//     No cross-thread sharing anywhere in either call path". Both caches
//     here are that class: locals + `&mut`, no synchronisation. The
//     sketch's `u64` key vs the port's `i64` is the divergence already
//     flagged at rust/glean-typed/src/typed/build.rs:32-38 (same class,
//     same owner) and is not re-flagged here (one flag per divergence);
//     this file's key must be `i64` because the SHIPPED decoder borrows
//     `&HashMap<i64, Fact>` (fact.rs:272-274).
//   * inventory.tsv:135 — source glean/typed/Glean/Typed/Predicate.hs,
//     construct the KeyType/ValueType type families: consumed by `keys`
//     (the `R::KeyType: Type<R::KeyType>` clause is the `Type (KeyType p)`
//     superclass of Predicate.hs:64). The row's "concrete types ... NO
//     generic trait fork" divergence is flagged at
//     rust/glean-typed/src/typed/predicate.rs:255-267; this file's consumer
//     clause rides that landed decision and is not re-flagged.
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
//   * `PredicateRef` — first sketch owned by
//     rust/glean-angle/src/angle/types.rs:52-75; reached through the shipped
//     consumer rust/glean-typed/src/typed/predicate.rs:83-96 (`pub struct
//     PredicateRef { pub name: String, pub version: Version }`).
//   * `Fact` — first sketch owned by rust/glean-typed/src/typed/build.rs:
//     104-105; reached through the shipped consumer
//     rust/glean-typed/src/typed/fact.rs:243-245 (`use glean_thrift::Fact;`).
//
// THIS UNIT'S EXTENSION: the query-shaped members, none used by an earlier
// shipped unit. IDL pins (glean/if/glean.thrift), assumed Rust image in the
// same block:
//
//     pub type Id = i64;                    // :31; no hs.newtype
//     pub type Version = i32;               // :39; gen-hs2/Types.hs:317
//
//     struct SourcePredicate { pub name: String,
//                              pub version: Option<Version> }   // :62-65
//
//     enum QuerySyntax { JSON = 1, ANGLE = 2 }  // :403-405, annotation :406
//         // (hs.nounknown). Value-name spelling ASSUMED as fbthrift's Rust
//         // codegen emits it (that generator is not on disk until Step 4
//         // builds glean_thrift; sentence shape util/service.rs:66-71,
//         // angle/types.rs:73-75). Both use sites write
//         // `QuerySyntax::ANGLE`, a path that names the item the same way
//         // whether the codegen emits enum variants or associated consts;
//         // if the generated spelling differs, only angle_query/angle_data
//         // change — nothing matches on QuerySyntax.
//
//     struct UserQueryOptions {  // :408-474; Default == hsthrift `def`
//         pub expand_results: bool,          // :411 (IDL default true)
//         pub recursive: bool,               // :414
//         pub max_results: Option<i64>,      // :419
//         pub max_bytes: Option<i64>,        // :428
//         pub max_time_ms: Option<i64>,      // :437
//         pub syntax: QuerySyntax,           // :452 (IDL default JSON;
//                                            // written at angle_query/
//                                            // angle_data only)
//         pub store_derived_facts: bool,     // :455
//         pub debug: QueryDebugOptions,      // :461 (struct, not Option)
//         pub expand_predicates: Vec<SourcePredicate>, // :470
//         pub just_check: bool,              // :473
//         /* no_base64_binary, continuation, collect_facts_searched,
//            omit_results, ... */ }
//
//     struct QueryDebugOptions { pub ir: bool, pub bytecode: bool,
//                                pub pred_has_facts: bool }   // :476-486
//
//     struct UserQuery {                    // :640-668
//         pub predicate: String,            // :641 (deprecated, required)
//         pub query: Vec<u8>,               // :643
//         pub predicate_version: Option<Version>, // :646
//         pub options: Option<UserQueryOptions>,  // :656
//         pub encodings: Vec<UserQueryEncoding>,  // :658 (default [])
//         /* client_info :662, schema_id :667 */ }
//         // Default = gen-hs2/Types.hs:5883-5888 (`def`): "" / empty /
//         // Nothing / Nothing / []
//
//     struct UserQueryStats {               // :705-730
//         pub num_facts: i64,               // :707
//         pub elapsed_ns: i64,              // :709
//         pub allocated_bytes: i64,         // :711
//         /* ... */ }
//
//     struct UserQueryResultsBin {          // :733-737
//         pub encoding: UserQueryEncodingBin,
//         pub facts: BTreeMap<Id, Fact>,        // :735
//         pub nested_facts: BTreeMap<Id, Fact> } // :736
//
//     struct UserQueryResultsListBin {      // :739-744
//         pub encoding: UserQueryEncodingListBin,
//         pub ids: Vec<Id>,                  // :741 (list_Id_2029, :1343)
//         pub facts: Vec<Fact>,              // :742 (list_Fact_2137, :1342)
//         pub nested_facts: BTreeMap<Id, Fact> } // :743
//
//     union UserQueryEncodedResults {       // :761-766
//         Empty,                             // union's unset value ==
//                                            // hsthrift
//                                            // UserQueryEncodedResults_EMPTY
//                                            // (gen-hs2 :7100; def :7178)
//                                            // — the SelectSchema Empty
//                                            // convention
//                                            // (backend/types.rs:180-186)
//         Bin(UserQueryResultsBin),          // :762
//         Json(UserQueryResultsJSON),        // :763
//         Compact(UserQueryResultsCompact),  // :764
//         Listbin(UserQueryResultsListBin) } // :765
//         // variant names = UpperCamel of the thrift field, the SelectSchema
//         // sketch convention (backend/types.rs:180-186); if the generated
//         // crate spells one differently, only the match arms in
//         // decode_results change, and the wildcard arm stays — it also
//         // absorbs `Empty`, exactly as the source's `_other` (:116-117) did
//
// The Haskell list typedefs are `Vector` (glean.thrift:1342-1343
// `hs.type = "Vector"`), rendered `Vec` here per the crate image above; the
// Haskell maps are Data.Map, rendered `BTreeMap` per types.rs:153-154.
//
// ---------------------------------------------------------------------------
// ASSUMED-IMPORT (RULE §1 logging row (A10)): the `glean_util::log` seam
// ---------------------------------------------------------------------------
//
// `import Util.Log` (Internal.hs:57) has NO manifest unit: Util.Log is
// hsthrift's glog shim (~/glean-build/Glean/hsthrift/common/util/Util/
// Log.hs:9-16, re-exporting Util.Log.String). RULEBOOK.md:38 (A10) pins the
// image as a `glean_util::log` module with LAZY message thunks ("message
// never forced unless verbose"), because `vlog level msg` only forces `msg`
// when `vlogIsOn level` (Util.Log.String.hs:36-41):
//
//     pub fn vlog(level: i64, msg: impl FnOnce() -> String);
//
// The module does not exist on disk yet; this file writes the FINAL call
// site against the pinned shape (the same forward-reference form
// `glean_thrift` is consumed by, and the form src/backend/types.rs uses for
// its future same-crate modules), so the swap when glean-util's log unit
// lands is only the import resolution, not a call-site edit. The interior
// crates' precedent for the same seam is the private stand-in at
// rust/glean-util/src/util/retry_channel_exception.rs:311-324 (unit 43,
// under that unit's brief); it is that file's to delete, not this one's.
// The closure below captures `stats` by reference and `vlog` consumes it
// within the call, so no `'static` demand arises (the pinned shape takes
// `impl FnOnce`, not a boxed thunk).
//
// ---------------------------------------------------------------------------
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, module-level)
// ---------------------------------------------------------------------------
//
// Outbound (rows 25-27): Internal.hs -> Backend/Types.hs (:25),
// Query/Angle.hs (:26), Typed.hs (:27) — the only source imports with
// manifest units. Inbound (rows 22/74/86/1221): Query/Thrift.hs (:22),
// client/hs/Glean.hs (:74), client/hs/Glean/Angle.hs (:86), and
// haxl/.../Glean/Common.hs (:1221, the haxl consumer of these
// combinators — the same file inventory.tsv:56 governs from the other
// side). `Glean.Types` (:61) is generated and carries NO edge row
// (generated modules are absent from the depmap — the reason it is an
// assumed import).
//
// Every import of Internal.hs:39-62 has a disposition:
//   * Control.Exception (:39; throwIO, ErrorCall) -> the typed
//     `DecodeError::ErrorCall` arm (build.rs:364-374; RULE §2 exceptions
//     row, RULEBOOK.md:79).
//   * Control.Monad (:40; forM, zipWithM) -> `for` / `Iterator::zip`.
//   * Control.Monad.IO.Class (:41; liftIO) -> erased: the port's decoder
//     and entry points are pure/typed, no IO monad.
//   * Data.Default (:42; def) -> `Default::default()` and
//     `Option::take().unwrap_or_default()`; the thrift structs' Default is
//     pinned equal to hsthrift `def` (backend/types.rs:156).
//   * Data.Dynamic (:43; Dynamic) -> `Box<dyn Any>` (the shipped decoder
//     cache type, build.rs:198/:416).
//   * Data.IntMap (:44-45; IntMap, qualified IntMap) -> `HashMap<i64, _>`;
//     the key image is the class build.rs:32-38 flags (Int64 -> Int
//     identity on the pinned 64-bit target; here the key stays i64 to
//     match fact.rs:272-274).
//   * Data.IORef (:46; newIORef) -> per-call `let mut` locals (rows 43/56;
//     the divergence flag sits at the first cache site).
//   * Data.Map (:47; Map.toList) -> BTreeMap iteration: ascending key
//     order, Data.Map.toList's order.
//   * Data.Maybe (:48; fromMaybe) -> `Option::take().unwrap_or_default()`.
//   * Data.Proxy (:49; Proxy) -> erased; associated functions carry the
//     type (binary.rs:263-267 precedent).
//   * Data.Text / qualified Data.Text.Encoding (:50-52; Text, decodeUtf8,
//     encodeUtf8, pack) -> `&str`/`String` + `encode_utf8` + the local
//     `decode_utf8_strict` (RULE §1 strings row, RULEBOOK.md:36).
//   * Compat.Prettyprinter (:53; Pretty(..)) -> the shipped
//     `predicate_ref` rendering (predicate.rs:557-566); one
//     prettyprinting owner (RULEBOOK.md:37).
//   * Data.Vector (:54; Vector.toList) -> slice/Vec iteration (the IDL
//     typedefs are `Vector` in Haskell, `Vec` here).
//   * Text.Printf (:55; printf) -> `show_ffloat_2` + `format!` (no std
//     `%.2f` produces GHC's shortest-digit rounding; see the helper).
//   * Util.Log (:57; vlog) -> the A10 seam above.
//   * Glean.Query.Angle (:59) -> `glean_typed::query::angle` (row 26;
//     `Angle`, `display`, `sig`).
//   * Glean.Typed as Typed (:60) -> `glean_typed::typed` (row 27; the
//     facade's curated re-exports, typed.rs:148-178).
//   * Glean.Types as Thrift (:61) -> `glean_thrift as thrift` (generated).
//   * Glean.Backend.Types (:62; Query(..)) -> `crate::backend::types`
//     (row 25; re-exported at the top of the code below).
//
// ---------------------------------------------------------------------------
// SIBLING-UNIT CONTRACTS
// ---------------------------------------------------------------------------
//
//   * crate::backend::types (row 41, SHIPPED) — owns `Query<A>` (:1397-1401
//     with `pub query_spec` and a private `_marker`), `Query::new`
//     (:1403-1413, `impl<A: Type<A> + 'static>`) and `into_spec`
//     (:1415-1422, unbounded). Its `into_spec` doc (:1386-1396) names THIS
//     unit as the consumer that must rebuild rather than functional-update:
//     the eleven record sites :133/:139/:164/:176/:188/:200/:208/:218/
//     :227/:233/:239 all go through into_spec + new below. The source's
//     `import Glean.Backend.Types (Query(..))` (:62) plus its `Query(..)`
//     export (:17) collapse into the single `pub use` in the code block.
//   * glean_typed::query::angle (row 44, SHIPPED) — `Angle<T>` (:488-491),
//     `display` (:575-577), `sig` (:1370-1376). Both functions take the
//     Angle BY VALUE (the source's pure value is freely passed along), so
//     `query` threads `a` through sig -> display exactly as `angleData .
//     display . sig` does (:82).
//   * glean_typed::typed (SHIPPED) — `Fid`/`IdOf` re-exports (typed.rs:169;
//     `IdOf::new(Fid(id))` is the source's `Typed.IdOf (Fid fid)`,
//     id.rs:197-217), `Predicate` (predicate.rs:268-271), `Type`
//     (binary.rs:235, `Type<A: 'static>`), `predicate_ref`
//     (predicate.rs:557-566), `DecodeError` (build.rs:364-374 — imported at
//     its module path because `Glean.Typed`'s export list does NOT re-export
//     it, typed.rs:154-157), and `decode_fact` (fact.rs:272-282).
//     `decode_fact::<P>`'s parameter list IS the D bound below, so row 47
//     can pass `decode_fact`/`A::decode_as_fact` directly; probe-verified
//     (/tmp/opencode/db45-probe/probe_bounds.rs).
//   * src/query.rs (this unit, Rust-only) — its prose block records row 47's
//     two wiring edits (see MODULE WIRING above); this file does not declare
//     them.
//   * row 47 (src/query/thrift.rs, NOT SHIPPED) — the facade that imports
//     this module (`import Glean.Query.Thrift.Internal`, Thrift.hs:46) and
//     re-exports 13 of the 18 names (its export list, Thrift.hs:9-34,
//     omits `reportUserQueryStats`/`showUserQueryStats`/`decodeResults`/
//     `justCheck`/`dbgPredHasFacts`) and consumes `reportUserQueryStats`
//     (:101) and `decodeResults` (:104) internally (the former reaches
//     `showUserQueryStats` through this module). Its `runQueryPage`
//     (:89-105) is the caller of `decode_results`:
//     `decodeResults userQueryResults_results decodeAsFact` (:104), and of
//     `report_user_query_stats` (:101).
//   * row 146 (src/haxl/data_source/glean/common.rs, NOT SHIPPED) — owns
//     the haxl half of inventory.tsv:56 (the StateStore caches the row
//     actually dissolves into BatchCtx) and is the consumer of the
//     `Query` combinators through the datasource (:1221 inbound edge).

// ---------------------------------------------------------------------------
// EXPORT MAPPING (Internal.hs:16-37 — 18 exported names; source order, not
// alphabetical: Types :17, combinators :19-29, support :31-36)
// ---------------------------------------------------------------------------
//
//    #  Internal.hs                              Rust image
//   --  --------------------------------------  --------------------------
//    1  Query(..)                         :17   pub use crate::backend::types::Query;
//                                               (struct + Query::new + into_spec;
//                                               `_marker` is private, so the
//                                               literal constructor+field export
//                                               cannot be reproduced — documented
//                                               at types.rs:1386-1396)
//    2  query                             :19   query
//    3  angle                             :20   angle
//    4  angleData                         :21   angle_data
//    5  keys                              :22   keys
//    6  recursive                         :23   recursive
//    7  limit                             :24   limit
//    8  limitBytes                        :25   limit_bytes
//    9  limitTime                         :26   limit_time
//   10  store                             :27   store
//   11  allFacts                          :28   all_facts
//   12  expanding                         :29   expanding
//   13  reportUserQueryStats              :31   report_user_query_stats
//   14  showUserQueryStats                :32   show_user_query_stats
//   15  displayQuery                      :33   display_query
//   16  decodeResults                     :34   decode_results
//   17  justCheck                         :35   just_check
//   18  dbgPredHasFacts                   :36   dbg_pred_has_facts
//
// NOT EXPORTED, still ported (source-private): `type ResultDecoder a`
// (:87-92) -> the `D` bound on `decode_results` (no named alias: its image
// is a trait bound, not a type); `angleQuery` (:141-151) -> private
// `angle_query`. Nothing else is defined in the source.
//
// ---------------------------------------------------------------------------
// RUST-ONLY ADDITIONS AND MAPPING CHOICES
// ---------------------------------------------------------------------------
//
//   * `decode_utf8_strict` (private) — the image of `Text.decodeUtf8`
//     (:121). angle.rs keeps its byte-pinned helper PRIVATE
//     (angle.rs:1085-1111), so this file carries an exact copy (same
//     message, same panic) so the behavior is not re-derived; Step-4
//     consolidation may hoist one shared helper into glean-typed. The
//     source THROWS UnicodeException from pure code; the shipped sibling's
//     pinned choice (panic with Data.Text's message) is re-used.
//   * `show_ffloat_2` / `round_to` / `int_to_digit` (private) — the image
//     of `printf "%.2f"` (:250). Rust's `{:.2}` rounds the exact binary
//     expansion; GHC rounds the SHORTEST round-trip digits ("2.675" ->
//     "2.68" in GHC, "2.67" in Rust), so the helper replicates
//     GHC.Float's formatRealFloatAlt FFFixed (Just 2) + roundTo — the
//     base-4.17.1.0 source text, identical to the pinned GHC 9.4.7's
//     base-4.17.2.0 for GHC.Float/Numeric/Text.Printf. Probes:
//     /tmp/opencode/db45-probe/ffmt.rs (helper + battery),
//     ffmt_samples.txt/ffmt_hs_ns.out (2,700,010 ns inputs: 0 mismatches;
//     exhaustive over the first millisecond, boundary residues and LCG
//     spread over the whole i64 range) and ffmt_bits.txt/
//     ffmt_hs_bits.out (200,033 bit patterns: 48 mismatches, ALL with
//     |x| >= 2^54, where GHC's floatToDigits stops emitting minimal
//     digits; both sides round-trip; outside the reachable domain below).
//   * Domain: `elapsed_ns` is i64, so |x| <= 9223372036854775807 /
//     1_000_000 = 9.2233720368547758e12 < 2^54 — exact over every input
//     this call site can see. The quirk at |x| >= 2^54 is left as a
//     documented residual, not a flag (the residual-risk doc-note
//     precedent, angle.rs:1092-1094): it cannot be reached through
//     `UserQueryStats.elapsed_ns`.
//   * Representation-imposed clauses: every function that CONSTRUCTS a
//     Query carries `A: Type<A> + 'static` (the shipped constructor's impl
//     bound, types.rs:1403; `Type<A: 'static>` is binary.rs:235). This is
//     the image of the source datatype context `data Query a = Type a =>
//     Query` (backend-api/Glean/Backend/Types.hs:346): in Haskell the
//     context rides every construction/pattern-match; in Rust only
//     construction needs it. `keys` additionally needs `R::KeyType:
//     Type<R::KeyType> + 'static` — the source's `Type (KeyType p)`
//     superclass (Predicate.hs:64); without it rustc rejects the call to
//     `Query::new` with E0277 (negative probe
//     /tmp/opencode/db45-probe/neg_keys.rs; positive probe probe_bounds.rs).
//   * `Int` -> `i64` for `limit`/`limitBytes`/`limitTime` (:199/:207/:215):
//     the source's `fromIntegral n` targets the IDL's `optional i64` fields
//     (:419/:428/:437) and is the identity on the pinned 64-bit target —
//     the same Int64 <-> Int identity class build.rs:32-38 pins. No
//     wrap/truncation semantics can differ.
//   * Effects: `throwIO $ ErrorCall` (:116-117) -> `Err(DecodeError::
//     ErrorCall(..))` (RULEBOOK.md:79); `IO [a]` -> `Result<Vec<A>,
//     DecodeError>`; `liftIO` erased. `reportUserQueryStats :: ... -> IO ()`
//     (:244-245) -> a plain call whose laziness lives at the log seam.
//   * Ownership/borrows: the combinators take `q: Query<A>` by value (the
//     Haskell chains them linearly — `limit n $ recursive $ query ...` —
//     and `into_spec` consumes); `display_query`/`report_user_query_stats`/
//     `show_user_query_stats` borrow (`&Query<A>`/`&UserQueryStats`) — pure
//     reads whose callers keep ownership; by value, every call site would
//     clone the heap value for no gain (the shipped pure-render shape:
//     `display_default`/`display_verbose` take `&T`,
//     glean-angle/src/display.rs:592-597).
//   * `all_facts` uses the shipped `predicate_ref::<R>()` instead of
//     re-deriving `show (pretty (getName (Proxy @q)))` (:124-125):
//     `predicate_ref` renders exactly `name.version` (predicate.rs:557-566,
//     format at :565), which is that expression's text; one
//     prettyprinting owner per RULEBOOK.md:37.
//   * Haskell pragmas (:9-13: TypeApplications, ConstraintKinds,
//     TypeFamilyDependencies, InstanceSigs, AllowAmbiguousTypes) have no
//     Rust image — language mechanics, no semantics.

use std::any::Any;
use std::collections::HashMap;

use glean_thrift as thrift;
use glean_thrift::Fact;
use glean_typed::query::angle::{display, sig, Angle};
use glean_typed::typed::build::DecodeError;
use glean_typed::typed::{predicate_ref, Fid, IdOf, Predicate, Type};
// ASSUMED-IMPORT (RULE §1 logging row (A10)): see the header block — the
// future `glean_util::log` surface, consumed at its pinned shape.
use glean_util::log::vlog;

// The source's `import Glean.Backend.Types (Query(..))` (:62) AND its
// `Query(..)` export (:17): one public re-export (the struct; the
// constructor is `Query::new` and the selector `into_spec`, types.rs:
// 1403-1422 — the private `_marker` makes a literal `Query(..)` export
// impossible). Consumers (this crate's row 47 facade and the CLI/haxl
// importers) get the type through here exactly as they got it through
// Internal.hs.
pub use crate::backend::types::Query;

// ---------------------------------------------------------------------------
// query (Internal.hs:81-85)
// ---------------------------------------------------------------------------

/// `query :: Type t => Angle t -> Query t` (Internal.hs:81-82) — "Build a
/// query. It can returns facts of a predicate: ... Or arbitrary values:
/// ..." (source doc :64-80, examples elided here but preserved in shape).
///
/// `query = angleData . display . sig` (:82). The port composes the same
/// three shipped functions right-to-left: `sig(a)` adds the type
/// signature (angle.rs:1370-1376), `display` renders the Angle text
/// (angle.rs:575-577), `angle_data` wraps it. The source's note
/// (:83-85) is load-bearing: the `sig` call makes the query's type agree
/// with the type Glean infers; without it the two "could diverge, leading
/// to deserialization errors or just wrong data".
pub fn query<A: Type<A> + 'static>(a: Angle<A>) -> Query<A> {
    angle_data(&display(sig(a)))
}

// ---------------------------------------------------------------------------
// decodeResults (Internal.hs:87-117)
// ---------------------------------------------------------------------------

/// `decodeResults :: UserQueryEncodedResults -> ResultDecoder a -> IO [a]`
/// (Internal.hs:94) — decode one page of results.
///
/// The source's private `type ResultDecoder a` (:87-92) is
/// `IntMap Thrift.Fact -> IORef (IntMap Dynamic) -> IdOf a -> Thrift.Fact
/// -> IO a`. Its Rust image is the `D` bound here (no named alias — the
/// image is a trait bound):
///   * `IntMap Thrift.Fact` (serialized nested facts) -> `&HashMap<i64,
///     Fact>` — the SHIPPED decoder's parameter shape (fact.rs:272-274),
///     so `decode_fact::<P>`/`A::decode_as_fact` satisfies `D` directly;
///   * `IORef (IntMap Dynamic)` -> `&mut HashMap<i64, Box<dyn Any>>` (the
///     row-43 cache class: local, no synchronisation);
///   * `IdOf a`/`Thrift.Fact` -> `&IdOf<A>`/`&Fact` (by shared borrow, the
///     decoder only reads them);
///   * `IO a` -> `Result<A, DecodeError>` (RULE §2 exceptions row; the
///     source's throwIO sites are decoder-internal, and this function's
///     own throw becomes the `Err` arm below).
///
/// Nothing is constrained on `A` — exactly as the source's signature
/// constrains nothing (the decoder carries its own bounds at its
/// definition; the trait bound `D` is where they surface here).
///
/// Arm order and semantics, mirroring :95-117:
///   * `bin` (:97-103): a FRESH cache per call (:98), the nested facts
///     materialized into the serialized map (:99-101), then the result
///     facts walked in `Map.toList` order — BTreeMap iteration is the same
///     ascending key order — each decoded with `IdOf (Fid fid)` (:102-103).
///   * `listbin` (:105-114): fresh cache (:106), the same serialized map
///     (:107-109), then `zipWithM f fids facts` (:110-114) — Rust `zip`
///     truncates to the shorter vector exactly as zipWithM does, and both
///     run left-to-right.
///   * anything else (:116-117): `throwIO $ ErrorCall "runQueryPage:
///     server returned the wrong encoding"` -> `Err(DecodeError::
///     ErrorCall(...))` with the byte-identical message. The wildcard also
///     absorbs a future generated union member (the source's `_other`), so
///     no generated-crate variant naming can make this arm unreachable
///     silently.
pub fn decode_results<A, D>(
    results: thrift::UserQueryEncodedResults,
    decoder: D,
) -> Result<Vec<A>, DecodeError>
where
    D: Fn(
        &HashMap<i64, Fact>,
        &mut HashMap<i64, Box<dyn Any>>,
        &IdOf<A>,
        &Fact,
    ) -> Result<A, DecodeError>,
{
    match results {
        // UserQueryEncodedResults_bin UserQueryResultsBin{..} (:97)
        thrift::UserQueryEncodedResults::Bin(bin) => {
            // cacheRef <- newIORef IntMap.empty (:98).
            //
            // INVENTORY-BUG-CANDIDATE row 56: the row's evidence names these
            // two caches, but its BatchCtx ruling is scoped to the haxl
            // StateStore caches it actually governs (Common.hs — row 146's
            // unit); here they are per-call locals, so the port keeps local
            // HashMaps (the inventory.tsv:43 class: local, `&mut`, no
            // synchronisation) instead of inventing a BatchCtx dependency.
            // Flagged per RULE §5/A9 (RULEBOOK.md:139-141).
            let mut cache: HashMap<i64, Box<dyn Any>> = HashMap::new();
            // let serialized = IntMap.fromList [ (fromIntegral id, f) |
            // (id, f) <- Map.toList nestedFacts ] (:99-101). `fromIntegral
            // id` (Id -> Int) is the identity on the pinned 64-bit target
            // (build.rs:32-38 class), and the decoder keys by i64, so the
            // id crosses unchanged.
            //
            // PERF(port): the source builds ONE lazy IntMap and shares it
            // with every decoder call; the shipped decoder borrows
            // `&HashMap<i64, Fact>` (fact.rs:272-274), so this arm (and
            // the listbin arm below) materializes the generated BTreeMap
            // into an owned HashMap with one clone per nested fact.
            // Faithful; fast version: widen the decoder to the generated
            // `BTreeMap<Id, Fact>` (or memoize the materialization) —
            // deferred because that signature is a SHIPPED sibling's
            // (fact.rs), outside this unit's edit set.
            let serialized: HashMap<i64, Fact> = bin
                .nested_facts
                .iter()
                .map(|(&id, nested)| (id, nested.clone()))
                .collect();
            // forM (Map.toList userQueryResultsBin_facts) $ \(fid, fact)
            // -> decoder serialized cacheRef (IdOf (Fid fid)) fact
            // (:102-103). Ascending BTreeMap order == Data.Map.toList.
            let mut out = Vec::with_capacity(bin.facts.len());
            for (&fid, fact) in &bin.facts {
                out.push(decoder(
                    &serialized,
                    &mut cache,
                    &IdOf::new(Fid(fid)),
                    fact,
                )?);
            }
            Ok(out)
        }
        // UserQueryEncodedResults_listbin UserQueryResultsListBin{..}
        // (:105)
        thrift::UserQueryEncodedResults::Listbin(listbin) => {
            // cacheRef <- newIORef IntMap.empty (:106) — fresh per call,
            // like the bin arm (its inventory note and flag sit there).
            let mut cache: HashMap<i64, Box<dyn Any>> = HashMap::new();
            // serialized = ... (:107-109) — the same materialize-and-clone
            // as the bin arm; see the performance note there.
            let serialized: HashMap<i64, Fact> = listbin
                .nested_facts
                .iter()
                .map(|(&id, nested)| (id, nested.clone()))
                .collect();
            // fids = Vector.toList ids; facts = Vector.toList facts;
            // zipWithM f fids facts (:110-114). `zip` stops at the shorter
            // side exactly as zipWithM does.
            let mut out = Vec::new();
            for (&fid, fact) in listbin.ids.iter().zip(listbin.facts.iter()) {
                out.push(decoder(
                    &serialized,
                    &mut cache,
                    &IdOf::new(Fid(fid)),
                    fact,
                )?);
            }
            Ok(out)
        }
        // _other -> throwIO $ ErrorCall
        //   "runQueryPage: server returned the wrong encoding" (:116-117).
        _ => Err(DecodeError::ErrorCall(
            "runQueryPage: server returned the wrong encoding".to_string(),
        )),
    }
}

// ---------------------------------------------------------------------------
// displayQuery (Internal.hs:119-121)
// ---------------------------------------------------------------------------

/// `displayQuery :: Query a -> Text` (Internal.hs:120) — "A human-readable
/// form of the Query." `displayQuery (Query UserQuery{..}) =
/// Text.decodeUtf8 userQuery_query` (:121).
///
/// The port borrows the Query (the source passes an immutable value; the
/// caller keeps using it — see RUST-ONLY ADDITIONS) and decodes the spec's
/// `query` bytes with the local `decode_utf8_strict`, the byte-pinned
/// image of `Text.decodeUtf8` (angle.rs:1085-1111). Unbounded in `A`: the
/// read never needs the constructor's context (types.rs:1415-1422 makes
/// the same call for `into_spec`).
pub fn display_query<A>(q: &Query<A>) -> String {
    decode_utf8_strict(&q.query_spec.query)
}

// ---------------------------------------------------------------------------
// allFacts (Internal.hs:123-125)
// ---------------------------------------------------------------------------

/// `allFacts :: forall q. Predicate q => Query q` (Internal.hs:123) — "A
/// query that fetches all facts of the predicate" (source construction:
/// `angle $ show (pretty (getName (Proxy :: Proxy q))) <> " _"`, :124-125).
///
/// `pretty (getName ...)` renders `name.version`, which is exactly what the
/// shipped `predicate_ref::<R>()` produces (predicate.rs:557-566); the port
/// calls that helper instead of re-deriving Pretty+show over the generated
/// crate's Display (one prettyprinting owner, RULEBOOK.md:37). The
/// trailing `" _"` is the Angle wildcard, byte-for-byte.
pub fn all_facts<R: Predicate>() -> Query<R> {
    angle(&format!("{} _", predicate_ref::<R>()))
}

// ---------------------------------------------------------------------------
// angle (Internal.hs:127-135)
// ---------------------------------------------------------------------------

/// `angle :: forall r. Predicate r => Text -> Query r` (Internal.hs:132) —
/// "Build a query for facts using Angle syntax".
///
/// Warning (source :129-131, preserved): nothing checks that the type of
/// the query matches the expected type at the callsite. If you get this
/// wrong, undefined behaviour will ensue.
///
/// `angle q = Query { querySpec = angleQuery (getName (Proxy @r)) q }`
/// (:133-135): the predicate name+version and the ANGLE syntax are stamped
/// into the spec by `angle_query`; only `R: Predicate` is needed (the
/// supertrait `Type<Self> + 'static` that `Query::new` demands elaborates
/// from it — probe_bounds.rs).
pub fn angle<R: Predicate>(q: &str) -> Query<R> {
    Query::new(angle_query(&R::get_name(), q))
}

// ---------------------------------------------------------------------------
// keys (Internal.hs:137-139)
// ---------------------------------------------------------------------------

/// `keys :: Predicate r => Query r -> Query (KeyType r)` (Internal.hs:138)
/// — "Fetch just the keys of the given fact query".
///
/// `keys (Query spec) = Query { querySpec = spec }` (:139): the spec is
/// untouched; only the type parameter changes, from the fact type to its
/// generated key type. The port rebuilds through `Query::new` (the private
/// `_marker` rules out record construction) and states the source's
/// `Type (KeyType r)` superclass explicitly: `R::KeyType: Type<R::KeyType>
/// + 'static` — the shipped constructor's bound (types.rs:1403), without
/// which rustc raises E0277 (negative probe neg_keys.rs).
pub fn keys<R>(q: Query<R>) -> Query<R::KeyType>
where
    R: Predicate,
    R::KeyType: Type<R::KeyType> + 'static,
{
    Query::new(q.into_spec())
}

// ---------------------------------------------------------------------------
// angleQuery (Internal.hs:141-151) — source-private helper
// ---------------------------------------------------------------------------

/// `angleQuery :: PredicateRef -> Text -> UserQuery` (Internal.hs:141) —
/// PRIVATE in the source (not in the export list). Builds the spec by
/// overriding `def`: predicate name (:143), version (:144-145), UTF-8 query
/// bytes (:146), and options with `expand_results = False` + `syntax =
/// ANGLE` (:147-150).
///
/// `fromIntegral (predicateRef_version pref)` (:145) is an identity
/// conversion: both sides are `Version` (i32) — predicateRef_version ::
/// Version (gen-hs2/Types.hs:340-342, IDL :39) and
/// userQuery_predicate_version :: Maybe Version (gen-hs2 :5670). The port
/// writes `Some(pref.version)` with no cast.
fn angle_query(pref: &thrift::PredicateRef, query: &str) -> thrift::UserQuery {
    thrift::UserQuery {
        predicate: pref.name.clone(),
        // `fromIntegral` identity, see above.
        predicate_version: Some(pref.version),
        // Text.encodeUtf8 query (:146).
        query: query.as_bytes().to_vec(),
        // userQuery_options = Just def { expand_results = False, syntax =
        // QuerySyntax_ANGLE } (:147-150). `def` here is the struct's
        // Default == hsthrift def (backend/types.rs:156).
        options: Some(thrift::UserQueryOptions {
            expand_results: false,
            syntax: thrift::QuerySyntax::ANGLE,
            ..Default::default()
        }),
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// angleData (Internal.hs:153-172)
// ---------------------------------------------------------------------------

/// `angleData :: Type r => Text -> Query r` (Internal.hs:163) — "A query
/// for arbitrary data, as opposed to 'angle' which queries for facts" ;
/// example: `angleData "{ 3, false }" :: Query (Nat, Bool)` (:158).
///
/// Warning (source :160-162, preserved): nothing checks that the type of
/// the query matches the expected type at the callsite. If you get this
/// wrong, undefined behaviour will ensue.
///
/// `angleData query = Query { querySpec = spec }` (:164-172) where spec is
/// `def` with the UTF-8 query bytes and the same ANGLE options as
/// `angleQuery` — but NO predicate/predicate_version: `angleData` leaves
/// both at their defaults (:166-172), exactly as the port does.
pub fn angle_data<A: Type<A> + 'static>(query: &str) -> Query<A> {
    let spec = thrift::UserQuery {
        query: query.as_bytes().to_vec(),
        options: Some(thrift::UserQueryOptions {
            expand_results: false,
            syntax: thrift::QuerySyntax::ANGLE,
            ..Default::default()
        }),
        ..Default::default()
    };
    Query::new(spec)
}

// ---------------------------------------------------------------------------
// recursive (Internal.hs:174-179)
// ---------------------------------------------------------------------------

/// `recursive :: Query a -> Query a` (Internal.hs:175) — "Make a query
/// recursive".
///
/// `q' = q { userQuery_options = Just (fromMaybe def (userQuery_options q))
/// { userQueryOptions_recursive = True } }` (:176-179). The port takes the
/// existing options (or hsthrift `def`), sets `recursive`, and writes them
/// back — the source's `Just (fromMaybe def o) { .. }` shape.
pub fn recursive<A: Type<A> + 'static>(q: Query<A>) -> Query<A> {
    let mut spec = q.into_spec();
    let mut opts = spec.options.take().unwrap_or_default();
    opts.recursive = true;
    spec.options = Some(opts);
    Query::new(spec)
}

// ---------------------------------------------------------------------------
// expanding (Internal.hs:181-195)
// ---------------------------------------------------------------------------

/// `expanding :: forall p a. Predicate p => Query a -> Query a` (:187) —
/// "Instead of 'recursive', pick individual predicates to expand in the
/// result"; e.g. `expanding @My.Predicate1 $ expanding @My.Predicate2 $
/// query ...` (:184-185).
///
/// `PredicateRef name version = getName (Proxy @p)` (:191), `ref =
/// Thrift.SourcePredicate name (Just version)` (:192), and the ref is
/// CONSED onto `userQueryOptions_expand_predicates` (:193-195), so the
/// most recently applied `expanding` is first. The port preserves that
/// order with `insert(0, ..)` — see the performance note at the insert.
pub fn expanding<P, A>(q: Query<A>) -> Query<A>
where
    P: Predicate,
    A: Type<A> + 'static,
{
    let mut spec = q.into_spec();
    let mut opts = spec.options.take().unwrap_or_default();
    let pref = P::get_name();
    let source_pred = thrift::SourcePredicate {
        name: pref.name,
        // Thrift.SourcePredicate name (Just version) (:192); Version is
        // i32 on both sides (no fromIntegral in the constructor position).
        version: Some(pref.version),
    };
    // PERF(port): `ref : userQueryOptions_expand_predicates old` is an
    // O(1) cons in the source (:194-195); Vec has no cons, so
    // insert(0, ..) shifts every existing element — O(n) per call. Faithful
    // (the field order the server reads is preserved) and n is a handful of
    // predicates at every callsite; fast version: keep the list reversed
    // and prepend by push (then reverse once at the read/serialization
    // boundary, changing the shared thrift field's order contract) or use
    // an immutable cons list upstream — deferred.
    opts.expand_predicates.insert(0, source_pred);
    spec.options = Some(opts);
    Query::new(spec)
}

// ---------------------------------------------------------------------------
// limit / limitBytes / limitTime (Internal.hs:197-221)
// ---------------------------------------------------------------------------

/// `limit :: Int -> Query a -> Query a` (Internal.hs:199) — "Set a limit on
/// the number of results returned by a query. This controls query result
/// page size when streaming."
///
/// `userQueryOptions_max_results = Just (fromIntegral n)` (:203): the IDL
/// field is `optional i64` (:419), and `Int -> i64` is the identity on the
/// pinned 64-bit target (RUST-ONLY ADDITIONS). The other two limits are the
/// same shape on their own fields.
pub fn limit<A: Type<A> + 'static>(n: i64, q: Query<A>) -> Query<A> {
    let mut spec = q.into_spec();
    let mut opts = spec.options.take().unwrap_or_default();
    opts.max_results = Some(n);
    spec.options = Some(opts);
    Query::new(spec)
}

/// `limitBytes :: Int -> Query a -> Query a` (Internal.hs:207) — "Set a
/// limit on the size of data returned by a query. This controls query
/// result page size when streaming." `max_bytes` is `optional i64` (:428).
pub fn limit_bytes<A: Type<A> + 'static>(n: i64, q: Query<A>) -> Query<A> {
    let mut spec = q.into_spec();
    let mut opts = spec.options.take().unwrap_or_default();
    opts.max_bytes = Some(n);
    spec.options = Some(opts);
    Query::new(spec)
}

/// `limitTime :: Int -> Query a -> Query a` (Internal.hs:215) — "Set a
/// limit on the time that a query can execute for. When streaming, this
/// imposes a time budget on each query page." `n` is milliseconds (source
/// :216); `max_time_ms` is `optional i64` (:437).
pub fn limit_time<A: Type<A> + 'static>(n: i64, q: Query<A>) -> Query<A> {
    let mut spec = q.into_spec();
    let mut opts = spec.options.take().unwrap_or_default();
    opts.max_time_ms = Some(n);
    spec.options = Some(opts);
    Query::new(spec)
}

// ---------------------------------------------------------------------------
// store (Internal.hs:223-230)
// ---------------------------------------------------------------------------

/// `store :: Query a -> Query a` (Internal.hs:226) — "Store derived facts
/// arising from the query. This is intended for caching derived facts after
/// creating a DB, not for normal usage. If the datbase is read-only, this
/// will cause the query to fail." (sic, source typo "datbase").
///
/// Sets `userQueryOptions_store_derived_facts = True` (:229-230; IDL :455).
pub fn store<A: Type<A> + 'static>(q: Query<A>) -> Query<A> {
    let mut spec = q.into_spec();
    let mut opts = spec.options.take().unwrap_or_default();
    opts.store_derived_facts = true;
    spec.options = Some(opts);
    Query::new(spec)
}

// ---------------------------------------------------------------------------
// justCheck (Internal.hs:232-236)
// ---------------------------------------------------------------------------

/// `justCheck :: Query a -> Query a` (:232) — sets
/// `userQueryOptions_just_check = True` (:235-236; IDL :473). No source doc
/// comment; the flag's meaning comes from the IDL: "if true, the query will
/// be compiled, but no facts will be retrieved".
pub fn just_check<A: Type<A> + 'static>(q: Query<A>) -> Query<A> {
    let mut spec = q.into_spec();
    let mut opts = spec.options.take().unwrap_or_default();
    opts.just_check = true;
    spec.options = Some(opts);
    Query::new(spec)
}

// ---------------------------------------------------------------------------
// dbgPredHasFacts (Internal.hs:238-242)
// ---------------------------------------------------------------------------

/// `dbgPredHasFacts :: Query a -> Query a` (:238) — sets
/// `userQueryOptions_debug = def { queryDebugOptions_pred_has_facts = True
/// }` (:241-242).
///
/// NOTE the source REPLACES the whole debug record with a fresh `def`
/// carrying the one flag — it does NOT update an existing debug struct.
/// Other debug flags (ir, bytecode) are therefore reset to False. The port
/// reproduces that replacement exactly (`..Default::default()`), not an
/// in-place field write.
pub fn dbg_pred_has_facts<A: Type<A> + 'static>(q: Query<A>) -> Query<A> {
    let mut spec = q.into_spec();
    let mut opts = spec.options.take().unwrap_or_default();
    opts.debug = thrift::QueryDebugOptions {
        pred_has_facts: true,
        ..Default::default()
    };
    spec.options = Some(opts);
    Query::new(spec)
}

// ---------------------------------------------------------------------------
// reportUserQueryStats (Internal.hs:244-246)
// ---------------------------------------------------------------------------

/// `reportUserQueryStats :: Thrift.UserQueryStats -> IO ()` (:244) —
/// `vlog 1 $ showUserQueryStats stats` (:245-246).
///
/// `vlog`'s message is lazy in the source: the String is only forced when
/// the level is enabled (Util.Log.String.hs:34-41). The A10 image takes a
/// thunk (`impl FnOnce() -> String`), so the port passes the formatter as a
/// closure and the verbosity gate stays the log owner's business
/// (ASSUMED-IMPORT block (b)). Borrows `stats`: the source's immutable
/// value is shared, not copied.
pub fn report_user_query_stats(stats: &thrift::UserQueryStats) {
    vlog(1, || show_user_query_stats(stats));
}

// ---------------------------------------------------------------------------
// showUserQueryStats (Internal.hs:248-253)
// ---------------------------------------------------------------------------

/// `showUserQueryStats :: Thrift.UserQueryStats -> String` (:248) —
/// `printf "%d facts, %.2fms, %ld bytes\n" userQueryStats_num_facts
/// (realToFrac userQueryStats_elapsed_ns / 1000000 :: Double)
/// userQueryStats_allocated_bytes` (:249-253).
///
/// Format mapping, all probe-pinned in /tmp/opencode/db45-probe/:
///   * `%d` on Int64 -> `{}` on i64 (signed decimal);
///   * `%.2f` on Double -> `show_ffloat_2` (NOT `{:.2}`: GHC rounds the
///     shortest round-trip digits, Rust rounds the exact binary expansion —
///     e.g. 2.675 -> "2.68" GHC vs "2.67" Rust);
///   * `%ld` -> `{}` on i64 (signed decimal);
///   * `\n` -> the literal newline.
/// `realToFrac ns / 1000000 :: Double` is `ns as f64 / 1_000_000.0`
/// (Int64 -> f64 rounds to nearest-even, exactly realToFrac's behavior;
/// the division is then one correctly-rounded f64 divide). Ground truth
/// for the whole line: /tmp/opencode/db45-probe/showstats_hs.hs, e.g.
/// (3, 2.675, 100) -> "3 facts, 2.68ms, 100 bytes\n".
pub fn show_user_query_stats(stats: &thrift::UserQueryStats) -> String {
    format!(
        "{} facts, {}ms, {} bytes\n",
        stats.num_facts,
        // elapsed_ns (i64) -> milliseconds (f64), see doc above.
        show_ffloat_2(stats.elapsed_ns as f64 / 1_000_000.0),
        stats.allocated_bytes
    )
}

// ---------------------------------------------------------------------------
// GHC `printf "%.2f"` parity — Rust-only helpers (see the additions block)
// ---------------------------------------------------------------------------

/// `printf "%.2f"` (Text.Printf -> Numeric.showFFloat (Just 2) ->
/// GHC.Float.formatRealFloatAlt FFFixed (Just 2) False). Source text:
/// base-4.17.1.0's GHC.Float (identical to the pinned GHC 9.4.7's
/// base-4.17.2.0 for GHC.Float/Numeric/Text.Printf).
/// Rust's `{:.2}` rounds the exact binary expansion; GHC rounds the
/// SHORTEST round-trip digit string (ties to even) — e.g. `2.675` is
/// "2.68" in GHC and "2.67" in Rust (probe: /tmp/opencode/db45-probe/).
///
/// Exact over this call site's whole domain: |x| <= 9223372036854775807 /
/// 1_000_000 < 2^54 (elapsed_ns is i64; the ns battery has 0 mismatches
/// over 2,700,010 inputs). Above ~2^54 GHC's floatToDigits can emit
/// non-minimal digits (48/200,033 bit-pattern mismatches, all with
/// |x| >= 2^54, both sides round-tripping) — unreachable here, documented
/// like angle.rs's residual risks rather than flagged.
fn show_ffloat_2(x: f64) -> String {
    // formatRealFloatAlt, prelude clauses (GHC.Float:834-837).
    if x.is_nan() {
        return "NaN".to_string();
    }
    if x.is_infinite() {
        return if x < 0.0 { "-Infinity" } else { "Infinity" }.to_string();
    }
    let neg = x < 0.0 || (x == 0.0 && x.is_sign_negative());
    let x = if neg { -x } else { x };

    // floatToDigits 10 x (:837): digits d1..dn with x = 0.d1..dn * 10^e.
    // The shortest correctly-rounded round-trip digits; Rust's `{:e}`
    // (LowerExp) prints the same digits ("d.ddd...e<exp>", x = d.ddd *
    // 10^exp).
    let (digits, e): (Vec<u32>, i32) = if x == 0.0 {
        (vec![0], 0)
    } else {
        let s = format!("{:e}", x);
        let (mant, exp) = s.split_once('e').expect("`{:e}` always emits `e`");
        let digits = mant
            .bytes()
            .filter(|b| *b != b'.')
            .map(|b| u32::from(b - b'0'))
            .collect();
        (digits, exp.parse::<i32>().expect("i32 exponent") + 1)
    };

    // doFmt, FFFixed + Just dec branch (:878-905); dec = 2, alt = False,
    // dec' = max dec 0 = 2.
    let dec: usize = 2;
    let is2;
    let ds;
    if e >= 0 {
        // (:894-899): (ei,is') = roundTo 10 (dec'+e) is;
        //            (ls,rs)  = splitAt (e+ei) (map intToDigit is');
        //            mk0 ls ++ '.' : rs
        let (ei, is_) = round_to(10, dec + e as usize, &digits);
        ds = is_.iter().map(|&d| int_to_digit(d)).collect::<String>();
        let (ls, rs) = ds.split_at(e as usize + ei as usize);
        let ls = if ls.is_empty() { "0" } else { ls };
        if rs.is_empty() {
            is2 = format!("{}", ls);
        } else {
            is2 = format!("{}.{}", ls, rs);
        }
    } else {
        // (:900-905): (ei,is') = roundTo 10 dec' (replicate (-e) 0 ++ is);
        //             d:ds' = map intToDigit (if ei > 0 then is' else 0:is');
        //             d : '.' : ds'
        let mut padded = vec![0u32; (-e) as usize];
        padded.extend_from_slice(&digits);
        let (ei, is_) = round_to(10, dec, &padded);
        let mut ds_ = if ei > 0 {
            is_.iter().map(|&d| int_to_digit(d)).collect::<String>()
        } else {
            format!(
                "0{}",
                is_.iter().map(|&d| int_to_digit(d)).collect::<String>()
            )
        };
        let d = ds_.remove(0);
        if ds_.is_empty() {
            is2 = format!("{}", d);
        } else {
            is2 = format!("{}.{}", d, ds_);
        }
    }
    if neg {
        format!("-{}", is2)
    } else {
        is2
    }
}

/// `intToDigit` (Data.Char) for 0..=9.
fn int_to_digit(d: u32) -> char {
    std::char::from_digit(d, 10).expect("digit in 0..=9")
}

/// `roundTo base d is` (GHC.Float:908-925). Returns `(carry, digits)` with
/// the carry's leading 1 already prepended to `digits` when carry == 1
/// (the `(1,xs) -> (1, 1:xs)` wrapper at :912).
fn round_to(base: u32, d: usize, is: &[u32]) -> (u32, Vec<u32>) {
    fn f(n: usize, e: bool, is: &[u32], base: u32, b2: u32) -> (u32, Vec<u32>) {
        // f n _ [] = (0, replicate n 0)
        if is.is_empty() {
            return (0, vec![0; n]);
        }
        // f 0 e (x:xs) | x == b2 && e && all (== 0) xs = (0, [])
        //              | otherwise = (if x >= b2 then 1 else 0, [])
        if n == 0 {
            let x = is[0];
            if x == b2 && e && is[1..].iter().all(|&d| d == 0) {
                return (0, Vec::new());
            }
            return (if x >= b2 { 1 } else { 0 }, Vec::new());
        }
        // f n _ (i:xs) = ... (c,ds) = f (n-1) (even i) xs; i' = c + i;
        //                i' == base -> (1, 0:ds); otherwise (0, i':ds)
        let i = is[0];
        let (c, ds) = f(n - 1, i % 2 == 0, &is[1..], base, b2);
        let i2 = c + i;
        let mut out = Vec::with_capacity(ds.len() + 1);
        out.push(if i2 == base { 0 } else { i2 });
        out.extend(ds);
        (if i2 == base { 1 } else { 0 }, out)
    }
    let (c, xs) = f(d, true, is, base, base / 2);
    if c == 0 {
        (0, xs)
    } else {
        let mut out = Vec::with_capacity(xs.len() + 1);
        out.push(1);
        out.extend(xs);
        (1, out)
    }
}

// ---------------------------------------------------------------------------
// Data.Text.Encoding.decodeUtf8 parity — Rust-only helper (see the
// additions block; exact copy of angle.rs:1085-1111, which stays private
// there)
// ---------------------------------------------------------------------------

/// `Text.decodeUtf8` (Data.Text.Encoding) for a byte string: valid UTF-8
/// becomes a String; invalid UTF-8 raises the same panic text as the
/// shipped sibling (angle.rs:1085-1111, probed there against text-2.0.2,
/// the GHC 9.4.7 boot lib: broken sequences report the first bad byte,
/// truncated-at-end inputs the LAST byte).
fn decode_utf8_strict(b: &[u8]) -> String {
    match std::str::from_utf8(b) {
        Ok(s) => s.to_string(),
        Err(e) => {
            let offending = match e.error_len() {
                // Broken sequence: text reports its first byte.
                Some(_) => b[e.valid_up_to()],
                // Unexpected end of input: text reports the last byte.
                None => b[b.len() - 1],
            };
            panic!(
                "Cannot decode byte '\\x{:02x}': Data.Text.Encoding: Invalid UTF-8 stream",
                offending
            );
        }
    }
}

// Coverage: every export of the source module (Internal.hs:16-37 — 18
// names) has a Rust counterpart, all `pub`, nothing extra `pub`:
//   * Query(..) (:17) -> `pub use crate::backend::types::Query;` (the
//     struct, with `Query::new`/`into_spec` as its constructor/selector
//     images; the export mapping table above lists all 18).
//   * the eleven combinators (:19-29) -> query, angle, angle_data, keys,
//     recursive, limit, limit_bytes, limit_time, store, all_facts,
//     expanding.
//   * the six support names (:31-36) -> report_user_query_stats,
//     show_user_query_stats, display_query, decode_results, just_check,
//     dbg_pred_has_facts.
// The two source-private definitions are private here too: angleQuery
// (:141) -> angle_query; `type ResultDecoder` (:87) -> the `D` bound on
// decode_results. The four Rust-only helpers (show_ffloat_2, round_to,
// int_to_digit, decode_utf8_strict) are private and listed in RUST-ONLY
// ADDITIONS.
//
// Marker breakdown (closed set — two performance deferrals and one
// inventory divergence; zero port todos, zero source-defect
// reproductions):
//   * two performance deferrals, both flagged at their sites: expanding's
//     front insert (O(n) vs the source cons) and decode_results's
//     materialized+cloned nested-fact map (the shipped decoder's borrowed
//     HashMap). Each names its fast version.
//   * no port todos: every surface above is implemented; the assumed
//     imports are pinned surfaces consumed at their declared shape (the
//     generated crate per RULE §4 B4; the log seam per A10, whose owner and
//     pinned signature are named), so nothing is stubbed or deferred.
//   * no source-defect reproductions: the module's only partial path is the
//     union's wrong-encoding arm, and it is total here through the typed
//     error channel (the source's own specified behavior, not a defect);
//     the module contains no error/head/fromJust/unsafePerformIO sites.
//   * one inventory divergence flag: row 56 (inventory.tsv:56 /
//     inventory.stm.tsv:35) at the decode_results cache site — per-call
//     locals kept, BatchCtx ruling scoped to the haxl half (row 146).
//     Rows 43 and 135 are consumed; their flags stay with their owners
//     (build.rs:32-38, predicate.rs:255-267).
//
// PORT STATUS: confidence=medium todos=0
