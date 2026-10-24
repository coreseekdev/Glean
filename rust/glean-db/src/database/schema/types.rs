// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// Ported from: glean/db/Glean/Database/Schema/Types.hs (367 lines).
// Manifest unit: rust/glean-db/src/database/schema/types.rs
// (migration/manifest.tsv line 89, crate glean-db). RULE §4: one source
// file -> one target file; the manifest path is authoritative. Module
// wiring: database/schema.rs's header pinned this file ("ROW 89'S WIRING
// CONTRACT"); this unit appended 'pub mod types;' there (the file existed
// as row 66's scaffolding).
//
// WHAT THIS FILE IS: the DB's schema handle. `DbSchema` is the record a
// database holds once its schema is loaded: the id-keyed predicate/type
// detail maps (including each predicate's compiled typecheck/traversal
// subroutines), the per-SchemaId name environments, the pid-keyed map, the
// per-client-schema-id query transformations, the RTS inventory, and the
// "all" schema version/id/source triple. Around the record: the lookup
// family (by SourceRef/PredicateId/Pid/TypeId), the SchemaSelector
// ("latest" vs a specific schema id), the temp-predicate plumbing
// (tempPid/addTmpPredicate — the "_tmp_" predicate used by query
// compilation, Angle/Types.hs:597-598), the transitive-dependency walk
// used by query transformation inference, the Schema.Type' -> RTS Type
// conversion (mkRtsType), and mkQueryTransformations — the strongly-
// connected-components pass that propagates "this predicate's query needs
// transforming" through the predicate dependency graph.
//
// ---------------------------------------------------------------------------
// INVENTORY (RULE §5; migration/inventory*.tsv)
// ---------------------------------------------------------------------------
//
// ZERO rows govern this file. Exact command at the repo root:
//
//   $ grep -n 'Database/Schema/Types' migration/inventory*.tsv
//
// returns nothing (also checked: 'Schema/Types.hs' in inventory.tsv — zero;
// 'DbSchema' — inventory.tsv:23/inventory.stm.tsv:21 (a third SUBSTRING match, inventory.tsv:128 'loadDbSchema', is the LocalOrRemote row — not this module), which are
// Database/Env.hs rows naming envDbSchemaCache, an Env field, not this
// module). Nothing here is an inventory-owned class: no FFI, no STM, no
// unsafePerformIO, no lazy IO. The nearest rows are the SIBLING rows cited
// below (inventory.tsv:27 for ComputeIds; the Subroutine fields ride
// Foreign/Bytecode's row 63 via the shipped Subroutine type).
//
// No INVENTORY-BUG-CANDIDATE applies: there is no row to diverge from.
//
// ---------------------------------------------------------------------------
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, checked not assumed)
// ---------------------------------------------------------------------------
//
//   * OUTBOUND (16), edges.tsv:388-403 — 14 shipped, 2 blocked markers:
//       :388 Angle/Hash.hs          -> glean_angle::angle::hash::Hash
//       :389 Angle/Types.hs         -> glean_angle::angle::types::{
//          DerivingInfo, PredicateDef_, PredicateId, SourceRef, SrcSpan,
//          SourceSchemas, TypeId, Type_, temp_predicate_id, Version}
//          (Version is angle/types.rs:104's re-export of glean_thrift's)
//       :390 Angle/Schema/Types.hs  -> glean_angle::schema::types::{
//          lookup_result_to_either, resolve_ref, LookupResult, NameEnv,
//          RefTarget}
//       :391 Angle/Schema/Util.hs   -> glean_angle::schema::util::ShowRef
//       :392 Bytecode/Types.hs      -> glean_bytecode::bytecode::types::{
//          DataPtr, Register}
//       :393 Database/Schema/ComputeIds.hs -> crate::database::schema::
//          compute_ids::RefTargetId (re-exported below: the source's export
//          list includes RefTargetId, which it gets from ComputeIds —
//          Types.hs:13/:62)
//       :394 Codegen/QueryRegs.hs   -> crate::query::codegen::query_regs::
//          QueryRegs
//       :395 Codegen/Types.hs       -> crate::query::codegen::types::{
//          Match, Output, TransformAndBind}
//       :396 Typecheck/Types.hs     -> crate::query::typecheck::types::
//          TypecheckedQuery
//       :397 RTS/Bytecode/Code.hs   -> glean_core::rts::bytecode::code::Code
//       :398 RTS/Foreign/Bytecode.hs-> glean_core::rts::foreign::bytecode::
//          Subroutine
//       :399 RTS/Foreign/Inventory.hs -> glean_core::rts::foreign::inventory::
//          {CompiledTypecheck, CompiledTraversal, Inventory}
//       :400 RTS/Term.hs            -> glean_core::rts::term::Term
//       :401 RTS/Traverse.hs        -> NOT SHIPPED (manifest row 23, the
//          traversal compiler). This unit needs only its phantom KIND TAG
//          `data CompiledTraversal` (Traverse.hs:25); the shipped stand-in
//          is inventory.rs:259's `pub struct CompiledTraversal` (its own
//          TODO moves it to rts/traverse.rs when row 23 lands — the import
//          below tracks it).
//       :402 RTS/Typecheck.hs       -> NOT SHIPPED (manifest row 24, the
//          typechecking compiler); tag stand-in inventory.rs:258's
//          `pub struct CompiledTypecheck` (Typecheck.hs:26).
//       :403 RTS/Types.hs           -> glean_core::rts::types::{
//          ExpandedType, FieldDef, Pid, PidRef, Type}
//   * INBOUND (24), all future units: edges.tsv:201 (CompletePredicates),
//     :248 (Create), :325 (Open), :372 (Schema), :452 (Database/Types),
//     :500 (Write/Queue), :532 (Codegen), :560 (Derive), :573 (Expand),
//     :582 (Flatten), :594 (Flatten/Types), :615 (Prune), :622 (Reorder),
//     :632 (Transform), :643/:654/:669 (Typecheck{,/Monad,/Unify}),
//     :685 (UserQuery), :720 (Write/JSON), :1558 (schema/gen Main),
//     :1631 (Shell), :1691/:1703/:1760 (tools). They consume the pub items
//     of the EXPORT MAPPING below; see SIBLING-UNIT CONTRACTS for the
//     verified call sites.
//   * The generated Glean.Types has no depmap row (generated code, RULE §4)
//     — an ASSUMED IMPORT: see that block.
//
// ---------------------------------------------------------------------------
// ASSUMED IMPORTS (RULE §4 B4)
// ---------------------------------------------------------------------------
//
//   * glean_thrift::{Fact, SchemaId}. Pinned shapes (glean/if/glean.thrift):
//         typedef i32 Version;                    // :39 (imported via the
//            angle/types.rs:104 re-export, the compute_ids.rs:238-241 form)
//         typedef string SchemaId (hs.newtype);   // :91 -> Rust String
//         struct Fact { .. }                      // the thrift fact record
//     The sibling compute_ids.rs:81-89 recorded the same three (its
//     `SchemaId is String` disposition is followed: empty_hashed_schema
//     builds `SchemaId = String::new()`); Fact is the generated record —
//     typed/build.rs:163 already consumes `glean_thrift::Fact` as an
//     assumed import. Fact appears exactly once here, as the argument and
//     result type of `tTransformFactBack :: Thrift.Fact -> Thrift.Fact`
//     (Types.hs:126), consumed at Transform.hs:157 `tTransformFactBack
//     fact` (by-value in, by-value out).
//
// ---------------------------------------------------------------------------
// SIBLING-UNIT CONTRACTS
// ---------------------------------------------------------------------------
//
//   * ComputeIds (row 66, SHIPPED compute_ids.rs): `pub type RefTargetId =
//     RefTarget<PredicateId, TypeId>` (:250). Re-exported below per the
//     source's export list.
//   * Codegen/QueryRegs (row 86, SHIPPED query_regs.rs): the record's
//     header DESIGN pins THIS unit — "row 89 ... takes it as the first
//     argument of its optional `Maybe (QueryRegs -> …)` transformation
//     fields (Types.hs:132/:138), i.e. `Maybe` of a function TAKING the
//     record". Honored: both erased closures take `&QueryRegs` first.
//   * Codegen/Types (row 87, SHIPPED): `Match<Ext, V>` (:378),
//     `Output = Typed<Register<BinaryOutputPtr>>` (:301),
//     `TransformAndBind(pub Type, pub Output)` (:565). The matcher type is
//     `Term<Match<TransformAndBind, Output>>` — the source's
//     `Term (Match TransformAndBind Output)`.
//   * Typecheck/Types (row 88, SHIPPED): `pub type TypecheckedQuery =
//     QueryWithInfo<TcQuery>` (:587). NOT Clone (its header says so,
//     :101-102) — which is why PredicateDetails derives nothing.
//   * Foreign/Bytecode (row 18, SHIPPED): `pub struct Subroutine<S>` with
//     the phantom-kind marker (bytecode.rs:259). The marker types are the
//     shipped `pub struct CompiledTypecheck;` / `pub struct
//     CompiledTraversal;` (inventory.rs:258-259) — the exact tags
//     CompiledPredicate's fields use (inventory.rs:244-246).
//   * Foreign/Inventory (row 25, SHIPPED): `pub struct Inventory` over the
//     rts::Inventory handle (inventory.rs:300). Not Clone (raw pointer +
//     Drop) — which is why PredicateDetails is not Clone and the lookup
//     family returns borrows.
//   * RTS/Types (row 11, SHIPPED): `Pid(pub i64)` (Copy+Ord+Hash,
//     types.rs:120-122; `.0` IS `fromPid`, Types.hs:51), `PidRef { pid,
//     predicate_id }` (:185), `ExpandedType { expanded_type_ref,
//     expanded_type_type }` (:229), `type Type = Type_<(), PidRef,
//     ExpandedType>` (:246), `type FieldDef = FieldDef_<(), PidRef,
//     ExpandedType>` (:250).
//   * Angle/Types (row 4, SHIPPED): `Type_`'s `bifold_map` walk
//     (types.rs:1493 — the B6 full-tree walk predicateDeps composes),
//     `temp_predicate_id()` (:1010 = Types.hs:597-598, PredicateRef
//     "_tmp_" 0 with hash0), `PredicateDef_<S, ST, P, T>` (:901), `SrcSpan`
//     (:401), `SourceSchemas = SourceSchemas_<SrcSpan>` (:1107),
//     `DerivingInfo<Q>` (:1039).
//   * Schema/Types (row 6, SHIPPED): `NameEnv<T> = HashMap<SourceRef,
//     BTreeSet<T>>` (:198), `RefTarget<P, T>` (:107), `LookupResult<T>`
//     (:230), `resolve_ref` (:274), `lookup_result_to_either` (:245).
//   * Schema/Util (row 5, SHIPPED): `ShowRef` for SourceRef/PredicateRef/
//     PredicateId (util.rs:273-303) — lookup_predicate_source_ref's error
//     text renders through it, byte-for-byte the source's.
//   * Code (row 22, SHIPPED): `Code` combinators are `&mut self` methods
//     (code.rs:129-138 DESIGN (1), struct Code :256); `Code a` in a source signature is "an `A`
//     returned by a Code method, sequenced by `&mut`" — the image used for
//     the transformation callbacks below.
//   * QueryRegs (row 86, SHIPPED): the erased transformation closures take
//     `&QueryRegs` (its DESIGN names row 89 as the consumer that passes
//     the record; Transform.hs:371 threads it as a plain argument).
//   * Database/Schema.hs (row 98, future) CONSTRUCTS the transformation
//     records: `transformKeyPattern = transformPattern (key requested)
//     (key available)` (Schema.hs:782-783); Query/Transform.hs (row 95,
//     future) builds the rank-2 actions (Transform.hs:342-357).
//   * Codegen.hs (row 110, future) CONSUMES: `lookupTransformation pid
//     qtrans` (Codegen.hs:628), the partial applications
//     `(transformKeyPattern =<< mtrans) <*> Just syscalls` (:633/:635,
//     :1098/:1099) and `tAvailable` (derived below). Transform.hs (row 95,
//     future) consumes `tTransformFactBack fact` (:157) and `tAvailable`
//     (:294).
//
// ---------------------------------------------------------------------------
// EXPORT MAPPING (source export list Types.hs:10-38, all 29 names)
// ---------------------------------------------------------------------------
//
//   DbSchema(..)              -> pub struct DbSchema (all 11 fields pub)
//   DbSchemaCache             -> pub type DbSchemaCache
//   DbSchemaCacheKey(..)      -> pub struct DbSchemaCacheKey (pub field)
//   RefTargetId               -> pub use compute_ids::RefTargetId (the
//                                source re-exports ComputeIds' synonym,
//                                Types.hs:13/:62)
//   PredicateDetails(..)      -> pub struct PredicateDetails (9 fields pub)
//   predicateRef              -> pub fn predicate_ref
//   PredicateTransformation(..)-> pub struct PredicateTransformation
//                                (5 fields pub; the two rank-2 fields are
//                                the erased-callback pair + generic method,
//                                see RANK-2 IMAGE)
//   Bytes(..)                 -> pub struct Bytes (2 fields pub)
//   IsPointQuery              -> pub type IsPointQuery
//   SchemaSelector(..)        -> pub enum SchemaSelector
//   schemaNameEnv             -> pub fn schema_name_env
//   allSchemaVersion          -> pub fn all_schema_version
//   addTmpPredicate           -> pub fn add_tmp_predicate
//   lookupSourceRef           -> pub fn lookup_source_ref
//   lookupPredicateSourceRef  -> pub fn lookup_predicate_source_ref
//   lookupPredicateId         -> pub fn lookup_predicate_id
//   lookupPid                 -> pub fn lookup_pid
//   lookupTransformation      -> pub fn lookup_transformation
//   needsTransformation       -> pub fn needs_transformation
//   QueryTransformations      -> pub struct QueryTransformations (the
//                                newtype constructor is exported WITHOUT
//                                (..) in the source, Types.hs:29 — the
//                                inner map field is PRIVATE; consumers go
//                                through needs/lookup_transformation)
//   mkQueryTransformations    -> pub fn mk_query_transformations
//   TypeDetails(..)           -> pub struct TypeDetails (2 fields pub)
//   lookupTypeId              -> pub fn lookup_type_id
//   dbSchemaRtsType           -> pub fn db_schema_rts_type
//   mkRtsType                 -> pub fn mk_rts_type
//   tempPid                   -> pub fn temp_pid
//   pidRef                    -> pub fn pid_ref
//   transitiveDeps            -> pub fn transitive_deps
//
// Private, as in the source: the `transitive` worker (generic over Ord a
// in the source — collapsed to the Pid instantiation, its only use:
// `transitiveDeps detailsFor = transitive (predicateDeps . detailsFor)`
// makes the parameter unobservable, same collapse logic as compute_ids.rs
// COLLAPSE), `predicateDeps`/`typeDeps` (not exported; only transitiveDeps
// and mkQueryTransformations — both this module — use it), and
// `fieldType`/`rtsType` (mkRtsType's where-block).
//
// ---------------------------------------------------------------------------
// MAP IMAGES (RULE §2; every Data.X container of the source, with the
// precedent that pins it)
// ---------------------------------------------------------------------------
//
//   HashMap PredicateId PredicateDetails   -> std HashMap  (acl_config.rs:113
//     precedent; key std-Hash: angle/types.rs:945)
//   HashMap PredicateId [PredicateId]      -> HashMap<PredicateId,
//     Vec<PredicateId>>
//   HashMap TypeId TypeDetails             -> std HashMap (angle/types.rs:994)
//   Map SchemaId (NameEnv RefTargetId)     -> BTreeMap<SchemaId, _> —
//     Data.Map is ORDERED; SchemaId = String is Ord (compute_ids.rs:840
//     used BTreeMap for a Data.Map keyed by Version); only lookup is done
//     (schemaNameEnv), so ordering is unobservable, BTreeMap keeps it
//     available anyway
//   IntMap PredicateDetails                -> HashMap<i64, _> keyed by the
//     fromIntegral pid — the pinned IntMap image (code.rs:272 "IntMap ↦
//     HashMap keyed by the wrapped i64"; query/thrift/internal.rs:241
//     "Data.IntMap -> `HashMap<i64, _>`"). intPid's `fromIntegral ::
//     Int64 -> Int` is lossless on the pinned 64-bit GHC (Int = 64 bit),
//     so the key IS pid.0. lookupPid/needsTransformation/
//     lookupTransformation key by pid.0 directly.
//   IntMap TransDetails (inside the QueryTransformations newtype) -> same
//   HashMap SchemaId QueryTransformations  -> std HashMap (SchemaId is
//     Hash-able String)
//   HashMap DbSchemaCacheKey DbSchema      -> std HashMap (see DERIVING)
//   Set Pid (transitiveDeps' visited)      -> BTreeSet<Pid> — Data.Set is
//     Ord-based; Pid has Ord (rts/types.rs:120); membership + insert only,
//     so the ordering choice is unobservable, BTreeSet mirrors the source's
//     Set
//   Data.Graph.stronglyConnComp            -> strongly_conn_comp (Tarjan)
//     IN THIS FILE — no shipped SCC exists; see SCC ORDERING for the
//     probe-verified ordering contract
//
// ---------------------------------------------------------------------------
// RANK-2 IMAGE (transformKeyPattern / transformValuePattern, Types.hs:131-147)
// ---------------------------------------------------------------------------
//
// The source field is
//
//   transformKeyPattern :: forall a
//     . Maybe ( QueryRegs
//             -> (Type -> Term (Match TransformAndBind Output) -> Code ())
//             -> Term (Match TransformAndBind Output)
//             -> (Term (Match TransformAndBind Output) -> Code a)
//             -> Code a)
//
// — a rank-2 function (the `a` of the continuation is chosen by the
// CALLER's do-block, Codegen.hs:637-643 binds `a <- transKeyPat … $ \kpat'
// -> …`). A Rust closure cannot be generic over the continuation's result,
// so the field stores the ERASED form and a GENERIC METHOD of the same
// name carries the forall — the established house image (config.rs
// DIVERGENCES 1: "a boxed erased-callback closure and adds a GENERIC
// METHOD of the same name"; the with_value precedent thrift_source.rs:
// 158-215, whose erased field is
// `Arc<dyn Fn(&C, &mut dyn FnMut(&Observed<A>)) -> Result<(), ConfigError>>`
// and whose generic `with_value<B, F: FnOnce(&Observed<A>) -> B>` recovers
// B through an Option out-slot inside the `&mut dyn FnMut` adapter).
// Applied here:
//
//   * Maybe -> Option<Box<dyn Fn(..)>>; Box (not Arc/Rc): the record owns
//     its actions exactly as the Haskell record owns its closures.
//   * The erased continuation `(Term Matcher -> Code a)` becomes
//     `&mut dyn FnMut(&Term Matcher, &mut Code)` — the `a`-result is
//     erased to `()` exactly as with_value erased the callback's `IO b`
//     result; the generic method's adapter closure writes the caller's
//     `A` into an `Option<A>` out-slot. The `Code` context is threaded as
//     the LAST argument — the established closure convention
//     (query_regs.rs DESIGN: "&mut Code LAST is the established in-tree
//     closure convention"; the source's arguments keep their order).
//   * The emit callback `(Type -> Term Matcher -> Code ())` needs no
//     erasure (its result is `()`): `&mut dyn FnMut(&Type, &Term Matcher,
//     &mut Code)`. FnMut, not Fn: the source's actions are first-class
//     functions applied repeatedly (the `discard` callback runs per record
//     field, Transform.hs:384-390).
//   * The generic method's `k: FnMut(&Term Matcher, &mut Code) -> A` is
//     called through the adapter, which keeps calling it on EVERY
//     invocation (Code side effects always run) and overwrites the
//     out-slot — the source's `runCont` pipeline calls the continuation
//     exactly once (Transform.hs:361), so the out-slot holds its result;
//     overwriting (rather than with_value's take-once) keeps multiple
//     invocations well-defined instead of silently dropping effects. The
//     exactly-once expectation is stated in the method's expect text, the
//     with_value form.
//   * No Send/Sync bound on the boxed closures: the source record has no
//     such constraint (QueryRegs.rs made the same call for its 19 emitter
//     fields).
//
// ---------------------------------------------------------------------------
// SCC ORDERING (mkQueryTransformations, Types.hs:337-366) — probe-verified
// ---------------------------------------------------------------------------
//
// The source is `foldr addSCC mempty (reverse $ Graph.stronglyConnComp
// edges)`. Semantics pinned by probe (pinned GHC 9.4.7, scratch
// /tmp/scc_probe, programs probe.hs/probe2.hs/probe3.hs, 2026-10-24):
//
//   1. stronglyConnComp output is DEPENDENCIES-FIRST: edges
//      [("a","a",["b"]), ("b","b",[])] (a depends on b, a listed first)
//      yield [AcyclicSCC "b", AcyclicSCC "a"]; the chain 4→3→2→1 yields
//      [1,2,3,4] regardless of listing order (probes 1-3). This is the
//      documented contract ("a component C is only ever mentioned after
//      all components that C depends on appear earlier in the list").
//   2. Edges to keys outside the vertex list are SILENTLY IGNORED (probe
//      4: stronglyConnComp [("a","a",["zzz"])] evaluates without error) —
//      the compute_ids.rs FIDELITY NOTE adjudication, re-verified here.
//   3. The reversed foldr: Data.IntMap.insert forces the accumulator's
//      spine, so the components are DECIDED+INSERTED in stronglyConnComp's
//      OUTPUT order, and component i's `defaultForSCC` sees exactly the
//      entries of components 1..i-1 (probe2: per-component snapshots
//      pid4->[1,2,3], pid3->[1,2], pid2->[1], pid1->[]).
//   4. Exact-replica probe3 (the source's addSCC verbatim over IntMap):
//      chain 4→3→2→1 with a transformation only on pid 1 gives
//      {1: HasTransformation, 2: Dependencies, 3: Dependencies,
//      4: Dependencies}; with it only on pid 2 gives {2: Has, 3: Deps,
//      4: Deps} and pid 1 gets NO entry; the cycle 5<->6 with a
//      transformation only on 6 gives {6: Has} and NO entry for 5 —
//      within-SCC dependencies do not count (the decision reads the
//      pre-component accumulator).
//
// The port therefore: Tarjan over the dep edges (adjacency = the source's
// predicateDeps lists, roots iterated in edge-list order = ascending pid,
// as Data.Graph's vertex order is the edge-list order), emitting each
// SCC when its root closes — children (dependencies) emitted first, which
// IS the dependencies-first order (probe 1 semantics reproduced: on a→b,
// Tarjan emits [b] then [a]); then ONE pass over the components in
// emission order deciding members against the map accumulated so far.
// Unobservable details, deliberately not reproduced: (a) intra-SCC member
// order (GHC's flattenSCC pre-order vs Tarjan's stack-pop order) — every
// member of one SCC is decided against the same pre-component accumulator
// and inserted under a distinct key (IntMap keys are unique per SCC), so
// the final map is identical; (b) order among INDEPENDENT components — a
// component's decision only reads pids it depends on, which always live
// in its dependency components. (Both are the compute_ids.rs adjudication,
// re-derived for this fold.) The Acyclic/Cyclic distinction is dropped:
// addSCC iterates `scc` uniformly (Types.hs:349), so the SCC tag never
// reaches the result — only the partition and the order do.
//
// ---------------------------------------------------------------------------
// DERIVING PARITY (every Haskell deriving clause has a disposition)
// ---------------------------------------------------------------------------
//
//   * DbSchemaCacheKey derives (Eq, Hashable) — Types.hs:108. Hash is the
//     GHC-fingerprint newtype (hash.rs:90 `pub type Hash = Fingerprint`),
//     and hash.rs:82-87's amendment-3 std Hash impl hashes the two words.
//     The port derives PartialEq/Eq/Hash on the newtype: derive(Hash)
//     delegates to Fingerprint's std Hash — the same two-word hash the
//     source's `Hashable Hash` instance computes. (The hash_fingerprint
//     placeholder port-todo marker at angle/types.rs:940 is about the hashable
//     salt CHAIN for PredicateId/TypeId keys; DbSchemaCacheKey hashes the
//     Fingerprint value directly and inherits hash.rs's real impl, not
//     that placeholder.)
//   * Bytes derives Show — Types.hs:158 -> derive(Debug) (RULE §2
//     deriving-Show row). No consumer in the tree renders Bytes with show
//     (grep: its only consumers pattern-match `Bytes start end`,
//     Transform.hs:386, Codegen.hs:59 import); Debug keeps the record
//     shape for diagnostics.
//   * DbSchema, DbSchemaCache, PredicateDetails, PredicateTransformation,
//     TypeDetails, TransDetails, SchemaSelector: NO deriving clause in the
//     source — no derives here. Checked against the tree: no Eq/Ord use of
//     PredicateDetails or SchemaSelector anywhere (grep for `==`, `/=`,
//     `compare` over their consumers: zero). PredicateDetails CANNOT
//     derive Clone anyway (Subroutine's Inventory handle and
//     DerivingInfo<TypecheckedQuery> are not Clone) — matching the
//     source's no-instance. TypecheckedQuery not Clone is the sibling's
//     own pinned disposition (typecheck/types.rs:101-102).
//   * QueryTransformations: newtype, no deriving -> no derives.
//
// ---------------------------------------------------------------------------
// FIDELITY NOTES (the rest of the one-decision-per-construct ledger)
// ---------------------------------------------------------------------------
//
//   * Ownership/laziness: the Haskell record fields are shared thunks; the
//     port's lookup family returns BORROWS (Option<&PredicateDetails>,
//     Option<&TypeDetails>, Option<&NameEnv<..>>, Option<&SchemaId>,
//     Option<&PredicateTransformation>, Result<&PredicateDetails, String>)
//     — the image of Haskell sharing. PredicateDetails/PredicateTransformation
//     are not Clone (closures + FFI handles), so nothing here returns them
//     by value.
//   * mkQueryTransformations takes `tmap` BY VALUE and moves the
//     transformations into the result (TransDetails OWNS its
//     HasTransformation payload; the source shares the thunk). tmap is
//     dead after the call — its only use is `transFor`'s lookup feeding
//     the result (Types.hs:344) — so consuming it is the by-value image.
//     `pmap` is only read (lookups + `!`), so it borrows. `pmap
//     IntMap.! intPid pid` (Types.hs:345) is total on this port's inputs:
//     detailsFor is applied only to pids that come from pmap's own keys
//     (edges) or to members of components built from those keys — a
//     missing key is impossible, so the source's partial `!` needs no
//     panic arm (the membership is structural).
//   * IntMap.keys is ASCENDING (Data.IntMap.Int64 keys, ascending numeric);
//     mkQueryTransformations builds its edge list from it, and the
//     component order (and with it the fold's decision order) depends on
//     that vertex order. The HashMap image iterates in unspecified order,
//     so mk_query_transformations SORTS the pids ascending before building
//     edges — reproducing the source's vertex order, not diverging.
//   * `transitiveDeps`: the generic `transitive :: Ord a => (a -> [a]) ->
//     Set a -> a -> [a]` becomes the Pid-monomorphic loop (collapse
//     rationale above). `next x <> xs` puts the deps of x at the FRONT of
//     the worklist (DFS, first-visit order is the output order) — a
//     VecDeque pop_front + push_front-over-reversed-deps reproduces it
//     exactly. The `initial` Set is a pure input used as the first
//     `visited` (the source cannot mutate it) — the port clones it into
//     the working set.
//   * `predicateDeps`: `bifoldr' overPidRef overExpanded` over the RTS
//     Type. The shipped Type_ walk is `bifold_map` (angle/types.rs:1493),
//     whose doc already adjudicates the equivalence ("The Haskell instance
//     is `bifoldr`; `bifoldMap` over it still yields elements in
//     left-to-right order (foldr preserves element order in the Monoid
//     chain)"). The source's value-type-first-then-key CONS is reproduced
//     as key-deps-then-value-deps appended to one Vec:
//     `typeDeps key (typeDeps value [])` = key-pids ++ value-pids (each
//     cons chain puts earlier elements at the head), so the port pushes
//     key deps first, then value deps. overPidRef collects `pid`
//     (PidRef's first field); overExpanded recurses through
//     `expanded_type_type` — the B6 full-tree walk (nested
//     Array/Record/Sum/Set/Maybe/HasTy/HasKey/ElementsOf predicates all
//     evolve — angle/types.rs:1501-1514).
//   * `tempPid = succ . schemaMaxPid`: Pid derives Enum in the source
//     (Types.hs:52) — succ is the Int64 successor; the port is
//     `Pid(db_schema.schema_max_pid.0 + 1)` (RULE §2 numeric row: explicit,
//     no overflow check — the source has none).
//   * `lookupPid`'s IntMap key `fromIntegral (fromPid pid)`: Int is 64-bit
//     on the pinned GHC, so the conversion is the identity and the port
//     keys by pid.0 (the code.rs:272 image, restated).
//   * dbSchemaRtsType's lookupPid fallback: `_ | tempPredicateId == ref ->
//     Just $ tempPid dbSchema` (Types.hs:298) — PredicateId's Eq compares
//     ONLY the hash (angle/types.rs:953, reproduced bug-for-bug by the
//     sibling), so the port's `temp_predicate_id() == *ref_` is likewise a
//     hash-only comparison: any predicate id with hash0 matches. The
//     source's own comment ("detect temporary predicates that might have
//     been serialized / see Glean.Query.Flatten.captureKey") is kept.
//   * mkRtsType returns Option<Type> where the source returns Maybe — the
//     four `error` arms panic with the SAME messages (RULE §2
//     partial-functions row): "rtsType: TyVar" / "rtsType: HasTy" /
//     "rtsType: HasKey" / "rtsType: ElementsOf". They are unreachable for
//     schema-derived types (TyVar/HasTy/HasKey/ElementsOf exist only
//     during typechecking), exactly as in the source.
//   * mkRtsType's lookupType/lookupPid are first-class function arguments
//     in the source; the port keeps them first-class as `&dyn Fn` params
//     (no monomorphization is needed — one caller shape, dbSchemaRtsType).
//     The RTS `Type` it builds is owned: the NamedTy expansion CLONES the
//     looked-up `typeType` (the Haskell shares the thunk; Rust's
//     ExpandedType owns its Type).
//   * The source's own TODO at Types.hs:321-322 ("This will loop if we
//     have recursive typedefs but we don't allow those at the moment") is
//     transcribed verbatim at the NamedTy arm below. Adjudication: a bare
//     source TODO describing the source's own limitation — NOT a
//     port-todo marker (the port's recursion is the source's recursion;
//     no port work item is being deferred), so it is not counted in the
//     trailer.
//   * Pretty SchemaSelector -> glean_angle::display::Display (RULE §1
//     prettyprinting row; the Pid precedent rts/types.rs:127-133 folds
//     `instance Pretty` into the Doc-producing Display trait). `pretty id`
//     on the SchemaId's Text is `Doc::text` (display.rs's `impl Display
//     for String`), composed with `<>` -> `.append`.
//   * lookupPredicateSourceRef: `Either Text PredicateDetails` ->
//     Result<&PredicateDetails, String> (RULE §1 strings row for the Text
//     payload; the Right payload is shared in Haskell and unclonable
//     here — see DERIVING PARITY). The three error arms keep the source's
//     text byte-for-byte, rendered through ShowRef ("internal error: "
//     <> showRef pred <> " not found"; showRef ref <> " is a type, not a
//     predicate"; the Left passthrough is lookup_result_to_either's
//     "not in scope: …"/"… is ambiguous. …" text).
//   * `fromMaybe acc` in addSCC/add: a Nothing decision leaves the
//     accumulator untouched (no entry inserted) — probe3 case 2 confirms
//     (pid 1 absent from the result).
//   * No unsafe code.
//
// MARKERS: zero port todos, zero BUG(port), zero PERF(port), zero
// INVENTORY-BUG-CANDIDATE, zero SAFETY notes (no unsafe in this file). The
// source's bare TODO at Types.hs:321 is transcribed (not a marker class;
// see FIDELITY NOTES for the adjudication).

