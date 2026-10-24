// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// MANIFEST UNIT: this file IS manifest row 92's target
// (glean/db/Glean/Query/Typecheck.hs -> rust/glean-db/src/query/
// typecheck.rs, crate glean-db) — its Typecheck.hs port is NOT YET
// SHIPPED. The file was CREATED EARLY, by row 88's unit (the typecheck
// area's first file, query/typecheck/types.rs, manifest.tsv line 88), as
// the Rust-2018 directory-module anchor: rows 88/90/91 have
// DIRECTORY-LEVEL manifest paths (src/query/typecheck/types.rs — line 88,
// src/query/typecheck/monad.rs — line 90, src/query/typecheck/unify.rs —
// line 91), which Rust resolves only through src/query/typecheck.rs +
// src/query/typecheck/ — the same 2018 layout as src/query/codegen.rs +
// codegen/ (created early by row 86, the unit-87-anchored precedent this
// file mirrors). Without the anchor line below, the area's files are
// unwireable (src/query.rs cannot name a directory module that has no
// file).
//
// OWNERSHIP HANDOFF (row 92's implementer: read before appending):
//   * The `pub mod types;` line below (row 88's wiring) and this header
//     are rows 88's contribution — keep the line, replace nothing else.
//     Rows 90 (monad.rs) and 91 (unify.rs) append their own `pub mod`
//     lines beside it when they land: row 90's (`pub mod monad;`) is
//     already below; row 91's is below too.
//   * The FILE BODY (the whole Typecheck.hs port: the `typecheck` /
//     `typecheckDeriving` entry points — Typecheck.hs:78-116/:119-174,
//     the infer* drivers, typecheckPattern, promote/demote, resolvePromote,
//     zonk — plus `tcQueryDeps`, consumed by Prune.hs:29/Derive.hs:50/
//     Schema/Gen/Main.hs:59) is row 92's obligation. Append below the
//     wiring lines.
//   * The TcPred span image row 92 builds against is DECIDED in
//     query/typecheck/types.rs (the TcPred archaeology block): the
//     concrete `SrcSpan` — both production entry-point callers
//     (UserQuery.hs:980 via the parser, Schema.hs:881 via
//     predicateSchema :: PredicateDef' SrcSpan ()) instantiate the
//     typechecker at SrcSpan; the reopen condition is recorded there.
//
// Area wiring: `pub mod typecheck;` hangs off src/query.rs (appended by
// row 88's unit, with the ownership note); `crate::query` hangs off
// lib.rs (lib.rs:64 — already present). The crate-root lib.rs carries no
// `pub mod typecheck;` line — that would resolve to crate::typecheck and
// need src/typecheck.rs, which is not this area's manifest path (RULE §4:
// the module header decides the path).

pub mod types;

pub mod monad;

pub mod unify;

// POST-PORT NOTE: the anchor header above records the PRE-PORT state (it
// was written by row 88 before this unit landed); its "NOT YET SHIPPED"
// and "The Typecheck.hs port itself has NOT started here" sentences are
// historical. Row 92's body follows, and the anchor's single port todo
// has been deleted. The three `pub mod` lines above are still rows
// 88/90/91's wiring and are unchanged.

// -----------------------------------------------------------------------------
// INVENTORY (RULE §5)
// -----------------------------------------------------------------------------
// ZERO rows. grep -n 'db/Glean/Query/Typecheck.hs' migration/inventory*.tsv
// (inventory.tsv, .ffi, .protocol, .sites, .stm, .codeflow) returns
// NOTHING, and grep -rn 'db/Glean/Query/Typecheck.hs' migration/ hits
// only HANDOFF.md:201/:478, depmap/edges.tsv (:376/:566/:617/:640-:650/
// :696/:1560), depmap/order.txt:91 and manifest.tsv:92 — no inventory row.
// The Typecheck-hitting rows that DO exist (inventory.tsv:77/:78
// RTS/Foreign/Typecheck.hs FFI, :81 RTS/Typecheck.hs mdo labels;
// inventory.codeflow.tsv:11/:17 and inventory.ffi.tsv:16 the same two
// modules; inventory.sites.tsv:17/:18/:171/:522-:529) govern OTHER
// modules, not this one. Nothing in this file is an inventory-owned class:
// no FFI, no STM, no unsafePerformIO, no dynamic exception (the error
// calls are pure partial functions, mapped per RULE §2's partial-functions
// row), no laziness with observable effect (the Negation arm's cyclic ord
// thunk is never forced — see the note there). No target_translation to
// diverge from, hence no inventory-divergence flag is possible or needed.

// -----------------------------------------------------------------------------
// DEPENDENCY TRUTH (migration/depmap/edges.tsv — checked, not assumed)
// -----------------------------------------------------------------------------
// OUT (11 edges; the source's import block is Typecheck.hs:28-67):
//   :640 Typecheck.hs -> angle/Glean/Angle/Types.hs (Typecheck.hs:53
//        'import Glean.Angle.Types hiding (Type)', :54 qualified as
//        Schema) -> crate glean-angle, glean_angle::angle::types
//        (Field, FieldName, IsSrcSpan, Name, Ordered, PredicateId,
//        PrimOp, RecordOrSum, SeekSection, SourcePat_, SourceQuery_,
//        SourceStatement_, SrcSpan, TypeId, Type_, rm_loc_type,
//        source_pat_span, span_between).
//   :641 Typecheck.hs -> angle/Glean/Display.hs (Typecheck.hs:55
//        'import Glean.Display'; :49 also imports Compat.Prettyprinter,
//        whose Doc stand-in this tree owns per RULE §1) ->
//        glean_angle::display (Display, Doc, display_default, hang,
//        hsep, nest, punctuate, vcat).
//   :642 Typecheck.hs -> angle/Glean/Schema/Util.hs (Typecheck.hs:66
//        'import Glean.Schema.Util') -> glean_angle::schema::util
//        (lower_maybe :78, tuple_schema :143, unit :72).
//   :643 Typecheck.hs -> db/Glean/Database/Schema/Types.hs
//        (Typecheck.hs:65) -> SAME CRATE, crate::database::schema::types
//        (DbSchema :568, PredicateDetails :756).
//   :644 Typecheck.hs -> db/Glean/Query/Codegen/Types.hs
//        (Typecheck.hs:56-57 'import Glean.Query.Codegen.Types
//        (Match(..), Var(..), QueryWithInfo(..), Typed(..))') -> SAME
//        CRATE, crate::query::codegen::types (Match :378, Var :323,
//        QueryWithInfo :148, Typed :574).
//   :645 Typecheck.hs -> db/Glean/Query/Typecheck/Monad.hs
//        (Typecheck.hs:59) -> SAME CRATE, crate::query::typecheck::monad
//        (the monad image T :311, TypecheckState :360, TcEnv :343,
//        TcOpts :412, TcMode :328, ToRtsType :321, initialTypecheckState
//        :438, whenDebug :478, freshTyVarInt :501, freshTyVar :508,
//        getPredicateDetails :529, mkWild :566, prettyError :583,
//        prettyErrorIn :589, prettyErrorAt :611, inPat :619, addErrSpan
//        :642).
//   :646 Typecheck.hs -> db/Glean/Query/Typecheck/Types.hs
//        (Typecheck.hs:58) -> SAME CRATE, crate::query::typecheck::types
//        (TcQuery :133, TcStatement :190, TcPat :245, TcPred :322,
//        TcTerm :360, TypecheckedQuery :587, lookup_field :615).
//   :647 Typecheck.hs -> db/Glean/Query/Typecheck/Unify.hs
//        (Typecheck.hs:60) -> SAME CRATE, crate::query::typecheck::unify
//        (unify :324, apply :696, zonkVars :925, zonkTcQuery :974).
//   :648 Typecheck.hs -> hs/Glean/RTS/Term.hs (Typecheck.hs:62-64
//        'import Glean.RTS.Term hiding (...)' + qualified as RTS) -> crate
//        glean-core, glean_core::rts::term::Term (:130; fold_map :238).
//   :649 Typecheck.hs -> hs/Glean/RTS/Types.hs (Typecheck.hs:61
//        'import Glean.RTS.Types as RTS') -> crate glean-core,
//        glean_core::rts::types (Type :250, PidRef :185, Fid :155,
//        ExpandedType :229, eq_type :328, deref_type :397; Type_'s
//        bifold_map lives in angle/types.rs:1493).
//   :650 Typecheck.hs -> util/Glean/Util/Some.hs (Typecheck.hs:67
//        'import Glean.Util.Some' — the Some IsSrcSpan of TcPred and
//        tcPromote) -> the unit-38 decision record, rust/glean-util/src/
//        util/some.rs, ships ZERO items (row 136's per-use-site rule), so
//        this edge DISSOLVES into the concrete-SrcSpan image (row 88,
//        query/typecheck/types.rs:322; DECISIONS.md 2026-10-24).
// IN (5 edges, every consumer's import cite verified):
//   :376 Database/Schema.hs (imports the whole module at :98; calls
//        typecheckDeriving at :881),
//   :566 Query/Derive.hs (:50 'import Glean.Query.Typecheck (tcQueryDeps)'),
//   :617 Query/Prune.hs (:29 the same import),
//   :696 Query/UserQuery.hs (:94 whole-module import; calls typecheck at
//        :980, and imports TcPred at :99),
//   :1560 schema/gen/Glean/Schema/Gen/Main.hs (:59 the tcQueryDeps import).
//   None of the five is shipped; this file is the producer.
//
// CARGO EDGES: none added — rust/glean-db/Cargo.toml has no
// [dependencies] entries yet; every edge is deferred to the Step-4
// workspace scaffold. A shipped sibling (query/typecheck/types.rs:109-114)
// already imports both paths, and Step-4 wiring owns the actual edges.

// -----------------------------------------------------------------------------
// ASSUMED-IMPORT (RULE §4 B4)
// -----------------------------------------------------------------------------
// NONE. Every type this file consumes is owned by a SHIPPED manifest unit
// (the eleven OUT edges above resolve to rust/glean-angle/src/{angle/
// types.rs, display.rs, schema/util.rs}, rust/glean-core/src/rts/{term.rs,
// types.rs} and this crate's database/schema/types.rs,
// query/codegen/types.rs, query/typecheck/{types,monad,unify}.rs). No
// generated code (glean_thrift) and no pinned-compiler newtype image is
// named here: PredicateRef is reached only through PredicateId's public
// field predicate_id_ref (angle/types.rs:933), whose assumed shape row
// 88/angle-types already pinned, and is never imported by name.

// -----------------------------------------------------------------------------
// SIBLING-UNIT CONTRACTS (RULE §4; read, never re-declared)
// -----------------------------------------------------------------------------
//   * SHIPPED row 88 rust/glean-db/src/query/typecheck/types.rs owns
//     TcQuery (:133, tuple struct of 5), TcStatement (:190), TcPat (:245,
//     Term<Match<Typed<TcTerm>, Var>>), TcTerm (:360, the 16-variant
//     enum whose Box-ed payloads this file constructs with Box::new),
//     TcPred (:322 — the 2026-10-24 concrete-SrcSpan ruling),
//     TypecheckedQuery (:587) and lookup_field (:615, borrowed pairs).
//   * SHIPPED row 87 rust/glean-db/src/query/codegen/types.rs owns
//     Match (:378, bifold_map :519), Var (:323), Typed (:574) and
//     QueryWithInfo (:148). Term::fold_map is glean-core's (term.rs:238).
//   * SHIPPED row 89 rust/glean-db/src/database/schema/types.rs owns
//     DbSchema (:568) and PredicateDetails (:756; fields predicate_pid
//     :757, predicate_id :758, predicate_key_type :763,
//     predicate_value_type :764). Neither DbSchema nor PredicateDetails
//     is Clone (the header's DERIVING PARITY, :753-756/:568) — the
//     consequence is the OWNERSHIP divergence below.
//   * SHIPPED row 90 rust/glean-db/src/query/typecheck/monad.rs owns the
//     monad image (T<A> = Result<A, Doc>, :311), TypecheckState (:360),
//     TcEnv (:343), TcOpts (:412), ToRtsType (:321) and the combinators
//     listed in DEPENDENCY TRUTH. Its three obligations to rows 91/92:
//       (a) tc_rts_type is called inline — (st.tc_rts_type)(&ty);
//       (b) tc_free/tc_uses/tc_bindings are std HashSet and row 92 owns
//           the two error-text renderings (the HashSet decision below);
//       (c) a consumer holding &DbSchema cannot build the 'static
//           ToRtsType box; it must OWN the schema (Rc/Arc) — see the
//           OWNERSHIP divergence, which is where this file departs.
//   * SHIPPED row 91 rust/glean-db/src/query/typecheck/unify.rs owns
//     unify (:324), apply (:696, &mut TypecheckState), zonkVars (:925)
//     and zonkTcQuery (:974). Its zonk family only READS state (verified:
//     the only st. uses in :1015-:1480 are reads of tc_display_opts and
//     tc_vars), which is what makes this file's :108 catch handler
//     observable-equivalent under row 90's no-rollback image.
//   * CONTRACT (row 92 -> rows 98/144): typecheck_deriving takes TcEnv,
//     PredicateDetails and ToRtsType BY VALUE, yet Schema.hs reuses all
//     three across a mapM (:872 env, :881 the call, :884 the returned
//     details{...}, :887 mapM tcDeriving). TcEnv/PredicateDetails are not
//     Clone and ToRtsType is a Box<dyn Fn>, so the caller's loop is
//     inexpressible until a CENTRAL row-89/90 change lands (Arc-shared
//     TcEnv maps / Rc<DbSchema> / a &-taking initialTypecheckState). See
//     the TODO(port) at typecheck_deriving; this is not a local patch.
//
// CATCH SITES AND ROW 90'S STATE-ROLLBACK TODO (Typecheck.hs:108/:132):
//   * :108 catches ONLY zonkTcQuery q (:107); zonkVars (:106) is OUTSIDE
//     it. The handler reads tcDisplayOpts and tcSubst (via apply retTy)
//     and then fails. The caught action only reads, so the &mut image's
//     missing rollback is unobservable HERE. The handler also WRITES via
//     needsResult's freshVariable (:188), but that write happens
//     after the failure and the source's handler runs against the
//     pre-action state — identical because the action wrote nothing. No
//     stop-and-report condition.
//   * :132 wraps the whole deriving body and immediately re-throws with
//     the "In <pred>:" prefix; the state is discarded on both paths
//     (typecheckDeriving returns only d, :175). Unobservable.
//
// THE HASHSET DECISION (row 90's inherited TODO; row 92 is OWNER):
//   The source renders HashSet.toList into two error texts
//   (Typecheck.hs:817 free-variable list, :828 unbound-variable list).
//   unordered-containers' order is a function of its hashWithSalt chain,
//   which no shipped unit ports; std HashSet is a DIFFERENT hash order.
//   DECISION (a): keep std HashSet and carry the open question with a
//   named owner — the Step-6 judge (the diagnostics are golden-compared
//   there) or a future hashing unit that ports the chain. Sorting here
//   was rejected as option (b): it would be a third order, no closer to
//   the source. The exact user-visible difference is name-list ORDER
//   only; membership, separators and message text are identical. The
//   open question is flagged once, at free_variables_are_errors.
//
// THE OWNERSHIP DIVERGENCE (typecheck's DbSchema parameter):
//   The source shares the immutable DbSchema and its caller keeps using
//   it after typechecking (UserQuery.hs:982-999 calls flatten/reorder
//   with the same schema). This file CONSUMES it: TcEnv (row 90, frozen)
//   owns the two maps and neither PredicateDetails nor TypeDetails is
//   Clone, so moving the maps out of an owned schema is the only
//   type-coherent image. Flagged once at typecheck; the caller-side
//   resolution is named there. Inside this file there is no semantic
//   difference — the maps are the same maps.
//
// THE MONOMORPHIC-SPAN DECISION (DECISIONS.md 2026-10-24):
//   The source is IsSrcSpan s-polymorphic. The TcPred and tcPromote
//   images are concrete (PidRef, SrcSpan) / (Type, Type, SrcSpan)
//   (row 88 :322, row 90 :403), and both production callers instantiate
//   at SrcSpan (UserQuery.hs:980, Schema.hs:881). This file therefore
//   fixes s = SrcSpan (the Pat<ST> aliases below) — a generic s could not
//   build the state fields. The ST parameter (the span type inside
//   TypeSignature) stays generic; it is erased by rm_loc_type before it
//   reaches ToRtsType. ST: Clone is an added bound: the source shares
//   patterns freely, Rust must clone them in needsResult/ignoreResult,
//   and Type_<ST,..> is only Clone when ST is. The only ST instantiated
//   by the tree is () (a Clone type).
//
// KNOWN NARROW DIVERGENCE (checkVarCase's isUpper):
//   Data.Char.isUpper is the Unicode Lu category only. Rust's
//   char::is_uppercase additionally covers Other_Uppercase (e.g. U+2160
//   ROMAN NUMERAL ONE), so a variable name starting with such a character
//   is accepted here and rejected by the source. No Unicode-data table is
//   shipped (no manifest unit owns one); flagged at check_var_case.
//
// CLONE NOTE: Haskell sharing becomes cloning at the TcPat/Var/Type
// boundaries (Var::clone, Type::clone, TcPat::clone). These are
// structural, not semantic; the two whole-list clones (needs_result's
// statement list and resolve_promote's tc_promote snapshot) are covered
// by the single PERF marker at needs_result. No PERF marker is added per
// field clone — that would bury the review.

