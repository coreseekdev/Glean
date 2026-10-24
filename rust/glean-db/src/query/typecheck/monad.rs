// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// Ported from: glean/db/Glean/Query/Typecheck/Monad.hs (187 lines)
// Manifest unit: rust/glean-db/src/query/typecheck/monad.rs
// (migration/manifest.tsv line 90, crate glean-db). RULE §4: one source
// file -> one target file; the manifest path is authoritative.
//
// WHAT THIS FILE IS: the typechecker's MONAD and its environment. The
// exported surface is the state record (TypecheckState, field for field
// with its 17 fields), the two option/environment records (TcOpts,
// TcEnv), the mode tag (TcMode), the state constructor
// (initialTypecheckState), four small combinators over the state
// (whenDebug, freshTyVar, freshTyVarInt, getPredicateDetails), one pattern
// smart constructor (mkWild) and the five error helpers (prettyError,
// prettyErrorIn, prettyErrorAt, inPat, addErrSpan). No IO is performed
// here, no FFI, no unsafe.
//
// -----------------------------------------------------------------------
// THE MONAD IMAGE (the one design decision this unit had to make)
// -----------------------------------------------------------------------
//
// The source is
//
//     type T a = StateT TypecheckState (ExceptT (Doc ()) IO) a
//     type ToRtsType = Schema.Type' () -> Maybe Type        (Monad.hs:55, :57)
//
// Rust has no monad transformers, and no shipped unit in this tree ports
// a two-layer StateT-over-ExceptT stack, so this unit pins the image for
// rows 90/91/92. It is assembled from the three precedents that DO ship:
//
//   * StateT -> an explicit &mut state parameter. There is no RULEBOOK
//     row for this (migration/RULEBOOK.md carries no State row); the
//     shipped precedent is rust/glean-core/src/rts/bytecode/code.rs:131-160,
//     which folds `newtype Code a = Code { runCode :: StateT CodeS IO a }`
//     into ONE struct whose fields are the state record and whose
//     combinators are `&mut self` methods. The same-crate sibling
//     database/config.rs:798-803 is the OTHER shipped shape:
//     process_schema_cached threads `SchemaParserCache` BY VALUE and
//     returns the grown cache (`Result<(SchemaParserCache,
//     ProcessedSchema), String>`) — no `&mut`, because the cache is owned
//     and cheap to move. Here the state record is TypecheckState ITSELF
//     (it is exported with (..) in the source, so it must stay a public
//     record with public fields), which leaves the action's carrier as a
//     parameter:
//
//         pub type T<A> = Result<A, Doc>;
//
//     and every T-action takes the state first:
//         fn some_action(st: &mut TypecheckState, ...) -> T<A>
//
//   * ExceptT e -> Result<_, e>, payload UNCHANGED. The payload here is
//     `Doc ()` (not Text), so the image is `Result<A, Doc>`: the Doc
//     annotation parameter is dropped by the owned Doc stand-in
//     (display.rs:105-109), and the payload stays a Doc because
//     addErrSpan vcat's it and the OUTER boundary converts
//     (`withExceptT (Text.pack . show)`, Typecheck.hs:92/:131 — row 92's
//     obligation, not this file's). The ReaderT sibling precedent is
//     angle/schema/resolve.rs:56-66 (borrowed context struct +
//     Result<_, String>).
//
//   * IO is ambient in Rust. The source's IO exists for `liftIO` inside
//     actions (Typecheck.hs:98's hPutStrLn debug print; the same file
//     has another at :1276); the port's
//     closures may perform IO freely, so the layer contributes nothing
//     to the type.
//
// WHAT A CONSUMER (rows 91/92) MUST DO DIFFERENTLY — this is the
// contract, stated once here:
//   1. `x <- act` becomes `let x = act(st)?;`. A `T<A>` VALUE in Rust is
//      an already-computed Result, so a T-action must be passed as a
//      THUNK (`impl FnOnce(&mut TypecheckState) -> T<A>`) whenever the
//      source passes an unevaluated action: whenDebug, inPat and
//      addErrSpan take closures for exactly that reason (a `T<()>` value
//      would have run its effects at construction time).
//   2. `gets f` / `modify g` become field reads / field writes on `st`.
//      The record is public, so no accessor layer is invented.
//   3. `catchError` becomes `match`: the only catch sites in the T monad
//      are addErrSpan (Monad.hs:183) and Typecheck.hs:108/:132, and the
//      error channel is `Result::Err`, not a Rust panic. `error`/`panic!`
//      (e.g. getPredicateDetails' miss) is NOT catchable this way — which
//      is exactly the source's split: `error` throws ErrorCall, which
//      `catchError` in the ExceptT channel does not catch.
//   4. STATE ROLLBACK — the one semantic asymmetry, analyzed not waved
//      away. In the source the stack is `StateT s (ExceptT e IO)`, whose
//      MonadError instance is
//        catchError m h = StateT $ \s -> catchError (runStateT m s)
//                                          (\e -> runStateT (h e) s)
//      i.e. a handler runs against the state as it was BEFORE the failing
//      action, discarding that action's writes (the tree knows this:
//      Query/Reorder.hs:1020 comments "with the StateT outside, m
//      `catchError` h will restore the state"). The `&mut` image cannot
//      reproduce it: TypecheckState is not Clone (TcEnv holds
//      PredicateDetails, which owns Subroutine/DerivingInfo and derives
//      nothing — database/schema/types.rs:753-756), so there is no
//      snapshot to restore. The divergence is UNOBSERVABLE in this tree:
//      addErrSpan/inPat instantiate the Monad.hs:183 handler at nine
//      Typecheck.hs sites (:562, :685, :795, :1213, :1218, :1235, :1240,
//      :1266, :1271), and that handler never reads state — it only
//      vcat's the span onto the caught Doc. The two OTHER catch sites are
//      Typecheck.hs:108-115 (handler reads only tcDisplayOpts and
//      tcSubst — and the guarded action, zonkTcQuery, only READS tcSubst,
//      so a partial run leaves nothing behind) and Typecheck.hs:132-135
//      (handler immediately re-throws). No handler observes state written
//      by the action it caught. Flagged at add_err_span with the reopen
//      condition.
//
// -----------------------------------------------------------------------
// INVENTORY (RULE §5)
// -----------------------------------------------------------------------
// ZERO rows. `grep -n 'glean/db/Glean/Query/Typecheck/Monad.hs'
// migration/inventory*.tsv` (inventory.tsv, .ffi, .protocol, .sites,
// .stm, .codeflow) returns NOTHING, and `grep -rn 'Typecheck/Monad'
// migration/` hits only depmap/edges.tsv (:651-:659 out, :645/:671 in),
// depmap/order.txt:89 and manifest.tsv:90 — no inventory row. Nothing in
// this file is an inventory-owned class: no FFI, no STM, no
// unsafePerformIO, no laziness with observable effect, no dynamic
// exception (`error` is a pure call, mapped per RULE §2's partial-
// functions row). No target_translation to diverge from, hence no
// inventory-divergence flag is possible or needed.
//
// -----------------------------------------------------------------------
// DEPENDENCY TRUTH (migration/depmap/edges.tsv — checked, not assumed)
// -----------------------------------------------------------------------
//   OUT :651 Monad.hs -> angle/Glean/Angle/Types.hs  (Monad.hs:44 `import
//         Glean.Angle.Types hiding (Type)`, :45 `qualified ... as Schema`)
//         -> crate glean-angle, `glean_angle::angle::types` (AngleVersion,
//         IsSrcSpan, Name, PredicateId, SourcePat_, SrcSpan, TypeId, Type_,
//         source_pat_span).
//   OUT :652 Monad.hs -> angle/Glean/Display.hs      (Monad.hs:48 `import
//         Glean.Display`; :42 also imports Compat.Prettyprinter, whose Doc
//         stand-in this tree owns per RULE §1) -> `glean_angle::display`
//         (Doc, vcat, display_default, DisplayOpts, default_display_opts).
//   OUT :653 Monad.hs -> db/Glean/Database/Config.hs (Monad.hs:46
//         `qualified ... as Config`, for Config.DebugFlags/Config.tcDebug)
//         -> SAME CRATE, `crate::database::config` (DebugFlags,
//         DebugFlags::tc_debug).
//   OUT :654 Monad.hs -> db/Glean/Database/Schema/Types.hs (Monad.hs:47)
//         -> SAME CRATE, `crate::database::schema::types` (TypeDetails,
//         PredicateDetails).
//   OUT :655 Monad.hs -> db/Glean/Query/Codegen/Types.hs (Monad.hs:50
//         `import Glean.Query.Codegen.Types`) -> SAME CRATE,
//         `crate::query::codegen::types` (Match, Var).
//   OUT :656 Monad.hs -> db/Glean/Query/Typecheck/Types.hs (Monad.hs:49)
//         -> SAME CRATE, `crate::query::typecheck::types` (TcPat, TcPred;
//         TcTerm reaches this file only as TcPat's payload).
//   OUT :657 Monad.hs -> hs/Glean/RTS/Term.hs        (Monad.hs:51
//         `import Glean.RTS.Term as RTS`) -> crate glean-core,
//         `glean_core::rts::term::Term` (the RTS.Tuple/RTS.Ref
//         constructors mkWild builds).
//   OUT :658 Monad.hs -> hs/Glean/RTS/Types.hs       (Monad.hs:52
//         `import Glean.RTS.Types as RTS`) -> crate glean-core,
//         `glean_core::rts::types` (Type, PidRef, deref_type).
//   OUT :659 Monad.hs -> util/Glean/Util/Some.hs     (Monad.hs:53 `import
//         Glean.Util.Some` — the `Some IsSrcSpan` of tcPromote) -> the
//         unit-38 decision record (rust/glean-util/src/util/some.rs) ships
//         ZERO items (row 136's per-use-site rule), so this edge
//         DISSOLVES into the TcPred image, exactly as it did for row 88
//         (query/typecheck/types.rs:63-66): see the tc_promote field.
//   IN  :645 Typecheck.hs (row 92), :671 Typecheck/Unify.hs (row 91) —
//         both unshipped; both consume this file's whole export list.
//
// CARGO EDGES: none added. glean-angle and glean-core are already this
// crate's dependencies (the shipped sibling query/typecheck/types.rs:114
// imports both paths).
//
// -----------------------------------------------------------------------
// ASSUMED-IMPORT (RULE §4 B4)
// -----------------------------------------------------------------------
// NONE. Every type this file consumes is owned by a SHIPPED manifest unit
// (the nine edges above resolve to rust/glean-angle/src/{angle/types.rs,
// display.rs}, rust/glean-core/src/rts/{term.rs,types.rs} and this crate's
// database/config.rs, database/schema/types.rs, query/codegen/types.rs,
// query/typecheck/types.rs). Nothing here comes from generated code
// (glean_thrift) or from the pinned compiler's newtype images, so there is
// no assumed shape to pin and no stub to declare.
//
// -----------------------------------------------------------------------
// DERIVING PARITY
// -----------------------------------------------------------------------
//   * TcMode: the source derives Eq ONLY (Monad.hs:73-74) -> PartialEq +
//     Eq. No Clone/Copy/Debug/Ord/Hash is added: the unit brief pins "Eq
//     only", and the source's own consumers only compare it
//     (`tcMode == TcModeQuery` shapes). Consequence: read it through the
//     record (`st.tc_mode == TcMode::TcModeQuery`) rather than copying it.
//   * TcEnv, TcOpts, TypecheckState: NO deriving clause in the source
//     (Monad.hs:76-79, :84-108, :111-114) and none is possible — TcEnv
//     holds PredicateDetails (owns Subroutine/DerivingInfo, derives
//     nothing: database/schema/types.rs:753-756), TcOpts holds DebugFlags
//     (derives nothing: database/config.rs:644), and TypecheckState holds
//     `Box<dyn Fn>` (not Clone). None is added.
//   * DisplayOpts inside TypecheckState is Copy (display.rs:569) but the
//     containing record is not; that is the source's shape too.
//
// -----------------------------------------------------------------------
// SIBLING-UNIT CONTRACTS (RULE §4; read, never re-declared)
// -----------------------------------------------------------------------
//   * SHIPPED row 88 rust/glean-db/src/query/typecheck/types.rs owns TcPat
//     (:245), TcPred (:322 — the CONCRETE `(PidRef, SrcSpan)` decided by
//     the 2026-10-24 TcPred ruling, DECISIONS.md) and TcTerm. This file
//     imports them and INHERITS the ruling: `tc_preds: Vec<TcPred>` and
//     `tc_promote: Vec<(Type, Type, SrcSpan)>` both carry concrete
//     SrcSpan (query/typecheck/types.rs:247-322 — :282 states that rows
//     90/91/92 inherit the ruling verbatim, :311 is the reopen condition).
//   * SHIPPED row 87 rust/glean-db/src/query/codegen/types.rs owns
//     Var (:323), Match (:378, MatchWild :380) and Typed (:574).
//   * SHIPPED rust/glean-core/src/rts/types.rs owns Type (:250),
//     PidRef (:185) and deref_type (:394-400, `&Type -> &Type` — mkWild
//     borrows the input for the RecordTy test and keeps the ORIGINAL value
//     for the otherwise arm, which is the source's no-deref arm).
//   * SHIPPED rust/glean-core/src/rts/term.rs owns Term<R> (:130) — the
//     Tuple and Ref constructors mkWild builds.
//   * SHIPPED rust/glean-angle/src/angle/types.rs owns Name (:369),
//     SrcSpan (:401), source_pat_span (:711, `&SourcePat_<..> -> &S`),
//     Type_ (:749), PredicateId (:931), TypeId (:986), AngleVersion
//     (:1168, Copy) and IsSrcSpan (:1932, supertrait Display).
//   * SHIPPED rust/glean-angle/src/display.rs owns Doc (:110), vcat
//     (:380), DisplayOpts (:570), default_display_opts (:577) and
//     display_default (:592). It deliberately RENDERS nothing (:99-100,
//     :537-546), which is why getPredicateDetails' `show` needs the
//     private stand-in below.
//   * SHIPPED rust/glean-db/src/database/config.rs owns DebugFlags (:644,
//     field tc_debug :645) and is the same-crate StateT precedent in BOTH
//     shapes: load_instances threads `StateT SchemaParserCache` as `&mut`
//     explicitly (:901), while process_schema_cached threads the cache by
//     value and returns it (:798-803) — see THE MONAD IMAGE.
//   * SHIPPED rust/glean-db/src/database/schema/types.rs owns TypeDetails
//     (:748) and PredicateDetails (:756), and mk_rts_type (:1028) — the
//     PRODUCER of a ToRtsType value: it takes `&Type_<S, PredicateId,
//     TypeId>` and `&dyn Fn` lookups (db_schema_rts_type :998-1018 is the
//     wrapper that closes them over a `&DbSchema`), which is why ToRtsType
//     takes its argument by REFERENCE and is a boxed closure rather than a
//     fn pointer (Schema.hs:833 builds it by partial application —
//     `mkRtsType lookupType` applied to a `HashMap.lookup idToPid` closure).
//   * UNSHIPPED rows 91/92 (Unify.hs, Typecheck.hs) consume everything
//     here. Three obligations this file's field types create for them:
//       (a) `tc_rts_type` is a `Box<dyn Fn>` stored IN the state, so
//           `gets tcRtsType` must be called in one expression —
//           `(st.tc_rts_type)(&ty)` — not hoisted into a local borrow
//           that outlives a `&mut st` call (the borrow checker rejects
//           that; the closure needs no state, so the inline call is
//           always available).
//       (b) `tc_free` / `tc_uses` / `tc_bindings` are std HashSet<String>.
//           The source renders `HashSet.toList` into ERROR TEXT
//           (Typecheck.hs:817, :828 — "variables were mentioned only
//           once" / "not bound anywhere"), and unordered-containers'
//           toList order is hash order. std HashSet's iteration order is
//           a different hash order, so those two diagnostics can list the
//           same names in a different order. Row 92 owns that site; it
//           must either reproduce the source's order (it is a function of
//           the source's hashWithSalt chain, which no shipped unit ports)
//           or flag it. Recorded here because the field TYPE is this
//           file's choice: RULEBOOK §1 has no container row, so the std
//           default stands — the tree's standing facts
//           (migration/RUNBOOK.md:59) pin only the ORDERED containers
//           (`Data.Set` -> `BTreeSet`, `Data.Map` -> `BTreeMap`) and say
//           nothing about the unordered ones.
//       (c) `tc_rts_type` is a `Box<dyn Fn ...>` with an implicit
//           'static bound, but its producer is a partial application over
//           a SCHEMA (Schema.hs:833 `rtsType = mkRtsType lookupType
//           (`HashMap.lookup` idToPid)`) and the shipped producer
//           BORROWS: `db_schema_rts_type(db_schema: &DbSchema, ty:
//           &Type_<..>)` (database/schema/types.rs:998-1001; its lookups
//           are `&dyn Fn`, :1028-1032) and DbSchema (schema/types.rs:568)
//           derives nothing. So a row-91/92 consumer holding `&DbSchema`
//           CANNOT build this box: it must OWN the schema
//           (`Rc<DbSchema>`/`Arc<DbSchema>`) — or this alias needs a
//           lifetime parameter — and the lazy typedefs knot
//           (Schema.hs:822-831: `typedefs` is a `Lazy.HashMap` whose
//           values call `rtsType` back through `lookupType`) needs
//           `Rc<RefCell<...>>`, not an eager clone (which would diverge
//           on exactly the recursive typedefs that knot exists for).
//   * tc_vars / tc_subst are `BTreeMap<i64, _>`, NOT the tree's usual
//     `HashMap<i64, _>` (code.rs:272, thrift/internal.rs:241). Reason:
//     IntMap is an ORDERED map and its order is read here — Typecheck.hs
//     :102 prints `IntMap.toList subst` and Unify.hs:279 traverses
//     tcVars; BTreeMap<i64, _> reproduces IntMap's ascending-key order
//     exactly, a HashMap would not. (migration/RUNBOOK.md:59's standing
//     fact `Data.Map` -> `BTreeMap` is the same ordered-map reasoning;
//     RULEBOOK §2 has no Data.Map row either.)