use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};

use glean_angle::angle::hash::Hash;
use glean_angle::angle::types::{
    DerivingInfo, FieldDef_, PredicateDef_, PredicateId, PredicateRef,
    SourceRef, SourceSchemas, SrcSpan, TypeId, Type_, Version,
};
use glean_angle::display::{Display, Doc, DisplayOpts};
use glean_angle::schema::types::{
    lookup_result_to_either, resolve_ref, LookupResult, NameEnv, RefTarget,
};
use glean_angle::schema::util::ShowRef;
use glean_bytecode::bytecode::types::{DataPtr, Register};
use glean_core::rts::bytecode::code::Code;
use glean_core::rts::foreign::bytecode::Subroutine;
use glean_core::rts::foreign::inventory::{CompiledTypecheck, CompiledTraversal, Inventory};
use glean_core::rts::term::Term;
use glean_core::rts::types::{ExpandedType, FieldDef, Pid, PidRef, Type};
use glean_thrift::{Fact, SchemaId};

use crate::query::codegen::query_regs::QueryRegs;
use crate::query::codegen::types::{Match, Output, TransformAndBind};
use crate::query::typecheck::types::TypecheckedQuery;

// The source re-exports ComputeIds' RefTargetId (Types.hs:13 in the export
// list, :62 the import) — the same-crate re-export is the faithful image
// (RULE §4 bars only CROSS-crate glob re-exports).
pub use crate::database::schema::compute_ids::RefTargetId;
// `tempPredicateId` (Types.hs:35, defined at Angle/Types.hs:597-598) is
// re-exported by the source module; its consumers (Flatten.hs:726,
// UserQuery.hs:752) import it through THIS module (edges 582/685).
pub use glean_angle::angle::types::temp_predicate_id;