// -----------------------------------------------------------------------------
// EXPORTS AND PRIVACY
// -----------------------------------------------------------------------------
// Exactly the source's ten names (Typecheck.hs:15-26):
//   typecheck, typecheckDeriving, ToRtsType (re-export of row 90's alias),
//   TcEnv(..), TcOpts(..), defaultTcOpts, emptyTcEnv, tcQueryDeps,
//   tcQueryUsesNegation, UseOfNegation(..).
// Everything else is private: Typecheck.hs defines 44 top-level functions
// and exports 4 of them (the other five export names — ToRtsType, TcEnv,
// TcOpts, defaultTcOpts, emptyTcEnv — are Monad.hs re-exports; UseOfNegation
// is a data type defined here), so its 40 private ones stay private here.
// The aliases Pat/Statement/Query/DerivingInfoPrime, Context, VarSet and
// every helper — including typecheck_body, typecheck_deriving_body and
// last_statement_error — are module-private.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use glean_angle::angle::types::{
    rm_loc_type, source_pat_span, span_between, DerivingInfo, Field, FieldName, IsSrcSpan, Name,
    Ordered, PredicateId, PrimOp, RecordOrSum, SeekSection, SourcePat_, SourceQuery_,
    SourceStatement_, SrcSpan, TypeId, Type_,
};
use glean_angle::display::{
    append_space, display_default, hang, hsep, nest, punctuate, vcat, Display, Doc,
};
use glean_angle::schema::util::{lower_maybe, tuple_schema, unit};
use glean_core::rts::term::Term;
use glean_core::rts::types::{deref_type, eq_type, ExpandedType, Fid, PidRef, Type};

use crate::database::schema::types::{DbSchema, PredicateDetails};
use crate::query::codegen::types::{Match, QueryWithInfo, Typed, Var};
use crate::query::typecheck::monad::{
    add_err_span, fresh_ty_var, fresh_ty_var_int, get_predicate_details, in_pat,
    initial_typecheck_state, mk_wild, pretty_error, pretty_error_at, pretty_error_in, when_debug,
    TcMode, TypecheckState, T,
};
use crate::query::typecheck::types::{
    lookup_field, TcPat, TcPred, TcQuery, TcStatement, TcTerm, TypecheckedQuery,
};
use crate::query::typecheck::unify::{apply, unify, zonk_tc_query, zonk_vars};

// The source's export list, minus the re-exported monad vocabulary.
pub use crate::query::typecheck::monad::{default_tc_opts, empty_tc_env, TcEnv, TcOpts, ToRtsType};

// -----------------------------------------------------------------------------
// Source-type aliases (Typecheck.hs:69-72), specialised at SrcSpan
// -----------------------------------------------------------------------------

/// 'type Pat' s st = SourcePat_ s st PredicateId TypeId' (Typecheck.hs:69),
/// at s = SrcSpan per the MONOMORPHIC-SPAN decision.
type Pat<ST> = SourcePat_<SrcSpan, ST, PredicateId, TypeId>;

/// 'type Statement' s st = SourceStatement_ s st PredicateId TypeId'
/// (Typecheck.hs:70).
type Statement<ST> = SourceStatement_<SrcSpan, ST, PredicateId, TypeId>;

/// 'type Query' s st = SourceQuery_ s st PredicateId TypeId'
/// (Typecheck.hs:71).
type Query<ST> = SourceQuery_<SrcSpan, ST, PredicateId, TypeId>;

/// 'type DerivingInfo' s st = DerivingInfo (Query' s st)'
/// (Typecheck.hs:72). Named with a Prime suffix because DerivingInfo is
/// the imported angle type.
type DerivingInfoPrime<ST> = DerivingInfo<Query<ST>>;

/// 'data Context = ContextExpr | ContextPat' (Typecheck.hs:267).
///
/// The source derives nothing; Copy is added because Haskell values are
/// freely copyable and every call site passes it by value (the term.rs:127
/// precedent). No Eq is added: the source never compares a Context
/// (bindOrUse pattern-matches, :757-761).
#[derive(Clone, Copy)]
enum Context {
    ContextExpr,
    ContextPat,
}

// -----------------------------------------------------------------------------
// typecheck (Typecheck.hs:77-116)
// -----------------------------------------------------------------------------

/// Typecheck a 'SourceQuery' which is in terms of schema types and turn it
/// into a 'TypecheckedQuery' which is in terms of raw Terms, ready for
/// compiling to bytecode. (Source haddock, Typecheck.hs:74-76.)
///
/// SIGNATURE NOTE (the source's boundary types): the source returns
/// @ExceptT Text IO (TypecheckedQuery, [TcPred])@ and packs the Doc with
/// @withExceptT (Text.pack . show)@ (:92). The image is the house
/// @Result<_, String>@ escape convention (schema/resolve.rs:256, the
/// shipped precedent), rendered by the private show_doc below — the same
/// stand-in as monad.rs's, with the same documented flat-branch caveat.
/// The IO layer itself has no counterpart here: the body's only IO is the
/// when_debug eprintln! (typecheck_body; likewise resolve_promote_loop),
/// matching the source's whenDebug hPutStrLn (:100/:1278).
///
/// BORROWED ARGUMENTS: query is taken by & although the source takes it by
/// value — the source SHARES it across varsQuery, inferQuery and (in the
/// :108 handler) needsResult, and the caller keeps it; a by-value Rust
/// parameter could not be used three times without clones.
///
/// TODO(port): DbSchema is CONSUMED (see THE OWNERSHIP DIVERGENCE). The
/// source shares it and UserQuery.hs:982-999 keeps using it after the
/// call (flatten and reorder take the same dbSchema), so "pass an owned
/// schema" is not a resolution for that caller — the honest fix is
/// Rc/Arc<DbSchema>, or a row-89/90 change that lets TcEnv borrow the
/// maps: PredicateDetails is not Clone
/// (database/schema/types.rs:753-756), so a shared borrow cannot build
/// the frozen TcEnv as it stands. Owner: the first shipped caller
/// (UserQuery, manifest row 144; Database/Schema.hs, row 98) — see the
/// report.
pub fn typecheck<ST: Clone>(
    db_schema: DbSchema,
    opts: &TcOpts,
    rts_type: ToRtsType,
    query: &Query<ST>,
) -> Result<(TypecheckedQuery, Vec<TcPred>), String> {
    // let tcEnv = TcEnv { tcEnvPredicates = predicatesById dbSchema
    //                   , tcEnvTypes = typesById dbSchema }
    let DbSchema {
        predicates_by_id,
        types_by_id,
        ..
    } = db_schema;
    let tc_env = TcEnv {
        tc_env_predicates: predicates_by_id,
        tc_env_types: types_by_id,
    };
    let mut st = initial_typecheck_state(tc_env, opts, rts_type, TcMode::TcModeQuery);
    typecheck_body(&mut st, query).map_err(|doc| show_doc(&doc))
}

fn typecheck_body<ST: Clone>(
    st: &mut TypecheckState,
    query: &Query<ST>,
) -> T<(TypecheckedQuery, Vec<TcPred>)> {
    // modify $ \s -> s { tcVisible = varsQuery query mempty }
    st.tc_visible = vars_query(query, HashSet::new());

    // q@(TcQuery retTy _ _ _ _) <- inferQuery ContextPat query
    //   <* freeVariablesAreErrors <* unboundVariablesAreErrors
    let pre = infer_query(st, Context::ContextPat, query)?;
    free_variables_are_errors(st)?;
    unbound_variables_are_errors(st)?;
    // resolvePromote
    resolve_promote(st)?;

    // whenDebug $ liftIO $ hPutStrLn stderr $ show $ vcat [...]
    when_debug(st, |st| {
        let mut subst_lines = Vec::with_capacity(st.tc_subst.len());
        for (n, ty) in st.tc_subst.iter() {
            subst_lines.push(
                Doc::text(n.to_string())
                    .append(Doc::text(" := "))
                    .append(display_default(ty)),
            );
        }
        let doc = vcat(vec![
            Doc::text("subst:"),
            indent(2, vcat(subst_lines)),
            Doc::text("query: ").append(display_default(&pre)),
        ]);
        eprintln!("{}", show_doc(&doc));
        Ok(())
    })?;

    // zonkVars
    zonk_vars(st)?;

    // zonkTcQuery q, catching the error and reporting the ambiguous type
    let pre_ret_ty = pre.0.clone();
    let q = match zonk_tc_query(st, &pre) {
        Ok(q) => q,
        Err(_) => {
            // (head,_,_) <- needsResult query
            let (head, _, _) = needs_result(st, query)?;
            // opts <- gets tcDisplayOpts
            let opts = st.tc_display_opts;
            // retTy' <- apply retTy
            let ret_ty = apply(st, &pre_ret_ty)?;
            return pretty_error_at(
                source_pat_span(&head),
                vcat(vec![
                    Doc::text("query has ambiguous type"),
                    indent(4, Doc::text("type: ").append(ret_ty.display(&opts))),
                ]),
            );
        }
    };
    // return (QueryWithInfo q tcNextVar Nothing ty, tcPreds)
    let ty = q.0.clone();
    let num_vars = st.tc_next_var;
    Ok((
        QueryWithInfo {
            qi_query: q,
            qi_num_vars: num_vars,
            qi_generator: None,
            qi_return_type: ty,
        },
        std::mem::take(&mut st.tc_preds),
    ))
}

// -----------------------------------------------------------------------------
// typecheckDeriving (Typecheck.hs:119-175)
// -----------------------------------------------------------------------------

/// Typecheck the query for a derived predicate. (Source haddock :118.)
///
/// The whole body is wrapped in the source's
/// @flip catchError (\e -> throwError $ vcat ["In " <> pretty (predicateIdRef
/// predicateId) <> ":", indent 2 e])@ (:131-135) — applied here as a
/// map_err on the body's outcome, which is exactly that handler: it
/// rewrites the error channel only, and the success value is untouched.
///
/// TODO(port): this signature takes TcEnv, PredicateDetails and ToRtsType
/// BY VALUE, but the source's only caller reuses all three across a mapM
/// (Schema.hs:872 @env = TcEnv@, :881 the call, :884 the returned
/// @details{...}@, :887 @mapM tcDeriving@). TcEnv/PredicateDetails are
/// not Clone and ToRtsType is a Box<dyn Fn>, so that loop is
/// inexpressible as written. The resolution must be CENTRAL, at row 89/90
/// (Arc-shared TcEnv maps / Rc<DbSchema> / a &-taking
/// initialTypecheckState), not a local patch in this file. Owners: the
/// Database/Schema.hs unit (manifest row 98) and the UserQuery unit
/// (row 144) — see SIBLING-UNIT CONTRACTS.
pub fn typecheck_deriving<ST: Clone>(
    tc_env: TcEnv,
    opts: &TcOpts,
    rts_type: ToRtsType,
    predicate_details: PredicateDetails,
    deriving_info: DerivingInfoPrime<ST>,
) -> Result<DerivingInfo<TypecheckedQuery>, String> {
    // typecheckDeriving tcEnv opts rtsType PredicateDetails{..} derivingInfo
    let PredicateDetails {
        predicate_id,
        predicate_key_type,
        predicate_value_type,
        ..
    } = predicate_details;
    typecheck_deriving_body(
        tc_env,
        opts,
        rts_type,
        &predicate_key_type,
        &predicate_value_type,
        deriving_info,
    )
    .map_err(|e| {
        show_doc(&vcat(vec![
            Doc::text("In ")
                .append(display_default(&predicate_id.predicate_id_ref))
                .append(Doc::text(":")),
            indent(2, e),
        ]))
    })
}

fn typecheck_deriving_body<ST: Clone>(
    tc_env: TcEnv,
    opts: &TcOpts,
    rts_type: ToRtsType,
    predicate_key_type: &Type,
    predicate_value_type: &Type,
    deriving_info: DerivingInfoPrime<ST>,
) -> T<DerivingInfo<TypecheckedQuery>> {
    let mut st = initial_typecheck_state(tc_env, opts, rts_type, TcMode::TcModePredicate);
    match deriving_info {
        // NoDeriving -> return NoDeriving
        DerivingInfo::NoDeriving => Ok(DerivingInfo::NoDeriving),
        // Derive deriveWhen q -> do
        DerivingInfo::Derive(derive_when, q) => {
            // modify $ \s -> s { tcVisible = varsQuery q mempty }
            st.tc_visible = vars_query(&q, HashSet::new());

            // we typecheck the pattern first, because we know its type.
            // (head, stmts, ord) <- needsResult q
            let (head, stmts, ord) = needs_result(&mut st, &q)?;

            // let (key, maybeVal) = case head of ...
            let (key, maybe_val) = match &head {
                SourcePat_::KeyValue(_, key, val) => {
                    (key.as_ref().clone(), Some(val.as_ref().clone()))
                }
                SourcePat_::App(_, f, xs) => {
                    if let SourcePat_::KeyValue(_, key, val) = f.as_ref() {
                        // let end | null xs = val | otherwise = last xs
                        let end: &Pat<ST> = if xs.is_empty() {
                            val.as_ref()
                        } else {
                            xs.last().expect("non-empty")
                        };
                        // span = spanBetween (sourcePatSpan val) (sourcePatSpan end)
                        let span = span_between(source_pat_span(val), source_pat_span(end));
                        (
                            key.as_ref().clone(),
                            Some(SourcePat_::App(span, val.clone(), xs.clone())),
                        )
                    } else {
                        // _other -> (head, Nothing)
                        (head.clone(), None)
                    }
                }
                _other => (head.clone(), None),
            };

            // key' <- typecheckPattern ContextPat predicateKeyType key
            let key2 = typecheck_pattern(&mut st, Context::ContextPat, predicate_key_type, &key)?;

            // maybeVal' <- case maybeVal of ...
            let maybe_val2 = match &maybe_val {
                None => {
                    if eq_type(
                        opts.tc_opt_angle_version,
                        &unit::<(), PidRef, ExpandedType>(),
                        predicate_value_type,
                    ) {
                        None
                    } else {
                        return pretty_error_in(
                            &head,
                            nest(
                                4,
                                vcat(vec![
                                    Doc::text("a functional predicate must return a value,"),
                                    Doc::text(
                                        "i.e. the query should have the form 'X -> Y where ..",
                                    ),
                                ]),
                            ),
                        );
                    }
                }
                Some(val) => Some(typecheck_pattern(
                    &mut st,
                    Context::ContextPat,
                    predicate_value_type,
                    val,
                )?),
            };

            // stmts' <- mapM typecheckStatement stmts
            let mut stmts2 = Vec::with_capacity(stmts.len());
            for s in stmts.iter() {
                stmts2.push(typecheck_statement(&mut st, s)?);
            }

            free_variables_are_errors(&mut st)?;
            unbound_variables_are_errors(&mut st)?;
            resolve_promote(&mut st)?;
            zonk_vars(&mut st)?;

            // q <- zonkTcQuery (TcQuery predicateKeyType key' maybeVal' stmts' ord)
            let q = zonk_tc_query(
                &mut st,
                &TcQuery(predicate_key_type.clone(), key2, maybe_val2, stmts2, ord),
            )?;
            // nextVar <- gets tcNextVar
            let next_var = st.tc_next_var;
            // return $ Derive deriveWhen $
            //   QueryWithInfo q nextVar Nothing predicateKeyType
            Ok(DerivingInfo::Derive(
                derive_when,
                QueryWithInfo {
                    qi_query: q,
                    qi_num_vars: next_var,
                    qi_generator: None,
                    qi_return_type: predicate_key_type.clone(),
                },
            ))
        }
    }
}

// -----------------------------------------------------------------------------
// needsResult (Typecheck.hs:177-199)
// -----------------------------------------------------------------------------