use std::collections::{BTreeMap, HashMap, HashSet};

use glean_angle::angle::types::{
    source_pat_span, AngleVersion, IsSrcSpan, Name, PredicateId, SourcePat_, SrcSpan, TypeId, Type_,
};
use glean_angle::display::{default_display_opts, display_default, vcat, DisplayOpts, Doc};
use glean_core::rts::term::Term;
use glean_core::rts::types::{deref_type, Type};

use crate::database::config::DebugFlags;
use crate::database::schema::types::{PredicateDetails, TypeDetails};
use crate::query::codegen::types::{Match, Var};
use crate::query::typecheck::types::{TcPat, TcPred};

// -----------------------------------------------------------------------------
// The monad (Monad.hs:55, :57)
// -----------------------------------------------------------------------------

/// `type T a = StateT TypecheckState (ExceptT (Doc ()) IO) a`
/// (Monad.hs:55) — see THE MONAD IMAGE in the header. A `T<A>` is the
/// action's OUTCOME (its error channel); every STATEFUL action takes the
/// state as an explicit `&mut TypecheckState` first parameter, and an
/// unevaluated action is a thunk `FnOnce(&mut TypecheckState) -> T<A>`.
/// The three `pretty_error*` constructors build an error value without
/// touching state, so they take no state parameter.
/// The `Doc ()` payload keeps its annotation dropped (display.rs:105-109).
pub type T<A> = Result<A, Doc>;