/// `Term (Match TransformAndBind Output)` (Types.hs:133/:139) — the
/// pattern matcher the transformations rewrite. Rust-only private alias
/// (not a source export — Transform.hs keeps its own private `type
/// Matcher`); type aliases are transparent, so the pub signatures that
/// name it expose only the public underlying type.
type Matcher = Term<Match<TransformAndBind, Output>>;

/// The erased-callback image of the rank-2 `transformKeyPattern`/
/// `transformValuePattern` fields (Types.hs:131-147). See RANK-2 IMAGE in
/// the header: the `a` of the continuation is erased to the trailing
/// `&mut dyn FnMut` adapter's `()`; the generic methods of the same names
/// on `PredicateTransformation` carry the forall.
type TransformPatternAction = Box<
    dyn Fn(
        &QueryRegs,
        &mut dyn FnMut(&Type, &Matcher, &mut Code),
        &Matcher,
        &mut dyn FnMut(&Matcher, &mut Code),
        &mut Code,
    ),
>;

/// 'The Schema used by a DB' (Types.hs:75-102), field for field.
///
/// The HashMap/IntMap/Map images are recorded in the header's MAP IMAGES
/// block. All fields are exported by `DbSchema(..)`, hence pub.
pub struct DbSchema {
    pub predicates_by_id: HashMap<PredicateId, PredicateDetails>,
    pub derivation_depends: HashMap<PredicateId, Vec<PredicateId>>,
    pub types_by_id: HashMap<TypeId, TypeDetails>,