/// @needsResult :: Query' s st -> T (Pat' s st, [Statement' s st], Ordered)@
/// (:177-179) — find the query's result pattern, synthesising a temporary
/// variable when the last statement is a bare wildcard.
///
/// PERF(port): the source SHARES the statement list (Haskell lists are
/// persistent); each arm here clones it, and the wildcard arm rebuilds it.
/// needsResult is called at three sites per run (typecheck_body, the :108
/// handler, typecheck_deriving_body) and its result is immediately walked,
/// so the clone is structural, not semantic. Not hoisted to an Rc/Arc
/// because the lists are short. The same deferral covers resolve_promote's
/// @st.tc_promote.clone()@ snapshot (the source walks the field in place).
fn needs_result<ST: Clone>(
    st: &mut TypecheckState,
    q: &Query<ST>,
) -> T<(Pat<ST>, Vec<Statement<ST>>, Ordered)> {
    match q {
        // needsResult (SourceQuery (Just p) stmts ord) = return (p, stmts, ord)
        SourceQuery_ {
            src_query_head: Some(p),
            src_query_stmts: stmts,
            src_query_ordered: ord,
        } => Ok((p.clone(), stmts.clone(), *ord)),
        SourceQuery_ {
            src_query_head: None,
            src_query_stmts: stmts,
            src_query_ordered: ord,
        } => {
            // case reverse stmts of ...
            let rev: Vec<&Statement<ST>> = stmts.iter().rev().collect();
            match rev.as_slice() {
                // (SourceStatement (Variable s v) _ : _) ->
                //   return (Variable s v, stmts, ord)
                [SourceStatement_(SourcePat_::Variable(s, v), _), ..] => {
                    Ok((SourcePat_::Variable(*s, v.clone()), stmts.clone(), *ord))
                }
                // [SourceStatement Wildcard{} rhs] -> return (rhs, [], ord)
                [SourceStatement_(SourcePat_::Wildcard(_), rhs)] => {
                    Ok((rhs.clone(), Vec::new(), *ord))
                }
                // (SourceStatement (Wildcard s) rhs : rstmts) -> do
                //   tmp <- freshVariable s
                //   return (tmp, reverse (SourceStatement rhs tmp : rstmts), ord)
                [SourceStatement_(SourcePat_::Wildcard(s), rhs), rest @ ..] => {
                    let tmp = fresh_variable(st, s)?;
                    let mut new_stmts: Vec<Statement<ST>> =
                        rest.iter().rev().map(|x| (**x).clone()).collect();
                    new_stmts.push(SourceStatement_(rhs.clone(), tmp.clone()));
                    Ok((tmp, new_stmts, *ord))
                }
                // (SourceStatement pat _ : _) -> prettyErrorIn pat =<< err
                [SourceStatement_(pat, _), ..] => {
                    let doc = last_statement_error(st, q);
                    pretty_error_in(pat, doc)
                }
                // _ -> prettyError =<< err
                [] => pretty_error(last_statement_error(st, q)),
            }
        }
    }
}

/// The @err@ local of needsResult (:195-198):
/// @opts <- gets tcDisplayOpts; return $ "the last statement should be an
/// expression: " <> display opts q@.
fn last_statement_error<ST>(st: &TypecheckState, q: &Query<ST>) -> Doc {
    let opts = st.tc_display_opts;
    Doc::text("the last statement should be an expression: ").append(q.display(&opts))
}

// -----------------------------------------------------------------------------
// ignoreResult (Typecheck.hs:201-215)
// -----------------------------------------------------------------------------

/// Add a unit result if the pattern doesn't have a result. (Source comment
/// :200.) Note the source's @where@: @empty@ is built from the ORIGINAL
/// pattern's span (@fullSpan = sourcePatSpan p@) even in the NestedQuery
/// arm, whose own span @s@ is a different value — preserved.
fn ignore_result<ST: Clone>(p: &Pat<ST>) -> Pat<ST> {
    let full_span = *source_pat_span(p);
    // startPos = mkSpan (startLoc fullSpan) (startLoc fullSpan)
    let start_pos = SrcSpan::mk_span(full_span.start_loc(), full_span.start_loc());
    // empty = TypeSignature startPos (Tuple startPos []) unit
    let empty = SourcePat_::TypeSignature(
        start_pos,
        Box::new(SourcePat_::Tuple(start_pos, Vec::new())),
        Box::new(unit::<ST, PredicateId, TypeId>()),
    );
    match p {
        // OrPattern s a b -> OrPattern s (ignoreResult a) (ignoreResult b)
        SourcePat_::OrPattern(s, a, b) => {
            SourcePat_::OrPattern(*s, Box::new(ignore_result(a)), Box::new(ignore_result(b)))
        }
        // IfPattern s a b c -> IfPattern s a (ignoreResult b) (ignoreResult c)
        SourcePat_::IfPattern {
            span,
            cond,
            then_,
            else_,
        } => SourcePat_::IfPattern {
            span: *span,
            cond: cond.clone(),
            then_: Box::new(ignore_result(then_)),
            else_: Box::new(ignore_result(else_)),
        },
        // NestedQuery s (SourceQuery Nothing stmts ord) ->
        //   NestedQuery s (SourceQuery (Just empty) stmts ord)
        SourcePat_::NestedQuery(s, q) if q.src_query_head.is_none() => SourcePat_::NestedQuery(
            *s,
            Box::new(SourceQuery_ {
                src_query_head: Some(empty),
                src_query_stmts: q.src_query_stmts.clone(),
                src_query_ordered: q.src_query_ordered,
            }),
        ),
        // other -> NestedQuery fullSpan
        //   (SourceQuery (Just empty) [SourceStatement (Wildcard fullSpan) other] Unordered)
        other => SourcePat_::NestedQuery(
            full_span,
            Box::new(SourceQuery_ {
                src_query_head: Some(empty),
                src_query_stmts: vec![SourceStatement_(
                    SourcePat_::Wildcard(full_span),
                    other.clone(),
                )],
                src_query_ordered: Ordered::Unordered,
            }),
        ),
    }
}

// -----------------------------------------------------------------------------
// inferQuery / typecheckQuery / unexpectedValue (Typecheck.hs:217-237)
// -----------------------------------------------------------------------------

/// @inferQuery ctx q@ (:217-222) — statement order is load-bearing:
/// statements are typechecked BEFORE the head.
fn infer_query<ST: Clone>(st: &mut TypecheckState, ctx: Context, q: &Query<ST>) -> T<TcQuery> {
    let (head, stmts, ord) = needs_result(st, q)?;
    let mut stmts2 = Vec::with_capacity(stmts.len());
    for s in stmts.iter() {
        stmts2.push(typecheck_statement(st, s)?);
    }
    let (head2, ty) = infer_expr(st, ctx, &head)?;
    Ok(TcQuery(ty, head2, None, stmts2, ord))
}

/// @typecheckQuery ctx ty q@ (:224-233) — head BEFORE statements.
fn typecheck_query<ST: Clone>(
    st: &mut TypecheckState,
    ctx: Context,
    ty: &Type,
    q: &Query<ST>,
) -> T<TcQuery> {
    let (head, stmts, ord) = needs_result(st, q)?;
    let head2 = typecheck_pattern(st, ctx, ty, &head)?;
    let mut stmts2 = Vec::with_capacity(stmts.len());
    for s in stmts.iter() {
        stmts2.push(typecheck_statement(st, s)?);
    }
    Ok(TcQuery(ty.clone(), head2, None, stmts2, ord))
}

/// @unexpectedValue pat@ (:236-237) — polymorphic in its result, as the
/// source's @T b@ is (it is used at both TcPat and (TcPat, Type)).
fn unexpected_value<ST, A>(pat: &Pat<ST>) -> T<A> {
    pretty_error_in(
        pat,
        Doc::text("a key/value pattern (X -> Y) cannot be used here"),
    )
}

// -----------------------------------------------------------------------------
// typecheckStatement (Typecheck.hs:240-263)
// -----------------------------------------------------------------------------

fn typecheck_statement<ST: Clone>(st: &mut TypecheckState, stmt: &Statement<ST>) -> T<TcStatement> {
    let SourceStatement_(lhs, rhs0) = stmt;
    // let ignoreTopResult (OrPattern s a b) = ...
    //     ignoreTopResult (IfPattern s a b c) = ...
    //     ignoreTopResult other = other
    let ignore_top_result = |p: &Pat<ST>| -> Pat<ST> {
        match p {
            SourcePat_::OrPattern(s, a, b) => {
                SourcePat_::OrPattern(*s, Box::new(ignore_result(a)), Box::new(ignore_result(b)))
            }
            SourcePat_::IfPattern {
                span,
                cond,
                then_,
                else_,
            } => SourcePat_::IfPattern {
                span: *span,
                cond: cond.clone(),
                then_: Box::new(ignore_result(then_)),
                else_: Box::new(ignore_result(else_)),
            },
            other => other.clone(),
        }
    };
    // rhs | Wildcard _ <- lhs = ignoreTopResult rhs0 | otherwise = rhs0
    let rhs = if matches!(lhs, SourcePat_::Wildcard(_)) {
        ignore_top_result(rhs0)
    } else {
        rhs0.clone()
    };
    let (rhs2, ty) = infer_expr(st, Context::ContextPat, &rhs)?;
    let lhs2 = typecheck_pattern(st, Context::ContextPat, &ty, lhs)?;
    Ok(TcStatement(ty, lhs2, rhs2))
}

// -----------------------------------------------------------------------------
// inferExpr (Typecheck.hs:274-432) — arm order, guards and messages are the
// source's; the catch-all is last.
// -----------------------------------------------------------------------------

