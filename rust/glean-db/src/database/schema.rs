// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// RUST-ONLY SCAFFOLDING (no Haskell body yet). This file is the directory
// module for the Schema/ namespace: it declares 'pub mod compute_ids;' so
// that rust/glean-db/src/database/schema/compute_ids.rs (row 66's port of
// Schema/ComputeIds.hs) resolves at crate::database::schema::compute_ids,
// and 'pub mod types;' so that .../schema/types.rs (row 89's port of
// Database/Schema/Types.hs) resolves at crate::database::schema::types.
//
// TWO ROLES AT THIS PATH (the catalog.rs, backup.rs and logger.rs
// precedent). migration/manifest.tsv line 98 maps
// glean/db/Glean/Database/Schema.hs to THIS SAME path,
// rust/glean-db/src/database/schema.rs (crate glean-db), so a later unit
// would add the port body of Schema.hs here, beside the module
// declarations. Row 66 created only the scaffolding and ported no body.
//
// ROW 98'S BODY HAS NOW LANDED (this unit): the port of
// glean/db/Glean/Database/Schema.hs (1258 lines, module
// Glean.Database.Schema, 24 exported names) is appended below the module
// declarations, after this header. NOTE THE HIDDEN-ROW PROVENANCE:
// manifest line 98 is not a queue row — the queue command skips it;
// the unit was taken explicitly per HANDOFF §2.4's warning over
// hidden rows (the row-66 anchor at this path predates it as the early
// anchor). Verified against the manifest directly:
//   $ sed -n '98p' migration/manifest.tsv
//     glean/db/Glean/Database/Schema.hs	rust/glean-db/src/database/schema.rs
// and the checkout's own INVENTORY (grep -n 'Database/Schema\.hs'
// migration/inventory*.tsv) returns NOTHING: no inventory row governs
// this file. The ComputeIds.hs row (inventory.tsv:27) governs
// compute_ids.rs; Schema.hs has no rows of its own. Nothing here is an
// inventory-owned class (no FFI, no STM, no unsafePerformIO, no lazy IO),
// so there is no row for any site to diverge from and no row-flag
// surface at all. The one flagged divergence cluster (the SHARING WALLS below) is
// recorded here because the brief must not discover it at the call sites.
//
// WIRING: database.rs declares 'pub mod schema;' after
// 'pub mod batch_location;'; this file in turn declares
// 'pub mod compute_ids;' and 'pub mod types;'.
//
// ROW 66'S WIRING CONTRACT — CONSUMED
// Row 66 (Schema/ComputeIds.hs, manifest line 66) appended
// 'pub mod compute_ids;' below, resolving .../schema/compute_ids.rs.
//
// ROW 89'S WIRING CONTRACT — CONSUMED
// Row 89 (Schema/Types.hs, manifest line 89) appended 'pub mod types;'
// below, resolving .../schema/types.rs (its unit also updated this
// header's scaffolding and coverage paragraphs).
//
// ---------------------------------------------------------------------------
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, checked not assumed)
// ---------------------------------------------------------------------------
//
// OUTBOUND — Schema.hs's 20 edges, edges.tsv:364-383:
//   :364 Angle/Hash.hs            -> glean_angle::angle::hash::hash_binary
//   :365 Angle/Types.hs           -> glean_angle::angle::types::{
//      DeriveWhen, DerivingInfo, PredicateDef_, PredicateId, SourceQuery_,
//      SrcSpan, TypeDef_, TypeId, Type_, rm_loc_type, rm_type_loc_pred_def,
//      latest_angle_version, AngleVersion, SourceSchemas, Version
//      (the last three re-exports through angle/types.rs:104/:1107), and
//      ResolvedSchema/ResolvedSchemas/ResolvedSchemaRef — which live in
//      Schema/Types.hs's image, glean_angle::schema::types (edge :368)}
//   :366 Display.hs               -> glean_angle::display::{
//      display_default, Display, Doc, DisplayOpts}
//   :367 Schema/Evolve.hs         -> glean_angle::schema::evolve::{
//      calc_evolutions, can_be_stored, direct_schema_evolutions,
//      map_visible, validate_evolutions, visible_predicates,
//      VisiblePredicates} (visibleDefined is VisiblePredicates's pub
//      field visible_defined — evolve.rs:177)
//   :368 Angle/Schema/Types.hs    -> glean_angle::schema::types::schema_ref
//   :369 Schema/Util.hs           -> glean_angle::schema::util::ShowRef
//   :370 Database/Config.hs       -> crate::database::config::{
//      process_schema, DebugFlags, SchemaIndex, schema_for_schema_id,
//      ProcessedSchema (proc_schema_source/resolved/hashed,
//      config.rs:751-754)}
//   :371 Schema/ComputeIds.hs     -> crate::database::schema::compute_ids::
//      HashedSchema (hashed_types/preds, schema_ref_to_id_env,
//      hashed_schema_env, hashed_schema_all_version, hashed_schema_id —
//      compute_ids.rs:264-270)
//   :372 Schema/Types.hs          -> crate::database::schema::types::{
//      DbSchema, DbSchemaCache, DbSchemaCacheKey, PredicateDetails,
//      PredicateTransformation, SchemaSelector, TypeDetails,
//      predicate_ref, pid_ref, mk_rts_type, mk_query_transformations,
//      QueryTransformations}
//   :373 Codegen/Types.hs         -> crate::query::codegen::types::
//      QueryWithInfo (qi_query et al — codegen/types.rs via
//      typecheck/types.rs:587's TypecheckedQuery alias)
//   :374 Query/Prune.hs           -> crate::query::prune::prune_derivations
//   :375 Query/Transform.hs       -> crate::query::transform::{
//      transform_fact (:1292), transform_pattern (:1825)}
//   :376 Query/Typecheck.hs       -> crate::query::typecheck::{
//      typecheck_deriving (:494), TcEnv, TcOpts, default_tc_opts,
//      empty_tc_env, tc_query_deps (:2396), tc_query_uses_negation
//      (:2515), UseOfNegation (:2504), ToRtsType (monad.rs:321)}
//   :377 Typecheck/Types.hs       -> crate::query::typecheck::types::TcQuery
//   :378 ServiceData/GlobalStats.hs -> glean_fb303::service_data::
//      global_stats::add_stat_value_type (global_stats.rs:147; the source
//      call sites are Schema.hs:328-338 — pinned there)
//   :379 ServiceData/Types.hs     -> glean_fb303::service_data::types::
//      ExportType (Sum/Avg; the ODS.Sum image — types.rs:39-50 names this
//      file's `import ServiceData.Types as ODS` + `ODS.Sum` duality)
//   :380 RTS/Foreign/Inventory.hs -> glean_core::rts::foreign::inventory::{
//      new, CompiledPredicate, CompiledTypecheck, CompiledTraversal,
//      Inventory}
//   :381 RTS/Traverse.hs          -> NOT SHIPPED (manifest row 23) — see
//      BLOCKED DEPENDENCIES below
//   :382 RTS/Typecheck.hs         -> NOT SHIPPED (manifest row 24) — see
//      BLOCKED DEPENDENCIES below
//   :383 RTS/Types.hs             -> glean_core::rts::types::{
//      lowest_pid (types.rs:142), ExpandedType, FieldDef, Pid, PidRef, Type}
// Glean.Types (Thrift) has no depmap row (generated, RULE §4) — an
// ASSUMED IMPORT (block below). Glean.Internal.Types and
// Glean.ServerConfig.Types are generated too (inventory.tsv:142 row for
// the former) — ASSUMED IMPORTS. Util.Log.Text is an external hsthrift
// package (no edges row) — ASSUMED IMPORT via the pinned log seam.
// Data.Graph/Data.HashMap/Data.IntMap/Data.Map/... are containers —
// library images in the MAP IMAGES block.
//
// INBOUND — every one is a future unit (edges.tsv:41,61,151,200,246,324,
// 471,479,593,603,1556,1629,1675,1690,1702,1759,1795): Backend/Local,
// Dump, Backup, CompletePredicates, Create, Open, Validate, Write/Batch,
// Flatten/Types, JSON, schema/gen Main, Shell, diff, disassemble,
// GleanCLI (+Merge/Write). They consume the EXPORT MAPPING below.
//
// ---------------------------------------------------------------------------
// BLOCKED DEPENDENCIES (rows 23/24) — LIVE USES, NOT VESTIGIAL
// ---------------------------------------------------------------------------
//
// The source imports Glean.RTS.Traverse (:81) and Glean.RTS.Typecheck
// (:82) — manifest rows 23/24, both NOT SHIPPED (rust/glean-core/src/rts/
// traverse.rs and rts/typecheck.rs do not exist). GREP-VERIFIED USES —
// neither import is vestigial:
//   * Glean.RTS.Typecheck exports checkSignature (Typecheck.hs:13/:206,
//     `checkSignature :: Type -> Type -> IO (Subroutine CompiledTypecheck)`)
//     — used at Schema.hs:846 (`typecheck <- checkSignature keyType
//     valueType`), the per-predicate bytecode compilation inside
//     mkPredicateDetails. This is THE expensive step the DbSchema cache
//     exists to amortize (the source's caching haddock, Schema.hs:285-311).
//   * Glean.RTS.Traverse exports genTraversal (Traverse.hs:12/:94,
//     `genTraversal :: Type -> Type -> IO (Subroutine CompiledTraversal)`)
//     — used at Schema.hs:847. Same shape.
//   * The two KIND TAGS (Glean.RTS.Typecheck's `data CompiledTypecheck`,
//     Typecheck.hs:26; Glean.RTS.Traverse's `data CompiledTraversal`,
//     Traverse.hs:25) arrive through the shipped stand-ins
//     inventory.rs:258-259, exactly as row 89's header recorded (its
//     ":401/:402 NOT SHIPPED" block).
// So the port imports the two functions from their manifest-pinned future
// homes. The signatures below are pinned from the source's own exports
// (by-& Type args: the RTS Type is an owned value the Haskell shares;
// the Subroutine result is the ForeignPtr handle both rows will produce;
// IO -> infallible per the allocation-guard convention —
// glean_inventory_new's abort shape, inventory.rs:383-389):
//     pub fn check_signature(key: &Type, val: &Type)
//         -> Subroutine<CompiledTypecheck>;   // rts/typecheck.rs (row 24)
//     pub fn gen_traversal(key: &Type, val: &Type)
//         -> Subroutine<CompiledTraversal>;   // rts/traverse.rs (row 23)
// Until those units land these use lines are the Step-4 compile error,
// never a silent divergence (the meta.rs:83-105 shape).
//
// ---------------------------------------------------------------------------
// ASSUMED IMPORTS (RULE §4 B4)
// ---------------------------------------------------------------------------
//
//   * crate::internal::types::StoredSchema — the generated
//     Glean.Internal.Types image (inventory.tsv:142 is the confirmed row:
//     generated from glean/if/internal.thrift, no manifest unit, home
//     pinned by DECISIONS.md 2026-10-05; cross-crate caveat recorded at
//     meta.rs:81-105 — only the import path would move). THIS UNIT IS THE
//     FIRST FIELD-LEVEL CONSUMER: storage.rs:148-154 and data.rs:7/:116
//     consume the type OPAQUELY (whole-struct serialize/deserialize), so
//     their sketches stopped at the struct name. The fields this unit's
//     code touches (internal.thrift:20-31):
//         pub struct StoredSchema {
//             pub schema: Vec<u8>,   // 1: string_321 — storage.rs:148-154's
//                                    // pinned `Vec<u8>` disposition (the
//                                    // field is UTF-8 schema source bytes)
//             pub predicate_ids: BTreeMap<Id, PredicateRef>,
//                                    // 2: map<glean.Id, glean.PredicateRef>
//                                    // — thrift map -> BTreeMap is the
//                                    // PINNED codegen image
//                                    // (backend/types.rs:152-156, the
//                                    // typed/predicate.rs:482-486 ruling);
//                                    // NOTE storage.rs's opacity-only sketch
//                                    // wrote HashMap — the field-level
//                                    // consumer follows the pinned BTreeMap,
//                                    // which is also the semantic one (the
//                                    // source reads this field's Data.Map in
//                                    // ascending key order at :146)
//             pub versions: BTreeMap<String, Version>,
//                                    // 3: map<string, glean.Version> —
//                                    // "MUST HAVE A SINGLE ENTRY"
//                                    // (internal.thrift:26-30)
//         }
//     If the generated crate ships different names, only this file's
//     field reads/writes change — a compile error, never silent behavior.
//   * glean_thrift (generated crate, A10; first sketch
//     backend/types.rs:87-226 — not re-sketched here, extended only):
//       - `use glean_thrift::Config as ServerConfig` — the
//         Glean.ServerConfig.Types image (generated from
//         glean/config/server/server_config.thrift; the SAME alias the
//         shipped config.rs:341 uses, which is the row-85 unit's own
//         disposition of the module: one generated crate for all .thrift
//         files, per-file-namespace caveat recorded at meta.rs:95-105).
//       - PredicateStats { pub count: i64, pub size: i64 }
//         (glean.thrift:344-347) — EXTENDING the first sketch, which left
//         PredicateStats opaque (backend/types.rs:108). This unit reads
//         .count (predicateStats_count, Schema.hs:728/:597/:934).
//       - SchemaInfo { pub schema: Vec<u8>, pub predicate_ids:
//         BTreeMap<Id, PredicateRef>, pub schema_ids:
//         BTreeMap<String, Version>, pub db_schema_ids:
//         BTreeMap<String, Version>, pub other_schema_ids:
//         Vec<BTreeMap<String, Version>>, pub derivation_dependencies:
//         BTreeMap<Id, Vec<Id>>, pub auth_status: Option<..>,
//         pub auth_message: Option<String> } (glean.thrift:983-1002) —
//         extending backend/types.rs:212's partial sketch.
//       - GetSchemaInfo { pub select: SelectSchema, pub omit_source: bool }
//         and SelectSchema { Empty, Stored, Current, SchemaId(SchemaId) }
//         (glean.thrift:987-1024; already sketched at
//         backend/types.rs:180-190 — consumed as pinned, not re-sketched).
//       - PredicateRef — home is the angle/types.rs:104 re-export
//         (`pub use glean_thrift::{PredicateRef, ...}`); imported from the
//         generated crate directly per the in-crate house form
//         (compute_ids.rs:247 `use glean_thrift::{PredicateRef, ..}`).
//         Pinned shape: compute_ids.rs:81-89 (struct { name, version },
//         Clone + Debug + Eq + Ord + Hash).
//       - SchemaId = String (compute_ids.rs:81-89 pins the hs.newtype
//         image), Id = i64 (backend/types.rs:159 pins `pub type Id =
//         i64`).
//   * glean_fb303 (rows 96/97, shipped) — a NEW CRATE EDGE for this
//     crate's Cargo.toml: depmap edges.tsv:378-379 (Schema.hs ->
//     ServiceData/{GlobalStats,Types}.hs) govern it, so it is a
//     depmap-recorded edge, not an invention. The Cargo.toml comment
//     entry (the crate's recorded-edge convention) was outside the unit's
//     single-file brief (schema.rs only) and is appended by the fix round
//     — comment-only, no [dependencies] entry.
//   * glean_util::log::vlog — the pinned log seam
//     (RULEBOOK §1 logging row; DECISIONS.md 2026-10-18): the module is
//     consumed as a forward reference until glean-util's log unit lands
//     (first sketch internal.rs:190-213, unit 45; the pinned shape as
//     consumed and re-documented at query/thrift.rs:271-292: `pub fn
//     vlog(level: i64, msg: impl FnOnce() -> String)`; the message is
//     forced iff vlogIsOn level — the LAZY-thunk discipline). This file's
//     three vlog sites (Schema.hs:329, :491-501, :503) pass closures
//     (:336 is the cache's modifyMVar_ insert, not a log).
//   * Compat.Prettyprinter.Render.Text (layoutCompact/renderStrict,
//     Schema.hs:63/:1220) — RENDER/LAYOUT PARITY: display.rs:537-542
//     (TODO(port) #2) names Schema.hs:1220 as one of the parity-critical
//     render sites and display.rs "deliberately renders nothing" — the
//     layout+render pair is UNOWNED. See render_schema_source below.
//
// ---------------------------------------------------------------------------
// SIBLING-UNIT CONTRACTS
// ---------------------------------------------------------------------------
//
//   * PRUNE CONSUMER OBLIGATION — row 93's contract, DISCHARGED at
//     `prune` below. prune.rs:110-124 ("OWED TO THE CONSUMER") pins the
//     shape: the Haskell call `pruneDerivations hasFacts pmap`
//     (Schema.hs:719) has hasFacts CLOSING OVER pmap (Schema.hs:723-728),
//     but prune_derivations (prune.rs:250-253) takes
//     `impl Fn(&PredicateId) -> bool` and CONSUMES the details map
//     (PredicateDetails is not Clone — schema/types.rs:753-755 — and the
//     callee moves DerivingInfo out of each entry, so a closure borrowing
//     the same map cannot be passed alongside it: E0505). The discharge,
//     sanctioned verbatim by that block ("the consumer therefore
//     precomputes the predicate's boolean answer — e.g. a
//     HashMap<PredicateId, bool> built from (pmap, transformations,
//     stats) — and passes a closure over THAT. Same question, same
//     answers, same call count per predicate; only the capture target
//     moves"): `prune` builds `HashMap<PredicateId, bool>` from
//     (pmap, transformations, stats) — for every pmap key, the source's
//     hasFacts answer (Schema.hs:723-728: details lookup, pid, the
//     transformation's tAvailable pid override, stats count > 0; keys
//     outside pmap answer False, exactly `fromMaybe False`) — and passes
//     a closure over that map into prune_derivations. &PredicateId (not
//     Copy) per the same block's `filter_f` precedent.
//   * rows 85-97 all SHIPPED and consumed (surfaces verified by grep
//     against the real files, cites in DEPENDENCY TRUTH):
//     config.rs (process_schema/SchemaIndex/DebugFlags/schema_for_schema_id),
//     query_regs+codegen-types+typecheck-types+schema-types (rows 86-89),
//     prune.rs (row 93), transform.rs (row 95: transform_fact/
//     transform_pattern), service_data rows 96-97 (add_stat_value_type/
//     ExportType). Rows 99-100 (metric/throttle) are NOT used by this
//     source — no imports to discharge.
//   * evolve.rs (row 7) PRE-ANNOUNCED this unit:
//     "the db-crate port folds these with Map.fromListWith (<>)
//     (Database/Schema.hs predicatesBySchemaRef)" — honored by
//     predicates_by_schema_ref below via VisiblePredicates::append
//     (evolve.rs:259-270).
//   * THE TCENV KNOT — typecheck.rs:488-503's TODO(port) NAMES THIS UNIT:
//     "this signature takes TcEnv, PredicateDetails and ToRtsType BY
//     VALUE, but the source's only caller reuses all three across a mapM
//     (Schema.hs:872 @env = TcEnv@, :881 the call, :884 the returned
//     @details{...}@, :887 @mapM tcDeriving@). TcEnv/PredicateDetails are
//     not Clone and ToRtsType is a Box<dyn Fn>, so that loop is
//     inexpressible as written. The resolution must be CENTRAL, at row
//     89/90 (Arc-shared TcEnv maps / Rc<DbSchema> / a &-taking
//     initialTypecheckState), not a local patch in this file. Owners: the
//     Database/Schema.hs unit (manifest row 98) and the UserQuery unit
//     (row 144)." This unit cannot edit rows 89/90 (single-file brief),
//     so typecheck_schema below writes the loop in the source's SHAPE
//     against the central fix's call form and carries the TODO(port)
//     marker. See SHARING WALLS.
//   * schema/types.rs (row 89): mk_rts_type's &dyn Fn lookups,
//     mk_query_transformations(&HashMap<i64, PredicateDetails>,
//     HashMap<i64, PredicateTransformation>) — the tmap argument is
//     CONSUMED (its transformations move into the QueryTransformations),
//     and pmap is borrowed. transform.rs (row 95):
//     transform_fact(&PredicateDetails, &PredicateDetails) ->
//     Box<dyn Fn(&Fact) -> Fact>; transform_pattern(Type, Type) ->
//     Option<Box<dyn Fn(..)>> whose closure shape is the erased
//     TransformPatternAction PredicateTransformation stores (row 89's
//     RANK-2 IMAGE; PatTerm there = the Matcher alias here's target type,
//     Term<Match<TransformAndBind, Output>>). config.rs (row 85):
//     SchemaIndex { pub schema_index_current: ProcessedSchema,
//     pub schema_index_older: Vec<ProcessedSchema> } (config.rs:731-733),
//     schema_for_schema_id borrows (config.rs:739).
//
// ---------------------------------------------------------------------------
// SHARING WALLS (the ownership ledger — one root cause, three sites)
// ---------------------------------------------------------------------------
//
// The source's DbSchema is an immutable value shared by reference across
// the cache, the twin pid/id maps, and the typecheck loop. Rust ownership
// forbids the aliasing of non-Clone values, and PredicateDetails cannot
// be Clone today (Subroutine<S> wraps crate::ffi::ForeignPtr, which IS
// clone-shared — bytecode.rs:268-270: "wrap/unwrap round-trips over the
// same clone-shared ForeignPtr (unwrap clones the Arc)" — with the
// finalizer running at most once (the Drop at ffi.rs:428-441) — but
// Subroutine carries no Clone impl; TypecheckedQuery's QueryWithInfo
// carrier derives nothing by row 88's pin, typecheck/types.rs:95-102;
// schema_inventory: Inventory is a unique raw-pointer FFI handle with
// Drop — the struct at inventory.rs:296-301, the impl Drop at :341).
// The three sites, each with its marker:
//   1. predicates_by_pid (build_db_schema): the source builds
//      `byPid = IntMap.fromList [(intPid p, p) | p <- HashMap.elems byId]`
//      (Schema.hs:469) — the SAME details stored under both keys
//      (Schema.hs:507-510). The port clones each PredicateDetails into
//      the pid map. TODO(port) at the site: needs impl Clone for
//      Subroutine<S> (an additive derive over the already-cloneable
//      ForeignPtr, row 18) and for QueryWithInfo/TypecheckedQuery
//      (row 87/88 — DerivingInfo is Clone, angle/types.rs:1042).
//      Cloning is semantics-preserving: both maps are immutable after
//      construction and the ForeignPtr clone shares the C++ handle
//      exactly as the Haskell thunk sharing did.
//   2. the schema cache (with_db_schema_cache): the source's
//      modifyMVar_ INSERTS the built schema and the same value is
//      returned (Schema.hs:335-336) — one Inventory handle owned by both
//      cache and caller. Not expressible with an owned FFI handle; the
//      port's cache is write-starved (TODO(port) + rationale at the
//      site): the hit/remove path and the exact key computation are
//      kept so the future fix (an Arc<DbSchema> cache payload — a
//      row-89 alias change — or an Inventory sharing seam, row 25)
//      lands without re-shaping this file.
//   3. the typecheck loop (typecheck_schema): see THE TCENV KNOT above.
//      The draft's call form needs the SAME PredicateDetails: Clone seam
//      as site 1 (the loop's env carries clones; each call mutates its
//      own owned details). No fourth wall: everything else in the file
//      is expressible with shipped types.
//
// ---------------------------------------------------------------------------
// MAP IMAGES (RULE §2; the house conventions with their precedents)
// ---------------------------------------------------------------------------
//
//   HashMap PredicateId PredicateDetails       -> std HashMap (row 89's
//     MAP IMAGES; key std-Hash: angle/types.rs:945)
//   HashMap PredicateRef Pid                   -> std HashMap
//   IntMap (keyed by fromIntegral pid)         -> HashMap<i64, _> — the
//     pinned IntMap image (code.rs:272, query/thrift/internal.rs:241);
//     the ORDER-READING sites re-order explicitly: identicalSchemas'
//     `IntMap.toList (fromStoredVersions ..)` is compared as an
//     ASCENDING list (BTreeMap keyed i64 — Data.IntMap's ascending
//     iteration is load-bearing there), and IntMap.lookupMax becomes an
//     explicit keys().max() (build_db_schema's maxPid)
//   Map SchemaId X / Map SchemaRef X           -> BTreeMap (Data.Map is
//     ORDERED; SchemaId = String and SchemaRef derive Ord —
//     compute_ids.rs:840 and angle schema/types.rs:73 precedents)
//   Map Text Version (thrift map fields)       -> BTreeMap<String, _>
//     (the pinned generated-crate image, backend/types.rs:152-156)
//   Set PredicateId / Set SchemaRef            -> BTreeSet (Set is
//     Ord-based; the RUNBOOK Set -> BTreeSet pin, typecheck.rs:2391)
//   HashSet PredicateRef (checkRecursiveDefinitions' bifoldMap acc) ->
//     BTreeSet — DELIBERATE, documented at the site: the source's
//     HashSet.toList feeds the DFS ADJACENCY order, which hashable fixes
//     per content but the API leaves unspecified; the port picks the
//     deterministic ascending image (the same cycles are found; member
//     rotation inside a rendered cycle can differ from hashable's order —
//     the source's own order is an implementation artifact, not spec)
//   Data.Graph (graphFromEdges/topSort/stronglyConnComp) -> the in-file
//     image below, PROBE-VERIFIED (pinned GHC 9.4.7, scratch
//     /tmp/graph_probe/{probe,probe2,probe3,probe4}.hs, 2026-10-26; the
//     schema/types.rs SCC-ORDERING probe discipline):
//       1. reverse (topSort g) is the plain POSTORDER of a DFS from
//          vertices in ascending-key order — with edges ref->deps it is
//          DEPENDENCIES-FIRST (probes 1-2: reverse topSort of a->b is
//          [b, a]).
//       2. graphFromEdges SORTS its edge list by key: vertex order is
//          ascending-key order, NOT edge-list order (probe4's
//          sort-keyed port matched all 15 graphs; the listing-order
//          candidate failed). Adjacency = the given key list; edges to
//          unknown keys are SILENTLY DROPPED (probe4 graph 13;
//          duplicate adjacency entries are harmless — visited-once).
//       3. stronglyConnComp = dfs on the ORIGINAL from roots
//          `reverse (postOrd (transposeG g))` (containers-0.6.7's scc,
//          verified against the v0.6.7 source): pass1 = postorder DFS on
//          the TRANSPOSE from vertices ascending; pass2 = DFS on the
//          original from the reversed postorder; CyclicSCC members are
//          the pass-2 trees flattened PRE-ORDER; a singleton tree is
//          cyclic iff it self-edges. The port reproduces this EXACTLY
//          (probe4: 15/15 graphs match on cycles AND postorder — incl.
//          diamond, twin cycles, shared-root cycle, self-loop,
//          unknown-key edges, duplicate adjacency).
//   Lazy.HashMap (typecheckSchema's typedefs, Schema.hs:822-825) -> the
//     memoized recursion at the site: laziness is unobservable in the
//     port EXCEPT for the knot's nontermination on recursive typedefs,
//     where the source's own comment (Types.hs:321-322, transcribed by
//     row 89) says "This will loop if we have recursive typedefs but we
//     don't allow those at the moment" — the memo walk's recursion
//     overflows in exactly that case (the evolve.rs:716-722 BUG(port)
//     nontermination class), so no marker: same class, same trigger.
//   `Data.List.Extra.sort` -> sort_by the derived Ord (PredicateId's Ord
//     is the STRUCTURAL (ref, hash) order — angle/types.rs:920-930 —
//     while its Eq is hash-only, both reproduced bug-for-bug by row 4)
//   `Safe (maximumMay)` -> `.max()` (Option) — Option-first per the brief
//   `TextShow (showt)` -> format!/{:?}/to_string per the schema/util.rs
//     :282-284 decimal ruling
//   `Control.Concurrent (MVar)` -> std::sync::Mutex — the source's cache
//     ops are readMVar/modifyMVar_ over an always-full cell (the cache
//     haddock, Schema.hs:285-311); A3's blocking-slot image is for
//     oneshot channels, not this. (The lock is held only for the
//     lookup/remove — no callback runs under it.)
//
// ---------------------------------------------------------------------------
// EXPORT MAPPING (source export list Schema.hs:10-33, all 24 names)
// ---------------------------------------------------------------------------
//
//   DbSchema(..)               -> pub use crate::database::schema::types::
//                                 DbSchema (the source RE-EXPORTS it from
//                                 .Types — Types.hs is imported open at
//                                 :74; same for the next four)
//   PredicateDetails(..)       -> pub use ...types::PredicateDetails
//   predicateRef               -> pub use ...types::predicate_ref
//   TypeDetails(..)            -> pub use ...types::TypeDetails
//   lookupPid                  -> pub use ...types::lookup_pid
//   schemaSize                 -> pub fn schema_size
//   schemaPredicates           -> pub fn schema_predicates
//   fromStoredSchema           -> pub fn from_stored_schema
//   toStoredSchema             -> pub fn to_stored_schema
//   newDbSchema                -> pub fn new_db_schema
//   newMergedDbSchema          -> pub fn new_merged_db_schema
//   mkDbSchemaFromSource       -> pub fn mk_db_schema_from_source
//   compareSchemaPredicates    -> pub fn compare_schema_predicates
//   validateNewSchema          -> pub fn validate_new_schema
//   validateNewSchemaInstance  -> pub fn validate_new_schema_instance
//   DbContent                  -> pub enum DbContent (DbWritable |
//                                 DbReadOnly(HashMap<Pid, PredicateStats>))
//                                 [Rust-forced widening: the source
//                                 exports the type WITHOUT (..) — the
//                                 constructors stay hidden (Schema.hs:23)
//                                 and only the smart constructors
//                                 readOnlyContent/readWriteContent build
//                                 them; a Rust enum's variants are public
//                                 whenever the enum is, so the port's
//                                 surface is constructor-widened — the
//                                 language leaves no choice]
//   readOnlyContent            -> pub fn read_only_content
//   readWriteContent           -> pub fn read_write_content
//   getSchemaInfo              -> pub fn get_schema_info
//   getSchemaInfoForSchema     -> pub fn get_schema_info_for_schema
//   renderSchemaSource         -> pub fn render_schema_source
//   toStoredVersions           -> pub fn to_stored_versions
//   derivationEdges            -> pub fn derivation_edges
//   newDbSchemaForTesting      -> pub fn new_db_schema_for_testing
// (fromStoredVersions is module-private here exactly as in the source —
// not in the export list.)
//
// Private-image notes: `newDbSchemaForTesting`'s rank-2
// `(forall k v. HashMap k v -> [(k,v)])` toList parameter is COLLAPSED
// (the compute_ids.rs:165-171 COLLAPSE and config.rs processSchema
// DIVERGENCES-5 precedent): the forall spans every call-site
// instantiation, which a Rust fn parameter cannot express, and the only
// value ever passed in production is HashMap.toList (:218, :205, :264,
// :1013, :1195) whose semantics — "enumerate the entries in unspecified
// order" — IS what std HashMap iteration does. The testing hook's purpose
// ("weed out ordering assumptions", Schema.hs:220-221) is thereby
// inherent; the parameter is dropped and new_db_schema delegates.
// `mkPredicateTransformation`'s `Eq id => (id -> PredicateDetails) ->
// id -> id` collapses to the PredicateId instantiation (its only use:
// mkTransformations :541) with the accessor returning a BORROW (the
// transitive_deps precedent, schema/types.rs:931); its `error` fallback
// propagates as the Err of the enclosing Either Text.
//
// ERROR IMAGE: the source throws ErrorCall (Schema.hs:193, :233, :262,
// :461, :883, :987-991, :1025, :1029-1030...), Thrift.Exception (:460,
// :1009, :1235) and userError (:1192) from these IO surfaces. The crate's
// single funnel is DbError (exception.rs:9's "all db throws funnel
// through DbError so the server/thrift mapping is single-point"; the
// enum at exception.rs:201-202); every throw site lands in DbError::Msg
// with the source's message text byte-for-byte (the exception-TYPE distinction is
// not preserved — no in-tree caller discriminates: grep of Open.hs/
// Create.hs/Validate.hs finds no catch around these calls — flagged as an
// open question for the inbound units). failOnLeft (:1022-1025) maps the
// Either Text channel onto the same funnel.
//
// Coverage: the 24-name export surface above + the private helpers
// (storedSchemaPids, dbSchemaKey + its Binary payload, withDbSchemaCache,
// mkDbSchema/buildDbSchema, mkTransformations, mkEvolutions,
// predicatesByRef, predicatesBySchemaRef, directEvolutions,
// calcAutoEvolutions, calcSchemaEvolutions, close, prune, compareInsert,
// mkPredicateTransformation, typecheckSchema, enableSelectiveDerivations,
// usesOfNegation, checkStoredType, failOnLeft, checkRecursiveDefinitions,
// definitions, findSchemaSource, findSchemaInDB, findSchemaInIndex, the
// Data.Graph image, showDoc) — everything the source's where-clauses and
// unexported top-levels carry.