/// `type ToRtsType = Schema.Type' () -> Maybe Type` (Monad.hs:57), where
/// `Schema.Type' () = Type_ () PredicateId TypeId` (Angle/Types.hs:641).
///
/// A BOXED closure, not a `fn` pointer: the only producer, Schema.hs:833,
/// builds it by partial application — `mkRtsType lookupType` applied to a
/// `HashMap.lookup idToPid` closure — and the shipped producer mk_rts_type
/// (database/schema/types.rs:1028) takes its type argument by reference —
/// so the alias takes `&Type_<..>` too, avoiding a clone per conversion.
pub type ToRtsType = Box<dyn Fn(&Type_<(), PredicateId, TypeId>) -> Option<Type>>;

/// `data TcMode = TcModeQuery | TcModePredicate deriving Eq` (Monad.hs:73-74).
///
/// Eq only, exactly as the source: no Clone/Copy/Debug/Ord/Hash (DERIVING
/// PARITY in the header). Compare through the record.
#[derive(PartialEq, Eq)]
pub enum TcMode {
    TcModeQuery,
    TcModePredicate,
}

/// `data TcEnv = TcEnv { tcEnvTypes :: HashMap TypeId TypeDetails
/// , tcEnvPredicates :: HashMap PredicateId PredicateDetails }`
/// (Monad.hs:76-79), exported with (..) — fields stay public.
///
/// Both maps are `std::collections::HashMap` — the std default, since
/// RULEBOOK §1 has no container row and the standing facts
/// (migration/RUNBOOK.md:59) pin only the ordered containers. Their
/// iteration order is never observed by this module, and the consumers
/// only look up. TypeId/PredicateId carry the shipped hash-only
/// Eq + Hash impls (angle/types.rs:945-957, :994-1005).
pub struct TcEnv {
    pub tc_env_types: HashMap<TypeId, TypeDetails>,
    pub tc_env_predicates: HashMap<PredicateId, PredicateDetails>,
}