/// For the term built by a Query, we have no local type information, so we
/// attempt to infer the type of the expression. Therefore we don't allow
/// structs or alts. Variables must be occurrences (because this is a term,
/// not a pattern), and Wildcards are disallowed. (Source comment :269-273.)
fn infer_expr<ST: Clone>(st: &mut TypecheckState, ctx: Context, pat: &Pat<ST>) -> T<(TcPat, Type)> {
    match pat {
        // Nat _ w -> promote (sourcePatSpan pat) (RTS.Nat w) NatTy
        SourcePat_::Nat(_, w) => promote(st, source_pat_span(pat), Term::Nat(*w), Type_::NatTy),
        // String _ s -> promote ... (RTS.String (Text.encodeUtf8 s)) StringTy
        SourcePat_::String(_, s) => promote(
            st,
            source_pat_span(pat),
            Term::String(s.as_bytes().to_vec()),
            Type_::StringTy,
        ),
        // StringPrefix _ s -> promote ... (RTS.Ref (MatchPrefix ...)) StringTy
        SourcePat_::StringPrefix(_, s) => promote(
            st,
            source_pat_span(pat),
            Term::Ref(Match::MatchPrefix(
                s.as_bytes().to_vec(),
                Box::new(mk_wild(Type_::StringTy)),
            )),
            Type_::StringTy,
        ),
        // ByteArray _ b -> promote ... (RTS.ByteArray b) (ArrayTy ByteTy)
        SourcePat_::ByteArray(_, b) => promote(
            st,
            source_pat_span(pat),
            Term::ByteArray(b.clone()),
            Type_::ArrayTy(Box::new(Type_::ByteTy)),
        ),
        // (App _ (StringPrefix _ s) [pat]) -> do
        //   rest <- typecheckPattern ctx StringTy pat
        //   promote (sourcePatSpan pat) (RTS.Ref (MatchPrefix .. rest)) StringTy
        //
        // NOTE: the source's inner @pat@ SHADOWS the function argument, so
        // the promote span is the ARGUMENT's span, not the App's.
        SourcePat_::App(_, f, xs)
            if xs.len() == 1 && matches!(f.as_ref(), SourcePat_::StringPrefix(..)) =>
        {
            let SourcePat_::StringPrefix(_, s) = f.as_ref() else {
                unreachable!()
            };
            let inner = &xs[0];
            let rest = typecheck_pattern(st, ctx, &Type_::StringTy, inner)?;
            promote(
                st,
                source_pat_span(inner),
                Term::Ref(Match::MatchPrefix(s.as_bytes().to_vec(), Box::new(rest))),
                Type_::StringTy,
            )
        }
        // Tuple _ ts -> do (ts,tys) <- unzip <$> mapM (inferExpr ctx) ts
        //   promote ... (RTS.Tuple ts) (tupleSchema tys)
        SourcePat_::Tuple(_, ts) => {
            let mut pats = Vec::with_capacity(ts.len());
            let mut tys = Vec::with_capacity(ts.len());
            for t in ts.iter() {
                let (p, ty) = infer_expr(st, ctx, t)?;
                pats.push(p);
                tys.push(ty);
            }
            promote(
                st,
                source_pat_span(pat),
                Term::Tuple(pats),
                tuple_schema(tys),
            )
        }
        // Array _ [] -> do x <- freshTyVar; promote ... (RTS.Array []) (ArrayTy x)
        SourcePat_::Array(_, ts) if ts.is_empty() => {
            let x = fresh_ty_var(st)?;
            promote(
                st,
                source_pat_span(pat),
                Term::Array(Vec::new()),
                Type_::ArrayTy(Box::new(x)),
            )
        }
        // Array _ (t:ts) -> do (t',ty) <- inferExpr ctx t
        //   ts' <- mapM (typecheckPattern ctx ty) ts
        //   promote ... (RTS.Array (t':ts')) (ArrayTy ty)
        SourcePat_::Array(_, ts) => {
            let (t2, ty) = infer_expr(st, ctx, &ts[0])?;
            let mut ts2 = Vec::with_capacity(ts.len() - 1);
            for t in ts[1..].iter() {
                ts2.push(typecheck_pattern(st, ctx, &ty, t)?);
            }
            ts2.insert(0, t2);
            promote(
                st,
                source_pat_span(pat),
                Term::Array(ts2),
                Type_::ArrayTy(Box::new(ty)),
            )
        }
        // ArrayPrefix _ (t:|ts) -> do (t',ty) <- inferExpr ctx t
        //   ts' <- mapM (typecheckPattern ctx ty) ts
        //   let aty = ArrayTy ty
        //       tcPat = RTS.Ref (MatchArrayPrefix ty (t':ts') (RTS.Ref (MatchWild aty)))
        //   promote ... tcPat aty
        SourcePat_::ArrayPrefix(_, pre) => {
            let (t2, ty) = infer_expr(st, ctx, &pre.head)?;
            let mut ts2 = Vec::with_capacity(pre.tail.len());
            for t in pre.tail.iter() {
                ts2.push(typecheck_pattern(st, ctx, &ty, t)?);
            }
            ts2.insert(0, t2);
            let aty = Type_::ArrayTy(Box::new(ty.clone()));
            let tc_pat = Term::Ref(Match::MatchArrayPrefix(
                ty,
                ts2,
                Box::new(Term::Ref(Match::MatchWild(aty.clone()))),
            ));
            promote(st, source_pat_span(pat), tc_pat, aty)
        }
        // Variable span name -> inferVar ctx span name
        SourcePat_::Variable(span, name) => infer_var(st, ctx, span, name),
        // Prim span primOp args -> ...
        SourcePat_::Prim(span, prim_op, args) => {
            let (prim_arg_tys, ret_ty) = prim_op_type(st, *prim_op)?;
            check_prim_op_args(span, args, &prim_arg_tys)?;
            let mut args2 = Vec::with_capacity(args.len());
            for (ty, arg) in prim_arg_tys.iter().zip(args.iter()) {
                args2.push(typecheck_pattern(st, Context::ContextExpr, ty, arg)?);
            }
            Ok((
                Term::Ref(Match::MatchExt(Typed(
                    ret_ty.clone(),
                    TcTerm::TcPrimCall(*prim_op, args2),
                ))),
                ret_ty,
            ))
        }
        // Clause _ prSpan pred pat range -> tcFactGenerator pred pat range (Just prSpan)
        SourcePat_::Clause(_, pr_span, pred, inner, range) => {
            tc_fact_generator(st, pred, inner, *range, Some(*pr_span))
        }
        // OrPattern _ a b -> disjunction ... ; Typed ty (TcOr a' b')
        SourcePat_::OrPattern(_, a, b) => {
            let vars_a = vars_pat(a, HashSet::new());
            let vars_b = vars_pat(b, HashSet::new());
            let ((a2, ty), b2) = disjunction(
                st,
                vars_a,
                |st| infer_expr(st, ctx, a),
                vars_b,
                |st, at: &(TcPat, Type)| typecheck_pattern(st, ctx, &at.1, b),
            )?;
            Ok((
                Term::Ref(Match::MatchExt(Typed(
                    ty.clone(),
                    TcTerm::TcOr(Box::new(a2), Box::new(b2)),
                ))),
                ty,
            ))
        }
        // NestedQuery _ q -> do q@(TcQuery ty _ _ _ _) <- inferQuery ctx q
        //   return (Ref (MatchExt (Typed ty (TcQueryGen q))), ty)
        SourcePat_::NestedQuery(_, q) => {
            let q2 = infer_query(st, ctx, q)?;
            let ty = q2.0.clone();
            Ok((
                Term::Ref(Match::MatchExt(Typed(
                    ty.clone(),
                    TcTerm::TcQueryGen(Box::new(q2)),
                ))),
                ty,
            ))
        }
        // Negation _ _ -> (,unit) <$> typecheckPattern ctx unit pat
        SourcePat_::Negation(_, _) => {
            let u = unit::<(), PidRef, ExpandedType>();
            let p = typecheck_pattern(st, ctx, &u, pat)?;
            Ok((p, u))
        }
        // IfPattern _ srcCond srcThen srcElse -> ...
        SourcePat_::IfPattern {
            span: _,
            cond,
            then_,
            else_,
        } => {
            let vars_a = vars_pat(cond, vars_pat(then_, HashSet::new()));
            let vars_b = vars_pat(else_, HashSet::new());
            let ((cond2, then2, ty), else2) = disjunction(
                st,
                vars_a,
                |st| {
                    let cond_pat = ignore_result(cond);
                    let (c, cond_ty) = infer_expr(st, ctx, &cond_pat)?;
                    let (t, then_ty) = infer_expr(st, ctx, then_)?;
                    Ok((Typed(cond_ty, c), t, then_ty))
                },
                vars_b,
                |st, at: &(Typed<TcPat>, TcPat, Type)| typecheck_pattern(st, ctx, &at.2, else_),
            )?;
            Ok((
                Term::Ref(Match::MatchExt(Typed(
                    ty.clone(),
                    TcTerm::TcIf {
                        cond: Box::new(cond2),
                        then_: Box::new(then2),
                        else_: Box::new(else2),
                    },
                ))),
                ty,
            ))
        }
        // ElementsOfArray _ e -> ...
        SourcePat_::ElementsOfArray(_, e) => {
            let (e2, ty) = infer_expr(st, Context::ContextExpr, e)?;
            let elem_ty = fresh_ty_var(st)?;
            let container_ty = fresh_ty_var_int(st)?;
            unify(
                st,
                &ty,
                &Type_::ElementsOf(Box::new(elem_ty.clone()), container_ty),
            )?;
            Ok((
                Term::Ref(Match::MatchExt(Typed(
                    elem_ty.clone(),
                    TcTerm::TcElementsUnresolved(Type_::TyVar(container_ty), Box::new(e2)),
                ))),
                elem_ty,
            ))
        }
        // Elements _ e -> ... case ty of SetTy elemTy -> ... | _other -> error
        SourcePat_::Elements(_, e) => {
            let (e2, ty) = infer_expr(st, Context::ContextExpr, e)?;
            match &ty {
                Type_::SetTy(elem_ty) => {
                    let elem_ty = elem_ty.as_ref().clone();
                    Ok((
                        Term::Ref(Match::MatchExt(Typed(
                            elem_ty.clone(),
                            TcTerm::TcElementsOfSet(Box::new(e2)),
                        ))),
                        elem_ty,
                    ))
                }
                _other => {
                    let opts = st.tc_display_opts;
                    pretty_error_in(
                        pat,
                        nest(
                            4,
                            vcat(vec![
                                Doc::text("type error in set element generator:"),
                                Doc::text("expression: ").append(e.display(&opts)),
                                Doc::text("does not have a set type"),
                            ]),
                        ),
                    )
                }
            }
        }
        // All _ e -> ...
        SourcePat_::All(_, e) => {
            let (e2, element_ty) = infer_expr(st, ctx, e)?;
            let q = TcQuery(element_ty.clone(), e2, None, Vec::new(), Ordered::Unordered);
            let ty = Type_::SetTy(Box::new(element_ty));
            Ok((
                Term::Ref(Match::MatchExt(Typed(
                    ty.clone(),
                    TcTerm::TcAll(Box::new(q)),
                ))),
                ty,
            ))
        }
        // TypeSignature s e ty -> do rtsType <- gets tcRtsType
        //   typ <- convertType s rtsType ty
        //   (,typ) <$> typecheckPattern ctx typ e
        SourcePat_::TypeSignature(s, e, sig_ty) => {
            let typ = convert_type(st, s, sig_ty.as_ref().clone())?;
            let p = typecheck_pattern(st, ctx, &typ, e)?;
            Ok((p, typ))
        }
        // v@KeyValue{} -> unexpectedValue v
        SourcePat_::KeyValue(..) => unexpected_value(pat),
        // FieldSelect _ p field sum -> do (p', ty) <- inferExpr ctx p
        //   fieldSelect pat ty p' field sum
        SourcePat_::FieldSelect(_, p, field, sum) => {
            let (p2, ty) = infer_expr(st, ctx, p)?;
            field_select(st, pat, &ty, p2, field, *sum)
        }
        // Deref _ p -> ...
        SourcePat_::Deref(_, p) => {
            let (p2, pred_ty) = infer_expr(st, ctx, p)?;
            let ty = match &pred_ty {
                Type_::PredicateTy(_, pref) => {
                    let details = get_predicate_details(st, &pref.predicate_id);
                    details.predicate_key_type.clone()
                }
                _ => {
                    let y = fresh_ty_var_int(st)?;
                    let key_ty = fresh_ty_var(st)?;
                    unify(st, &pred_ty, &Type_::HasKey(Box::new(key_ty.clone()), y))?;
                    key_ty
                }
            };
            Ok((
                Term::Ref(Match::MatchExt(Typed(
                    ty.clone(),
                    TcTerm::TcDeref(pred_ty, Box::new(p2)),
                ))),
                ty,
            ))
        }
        // Enum _ "true" -> promote ... trueVal BooleanTy
        SourcePat_::Enum(_, name) if name == "true" => {
            promote(st, source_pat_span(pat), true_val(), Type_::BooleanTy)
        }
        // Enum _ "false" -> promote ... falseVal BooleanTy
        SourcePat_::Enum(_, name) if name == "false" => {
            promote(st, source_pat_span(pat), false_val(), Type_::BooleanTy)
        }
        // Struct _ [ Field "just" e ] -> do (e', ty) <- inferExpr ctx e
        //   return (RTS.Alt 1 e', MaybeTy ty)
        //
        // NOTE: this arm does NOT promote (the source returns directly).
        SourcePat_::Struct(_, fields) if fields.len() == 1 && fields[0].0 == "just" => {
            let (e2, ty) = infer_expr(st, ctx, &fields[0].1)?;
            Ok((Term::Alt(1, Box::new(e2)), Type_::MaybeTy(Box::new(ty))))
        }
        // Struct _ fields -> ...
        SourcePat_::Struct(_, fields) => {
            let mut field_pats = Vec::with_capacity(fields.len());
            let mut types: BTreeMap<Name, Type> = BTreeMap::new();
            for Field(name, expr) in fields {
                let (e2, ty) = infer_expr(st, ctx, expr)?;
                field_pats.push((name.clone(), e2));
                types.insert(name.clone(), ty);
            }
            let x = fresh_ty_var_int(st)?;
            let must_be_rec = if fields.len() > 1 {
                Some(RecordOrSum::Record)
            } else {
                None
            };
            let ty = Type_::HasTy(types, must_be_rec, x);
            promote(
                st,
                source_pat_span(pat),
                Term::Ref(Match::MatchExt(Typed(
                    ty.clone(),
                    TcTerm::TcStructPat(field_pats),
                ))),
                ty,
            )
        }
        // Wildcard{} -> do x <- freshTyVar; return (mkWild x, x)
        SourcePat_::Wildcard(_) => {
            let x = fresh_ty_var(st)?;
            Ok((mk_wild(x.clone()), x))
        }
        // Enum _ name -> do x <- freshTyVarInt
        //   let ty = HasTy (Map.singleton name unit) Nothing x
        //   promote ... (Ref (MatchExt (Typed ty (TcStructPat [(name, RTS.Tuple [])])))) ty
        SourcePat_::Enum(_, name) => {
            let x = fresh_ty_var_int(st)?;
            let mut m: BTreeMap<Name, Type> = BTreeMap::new();
            m.insert(name.clone(), unit::<(), PidRef, ExpandedType>());
            let ty = Type_::HasTy(m, None, x);
            let fields = vec![(name.clone(), Term::Tuple(Vec::new()))];
            promote(
                st,
                source_pat_span(pat),
                Term::Ref(Match::MatchExt(Typed(
                    ty.clone(),
                    TcTerm::TcStructPat(fields),
                ))),
                ty,
            )
        }
        // _ -> can't infer the type of
        _ => {
            let opts = st.tc_display_opts;
            pretty_error_in(
                pat,
                nest(
                    4,
                    vcat(vec![
                        Doc::text("can't infer the type of: ").append(pat.display(&opts)),
                        Doc::text("try adding a type annotation like (")
                            .append(pat.display(&opts))
                            .append(Doc::text(" : T)")),
                    ]),
                ),
            )
        }
    }
}

// -----------------------------------------------------------------------------
// fieldSelect (Typecheck.hs:434-495)
// -----------------------------------------------------------------------------

fn field_select<ST: Clone>(
    st: &mut TypecheckState,
    src: &Pat<ST>,
    ty: &Type,
    pat: TcPat,
    field_name: &FieldName,
    record_or_sum: RecordOrSum,
) -> T<(TcPat, Type)> {
    let opts = st.tc_display_opts;
    // let err x = prettyErrorIn src $ nest 4 $ vcat
    //   [ x, "type of expression: " <> display opts ty ]
    let err = |x: Doc| -> Doc {
        nest(
            4,
            vcat(vec![
                x,
                Doc::text("type of expression: ").append(ty.display(&opts)),
            ]),
        )
    };
    // ty' <- apply ty
    let ty2 = apply(st, ty)?;
    match deref_type(&ty2) {
        // PredicateTy _ (PidRef _ ref) -> ... fieldSelect src predicateKeyType ...
        Type_::PredicateTy(_, pref) => {
            let key_ty = match st.tc_env.tc_env_predicates.get(&pref.predicate_id) {
                None => {
                    return pretty_error_in(
                        src,
                        Doc::text("fieldSelect: ").append(display_default(&pref.predicate_id)),
                    )
                }
                Some(details) => details.predicate_key_type.clone(),
            };
            // let deref = TcDeref ty pat
            let deref = TcTerm::TcDeref(ty.clone(), Box::new(pat));
            let pat2 = Term::Ref(Match::MatchExt(Typed(key_ty.clone(), deref)));
            field_select(st, src, &key_ty, pat2, field_name, record_or_sum)
        }
        // RecordTy fields | Record <- recordOrSum -> ...
        Type_::RecordTy(fields) if record_or_sum == RecordOrSum::Record => {
            let looked = lookup_field(field_name, fields);
            match looked.first() {
                Some((field_ty, _)) => {
                    let field_ty = (*field_ty).clone();
                    let sel =
                        TcTerm::TcFieldSelect(Box::new(Typed(ty.clone(), pat)), field_name.clone());
                    Ok((
                        Term::Ref(Match::MatchExt(Typed(field_ty.clone(), sel))),
                        field_ty,
                    ))
                }
                None => pretty_error_in(
                    src,
                    err(Doc::text("record does not contain the field '")
                        .append(field_name.display(&opts))
                        .append(Doc::text("'"))),
                ),
            }
        }
        Type_::RecordTy(_) => pretty_error_in(
            src,
            err(Doc::text("expression is a record, use '.")
                .append(display_default(field_name))
                .append(Doc::text("' not '."))
                .append(display_default(field_name))
                .append(Doc::text("?'"))),
        ),
        // SumTy fields | Sum <- recordOrSum -> ...
        Type_::SumTy(fields) if record_or_sum == RecordOrSum::Sum => {
            let looked = lookup_field(field_name, fields);
            match looked.first() {
                Some((field_ty, _)) => {
                    let field_ty = (*field_ty).clone();
                    let sel =
                        TcTerm::TcAltSelect(Box::new(Typed(ty.clone(), pat)), field_name.clone());
                    Ok((
                        Term::Ref(Match::MatchExt(Typed(field_ty.clone(), sel))),
                        field_ty,
                    ))
                }
                None => pretty_error_in(
                    src,
                    err(Doc::text("union type does not contain the alternative '")
                        .append(field_name.display(&opts))
                        .append(Doc::text("'"))),
                ),
            }
        }
        Type_::SumTy(_) => pretty_error_in(
            src,
            err(Doc::text("expression is a union type, use '.")
                .append(display_default(field_name))
                .append(Doc::text("?' not '."))
                .append(display_default(field_name))
                .append(Doc::text("'"))),
        ),
        // MaybeTy elemTy -> fieldSelect src (lowerMaybe elemTy) pat fieldName recordOrSum
        Type_::MaybeTy(elem_ty) => {
            let lower = lower_maybe(elem_ty.as_ref().clone());
            field_select(st, src, &lower, pat, field_name, record_or_sum)
        }
        // TyVar{} -> ...
        Type_::TyVar(_) => {
            let x = fresh_ty_var_int(st)?;
            let field_ty = fresh_ty_var(st)?;
            let mut m: BTreeMap<Name, Type> = BTreeMap::new();
            m.insert(field_name.clone(), field_ty.clone());
            let rec_ty = Type_::HasTy(m, Some(record_or_sum), x);
            // fn <- demoteTo (sourcePatSpan src) ty' recTy
            let f = demote_to(st, source_pat_span(src), &ty2, &rec_ty)?;
            let sel = match record_or_sum {
                RecordOrSum::Sum => {
                    TcTerm::TcAltSelect(Box::new(Typed(rec_ty, f(pat))), field_name.clone())
                }
                RecordOrSum::Record => {
                    TcTerm::TcFieldSelect(Box::new(Typed(rec_ty, f(pat))), field_name.clone())
                }
            };
            Ok((
                Term::Ref(Match::MatchExt(Typed(field_ty.clone(), sel))),
                field_ty,
            ))
        }
        // _other -> err $ "expression is not a " <> case recordOrSum of ...
        _other => pretty_error_in(
            src,
            err(
                Doc::text("expression is not a ").append(Doc::text(match record_or_sum {
                    RecordOrSum::Sum => "union type",
                    RecordOrSum::Record => "record",
                })),
            ),
        ),
    }
}

// -----------------------------------------------------------------------------
// convertType (Typecheck.hs:497-503)
// -----------------------------------------------------------------------------

/// @convertType span rtsType ty = do
///   let ty' = rmLocType ty
///   case rtsType ty' of Just typ -> return typ
///                       Nothing -> prettyErrorAt span "cannot convert type"@
fn convert_type<ST: Clone>(
    st: &mut TypecheckState,
    span: &SrcSpan,
    ty: Type_<ST, PredicateId, TypeId>,
) -> T<Type> {
    let ty2 = rm_loc_type(ty);
    match (st.tc_rts_type)(&ty2) {
        Some(typ) => Ok(typ),
        None => pretty_error_at(span, Doc::text("cannot convert type")),
    }
}

// -----------------------------------------------------------------------------
// typecheckPattern (Typecheck.hs:507-686) — the (expected type, pattern)
// matrix, in the source's arm order; guards that can fail fall through to
// the later arms exactly as the source's pattern guards do.
// -----------------------------------------------------------------------------