    /// for each SchemaId, the environment of types/predicates (source doc,
    /// Types.hs:82)
    pub schema_envs: BTreeMap<SchemaId, NameEnv<RefTargetId>>,

    pub predicates_by_pid: HashMap<i64, PredicateDetails>,

    /// Transformations are specific to the union of evolutions in the
    /// client and server schemas. Indexed by client schema id.
    /// (source doc, Types.hs:87-88)
    pub predicates_transformations: HashMap<SchemaId, QueryTransformations>,

    /// Information about the predicates in an open DB (source doc,
    /// Types.hs:91)
    pub schema_inventory: Inventory,

    pub schema_max_pid: Pid,

    // These two fields relate to the latest available schema. In a
    // writable DB this will be the DB schema, but in a readable DB it
    // will be the current schema in the schema index. (source doc,
    // Types.hs:94-96)
    pub schema_all_version: Version,
    pub schema_id: SchemaId,

    /// This is the DB schema, used by toStoredSchema (source doc,
    /// Types.hs:101)
    pub schema_source: (SourceSchemas, Version, SchemaId),
}

/// 'Cached DbSchemas, so that we can share them amongst different DBs'
/// (Types.hs:104-105).
pub type DbSchemaCache = HashMap<DbSchemaCacheKey, DbSchema>;