pub mod compute_ids;
pub mod types;

// ---------------------------------------------------------------------------
// Named re-exports (the source re-exports these five from
// Glean.Database.Schema.Types via its open import at :74 — the export
// list names DbSchema(..), PredicateDetails(..), predicateRef,
// TypeDetails(..) and lookupPid; RULE §4 bars only CROSS-crate glob
// re-exports, and the named-re-export discipline applies).
// ---------------------------------------------------------------------------
pub use crate::database::schema::types::{
    predicate_ref, lookup_pid, DbSchema, PredicateDetails, TypeDetails,
};

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::rc::Rc;
use std::sync::Mutex;

use glean_angle::angle::hash::hash_binary;
use glean_angle::angle::types::{
    latest_angle_version, rm_loc_type, rm_type_loc_pred_def, Binary, DeriveWhen, DerivingInfo,
    PredicateDef_, PredicateId, SourceQuery_, SourceSchemas, SrcSpan, TypeDef_, TypeId, TypeRef,
    Type_,
};
use glean_angle::display::{display_default, Doc};
use glean_angle::schema::evolve::{
    calc_evolutions, can_be_stored, direct_schema_evolutions, map_visible, validate_evolutions,
    visible_predicates, VisiblePredicates,
};
use glean_angle::schema::types::{
    schema_ref, NameEnv, RefTargetId, ResolvedSchemaRef, SchemaRef,
};
use glean_angle::schema::util::ShowRef;
use glean_core::rts::foreign::inventory::{new as inventory_new, CompiledPredicate, Inventory};
use glean_core::rts::types::{lowest_pid, ExpandedType, Pid, PidRef, Type};
use glean_fb303::service_data::global_stats::add_stat_value_type;
use glean_fb303::service_data::types::ExportType;
use glean_thrift::{
    GetSchemaInfo, PredicateRef, PredicateStats, SchemaId, SchemaInfo, SelectSchema,
};
use glean_thrift::Config as ServerConfig;