/// Check that the pattern has the correct type, and generate the low-level
/// pattern with type-annotated variables. (Source haddock :505-506.)
fn typecheck_pattern<ST: Clone>(
    st: &mut TypecheckState,
    ctx: Context,
    typ: &Type,
    pat: &Pat<ST>,
) -> T<TcPat> {
    match (typ, pat) {
        // (ByteTy, Nat _ w) -> return (RTS.Byte (fromIntegral w))
        (Type_::ByteTy, SourcePat_::Nat(_, w)) => Ok(Term::Byte(*w as u8)),
        // (NatTy, Nat _ w) -> return (RTS.Nat w)
        (Type_::NatTy, SourcePat_::Nat(_, w)) => Ok(Term::Nat(*w)),
        // (StringTy, String _ s) -> return (RTS.String (Text.encodeUtf8 s))
        (Type_::StringTy, SourcePat_::String(_, s)) => Ok(Term::String(s.as_bytes().to_vec())),
        // (StringTy, StringPrefix _ s) -> return (RTS.Ref (MatchPrefix ... (mkWild StringTy)))
        (Type_::StringTy, SourcePat_::StringPrefix(_, s)) => Ok(Term::Ref(Match::MatchPrefix(
            s.as_bytes().to_vec(),
            Box::new(mk_wild(Type_::StringTy)),
        ))),
        // (StringTy, App _ (StringPrefix _ s) [pat]) -> do
        //   pat' <- typecheckPattern ctx StringTy pat
        //   return (RTS.Ref (MatchPrefix (encodeUtf8 s) pat'))
        (Type_::StringTy, SourcePat_::App(_, f, xs))
            if xs.len() == 1 && matches!(f.as_ref(), SourcePat_::StringPrefix(..)) =>
        {
            let SourcePat_::StringPrefix(_, s) = f.as_ref() else {
                unreachable!()
            };
            let p2 = typecheck_pattern(st, ctx, &Type_::StringTy, &xs[0])?;
            Ok(Term::Ref(Match::MatchPrefix(
                s.as_bytes().to_vec(),
                Box::new(p2),
            )))
        }
        // (ArrayTy ByteTy, String _ s) -> return (RTS.ByteArray (encodeUtf8 s))
        (Type_::ArrayTy(elem), SourcePat_::String(_, s)) if **elem == Type_::ByteTy => {
            Ok(Term::ByteArray(s.as_bytes().to_vec()))
        }
        // (ArrayTy ByteTy, ByteArray _ s) -> return (RTS.ByteArray s)
        (Type_::ArrayTy(elem), SourcePat_::ByteArray(_, b)) if **elem == Type_::ByteTy => {
            Ok(Term::ByteArray(b.clone()))
        }
        // (ArrayTy elemTy, Array _ pats) ->
        //   RTS.Array <$> mapM (typecheckPattern ctx elemTy) pats
        (Type_::ArrayTy(elem), SourcePat_::Array(_, pats)) => {
            let elem = elem.as_ref().clone();
            let mut out = Vec::with_capacity(pats.len());
            for p in pats.iter() {
                out.push(typecheck_pattern(st, ctx, &elem, p)?);
            }
            Ok(Term::Array(out))
        }
        // (ArrayTy elemTy, ArrayPrefix _ pre) -> do
        //   pre <- mapM (typecheckPattern ctx elemTy) pre
        //   let match = MatchArrayPrefix elemTy (toList pre) (RTS.Ref (MatchWild typ))
        (Type_::ArrayTy(elem), SourcePat_::ArrayPrefix(_, pre)) => {
            let elem = elem.as_ref().clone();
            let mut out = Vec::with_capacity(pre.tail.len() + 1);
            out.push(typecheck_pattern(st, ctx, &elem, &pre.head)?);
            for p in pre.tail.iter() {
                out.push(typecheck_pattern(st, ctx, &elem, p)?);
            }
            let m = Match::MatchArrayPrefix(
                elem,
                out,
                Box::new(Term::Ref(Match::MatchWild(typ.clone()))),
            );
            Ok(Term::Ref(m))
        }
        // (RecordTy fields, Tuple _ pats) | length fields == length pats ->
        //   RTS.Tuple <$> mapM (\(t,p) -> typecheckPattern ctx t p)
        //     (zip (map fieldDefType fields) pats)
        (Type_::RecordTy(fields), SourcePat_::Tuple(_, pats)) if fields.len() == pats.len() => {
            let mut out = Vec::with_capacity(pats.len());
            for (fld, p) in fields.iter().zip(pats.iter()) {
                out.push(typecheck_pattern(st, ctx, &fld.field_def_type, p)?);
            }
            Ok(Term::Tuple(out))
        }
        // (RecordTy fieldSchema, Struct _ fields)
        //   | all (is an element of map fieldDefName fieldSchema)
        //     (map fieldName fields) ->
        //     RTS.Tuple <$> mapM doField fieldSchema
        //   where doField (FieldDef name ty) =
        //           case [ pat | Field name' pat <- fields, name == name' ] of
        //             (pat:_) -> typecheckPattern ctx ty pat
        //             [] -> return (mkWild ty)     -- missing field is a wildcard
        (Type_::RecordTy(field_schema), SourcePat_::Struct(_, fields))
            if fields.iter().all(|Field(n, _)| {
                field_schema
                    .iter()
                    .any(|f| f.field_def_name.as_str() == n.as_str())
            }) =>
        {
            let mut out = Vec::with_capacity(field_schema.len());
            for fld in field_schema.iter() {
                match fields
                    .iter()
                    .find(|Field(n, _)| n.as_str() == fld.field_def_name.as_str())
                {
                    Some(Field(_, p)) => {
                        out.push(typecheck_pattern(st, ctx, &fld.field_def_type, p)?)
                    }
                    // missing field is a wildcard
                    None => out.push(mk_wild(fld.field_def_type.clone())),
                }
            }
            Ok(Term::Tuple(out))
        }
        // (SumTy fields, Struct _ [Field fieldName pat]) ->
        //   case lookupField fieldName fields of
        //     (ty, n) :_ -> RTS.Alt n <$> typecheckPattern ctx ty pat
        //     _ -> patTypeErrorDesc ("unknown alt: " <> fieldName) pat typ
        (Type_::SumTy(fields), SourcePat_::Struct(_, fs)) if fs.len() == 1 => {
            let Field(name, p) = &fs[0];
            let looked = lookup_field(name, fields);
            match looked.first() {
                Some((field_ty, n)) => {
                    let field_ty = (*field_ty).clone();
                    let p2 = typecheck_pattern(st, ctx, &field_ty, p)?;
                    Ok(Term::Alt(*n, Box::new(p2)))
                }
                None => pat_type_error_desc(st, &format!("unknown alt: {name}"), pat, typ),
            }
        }
        // (SumTy fields, Enum _ fieldName)
        //   | ((ty, n):_) <- lookupField fieldName fields ->
        //     return (RTS.Alt n (mkWild ty))
        (Type_::SumTy(fields), SourcePat_::Enum(_, name))
            if !lookup_field(name, fields).is_empty() =>
        {
            let looked = lookup_field(name, fields);
            let (field_ty, n) = looked[0];
            Ok(Term::Alt(n, Box::new(mk_wild(field_ty.clone()))))
        }
        // (SumTy _, Struct _ _) ->
        //   patTypeErrorDesc "matching on a union type should have the form
        //   { field = pattern }" pat typ
        (Type_::SumTy(_), SourcePat_::Struct(..)) => pat_type_error_desc(
            st,
            "matching on a union type should have the form { field = pattern }",
            pat,
            typ,
        ),
        // (NamedTy _ (ExpandedType _ ty), term) -> typecheckPattern ctx ty term
        (Type_::NamedTy(_, expanded), term) => {
            typecheck_pattern(st, ctx, &expanded.expanded_type_type, term)
        }
        // (ty, Prim span primOp args) -> ... inPat pat $ unify ty retTy ...
        (ty, SourcePat_::Prim(span, prim_op, args)) => {
            let (prim_arg_tys, ret_ty) = prim_op_type(st, *prim_op)?;
            check_prim_op_args(span, args, &prim_arg_tys)?;
            let mut args2 = Vec::with_capacity(args.len());
            for (t, a) in prim_arg_tys.iter().zip(args.iter()) {
                args2.push(typecheck_pattern(st, Context::ContextExpr, t, a)?);
            }
            in_pat(st, pat, |st| unify(st, ty, &ret_ty))?;
            Ok(Term::Ref(Match::MatchExt(Typed(
                ret_ty,
                TcTerm::TcPrimCall(*prim_op, args2),
            ))))
        }
        // (PredicateTy _ (PidRef _ ref), Clause _ spanPred ref' arg range) ->
        //   if ref == ref' then fst <$> tcFactGenerator ref arg range (Just spanPred)
        //   else patTypeError pat typ
        (Type_::PredicateTy(_, pref), SourcePat_::Clause(_, span_pred, ref2, arg, range)) => {
            if pref.predicate_id == *ref2 {
                let (p, _) = tc_fact_generator(st, ref2, arg, *range, Some(*span_pred))?;
                Ok(p)
            } else {
                pat_type_error(st, pat, typ)
            }
        }
        // (MaybeTy elemTy, pat) -> typecheckPattern ctx (lowerMaybe elemTy) pat
        (Type_::MaybeTy(elem_ty), p) => {
            let lower = lower_maybe(elem_ty.as_ref().clone());
            typecheck_pattern(st, ctx, &lower, p)
        }
        // (EnumeratedTy names, Enum _ name) | Just n <- elemIndex name names ->
        //   return (RTS.Alt (fromIntegral n) (RTS.Tuple []))
        (Type_::EnumeratedTy(names), SourcePat_::Enum(_, name))
            if names.iter().any(|n| n == name) =>
        {
            let n = names.iter().position(|n| n == name).expect("checked");
            Ok(Term::Alt(n as u64, Box::new(Term::Tuple(Vec::new()))))
        }
        // (BooleanTy, Enum _ name) | name == "false" -> return falseVal
        (Type_::BooleanTy, SourcePat_::Enum(_, name)) if name == "false" => Ok(false_val()),
        // (BooleanTy, Enum _ name) | name == "true" -> return trueVal
        (Type_::BooleanTy, SourcePat_::Enum(_, name)) if name == "true" => Ok(true_val()),
        // (EnumeratedTy names, Variable _ name) | Just n <- elemIndex name names ->
        //   return (RTS.Alt (fromIntegral n) (RTS.Tuple []))
        (Type_::EnumeratedTy(names), SourcePat_::Variable(_, name))
            if names.iter().any(|n| n == name) =>
        {
            let n = names.iter().position(|n| n == name).expect("checked");
            Ok(Term::Alt(n as u64, Box::new(Term::Tuple(Vec::new()))))
        }
        // (ty, ElementsOfArray _ pat) -> do containerTy <- freshTyVarInt
        //   pat' <- typecheckPattern ContextExpr (ElementsOf ty containerTy) pat
        //   return (Ref (MatchExt (Typed ty (TcElementsUnresolved (TyVar containerTy) pat'))))
        (ty, SourcePat_::ElementsOfArray(_, p)) => {
            let container_ty = fresh_ty_var_int(st)?;
            let elem_pat = Type_::ElementsOf(Box::new(ty.clone()), container_ty);
            let p2 = typecheck_pattern(st, Context::ContextExpr, &elem_pat, p)?;
            Ok(Term::Ref(Match::MatchExt(Typed(
                ty.clone(),
                TcTerm::TcElementsUnresolved(Type_::TyVar(container_ty), Box::new(p2)),
            ))))
        }
        // (ty, Elements _ pat) -> same, TcElementsUnresolved
        (ty, SourcePat_::Elements(_, p)) => {
            let container_ty = fresh_ty_var_int(st)?;
            let elem_pat = Type_::ElementsOf(Box::new(ty.clone()), container_ty);
            let p2 = typecheck_pattern(st, Context::ContextExpr, &elem_pat, p)?;
            Ok(Term::Ref(Match::MatchExt(Typed(
                ty.clone(),
                TcTerm::TcElementsUnresolved(Type_::TyVar(container_ty), Box::new(p2)),
            ))))
        }
        // (ty, OrPattern _ left right) -> disjunction ...; Typed ty (TcOr left' right')
        (ty, SourcePat_::OrPattern(_, left, right)) => {
            let vars_l = vars_pat(left, HashSet::new());
            let vars_r = vars_pat(right, HashSet::new());
            let (l2, r2) = disjunction(
                st,
                vars_l,
                |st| typecheck_pattern(st, ctx, ty, left),
                vars_r,
                |st, _: &TcPat| typecheck_pattern(st, ctx, ty, right),
            )?;
            Ok(Term::Ref(Match::MatchExt(Typed(
                ty.clone(),
                TcTerm::TcOr(Box::new(l2), Box::new(r2)),
            ))))
        }
        // (ty, IfPattern _ srcCond srcThen srcElse) -> ...
        (
            ty,
            SourcePat_::IfPattern {
                span: _,
                cond,
                then_,
                else_,
            },
        ) => {
            let vars_a = vars_pat(cond, vars_pat(then_, HashSet::new()));
            let vars_b = vars_pat(else_, HashSet::new());
            let ((cond2, then2), else2) = disjunction(
                st,
                vars_a,
                |st| {
                    let cond_pat = ignore_result(cond);
                    let (c, cond_ty) = infer_expr(st, ctx, &cond_pat)?;
                    let t = typecheck_pattern(st, ctx, ty, then_)?;
                    Ok((Typed(cond_ty, c), t))
                },
                vars_b,
                |st, _: &(Typed<TcPat>, TcPat)| typecheck_pattern(st, ctx, ty, else_),
            )?;
            Ok(Term::Ref(Match::MatchExt(Typed(
                ty.clone(),
                TcTerm::TcIf {
                    cond: Box::new(cond2),
                    then_: Box::new(then2),
                    else_: Box::new(else2),
                },
            ))))
        }
        // (ty, NestedQuery _ query) ->
        //   Ref . MatchExt . Typed ty . TcQueryGen <$> typecheckQuery ctx ty query
        (ty, SourcePat_::NestedQuery(_, query)) => {
            let q = typecheck_query(st, ctx, ty, query)?;
            Ok(Term::Ref(Match::MatchExt(Typed(
                ty.clone(),
                TcTerm::TcQueryGen(Box::new(q)),
            ))))
        }
        // (ty, Negation s pat) | ty == unit -> do ... (see NOTE below)
        (ty, SourcePat_::Negation(s, inner)) if *ty == unit::<(), PidRef, ExpandedType>() => {
            let start_pos = SrcSpan::mk_span(s.start_loc(), s.start_loc());
            // empty = Tuple startPos []
            let empty = SourcePat_::Tuple(start_pos, Vec::new());
            // (stmts, ord) = case pat of
            //   NestedQuery _ (SourceQuery Nothing stmts _) -> (stmts, ord)
            //   other -> ([SourceStatement (Wildcard s) other], Unordered)
            //
            // NOTE (not a port defect): the source's first branch binds ord
            // cyclically (the let-bound ord from its own definition), but
            // that value is NEVER forced: needsResult returns it lazily
            // (Typecheck.hs:181/:184/:186/:189), typecheckQuery stores it
            // (:234), TcQuery's Ordered field has no bang
            // (Query/Typecheck/Types.hs:33), SourceQuery has no strict
            // fields (Angle/Types.hs:175-179), and the only consumer
            // (:653) discards it with `TcQuery _ _ _ stmts _ <- ...`. Any
            // value is observationally equal here, so the image binds
            // Unordered.
            let (stmts, ord) = match inner.as_ref() {
                SourcePat_::NestedQuery(_, q) if q.src_query_head.is_none() => {
                    (q.src_query_stmts.clone(), Ordered::Unordered)
                }
                other => (
                    vec![SourceStatement_(SourcePat_::Wildcard(*s), other.clone())],
                    Ordered::Unordered,
                ),
            };
            let query = SourceQuery_ {
                src_query_head: Some(empty),
                src_query_stmts: stmts,
                src_query_ordered: ord,
            };
            let u = unit::<(), PidRef, ExpandedType>();
            let TcQuery(_, _, _, stmts2, _) =
                enclose(st, inner, |st| typecheck_query(st, ctx, &u, &query))?;
            Ok(Term::Ref(Match::MatchExt(Typed(
                u,
                TcTerm::TcNegation(stmts2),
            ))))
        }
        // (PredicateTy _ _, FactId _ Nothing fid) -> do isFactIdAllowed pat
        //   return $ Ref (MatchFid (Fid (fromIntegral fid)))
        (Type_::PredicateTy(_, _), SourcePat_::FactId(_, None, fid)) => {
            is_fact_id_allowed(st, pat)?;
            Ok(Term::Ref(Match::MatchFid(Fid(*fid as i64))))
        }
        // (ty, TypeSignature s e sigty) -> do rtsType <- gets tcRtsType
        //   sigty' <- convertType s rtsType sigty
        //   e' <- typecheckPattern ctx sigty' e
        //   f <- promoteTo (sourcePatSpan pat) sigty' ty
        //   return (f e')
        (ty, SourcePat_::TypeSignature(s, e, sig_ty)) => {
            let sigty = convert_type(st, s, sig_ty.as_ref().clone())?;
            let e2 = typecheck_pattern(st, ctx, &sigty, e)?;
            let f = promote_to(st, source_pat_span(pat), &sigty, ty)?;
            Ok(f(e2))
        }
        // (ty@(SetTy elemTy), All _ query) -> do arg <- typecheckPattern ctx elemTy query
        //   let q = TcQuery elemTy arg Nothing [] Unordered
        //   return (Ref (MatchExt (Typed ty (TcAll q))))
        (Type_::SetTy(elem_ty), SourcePat_::All(_, query)) => {
            let elem_ty = elem_ty.as_ref().clone();
            let arg = typecheck_pattern(st, ctx, &elem_ty, query)?;
            let q = TcQuery(elem_ty, arg, None, Vec::new(), Ordered::Unordered);
            Ok(Term::Ref(Match::MatchExt(Typed(
                typ.clone(),
                TcTerm::TcAll(Box::new(q)),
            ))))
        }
        // (PredicateTy _ (PidRef _ ref), pat) | matchesKey pat ->
        //   fst <$> tcFactGenerator ref pat SeekOnAllFacts Nothing
        (Type_::PredicateTy(_, pref), p) if matches_key(p) => {
            let (r, _) =
                tc_fact_generator(st, &pref.predicate_id, p, SeekSection::SeekOnAllFacts, None)?;
            Ok(r)
        }
        // (ty, Wildcard{}) -> return (mkWild ty)
        (ty, SourcePat_::Wildcard(_)) => Ok(mk_wild(ty.clone())),
        // (ty, Never{}) -> return $ Ref (MatchNever ty)
        (ty, SourcePat_::Never(_)) => Ok(Term::Ref(Match::MatchNever(ty.clone()))),
        // (ty, Variable span name) -> varOcc ctx span name ty
        (ty, SourcePat_::Variable(span, name)) => var_occ(st, ctx, span, name, ty),
        // (_, KeyValue{}) -> unexpectedValue pat
        (_, SourcePat_::KeyValue(..)) => unexpected_value(pat),
        // (ty, _) -> do (p', patTy) <- inferExpr ctx pat
        //   inPat pat $ unify ty patTy
        //   return p'
        (ty, _) => {
            let (p2, pat_ty) = infer_expr(st, ctx, pat)?;
            in_pat(st, pat, |st| unify(st, ty, &pat_ty))?;
            Ok(p2)
        }
    }
}