/// `emptyTcEnv = TcEnv HashMap.empty HashMap.empty` (Monad.hs:81-82).
pub fn empty_tc_env() -> TcEnv {
    TcEnv {
        tc_env_types: HashMap::new(),
        tc_env_predicates: HashMap::new(),
    }
}

/// `data TypecheckState` (Monad.hs:84-108), field for field, in
/// declaration order. Exported with (..) in the source, so every field is
/// `pub` — rows 91/92 read and write them directly (`gets`/`modify` in the
/// source, plain field access here; no accessor layer is invented).
pub struct TypecheckState {
    pub tc_env: TcEnv,
    pub tc_angle_version: AngleVersion,
    /// `!Bool` (Monad.hs:87).
    pub tc_debug: bool,
    pub tc_rts_type: ToRtsType,
    /// `{-# UNPACK #-} !Int` (Monad.hs:89) -> i64 (LP64; the unboxing
    /// pragma is a GHC heap-layout detail, dropped as in codegen/types.rs
    /// :325-327).
    pub tc_next_var: i64,
    /// `{-# UNPACK #-} !Int` (Monad.hs:90) -> i64.
    pub tc_next_ty_var: i64,
    /// Variables that we have types for, and have allocated a Var
    /// (source comment, Monad.hs:91-92).
    pub tc_scope: HashMap<Name, Var>,
    /// Variables that are currently visible (Monad.hs:93-94).
    pub tc_visible: HashSet<Name>,
    /// Variables that are mentioned only once (Monad.hs:95-96).
    // TODO(port): unordered-containers hash order is not reproducible with
    // std HashSet for tc_free / tc_uses / tc_bindings, and row 92 renders
    // `HashSet.toList` into two error messages (Typecheck.hs:817, :828) —
    // the name lists can come out in a different order. Owner: row 92.
    pub tc_free: HashSet<Name>,
    /// Accumulates variables that appear in an ContextExpr context
    /// (Monad.hs:97-98).
    pub tc_uses: HashSet<Name>,
    /// Accumulates variables that appear in an ContextPat context
    /// (Monad.hs:99-100).
    pub tc_bindings: HashSet<Name>,
    pub tc_mode: TcMode,
    /// Options for pretty-printing (Monad.hs:102-103).
    pub tc_display_opts: DisplayOpts,
    /// `IntMap Var` (Monad.hs:104) -> BTreeMap keyed by i64 (header:
    /// IntMap is ordered and its order is read).
    pub tc_vars: BTreeMap<i64, Var>,
    /// `IntMap Type` (Monad.hs:105) -> BTreeMap keyed by i64.
    pub tc_subst: BTreeMap<i64, Type>,
    /// `[(Type, Type, Some IsSrcSpan)]` (Monad.hs:106). The existential is
    /// the CONCRETE SrcSpan per the 2026-10-24 TcPred ruling (DECISIONS.md;
    /// query/typecheck/types.rs:664-679 states rows 90/91/92 inherit it
    /// verbatim — the same two SrcSpan-instantiated callers wrap here via
    /// addPromote Typecheck.hs:1243-1246 (`Some span` is built at :1246)
    /// and unpack in resolvePromote :1248-1269).
    pub tc_promote: Vec<(Type, Type, SrcSpan)>,
    /// predicate identifiers in query, used for diagnostics
    /// (Monad.hs:107-108); TcPred is the shipped concrete pair.
    pub tc_preds: Vec<TcPred>,
}