use crate::database::config::{
    process_schema, schema_for_schema_id, DebugFlags, ProcessedSchema, SchemaIndex,
};
use crate::database::exception::DbError;
use crate::database::schema::compute_ids::HashedSchema;
use crate::database::schema::types::{
    mk_query_transformations, mk_rts_type, pid_ref, DbSchemaCache, DbSchemaCacheKey,
    PredicateTransformation, QueryTransformations, SchemaSelector,
};
use crate::internal::types::StoredSchema;
use crate::query::prune::prune_derivations;
use crate::query::transform::{transform_fact, transform_pattern};
use crate::query::typecheck::types::TcQuery;
use crate::query::typecheck::{
    default_tc_opts, empty_tc_env, tc_query_deps, tc_query_uses_negation, typecheck_deriving,
    TcEnv, TcOpts, ToRtsType, UseOfNegation,
};

// BLOCKED DEPENDENCIES (manifest rows 23/24 — NOT SHIPPED). See the
// BLOCKED DEPENDENCIES block above: these are LIVE uses of the source
// (Schema.hs:846-847), pinned from Typecheck.hs:206 and Traverse.hs:94.
// Step-4 compile error until the two units land; never a silent divergence.
use glean_core::rts::traverse::gen_traversal;   // rts/traverse.rs (row 23)
use glean_core::rts::typecheck::check_signature; // rts/typecheck.rs (row 24)

// ASSUMED-IMPORT (RULE §1 logging row / DECISIONS.md 2026-10-18): the
// pinned `vlog` seam (first sketch internal.rs:190-213, unit 45; consumed
// at query/thrift.rs:271-292), used here as a forward reference until
// glean-util's log unit lands — import-resolution swap only, per that
// block.
use glean_util::log::vlog;

// TODO(port): RENDER/LAYOUT PARITY (display.rs:537-542, TODO(port) #2 — it
// names Schema.hs:1220, this file's render_schema_source, as a
// parity-critical site and display.rs "deliberately renders nothing"). The
// call site is written in the display.rs-owned shape; the assumed surface
// (prettyprinter-1.7.1's contract, judged byte-for-byte against old-binary
// output when the render owner lands):
//     pub fn layout_compact(doc: &Doc) -> SimpleDoc;
//     pub fn render_strict(s: &SimpleDoc) -> String;
// Until then this is the Step-4 compile error, never a local hand-rolled
// renderer (RULE §1: one owner, no per-unit re-implementations).
use glean_angle::display::{layout_compact, render_strict};

/// 'Used to decide whether to activate 'derive default' definitions and
/// 'evolves' directives.
/// 'derive default' takes effect for a read-only DB when there are no
/// facts of a predicate.
/// 'evolves' directives take effect for a read-only DB when there are no
/// facts of any predicate in a schema evolved by another schema.'
/// (Schema.hs:106-114.)
pub enum DbContent {
    DbWritable,
    DbReadOnly(HashMap<Pid, PredicateStats>),
}

/// 'readOnlyContent = DbReadOnly' (Schema.hs:116-117).
pub fn read_only_content(stats: HashMap<Pid, PredicateStats>) -> DbContent {
    DbContent::DbReadOnly(stats)
}

/// 'readWriteContent = DbWritable' (Schema.hs:119-120).
pub fn read_write_content() -> DbContent {
    DbContent::DbWritable
}

/// 'schemaSize = HashMap.size . predicatesById' (Schema.hs:123-124).
pub fn schema_size(db_schema: &DbSchema) -> usize {
    db_schema.predicates_by_id.len()
}

/// 'schemaPredicates = HashMap.elems . predicatesById' (Schema.hs:126-127).
/// Haskell elems order is unspecified; the consumers (Backups/Shell) only
/// iterate, so the port returns the values unordered.
pub fn schema_predicates(db_schema: &DbSchema) -> Vec<&PredicateDetails> {
    db_schema.predicates_by_id.values().collect()
}

/// 'fromStoredSchema' (Schema.hs:130-143): build a DbSchema from the
/// StoredSchema stored in a DB.
pub fn from_stored_schema(
    schema_cache: Option<&Mutex<DbSchemaCache>>,
    info: &StoredSchema,
    db_content: &DbContent,
    debug: &DebugFlags,
) -> Result<DbSchema, DbError> {
    mk_db_schema_from_source(
        schema_cache,
        Some(&stored_schema_pids(info)),
        db_content,
        debug,
        &info.schema,
    )
}

/// 'storedSchemaPids' (Schema.hs:144-146): the stored schema's
/// (PredicateRef -> Pid) map. Data.Map.toList is ASCENDING by Id; the
/// BTreeMap image iterates ascending too (ASSUMED-IMPORT note on
/// predicate_ids), so the later-entry-wins semantics of the HashMap the
/// source builds is reproduced.
fn stored_schema_pids(info: &StoredSchema) -> HashMap<PredicateRef, Pid> {
    info.predicate_ids
        .iter()
        .map(|(pid, r#ref)| (r#ref.clone(), Pid(*pid)))
        .collect()
}

/// 'toStoredSchema' (Schema.hs:148-162). Note: only the stored predicates
/// are written — 'There can be multiple predicates with the same
/// PredicateRef (different Pid and PredicateId), but only one of them is
/// from the stored schema and corresponds to the Pids of facts stored in
/// the DB.' (source comment, Schema.hs:157-160, kept).
pub fn to_stored_schema(db_schema: &DbSchema) -> StoredSchema {
    StoredSchema {
        schema: render_schema_source(&db_schema.schema_source.0),
        predicate_ids: db_schema
            .predicates_by_id
            .values()
            .filter(|p| p.predicate_in_stored_schema)
            .map(|p| (p.predicate_pid.0, predicate_ref(p)))
            .collect(),
        versions: to_stored_versions(
            &db_schema.schema_source.1,
            &db_schema.schema_source.2,
        ),
    }
}

/// 'newMergedDbSchema' (Schema.hs:164-206): the DB's stored schema merged
/// with the current schema index.
pub fn new_merged_db_schema(
    schema_cache: Option<&Mutex<DbSchemaCache>>,
    stored_schema: &StoredSchema,
    index: &SchemaIndex,
    db_content: &DbContent,
    debug: &DebugFlags,
) -> Result<DbSchema, DbError> {
    // 'If we have an identical schema in the index, then we can use the
    // cached ProcessedSchema. ... Schema equality is by comparing the
    // SchemaIds. This is safe since the only way a schema can be used is
    // through a SchemaId ...' (source comment, Schema.hs:174-182, kept).
    //
    // identicalSchemas (Schema.hs:183-189):
    //   IntMap.toList (fromStoredVersions storedSchema_versions) ==
    //     [(fromIntegral hashedSchemaAllVersion, hashedSchemaId)]
    // — the stored versions map (ordered, ascending) must equal the
    // ONE-element list. The port compares ascending Vecs (BTreeMap
    // iterates ascending — the IntMap ORDERED image, MAP IMAGES).
    let stored_versions: Vec<(i64, &SchemaId)> =
        from_stored_versions(&stored_schema.versions)
            .iter()
            .map(|(k, v)| (*k, v))
            .collect();

    let candidates = index_schemas(index);

    // The identical-schema hit returns the SHARED ProcessedSchema (the
    // source shares the value, Schema.hs:200); a miss parses fresh. The
    // late-init local carries the owned parse result across the borrow.
    let processed_local;
    let from_db: &ProcessedSchema = if stored_schema.versions.is_empty() {
        processed_local = process_stored_schema_text(stored_schema)?;
        &processed_local
    } else {
        match candidates.iter().find(|proc| {
            let h = &proc.proc_schema_hashed;
            stored_versions
                == vec![(
                    h.hashed_schema_all_version as i64,
                    &h.hashed_schema_id,
                )]
        }) {
            // 'For the "source", we'll use the current schema source. It
            // doesn't reflect what's in the DB, but it reflects what you
            // can query for.' (source comment, Schema.hs:203-204, kept.)
            Some(proc) => proc,
            None => {
                processed_local = process_stored_schema_text(stored_schema)?;
                &processed_local
            }
        }
    };

    mk_db_schema(
        schema_cache,
        Some(&stored_schema_pids(stored_schema)),
        db_content,
        from_db,
        Some(index),
        debug,
    )
}

/// 'processStoredSchema' (Schema.hs:191-194): parse+resolve+hash the DB's
/// stored schema text; Left -> ErrorCall.
fn process_stored_schema_text(stored_schema: &StoredSchema) -> Result<ProcessedSchema, DbError> {
    process_schema(None, &stored_schema.schema).map_err(DbError::Msg)
}

/// The index view `[schemaIndexCurrent, schemaIndexOlder..]` — the source's
/// `schemaIndexCurrent index : schemaIndexOlder index` (Schema.hs:185,
/// :278-279, :388-390).
fn index_schemas(index: &SchemaIndex) -> Vec<&ProcessedSchema> {
    std::iter::once(&index.schema_index_current)
        .chain(index.schema_index_older.iter())
        .collect()
}

/// The build view `stored : otherSchemas` (Schema.hs:569, :1091-1092,
/// :1129-1133).
fn processed_list<'a>(
    stored: &'a ProcessedSchema,
    others: &[&'a ProcessedSchema],
) -> Vec<&'a ProcessedSchema> {
    std::iter::once(stored).chain(others.iter().copied()).collect()
}

/// 'newDbSchema = newDbSchemaForTesting HashMap.toList' (Schema.hs:211-218).
pub fn new_db_schema(
    schema_cache: Option<&Mutex<DbSchemaCache>>,
    index: &SchemaIndex,
    selector: &SchemaSelector,
    db_content: &DbContent,
    debug: &DebugFlags,
) -> Result<DbSchema, DbError> {
    new_db_schema_for_testing(schema_cache, index, selector, db_content, debug)
}

/// 'Testing version of 'newDbSchema' that accepts a custom @HashMap.toList@
/// function to weed out ordering assumptions' (Schema.hs:220-229).
///
/// The rank-2 `toList` parameter is COLLAPSED (header EXPORT MAPPING): the
/// only production value is HashMap.toList, whose "entries in unspecified
/// order" is what std HashMap iteration is; the testing hook's purpose is
/// inherent (config.rs DIVERGENCES 5 / compute_ids.rs COLLAPSE precedent).
pub fn new_db_schema_for_testing(
    schema_cache: Option<&Mutex<DbSchemaCache>>,
    index: &SchemaIndex,
    selector: &SchemaSelector,
    db_content: &DbContent,
    debug: &DebugFlags,
) -> Result<DbSchema, DbError> {
    let schema: &ProcessedSchema = match selector {
        SchemaSelector::SpecificSchemaId(id) => match schema_for_schema_id(index, id) {
            Some(schema) => schema,
            // 'throwIO $ ErrorCall $ "schema " <> show id <> " not found"'
            // (:233). The source's `show id` renders the gen-hs2 Show of
            // the SchemaId newtype (`SchemaId {unSchemaId = "..."}`); the
            // port's SchemaId is a String alias, so its Debug (the quoted
            // text) is the closest available image — the string CONTENT is
            // preserved, the wrapper shape is not (ASSUMED-IMPORT note).
            None => {
                return Err(DbError::Msg(format!("schema {:?} not found", id)));
            }
        },
        _otherwise => &index.schema_index_current,
    };
    mk_db_schema(schema_cache, None, db_content, schema, None, debug)
}

/// 'inventory' (Schema.hs:239-247): the RTS inventory over the predicates'
/// compiled typecheck/traversal subroutines.
///
/// CompiledPredicate OWNS its Subroutine fields (inventory.rs:238-248),
/// but the details keep theirs — the clone is the PredicateDetails: Clone
/// seam (SHARING WALLS 1).
fn inventory<'a>(ps: impl IntoIterator<Item = &'a PredicateDetails>) -> Inventory {
    let compiled: Vec<CompiledPredicate> = ps
        .into_iter()
        .map(|d| CompiledPredicate {
            pid: d.predicate_pid,
            r#ref: predicate_ref(d),
            // TODO(port): SHARING WALLS 1 — needs impl Clone for
            // Subroutine<S> (bytecode.rs:268-270: the ForeignPtr inside
            // is already clone-shared; row 18's additive derive) and for
            // QueryWithInfo/TypecheckedQuery (rows 87/88). Step-4 compile
            // error until then; the Haskell shares the handle (the C++
            // SharedSubroutine is refcounted), the clone reproduces that
            // sharing exactly once the derive lands.
            typecheck: d.predicate_typecheck.clone(),
            traversal: d.predicate_traversal.clone(),
        })
        .collect();
    inventory_new(&compiled)
}

/// 'Make a DbSchema for an existing database, using the predicates stored
/// in the database. Anything in the 'Schemas' but not in the database is
/// dropped from the 'DbSchema'' (Schema.hs:249-266).
pub fn mk_db_schema_from_source(
    schema_cache: Option<&Mutex<DbSchemaCache>>,
    known_pids: Option<&HashMap<PredicateRef, Pid>>,
    db_content: &DbContent,
    debug: &DebugFlags,
    source: &[u8],
) -> Result<DbSchema, DbError> {
    let processed = process_schema(None, source).map_err(DbError::Msg)?;
    mk_db_schema(schema_cache, known_pids, db_content, &processed, None, debug)
}