/// 'newtype DbSchemaCacheKey = DbSchemaCacheKey Hash deriving (Eq,
/// Hashable)' (Types.hs:107-108). derive(Hash) delegates to Hash's
/// (= Fingerprint's) std Hash — the two-word hash; see DERIVING PARITY.
/// Clone/Copy mirror the Haskell newtype's free copying (16 bytes).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct DbSchemaCacheKey(pub Hash);

/// 'Transformations to be applied to a query. Keyed by the requested
/// predicate (i.e. the one present in the query)' (Types.hs:110-112).
///
/// `newtype QueryTransformations = QueryTransformations (IntMap
/// TransDetails)` — the IntMap image is `HashMap<i64, _>` keyed by the pid
/// (header MAP IMAGES). The constructor is exported WITHOUT (..) in the
/// source (Types.hs:29), so the inner field is private; consumers use
/// [`needs_transformation`]/[`lookup_transformation`], producers use
/// [`mk_query_transformations`].
pub struct QueryTransformations(HashMap<i64, TransDetails>);

/// 'data TransDetails' (Types.hs:114-116).
// `data TransDetails` is NOT in the source's export list (Types.hs
// :114-116) — private, like Matcher/TransformPatternAction (all its
// consumers are in-module).
enum TransDetails {
    HasTransformation(PredicateTransformation),
    /// transitive deps (source comment, Types.hs:116)
    DependenciesHaveTransformations,
}

/// 'Data required to transform a predicate that was requested in a query
/// into one that we have available in the database, and then to transform
/// the facts found back into the requested predicate.' (source doc,
/// Types.hs:118-120).
///
/// The two rank-2 fields are the erased-callback field + generic method
/// pair (see RANK-2 IMAGE in the header; the config.rs DIVERGENCES 1 /
/// thrift_source.rs:158-215 precedent).
pub struct PredicateTransformation {
    /// the predicate that was queried for (source doc, Types.hs:122)
    pub t_requested: PidRef,

    /// the predicate we have in the database (source doc, Types.hs:124)
    pub t_available: PidRef,

    /// available -> requested
    /// transform a fact of the available predicate into a fact
    /// of the requested predicate. (source doc, Types.hs:126-129;
    /// consumed at Transform.hs:157)
    pub t_transform_fact_back: Box<dyn Fn(&Fact) -> Fact>,

    // requested -> available
    // Transform a pattern into one which can be used to build a prefix for
    // the available predicate. The resulting pattern cannot be used for
    // variable binding. (source doc, Types.hs:143-146)
    //
    /// 'transformKeyPattern :: forall a. Maybe (…)' (Types.hs:131-136).
    /// The erased form; the callable form is
    /// [`PredicateTransformation::transform_key_pattern`].
    pub transform_key_pattern: Option<TransformPatternAction>,

    /// 'transformValuePattern :: forall a. Maybe (…)' (Types.hs:137-142).
    /// The erased form; the callable form is
    /// [`PredicateTransformation::transform_value_pattern`].
    pub transform_value_pattern: Option<TransformPatternAction>,
}

impl PredicateTransformation {
    /// 'transformKeyPattern' (Types.hs:131-136), generic over the
    /// continuation's result — the forall-carrying half of the RANK-2
    /// IMAGE. Arguments in source order; the `Code` context last.
    pub fn transform_key_pattern<A>(
        &self,
        regs: &QueryRegs,
        emit: &mut dyn FnMut(&Type, &Matcher, &mut Code),
        term: &Matcher,
        k: impl FnMut(&Matcher, &mut Code) -> A,
        code: &mut Code,
    ) -> A {
        self.run_transform(&self.transform_key_pattern, regs, emit, term, k, code)
    }