/// `data TcOpts = TcOpts { tcOptDebug :: !Config.DebugFlags
/// , tcOptAngleVersion :: !AngleVersion }` (Monad.hs:111-114), exported
/// with (..) — fields stay public.
pub struct TcOpts {
    pub tc_opt_debug: DebugFlags,
    pub tc_opt_angle_version: AngleVersion,
}

/// `defaultTcOpts debug v = TcOpts { tcOptDebug = debug
/// , tcOptAngleVersion = v }` (Monad.hs:116-121). A plain constructor: the
/// source's record syntax carries no behaviour.
pub fn default_tc_opts(debug: DebugFlags, v: AngleVersion) -> TcOpts {
    TcOpts {
        tc_opt_debug: debug,
        tc_opt_angle_version: v,
    }
}

/// `initialTypecheckState tcEnv TcOpts{..} rtsType mode` (Monad.hs:122-147),
/// field for field.
///
/// `opts` is taken by REFERENCE although the source passes it by value:
/// typecheckDeriving reads `tcOptAngleVersion opts` at Typecheck.hs:159,
/// AFTER building the state at :129 (the typecheck entry point hands
/// `opts` to the constructor at :91 and never touches it again) — Haskell
/// gets that for free from sharing, whereas a by-value Rust parameter
/// would move TcOpts, and TcOpts is not Clone
/// (DebugFlags derives nothing, database/config.rs:644). The two fields
/// read are Copy (AngleVersion :1168, bool), so no clone is needed.
pub fn initial_typecheck_state(
    tc_env: TcEnv,
    opts: &TcOpts,
    rts_type: ToRtsType,
    mode: TcMode,
) -> TypecheckState {
    TypecheckState {
        tc_env,
        tc_angle_version: opts.tc_opt_angle_version,
        // Config.tcDebug tcOptDebug
        tc_debug: opts.tc_opt_debug.tc_debug,
        tc_rts_type: rts_type,
        tc_next_var: 0,
        tc_next_ty_var: 0,
        tc_scope: HashMap::new(),
        tc_visible: HashSet::new(),
        tc_free: HashSet::new(),
        tc_uses: HashSet::new(),
        tc_bindings: HashSet::new(),
        tc_mode: mode,
        // defaultDisplayOpts — "might make this configurable with flags
        // later" (source comment, Monad.hs:142).
        tc_display_opts: default_display_opts(),
        tc_subst: BTreeMap::new(),
        tc_vars: BTreeMap::new(),
        tc_promote: Vec::new(),
        tc_preds: Vec::new(),
    }
}