/// 'dbSchemaKey' (Schema.hs:268-283): the DbSchema cache key —
/// hashBinary over ([(showRef ref, pid)], versions, index-versions).
fn db_schema_key(
    pid_map: &HashMap<PredicateRef, Pid>,
    stored: &ProcessedSchema,
    maybe_index: Option<&SchemaIndex>,
) -> DbSchemaCacheKey {
    // `HashMap.toList pidMap` order is hashable's artifact and feeds a
    // HASH; the port sorts (deterministic within a process, which is all
    // the key needs — it never leaves the cache).
    let mut pids: Vec<(String, i64)> = pid_map
        .iter()
        .map(|(r, p)| (r.show_ref(), p.0))
        .collect();
    pids.sort();
    let versions = {
        let h = &stored.proc_schema_hashed;
        (h.hashed_schema_all_version, h.hashed_schema_id.clone())
    };
    let index_versions: Vec<(i32, String)> = match maybe_index {
        None => Vec::new(),
        Some(index) => index_schemas(index)
            .iter()
            .map(|p| {
                let h = &p.proc_schema_hashed;
                (h.hashed_schema_all_version, h.hashed_schema_id.clone())
            })
            .collect(),
    };
    DbSchemaCacheKey(hash_binary(&CacheKeyPayload {
        pids,
        versions,
        index_versions,
    }))
}

/// The hashBinary payload of dbSchemaKey: the source's 3-tuple
/// `([(showRef ref, pid)], (allVersion, unSchemaId id),
/// [(allVersion, unSchemaId id)])`. Hand-written Binary replicating the
/// Data.Binary generic composition (RULE §2 Data.Binary row):
/// tuple = concat of parts; list = Int64be count + elements; pair =
/// concat; Text = Int64be len + UTF-8 (angle/types.rs:248-254);
/// Version = Int32 = 4-byte BE (types.rs:241); Id = Int = 8-byte BE
/// (types.rs:245). (The generic Vec<T> impl is not shipped — only
/// encode_list — so the payload is spelled out; bytes identical.)
struct CacheKeyPayload {
    pids: Vec<(String, i64)>,
    versions: (i32, String),
    index_versions: Vec<(i32, String)>,
}

impl Binary for CacheKeyPayload {
    fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend((self.pids.len() as i64).to_be_bytes());
        for (name, pid) in &self.pids {
            out.extend(name.encode());
            out.extend(pid.to_be_bytes());
        }
        out.extend(self.versions.0.to_be_bytes());
        out.extend(self.versions.1.encode());
        out.extend((self.index_versions.len() as i64).to_be_bytes());
        for (v, id) in &self.index_versions {
            out.extend(v.to_be_bytes());
            out.extend(id.encode());
        }
        out
    }
}

/// 'Caching DbSchema ... DbSchema is quite expensive to build ... So we'd
/// like to cache these to avoid repeating the work and to share as much as
/// possible between open DBs.' (Schema.hs:285-311 — the haddock kept in
/// substance at the two branch sites below.)
///
/// 'withDbSchemaCache' (Schema.hs:313-339). The lock is std::sync::Mutex
/// (MAP IMAGES: readMVar/modifyMVar_ over an always-full cell); no
/// callback runs under it.
fn with_db_schema_cache<F>(
    maybe_cache: Option<&Mutex<DbSchemaCache>>,
    maybe_pids: Option<&HashMap<PredicateRef, Pid>>,
    stored: &ProcessedSchema,
    maybe_index: Option<&SchemaIndex>,
    mk: F,
) -> Result<DbSchema, DbError>
where
    F: FnOnce() -> Result<DbSchema, DbError>,
{
    match (maybe_cache, maybe_pids) {
        // 'only use the cache when we have stored Pids' (:323)
        (Some(cache_var), Some(pid_map)) => {
            let key = db_schema_key(pid_map, stored, maybe_index);
            let cache = cache_var.lock().expect("DbSchema cache mutex poisoned");
            if let Some(schema) = cache.remove(&key) {
                add_stat_value_type(b"glean.db.schema.cache.hit", 1, ExportType::Sum);
                vlog(2, || {
                    format!(
                        "DbSchema cache hit, stored SchemaId: {}",
                        stored.proc_schema_hashed.hashed_schema_id
                    )
                });
                return Ok(schema);
            }
            drop(cache);
            add_stat_value_type(b"glean.db.schema.cache.miss", 1, ExportType::Sum);
            let schema = mk()?;
            // TODO(port): SHARING WALLS 2 — the source inserts the built
            // schema into the cache AND returns the same value
            // (`modifyMVar_ cacheVar $ return . HashMap.insert key schema`,
            // Schema.hs:335-336): one Inventory handle owned by both cache
            // and caller. DbSchema is not Clone (schema_inventory is a
            // unique FFI handle with Drop — inventory.rs:296-301/:341), so
            // the
            // insert-and-return cannot be expressed; the port SKIPS the
            // insert — the cache is write-starved (entries only ever leave
            // via the hit/remove path above, so in practice every open
            // rebuilds). Restoring the source's sharing needs an
            // Arc<DbSchema> cache payload (a row-89 DbSchemaCache alias
            // change) or an Inventory sharing seam (row 25); the hit path
            // and the exact key computation above are kept so that fix
            // lands without re-shaping this file. PERF(port): the rebuild
            // is the expensive bytecode compilation the cache exists to
            // amortize.
            let _ = key;
            Ok(schema)
        }
        _otherwise => {
            add_stat_value_type(b"glean.db.schema.cache.fail", 1, ExportType::Sum);
            mk()
        }
    }
}

/// 'mkDbSchema' (Schema.hs:341-390). The rank-2 toList parameter is
/// collapsed (EXPORT MAPPING).
///
/// Flow (source :353-385): the cache lookup covers the expensive part
/// (buildDbSchema); then the DbContent-dependent pass runs per DB —
/// mkTransformations (BORROWS the details), prune (CONSUMES them — row
/// 93's obligation), enableSelectiveDerivations, the pid map and the
/// per-SchemaId query transformations.
fn mk_db_schema(
    cache_var: Option<&Mutex<DbSchemaCache>>,
    known_pids: Option<&HashMap<PredicateRef, Pid>>,
    db_content: &DbContent,
    proc_stored: &ProcessedSchema,
    index: Option<&SchemaIndex>,
    debug: &DebugFlags,
) -> Result<DbSchema, DbError> {
    // addedSchemas (:388-390): the index's current then older.
    let added_schemas: Vec<&ProcessedSchema> = match index {
        None => Vec::new(),
        Some(i) => index_schemas(i),
    };

    // 'either fetch a cached DbSchema or build a new one' (:353-356).
    let schema = with_db_schema_cache(
        cache_var,
        known_pids,
        proc_stored,
        index,
        || build_db_schema(known_pids, proc_stored, &added_schemas, debug),
    )?;

    // mkTransformations (:360-364) — borrows the details.
    let transformations = mk_transformations(db_content, &schema.predicates_by_id, proc_stored, &added_schemas)
        .map_err(DbError::Msg)?;

    // allTrans (:378): `fold $ HashMap.elems transformations` — the
    // left-biased union over an UNSPECIFIED elem order (the source's own
    // HashMap order; per-schema transformations can in principle conflict
    // on the same pid and the source picks whichever comes first — the
    // port's std HashMap order is the same nondeterminism class). The
    // union and mkQueryTransformations both consume the same maps and
    // PredicateTransformation is not Clone, so the union is built as the
    // LIGHTWEIGHT projection prune reads: `transformationFor` only uses
    // tAvailable's pid (Schema.hs:730-734). First-seen wins (left-biased).
    let mut all_trans_pid: HashMap<i64, i64> = HashMap::new();
    for m in transformations.values() {
        for (k, t) in m {
            all_trans_pid.entry(*k).or_insert(t.t_available.pid.0);
        }
    }

    // byId (:371-373): `enableSelectiveDerivations dbContent <$>
    //   prune dbContent allTrans (predicatesById schema)` — prune CONSUMES
    // the map (row 93's obligation; the PRUNE CONSUMER OBLIGATION block).
    let predicates_by_id = prune(db_content, &all_trans_pid, schema.predicates_by_id);
    for details in predicates_by_id.values_mut() {
        enable_selective_derivations(db_content, details);
    }

    // intPid / byPid (:375-376): `IntMap.fromList [(intPid p, p) | p <-
    // HashMap.elems byId]` — duplicate pids would be last-wins over an
    // unordered iteration in the source; the port's collect is the same
    // last-wins class.
    //
    // TODO(port): SHARING WALLS 1 — the source's byPid and byId share the
    // SAME PredicateDetails values (Schema.hs:507-510); the port must
    // clone each one into the pid map (PredicateDetails is not Clone —
    // Subroutine + TypecheckedQuery; see SHARING WALLS 1 for the exact
    // additive seams, rows 18/87/88). Cloning preserves semantics: both
    // maps are immutable after construction. Step-4 compile error until
    // the seam lands.
    let by_pid: HashMap<i64, PredicateDetails> = predicates_by_id
        .values()
        .map(|d| (d.predicate_pid.0, d.clone()))
        .collect();

    // predicatesTransformations (:383-384): `mkQueryTransformations byPid
    // <$> transformations` — each per-SchemaId map CONSUMED.
    let predicates_transformations: HashMap<SchemaId, QueryTransformations> = transformations
        .into_iter()
        .map(|(schema_id, tmap)| (schema_id, mk_query_transformations(&by_pid, tmap)))
        .collect();

    let DbSchema {
        predicates_by_id: _,
        predicates_by_pid: _,
        predicates_transformations: _,
        schema_inventory,
        schema_envs,
        types_by_id,
        schema_source,
        schema_max_pid,
        schema_all_version,
        schema_id,
        derivation_depends,
    } = schema;

    Ok(DbSchema {
        predicates_by_id,
        predicates_by_pid: by_pid,
        predicates_transformations,
        schema_inventory,
        schema_envs,
        types_by_id,
        schema_source,
        schema_max_pid,
        schema_all_version,
        schema_id,
        derivation_depends,
    })
}

/// 'buildDbSchema' (Schema.hs:392-518) — the expensive, cacheable part.
fn build_db_schema(
    known_pids: Option<&HashMap<PredicateRef, Pid>>,
    proc_stored: &ProcessedSchema,
    added_schemas: &[&ProcessedSchema],
    debug: &DebugFlags,
) -> Result<DbSchema, DbError> {
    let stored = &proc_stored.proc_schema_hashed;

    // ---- Assign Pids to predicates (:394-424). ----------------------------
    // '- predicates in the stored schema get Pids from the StoredSchema
    //    (unless this is a new DB, in which case we'll assign fresh Pids)
    //  - predicates in the merged schema get new fresh Pids.
    //  Sort the predicates in order to ensure a deterministic result since
    //  'toList' does not give us any ordering guarantees' (source comment,
    //  Schema.hs:394-400, kept). `sort` is PredicateId's STRUCTURAL Ord
    //  (angle/types.rs:920-930).
    let mut sorted_ids: Vec<PredicateId> = stored.hashed_preds.keys().cloned().collect();
    sorted_ids.sort();

    let stored_pids: Vec<(PredicateId, Pid)> = match known_pids {
        None => sorted_ids
            .iter()
            .enumerate()
            .map(|(i, id)| (id.clone(), Pid(lowest_pid().0 + i as i64)))
            .collect(),
        Some(pid_map) => {
            // first = maybe lowestPid succ $ maximumMay (snd <$> toList
            // pidMap) (:407)
            let mut next = pid_map
                .values()
                .map(|p| p.0)
                .max()
                .map(|m| m + 1)
                .unwrap_or_else(|| lowest_pid().0);
            sorted_ids
                .iter()
                .map(|id| match pid_map.get(&id.predicate_id_ref) {
                    Some(pid) => (id.clone(), *pid),
                    // assign next (id : ids) (:408-412)
                    None => {
                        let pid = Pid(next);
                        next += 1;
                        (id.clone(), pid)
                    }
                })
                .collect()
        }
    };

    // nextPid (:414): the successor of the largest assigned pid.
    let next_pid = stored_pids
        .iter()
        .map(|(_, p)| p.0)
        .max()
        .map(|m| m + 1)
        .unwrap_or_else(|| lowest_pid().0);

    // addedPreds (:416-417): the union of the added schemas' hashed preds
    // (HashMap.unions is LEFT-biased: earlier schemas win — the port's
    // or_insert keeps the first, same bias).
    let mut added_preds: HashMap<PredicateId, PredicateDef_<SrcSpan, SrcSpan, PredicateId, TypeId>> =
        HashMap::new();
    for s in added_schemas {
        for (id, def) in &s.proc_schema_hashed.hashed_preds {
            added_preds.entry(id.clone()).or_insert(def.clone());
        }
    }

    // addedPids (:419-422): `zip ids [nextPid ..]` over the added preds
    // that are NOT stored. `HashMap.keys` order is hashable's artifact;
    // the port sorts.
    // TODO(port): the sorted image diverges from hashable's order — which,
    // unlike std HashMap's, is REPRODUCIBLE in the old binary (same
    // key-set, same iteration order), and the resulting Pid assignment
    // difference is observable cross-binary via getSchemaInfo.predicateIds
    // and the Pids of derived facts. Owner: the Step-6 judge's
    // cross-binary comparison / the unordered-container convention review.
    let mut added_ids: Vec<PredicateId> = added_preds
        .keys()
        .filter(|id| !stored.hashed_preds.contains_key(*id))
        .cloned()
        .collect();
    added_ids.sort();
    let added_pids: Vec<(PredicateId, Pid)> = added_ids
        .iter()
        .enumerate()
        .map(|(i, id)| (id.clone(), Pid(next_pid + i as i64)))
        .collect();

    // idToPid (:424) — keys are unique (addedPids excludes stored ids).
    let id_to_pid: HashMap<PredicateId, Pid> = stored_pids
        .iter()
        .chain(added_pids.iter())
        .map(|(id, pid)| (id.clone(), *pid))
        .collect();

    // ---- typecheck all the schemas (:426-437). ----------------------------
    // The source's `typecheck` helper (:427-432) carries its tcOpts in a
    // `where` clause hanging off the PER-ELEMENT equation — the fold
    // (`foldM typecheck emptyTcEnv ((True, procStored) : map (False,)
    // addedSchemas)`, :435-436) computes a FRESH tcOpts for EACH schema,
    // from that schema's OWN resolved list:
    //     angleVersion = maybe latestAngleVersion resolvedSchemaAngleVersion
    //       (listToMaybe (schemasResolved resolved))
    // An added schema therefore typechecks under ITS OWN Angle version,
    // never under the stored schema's — and the field is load-bearing:
    // TcOpts.angleVersion gates the functional-predicate rule
    // (Typecheck.hs:158-160, `eqType (tcOptAngleVersion opts) unit
    // predicateValueType`). One tcOpts per step, computed inside the
    // loop (the per-equation where-clause placement reproduced).
    let typecheck_step = |is_stored: bool,
                          processed: &ProcessedSchema,
                          tc_env: &mut TcEnv|
     -> Result<(), DbError> {
        let angle_version = processed
            .proc_schema_resolved
            .schemas_resolved
            .first()
            .map(|r| r.resolved_schema_angle_version)
            .unwrap_or_else(|| latest_angle_version());
        // defaultTcOpts debug angleVersion (:430) — DebugFlags is not
        // Clone (config.rs:644) and the source shares it; the port
        // reconstructs the record from its Copy fields (the field-wise
        // image of the sharing).
        let tc_opts = default_tc_opts(
            DebugFlags {
                tc_debug: debug.tc_debug,
                query_debug: debug.query_debug,
            },
            angle_version,
        );
        typecheck_schema(
            &id_to_pid,
            is_stored,
            &tc_opts,
            &processed.proc_schema_hashed,
            tc_env,
        )
    };

    let mut tc_env = empty_tc_env();
    typecheck_step(true, proc_stored, &mut tc_env)?;
    for s in added_schemas {
        typecheck_step(false, s, &mut tc_env)?;
    }

    // ---- the negation invariant (:439-462). -------------------------------
    // 'Check the invariant that stored predicates do not make use of
    // negation or refer to derived predicates that make use of it. See
    // Note [Negation in stored predicates]' (source comment, kept).
    let using_negation = uses_of_negation(&tc_env.tc_env_predicates);
    for d in tc_env.tc_env_predicates.values() {
        let stored_deriving = match &d.predicate_deriving {
            // isStored (:443-448)
            DerivingInfo::NoDeriving => true,
            DerivingInfo::Derive(when, _) => matches!(
                when,
                DeriveWhen::DerivedAndStored | DeriveWhen::DeriveIfEmpty
            ),
        };
        if stored_deriving {
            if let Some(use_) = using_negation.get(&d.predicate_id) {
                let feature = match use_ {
                    UseOfNegation::PatternNegation => "negation",
                    UseOfNegation::IfStatement => "if-statements",
                };
                return Err(DbError::Msg(format!(
                    "use of {} is not allowed in a stored predicate: {}",
                    feature,
                    predicate_ref(d).show_ref()
                )));
            }
        }
    }

    // ---- assemble the record (:464-518). ----------------------------------
    let latest_hashed = match added_schemas.first() {
        // the index's CURRENT (:479-482) — added_schemas[0] IS it when the
        // index exists (index_schemas puts current first).
        Some(current) => &current.proc_schema_hashed,
        None => stored,
    };

    let db_schema_id = stored.hashed_schema_id.clone();
    let db_schema_all_version = stored.hashed_schema_all_version;

    // schemaEnvMap (:474-477): Map.fromList over (procStored :
    // addedSchemas) — dup ids last-wins, same as collect.
    let schema_envs: BTreeMap<SchemaId, NameEnv<RefTargetId>> = processed_list(proc_stored, added_schemas)
        .into_iter()
        .map(|p| {
            let h = &p.proc_schema_hashed;
            (h.hashed_schema_id.clone(), h.hashed_schema_env.clone())
        })
        .collect();

    // derivationDepends (:488-489): fromListWith (++) over the edges —
    // keys are unique (one edge per predicate with a derivation), so the
    // combine never fires.
    let derivation_depends: HashMap<PredicateId, Vec<PredicateId>> =
        derivation_edges_map(&tc_env.tc_env_predicates);

    vlog(2, || {
        format!(
            "DB schema {} has {} types/{} predicates, global schema has {} types/{} predicates, final schema has {} types/{} predicates.",
            stored.hashed_schema_id,
            stored.hashed_types.len(),
            stored.hashed_preds.len(),
            added_schemas
                .iter()
                .map(|s| s.proc_schema_hashed.hashed_types.len())
                .sum::<usize>(),
            added_schemas
                .iter()
                .map(|s| s.proc_schema_hashed.hashed_preds.len())
                .sum::<usize>(),
            tc_env.tc_env_types.len(),
            tc_env.tc_env_predicates.len(),
        )
    });
    vlog(2, || {
        format!(
            "all.{} = {}",
            latest_hashed.hashed_schema_all_version, latest_hashed.hashed_schema_id
        )
    });

    // byPid (:467-469) + maxPid (:471-472) + the inventory (:512) — all
    // computed from the env's predicates BEFORE the map moves into the
    // record.
    //
    // TODO(port): SHARING WALLS 1 — the source's byPid and byId share the
    // SAME PredicateDetails values (Schema.hs:469/:507-510); the port
    // clones each one into the pid map (PredicateDetails is not Clone —
    // Subroutine + TypecheckedQuery; see SHARING WALLS 1 for the exact
    // additive seams, rows 18/87/88). Cloning preserves semantics: both
    // maps are immutable after construction. Step-4 compile error until
    // the seam lands.
    let predicates_by_pid: HashMap<i64, PredicateDetails> = tc_env
        .tc_env_predicates
        .values()
        .map(|d| (d.predicate_pid.0, d.clone()))
        .collect();
    let max_pid = predicates_by_pid
        .keys()
        .max()
        .map(|&k| Pid(k))
        .unwrap_or_else(lowest_pid);
    let schema_inventory = inventory(tc_env.tc_env_predicates.values());

    Ok(DbSchema {
        predicates_by_id: tc_env.tc_env_predicates,
        types_by_id: tc_env.tc_env_types,
        schema_envs,
        predicates_by_pid,
        predicates_transformations: HashMap::new(),
        schema_inventory,
        schema_source: (proc_stored.proc_schema_source.clone(), db_schema_all_version, db_schema_id),
        schema_max_pid: max_pid,
        schema_all_version: latest_hashed.hashed_schema_all_version,
        schema_id: latest_hashed.hashed_schema_id.clone(),
        derivation_depends,
    })
}