    /// 'transformValuePattern' (Types.hs:137-142), same image.
    pub fn transform_value_pattern<A>(
        &self,
        regs: &QueryRegs,
        emit: &mut dyn FnMut(&Type, &Matcher, &mut Code),
        term: &Matcher,
        k: impl FnMut(&Matcher, &mut Code) -> A,
        code: &mut Code,
    ) -> A {
        self.run_transform(&self.transform_value_pattern, regs, emit, term, k, code)
    }

    /// The with_value adapter (thrift_source.rs:201-215): wrap the
    /// caller's continuation in an FnMut that writes its result into the
    /// out-slot, call the erased field, recover the result. Every adapter
    /// invocation RUNS the continuation (its Code effects always happen)
    /// and overwrites the out-slot; the source's runCont calls it exactly
    /// once (Transform.hs:361), which the expect below states.
    fn run_transform<A>(
        &self,
        action: &Option<TransformPatternAction>,
        regs: &QueryRegs,
        emit: &mut dyn FnMut(&Type, &Matcher, &mut Code),
        term: &Matcher,
        mut k: impl FnMut(&Matcher, &mut Code) -> A,
        code: &mut Code,
    ) -> A {
        let action = action
            .as_ref()
            .expect("PredicateTransformation: the transformation action is present");
        let mut out: Option<A> = None;
        (action)(
            regs,
            emit,
            term,
            &mut |t, code| {
                out = Some(k(t, code));
            },
            code,
        );
        out.expect(
            "PredicateTransformation::transform: the continuation runs exactly once",
        )
    }
}

/// 'Whether a prefix will match a fact from beginning to end and therefore
/// can only have a single result.' (source doc, Types.hs:149-151).
pub type IsPointQuery = bool;

/// 'bytes in a binary output' (source doc, Types.hs:153-158).
///
/// 'data Bytes = Bytes { b_start :: Register 'DataPtr, b_end :: Register
/// 'DataPtr } deriving (Show)' — Show -> Debug (DERIVING PARITY; Register
/// has a manual Debug that renders `Register(5)`, bytecode/types.rs:386).
#[derive(Debug)]
pub struct Bytes {
    pub b_start: Register<DataPtr>,
    pub b_end: Register<DataPtr>,
}

/// 'data TypeDetails' (Types.hs:160-163). No deriving clause in the source.
pub struct TypeDetails {
    pub type_id: TypeId,
    pub type_type: Type,
}

/// 'data PredicateDetails' (Types.hs:165-177), field for field. No
/// deriving clause in the source — and none is possible here (Subroutine
/// / DerivingInfo<TypecheckedQuery> are not Clone; see DERIVING PARITY).
pub struct PredicateDetails {
    pub predicate_pid: Pid,
    pub predicate_id: PredicateId,
    /// `PredicateDef' SrcSpan ()` = `PredicateDef_ SrcSpan () PredicateId
    /// TypeId` (Angle/Types.hs:696; the def carries its source span, the
    /// key/value types inside it are span-less).
    pub predicate_schema: PredicateDef_<SrcSpan, (), PredicateId, TypeId>,
    pub predicate_key_type: Type,
    pub predicate_value_type: Type,
    /// `Subroutine CompiledTypecheck` (Types.hs:171) — the shipped
    /// Subroutine<S> over the shipped kind tag (inventory.rs:244/:258).
    pub predicate_typecheck: Subroutine<CompiledTypecheck>,
    /// `Subroutine CompiledTraversal` (Types.hs:172) — inventory.rs:246/:259.
    pub predicate_traversal: Subroutine<CompiledTraversal>,
    pub predicate_deriving: DerivingInfo<TypecheckedQuery>,
    /// True if this prediate is part of the schema stored in the DB.
    /// Only predicates in the stored schema can be written. (source doc,
    /// Types.hs:174-176; "prediate" is the source's typo, kept)
    pub predicate_in_stored_schema: bool,
}

/// 'predicateRef = predicateIdRef . predicateId' (Types.hs:179-180).
pub fn predicate_ref(details: &PredicateDetails) -> PredicateRef {
    details.predicate_id.predicate_id_ref.clone()
}

/// 'data SchemaSelector' (Types.hs:182-184).
///
/// No deriving clause in the source (no Eq use in the tree — DERIVING
/// PARITY).
pub enum SchemaSelector {
    LatestSchema,
    SpecificSchemaId(SchemaId),
}

/// 'instance Pretty SchemaSelector' (Types.hs:186-188) — the
/// prettyprinter image (RULE §1; Pid precedent): `pretty LatestSchema =
/// "latest"`, `pretty (SpecificSchemaId (SchemaId id)) = "schema-id:" <>
/// pretty id`.
impl Display for SchemaSelector {
    fn display(&self, _opts: &DisplayOpts) -> Doc {
        match self {
            SchemaSelector::LatestSchema => Doc::text("latest"),
            // SchemaId is a Text newtype in the source; the port's String
            // renders through Doc::text (display.rs's `impl Display for
            // String`).
            SchemaSelector::SpecificSchemaId(id) => {
                Doc::text("schema-id:").append(Doc::text(id.clone()))
            }
        }
    }
}

/// 'allSchemaVersion' (Types.hs:190-192). The SchemaId is shared in the
/// source — returned by borrow.
pub fn all_schema_version<'a>(
    db_schema: &'a DbSchema,
    selector: &'a SchemaSelector,
) -> Option<&'a SchemaId> {
    match selector {
        SchemaSelector::SpecificSchemaId(v) => Some(v),
        SchemaSelector::LatestSchema => Some(&db_schema.schema_id),
    }
}

/// 'schemaNameEnv' (Types.hs:194-197).
pub fn schema_name_env<'a>(
    db_schema: &'a DbSchema,
    schema_ver: &'a SchemaSelector,
) -> Option<&'a NameEnv<RefTargetId>> {
    let hash = all_schema_version(db_schema, schema_ver)?;
    db_schema.schema_envs.get(hash)
}

/// 'addTmpPredicate' (Types.hs:199-204): insert the "_tmp_" predicate
/// under its versioned source ref.
pub fn add_tmp_predicate(mut env: NameEnv<RefTargetId>) -> NameEnv<RefTargetId> {
    // PredicateRef name ver = predicateIdRef tempPredicateId
    // sourceRef = SourceRef name (Just ver)   (Types.hs:203-204)
    let temp = temp_predicate_id();
    let PredicateRef { name, version } = temp.predicate_id_ref.clone();
    env.insert(
        SourceRef {
            source_ref_name: name,
            source_ref_version: Some(version),
        },
        BTreeSet::from([RefTarget::RefPred(temp)]),
    );
    env
}

/// 'lookupSourceRef' (Types.hs:206-215).
pub fn lookup_source_ref(
    ref_: &SourceRef,
    schema_ver: &SchemaSelector,
    db_schema: &DbSchema,
) -> LookupResult<RefTargetId> {
    match schema_name_env(db_schema, schema_ver) {
        None => LookupResult::OutOfScope,
        Some(env) => resolve_ref(env, ref_),
    }
}

/// 'lookupPredicateSourceRef' (Types.hs:217-231).
///
/// `Either Text PredicateDetails` -> `Result<&PredicateDetails, String>`
/// (RULE §1 strings row; the Right payload is a borrow — see FIDELITY
/// NOTES). Error text byte-for-byte through ShowRef.
pub fn lookup_predicate_source_ref<'a>(
    ref_: &SourceRef,
    schema_ver: &SchemaSelector,
    db_schema: &'a DbSchema,
) -> Result<&'a PredicateDetails, String> {
    match lookup_result_to_either(ref_, lookup_source_ref(ref_, schema_ver, db_schema)) {
        Ok(RefTarget::RefPred(pred)) => match lookup_predicate_id(&pred, db_schema) {
            Some(details) => Ok(details),
            None => Err(format!("internal error: {} not found", pred.show_ref())),
        },
        Ok(RefTarget::RefType(_)) => {
            Err(format!("{} is a type, not a predicate", ref_.show_ref()))
        }
        Err(err) => Err(err),
    }
}