// -----------------------------------------------------------------------------
// State combinators (Monad.hs:59-68, :149-154)
// -----------------------------------------------------------------------------

/// `whenDebug act = do { lvl <- gets tcDebug; when lvl act }`
/// (Monad.hs:59-62).
///
/// The action is a THUNK: the source's `act :: T ()` is unevaluated until
/// `when lvl act` runs it, and a Rust `T<()>` value would already have run.
/// The read happens before the branch, exactly as `gets` does.
pub fn when_debug<F>(st: &mut TypecheckState, act: F) -> T<()>
where
    F: FnOnce(&mut TypecheckState) -> T<()>,
{
    // lvl <- gets tcDebug
    let lvl = st.tc_debug;
    // when lvl act
    if lvl {
        act(st)
    } else {
        Ok(())
    }
}

/// `freshTyVarInt = do { TypecheckState{tcNextTyVar = n} <- get
/// ; modify $ \s -> s { tcNextTyVar = n+1 }; return n }`
/// (Monad.hs:64-68) — the OLD value is returned and the counter advances.
///
/// The increment wraps like the source's `Int` `+` (GHC `Int` is two's
/// complement, and the kit pins `Int` = `i64`);
/// unreachable in practice (2^63 tyvars), but wrapping is what the source
/// does, so `wrapping_add` is the faithful image rather than a debug-build
/// panic.
pub fn fresh_ty_var_int(st: &mut TypecheckState) -> T<i64> {
    let n = st.tc_next_ty_var;
    st.tc_next_ty_var = n.wrapping_add(1);
    Ok(n)
}

/// `freshTyVar = TyVar <$> freshTyVarInt` (Monad.hs:70-71).
pub fn fresh_ty_var(st: &mut TypecheckState) -> T<Type> {
    Ok(Type_::TyVar(fresh_ty_var_int(st)?))
}

/// `getPredicateDetails pred = do { TcEnv{..} <- gets tcEnv
/// ; case HashMap.lookup pred tcEnvPredicates of
///     Nothing -> error $ "predicateKeyTYpe: " <> show (displayDefault pred)
///     Just d -> return d }` (Monad.hs:149-154).
///
/// Image note: the source's type is `T PredicateDetails`, but the body
/// performs no state write and has no throwError path — its only effects
/// are a state READ and an `error` (a panic, which `catchError` in the
/// ExceptT channel would NOT catch either). Returning `Result<&_>` would
/// therefore be a fiction; the image is a borrowed lookup, and a caller
/// that needs an owned field clones it (PredicateDetails is not Clone —
/// database/schema/types.rs:753-756; Type IS Clone, so `predicate_key_type`
/// clones).
///
/// The panic text is the source's, typo included:
/// `"predicateKeyTYpe: " <> show (displayDefault pred)` — `show` on a Doc
/// renders it (the private `show_doc` below).
pub fn get_predicate_details<'a>(
    st: &'a TypecheckState,
    pred: &PredicateId,
) -> &'a PredicateDetails {
    match st.tc_env.tc_env_predicates.get(pred) {
        // Nothing -> error $ "predicateKeyTYpe: " <> show (displayDefault pred)
        None => panic!("predicateKeyTYpe: {}", show_doc(&display_default(pred))),
        // Just d -> return d
        Some(d) => d,
    }
}