/// 'mkTransformations' (Schema.hs:520-575): the per-SchemaId pid-keyed
/// query transformations, given the DB's content.
fn mk_transformations(
    content: &DbContent,
    by_id: &HashMap<PredicateId, PredicateDetails>,
    stored: &ProcessedSchema,
    other_schemas: &[&ProcessedSchema],
) -> Result<HashMap<SchemaId, HashMap<i64, PredicateTransformation>>, String> {
    // processed :: [ProcessedSchema] (:568-569)
    let processed = processed_list(stored, other_schemas);

    match content {
        DbContent::DbWritable => {
            // 'Right $ HashMap.fromList [(schemaId, mempty) | schema <-
            // processed, schemaId <- versionsOfAll schema]' (:528-532)
            Ok(processed
                .iter()
                .map(|s| (versions_of_all(s), HashMap::new()))
                .collect())
        }
        DbContent::DbReadOnly(stats) => {
            // evolutions :: [HashMap PredicateId PredicateId] (:534-536)
            let mut evolutions: Vec<HashMap<PredicateId, PredicateId>> = Vec::new();
            for schema in &processed {
                evolutions.push(mk_evolutions(stats, by_id, &[stored, *schema])?);
            }

            // allPredicateTrans (:537-542): nubOrd over the concatenated
            // pairs (first occurrence wins), keeping only the pairs whose
            // mkPredicateTransformation is a Just. nubOrd dedupes on Ord —
            // the tuple's STRUCTURAL (ref, hash) order, the same class a
            // BTreeSet keyed on the pair uses (a hash-only-Eq set would
            // additionally collapse same-hash/different-ref pairs the
            // source keeps distinct). The source stores the SHARED
            // transformation value here; PredicateTransformation is not
            // Clone (its two Box closures), so the port records the
            // DECIDED pairs and constructs a fresh (pure, equivalent)
            // transformation per occurrence in the pass below — the
            // sharing of an immutable value is unobservable.
            let mut just_pairs: BTreeSet<(PredicateId, PredicateId)> = BTreeSet::new();
            let mut seen: BTreeSet<(PredicateId, PredicateId)> = BTreeSet::new();
            for e in &evolutions {
                for (old, new) in e {
                    let pair = (old.clone(), new.clone());
                    if seen.insert(pair.clone()) {
                        if mk_predicate_transformation(&|id| details_by_id(by_id, id), old, new)?
                            .is_some()
                        {
                            just_pairs.insert(pair);
                        }
                    }
                }
            }

            // transformations (:543-556): one pid-keyed map per evolutions
            // map; `let intPid = fromIntegral $ fromPid $ predicatePid $
            // detailsById old` (:550-553).
            let mut trans_maps: Vec<HashMap<i64, PredicateTransformation>> = Vec::new();
            for e in &evolutions {
                let mut m: HashMap<i64, PredicateTransformation> = HashMap::new();
                for (old, new) in e {
                    if just_pairs.contains(&(*old, *new)) {
                        if let Some(trans) =
                            mk_predicate_transformation(&|id| details_by_id(by_id, id), old, new)?
                        {
                            let details = details_by_id(by_id, old)?;
                            m.insert(details.predicate_pid.0, trans);
                        }
                    }
                }
                trans_maps.push(m);
            }

            // 'Transformations are per ProcessedSchema. Here we find the
            // transformations applicable to the union of the stored and a
            // foreign ProcessedSchema and then we associate them with all
            // 'all' schemas of the foreign ProcessedSchema.' (source
            // comment, Schema.hs:558-561, kept.)
            Ok(processed
                .iter()
                .zip(trans_maps.into_iter())
                .map(|(s, trans)| (versions_of_all(s), trans))
                .collect())
        }
    }
}

/// detailsById (:574-575) — the source's `error` fallback; note the
/// message says "mkEvolutions" even though this is mkTransformations (the
/// source's own quirk, kept bug-for-bug).
fn details_by_id<'d>(
    by_id: &'d HashMap<PredicateId, PredicateDetails>,
    id: &PredicateId,
) -> Result<&'d PredicateDetails, String> {
    by_id
        .get(id)
        .ok_or_else(|| format!("mkEvolutions: {}", show_doc(&display_default(id))))
}

/// 'versionsOfAll :: ProcessedSchema -> [SchemaId]' (:571-572): a ONE-element
/// list (the name promises more than the source delivers — kept as-is).
fn versions_of_all(s: &ProcessedSchema) -> SchemaId {
    s.proc_schema_hashed.hashed_schema_id.clone()
}

/// 'Calculate predicate evolutions for a set of ProcessedSchema'
/// (Schema.hs:578-603).
fn mk_evolutions(
    stats: &HashMap<Pid, PredicateStats>,
    by_id: &HashMap<PredicateId, PredicateDetails>,
    schemas: &[&ProcessedSchema],
) -> Result<HashMap<PredicateId, PredicateId>, String> {
    // hasFacts (:593-597)
    let has_facts = |id: &PredicateId| -> bool {
        match by_id.get(id) {
            Some(details) => stats
                .get(&details.predicate_pid)
                .map(|s| s.count > 0)
                .unwrap_or(false),
            None => false,
        }
    };

    let (types, preds) = definitions(schemas);
    // byPredRef / bySchemaRef (:599-603)
    let by_pred_ref = predicates_by_ref(schemas);
    let by_schema_ref = predicates_by_schema_ref(schemas);

    let manual = calc_schema_evolutions(&has_facts, &by_schema_ref, &direct_evolutions(schemas)?);
    let auto = calc_auto_evolutions(&has_facts, &by_pred_ref)?;
    let evolutions = calc_evolutions(
        |id: &PredicateId| id.predicate_id_ref.clone(),
        &by_pred_ref,
        &by_schema_ref,
        &manual,
        &auto,
    )?;
    validate_evolutions(Some(&has_facts), &types, &preds, &evolutions)?;
    Ok(evolutions)
}

/// 'Predicates (which can be evolved) by predicate ref' (:605-614):
/// `Set.toList <$> Map.fromListWith (<>) [...]` — ascending lists.
fn predicates_by_ref(
    schemas: &[&ProcessedSchema],
) -> BTreeMap<PredicateRef, Vec<PredicateId>> {
    let mut m: BTreeMap<PredicateRef, BTreeSet<PredicateId>> = BTreeMap::new();
    for processed in schemas {
        for (id, def) in &processed.proc_schema_hashed.hashed_preds {
            if can_be_stored(&def.predicate_def_deriving) {
                m.entry(id.predicate_id_ref.clone())
                    .or_default()
                    .insert(id.clone());
            }
        }
    }
    m.into_iter()
        .map(|(k, v)| (k, v.into_iter().collect()))
        .collect()
}

/// 'predicatesBySchemaRef' (:616-628): `Map.fromListWith (<>) [...]` — the
/// fold evolve.rs pre-announced (SIBLING-UNIT CONTRACTS), via
/// VisiblePredicates::append. The union is field-wise HashSet union —
/// membership-commutative, so the fold order is unobservable.
fn predicates_by_schema_ref(
    schemas: &[&ProcessedSchema],
) -> BTreeMap<SchemaRef, VisiblePredicates<PredicateId>> {
    let mut m: BTreeMap<SchemaRef, VisiblePredicates<PredicateId>> = BTreeMap::new();
    for processed in schemas {
        // env (:623) / refToId (:624-625): `HashMap.lookupDefault (error
        // "predicatesBySchemaRef") ref env`. The shipped map_visible takes
        // its mapper over OWNED elements (`F: Fn(P) -> Q`, evolve.rs:231-234
        // — the shape of HashSet.map), so the closure owns its PredicateRef
        // and the map key is cloned into the lookup (PredicateRef: Clone;
        // the Haskell shared it).
        let env = &processed.proc_schema_hashed.schema_ref_to_id_env.pred_ref_to_id;
        let ref_to_id = |r: PredicateRef| -> PredicateId {
            env.get(&r)
                .cloned()
                .unwrap_or_else(|| panic!("predicatesBySchemaRef"))
        };
        for resolved in &processed.proc_schema_resolved.schemas_resolved {
            let visible = map_visible(ref_to_id, visible_predicates(resolved));
            let entry = m
                .entry(schema_ref(resolved))
                .or_insert_with(Default::default);
            *entry = std::mem::take(entry).append(visible);
        }
    }
    m
}

// (SchemaRef is glean_angle::schema::types::SchemaRef — (Name, Version),
// Ord+Hash derived, schema/types.rs:73.)

/// 'Evolves relationships as per source code annotations. ... We only
/// consider evolution annotations in the current schema.' (:630-637)
fn direct_evolutions(
    schemas: &[&ProcessedSchema],
) -> Result<BTreeMap<SchemaRef, SchemaRef>, String> {
    // `concatMap (schemasResolved . procSchemaResolved) schemas` — the
    // resolved schemas are scattered across borrowed ProcessedSchemas, so
    // the flat slice the shipped direct_schema_evolutions takes is built
    // by clone (read-only consumers; the Haskell shares).
    let resolved: Vec<ResolvedSchemaRef> = schemas
        .iter()
        .flat_map(|s| s.proc_schema_resolved.schemas_resolved.iter().cloned())
        .collect();
    direct_schema_evolutions(&resolved)
}

/// 'Calculate predicate evolutions caused by version-less schema
/// migrations.' (:639-657): `Map.traverseMaybeWithKey choose byRef`.
fn calc_auto_evolutions(
    has_facts: &dyn Fn(&PredicateId) -> bool,
    by_ref: &BTreeMap<PredicateRef, Vec<PredicateId>>,
) -> Result<BTreeMap<PredicateRef, PredicateId>, String> {
    let mut out = BTreeMap::new();
    for (ref_, xs) in by_ref {
        let factful: Vec<&PredicateId> = xs.iter().filter(|x| has_facts(x)).collect();
        match factful.as_slice() {
            [] => {} // Right Nothing
            [y] => {
                out.insert(ref_.clone(), (*y).clone());
            }
            // Left $ Text.unwords [...]
            _ => {
                return Err(format!(
                    "db has facts for multiple instances of {} : {}",
                    ref_.show_ref(),
                    factful
                        .iter()
                        .map(|y| y.show_ref())
                        .collect::<Vec<_>>()
                        .join(" ")
                ))
            }
        }
    }
    Ok(out)
}

/// 'Calculate which schema should effectively evolve which by taking the
/// DB's content into account. ... If A.3 evolves A.2 which evolves A.1,
/// but we only have facts for predicates from A.3, then we end-up with
/// both A.1 and A.2 being directly evolved by A.3.' (:659-703)
fn calc_schema_evolutions(
    has_facts: &dyn Fn(&PredicateId) -> bool,
    by_schema_ref: &BTreeMap<SchemaRef, VisiblePredicates<PredicateId>>,
    direct: &BTreeMap<SchemaRef, SchemaRef>,
) -> BTreeMap<SchemaRef, SchemaRef> {
    // withFacts (:701-703): the schemas with ANY factful defined predicate.
    let with_facts: BTreeSet<&SchemaRef> = by_schema_ref
        .iter()
        .filter(|(_, visible)| visible.visible_defined.iter().any(|id| has_facts(id)))
        .map(|(sref, _)| sref)
        .collect();

    // reverseDirect (:694-696): Map.fromList (map swap ...) — later pairs
    // overwrite (last-wins), same as collect.
    let reverse_direct: BTreeMap<&SchemaRef, &SchemaRef> =
        direct.iter().map(|(k, v)| (v, k)).collect();

    let mut out = BTreeMap::new();
    for (ref_, _) in by_schema_ref {
        if let Some(target) = transformation_target(ref_, direct, &reverse_direct, &with_facts, has_facts) {
            out.insert(ref_.clone(), target);
        }
    }
    out
}