/// 'lookupPredicateId' (Types.hs:233-234).
pub fn lookup_predicate_id<'a>(
    ref_: &PredicateId,
    db_schema: &'a DbSchema,
) -> Option<&'a PredicateDetails> {
    db_schema.predicates_by_id.get(ref_)
}

/// 'lookupPid (Pid pid) = IntMap.lookup (fromIntegral pid) .
/// predicatesByPid' (Types.hs:236-237) — the IntMap image keys by pid.0
/// (header MAP IMAGES; fromIntegral is the identity on the 64-bit GHC).
pub fn lookup_pid<'a>(pid: Pid, db_schema: &'a DbSchema) -> Option<&'a PredicateDetails> {
    db_schema.predicates_by_pid.get(&pid.0)
}

/// 'lookupTypeId' (Types.hs:239-240).
pub fn lookup_type_id<'a>(ref_: &TypeId, db_schema: &'a DbSchema) -> Option<&'a TypeDetails> {
    db_schema.types_by_id.get(ref_)
}

/// 'Either the type or a type transitively referenced by it needs to be
/// transformed.' (source doc, Types.hs:242-243).
///
/// 'needsTransformation (QueryTransformations tmap) pid = isJust $
/// IntMap.lookup (fromIntegral $ fromPid pid) tmap' (Types.hs:248-249).
pub fn needs_transformation(t: &QueryTransformations, pid: Pid) -> bool {
    t.0.contains_key(&pid.0)
}

/// 'lookupTransformation' (Types.hs:251-259). The Haskell returns the
/// shared transformation; the port borrows it.
pub fn lookup_transformation<'a>(
    pid: Pid,
    t: &'a QueryTransformations,
) -> Option<&'a PredicateTransformation> {
    match t.0.get(&pid.0) {
        Some(TransDetails::HasTransformation(trans)) => Some(trans),
        Some(TransDetails::DependenciesHaveTransformations) => None,
        None => None,
    }
}

/// 'transitiveDeps' (Types.hs:261-270) with its private `transitive`
/// worker collapsed to the Pid instantiation (header EXPORT MAPPING).
/// `next x <> xs` — the deps of x join the FRONT of the worklist; the
/// result is first-visit order. The `initial` set is the first `visited`
/// (pure input, cloned into the working set — FIDELITY NOTES). The
/// details accessor returns a borrow with its own lifetime 'd (the
/// Haskell's `Pid -> PredicateDetails` is a shared-value function; the
/// Rust image borrows from the accessor's captured environment).
pub fn transitive_deps<'d, F>(details_for: F, initial: &BTreeSet<Pid>, root: Pid) -> Vec<Pid>
where
    F: Fn(&Pid) -> &'d PredicateDetails,
{
    let mut visited: BTreeSet<Pid> = initial.clone();
    let mut work: VecDeque<Pid> = VecDeque::from([root]);
    let mut out: Vec<Pid> = Vec::new();
    while let Some(x) = work.pop_front() {
        if visited.contains(&x) {
            continue; // | x `Set.member` visited = go xs visited
        }
        // | otherwise = x : go (next x <> xs) (Set.insert x visited)
        visited.insert(x);
        out.push(x);
        let deps = predicate_deps(details_for(&x));
        for d in deps.iter().rev() {
            work.push_front(*d);
        }
    }
    out
}

/// 'All predicates mentioned in a predicate's type.
/// Does not include predicates from the derivation query.'
/// (source doc, Types.hs:272-281.)
///
/// The source folds value-type deps first and conses key-type deps onto
/// them: `typeDeps key (typeDeps value [])` = key-pids ++ value-pids. The
/// port pushes key deps first, then value deps — the same list (FIDELITY
/// NOTES). `bifoldr'` becomes the shipped `bifold_map` walk
/// (angle/types.rs:1493): overPidRef collects the PidRef's pid,
/// overExpanded recurses through the expansion — the B6 full-tree walk.
fn predicate_deps(details: &PredicateDetails) -> Vec<Pid> {
    let mut pids = Vec::new();
    type_deps(&details.predicate_key_type, &mut pids);
    type_deps(&details.predicate_value_type, &mut pids);
    pids
}

/// 'typeDeps ty r = bifoldr' overPidRef overExpanded r ty' (Types.hs:279).
fn type_deps(ty: &Type, out: &mut Vec<Pid>) {
    ty.bifold_map(
        out,
        // overPidRef (PidRef pid _) r = pid : r   (Types.hs:281)
        &mut |p: &PidRef, acc: &mut Vec<Pid>| acc.push(p.pid),
        // overExpanded (ExpandedType _ ty) r = typeDeps ty r (:280)
        &mut |e: &ExpandedType, acc: &mut Vec<Pid>| type_deps(&e.expanded_type_type, acc),
    );
}

/// 'tempPid = succ . schemaMaxPid' (Types.hs:283-284) — Pid's Enum
/// successor (Types.hs:52), the explicit i64 + 1 (FIDELITY NOTES).
pub fn temp_pid(db_schema: &DbSchema) -> Pid {
    Pid(db_schema.schema_max_pid.0 + 1)
}

/// 'pidRef details = PidRef (predicatePid details) (predicateId details)'
/// (Types.hs:286-287).
pub fn pid_ref(details: &PredicateDetails) -> PidRef {
    PidRef {
        pid: details.predicate_pid,
        predicate_id: details.predicate_id.clone(),
    }
}

/// 'Convert Schema types to RTS types using a DbSchema'
/// (Types.hs:289-299).
pub fn db_schema_rts_type<S>(
    db_schema: &DbSchema,
    ty: &Type_<S, PredicateId, TypeId>,
) -> Option<Type> {
    mk_rts_type(
        // lookupType ref = typeType <$> lookupTypeId ref dbSchema
        &|ref_: &TypeId| lookup_type_id(ref_, db_schema).map(|d| d.type_type.clone()),
        &|ref_: &PredicateId| {
            match lookup_predicate_id(ref_, db_schema) {
                Some(pid) => Some(pid.predicate_pid),
                // detect temporary predicates that might have been serialized
                // see Glean.Query.Flatten.captureKey (source comment,
                // Types.hs:296-297). The comparison is PredicateId Eq =
                // hash-only (FIDELITY NOTES).
                _ if temp_predicate_id() == *ref_ => Some(temp_pid(db_schema)),
                _ => None,
            }
        },
        ty,
    )
}

/// 'Convert from Schema Types to RTS Types. This involves
/// 1. PredicateRef -> PidRef
/// 2. Expand NamedTypes' (Types.hs:301-335).
///
/// The four `error` arms panic with the same messages (RULE §2
/// partial-functions row). `lookupType :: TypeId -> Maybe Type` and
/// `lookupPid :: PredicateId -> Maybe Pid` stay first-class `&dyn Fn`
/// arguments.
pub fn mk_rts_type<S>(
    lookup_type: &dyn Fn(&TypeId) -> Option<Type>,
    lookup_pid: &dyn Fn(&PredicateId) -> Option<Pid>,
    ty: &Type_<S, PredicateId, TypeId>,
) -> Option<Type> {
    match ty {
        Type_::ByteTy => Some(Type_::ByteTy),
        Type_::NatTy => Some(Type_::NatTy),
        Type_::ArrayTy(elty) => Some(Type_::ArrayTy(Box::new(mk_rts_type(
            lookup_type, lookup_pid, elty,
        )?))),
        Type_::RecordTy(fields) => Some(Type_::RecordTy(
            fields
                .iter()
                .map(|f| rts_field_type(lookup_type, lookup_pid, f))
                .collect::<Option<Vec<FieldDef>>>()?,
        )),
        Type_::SumTy(fields) => Some(Type_::SumTy(
            fields
                .iter()
                .map(|f| rts_field_type(lookup_type, lookup_pid, f))
                .collect::<Option<Vec<FieldDef>>>()?,
        )),
        Type_::SetTy(elty) => Some(Type_::SetTy(Box::new(mk_rts_type(
            lookup_type, lookup_pid, elty,
        )?))),
        Type_::StringTy => Some(Type_::StringTy),
        Type_::PredicateTy(_, ref_) => {
            let pid = lookup_pid(ref_)?;
            Some(Type_::PredicateTy(
                (),
                PidRef {
                    pid,
                    predicate_id: ref_.clone(),
                },
            ))
        }
        // TODO: This will loop if we have recursive typedefs but we don't allow
        // those at the moment. (source TODO, Types.hs:321-322 — transcribed
        // verbatim; bare source TODO, not a port marker: the port's recursion
        // is the source's recursion, FIDELITY NOTES)
        Type_::NamedTy(_, ref_) => {
            let ty = lookup_type(ref_)?;
            Some(Type_::NamedTy(
                (),
                ExpandedType {
                    expanded_type_ref: ref_.clone(),
                    expanded_type_type: Box::new(ty),
                },
            ))
        }
        Type_::MaybeTy(elt_ty) => Some(Type_::MaybeTy(Box::new(mk_rts_type(
            lookup_type, lookup_pid, elt_ty,
        )?))),
        Type_::EnumeratedTy(names) => Some(Type_::EnumeratedTy(names.clone())),
        Type_::BooleanTy => Some(Type_::BooleanTy),
        Type_::TyVar(_) => panic!("rtsType: TyVar"),
        Type_::HasTy(..) => panic!("rtsType: HasTy"),
        Type_::HasKey(..) => panic!("rtsType: HasKey"),
        Type_::ElementsOf(..) => panic!("rtsType: ElementsOf"),
    }
}