// -----------------------------------------------------------------------------
// mkWild (Monad.hs:156-164)
// -----------------------------------------------------------------------------

/// Smart constructor for wildcard patterns; replaces a wildcard that
/// matches the unit type with a concrete pattern.  This is necessary
/// when we have an enum in an expression position: we can't translate
/// @nothing@ into @{ nothing = _ }@ because the wildcard would be
/// illegal in an expression. (Source doc, Monad.hs:156-160 — transcribed.)
///
/// ```haskell
/// mkWild ty
///   | RecordTy [] <- derefType ty = RTS.Tuple []
///   | otherwise = RTS.Ref (MatchWild ty)
/// ```
///
/// Two details preserved deliberately:
///   * the guard DEREFS (`derefType ty`) while the otherwise arm does
///     NOT: a NamedTy expanding to the unit record takes the Tuple arm,
///     but any other type is wrapped ORIGINAL, not dereferenced (RULE §2's
///     pattern-shape row — a deref in the otherwise arm would change what
///     MatchWild carries);
///   * the unit test is `RecordTy []`, i.e. a RecordTy with NO fields —
///     not `isUnit` (that lives in Typecheck.hs and also accepts the
///     maybe-of-unit shapes; mkWild does not use it).
pub fn mk_wild(ty: Type) -> TcPat {
    // RecordTy [] <- derefType ty
    if let Type_::RecordTy(fields) = deref_type(&ty) {
        if fields.is_empty() {
            // RTS.Tuple []
            return Term::Tuple(Vec::new());
        }
    }
    // otherwise = RTS.Ref (MatchWild ty) — the ORIGINAL ty, no deref.
    Term::Ref(Match::MatchWild(ty))
}

// -----------------------------------------------------------------------------
// Errors (Monad.hs:166-187)
// -----------------------------------------------------------------------------

/// `prettyError = throwError` (Monad.hs:166-167).
pub fn pretty_error<A>(doc: Doc) -> T<A> {
    Err(doc)
}

/// `prettyErrorIn pat doc = prettyErrorAt (sourcePatSpan pat) doc`
/// (Monad.hs:169-170).
pub fn pretty_error_in<S, ST, P, TT, A>(pat: &SourcePat_<S, ST, P, TT>, doc: Doc) -> T<A>
where
    S: IsSrcSpan,
{
    pretty_error_at(source_pat_span(pat), doc)
}

/// `prettyErrorAt span doc = prettyError $ vcat [ pretty span, doc ]`
/// (Monad.hs:172-176).
///
/// `vcat` order is load-bearing: the SPAN line comes FIRST, then the
/// message. The span is taken by reference (the source passes it by value
/// but only renders it; a generic `IsSrcSpan` has no Copy bound — the same
/// borrow convention as Display's opts, display.rs:613-615).
///
/// The source renders the span with `pretty`, i.e. through the Pretty
/// class, which `IsSrcSpan` carries as a supertrait (Angle/Types.hs:865);
/// the port calls `display_default` directly. That is EQUAL for the only
/// instance that reaches here — `instance Pretty SrcSpan` is `pretty =
/// displayDefault` (Angle/Types.hs:887-888) — and the narrower call cannot
/// express SpanAngleDSL, which the 2026-10-24 ruling keeps out of scope
/// (query/typecheck/types.rs:283-294).
pub fn pretty_error_at<S, A>(span: &S, doc: Doc) -> T<A>
where
    S: IsSrcSpan,
{
    pretty_error(vcat([display_default(span), doc]))
}

/// `inPat pat = addErrSpan (sourcePatSpan pat)` (Monad.hs:180-181).
pub fn in_pat<S, ST, P, TT, A, F>(
    st: &mut TypecheckState,
    pat: &SourcePat_<S, ST, P, TT>,
    act: F,
) -> T<A>
where
    S: IsSrcSpan,
    F: FnOnce(&mut TypecheckState) -> T<A>,
{
    add_err_span(st, source_pat_span(pat), act)
}

/// addErrSpan span act = act `catchError` \errDoc -> do
///    prettyError $ vcat [ pretty span, errDoc ]   (Monad.hs:181-187)
///
/// It rewrites ONLY the error channel: the success value passes through
/// untouched and the handler immediately re-throws, so this combinator can
/// never turn a failure into a success. Ordering is the source's — span
/// line first, then the caught doc.
///
/// `act` is a thunk for the reason given at when_debug. A Rust `panic!`
/// inside `act` is not caught here, which matches the source: `error`
/// throws ErrorCall, not the ExceptT channel's Doc.
pub fn add_err_span<S, A, F>(st: &mut TypecheckState, span: &S, act: F) -> T<A>
where
    S: IsSrcSpan,
    F: FnOnce(&mut TypecheckState) -> T<A>,
{
    match act(st) {
        // act `catchError` \errDoc -> prettyError $ vcat [pretty span, errDoc]
        Err(err_doc) => pretty_error(vcat([display_default(span), err_doc])),
        ok => ok,
    }
}