// -----------------------------------------------------------------------------
// tcFactGenerator (Typecheck.hs:688-720)
// -----------------------------------------------------------------------------

fn tc_fact_generator<ST: Clone>(
    st: &mut TypecheckState,
    pred: &PredicateId,
    pat: &Pat<ST>,
    range: SeekSection,
    pred_span: Option<SrcSpan>,
) -> T<(TcPat, Type)> {
    // TcEnv{..} <- gets tcEnv
    // PredicateDetails{..} <- case HashMap.lookup ref tcEnvPredicates of
    //   Nothing -> prettyErrorIn pat $ "tcFactGenerator: " <> displayDefault ref
    //   Just details -> return details
    //
    // NOTE: this is the source's own lookup with its own message — NOT
    // monad.rs's getPredicateDetails, which panics on a missing predicate.
    let (predicate_pid, key_ty, val_ty) = match st.tc_env.tc_env_predicates.get(pred) {
        None => {
            return pretty_error_in(
                pat,
                Doc::text("tcFactGenerator: ").append(display_default(pred)),
            )
        }
        Some(details) => (
            details.predicate_pid,
            details.predicate_key_type.clone(),
            details.predicate_value_type.clone(),
        ),
    };
    let (kpat, vpat) = match pat {
        // KeyValue _ kpat vpat -> do
        //   kpat' <- typecheckPattern ContextPat predicateKeyType kpat
        //   vpat' <- typecheckPattern ContextPat predicateValueType vpat
        SourcePat_::KeyValue(_, kpat, vpat) => {
            let k = typecheck_pattern(st, Context::ContextPat, &key_ty, kpat)?;
            let v = typecheck_pattern(st, Context::ContextPat, &val_ty, vpat)?;
            (k, v)
        }
        // _other -> do kpat' <- typecheckPattern ContextPat predicateKeyType pat
        //   return (kpat', mkWild predicateValueType)
        _other => {
            let k = typecheck_pattern(st, Context::ContextPat, &key_ty, pat)?;
            (k, mk_wild(val_ty.clone()))
        }
    };
    // let pidRef = PidRef predicatePid ref
    //     ty = PredicateTy () pidRef
    let pid_ref = PidRef {
        pid: predicate_pid,
        predicate_id: pred.clone(),
    };
    let ty = Type_::PredicateTy((), pid_ref.clone());
    // case predSpan of Just span -> modify $ \s -> s { tcPreds = (pidRef,Some span):tcPreds s }
    if let Some(span) = pred_span {
        st.tc_preds.insert(0, (pid_ref.clone(), span));
    }
    Ok((
        Term::Ref(Match::MatchExt(Typed(
            ty.clone(),
            TcTerm::TcFactGen(pid_ref, Box::new(kpat), Box::new(vpat), range),
        ))),
        ty,
    ))
}

/// @matchesKey@ (Typecheck.hs:722-729) — the four constructors that mean
/// "match the key's shape" rather than "match the key".
fn matches_key<ST>(pat: &Pat<ST>) -> bool {
    match pat {
        SourcePat_::Wildcard(_)
        | SourcePat_::Variable(..)
        | SourcePat_::FieldSelect(..)
        | SourcePat_::Deref(..) => false,
        _ => true,
    }
}

/// Fact Id patterns (#1234 or #pred 1234) are only allowed in queries, not
/// in derived predicates, because they potentially allow a type-incorrect
/// fact to be constructed. (Source comment :729-731.)
fn is_fact_id_allowed<ST>(st: &TypecheckState, pat: &Pat<ST>) -> T<()> {
    let opts = st.tc_display_opts;
    if st.tc_mode != TcMode::TcModeQuery {
        pretty_error_in(
            pat,
            Doc::text("fact IDs are not allowed in a derived predicate: ")
                .append(pat.display(&opts)),
        )
    } else {
        Ok(())
    }
}

/// @falseVal, trueVal :: TcPat@ (:740-742).
fn false_val() -> TcPat {
    Term::Alt(0, Box::new(Term::Tuple(Vec::new())))
}

fn true_val() -> TcPat {
    Term::Alt(1, Box::new(Term::Tuple(Vec::new())))
}

/// @patTypeError = patTypeErrorDesc "type error in pattern"@ (:744-745).
fn pat_type_error<ST, A>(st: &TypecheckState, pat: &Pat<ST>, ty: &Type) -> T<A> {
    pat_type_error_desc(st, "type error in pattern", pat, ty)
}

/// @patTypeErrorDesc desc q ty@ (:747-755).
fn pat_type_error_desc<ST, A>(st: &TypecheckState, desc: &str, q: &Pat<ST>, ty: &Type) -> T<A> {
    let opts = st.tc_display_opts;
    pretty_error_in(
        q,
        nest(
            4,
            vcat(vec![
                display_default(desc),
                Doc::text("pattern: ").append(q.display(&opts)),
                Doc::text("expected type: ").append(ty.display(&opts)),
            ]),
        ),
    )
}

// -----------------------------------------------------------------------------
// bindOrUse / inferVar / varOcc / freshVariable (Typecheck.hs:757-809)
// -----------------------------------------------------------------------------

fn bind_or_use(ctx: Context, name: &Name, st: &mut TypecheckState) {
    match ctx {
        Context::ContextExpr => {
            st.tc_uses.insert(name.clone());
        }
        Context::ContextPat => {
            st.tc_bindings.insert(name.clone());
        }
    }
}

fn infer_var<ST: Clone>(
    st: &mut TypecheckState,
    ctx: Context,
    span: &SrcSpan,
    name: &Name,
) -> T<(TcPat, Type)> {
    check_var_case(span, name)?;
    match st.tc_scope.get(name).cloned() {
        // Just v@(Var ty _ _) -> do
        //   put $ bindOrUse ctx name $ state { tcFree = HashSet.delete name tcFree }
        //   return (Ref (MatchVar v), ty)
        Some(v) => {
            let ty = v.var_type.clone();
            st.tc_free.remove(name);
            bind_or_use(ctx, name, st);
            Ok((Term::Ref(Match::MatchVar(v)), ty))
        }
        // Nothing -> do x <- freshTyVar; p <- varOcc ctx span name x; return (p,x)
        None => {
            let x = fresh_ty_var(st)?;
            let p = var_occ(st, ctx, span, name, &x)?;
            Ok((p, x))
        }
    }
}

fn var_occ<ST: Clone>(
    st: &mut TypecheckState,
    ctx: Context,
    span: &SrcSpan,
    name: &Name,
    ty: &Type,
) -> T<TcPat> {
    check_var_case(span, name)?;
    match st.tc_scope.get(name).cloned() {
        // Nothing -> do
        //   let var = Var ty tcNextVar (Just name)
        //       !next = tcNextVar + 1
        //   put $ bindOrUse ctx name $ state { tcNextVar = next
        //     , tcScope = insert name var tcScope
        //     , tcFree = insert name tcFree
        //     , tcVars = IntMap.insert tcNextVar var tcVars }
        //   return (RTS.Ref (MatchBind var))
        None => {
            let var = Var {
                var_type: ty.clone(),
                var_id: st.tc_next_var,
                var_orig_name: Some(name.clone()),
            };
            st.tc_next_var = st.tc_next_var.wrapping_add(1);
            bind_or_use(ctx, name, st);
            st.tc_scope.insert(name.clone(), var.clone());
            st.tc_free.insert(name.clone());
            st.tc_vars.insert(var.var_id, var.clone());
            Ok(Term::Ref(Match::MatchBind(var)))
        }
        // Just v@(Var ty' _ _) -> do
        //   put $ bindOrUse ctx name $ state { tcFree = HashSet.delete name tcFree }
        //   addErrSpan span $ unify ty ty'
        //   return (Ref (MatchVar v))
        Some(v) => {
            let ty2 = v.var_type.clone();
            st.tc_free.remove(name);
            bind_or_use(ctx, name, st);
            add_err_span(st, span, |st| unify(st, ty, &ty2))?;
            Ok(Term::Ref(Match::MatchVar(v)))
        }
    }
}

/// @freshVariable s@ (:798-809) — note the name is built from the OLD
/// counter value and the counter then advances; tcVisible is updated as a
/// documented hack (source comment :805-807).
fn fresh_variable<ST>(st: &mut TypecheckState, s: &SrcSpan) -> T<Pat<ST>> {
    let next = st.tc_next_var.wrapping_add(1);
    let name = format!("Tmp__{}", st.tc_next_var);
    st.tc_next_var = next;
    st.tc_visible.insert(name.clone());
    Ok(SourcePat_::Variable(*s, name))
}

// -----------------------------------------------------------------------------
// Variable well-formedness (Typecheck.hs:811-857)
// -----------------------------------------------------------------------------

/// @freeVariablesAreErrors@ (:811-819).
///
/// TODO(port): the source renders HashSet.toList into the message
/// (Typecheck.hs:817), and unordered-containers' order is a function of its
/// hashWithSalt chain, which no shipped unit ports — std HashSet is a
/// DIFFERENT order. DECISION: keep std HashSet and carry the open question
/// with a named owner (see THE HASHSET DECISION in the header). The only
/// user-visible difference is the ORDER of the names inside the list;
/// membership, separators and message text are identical. Owner: the
/// Step-6 judge (these diagnostics are golden-compared there) or a future
/// hashing unit that ports the unordered-containers chain.
fn free_variables_are_errors(st: &mut TypecheckState) -> T<()> {
    let free = &st.tc_free;
    if free.is_empty() {
        return Ok(());
    }
    let names: Vec<Doc> = free.iter().map(|n| display_default(n)).collect();
    pretty_error(nest(
        4,
        vcat(vec![
            Doc::text("One or more variables were mentioned only once: ")
                .append(hsep(punctuate(Doc::text(","), names))),
            Doc::text("This is usually a mistake, so it is disallowed in Angle."),
        ]),
    ))
}

/// @unboundVariablesAreErrors@ (:821-824).
fn unbound_variables_are_errors(st: &mut TypecheckState) -> T<()> {
    let uses = st.tc_uses.clone();
    let binds = st.tc_bindings.clone();
    unbound_variables_are_errors_(&uses, &binds)
}

/// @unboundVariablesAreErrors_ uses binds@ (:826-841).
///
/// The name list comes from HashSet.toList (hash order) — same decision and
/// same owner as free_variables_are_errors.
fn unbound_variables_are_errors_(uses: &HashSet<Name>, binds: &HashSet<Name>) -> T<()> {
    let bad: Vec<&Name> = uses.difference(binds).collect();
    if bad.is_empty() {
        return Ok(());
    }
    let names: Vec<Doc> = bad.iter().map(|n| display_default(*n)).collect();
    pretty_error(nest(
        4,
        vcat(vec![
            Doc::text("One or more variables were not bound anywhere: ")
                .append(hsep(punctuate(Doc::text(","), names))),
            Doc::text("All variables must occur at least once in a context that will bind"),
            Doc::text("the variable to a value; i.e., *not*"),
            Doc::text("  * the argument of a primitive"),
            Doc::text("  * the array in an array generator Arr[..]"),
            Doc::text("A variable that is bound in just one side of '|'")
                .append_space(Doc::text("cannot be used outside of the '|'")),
            Doc::text("A variable that is bound in a negated query '!(...)'")
                .append_space(Doc::text("cannot be used outside of it")),
        ]),
    ))
}

/// @checkVarCase span name@ (:843-848) — Haskell's isUpper is Data.Char's
/// Unicode Lu-category predicate. char::is_uppercase is WIDER: it also
/// covers Other_Uppercase (e.g. U+2160 ROMAN NUMERAL ONE). Known narrow
/// divergence: a name starting with an Other_Uppercase character is
/// accepted here and rejected by the source. No table hack is attempted —
/// see the header's KNOWN NARROW DIVERGENCE note.
fn check_var_case(span: &SrcSpan, name: &Name) -> T<()> {
    match name.chars().next() {
        Some(h) if !h.is_uppercase() => pretty_error_at(
            span,
            Doc::text("variable does not begin with an upper-case letter: ")
                .append(display_default(name)),
        ),
        _ => Ok(()),
    }
}

// -----------------------------------------------------------------------------
// disjunction / oneBranch (Typecheck.hs:851-905)
// -----------------------------------------------------------------------------

/// Typechecking A|B (source comment :850-857):
///
/// 1. The set of variables that are considered to be *bound* by this
///    pattern are those that are bound in both branches.
/// 2. The set of variables that are considered to be *used* by this
///    pattern are those that are used in either branch.
///
/// state0 is snapshotted before either branch runs (the source's
/// @state0 <- get@); only tcBindings/tcUses are read from it.
fn disjunction<A, B, FA, FB>(
    st: &mut TypecheckState,
    vars_a: HashSet<Name>,
    ta: FA,
    vars_b: HashSet<Name>,
    tb: FB,
) -> T<(A, B)>
where
    FA: FnOnce(&mut TypecheckState) -> T<A>,
    FB: FnOnce(&mut TypecheckState, &A) -> T<B>,
{
    let binds0 = st.tc_bindings.clone();
    let uses0 = st.tc_uses.clone();
    let (a, uses_a, binds_a) = one_branch(st, vars_a, ta)?;
    let (b, uses_b, binds_b) = one_branch(st, vars_b, |st| tb(st, &a))?;
    // tcBindings = tcBindings state0 union (bindsA intersection bindsB)
    let mut both_binds = binds_a;
    both_binds.retain(|n| binds_b.contains(n));
    st.tc_bindings = binds0;
    st.tc_bindings.extend(both_binds);
    // tcUses = tcUses state0 union (usesA union usesB)
    let mut either_uses = uses_a;
    either_uses.extend(uses_b);
    st.tc_uses = uses0;
    st.tc_uses.extend(either_uses);
    Ok((a, b))
}

/// Typechecking either A or B in A|B (source comment :871-881):
///
/// 1. Variables that are not visible in the enclosing scope are local to
///    the inner scope, so they are removed from tcScope afterwards.
/// 2. Local variables must be bound locally, or we report an error.
/// 3. tcVisible is updated to the visible variables of the current pattern,
///    which determines what is local to any nested A|B subterms.
fn one_branch<A, F>(
    st: &mut TypecheckState,
    branch_vars: HashSet<Name>,
    ta: F,
) -> T<(A, HashSet<Name>, HashSet<Name>)>
where
    F: FnOnce(&mut TypecheckState) -> T<A>,
{
    let visible_before = st.tc_visible.clone();
    // modify $ \s -> s { tcUses = empty, tcBindings = empty
    //                  , tcVisible = visibleBefore union branchVars }
    st.tc_uses.clear();
    st.tc_bindings.clear();
    st.tc_visible = visible_before.clone();
    st.tc_visible.extend(branch_vars);
    let a = ta(st)?;
    // after <- get
    let after_uses = st.tc_uses.clone();
    let after_binds = st.tc_bindings.clone();
    // let localUses = tcUses after minus visibleBefore
    //     localBinds = tcBindings after minus visibleBefore
    let mut local_uses = after_uses.clone();
    local_uses.retain(|n| !visible_before.contains(n));
    let mut local_binds = after_binds.clone();
    local_binds.retain(|n| !visible_before.contains(n));
    unbound_variables_are_errors_(&local_uses, &local_binds)?;
    // tcScope = intersection (tcScope after) (toMap visibleBefore)
    // tcVisible = visibleBefore
    st.tc_scope.retain(|k, _| visible_before.contains(k));
    st.tc_visible = visible_before.clone();
    // extUses / extBinds = intersection (tcUses/tcBindings after) visibleBefore
    let mut ext_uses = after_uses;
    ext_uses.retain(|n| visible_before.contains(n));
    let mut ext_binds = after_binds;
    ext_binds.retain(|n| visible_before.contains(n));
    Ok((a, ext_uses, ext_binds))
}