/// transformationTarget (:677-692): 'Pick closest related schema with facts
/// in the db.' — newer first, then older.
fn transformation_target(
    src: &SchemaRef,
    direct: &BTreeMap<SchemaRef, SchemaRef>,
    reverse_direct: &BTreeMap<&SchemaRef, &SchemaRef>,
    with_facts: &BTreeSet<&SchemaRef>,
    has_facts: &dyn Fn(&PredicateId) -> bool,
) -> Option<SchemaRef> {
    let has_facts_in_db = |sref: &SchemaRef| with_facts.contains(sref);
    if has_facts_in_db(src) {
        return None; // hasFactsInDb src = Nothing
    }
    // `find hasFactsInDb newerSchemas <|> find hasFactsInDb olderSchemas`
    close_find(|r| direct.get(r), src, has_facts_in_db)
        .or_else(|| close_find(|r| reverse_direct.get(r), src, has_facts_in_db))
}

/// `close next x` (:705-708) consumed by `find`: the first factful schema
/// along the chain (the chain STARTS at next x — x itself is not tested).
fn close_find<F, G>(next: F, start: &SchemaRef, pred: G) -> Option<SchemaRef>
where
    F: Fn(&SchemaRef) -> Option<&SchemaRef>,
    G: Fn(&SchemaRef) -> bool,
{
    let mut cur = next(start)?;
    loop {
        if pred(cur) {
            return Some(cur.clone());
        }
        cur = next(cur)?;
    }
}

/// 'Optimise derivation queries for fetching facts given the particular
/// db's content.' (:710-734) — the PRUNE CONSUMER OBLIGATION discharge
/// (SIBLING-UNIT CONTRACTS / prune.rs:110-124): hasFacts closes over pmap
/// in the source, so its answers are PRECOMPUTED over pmap's keys from
/// (pmap, transformations, stats) and a closure over that map is passed;
/// `&PredicateId` per the `filter_f` precedent.
fn prune(
    db_content: &DbContent,
    transformations: &HashMap<i64, i64>,
    pmap: HashMap<PredicateId, PredicateDetails>,
) -> HashMap<PredicateId, PredicateDetails> {
    match db_content {
        // 'don't change derivations while db is still writable' (:718)
        DbContent::DbWritable => pmap,
        DbContent::DbReadOnly(stats) => {
            // hasFacts (:723-728): 'A predicate has facts in the db if it
            // or the predicate it would be transformed into has facts in
            // the db.' Keys outside pmap answer False (fromMaybe False).
            let mut answers: HashMap<PredicateId, bool> = HashMap::with_capacity(pmap.len());
            for (id, details) in &pmap {
                let pid = details.predicate_pid;
                let available_pid = transformations.get(&pid.0).copied().unwrap_or(pid.0);
                let has = stats
                    .get(&Pid(available_pid))
                    .map(|s| s.count > 0)
                    .unwrap_or(false);
                answers.insert(id.clone(), has);
            }
            let has_facts = |id: &PredicateId| -> bool {
                answers.get(id).copied().unwrap_or(false)
            };
            // `pruneDerivations hasFacts pmap` (:719) — the map is CONSUMED
            // here (row 93's signature; the borrow-impossible capture is
            // the whole reason for the precomputed map).
            prune_derivations(has_facts, pmap)
        }
    }
}

/// 'Compare schemas for compatibility. ... This is used to determine
/// whether we can safely use schema B for a stacked DB when the base DB is
/// using schema A.' (Schema.hs:736-754). The execState fold is a plain
/// loop over the HashMap image; each detection's error PREPENDS
/// (`State.put (err <> errs, pids')`, :754), so the returned list is in
/// REVERSE detection order — Create.hs:289-290 renders it via
/// `Text.intercalate ", "` directly, the order is observable.
/// PredicateId's Eq is HASH-ONLY
/// (angle/types.rs:952-957, bug-for-bug): "has changed" fires only when
/// the hashes differ.
pub fn compare_schema_predicates(preds_a: &[PredicateId], preds_b: &[PredicateId]) -> Vec<String> {
    let mut errors: Vec<String> = Vec::new();
    let mut pids: HashMap<PredicateRef, PredicateId> = HashMap::new();
    for id in preds_a.iter().chain(preds_b.iter()) {
        let err = compare_insert("predicate", &id.predicate_id_ref, id, &mut pids);
        // err <> errs (:754) — newest-first.
        errors.splice(0..0, err);
    }
    errors
}