// TODO(port): state rollback on the caught-error path — the source's
// `StateT s (ExceptT e IO)` catchError restores the pre-action state
// (transformers' MonadError instance; the tree relies on it at
// Query/Reorder.hs:1020), while the `&mut` image keeps whatever the
// failing action wrote. TypecheckState is not Clone (TcEnv holds
// PredicateDetails, database/schema/types.rs:753-756), so no snapshot is
// available. UNOBSERVABLE today: addErrSpan/inPat instantiate the
// Monad.hs:183 handler at nine Typecheck.hs sites (:562, :685, :795,
// :1213, :1218, :1235, :1240, :1266, :1271) and that handler never reads
// state — it only vcat's the span onto the caught Doc. The other two catch
// sites are Typecheck.hs:108-115 (reads only tcDisplayOpts/tcSubst, and
// the guarded zonkTcQuery only READS tcSubst) and :132-135 (re-throws).
// None observes a state write made by the action it caught. Reopen if rows
// 91/92 (or a later consumer) introduce a catch-and-continue handler that
// reads state. Owner: rows 91/92.
//
// TODO(port): `show (pretty …)` render parity — getPredicateDetails'
// panic message renders a Doc through prettyprinter 1.7.1's `Show (Doc
// ann)` (renderShowS (layoutPretty defaultLayoutOptions doc), 80-column
// Wadler layout). That surface is UNOWNED in the manifest (display.rs
// :537-546: this module deliberately renders nothing). This private
// flat-branch stand-in duplicates the sibling copies in
// rust/glean-angle/src/schema/resolve.rs:1958 and
// rust/glean-angle/src/schema/evolve.rs:1186 (no shared owner exists yet)
// and renders the FLAT branch of every group. It is NOT byte-exact with
// Haskell's `show (Doc)` in general: `Show (Doc ann)` escapes quotes,
// backslashes and newlines in the text payload, while `go` pushes text
// raw. It IS byte-exact for the only value that actually reaches it here —
// getPredicateDetails passes `displayDefault pred`, a PredicateId, which
// renders as a single `Text` (nothing to escape, no group to wrap). When
// display.rs's renderer lands, delete this copy and route all three units
// through it.
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
                let col = out.chars().rev().take_while(|c| *c != '\n').count() as i64;
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

// -----------------------------------------------------------------------------
// EXPORT PARITY (the source's list, Monad.hs:9-31, ticked item by item —
// the unit-18 addendum's checklist)
// -----------------------------------------------------------------------------
//   T                    -> pub type T<A>                      (alias, no (..))
//   ToRtsType            -> pub type ToRtsType                 (alias, no (..))
//   TypecheckState(..)   -> pub struct TypecheckState, 17 pub fields
//   TcOpts(..)           -> pub struct TcOpts, 2 pub fields
//   TcMode(..)           -> pub enum TcMode, 2 variants
//   defaultTcOpts        -> pub fn default_tc_opts
//   TcEnv(..)            -> pub struct TcEnv, 2 pub fields
//   emptyTcEnv           -> pub fn empty_tc_env
//   initialTypecheckState-> pub fn initial_typecheck_state
//   whenDebug            -> pub fn when_debug
//   freshTyVar           -> pub fn fresh_ty_var
//   freshTyVarInt        -> pub fn fresh_ty_var_int
//   getPredicateDetails  -> pub fn get_predicate_details
//   mkWild               -> pub fn mk_wild
//   prettyError          -> pub fn pretty_error
//   prettyErrorIn        -> pub fn pretty_error_in
//   prettyErrorAt        -> pub fn pretty_error_at
//   inPat                -> pub fn in_pat
//   addErrSpan           -> pub fn add_err_span
// All 19 exports are present. Nothing else is public: `show_doc` is
// private (it is not in the source's list — the source renders through the
// prettyprinter library).
//
// Marker breakdown (closed set — 3 open port todos, zero of every other
// class):
//   1. State rollback on the caught-error path (at add_err_span): the
//      &mut image cannot restore the pre-action state the source's
//      StateT-over-ExceptT catchError restores; unobservable at the nine
//      addErrSpan/inPat sites plus the two other catch sites enumerated
//      above; reopen condition and owner (rows 91/92) recorded.
//   2. Doc rendering for getPredicateDetails' panic text (at the private
//      show_doc): the flat-branch stand-in for the unowned render surface,
//      third private copy after resolve.rs:1958 and evolve.rs:1186.
//   3. HashSet iteration order for tc_free/tc_uses/tc_bindings (at the
//      three fields): std HashSet is not unordered-containers' hash order,
//      and row 92 renders two of these sets into error text
//      (Typecheck.hs:817, :828). Owner: row 92.
// No source-defect reproductions: the source has no defects here (the
// "predicateKeyTYpe" typo is reproduced verbatim as the panic TEXT, not
// flagged as a defect — RULE §2's partial-functions row requires the same
// message). No performance deferrals: nothing here is perf-critical.
// No inventory divergence: this module has no inventory row (INVENTORY
// above). No unsafe (RULE §3 vacuously satisfied).
//
// PORT STATUS: confidence=high todos=3