/// 'fieldType (Schema.FieldDef name ty) = Schema.FieldDef name <$>
/// rtsType ty' (Types.hs:334-335) — the RTS FieldDef's span is `()`, the
/// name is carried, the type converted.
fn rts_field_type<S>(
    lookup_type: &dyn Fn(&TypeId) -> Option<Type>,
    lookup_pid: &dyn Fn(&PredicateId) -> Option<Pid>,
    f: &FieldDef_<S, PredicateId, TypeId>,
) -> Option<FieldDef> {
    Some(FieldDef_ {
        field_def_name: f.field_def_name.clone(),
        field_def_type: mk_rts_type(lookup_type, lookup_pid, &f.field_def_type)?,
    })
}

/// 'mkQueryTransformations' (Types.hs:337-366).
///
/// `pmap` borrows (the source only reads it: `transFor`/`detailsFor`/
/// edges); `tmap` is consumed — its transformations MOVE into the result
/// (TransDetails owns its payload; tmap is dead after the call — FIDELITY
/// NOTES). The IntMap image is `HashMap<i64, _>` keyed by the pid; the
/// edge list is built over the pids in ASCENDING order (Data.IntMap.keys
/// is ascending and the component order feeds the fold — FIDELITY NOTES).
/// The SCC pass is the probe-verified contract of SCC ORDERING in the
/// header: components in dependencies-first emission order, each decided
/// against the map accumulated so far.
pub fn mk_query_transformations(
    pmap: &HashMap<i64, PredicateDetails>,
    tmap: HashMap<i64, PredicateTransformation>,
) -> QueryTransformations {
    let mut tmap = tmap;
    let mut result: HashMap<i64, TransDetails> = HashMap::new();

    // edges = [(pid, pid, deps pid) | pid <- Pid . fromIntegral <$>
    // IntMap.keys pmap]   (Types.hs:363-366)
    let mut pids: Vec<Pid> = pmap.keys().map(|&k| Pid(k)).collect();
    pids.sort_unstable(); // IntMap.keys ascending; keys are unique

    for comp in strongly_conn_comp(&pids, &|pid| predicate_deps(&pmap[&pid.0])) {
        // defaultForSCC (Types.hs:357-361): decided BEFORE any member of
        // this component is inserted — `any needsTransformation $
        // concatMap deps scc` over the PRE-component accumulator. The
        // decision is a Bool here; the TransDetails value is built per
        // member below (the variant carries no payload).
        let deps_have_transformations = comp
            .iter()
            .flat_map(|pid| predicate_deps(&pmap[&pid.0]))
            .any(|dep| result.contains_key(&dep.0));

        // addSCC scc depsTrans = foldr add depsTrans scc (Types.hs:349-355):
        // per member, transFor first, else the component default, else no
        // entry (fromMaybe acc). Member order within one SCC is
        // unobservable (header SCC ORDERING).
        for pid in comp {
            let trans = match tmap.remove(&pid.0) {
                Some(trans) => TransDetails::HasTransformation(trans),
                None => {
                    if deps_have_transformations {
                        TransDetails::DependenciesHaveTransformations
                    } else {
                        continue; // fromMaybe acc: keep the accumulator
                    }
                }
            };
            result.insert(pid.0, trans);
        }
    }

    QueryTransformations(result)
}

// ---------------------------------------------------------------------------
// Data.Graph.stronglyConnComp (base) -> Tarjan, specialized to the
// (pid, pid, deps) edge shape (node and key coincide, Types.hs:364)
// ---------------------------------------------------------------------------

/// The dependencies-first SCC decomposition of the pid graph. See SCC
/// ORDERING in the header for the probe evidence that this emission order
/// is the one the source's reversed foldr consumes, and for the two
/// deliberately-unobservable details (intra-SCC member order; order among
/// independent components). Nodes are the `pids` in edge-list order
/// (Data.Graph's vertex order); edges to pids outside the list are
/// silently ignored (probe 4; the compute_ids.rs:668-674 precedent).
fn strongly_conn_comp(pids: &[Pid], deps_of: &dyn Fn(&Pid) -> Vec<Pid>) -> Vec<Vec<Pid>> {
    let n = pids.len();

    // graphFromEdges: vertex index = edge-list position; unknown keys
    // dropped; duplicate keys cannot occur (the pids are map keys).
    let mut index_of: HashMap<i64, usize> = HashMap::with_capacity(n);
    for (i, pid) in pids.iter().enumerate() {
        index_of.entry(pid.0).or_insert(i);
    }
    let adj: Vec<Vec<usize>> = pids
        .iter()
        .map(|pid| {
            deps_of(pid)
                .iter()
                .filter_map(|d| index_of.get(&d.0).copied())
                .collect()
        })
        .collect();

    // Tarjan (iterative; roots in vertex order, children in adjacency
    // order — Data.Graph's DFS shape). An SCC is emitted when its root
    // closes; children complete first, so dependencies are emitted before
    // dependents (probe 1).
    let mut index = vec![usize::MAX; n];
    let mut low = vec![0usize; n];
    let mut on_stack = vec![false; n];
    let mut stack: Vec<usize> = Vec::new();
    let mut next_index = 0usize;
    let mut comps: Vec<Vec<Pid>> = Vec::new();

    struct Frame {
        v: usize,
        child: usize,
    }

    for start in 0..n {
        if index[start] != usize::MAX {
            continue;
        }
        index[start] = next_index;
        low[start] = next_index;
        next_index += 1;
        stack.push(start);
        on_stack[start] = true;

        let mut call = vec![Frame { v: start, child: 0 }];
        while let Some(frame) = call.last_mut() {
            let v = frame.v;
            if frame.child < adj[v].len() {
                let w = adj[v][frame.child];
                frame.child += 1;
                if index[w] == usize::MAX {
                    index[w] = next_index;
                    low[w] = next_index;
                    next_index += 1;
                    stack.push(w);
                    on_stack[w] = true;
                    call.push(Frame { v: w, child: 0 });
                } else if on_stack[w] {
                    low[v] = low[v].min(index[w]);
                }
            } else {
                call.pop();
                if let Some(parent) = call.last() {
                    low[parent.v] = low[parent.v].min(low[v]);
                }
                if low[v] == index[v] {
                    let mut comp = Vec::new();
                    loop {
                        let w = stack.pop().expect("SCC stack underflow");
                        on_stack[w] = false;
                        comp.push(pids[w]);
                        if w == v {
                            break;
                        }
                    }
                    comps.push(comp);
                }
            }
        }
    }

    comps
}

// PORT STATUS: confidence=high todos=0