/// 'compareInsert' (:756-768) — generic (Hashable a, Eq a, Eq b, ShowRef a)
/// collapsed to the (PredicateRef, PredicateId) instantiation (its only
/// use :753); the map mutates in place (the State put's second component).
fn compare_insert(
    thing: &str,
    r#ref: &PredicateRef,
    id: &PredicateId,
    tids: &mut HashMap<PredicateRef, PredicateId>,
) -> Vec<String> {
    match tids.get(r#ref) {
        Some(id2) => {
            if *id == *id2 {
                Vec::new()
            } else {
                vec![format!("{} {} has changed", thing, r#ref.show_ref())]
            }
        }
        None => {
            tids.insert(r#ref.clone(), id.clone());
            Vec::new()
        }
    }
}

/// 'mkPredicateTransformation' (:770-789): Nothing when the ids are equal
/// (PredicateId Eq = HASH-ONLY, bug-for-bug — two different ids with the
/// same hash are "the same predicate" here exactly as in the source). The
/// accessor's error (the detailsById `error`) propagates.
fn mk_predicate_transformation<'d, F>(
    details_for: &F,
    requested_id: &PredicateId,
    available_id: &PredicateId,
) -> Result<Option<PredicateTransformation>, String>
where
    F: Fn(&PredicateId) -> Result<&'d PredicateDetails, String>,
{
    if requested_id == available_id {
        return Ok(None);
    }
    let requested = (details_for)(requested_id)?;
    let available = (details_for)(available_id)?;
    Ok(Some(PredicateTransformation {
        t_requested: pid_ref(requested),
        t_available: pid_ref(available),
        // tTransformFactBack = transformFact available requested (:781)
        t_transform_fact_back: transform_fact(available, requested),
        // transformKeyPattern = transformPattern (key requested) (key
        // available) (:782); transformValuePattern likewise (:783). The
        // closure shapes line up with row 89's erased
        // TransformPatternAction (SIBLING-UNIT CONTRACTS; PatTerm in
        // transform.rs is the Matcher target type). The source builds
        // these lazily (thunks); the port computes at construction —
        // pure computation, forcing order unobservable.
        transform_key_pattern: transform_pattern(
            requested.predicate_key_type.clone(),
            available.predicate_key_type.clone(),
        ),
        transform_value_pattern: transform_pattern(
            requested.predicate_value_type.clone(),
            available.predicate_value_type.clone(),
        ),
    }))
}

// Note [Negation in stored predicates]
//
// Facts derived based on the absence of other facts could be invalidated when new
// facts are added to an incremental database.
//
// The work required to identify which fact to invalidate is close to that of
// re-deriving those predicates all over again.
//
// To keep the overhead on incremental databases as low as possible, stored
// derived predicates are not allowed to use negation.
// (Schema.hs:791-801, kept verbatim.)

/// 'typecheckSchema' (:803-916): typecheck one hashed schema against the
/// accumulated environment, in place.
///
/// The LAZY KNOT (Schema.hs:813-833): the source builds a LAZY HashMap of
/// per-typeid RTS types that mutually recurse through mkRtsType's NamedTy
/// lookups; the port is a memoized recursion (MAP IMAGES — Lazy.HashMap).
/// The knot's nontermination on recursive typedefs is the source's own
/// documented limitation (Types.hs:321-322, transcribed by row 89): the
/// memo walk's recursion overflows in exactly that case.
///
/// THE TCENV KNOT — this function's tcDeriving loop is the site
/// typecheck.rs:488-503 names; see SHARING WALLS 3 / the TODO(port) below.
fn typecheck_schema(
    id_to_pid: &HashMap<PredicateId, Pid>,
    stored: bool,
    tc_opts: &TcOpts,
    hashed: &HashedSchema,
    tc_env: &mut TcEnv,
) -> Result<(), DbError> {
    // ---- the typedefs knot (:813-833). -----------------------------------
    // 'NOTE: We store Maybe Type rather than filtering out those typedefs
    // for which rtsType returns Nothing here because we need to be
    // sufficiently lazy to tie the knot in rtsType. We get Nothing for
    // typedefs which refer to predicates that are not in the database but
    // do exist in the 'Schemas' (cf. 'mkDbSchema') or, more specifically,
    // in `refToPid`, and then also for typedefs which refer to those
    // typedefs and so on.' (source comment, Schema.hs:814-821, kept.)
    //
    // The lazy HashMap becomes a memo (Rc<RefCell> so the ToRtsType
    // closure can share it — single-threaded construction, the image of
    // the source's lazy map). Each step borrows briefly, so the
    // mkRtsType recursion through NamedTy never holds a cell lock across
    // a recursive call; a typedef CYCLE recurses to a stack overflow —
    // the source's own documented limitation (Types.hs:321-322, kept by
    // row 89), the evolve.rs nontermination class.
    let memo: Rc<std::cell::RefCell<HashMap<TypeId, Option<Type>>>> =
        Rc::new(std::cell::RefCell::new(HashMap::new()));

    fn rts_type_of(
        memo: &Rc<std::cell::RefCell<HashMap<TypeId, Option<Type>>>>,
        hashed: &HashedSchema,
        id_to_pid: &HashMap<PredicateId, Pid>,
        id: &TypeId,
    ) -> Option<Type> {
        if let Some(ty) = memo.borrow().get(id) {
            return ty.clone();
        }
        let def = hashed.hashed_types.get(id)?;
        // rtsType (rmLocType (typeDefType def)) (:824)
        let ty = mk_rts_type(
            &|r: &TypeId| rts_type_of(memo, hashed, id_to_pid, r),
            &|p: &PredicateId| id_to_pid.get(p).copied(),
            &rm_loc_type(def.type_def_type.clone()),
        );
        memo.borrow_mut().insert(id.clone(), ty.clone());
        ty
    }

    // `rtsType = mkRtsType lookupType (\`HashMap.lookup\` idToPid)`
    // (:833) — the ToRtsType the typecheckDeriving calls consume. The
    // Box's elided 'static is satisfied by the Rc clone inside.
    let rts_type: ToRtsType = {
        let memo = Rc::clone(&memo);
        Box::new(move |ty: &Type_<(), PredicateId, TypeId>| {
            mk_rts_type(
                &|r: &TypeId| rts_type_of(&memo, hashed, id_to_pid, r),
                &|p: &PredicateId| id_to_pid.get(p).copied(),
                ty,
            )
        })
    };

    // ---- new defs (:836-839). --------------------------------------------
    // `new = HashMap.difference hashedPreds (tcEnvPredicates tcEnv)` (by
    // key); `new' = HashMap.map rmTypeLocPredDef new`.
    let new_defs: Vec<(PredicateId, PredicateDef_<SrcSpan, (), PredicateId, TypeId>)> = hashed
        .hashed_preds
        .iter()
        .filter(|(id, _)| !tc_env.tc_env_predicates.contains_key(*id))
        .map(|(id, def)| (id.clone(), rm_type_loc_pred_def(def.clone())))
        .collect();

    // ---- mkPredicateDetails (:840-861). ----------------------------------
    let mut new_details: Vec<PredicateDetails> = Vec::new();
    for (id, def) in &new_defs {
        let pid = id_to_pid.get(&def.predicate_def_ref).copied();
        let key_type = mk_rts_type(
            &|r: &TypeId| rts_type_of(&memo, hashed, id_to_pid, r),
            &|p: &PredicateId| id_to_pid.get(p).copied(),
            &def.predicate_def_key_type,
        );
        let value_type = mk_rts_type(
            &|r: &TypeId| rts_type_of(&memo, hashed, id_to_pid, r),
            &|p: &PredicateId| id_to_pid.get(p).copied(),
            &def.predicate_def_value_type,
        );
        if let (Some(pid), Some(key_type), Some(value_type)) = (pid, key_type, value_type) {
            // typecheck <- checkSignature keyType valueType (:846) —
            // BLOCKED row 24 (BLOCKED DEPENDENCIES).
            let typecheck = check_signature(&key_type, &value_type);
            // traversal <- genTraversal keyType valueType (:847) —
            // BLOCKED row 23.
            let traversal = gen_traversal(&key_type, &value_type);
            new_details.push(PredicateDetails {
                predicate_pid: pid,
                predicate_id: id.clone(),
                predicate_schema: def.clone(),
                predicate_key_type: key_type,
                predicate_value_type: value_type,
                predicate_typecheck: typecheck,
                predicate_traversal: traversal,
                predicate_deriving: DerivingInfo::NoDeriving,
                predicate_in_stored_schema: stored,
            });
        }
        // `_ -> return Nothing` (:859) — dropped (catMaybes).
    }

    // ---- the env (:863-875). ---------------------------------------------
    // types (:864-866): TypeDetails for every typedef that HAS an rts type
    // — the source forces the WHOLE lazy map here (`HashMap.toList
    // typedefs`).
    let types: HashMap<TypeId, TypeDetails> = hashed
        .hashed_types
        .keys()
        .filter_map(|id| {
            rts_type_of(&memo, hashed, id_to_pid, id).map(|ty| {
                (
                    id.clone(),
                    TypeDetails {
                        type_id: id.clone(),
                        type_type: ty,
                    },
                )
            })
        })
        .collect();
    // tcEnvTypes = HashMap.union types (tcEnvTypes tcEnv) (:874) —
    // LEFT-biased: the CURRENT schema's types WIN (insert-overwrite; an
    // or_insert here would keep the earlier schema's entry on shared
    // TypeIds — the wrong bias). TypeDetails is not Clone (no derive,
    // schema/types.rs:748-751), so the entries are reconstructed
    // field-wise (TypeId and Type are Clone) and the LOCAL `types` map
    // stays live for checkStoredType below.
    for (id, td) in &types {
        tc_env.tc_env_types.insert(
            id.clone(),
            TypeDetails {
                type_id: td.type_id.clone(),
                type_type: td.type_type.clone(),
            },
        );
    }

    // TODO(port): THE TCENV KNOT (SHARING WALLS 3) — typecheck.rs:488-503
    // queues the CENTRAL resolution ("Arc-shared TcEnv maps / Rc<DbSchema>
    // / a &-taking initialTypecheckState") and names this unit. The source
    // shares ONE immutable env across the per-predicate mapM
    // (Schema.hs:871-887): `env = TcEnv { tcEnvPredicates = foldr
    // insertPred (tcEnvPredicates tcEnv) newDetails, ... }` and each
    // tcDeriving call typechecks ITS details against that env, writing the
    // result back (`return details { predicateDeriving = drv }`). With the
    // shipped by-value typecheck_deriving the loop is inexpressible. The
    // draft below is written in the source's shape against the central
    // fix's call form:
    //
    //   let drv = typecheck_deriving(&env, tc_opts, &rts_type, details,
    //                                &deriving_info)?;
    //
    // (TcEnv by reference — the "Arc-shared TcEnv maps" option; ToRtsType
    // by reference; PredicateDetails by &mut with predicate_deriving
    // written back; deriving_info by reference.) The clones of
    // new_details into env are the SAME PredicateDetails: Clone seam as
    // SHARING WALLS 1. Until both land this is the Step-4 compile error,
    // never a silent divergence.
    for d in &new_details {
        // env-building (the foldr insertPred image, :873 — new details
        // visible to each other's derivations; keys unique, fold order
        // moot; the clone is the SHARING WALLS 1 seam).
        tc_env
            .tc_env_predicates
            .entry(d.predicate_id.clone())
            .or_insert_with(|| d.clone());
    }
    for d in new_details.iter_mut() {
        // tcDeriving (:878-887): the deriving info comes from the def the
        // source reads through the same details value.
        let deriving_info = d.predicate_schema.predicate_def_deriving.clone();
        // TODO(port): THE TCENV KNOT — see the block above; this call is
        // in the central fix's shape (typecheck.rs:488-503 names this
        // unit as a co-owner of that resolution).
        let drv = typecheck_deriving(tc_env, tc_opts, &rts_type, d, &deriving_info)
            .map_err(DbError::Msg)?;
        d.predicate_deriving = drv;
    }
    // the final insertPred of the mutated details back over the map (:888)
    // — replaces the env's clones with the derivation-carrying values.
    let new_ids: Vec<PredicateId> = new_details
        .iter()
        .map(|d| d.predicate_id.clone())
        .collect();
    for d in new_details {
        tc_env
            .tc_env_predicates
            .insert(d.predicate_id.clone(), d);
    }

    // ---- the stored-predicate check (:890-914). --------------------------
    // 'Check the invariant that stored predicates do not refer to derived
    // predicates. We have to defer this check until last, because
    // derivations may have been attached to existing predicates now.'
    // (source comment, kept.)
    for id in &new_ids {
        let d = tc_env
            .tc_env_predicates
            .get(id)
            .expect("typecheckSchema: the just-inserted detail is present");
        let check = || -> Result<(), DbError> {
            // checkStoredType finalPreds types ... (:896-897): the SECOND
            // argument is the per-schema LOCAL types map (:864-866), NOT
            // the accumulated union — a TypeId outside this schema's
            // typedefs must hit the `error "checkStoredType: ..."` panic
            // in the source, not recurse through an earlier schema's
            // entry.
            check_stored_type(
                &tc_env.tc_env_predicates,
                &types,
                &d.predicate_id,
                &d.predicate_key_type,
            )?;
            check_stored_type(
                &tc_env.tc_env_predicates,
                &types,
                &d.predicate_id,
                &d.predicate_value_type,
            )
        };
        match &d.predicate_deriving {
            DerivingInfo::NoDeriving => {
                // 'In principle we should really enforce that predicates
                // without a derivation don't refer to non-stored derived
                // predicates. However, this can crop up in benign ways
                // with backwards compatibility. ...' (:900-913 — the
                // source's comment and the S.2/S.1 example, kept in the
                // source; the port keeps the same arm shape.)
            }
            DerivingInfo::Derive(DeriveWhen::DerivedAndStored, _) => check()?,
            _ => {}
        }
    }

    Ok(())
}

/// 'Enable or disable a DeriveIfEmpty derivation based on the DB
/// contents.' (:919-936) — in-place on the pruned details.
fn enable_selective_derivations(db_content: &DbContent, details: &mut PredicateDetails) {
    let is_derive_if_empty = matches!(
        &details.predicate_deriving,
        DerivingInfo::Derive(DeriveWhen::DeriveIfEmpty, _)
    );
    if !is_derive_if_empty {
        return;
    }
    match db_content {
        DbContent::DbWritable => details.predicate_deriving = DerivingInfo::NoDeriving,
        DbContent::DbReadOnly(stats) => match stats.get(&details.predicate_pid) {
            // 'Nothing -> details' (:931) — keep the derivation.
            None => {}
            // 'Just stats | predicateStats_count stats == 0 -> details'
            Some(s) if s.count == 0 => {}
            Some(_) => details.predicate_deriving = DerivingInfo::NoDeriving,
        },
    }
}

/// 'usesOfNegation' (:939-959): which predicates' derivations use negation
/// (directly or through a dependency).
///
/// The source's `derivations = toPred . getNode <$> reverse (topSort graph)`
/// is the plain POSTORDER of a DFS from vertices in ascending-key order
/// (graphFromEdges sorts by key), which — edges pointing ref -> deps — is
/// DEPENDENCIES-FIRST (PROBE record 1, MAP IMAGES). The fold then sees
/// dependencies before dependents, which is what the firstJust-over-deps
/// propagation requires.
fn uses_of_negation(
    preds: &HashMap<PredicateId, PredicateDetails>,
) -> HashMap<PredicateId, UseOfNegation> {
    let mut edges = derivation_edges(preds);
    // graphFromEdges sorts by key: PredicateId's STRUCTURAL Ord
    // (angle/types.rs:920-930).
    edges.sort_by(|a, b| a.1.cmp(&b.1));

    // key index — graphFromEdges' key→vertex mapping is Ord-keyed (the
    // containers constraint is Ord key; the sort above already used the
    // same class), so the index is a BTreeMap over PredicateId's
    // STRUCTURAL Ord — not the hash-only Hash/Eq pair (which would merge
    // same-hash/different-ref ids into one vertex the source keeps
    // apart). Keys are unique (derivationEdges comes from a HashMap
    // keyed by PredicateId), so or_insert never fires twice.
    let mut index_of: BTreeMap<&PredicateId, usize> = BTreeMap::new();
    for (i, (_, id, _)) in edges.iter().enumerate() {
        index_of.entry(id).or_insert(i);
    }
    // adjacency: Set.toList (tcQueryDeps query) ascending (BTreeSet image);
    // edges to unknown keys SILENTLY DROPPED (PROBE record 2).
    let adj: Vec<Vec<usize>> = edges
        .iter()
        .map(|(_, _, deps)| {
            deps.iter()
                .filter_map(|d| index_of.get(d).copied())
                .collect()
        })
        .collect();

    // reverse (topSort) = postorder DFS from vertices ascending.
    let order = postorder(&adj);

    let mut uses_negation: HashMap<PredicateId, UseOfNegation> = HashMap::new();
    for i in order {
        let (query, id, deps) = &edges[i];
        // recordUseOfNegation (:945-953)
        if let Some(use_) = tc_query_uses_negation(query) {
            uses_negation.insert(id.clone(), use_);
        } else if let Some(use_) = deps.iter().find_map(|d| uses_negation.get(d)) {
            // firstJust (`HashMap.lookup` usesNegation) deps — the dep list
            // is ascending (Set.toList); UseOfNegation carries no payload,
            // so re-matching reproduces the shared value without a Clone.
            let use_ = match use_ {
                UseOfNegation::PatternNegation => UseOfNegation::PatternNegation,
                UseOfNegation::IfStatement => UseOfNegation::IfStatement,
            };
            uses_negation.insert(id.clone(), use_);
        } else {
            // 'remove from Map in case a derived predicate in this schema
            // that does not use negation overrode a derived predicate that
            // used negation.' (source comment, :951-953, kept.)
            uses_negation.remove(id);
        }
    }
    uses_negation
}

/// 'derivationEdges' (:961-968): `(query, ref, Set.toList (tcQueryDeps
/// query))` per predicate with a derivation. The HashMap.toList source
/// order is unordered; graphFromEdges re-sorts by key (usesOfNegation) and
/// derivationDepends is key-unique, so the port keeps the collection
/// unordered. Also the file's pub export (derivationEdges).
pub fn derivation_edges(
    preds: &HashMap<PredicateId, PredicateDetails>,
) -> Vec<(&TcQuery, PredicateId, Vec<PredicateId>)> {
    preds
        .iter()
        .filter_map(|(id, details)| match &details.predicate_deriving {
            DerivingInfo::Derive(_, qwi) => {
                let query = &qwi.qi_query;
                Some((query, id.clone(), tc_query_deps(query).into_iter().collect()))
            }
            DerivingInfo::NoDeriving => None,
        })
        .collect()
}

/// (derivationDepends image, :488-489 — keys unique, so a plain collect.)
fn derivation_edges_map(
    preds: &HashMap<PredicateId, PredicateDetails>,
) -> HashMap<PredicateId, Vec<PredicateId>> {
    derivation_edges(preds)
        .into_iter()
        .map(|(_, id, deps)| (id, deps))
        .collect()
}

/// The iterative postorder DFS (PROBE record 1): children before parents,
/// roots in index order, adjacency order preserved.
fn postorder(adj: &[Vec<usize>]) -> Vec<usize> {
    let n = adj.len();
    let mut visited = vec![false; n];
    let mut out = Vec::new();
    for start in 0..n {
        if visited[start] {
            continue;
        }
        visited[start] = true;
        let mut stack: Vec<(usize, usize)> = vec![(start, 0)];
        while let Some(frame) = stack.last_mut() {
            let v = frame.0;
            if frame.1 < adj[v].len() {
                let w = adj[v][frame.1];
                frame.1 += 1;
                if !visited[w] {
                    visited[w] = true;
                    stack.push((w, 0));
                }
            } else {
                out.push(v);
                stack.pop();
            }
        }
    }
    out
}

/// 'Check that the type of a stored predicate doesn't refer to any derived
/// predicates.' (:970-995).
fn check_stored_type(
    preds: &HashMap<PredicateId, PredicateDetails>,
    types: &HashMap<TypeId, TypeDetails>,
    def: &PredicateId,
    ty: &Type,
) -> Result<(), DbError> {
    match ty {
        Type_::ArrayTy(t) => check_stored_type(preds, types, def, t),
        Type_::MaybeTy(t) => check_stored_type(preds, types, def, t),
        Type_::RecordTy(fields) | Type_::SumTy(fields) => {
            for f in fields {
                check_stored_type(preds, types, def, &f.field_def_type)?;
            }
            Ok(())
        }
        Type_::PredicateTy(_, PidRef { predicate_id, .. }) => match preds.get(predicate_id) {
            Some(details) => match &details.predicate_deriving {
                DerivingInfo::Derive(DeriveWhen::DerivedAndStored, _) => Ok(()),
                DerivingInfo::Derive(_, _) => Err(DbError::Msg(format!(
                    // `show $ "stored predicate " <> ...` — show of a Text
                    // QUOTES the message (the source's own quirk, kept).
                    "\"stored predicate {} refers to non-stored derived predicate {}\"",
                    show_doc(&display_default(def)),
                    show_doc(&display_default(predicate_id))
                ))),
                _ => Ok(()),
            },
            None => panic!("checkStoredType: {}", predicate_id.show_ref()),
        },
        Type_::NamedTy(_, expanded) => match types.get(&expanded.expanded_type_ref) {
            Some(td) => check_stored_type(preds, types, def, &td.type_type),
            // `error $ "checkStoredType: " <> Text.unpack (showRef ref)`
            // (:994) — ref is the TypeId.
            None => panic!("checkStoredType: {}", expanded.expanded_type_ref.show_ref()),
        },
        _ => Ok(()),
    }
}

/// 'Check that a new schema can be processed and merged with the current
/// schema index. This is used to ensure that a running server will not
/// choke on a new schema before we deploy it.' (:998-1021).
pub fn validate_new_schema(
    _config: &ServerConfig,
    new_src: &[u8],
    current: &SchemaIndex,
    debug: &DebugFlags,
) -> Result<(), DbError> {
    let schema = match process_schema(None, new_src) {
        // 'Left msg -> throwIO $ Thrift.Exception $ Text.pack msg' (:1009)
        Err(msg) => return Err(DbError::Msg(msg)),
        Ok(resolved) => resolved,
    };
    let cur_db_schema = mk_db_schema(
        None,
        None,
        &read_write_content(),
        &schema,
        None,
        debug,
    )?;
    // void $ newMergedDbSchema Nothing (toStoredSchema curDbSchema) ...
    new_merged_db_schema(
        None,
        &to_stored_schema(&cur_db_schema),
        current,
        &read_write_content(),
        debug,
    )?;
    Ok(())
}

/// 'failOnLeft' (:1022-1025) — the Either Text channel onto the DbError
/// funnel.
fn fail_on_left<T>(r: Result<T, String>) -> Result<T, DbError> {
    r.map_err(DbError::Msg)
}

/// 'Check that the current schema in the SchemaIndex is compatible with
/// each of the older schema instances. This is the validity check when
/// adding a new schema instance.' (:1027-1072).
pub fn validate_new_schema_instance(index: &SchemaIndex) -> Result<(), DbError> {
    fail_on_left(validate_new_schema_instance_left(index))
}

fn validate_new_schema_instance_left(index: &SchemaIndex) -> Result<(), String> {
    let schemas = index_schemas(index);

    // 'Prevent recursive predicates. Because this is a constraint we will
    // remove in the future, for now we only check this when a new schema
    // instance is to be generated.' (source comment, :1032-1034, kept.)
    for resolved in &index.schema_index_current.proc_schema_resolved.schemas_resolved {
        check_recursive_definitions(resolved)?;
    }

    // auto = Map.mapMaybe listToMaybe $ predicatesByRef [curr] (:1037)
    let curr = &schemas[0];
    let by_ref_all = predicates_by_ref(&[curr]);
    let auto: BTreeMap<PredicateRef, PredicateId> = by_ref_all
        .into_iter()
        .filter_map(|(r, mut ids)| ids.drain(..).next().map(|id| (r, id)))
        .collect();

    let direct = direct_evolutions(&schemas)?;
    let by_pred_ref = predicates_by_ref(&schemas);
    let by_schema_ref = predicates_by_schema_ref(&schemas);
    let (types, preds) = definitions(&schemas);
    let evolutions = calc_evolutions(
        |id: &PredicateId| id.predicate_id_ref.clone(),
        &by_pred_ref,
        &by_schema_ref,
        &direct,
        &auto,
    )?;

    // performExhaustiveCheck = False (:1047-1061) — 'This is O(n^2), and
    // therefore off by default.' (kept as the source's constant.)
    let perform_exhaustive_check = false;
    if perform_exhaustive_check {
        validate_evolutions(None::<&dyn Fn(&PredicateId) -> bool>, &types, &preds, &transitive(&evolutions))
    } else {
        validate_evolutions(None::<&dyn Fn(&PredicateId) -> bool>, &types, &preds, &evolutions)
    }
}

/// 'add transitive relations explicitly to the map' (:1063-1072): the
/// closure of the evolution map.
fn transitive(m: &HashMap<PredicateId, PredicateId>) -> HashMap<PredicateId, PredicateId> {
    let mut out = HashMap::new();
    for key in m.keys() {
        // close (`HashMap.lookup` m) key — the whole chain
        let mut cur = key.clone();
        loop {
            match m.get(&cur) {
                Some(v) => {
                    out.insert(key.clone(), v.clone());
                    cur = v.clone();
                }
                None => break,
            }
        }
    }
    out
}

/// 'Detect recursion and co-recursion in predicate derivations. We don't
/// check default derived predicates because whether they will in fact
/// recurse or not depends on the db's content.' (:1074-1123).
fn check_recursive_definitions(resolved: &ResolvedSchemaRef) -> Result<(), String> {
    let recursive_definitions = cycles_of(resolved);
    if recursive_definitions.is_empty() {
        return Ok(());
    }
    // Text.unlines: every line ENDS with '\n' (including the last).
    let mut msg = String::from("found cycles in predicate derivations: \n");
    for cycle in &recursive_definitions {
        let mut parts: Vec<String> = cycle.iter().map(|r| r.show_ref()).collect();
        if let Some(first) = cycle.first() {
            parts.push(first.show_ref()); // (x:xs ++ [x])
        }
        msg.push_str(&parts.join(" -> "));
        msg.push('\n');
    }
    Err(msg)
}

/// 'derived predicate to derived predicate dependency map' (:1090-1095)
/// and the cycle finder (:1086-1088, :1113-1116): stronglyConnComp over
/// (node, node, children) with CyclicSCC extraction — the PROBE-VERIFIED
/// Kosaraju image (MAP IMAGES record 3): vertices ascending by key
/// (graphFromEdges sorts), pass1 postorder on the transpose, pass2 on the
/// original from the reversed postorder, members in pass-2 PRE-ORDER,
/// singleton cyclic iff self-edge.
fn cycles_of(resolved: &ResolvedSchemaRef) -> Vec<Vec<PredicateRef>> {
    // derivationQueries (:1097-1107): mapMaybe (toQuery . derivingDefDeriveInfo)
    let mut derivation_queries: HashMap<
        PredicateRef,
        SourceQuery_<SrcSpan, SrcSpan, PredicateRef, TypeRef>,
    > = HashMap::new();
    for (r, ddef) in &resolved.resolved_schema_deriving {
        match &ddef.deriving_def_derive_info {
            DerivingInfo::NoDeriving => {}
            DerivingInfo::Derive(when, query) => match when {
                DeriveWhen::DeriveIfEmpty => {}
                DeriveWhen::DeriveOnDemand | DeriveWhen::DerivedAndStored => {
                    derivation_queries.insert(r.clone(), query.clone());
                }
            },
        }
    }

    // deps (:1090-1095): fmap (filter hasDerivation . preds)
    // preds (:1110-1111): `HashSet.toList $ bifoldMap HashSet.singleton
    // references query` — the acc is a BTreeSet (the deterministic image;
    // MAP IMAGES note on the HashSet order).
    let mut deps: HashMap<PredicateRef, Vec<PredicateRef>> = HashMap::new();
    for (r, q) in &derivation_queries {
        let mut acc: BTreeSet<PredicateRef> = BTreeSet::new();
        q.bifold_map(
            &mut acc,
            &mut |p: &PredicateRef, acc: &mut BTreeSet<PredicateRef>| {
                acc.insert(p.clone());
            },
            &mut |t: &TypeRef, acc: &mut BTreeSet<PredicateRef>| references(resolved, t, acc),
        );
        deps.insert(
            r.clone(),
            acc.into_iter()
                .filter(|p| derivation_queries.contains_key(p))
                .collect(),
        );
    }

    // cycles roots children = [cycle | CyclicSCC cycle <- scc]
    cyclic_sccs(&deps)
}

/// 'predicates referenced by type' (:1118-1122): TypeDef lookup, then the
/// B6 full-tree bifold over the type's (PredicateRef, TypeRef) positions.
fn references(resolved: &ResolvedSchemaRef, tref: &TypeRef, acc: &mut BTreeSet<PredicateRef>) {
    if let Some(tdef) = resolved.resolved_schema_types.get(tref) {
        tdef.type_def_type.bifold_map(
            acc,
            &mut |p: &PredicateRef, acc: &mut BTreeSet<PredicateRef>| {
                acc.insert(p.clone());
            },
            &mut |t: &TypeRef, acc: &mut BTreeSet<PredicateRef>| references(resolved, t, acc),
        );
    }
}

/// The probe-verified stronglyConnComp CyclicSCC image (MAP IMAGES record
/// 3), over the (node = key) shape checkRecursiveDefinitions uses
/// (:1113-1116). Keys sort by PredicateRef's derived Ord (name, version —
/// the gen'd struct's field order).
fn cyclic_sccs(deps: &HashMap<PredicateRef, Vec<PredicateRef>>) -> Vec<Vec<PredicateRef>> {
    let mut keys: Vec<PredicateRef> = deps.keys().cloned().collect();
    keys.sort(); // graphFromEdges sorts by key

    let index_of: HashMap<&PredicateRef, usize> =
        keys.iter().enumerate().map(|(i, k)| (k, i)).collect();
    // adjacency: given order, unknown keys dropped
    let adj: Vec<Vec<usize>> = keys
        .iter()
        .map(|k| {
            deps.get(k)
                .map(|ds| {
                    ds.iter()
                        .filter_map(|d| index_of.get(d).copied())
                        .collect()
                })
                .unwrap_or_default()
        })
        .collect();
    // transpose adjacency
    let mut radj: Vec<Vec<usize>> = vec![Vec::new(); keys.len()];
    for (v, ds) in adj.iter().enumerate() {
        for &w in ds {
            radj[w].push(v);
        }
    }

    // pass1: postorder on the transpose, vertices ascending.
    let post1 = postorder(&radj);
    // pass2: DFS on the original from the REVERSED postorder; trees
    // flattened pre-order; singleton cyclic iff self-edge.
    let mut visited = vec![false; keys.len()];
    let mut out: Vec<Vec<PredicateRef>> = Vec::new();
    for &root in post1.iter().rev() {
        if visited[root] {
            continue;
        }
        // collect the tree in pre-order
        let mut tree: Vec<usize> = Vec::new();
        visited[root] = true;
        let mut stack: Vec<(usize, usize)> = vec![(root, 0)];
        tree.push(root);
        while let Some(frame) = stack.last_mut() {
            let v = frame.0;
            if frame.1 < adj[v].len() {
                let w = adj[v][frame.1];
                frame.1 += 1;
                if !visited[w] {
                    visited[w] = true;
                    tree.push(w);
                    stack.push((w, 0));
                }
            } else {
                stack.pop();
            }
        }
        let cyclic = tree.len() > 1 || adj[root].contains(&root);
        if cyclic {
            out.push(tree.into_iter().map(|i| keys[i].clone()).collect());
        }
    }
    out
}

/// 'definitions' (:1124-1137): the union of the schemas' hashed types and
/// preds. HashMap.unions is LEFT-biased (earlier schemas win — or_insert);
/// fromList is LAST-wins (insert).
fn definitions(
    schemas: &[&ProcessedSchema],
) -> (
    HashMap<TypeId, TypeDef_<SrcSpan, PredicateId, TypeId>>,
    HashMap<PredicateId, PredicateDef_<SrcSpan, SrcSpan, PredicateId, TypeId>>,
) {
    let mut types: HashMap<TypeId, TypeDef_<SrcSpan, PredicateId, TypeId>> = HashMap::new();
    let mut preds: HashMap<PredicateId, PredicateDef_<SrcSpan, SrcSpan, PredicateId, TypeId>> =
        HashMap::new();
    for processed in schemas {
        for (id, def) in &processed.proc_schema_hashed.hashed_types {
            types.entry(id.clone()).or_insert(def.clone());
        }
        for (id, def) in &processed.proc_schema_hashed.hashed_preds {
            preds.insert(id.clone(), def.clone());
        }
    }
    (types, preds)
}

/// 'Interrogate the schema associated with a DB' (:1140-1188).
pub fn get_schema_info(
    db_schema: &DbSchema,
    index: &SchemaIndex,
    get_schema_info: &GetSchemaInfo,
) -> Result<SchemaInfo, DbError> {
    // pids (:1144-1147): Map.fromList over the elements — keys unique in
    // practice (one Pid per stored predicate); last-wins either way.
    let pids: BTreeMap<i64, PredicateRef> = db_schema
        .predicates_by_id
        .values()
        .map(|p| (p.predicate_pid.0, predicate_ref(p)))
        .collect();

    let (source, source_ver, source_id) = &db_schema.schema_source;
    let stored_schema = render_schema_source(source);

    let schema_ids = to_stored_versions(&db_schema.schema_all_version, &db_schema.schema_id);
    let db_schema_ids = to_stored_versions(source_ver, source_id);
    let other_schema_ids: Vec<BTreeMap<String, i32>> = index
        .schema_index_older
        .iter()
        .filter(|p| &p.proc_schema_hashed.hashed_schema_id != source_id)
        .map(|p| {
            to_stored_versions(
                &p.proc_schema_hashed.hashed_schema_all_version,
                &p.proc_schema_hashed.hashed_schema_id,
            )
        })
        .collect();

    // fromPredicateId (:1162-1163): `predicatesById dbSchema HashMap.!` —
    // the partial !, RULE §2: same panic, message pinned from
    // unordered-containers-0.2.20 (the evolve.rs:699-705 precedent).
    let from_predicate_id = |pred_id: &PredicateId| -> i64 {
        db_schema
            .predicates_by_id
            .get(pred_id)
            .map(|d| d.predicate_pid.0)
            .unwrap_or_else(|| panic!("Data.HashMap.Internal.(!): key not found"))
    };

    // derivationDependencies (:1165-1168)
    let derivation_dependencies: BTreeMap<i64, Vec<i64>> = db_schema
        .derivation_depends
        .iter()
        .map(|(p, pp)| {
            (
                from_predicate_id(p),
                pp.iter().map(&from_predicate_id).collect(),
            )
        })
        .collect();

    // source (:1170-1177)
    let source_bytes: Vec<u8> = if get_schema_info.omit_source {
        Vec::new()
    } else {
        match &get_schema_info.select {
            SelectSchema::Stored => stored_schema.clone(),
            SelectSchema::Current => {
                find_schema_source(index, db_schema, &db_schema.schema_id)?
            }
            SelectSchema::SchemaId(sid) => find_schema_source(index, db_schema, sid)?,
            _ => stored_schema.clone(), // SelectSchema_EMPTY
        }
    };

    Ok(SchemaInfo {
        schema: source_bytes,
        predicate_ids: pids,
        schema_ids,
        db_schema_ids,
        other_schema_ids,
        derivation_dependencies,
        auth_status: None,
        auth_message: None,
    })
}

/// 'getSchemaInfoForSchema' (:1190-1201).
pub fn get_schema_info_for_schema(
    index: &SchemaIndex,
    sid: &SchemaId,
    debug: &DebugFlags,
) -> Result<SchemaInfo, DbError> {
    // 'maybe (throwIO $ userError "schema id not found") return ...'
    let processed: &ProcessedSchema = match find_schema_in_index(index, sid) {
        Some(p) => p,
        None => return Err(DbError::Msg("schema id not found".to_string())),
    };
    let db_schema = mk_db_schema(
        None,
        None,
        &read_write_content(),
        processed,
        None,
        debug,
    )?;
    let schema_source = render_schema_source(&processed.proc_schema_source);

    let mut res = get_schema_info(
        &db_schema,
        index,
        &GetSchemaInfo {
            select: SelectSchema::SchemaId(sid.clone()),
            omit_source: true,
        },
    )?;
    res.schema = schema_source;
    Ok(res)
}

/// 'The version map in the 'StoredSchema', the 'SchemaIndex' and the
/// 'SchemaInfo' is a @Map Text Version@ whereas the one we use internally
/// is the inverse of this: @IntMap SchemaId@. These two functions convert
/// back and forth.
///
/// TODO: maybe they should be the same.' (Schema.hs:1204-1209 — the source
/// TODO kept verbatim; a bare source TODO, not a port marker, per the row-89
/// FIDELITY NOTES adjudication.)
///
/// 'toStoredVersions' (:1211-1212).
pub fn to_stored_versions(ver: &i32, id: &SchemaId) -> BTreeMap<String, i32> {
    let mut m = BTreeMap::new();
    m.insert(id.clone(), *ver);
    m
}

/// 'fromStoredVersions' (:1214-1216): the ordered IntMap image is a
/// BTreeMap keyed by the version (MAP IMAGES — this site's consumer reads
/// toList ascending).
fn from_stored_versions(versions: &BTreeMap<String, i32>) -> BTreeMap<i64, SchemaId> {
    versions
        .iter()
        .map(|(id, ver)| (*ver as i64, id.clone()))
        .collect()
}

/// 'renderSchemaSource' (:1218-1220): `Text.encodeUtf8 $ renderStrict $
/// layoutCompact $ displayDefault parsed`. RENDER/LAYOUT PARITY — see the
/// TODO(port) at the layout_compact/render_strict import: the Wadler fits
/// renderer is display.rs's queued obligation (it names THIS site at
/// :537-542); the call is written in that module's shape.
pub fn render_schema_source(parsed: &SourceSchemas) -> Vec<u8> {
    let doc = display_default(parsed);
    render_strict(&layout_compact(&doc)).into_bytes()
}

/// 'Try to find the source for the desired schema. It could be in our
/// SchemaIndex, or it could be the schema stored in the DB if we're
/// looking at an old DB.' (:1222-1236).
fn find_schema_source(
    index: &SchemaIndex,
    db_schema: &DbSchema,
    sid: &SchemaId,
) -> Result<Vec<u8>, DbError> {
    let in_db = find_schema_in_db(db_schema, sid);
    let in_index = find_schema_in_index(index, sid).map(|p| &p.proc_schema_source);
    // `inDB <|> fmap procSchemaSource inIndex` (:1233)
    match in_db.or(in_index) {
        Some(schema) => Ok(render_schema_source(schema)),
        // 'throwIO $ Thrift.Exception $ "SchemaId not found: " <>
        // Thrift.unSchemaId sid' (:1235-1236)
        None => Err(DbError::Msg(format!("SchemaId not found: {}", sid))),
    }
}

/// 'findSchemaInDB' (:1238-1245): the single schemaSource triple whose id
/// matches.
fn find_schema_in_db<'a>(
    db_schema: &'a DbSchema,
    sid: &SchemaId,
) -> Option<&'a SourceSchemas> {
    let (source, _, source_id) = &db_schema.schema_source;
    if sid == source_id {
        Some(source)
    } else {
        None
    }
}