/// The @enclose@ local of the Negation arm (Typecheck.hs:638-651): a
/// negated query's bindings, uses and scope are not visible outside it.
fn enclose<ST: Clone, A, F>(st: &mut TypecheckState, pat: &Pat<ST>, act: F) -> T<A>
where
    F: FnOnce(&mut TypecheckState) -> T<A>,
{
    // before <- get
    let before_visible = st.tc_visible.clone();
    let before_bindings = st.tc_bindings.clone();
    // modify $ \s -> s { tcVisible = varsPat pat (tcVisible before) }
    st.tc_visible = vars_pat(pat, before_visible.clone());
    let res = act(st)?;
    // modify $ \after -> after { tcBindings = tcBindings before
    //   , tcUses = tcUses after intersected with tcVisible before
    //   , tcScope = tcScope after intersected with toMap (tcVisible before)
    //   , tcVisible = tcVisible before }
    st.tc_bindings = before_bindings;
    st.tc_uses.retain(|n| before_visible.contains(n));
    st.tc_scope.retain(|k, _| before_visible.contains(k));
    st.tc_visible = before_visible;
    Ok(res)
}

// -----------------------------------------------------------------------------
// checkPrimOpArgs / primOpType (Typecheck.hs:907-962)
// -----------------------------------------------------------------------------

fn check_prim_op_args<ST>(span: &SrcSpan, args: &[Pat<ST>], tys: &[Type]) -> T<()> {
    if args.len() != tys.len() {
        pretty_error_at(
            span,
            Doc::text("expected ")
                .append(Doc::text(tys.len().to_string()))
                .append(Doc::text(" arguments, found "))
                .append(Doc::text(args.len().to_string())),
        )
    } else {
        Ok(())
    }
}

/// Several primops have polymorphic types, e.g. prim.length : forall a .
/// [a] -> nat. There's no first-class support for polymorphic types in the
/// type checker, but for primops we can just generate a fresh instantiation
/// of the primop's type for each call. (Source comment :915-921.)
fn prim_op_type(st: &mut TypecheckState, op: PrimOp) -> T<(Vec<Type>, Type)> {
    match op {
        PrimOp::PrimOpToLower => Ok((vec![Type_::StringTy], Type_::StringTy)),
        PrimOp::PrimOpLength => {
            let x = fresh_ty_var(st)?;
            Ok((vec![Type_::ArrayTy(Box::new(x))], Type_::NatTy))
        }
        PrimOp::PrimOpZip => {
            let x = fresh_ty_var(st)?;
            let y = fresh_ty_var(st)?;
            Ok((
                vec![
                    Type_::ArrayTy(Box::new(x.clone())),
                    Type_::ArrayTy(Box::new(y.clone())),
                ],
                Type_::ArrayTy(Box::new(tuple_schema(vec![x, y]))),
            ))
        }
        PrimOp::PrimOpConcat => {
            let x = fresh_ty_var(st)?;
            Ok((
                vec![
                    Type_::ArrayTy(Box::new(x.clone())),
                    Type_::ArrayTy(Box::new(x.clone())),
                ],
                Type_::ArrayTy(Box::new(x)),
            ))
        }
        PrimOp::PrimOpReverse => Ok((vec![Type_::StringTy], Type_::StringTy)),
        PrimOp::PrimOpSize => {
            let x = fresh_ty_var(st)?;
            Ok((vec![Type_::SetTy(Box::new(x))], Type_::NatTy))
        }
        PrimOp::PrimOpRelToAbsByteSpans => Ok((
            vec![Type_::ArrayTy(Box::new(tuple_schema(vec![
                Type_::NatTy,
                Type_::NatTy,
            ])))],
            Type_::ArrayTy(Box::new(tuple_schema(vec![Type_::NatTy, Type_::NatTy]))),
        )),
        PrimOp::PrimOpUnpackByteSpans => Ok((
            vec![Type_::ArrayTy(Box::new(tuple_schema(vec![
                Type_::NatTy,
                Type_::ArrayTy(Box::new(Type_::NatTy)),
            ])))],
            Type_::ArrayTy(Box::new(tuple_schema(vec![Type_::NatTy, Type_::NatTy]))),
        )),
        PrimOp::PrimOpGtNat
        | PrimOp::PrimOpGeNat
        | PrimOp::PrimOpLtNat
        | PrimOp::PrimOpLeNat
        | PrimOp::PrimOpNeNat => Ok((
            vec![Type_::NatTy, Type_::NatTy],
            unit::<(), PidRef, ExpandedType>(),
        )),
        PrimOp::PrimOpAddNat => Ok((vec![Type_::NatTy, Type_::NatTy], Type_::NatTy)),
        PrimOp::PrimOpNeExpr => {
            let x = fresh_ty_var(st)?;
            Ok((vec![x.clone(), x], unit::<(), PidRef, ExpandedType>()))
        }
    }
}

// -----------------------------------------------------------------------------
// VarSet / varsPat / varsQuery (Typecheck.hs:964-1007)
// -----------------------------------------------------------------------------

/// @type VarSet = HashSet Name@ (:964).
type VarSet = HashSet<Name>;

/// Variables visible in the pattern's enclosing scope (source comment
/// :966-970): or-patterns, negations and if statements don't bring
/// variables into scope. Fold order is the source's foldr in every arm but
/// ArrayPrefix, which folds head-then-tail in reverse; the result is a
/// HashSet, so the order is unobservable.
fn vars_pat<ST>(pat: &Pat<ST>, r: VarSet) -> VarSet {
    match pat {
        // Variable _ v -> HashSet.insert v r
        SourcePat_::Variable(_, v) => {
            let mut r = r;
            r.insert(v.clone());
            r
        }
        // Array _ ps -> foldr varsPat r ps
        SourcePat_::Array(_, ps) => ps.iter().rev().fold(r, |acc, p| vars_pat(p, acc)),
        // ArrayPrefix _ ps -> foldr varsPat r ps
        SourcePat_::ArrayPrefix(_, pre) => {
            let acc = vars_pat(&pre.head, r);
            pre.tail.iter().rev().fold(acc, |acc, p| vars_pat(p, acc))
        }
        // Tuple _ ps -> foldr varsPat r ps
        SourcePat_::Tuple(_, ps) => ps.iter().rev().fold(r, |acc, p| vars_pat(p, acc)),
        // Struct _ fs -> foldr (\(Field _ p) r -> varsPat p r) r fs
        SourcePat_::Struct(_, fs) => fs.iter().rev().fold(r, |acc, Field(_, p)| vars_pat(p, acc)),
        // App _ f ps -> varsPat f $! foldr varsPat r ps
        SourcePat_::App(_, f, ps) => {
            let acc = ps.iter().rev().fold(r, |acc, p| vars_pat(p, acc));
            vars_pat(f, acc)
        }
        // KeyValue _ k v -> varsPat k (varsPat v r)
        SourcePat_::KeyValue(_, k, v) => vars_pat(k, vars_pat(v, r)),
        // ElementsOfArray _ p -> varsPat p r
        SourcePat_::ElementsOfArray(_, p) => vars_pat(p, r),
        // All _ qs -> varsPat qs r
        SourcePat_::All(_, qs) => vars_pat(qs, r),
        // Elements _ pat -> varsPat pat r
        SourcePat_::Elements(_, p) => vars_pat(p, r),
        // OrPattern{} -> r -- ignore nested or-patterns. Note (1) above
        SourcePat_::OrPattern(..) => r,
        // IfPattern{} -> r -- ignore nested if-patterns
        SourcePat_::IfPattern { .. } => r,
        // NestedQuery _ q -> varsQuery q r
        SourcePat_::NestedQuery(_, q) => vars_query(q, r),
        // Negation{} -> r -- Note (1) above
        SourcePat_::Negation(..) => r,
        // TypeSignature _ p _ -> varsPat p r
        SourcePat_::TypeSignature(_, p, _) => vars_pat(p, r),
        // Nat{} / String{} / StringPrefix{} / ByteArray{} / Wildcard{} /
        // FactId{} / Never{} / Enum{} -> r
        SourcePat_::Nat(..)
        | SourcePat_::String(..)
        | SourcePat_::StringPrefix(..)
        | SourcePat_::ByteArray(..)
        | SourcePat_::Wildcard(..)
        | SourcePat_::FactId(..)
        | SourcePat_::Never(..)
        | SourcePat_::Enum(..) => r,
        // Clause _ _ _ p _ -> varsPat p r
        SourcePat_::Clause(_, _, _, p, _) => vars_pat(p, r),
        // Prim _ _ ps -> foldr varsPat r ps
        SourcePat_::Prim(_, _, ps) => ps.iter().rev().fold(r, |acc, p| vars_pat(p, acc)),
        // FieldSelect _ p _ _ -> varsPat p r
        SourcePat_::FieldSelect(_, p, _, _) => vars_pat(p, r),
        // Deref _ p -> varsPat p r
        SourcePat_::Deref(_, p) => vars_pat(p, r),
    }
}

/// @varsQuery (SourceQuery head stmts _) r =
///   foldr varsStmt (foldr varsPat r head) stmts@ (:1002-1007).
fn vars_query<ST>(q: &Query<ST>, r: VarSet) -> VarSet {
    let acc = match &q.src_query_head {
        Some(h) => vars_pat(h, r),
        None => r,
    };
    q.src_query_stmts.iter().rev().fold(acc, |acc, s| {
        let SourceStatement_(a, b) = s;
        vars_pat(a, vars_pat(b, acc))
    })
}

// -----------------------------------------------------------------------------
// tcQueryDeps (Typecheck.hs:1009-1051)
// -----------------------------------------------------------------------------

/// Capture all predicates that appear in a query. (Source haddock :1008.)
///
/// @Set.fromList $ map getRef (overQuery q)@: the BTreeSet both dedups and
/// orders, which is exactly Data.Set's behaviour (the container image is
/// RUNBOOK.md:59's Set -> BTreeSet).
///
/// The source takes TcQuery by value; the image borrows it. Haskell shares
/// the value (Prune.hs:230-240 walks the same query afterwards), and a
/// by-value Rust parameter would force the caller to clone the whole query.
pub fn tc_query_deps(q: &TcQuery) -> BTreeSet<PredicateId> {
    let mut refs: Vec<PidRef> = Vec::new();
    over_query(q, &mut refs);
    refs.into_iter().map(|r| r.predicate_id).collect()
}

fn over_query(q: &TcQuery, out: &mut Vec<PidRef>) {
    let TcQuery(ty, key, mval, stmts, _) = q;
    over_type(ty, out);
    over_pat(key, out);
    if let Some(v) = mval {
        over_pat(v, out);
    }
    for s in stmts.iter() {
        over_statement(s, out);
    }
}

/// @overType ty = bifoldMap pure (\(ExpandedType _ ty) -> overType ty) ty@.
fn over_type(ty: &Type, out: &mut Vec<PidRef>) {
    ty.bifold_map(
        out,
        &mut |pref: &PidRef, out: &mut Vec<PidRef>| out.push(pref.clone()),
        &mut |expanded: &ExpandedType, out: &mut Vec<PidRef>| {
            over_type(&expanded.expanded_type_type, out)
        },
    );
}

fn over_statement(s: &TcStatement, out: &mut Vec<PidRef>) {
    let TcStatement(ty, lhs, rhs) = s;
    over_type(ty, out);
    over_pat(lhs, out);
    over_pat(rhs, out);
}

/// @overPat pat = foldMap (bifoldMap overTyped (const mempty)) pat@ — the
/// Term-level foldMap visits the Refs, and Match's bifoldMap splits the
/// Ext (Typed TcTerm) from the Var (which contributes nothing).
fn over_pat(pat: &TcPat, out: &mut Vec<PidRef>) {
    pat.fold_map(
        out,
        &mut |m: &Match<Typed<TcTerm>, Var>, out: &mut Vec<PidRef>| {
            m.bifold_map(
                out,
                &mut |t: &Typed<TcTerm>, out: &mut Vec<PidRef>| over_typed(t, out),
                &mut |_v: &Var, _out: &mut Vec<PidRef>| {},
            );
        },
    );
}

fn over_typed(t: &Typed<TcTerm>, out: &mut Vec<PidRef>) {
    let Typed(ty, tc_term) = t;
    over_type(ty, out);
    over_term(tc_term, out);
}

fn over_term(t: &TcTerm, out: &mut Vec<PidRef>) {
    match t {
        TcTerm::TcOr(x, y) => {
            over_pat(x, out);
            over_pat(y, out);
        }
        TcTerm::TcFactGen(pref, x, y, _) => {
            out.push(pref.clone());
            over_pat(x, out);
            over_pat(y, out);
        }
        TcTerm::TcElementsOfArray(x) => over_pat(x, out),
        TcTerm::TcElementsOfSet(x) => over_pat(x, out),
        TcTerm::TcElementsUnresolved(_, x) => over_pat(x, out),
        TcTerm::TcQueryGen(q) => over_query(q, out),
        TcTerm::TcAll(q) => over_query(q, out),
        TcTerm::TcNegation(stmts) => {
            for s in stmts.iter() {
                over_statement(s, out);
            }
        }
        TcTerm::TcPrimCall(_, xs) => {
            for x in xs.iter() {
                over_pat(x, out);
            }
        }
        TcTerm::TcIf { cond, then_, else_ } => {
            over_pat(&cond.1, out);
            over_pat(then_, out);
            over_pat(else_, out);
        }
        TcTerm::TcDeref(_, p) => over_pat(p, out),
        TcTerm::TcFieldSelect(t, _) => over_pat(&t.1, out),
        TcTerm::TcAltSelect(t, _) => over_pat(&t.1, out),
        TcTerm::TcPromote(_, p) => over_pat(p, out),
        TcTerm::TcDemote(_, p) => over_pat(p, out),
        TcTerm::TcStructPat(fs) => {
            for (_, p) in fs.iter() {
                over_pat(p, out);
            }
        }
    }
}

// -----------------------------------------------------------------------------
// UseOfNegation / tcQueryUsesNegation (Typecheck.hs:1053-1109)
// -----------------------------------------------------------------------------

/// @data UseOfNegation = PatternNegation | IfStatement@ (:1053-1055). No
/// deriving clause in the source, so none here; consumers only match.
pub enum UseOfNegation {
    PatternNegation,
    IfStatement,
}

/// Whether a query uses negation in its definition. Does not check for
/// transitive uses of negation. (Source haddock :1057-1058.)
///
/// Borrowed argument, for the same reason as tc_query_deps: the consumer
/// (Database/Schema.hs:956-959) builds the value and the dependency list
/// from the same TcQuery.
pub fn tc_query_uses_negation(q: &TcQuery) -> Option<UseOfNegation> {
    let TcQuery(_, _, _, stmts, _) = q;
    stmts.iter().find_map(tc_statement_uses_negation)
}

/// @firstJust f xs@ is find_map: the first Some, short-circuiting.
fn tc_statement_uses_negation(s: &TcStatement) -> Option<UseOfNegation> {
    let TcStatement(_, lhs, rhs) = s;
    tc_pat_uses_negation(lhs).or_else(|| tc_pat_uses_negation(rhs))
}

fn tc_pat_uses_negation(p: &TcPat) -> Option<UseOfNegation> {
    match p {
        Term::Byte(_) | Term::Nat(_) | Term::ByteArray(_) | Term::String(_) => None,
        Term::Array(xs) | Term::Tuple(xs) => xs.iter().find_map(tc_pat_uses_negation),
        Term::Alt(_, t) => tc_pat_uses_negation(t),
        Term::Ref(m) => match_uses_negation(m),
    }
}

fn match_uses_negation(m: &Match<Typed<TcTerm>, Var>) -> Option<UseOfNegation> {
    match m {
        Match::MatchWild(_)
        | Match::MatchNever(_)
        | Match::MatchFid(_)
        | Match::MatchBind(_)
        | Match::MatchVar(_) => None,
        Match::MatchAnd(one, two) => {
            tc_pat_uses_negation(one).or_else(|| tc_pat_uses_negation(two))
        }
        Match::MatchPrefix(_, x) => tc_pat_uses_negation(x),
        Match::MatchArrayPrefix(_, prefix, all) => prefix
            .iter()
            .find_map(tc_pat_uses_negation)
            .or_else(|| tc_pat_uses_negation(all)),
        Match::MatchExt(Typed(_, t)) => tc_term_uses_negation(t),
    }
}