/// 'findSchemaInIndex' (:1247-1258): the first of current:older whose
/// hashed id matches.
fn find_schema_in_index<'a>(index: &'a SchemaIndex, sid: &SchemaId) -> Option<&'a ProcessedSchema> {
    index_schemas(index)
        .into_iter()
        .find(|proc| sid == &proc.proc_schema_hashed.hashed_schema_id)
}

// ---------------------------------------------------------------------------
// `show (displayDefault x)` — the error-text renderer. The canonical
// renderer is UNOWNED (display.rs:537-542); the evolve.rs:1173 precedent
// is a private flat-branch stand-in whose error text matches the
// old binary whenever the flat form fits 80 columns. This file's ONE use
// is the mkTransformations detailsById error (:575) — same class.
// ---------------------------------------------------------------------------

// TODO(port): show_doc render parity — the flat-branch stand-in until
// display.rs's unowned render surface (its TODO(port) #2) lands; the
// evolve.rs:771 marker-note class, one per file.

fn show_doc(doc: &Doc) -> String {
    fn go(doc: &Doc, out: &mut String) {
        match doc {
            Doc::Empty | Doc::Fail => {}
            Doc::Text(s) => out.push_str(s),
            Doc::Line => out.push('\n'),
            Doc::Cat(a, b) => {
                go(a, out);
                go(b, out);
            }
            Doc::Nest(_, x) => go(x, out), // indentation is inert on the flat branch
            Doc::Column(f) => {
                // current column = chars since the last newline
                let col = out
                    .chars()
                    .rev()
                    .take_while(|&c| c != '\n')
                    .count();
                go(&f(col), out);
            }
            Doc::Nesting(f) => go(&f(0), out), // flat branch nests at level 0
            Doc::Group(flat, _) => go(flat, out),
        }
    }
    let mut out = String::new();
    go(doc, &mut out);
    out
}

// ---------------------------------------------------------------------------
// Marker breakdown (RULE §6: reviewers check the trailer against this)
// ---------------------------------------------------------------------------
//
// Open port todos: 9
//   1. render_schema_source's layout_compact/render_strict import (:592)
//      — the display.rs-owned render parity (its second open todo names
//      Schema.hs:1220); Step-4 compile error until the render owner lands.
//   2. inventory's Subroutine clone (:832) — SHARING WALLS 1: impl Clone
//      for Subroutine<S> (row 18, over the already clone-shared
//      ForeignPtr) and for QueryWithInfo/TypecheckedQuery (rows 87/88).
//   3. with_db_schema_cache's write-starved cache (:966) — SHARING WALLS
//      2: the insert-and-return needs an Arc<DbSchema> cache payload (a
//      row-89 alias change) or an Inventory sharing seam (row 25).
//   4. mk_db_schema's byPid clone (:1057) — SHARING WALLS 1 again.
//   5. build_db_schema's addedPids sort (:1177) — the sorted image vs
//      hashable's reproducible keys order (Pids observable cross-binary
//      via getSchemaInfo.predicateIds and derived-fact Pids); owner: the
//      Step-6 judge's cross-binary comparison / the unordered-container
//      convention review.
//   6. build_db_schema's byPid clone (:1337) — SHARING WALLS 1 again.
//   7. typecheck_schema's TCENV KNOT block (:1968) — the central
//      resolution queued at typecheck.rs:488-503 (Arc-shared TcEnv maps /
//      &-taking initialTypecheckState), which names this unit.
//   8. the tcDeriving call itself (:2002) — the same central fix; the
//      call is written in the post-fix shape.
//   9. show_doc render parity (:2739) — the evolve.rs marker-note class.
// (One root cause behind 2/4/6/7/8: PredicateDetails cannot be Clone
// today — see SHARING WALLS.)
//
// Performance deferrals: 1 — the write-starved cache's rebuild cost
// (:980, inside SHARING WALLS 2's todo: the fast version is the Arc
// payload).
//
// Reproduced defects: 0. (The header's nontermination mention cites
// evolve.rs:716-722's own marker for the class the typedefs memo
// inherits; no new source defect is reproduced by this file beyond the
// transcribed message/quirk sites documented in place.)
//
// Inventory bug candidates: 0 (no row governs this file).
// Safety notes: 0 (no unsafe code).

// PORT STATUS: confidence=medium todos=9