fn tc_term_uses_negation(t: &TcTerm) -> Option<UseOfNegation> {
    match t {
        TcTerm::TcOr(x, y) => tc_pat_uses_negation(x).or_else(|| tc_pat_uses_negation(y)),
        TcTerm::TcFactGen(_, x, y, _) => {
            tc_pat_uses_negation(x).or_else(|| tc_pat_uses_negation(y))
        }
        TcTerm::TcElementsOfArray(x) => tc_pat_uses_negation(x),
        TcTerm::TcElementsOfSet(x) => tc_pat_uses_negation(x),
        TcTerm::TcElementsUnresolved(_, p) => tc_pat_uses_negation(p),
        TcTerm::TcQueryGen(q) => tc_query_uses_negation(q),
        TcTerm::TcAll(q) => tc_query_uses_negation(q),
        TcTerm::TcNegation(_) => Some(UseOfNegation::PatternNegation),
        TcTerm::TcPrimCall(_, xs) => xs.iter().find_map(tc_pat_uses_negation),
        // one can replicate negation using if statements
        TcTerm::TcIf { .. } => Some(UseOfNegation::IfStatement),
        TcTerm::TcDeref(_, p) => tc_pat_uses_negation(p),
        TcTerm::TcFieldSelect(t, _) => tc_pat_uses_negation(&t.1),
        TcTerm::TcAltSelect(t, _) => tc_pat_uses_negation(&t.1),
        TcTerm::TcPromote(_, p) => tc_pat_uses_negation(p),
        TcTerm::TcDemote(_, p) => tc_pat_uses_negation(p),
        TcTerm::TcStructPat(fs) => fs.iter().find_map(|(_, p)| tc_pat_uses_negation(p)),
    }
}

// -----------------------------------------------------------------------------
// Promotion (Typecheck.hs:1190-1292; the Note [Promotion] at :1111-1188 is
// the source's rationale — see the file header's reference to it)
// -----------------------------------------------------------------------------

/// @promote s pat t@ (:1190-1194) — invent a fresh type variable, record
/// the potential promotion, and wrap the pattern.
fn promote(st: &mut TypecheckState, span: &SrcSpan, pat: TcPat, t: Type) -> T<(TcPat, Type)> {
    let y = fresh_ty_var(st)?;
    add_promote(st, span, &t, &y);
    Ok((
        Term::Ref(Match::MatchExt(Typed(
            y.clone(),
            TcTerm::TcPromote(t, Box::new(pat)),
        ))),
        y,
    ))
}

/// The image of @promoteTo/demoteTo@'s result type: a function from TcPat
/// to TcPat, i.e. the Haskell's @TcPat -> TcPat@. The source returns
/// @id@ on the no-op paths, which is the identity closure here.
type PatFn = Box<dyn Fn(TcPat) -> TcPat>;

/// @promoteTo s t u@ (:1196-1219). Arm order matters: the first arm panics
/// when the SOURCE type is a TyVar, the second handles a TyVar TARGET.
fn promote_to(st: &mut TypecheckState, span: &SrcSpan, t: &Type, u: &Type) -> T<PatFn> {
    // promoteTo _ TyVar{} _ = error "promote: TyVar"
    //   -- we never promote an unknown type
    if let Type_::TyVar(_) = t {
        panic!("promote: TyVar");
    }
    // promoteTo s t (TyVar x) = do
    //   subst <- gets tcSubst
    //   case IntMap.lookup x subst of
    //     Just u -> promoteTo s t u
    //     Nothing -> do addPromote s t (TyVar x)
    //       return (Ref . MatchExt . Typed (TyVar x) . TcPromote t)
    if let Type_::TyVar(x) = u {
        match st.tc_subst.get(x).cloned() {
            Some(u2) => return promote_to(st, span, t, &u2),
            None => {
                let tv = Type_::TyVar(*x);
                add_promote(st, span, t, &tv);
                let t2 = t.clone();
                let f: PatFn = Box::new(move |p: TcPat| {
                    Term::Ref(Match::MatchExt(Typed(
                        tv.clone(),
                        TcTerm::TcPromote(t2.clone(), Box::new(p)),
                    )))
                });
                return Ok(f);
            }
        }
    }
    // promoteTo _ (PredicateTy _ (PidRef p _)) (PredicateTy _ (PidRef q _))
    //   | p == q = return id
    //     -- if not equal, q must be the key of p, so fall through
    //     -- (assume we don't have  predicate P : P)
    if let (Type_::PredicateTy(_, p), Type_::PredicateTy(_, q)) = (t, u) {
        if p.pid == q.pid {
            let f: PatFn = Box::new(|p: TcPat| p);
            return Ok(f);
        }
    }
    // promoteTo s t u@(PredicateTy _ (PidRef _ ref)) = do
    //   PredicateDetails{..} <- getPredicateDetails ref
    //   addErrSpan s $ unify predicateKeyType t
    //   return (Ref . MatchExt . Typed u . TcPromote t)
    if let Type_::PredicateTy(_, pref) = u {
        let key_ty = get_predicate_details(st, &pref.predicate_id)
            .predicate_key_type
            .clone();
        let t2 = t.clone();
        let u2 = u.clone();
        add_err_span(st, span, |st| unify(st, &key_ty, t))?;
        let f: PatFn = Box::new(move |p: TcPat| {
            Term::Ref(Match::MatchExt(Typed(
                u2.clone(),
                TcTerm::TcPromote(t2.clone(), Box::new(p)),
            )))
        });
        return Ok(f);
    }
    // promoteTo s t u = do
    //   -- the target type (u) is not a predicate or a tyvar, so it must be
    //   -- the key type and we can unify directly.
    //   addErrSpan s $ unify t u
    //   return id
    add_err_span(st, span, |st| unify(st, t, u))?;
    let f: PatFn = Box::new(|p: TcPat| p);
    Ok(f)
}

/// Dual to promoteTo. The destination type cannot be a TyVar.
/// (Source comment :1221.) @demoteTo _ _ TyVar{}@ panics; the TyVar SOURCE
/// case substitutes first.
fn demote_to(st: &mut TypecheckState, span: &SrcSpan, t: &Type, u: &Type) -> T<PatFn> {
    // demoteTo _ _ TyVar{} = error "demote: TyVar"
    if let Type_::TyVar(_) = u {
        panic!("demote: TyVar");
    }
    // demoteTo s t@(TyVar x) u = do
    //   subst <- gets tcSubst
    //   case IntMap.lookup x subst of
    //     Just t -> demoteTo s t u
    //     Nothing -> do addPromote s u (TyVar x)
    //       return (Ref . MatchExt . Typed u . TcDemote t)
    if let Type_::TyVar(x) = t {
        match st.tc_subst.get(x).cloned() {
            Some(t2) => return demote_to(st, span, &t2, u),
            None => {
                let tv = Type_::TyVar(*x);
                add_promote(st, span, u, &tv);
                let u2 = u.clone();
                let f: PatFn = Box::new(move |p: TcPat| {
                    Term::Ref(Match::MatchExt(Typed(
                        u2.clone(),
                        TcTerm::TcDemote(tv.clone(), Box::new(p)),
                    )))
                });
                return Ok(f);
            }
        }
    }
    // demoteTo _ (PredicateTy _ (PidRef p _)) (PredicateTy _ (PidRef q _))
    //   | p == q = return id
    if let (Type_::PredicateTy(_, p), Type_::PredicateTy(_, q)) = (t, u) {
        if p.pid == q.pid {
            let f: PatFn = Box::new(|p: TcPat| p);
            return Ok(f);
        }
    }
    // demoteTo s t@(PredicateTy _ (PidRef _ ref)) u = do
    //   PredicateDetails{..} <- getPredicateDetails ref
    //   addErrSpan s $ unify predicateKeyType u
    //   return (Ref . MatchExt . Typed u . TcDemote t)
    if let Type_::PredicateTy(_, pref) = t {
        let key_ty = get_predicate_details(st, &pref.predicate_id)
            .predicate_key_type
            .clone();
        let t2 = t.clone();
        let u2 = u.clone();
        add_err_span(st, span, |st| unify(st, &key_ty, u))?;
        let f: PatFn = Box::new(move |p: TcPat| {
            Term::Ref(Match::MatchExt(Typed(
                u2.clone(),
                TcTerm::TcDemote(t2.clone(), Box::new(p)),
            )))
        });
        return Ok(f);
    }
    // demoteTo s t u = do
    //   -- the source type (t) is not a predicate or a tyvar, so it must be
    //   -- the key type and we can unify directly.
    //   addErrSpan s $ unify t u
    //   return id
    add_err_span(st, span, |st| unify(st, t, u))?;
    let f: PatFn = Box::new(|p: TcPat| p);
    Ok(f)
}

/// @addPromote span from to = modify $ \s ->
///   s { tcPromote = (from, to, Some span) : tcPromote s }@ (:1243-1246).
///
/// The source CONSES, so this prepends: resolvePromote processes the list
/// head-first, and the order of the constraints is observable in which
/// unification error is reported first.
fn add_promote(st: &mut TypecheckState, span: &SrcSpan, from: &Type, to: &Type) {
    st.tc_promote.insert(0, (from.clone(), to.clone(), *span));
}

/// @resolvePromote@ (:1248-1292) — resolve every recorded promotion,
/// deferring the ones whose target is still a type variable until nothing
/// else can be done.
fn resolve_promote(st: &mut TypecheckState) -> T<()> {
    let promotes = st.tc_promote.clone();
    resolve_promote_loop(st, promotes)
}

fn resolve_promote_loop(st: &mut TypecheckState, promotes: Vec<(Type, Type, SrcSpan)>) -> T<()> {
    // loop [] = return ()
    if promotes.is_empty() {
        return Ok(());
    }
    // whenDebug $ liftIO $ hPutStrLn stderr $ show $ vcat
    //   [ "promotes: ", vcat [ displayDefault from <> " -> " <> displayDefault to | ... ] ]
    when_debug(st, |st| {
        let _ = st;
        let mut lines = Vec::with_capacity(promotes.len());
        for (from, to, _) in promotes.iter() {
            lines.push(
                display_default(from)
                    .append(Doc::text(" -> "))
                    .append(display_default(to)),
            );
        }
        let doc = vcat(vec![Doc::text("promotes: "), vcat(lines)]);
        eprintln!("{}", show_doc(&doc));
        Ok(())
    })?;
    // resolved <- resolve promotes False
    let resolved = resolve_promote_once(st, &promotes, false)?;
    // if length resolved < length promotes then loop resolved
    if resolved.len() < promotes.len() {
        resolve_promote_loop(st, resolved)
    } else {
        // only default unconstrained promotions when there's nothing else
        // we can do.
        let resolved2 = resolve_promote_once(st, &resolved, true)?;
        resolve_promote_loop(st, resolved2)
    }
}

fn resolve_promote_once(
    st: &mut TypecheckState,
    promotes: &[(Type, Type, SrcSpan)],
    default_ty_vars: bool,
) -> T<Vec<(Type, Type, SrcSpan)>> {
    let mut out: Vec<(Type, Type, SrcSpan)> = Vec::new();
    for (from, to, span) in promotes.iter() {
        // to' <- apply to
        let to2 = apply(st, to)?;
        match (from, &to2) {
            // (TyVar{}, _) -> error "resolvePromote: tyvar"
            (Type_::TyVar(_), _) => panic!("resolvePromote: tyvar"),
            // (PredicateTy _ (PidRef _ ref), PredicateTy _ (PidRef _ ref'))
            //   | ref == ref' -> return Nothing
            (Type_::PredicateTy(_, p), Type_::PredicateTy(_, q))
                if p.predicate_id == q.predicate_id => {}
            // (_other, PredicateTy _ (PidRef _ ref)) -> do
            //   PredicateDetails{..} <- getPredicateDetails ref
            //   addErrSpan span $ unify predicateKeyType from
            //   return Nothing
            (_, Type_::PredicateTy(_, pref)) => {
                let key_ty = get_predicate_details(st, &pref.predicate_id)
                    .predicate_key_type
                    .clone();
                add_err_span(st, span, |st| unify(st, &key_ty, from))?;
            }
            // (_other, TyVar{}) | not defaultTyVars ->
            //   return (Just (from, to', Some span))
            (_, Type_::TyVar(_)) if !default_ty_vars => {
                out.push((from.clone(), to2, *span));
            }
            // _other -> do addErrSpan span $ unify from to'; return Nothing
            _ => {
                add_err_span(st, span, |st| unify(st, from, &to2))?;
            }
        }
    }
    Ok(out)
}

// -----------------------------------------------------------------------------
// Local pretty-printing helpers (the source uses Compat.Prettyprinter's
// indent and Show (Doc); both surfaces are UNOWNED in the manifest)
// -----------------------------------------------------------------------------

/// prettyprinter's @indent i x = hang i (spaces i <> x)@. display.rs has
/// no indent (grep: zero hits for an indent combinator), so it is
/// re-implemented here rather than routed through a non-existent owner.
fn indent(i: i64, d: Doc) -> Doc {
    hang(i, Doc::text(" ".repeat(i as usize)).append(d))
}

/// TODO(port): @show (Doc ())@ render parity. The source's boundary
/// (Typecheck.hs:92/:131) and its debug prints (:100/:1278) render a Doc
/// through prettyprinter 1.7.1's Show instance — @renderShowS@ over
/// @layoutPretty defaultLayoutOptions@, i.e. the 80-column/ribbon Wadler
/// layout; showString pushes text raw, so there is NO escaping to match.
/// That surface is UNOWNED in the manifest (display.rs:537-546 renders
/// nothing). This private flat-branch stand-in is the fifth copy in the
/// tree — monad.rs:686, glean-angle/src/schema/resolve.rs:1958,
/// glean-angle/src/schema/evolve.rs:1186, and here — and it diverges in
/// two ways:
///   (a) it performs no layout: every group takes its flat branch, so a
///       document layoutPretty would break over 80 columns is emitted on
///       one line;
///   (b) it inherits display.rs's @line_prime = Group(Empty, Line)@: the
///       flat branch is Empty, so vcat joins with NO separator (e.g.
///       "type error:does not match:"), whereas prettyprinter's ungrouped
///       vcat takes the broken branch (a newline).
/// (b) is a display.rs property this file inherits, not a local defect.
/// Owner: the Step-6 judge (these diagnostics are golden-compared there)
/// / display.rs's renderer owner. When display.rs's renderer lands, delete
/// all five copies.
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
// EXPORT PARITY (Typecheck.hs:15-26, ticked item by item)
// -----------------------------------------------------------------------------
//   typecheck             -> pub fn typecheck                    OK
//   typecheckDeriving     -> pub fn typecheck_deriving           OK
//   ToRtsType             -> pub use of monad.rs's alias         OK
//   TcEnv(..)             -> pub use of monad.rs's struct        OK
//   TcOpts(..)            -> pub use of monad.rs's struct        OK
//   defaultTcOpts         -> pub use of monad.rs's fn            OK
//   emptyTcEnv            -> pub use of monad.rs's fn            OK
//   tcQueryDeps           -> pub fn tc_query_deps               OK
//   tcQueryUsesNegation   -> pub fn tc_query_uses_negation       OK
//   UseOfNegation(..)     -> pub enum UseOfNegation              OK
//
// Everything else is private (the source's 40 private top-level functions;
// this file splits resolvePromote and adds the over_* walkers and the two
// local pretty helpers): Pat/Statement/Query/DerivingInfoPrime, Context, VarSet, PatFn,
// typecheck_body, typecheck_deriving_body, last_statement_error,
// needs_result, ignore_result, infer_query, typecheck_query,
// unexpected_value, typecheck_statement, infer_expr, field_select,
// convert_type, typecheck_pattern, tc_fact_generator, matches_key,
// is_fact_id_allowed, false_val/true_val, pat_type_error(_desc), bind_or_use,
// infer_var, var_occ, fresh_variable, free_variables_are_errors,
// unbound_variables_are_errors(_), check_var_case, disjunction, one_branch,
// enclose, check_prim_op_args, prim_op_type, vars_pat, vars_query,
// over_*, tc_*_uses_negation, promote, promote_to, demote_to, add_promote,
// resolve_promote(_loop/_once), indent, show_doc.

// -----------------------------------------------------------------------------
// Marker breakdown (closed set)
// -----------------------------------------------------------------------------
// Port todos: 4 — (1) the DbSchema ownership at typecheck; (2) the
//   unordered-containers name order at free_variables_are_errors; (3) the
//   Show (Doc) render parity at show_doc; (4) typecheck_deriving's by-value
//   TcEnv/PredicateDetails/ToRtsType parameters (a central row-89/90
//   change, owned by rows 98/144).
// Reproduced defects: 0 (the Negation arm's cyclic ord is never forced;
//   see the note there).
// Performance deferrals: 1 — the whole-list clones in needs_result and
//   resolve_promote (one marker).
// Safety notes: 0 (no unsafe code).
// Inventory bug candidates: 0 (the inventory has no row for this file).
//
// PORT STATUS: confidence=high todos=4
