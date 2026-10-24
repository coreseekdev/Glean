// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// Ported from: glean/db/Glean/Query/Typecheck/Unify.hs (442 lines)
// Manifest unit: rust/glean-db/src/query/typecheck/unify.rs
// (migration/manifest.tsv line 91, crate glean-db). RULE §4: one source
// file -> one target file; the manifest path is authoritative.
//
// WHAT THIS FILE IS: the typechecker's UNIFIER and the zonker that runs
// at the end of typechecking. The source's export list is exactly four
// names (Unify.hs:9-14) and the port exports exactly those four:
//
//     unify       — the clause table, Unify.hs:37-168
//     apply       — the substitution walk, Unify.hs:192-196
//     zonkTcQuery — Unify.hs:292-299
//     zonkVars    — Unify.hs:277-290
//
// Everything else is private, as in the source: unifyError (:170-178),
// extend (:180-190), zonkType (:198-203), resolveHas (:208-216),
// apply_/go (:218-275), zonkTcPat (:301-381), zonkMatch (:383-404),
// zonkTcTerm (:406-435), zonkTcStatement (:437-442).
//
// -----------------------------------------------------------------------
// CLAUSE ORDER IS SEMANTICS (RULE §0: the old code is the spec)
// -----------------------------------------------------------------------
//
// `unify` is a flat clause table; the FIRST clause whose pattern and
// guards match fires, and a failed guard falls through to the next
// clause. Reordering or dropping a clause changes which clause handles a
// pair and therefore which error text a failure produces (unifyError
// renders its two arguments in order). The Rust `match` reproduces the
// table arm for arm, in source order, with guards as `if` guards (a
// failed Rust match guard also falls through). The order, with the
// source lines:
//
//   :38-:42  Byte/Byte, Nat/Nat, String/String, Array/Array, Set/Set
//   :43-:52  Record/Record   (length guard; per-field name-or-tuple test)
//   :53-:56  Sum/Sum         (length guard; per-field name equality)
//   :57-:58  Predicate/Predicate (pid equality)
//   :60-:63  Named/Named equal ref, then left-expand, then right-expand
//   :65-:69  Maybe: Maybe/Maybe, Maybe vs Sum, Maybe vs Has, Sum vs
//            Maybe, Has vs Maybe (each non-Maybe/Maybe pair lowers the
//            Maybe payload with lowerMaybe)
//   :71-:75  Enumerated: Enumerated/Enumerated equal list, then the same
//            four lowerEnum pairs
//   :77      Boolean/Boolean
//   :79-:81  TyVar: equal vars, then left bind, then right bind
//   :93-:116 HasTy/HasTy (the big one — see below)
//   :118-:139 HasTy vs RecordTy and HasTy vs SumTy, each preceded by the
//            `Just Sum`/`Just Record` short-circuit that errors instead
//   :141-:142 RecordTy/SumTy vs HasTy (swap)
//   :144-:154 HasKey vs PredicateTy, HasKey/HasKey, PredicateTy vs HasKey
//   :156-:166 ElementsOf vs SetTy, SetTy vs ElementsOf, ElementsOf vs
//            ArrayTy, ArrayTy vs ElementsOf, ElementsOf/ElementsOf
//   :168     the catch-all unifyError
//
// Two sub-orderings inside clauses are load-bearing and are preserved
// verbatim:
//   * HasTy/HasTy (:93-:116): `rec` is chosen FIRST (a `Just`/`Just`
//     disagreement errors before any field is unified), then the
//     Map.mergeA field unification, then the size/rec short-circuits that
//     avoid minting a fresh tyvar, in the source's if/else-if/else order.
//   * extend (:180-:190): `apply t` runs FIRST ("avoid creating a cycle
//     in the substitution"), then the `TyVar y | y == x` no-op guard
//     tests the ORIGINAL argument (not the applied one), then the
//     existing-substitution unify or the insert.
// The HasTy/RecordTy and HasTy/SumTy clauses unify fields BEFORE the
// leftover check and extend LAST (:120-:127, :131-:139); the ElementsOf
// clauses extend FIRST and unify second (:156-:158, :160-:162). Both
// orders are transcribed, not normalised.
//
// -----------------------------------------------------------------------
// THE MONAD IMAGE (row 90's contract — monad.rs, applied, not re-derived)
// -----------------------------------------------------------------------
//
// `T a = StateT TypecheckState (ExceptT (Doc ()) IO) a` (Monad.hs:55)
// images as `T<A> = Result<A, Doc>` with the state threaded as an
// explicit parameter (monad.rs:311, THE MONAD IMAGE in that file's
// header). This unit is row 90's first consumer; the call shapes it
// actually needed, all covered by that contract:
//
//   1. `x <- act` -> `let x = act(st)?`. Every T-action here takes
//      `&mut TypecheckState` first and returns `T<_>`. No unevaluated
//      action is ever passed to a combinator in this file (the only
//      combinator this file calls is prettyError, which is eager), so
//      the FnOnce-thunk half of the contract is unused here.
//   2. `gets tcSubst` / `gets tcVars` / `gets tcDisplayOpts` -> field
//      reads. apply_ reads tcSubst ONCE at entry and walks the snapshot
//      (the source's `subst <- gets tcSubst` then `go_ subst t`), which
//      is why apply_ takes `&TypecheckState` rather than `&mut`: no
//      callback it is given writes state, and a shared borrow states
//      that.
//   3. getPredicateDetails returns a BORROW of the state
//      (monad.rs:529-537, `&'a PredicateDetails`). Both call sites here
//      need to keep using `st` mutably afterwards, so each clones the
//      one Type field it wants (Type is Clone) before the next `&mut`
//      call — the pattern monad.rs:517-523 prescribes. PredicateDetails
//      itself is not Clone (database/schema/types.rs:753-756).
//   4. `modify $ \s -> s { tcSubst = ... }` / `{ tcVars = ... }` ->
//      direct field assignment. TypecheckState's fields are public
//      (monad.rs:360-401).
//   5. STATE ROLLBACK: the source's catchError restores the pre-action
//      state; the `&mut` image cannot (monad.rs, the note at
//      add_err_span). This file has no catch site. Its writes before a
//      later failure — extend-then-unify in the ElementsOf clauses, and
//      the HasTy/RecordTy extend after the leftover check — are visible
//      only if a caller catches and continues, which row 90's analysis
//      says no site does. No new marker: the divergence and its reopen
//      condition are row 90's, recorded there.
//   6. `error "..."` (the partial-function panics below) is ErrorCall,
//      not the ExceptT channel: `panic!` here, matching the source's
//      uncatchability (RULE §2 partial-functions row: same message).
//
// -----------------------------------------------------------------------
// THE Map.mergeA IMAGE (Unify.hs:100-104)
// -----------------------------------------------------------------------
//
//     union <- Map.mergeA
//       Map.preserveMissing
//       Map.preserveMissing
//       (Map.zipWithAMatched $ \_ a b -> do unify a b; return a)
//       fa fb
//
// The merge has three behaviours and exactly one effect:
//   * left-only keys  -> preserveMissing: copied through, no effect;
//   * right-only keys -> preserveMissing: copied through, no effect;
//   * shared keys     -> zipWithAMatched: unify the two field types in
//                        the T monad, keep the LEFT value in the result.
// The image is therefore: start from a clone of `fa` (left priority),
// walk `fa` in ascending key order unifying every key it shares with
// `fb`, then insert `fb`'s keys that are absent from the union. Effect
// order is ASCENDING KEY ORDER over the intersection — Data.Map's
// traversals are in-order, and the applicative combination sequences
// left-to-right; the missing-key branches carry no effects, so collapsing
// them out of the traversal cannot change the observable sequence. This
// matters because a failing unify aborts the merge (ExceptT short-
// circuit): which pair fails first, and therefore which error text
// surfaces, is the ascending order. (Data.Map.Merge.Lazy is an external
// package with no manifest unit; this is the first port of mergeA in the
// tree, so the reading above is recorded for review rather than cited to
// a shipped precedent.)
//
// -----------------------------------------------------------------------
// INVENTORY (RULE §5)
// -----------------------------------------------------------------------
// ZERO rows. `grep -rn 'Typecheck/Unify' migration/inventory.tsv
// migration/inventory.ffi.tsv migration/inventory.protocol.tsv
// migration/inventory.sites.tsv migration/inventory.stm.tsv
// migration/inventory.codeflow.tsv` returns NOTHING (as does
// `grep -rn 'Unify.hs'` over the same six files). The only non-inventory
// hits for the path under migration/ are the depmap edges/order
// (depmap/edges.tsv:647, :666-:674; depmap/order.txt:90), manifest.tsv:91
// and HANDOFF queue text (:202, :226, :469), and
// `grep -rin 'unif' migration/inventory.tsv` is empty. Nothing here is an
// inventory-owned class: no FFI, no STM, no unsafePerformIO, no laziness
// with observable effect, no dynamic exception (`error` is a pure call,
// mapped per RULE §2's partial-functions row). No target_translation to
// diverge from, hence no inventory-divergence flag is possible or needed.
//
// -----------------------------------------------------------------------
// DEPENDENCY TRUTH (migration/depmap/edges.tsv — checked, not assumed)
// -----------------------------------------------------------------------
//   OUT :666 Unify.hs -> angle/Glean/Angle/Types.hs  (Unify.hs:25
//         `import Glean.Angle.Types hiding (Type)`) -> crate glean-angle,
//         `glean_angle::angle::types` (Type_, RecordOrSum, SeekSection,
//         Name; FieldDef_ is reached through glean-core's `FieldDef`
//         alias, which names the same struct).
//   OUT :667 Unify.hs -> angle/Glean/Display.hs      (Unify.hs:27
//         `import Glean.Display`; :23 also imports Compat.Prettyprinter,
//         whose Doc stand-in this tree owns per RULE §1) ->
//         `glean_angle::display` (Doc, Display, display_default, vcat,
//         nest, hang).
//   OUT :668 Unify.hs -> angle/Glean/Schema/Util.hs  (Unify.hs:35
//         `import Glean.Schema.Util`) -> `glean_angle::schema::util`
//         (lower_maybe, lower_enum).
//   OUT :669 Unify.hs -> db/Glean/Database/Schema/Types.hs (Unify.hs:26)
//         -> SAME CRATE, `crate::database::schema::types`
//         (PredicateDetails).
//   OUT :670 Unify.hs -> db/Glean/Query/Codegen/Types.hs (Unify.hs:28)
//         -> SAME CRATE, `crate::query::codegen::types` (Match, Typed,
//         Var).
//   OUT :671 Unify.hs -> db/Glean/Query/Typecheck/Monad.hs (Unify.hs:29)
//         -> SAME CRATE, `crate::query::typecheck::monad` (T,
//         TypecheckState, fresh_ty_var_int, get_predicate_details,
//         mk_wild, pretty_error).
//   OUT :672 Unify.hs -> db/Glean/Query/Typecheck/Types.hs (Unify.hs:30)
//         -> SAME CRATE, `crate::query::typecheck::types` (TcQuery,
//         TcStatement, TcPat, TcTerm, lookup_field).
//   OUT :673 Unify.hs -> hs/Glean/RTS/Term.hs        (Unify.hs:31-33,
//         both the hiding import and `qualified ... as RTS`) -> crate
//         glean-core, `glean_core::rts::term::Term` (the Term
//         constructors zonkTcPat rebuilds).
//   OUT :674 Unify.hs -> hs/Glean/RTS/Types.hs       (Unify.hs:34
//         `import Glean.RTS.Types as RTS`) -> `glean_core::rts::types`
//         (Type, FieldDef; PidRef is the PredicateTy payload the clauses
//         read through, never named).
//   IN  :647 Typecheck.hs (row 92, UNSHIPPED) imports this module and
//         consumes all four exports (Typecheck.hs:106-116: zonkVars, then
//         zonkTcQuery under the catchError at :108, which binds to
//         zonkTcQuery ONLY; :170-171 the deriving path's zonkVars +
//         zonkTcQuery; unify/apply at the infer* sites).
//
// The remaining source imports are library/std only and own no manifest
// unit: Control.Monad (:16), Control.Monad.State (:17), Data.IntMap (:18),
// Data.Map (:19), Data.Map.Merge.Lazy (:20), Data.Maybe (:21), Data.Text
// (:22). IntMap -> BTreeMap and Data.Map -> BTreeMap per
// migration/RUNBOOK.md:59 (ordered containers; the order is read by
// zonkVars below and by resolveHas's Map.toList).
//
// CARGO EDGES: none added — rust/glean-db/Cargo.toml has no
// [dependencies] entries yet; every edge is deferred to the Step-4
// workspace scaffold. A shipped sibling (query/typecheck/types.rs:109-114)
// already imports both, and Step-4 wiring owns the actual edges.
//
// -----------------------------------------------------------------------
// ASSUMED-IMPORT (RULE §4 B4)
// -----------------------------------------------------------------------
// NONE. Every type and function this file consumes is owned by a SHIPPED
// manifest unit (the nine edges above resolve to
// rust/glean-angle/src/{angle/types.rs, display.rs, schema/util.rs},
// rust/glean-core/src/rts/{term.rs, types.rs} and this crate's
// database/schema/types.rs, query/codegen/types.rs,
// query/typecheck/{monad.rs, types.rs}). Nothing comes from generated
// code or from the pinned compiler's newtype images.
//
// Two prettyprinter surfaces are NOT owned by display.rs and are handled
// in-file instead of assumed:
//   * `indent` — display.rs owns `hang` (:342) and `nest` (:325) but not
//     `indent`; the pinned definition is `indent i d = hang i (spaces i
//     <> d)` (prettyprinter-1.7.1 Prettyprinter/Internal.hs:843-847, the
//     package RULE §1/inventory.tsv:141 pin). RULE §1 forbids per-unit
//     combinator forks, so no module-level `indent` is declared here: the
//     one-line definition is the local closure `indent2` in unify_error
//     (:638), called twice (:641, :643).
//   * `show (Doc)` — the render surface is deliberately unowned
//     (display.rs:537-546 renders nothing), so the panic message at
//     zonkTcPat's non-singleton sum pattern needs the same private
//     flat-branch stand-in the tree already carries three of
//     (resolve.rs:1958, evolve.rs:1186, monad.rs:686). The fourth copy is
//     below, with the same port todo.
//
// -----------------------------------------------------------------------
// DERIVING PARITY
// -----------------------------------------------------------------------
// This file declares no types, so there is no deriving clause to mirror.
//
// -----------------------------------------------------------------------
// SIBLING-UNIT CONTRACTS (RULE §4; read, never re-declared)
// -----------------------------------------------------------------------
//   * SHIPPED row 88 rust/glean-db/src/query/typecheck/types.rs owns
//     TcQuery (:133, tuple struct of 5), TcStatement (:190), TcPat
//     (:245, `Term<Match<Typed<TcTerm>, Var>>`), TcTerm (:360, the
//     16-variant enum whose Box-ed payloads rows 91/92 construct with
//     `Box::new`), TcPred (:322 — the 2026-10-24 concrete-SrcSpan
//     ruling, DECISIONS.md) and lookup_field (:615, `&[FieldDef] ->
//     Vec<(&Type, u64)>`, the borrowed pairs zonkTcPat's sum arm reads).
//   * SHIPPED row 87 rust/glean-db/src/query/codegen/types.rs owns
//     Match (:378: MatchWild/MatchNever/MatchFid/MatchBind/MatchVar/
//     MatchAnd/MatchPrefix/MatchArrayPrefix/MatchExt), Var (:323) and
//     Typed (:574, `Typed<X>(pub Type, pub X)`).
//   * SHIPPED row 89 rust/glean-db/src/database/schema/types.rs owns
//     PredicateDetails (:756) and its predicate_key_type (:763) /
//     predicate_value_type (:764) — both `Type`, both Clone.
//   * SHIPPED row 90 rust/glean-db/src/query/typecheck/monad.rs owns the
//     monad image (T :311, TypecheckState :360 with tc_display_opts :391,
//     tc_vars :394, tc_subst :396), fresh_ty_var_int (:501, returns the
//     OLD counter and wraps), get_predicate_details (:529), mk_wild
//     (:566, takes Type BY VALUE and keeps the original on the otherwise
//     arm), pretty_error (:583). Its contract obligations (a)-(c) do not
//     bind this file: (a) tc_rts_type is unused here, (b) the HashSet
//     ordering question belongs to row 92, (c) the DbSchema ownership
//     question belongs to row 92 and the schema producer.
//   * SHIPPED rust/glean-core/src/rts/types.rs owns Type (:250 =
//     Type_<(), PidRef, ExpandedType>), FieldDef (:254), PidRef (:185,
//     Eq/Ord by pid only — :194-:215), ExpandedType (:229, its payload
//     `Box<Type>` is why `&*e.expanded_type_type` is the deref shape) and
//     deref_type (:397). SHIPPED rust/glean-core/src/rts/term.rs owns
//     Term (:130).
//   * SHIPPED rust/glean-angle/src/angle/types.rs owns Type_ (:749:
//     HasTy's map is a BTreeMap<Name, Type>, TyVar/HasKey/ElementsOf
//     carry the i64 var id), RecordOrSum (:794, Copy), SeekSection
//     (:607, Copy, SeekOnAllFacts), FieldDef_ (:875), Name (:369),
//     TUPLE_FIELD (:889 — the constant the tuple test uses, matching
//     `Text.isInfixOf "tuplefield"`). SHIPPED schema/util.rs owns
//     lower_maybe (:78, by value) and lower_enum (:120, by `&[Name]`).
//   * SHIPPED rust/glean-angle/src/display.rs owns Doc (:110), Display
//     (:612), display_default (:592), vcat (:380), nest (:325), hang
//     (:342).
//   * UNSHIPPED row 92 (Typecheck.hs) is the only consumer of the four
//     exports; its catch site (Typecheck.hs:108) guards zonkTcQuery and
//     reads only tcDisplayOpts/tcSubst, which zonkTcQuery only READS —
//     so this file's error-path writes (see THE MONAD IMAGE item 5)
//     stay unobservable there too.

use std::collections::BTreeMap;

use glean_angle::angle::types::{Name, RecordOrSum, SeekSection, Type_, TUPLE_FIELD};
use glean_angle::display::{display_default, hang, nest, vcat, Display, Doc};
use glean_angle::schema::util::{lower_enum, lower_maybe};
use glean_core::rts::term::Term;
use glean_core::rts::types::{FieldDef, Type};

use crate::query::codegen::types::{Match, Typed, Var};
use crate::query::typecheck::monad::{
    fresh_ty_var_int, get_predicate_details, mk_wild, pretty_error, TypecheckState, T,
};
use crate::query::typecheck::types::{lookup_field, TcPat, TcQuery, TcStatement, TcTerm};

// -----------------------------------------------------------------------------
// unify (Unify.hs:37-168)
// -----------------------------------------------------------------------------

/// `unify :: Type -> Type -> T ()` (Unify.hs:37) — the clause table. The
/// arm order below IS the source's clause order (see CLAUSE ORDER IS
/// SEMANTICS in the header); do not sort, merge or hoist arms.
///
/// `a`/`b` are taken by SHARED REFERENCE: the Haskell matches on immutable
/// values and every recursive call shares them, and unifyError renders the
/// ORIGINAL pair, so the function parameters are the error arguments in
/// every arm.
pub fn unify(st: &mut TypecheckState, a: &Type, b: &Type) -> T<()> {
    match (a, b) {
        // unify ByteTy ByteTy = return ()  (:38)
        (Type_::ByteTy, Type_::ByteTy) => Ok(()),
        // unify NatTy NatTy = return ()  (:39)
        (Type_::NatTy, Type_::NatTy) => Ok(()),
        // unify StringTy StringTy = return ()  (:40)
        (Type_::StringTy, Type_::StringTy) => Ok(()),
        // unify (ArrayTy t) (ArrayTy u) = unify t u  (:41)
        (Type_::ArrayTy(t), Type_::ArrayTy(u)) => unify(st, t, u),
        // unify (SetTy t) (SetTy u) = unify t u  (:42)
        (Type_::SetTy(t), Type_::SetTy(u)) => unify(st, t, u),

        // unify a@(RecordTy ts) b@(RecordTy us)
        //   | length ts == length us = forM_ (zip ts us) $ \(FieldDef f t,
        //       FieldDef g u) -> if f == g || compareStructurally
        //                        then unify t u else unifyError a b
        //   where isTuple = all (Text.isInfixOf "tuplefield" . fieldDefName)
        //         compareStructurally = isTuple ts || isTuple us   (:43-:52)
        //
        // The length guard is the clause's only guard: unequal lengths fall
        // through to the catch-all (:168), not to a local error.
        (Type_::RecordTy(ts), Type_::RecordTy(us)) if ts.len() == us.len() => {
            // structural equality for tuples by ignoring field names.
            let is_tuple = |fields: &[FieldDef]| {
                fields
                    .iter()
                    .all(|f| f.field_def_name.contains(TUPLE_FIELD))
            };
            let compare_structurally = is_tuple(ts) || is_tuple(us);
            for (fd, gd) in ts.iter().zip(us.iter()) {
                if fd.field_def_name == gd.field_def_name || compare_structurally {
                    unify(st, &fd.field_def_type, &gd.field_def_type)?;
                } else {
                    return unify_error(st, a, b);
                }
            }
            Ok(())
        }

        // unify a@(SumTy ts) b@(SumTy us)
        //   | length ts == length us = forM_ (zip ts us) $ \(FieldDef f t,
        //       FieldDef g u) -> if f /= g then unifyError a b else unify t u
        //   (:53-:56)
        (Type_::SumTy(ts), Type_::SumTy(us)) if ts.len() == us.len() => {
            for (fd, gd) in ts.iter().zip(us.iter()) {
                if fd.field_def_name != gd.field_def_name {
                    return unify_error(st, a, b);
                }
                unify(st, &fd.field_def_type, &gd.field_def_type)?;
            }
            Ok(())
        }

        // unify (PredicateTy _ (PidRef p _)) (PredicateTy _ (PidRef q _))
        //   | p == q = return ()   (:57-:58)
        // `p`/`q` are the Pids (the FIRST PidRef field); Pid equality is
        // the pid field, which PidRef's own Eq also uses (rts/types.rs:194).
        (Type_::PredicateTy(_, p), Type_::PredicateTy(_, q)) if p.pid == q.pid => Ok(()),

        // unify (NamedTy _ (ExpandedType n _)) (NamedTy _ (ExpandedType m _))
        //   | n == m = return ()   (:60-:61)
        (Type_::NamedTy(_, e1), Type_::NamedTy(_, e2))
            if e1.expanded_type_ref == e2.expanded_type_ref =>
        {
            Ok(())
        }
        // unify (NamedTy _ (ExpandedType _ t)) u = unify t u   (:62)
        (Type_::NamedTy(_, e), _) => unify(st, &*e.expanded_type_type, b),
        // unify t (NamedTy _ (ExpandedType _ u)) = unify t u   (:63)
        (_, Type_::NamedTy(_, e)) => unify(st, a, &*e.expanded_type_type),

        // unify (MaybeTy t) (MaybeTy u) = unify t u   (:65)
        (Type_::MaybeTy(t), Type_::MaybeTy(u)) => unify(st, t, u),
        // unify (MaybeTy t) u@SumTy{} = unify (lowerMaybe t) u   (:66)
        (Type_::MaybeTy(t), Type_::SumTy(_)) => {
            let lowered = lower_maybe((**t).clone());
            unify(st, &lowered, b)
        }
        // unify (MaybeTy t) u@HasTy{} = unify (lowerMaybe t) u   (:67)
        (Type_::MaybeTy(t), Type_::HasTy(..)) => {
            let lowered = lower_maybe((**t).clone());
            unify(st, &lowered, b)
        }
        // unify t@SumTy{} (MaybeTy u) = unify t (lowerMaybe u)   (:68)
        (Type_::SumTy(_), Type_::MaybeTy(u)) => {
            let lowered = lower_maybe((**u).clone());
            unify(st, a, &lowered)
        }
        // unify t@HasTy{} (MaybeTy u) = unify t (lowerMaybe u)   (:69)
        (Type_::HasTy(..), Type_::MaybeTy(u)) => {
            let lowered = lower_maybe((**u).clone());
            unify(st, a, &lowered)
        }

        // unify (EnumeratedTy ns) (EnumeratedTy ms) | ns == ms = return ()
        //   (:71)
        (Type_::EnumeratedTy(ns), Type_::EnumeratedTy(ms)) if ns == ms => Ok(()),
        // unify (EnumeratedTy ns) u@SumTy{} = unify (lowerEnum ns) u   (:72)
        (Type_::EnumeratedTy(ns), Type_::SumTy(_)) => unify(st, &lower_enum(ns), b),
        // unify (EnumeratedTy ns) u@HasTy{} = unify (lowerEnum ns) u   (:73)
        (Type_::EnumeratedTy(ns), Type_::HasTy(..)) => unify(st, &lower_enum(ns), b),
        // unify t@SumTy{} (EnumeratedTy ns) = unify t (lowerEnum ns)   (:74)
        (Type_::SumTy(_), Type_::EnumeratedTy(ns)) => unify(st, a, &lower_enum(ns)),
        // unify t@HasTy{} (EnumeratedTy ns) = unify t (lowerEnum ns)   (:75)
        (Type_::HasTy(..), Type_::EnumeratedTy(ns)) => unify(st, a, &lower_enum(ns)),

        // unify BooleanTy BooleanTy = return ()   (:77)
        (Type_::BooleanTy, Type_::BooleanTy) => Ok(()),

        // unify (TyVar x) (TyVar y) | x == y = return ()   (:79)
        (Type_::TyVar(x), Type_::TyVar(y)) if x == y => Ok(()),
        // unify (TyVar x) t = extend x t   (:80) — t is the SECOND arg.
        (Type_::TyVar(x), _) => extend(st, *x, b),
        // unify t (TyVar x) = extend x t   (:81) — t is the FIRST arg.
        (_, Type_::TyVar(x)) => extend(st, *x, a),

        // {- Unifying HasTy ... -} (source comment, :83-:92)
        //
        // unify a@(HasTy fa ra x) b@(HasTy fb rb y)
        //   | x == y = return ()
        //   | otherwise = do
        //   rec <- case (ra,rb) of
        //     (Just x, Just y) | x /= y -> unifyError a b
        //     (Nothing, _) -> return rb
        //     _otherwise -> return ra
        //   union <- Map.mergeA ...   (see THE Map.mergeA IMAGE)
        //   let size = Map.size union
        //   if size == Map.size fa && ra == rec then extend y a
        //   else if size == Map.size fb && rb == rec then extend x b
        //   else do z <- freshTyVarInt
        //           let all = HasTy union rec z
        //           extend x all
        //           extend y all   (:93-:116)
        //
        // Branch order is semantic: `rec` before the merge (a Sum/Record
        // disagreement errors before any field is unified), then the two
        // no-fresh-tyvar short-circuits in source order.
        (Type_::HasTy(fa, ra, x), Type_::HasTy(fb, rb, y)) => {
            if x == y {
                return Ok(());
            }
            let rec: Option<RecordOrSum> = match (*ra, *rb) {
                (Some(p), Some(q)) if p != q => return unify_error(st, a, b),
                (None, _) => *rb,
                _ => *ra,
            };
            // union <- Map.mergeA preserveMissing preserveMissing
            //   (zipWithAMatched $ \_ a b -> do unify a b; return a) fa fb
            let mut union: BTreeMap<Name, Type> = fa.clone();
            for (k, a_ty) in fa.iter() {
                if let Some(b_ty) = fb.get(k) {
                    unify(st, a_ty, b_ty)?;
                }
            }
            for (k, b_ty) in fb.iter() {
                union.entry(k.clone()).or_insert_with(|| b_ty.clone());
            }
            // if either a or b is the same as the unified type, avoid
            // creating a new type variable.
            let size = union.len();
            if size == fa.len() && *ra == rec {
                extend(st, *y, a)
            } else if size == fb.len() && *rb == rec {
                extend(st, *x, b)
            } else {
                let z = fresh_ty_var_int(st)?;
                let all = Type_::HasTy(union, rec, z);
                extend(st, *x, &all)?;
                extend(st, *y, &all)
            }
        }

        // unify a@(HasTy _ (Just Sum) _) b@RecordTy{} = unifyError a b
        //   (:118-:119)
        (Type_::HasTy(_, Some(RecordOrSum::Sum), _), Type_::RecordTy(_)) => unify_error(st, a, b),
        // unify a@(HasTy m _ x) b@(RecordTy fs) = do
        //   forM_ fs $ \(FieldDef f ty) -> case Map.lookup f m of
        //     Nothing -> return (); Just ty' -> unify ty ty'
        //   when (not (Map.null (foldr (Map.delete . fieldDefName) m fs))) $
        //     unifyError a b
        //   extend x (RecordTy fs)   (:120-:127)
        (Type_::HasTy(m, _, x), Type_::RecordTy(fs)) => {
            for fd in fs.iter() {
                if let Some(ty2) = m.get(&fd.field_def_name) {
                    unify(st, &fd.field_def_type, ty2)?;
                }
            }
            // `m` minus every name in `fs` is non-empty iff some key of `m`
            // is not a field name of `fs` — the foldr/Map.delete test
            // without the intermediate map.
            if m.keys().any(|k| !fs.iter().any(|f| &f.field_def_name == k)) {
                return unify_error(st, a, b);
            }
            extend(st, *x, &Type_::RecordTy(fs.clone()))
        }

        // unify a@(HasTy _ (Just Record) _) b@SumTy{} = unifyError a b
        //   (:129-:130)
        (Type_::HasTy(_, Some(RecordOrSum::Record), _), Type_::SumTy(_)) => unify_error(st, a, b),
        // unify a@(HasTy m _ x) b@(SumTy fs) = do
        //   forM_ fs $ \(FieldDef f ty) -> case Map.lookup f m of
        //     Nothing -> return (); Just ty' -> unify ty ty'
        //   forM_ (Map.keys m) $ \n ->
        //     when (n `notElem` map fieldDefName fs) $ unifyError a b
        //   extend x (SumTy fs)   (:131-:139)
        (Type_::HasTy(m, _, x), Type_::SumTy(fs)) => {
            for fd in fs.iter() {
                if let Some(ty2) = m.get(&fd.field_def_name) {
                    unify(st, &fd.field_def_type, ty2)?;
                }
            }
            for n in m.keys() {
                if !fs.iter().any(|f| &f.field_def_name == n) {
                    return unify_error(st, a, b);
                }
            }
            extend(st, *x, &Type_::SumTy(fs.clone()))
        }

        // unify a@RecordTy{} b@HasTy{} = unify b a   (:141)
        (Type_::RecordTy(_), Type_::HasTy(..)) => unify(st, b, a),
        // unify a@SumTy{} b@HasTy{} = unify b a   (:142)
        (Type_::SumTy(_), Type_::HasTy(..)) => unify(st, b, a),

        // unify (HasKey keyTy x) predTy@(PredicateTy _ (PidRef _ ref)) = do
        //   PredicateDetails{..} <- getPredicateDetails ref
        //   unify predicateKeyType keyTy
        //   extend x predTy   (:144-:147)
        //
        // The details are borrowed from the state; the one Type field is
        // cloned so the borrow ends before the `&mut` calls (THE MONAD
        // IMAGE item 3).
        (Type_::HasKey(key_ty, x), pred_ty @ Type_::PredicateTy(_, pid_ref)) => {
            let key_type = get_predicate_details(st, &pid_ref.predicate_id)
                .predicate_key_type
                .clone();
            unify(st, &key_type, key_ty)?;
            extend(st, *x, pred_ty)
        }
        // unify (HasKey a x) (HasKey b y)
        //   | x == y = return ()
        //   | otherwise = do unify a b
        //                    extend x (HasKey a y)
        //                    extend y (HasKey a x)   (:148-:153)
        //
        // Both extends use the LEFT key type (`a`), not the unified one;
        // `a`/`b` are renamed here so they do not shadow unify's args.
        (Type_::HasKey(key_a, x), Type_::HasKey(key_b, y)) => {
            if x == y {
                return Ok(());
            }
            unify(st, key_a, key_b)?;
            extend(st, *x, &Type_::HasKey(key_a.clone(), *y))?;
            extend(st, *y, &Type_::HasKey(key_a.clone(), *x))
        }
        // unify a@PredicateTy{} b@HasKey{} = unify b a   (:154)
        (Type_::PredicateTy(..), Type_::HasKey(..)) => unify(st, b, a),

        // unify (ElementsOf elemTy x) setTy@(SetTy ty) = do
        //   extend x setTy
        //   unify elemTy ty   (:156-:158) — EXTEND FIRST.
        (Type_::ElementsOf(elem_ty, x), set_ty @ Type_::SetTy(ty)) => {
            extend(st, *x, set_ty)?;
            unify(st, elem_ty, ty)
        }
        // unify a@SetTy{} b@ElementsOf{} = unify b a   (:159)
        (Type_::SetTy(_), Type_::ElementsOf(..)) => unify(st, b, a),
        // unify (ElementsOf elemTy x) arrTy@(ArrayTy ty) = do
        //   extend x arrTy
        //   unify elemTy ty   (:160-:162) — EXTEND FIRST.
        (Type_::ElementsOf(elem_ty, x), arr_ty @ Type_::ArrayTy(ty)) => {
            extend(st, *x, arr_ty)?;
            unify(st, elem_ty, ty)
        }
        // unify a@ArrayTy{} b@ElementsOf{} = unify b a   (:163)
        (Type_::ArrayTy(_), Type_::ElementsOf(..)) => unify(st, b, a),
        // unify (ElementsOf elemTyA avar) b@(ElementsOf elemTyB _) = do
        //   unify elemTyA elemTyB
        //   extend avar b   (:164-:166)
        (Type_::ElementsOf(elem_a, avar), elem_of_b @ Type_::ElementsOf(elem_b, _)) => {
            unify(st, elem_a, elem_b)?;
            extend(st, *avar, elem_of_b)
        }

        // unify a b = unifyError a b   (:168)
        _ => unify_error(st, a, b),
    }
}

// -----------------------------------------------------------------------------
// unifyError (Unify.hs:170-178)
// -----------------------------------------------------------------------------

/// `unifyError a b = do opts <- gets tcDisplayOpts
///   prettyError $ vcat [ "type error:", indent 2 (display opts a)
///                      , "does not match:", indent 2 (display opts b) ]`
/// (Unify.hs:170-178).
///
/// The four lines and their order are the whole message: "type error:",
/// the first type indented 2, "does not match:", the second type indented
/// 2. `indent 2 x` is `hang 2 (spaces 2 <> x)` (prettyprinter-1.7.1
/// Prettyprinter/Internal.hs:843-847) — it PREPENDS two spaces to the
/// first line as well as indenting continuations, so it is not `nest 2`.
/// display.rs owns `hang` but not `indent`, so the definition is a local
/// closure in unify_error (see ASSUMED-IMPORT in the header).
///
/// Polymorphic in the success type, like the source's `T a`: it is used
/// for `T ()` from the table and for `T (Maybe RecordOrSum)` inside the
/// HasTy clause's `rec` selection.
fn unify_error<A>(st: &TypecheckState, a: &Type, b: &Type) -> T<A> {
    // opts <- gets tcDisplayOpts
    let opts = st.tc_display_opts;
    // indent 2 x = hang 2 (spaces 2 <> x)
    let indent2 = |d: Doc| hang(2, Doc::text("  ").append(d));
    pretty_error(vcat(vec![
        Doc::text("type error:"),
        indent2(a.display(&opts)),
        Doc::text("does not match:"),
        indent2(b.display(&opts)),
    ]))
}

// -----------------------------------------------------------------------------
// extend (Unify.hs:180-190)
// -----------------------------------------------------------------------------

/// `extend x t = do t' <- apply t
///   if | TyVar y <- t, y == x -> return ()
///      | otherwise -> case IntMap.lookup x subst of
///          Just u -> unify t' u
///          Nothing -> modify (insert x t')` (Unify.hs:180-190).
///
/// Three details, all load-bearing:
///   * `apply t` runs FIRST — "avoid creating a cycle in the substitution"
///     (source comment, :182). So the stored/compared value is the applied
///     one;
///   * the no-op guard tests the ORIGINAL `t` (the Haskell `if` guards
///     scrutinise `t`, not `t'`): `extend x (TyVar x)` is a no-op even
///     though `t'` may differ;
///   * the existing-substitution branch unifies `t'` (left) with the
///     stored value (right), preserving unifyError's argument order.
fn extend(st: &mut TypecheckState, x: i64, t: &Type) -> T<()> {
    // t' <- apply t
    let t2 = apply(st, t)?;
    // TyVar y <- t, y == x -> return ()
    if let Type_::TyVar(y) = t {
        if *y == x {
            return Ok(());
        }
    }
    // subst <- gets tcSubst; case IntMap.lookup x subst of ...
    // (the stored value is cloned out so the borrow ends before `unify`)
    let existing = st.tc_subst.get(&x).cloned();
    match existing {
        // Just u -> unify t' u
        Some(u) => unify(st, &t2, &u),
        // Nothing -> modify (insert x t')
        None => {
            st.tc_subst.insert(x, t2);
            Ok(())
        }
    }
}

// -----------------------------------------------------------------------------
// apply / apply_ (Unify.hs:192-196, :218-275)
// -----------------------------------------------------------------------------

/// `apply t = apply_ (const (return Nothing)) (\_ _ _ -> return Nothing) t`
/// (Unify.hs:192-196) — the substitution walk with NO defaulting: an
/// unbound tyvar stays a TyVar.
pub fn apply(st: &mut TypecheckState, t: &Type) -> T<Type> {
    // let unbound _ = return Nothing
    //     unboundHas _ _ _ = return Nothing
    apply_(
        st,
        &|_: i64| Ok(None),
        &|_: i64, _: &BTreeMap<Name, Type>, _: Option<RecordOrSum>| Ok(None),
        t,
    )
}

/// `zonkType t = do opts <- gets tcDisplayOpts
///   let unbound x = prettyError $ "ambiguous type: " <> display opts
///                     (TyVar x :: Type)
///   apply_ unbound resolveHas t` (Unify.hs:198-203).
///
/// The unbound-tyvar message is the FIRST error zonkVars/zonkTcQuery can
/// raise for an ambiguous query; its text and the `display opts` rendering
/// of `TyVar x` (which renders as `T<n>`, angle/types.rs:1717) are the
/// source's.
fn zonk_type(st: &mut TypecheckState, t: &Type) -> T<Type> {
    // opts <- gets tcDisplayOpts
    let opts = st.tc_display_opts;
    // let unbound x = prettyError $ "ambiguous type: " <> display opts (TyVar x)
    let unbound = |x: i64| -> T<Option<Type>> {
        // The source annotates (TyVar x :: Type); the alias pins the
        // instantiation, which a bare Type_::TyVar cannot infer.
        pretty_error(Doc::text("ambiguous type: ").append(Type::TyVar(x).display(&opts)))
    };
    apply_(st, &unbound, &resolve_has, t)
}

/// `resolveHas _ fieldmap mr = case mr of
///    Nothing -> rec; Just Record -> rec; Just Sum -> sum
///   where rec = return (Just (RecordTy fields))
///         sum = return (Just (SumTy fields))
///         fields = [FieldDef name ty | (name,ty) <- Map.toList fieldmap]`
/// (Unify.hs:205-216).
///
/// Turns an unbound HasTy into a concrete record or sum so that a query
/// like `{ a = 3 }` is not ambiguous. `Map.toList` is ascending key order,
/// which BTreeMap iteration reproduces (migration/RUNBOOK.md:59); the
/// field list order is observable in the type and in error text.
fn resolve_has(
    _x: i64,
    fieldmap: &BTreeMap<Name, Type>,
    mr: Option<RecordOrSum>,
) -> T<Option<Type>> {
    // fields = [ FieldDef name ty | (name,ty) <- Map.toList fieldmap ]
    let fields: Vec<FieldDef> = fieldmap
        .iter()
        .map(|(name, ty)| FieldDef {
            field_def_name: name.clone(),
            field_def_type: ty.clone(),
        })
        .collect();
    match mr {
        // Nothing -> rec; Just Record -> rec
        None | Some(RecordOrSum::Record) => Ok(Some(Type_::RecordTy(fields))),
        // Just Sum -> sum
        Some(RecordOrSum::Sum) => Ok(Some(Type_::SumTy(fields))),
    }
}

/// `apply_ :: (Int -> T (Maybe Type))
///         -> (Int -> Map Name Type -> Maybe RecordOrSum -> T (Maybe Type))
///         -> Type -> T Type` (Unify.hs:218-225).
///
/// Reads `tcSubst` ONCE (`subst <- gets tcSubst`) and walks that snapshot:
/// the two callbacks receive no state, so the snapshot is a shared borrow
/// and the whole walk is a pure function of it. `unbound` decides what an
/// unbound plain tyvar means (`apply`: keep it; `zonkType`/`zonkVars`:
/// error) and `unboundHas` decides what an unbound HasTy means (`apply`:
/// keep it; the zonkers: resolve_has).
fn apply_<F, G>(st: &TypecheckState, unbound: &F, unbound_has: &G, t: &Type) -> T<Type>
where
    F: Fn(i64) -> T<Option<Type>>,
    G: Fn(i64, &BTreeMap<Name, Type>, Option<RecordOrSum>) -> T<Option<Type>>,
{
    // subst <- gets tcSubst
    let subst = &st.tc_subst;
    apply_go(subst, unbound, unbound_has, t)
}

/// `go` from Unify.hs:239-275 — the recursive walk over the snapshot.
///
/// Arm-by-arm, the source returns `t` unchanged for ByteTy, NatTy,
/// StringTy, PredicateTy, NamedTy, EnumeratedTy and BooleanTy (no
/// recursion into PredicateTy/NamedTy/EnumeratedTy payloads — the source
/// deliberately leaves those unzonked), rebuilds ArrayTy/RecordTy/SumTy/
/// MaybeTy/SetTy over the walked payloads, and follows the substitution at
/// TyVar/HasTy/HasKey/ElementsOf. A TyVar/HasTy whose id is absent calls
/// the matching callback and, if that returns Nothing, keeps the ORIGINAL
/// node (`return t`).
// PERF(port): the source SHARES the value in every `return t` arm (and in
// the subst-hit arms, where `go u` walks the stored Type by reference);
// this port returns owned Types, so each pass-through deep-clones —
// including a NamedTy's whole `ExpandedType` payload and an unresolved
// HasTy's field map — while each subst hit is walked by reference (no
// clone there). apply runs from every `extend`, so a query with many
// named/expanded types pays copies the Haskell does not. The same
// sharing gap drives three more clones the reviewers found: the merge's
// `fa.clone()` and its right-only `b_ty.clone()` (:473, :480) and
// `extend`'s `existing` clone (:677). The fast version needs sharing in
// `Type` itself (Rc<Type> fields, or Cow<Type> returns), which is
// glean-core's type to change, not this unit's; deferred to the Step-4
// consolidation, not fixed here.
fn apply_go<F, G>(subst: &BTreeMap<i64, Type>, unbound: &F, unbound_has: &G, t: &Type) -> T<Type>
where
    F: Fn(i64) -> T<Option<Type>>,
    G: Fn(i64, &BTreeMap<Name, Type>, Option<RecordOrSum>) -> T<Option<Type>>,
{
    match t {
        // ByteTy -> return t; NatTy -> return t; StringTy -> return t
        Type_::ByteTy => Ok(t.clone()),
        Type_::NatTy => Ok(t.clone()),
        Type_::StringTy => Ok(t.clone()),
        // ArrayTy t -> ArrayTy <$> go t   (:243)
        Type_::ArrayTy(x) => Ok(Type_::ArrayTy(Box::new(apply_go(
            subst,
            unbound,
            unbound_has,
            x,
        )?))),
        // RecordTy fs -> fmap RecordTy $ forM fs $ \(FieldDef n t) ->
        //   FieldDef n <$> go t   (:244-:247)
        Type_::RecordTy(fs) => {
            let mut out = Vec::with_capacity(fs.len());
            for fd in fs.iter() {
                out.push(FieldDef {
                    field_def_name: fd.field_def_name.clone(),
                    field_def_type: apply_go(subst, unbound, unbound_has, &fd.field_def_type)?,
                });
            }
            Ok(Type_::RecordTy(out))
        }
        // SumTy fs -> fmap SumTy $ forM fs $ \(FieldDef n t) ->
        //   FieldDef n <$> go t   (:248-:251)
        Type_::SumTy(fs) => {
            let mut out = Vec::with_capacity(fs.len());
            for fd in fs.iter() {
                out.push(FieldDef {
                    field_def_name: fd.field_def_name.clone(),
                    field_def_type: apply_go(subst, unbound, unbound_has, &fd.field_def_type)?,
                });
            }
            Ok(Type_::SumTy(out))
        }
        // PredicateTy{} -> return t; NamedTy{} -> return t   (:250, :251)
        Type_::PredicateTy(..) => Ok(t.clone()),
        Type_::NamedTy(..) => Ok(t.clone()),
        // MaybeTy t -> MaybeTy <$> go t   (:252)
        Type_::MaybeTy(x) => Ok(Type_::MaybeTy(Box::new(apply_go(
            subst,
            unbound,
            unbound_has,
            x,
        )?))),
        // EnumeratedTy{} -> return t; BooleanTy -> return t   (:253, :254)
        Type_::EnumeratedTy(_) => Ok(t.clone()),
        Type_::BooleanTy => Ok(t.clone()),
        // TyVar x -> do m <- lookup x; case m of
        //   Nothing -> return t; Just u -> go u   (:255-:260)
        Type_::TyVar(x) => match subst.get(x) {
            // lookup x = case IntMap.lookup x subst of
            //   Nothing -> unbound x; Just ty -> return (Just ty)
            None => match unbound(*x)? {
                None => Ok(t.clone()),
                Some(u) => apply_go(subst, unbound, unbound_has, &u),
            },
            Some(u) => apply_go(subst, unbound, unbound_has, u),
        },
        // HasTy f r x -> do m <- lookupHas x f r; case m of
        //   Nothing -> return t; Just u -> go u   (:261-:265)
        Type_::HasTy(f, r, x) => match subst.get(x) {
            None => match unbound_has(*x, f, *r)? {
                None => Ok(t.clone()),
                Some(u) => apply_go(subst, unbound, unbound_has, &u),
            },
            Some(u) => apply_go(subst, unbound, unbound_has, u),
        },
        // HasKey _ x -> do m <- lookup x; ...   (:266-:269)
        Type_::HasKey(_, x) => match subst.get(x) {
            None => match unbound(*x)? {
                None => Ok(t.clone()),
                Some(u) => apply_go(subst, unbound, unbound_has, &u),
            },
            Some(u) => apply_go(subst, unbound, unbound_has, u),
        },
        // SetTy t -> SetTy <$> go t   (:270)
        Type_::SetTy(x) => Ok(Type_::SetTy(Box::new(apply_go(
            subst,
            unbound,
            unbound_has,
            x,
        )?))),
        // ElementsOf _ x -> do m <- lookup x; ...   (:271-:275)
        Type_::ElementsOf(_, x) => match subst.get(x) {
            None => match unbound(*x)? {
                None => Ok(t.clone()),
                Some(u) => apply_go(subst, unbound, unbound_has, &u),
            },
            Some(u) => apply_go(subst, unbound, unbound_has, u),
        },
    }
}

// -----------------------------------------------------------------------------
// zonkVars (Unify.hs:277-290)
// -----------------------------------------------------------------------------

/// `zonkVars = do vars <- gets tcVars
///   zonked <- forM vars $ \Var{..} -> do
///     let unbound _ = prettyError $ vcat
///           [ "variable " <> pretty var <> " has unknown type"
///           , "    try adding a type signature, like: " <> pretty var
///             <> " : T" ]
///         where var = fromMaybe (Text.pack ('_':show varId)) varOrigName
///     t <- apply_ unbound resolveHas varType
///     return (Var { varType = t, ..})
///   modify $ \s -> s { tcVars = zonked }` (Unify.hs:277-290).
///
/// `forM` over an IntMap traverses in ASCENDING KEY ORDER and ExceptT
/// short-circuits, so the FIRST variable (lowest id) that fails to zonk
/// decides the error text. tc_vars is a BTreeMap<i64, Var> precisely to
/// keep that order (monad.rs:394; the rationale is monad.rs:276-283).
/// The write-back happens once, after every variable succeeded —
/// a failure leaves tcVars untouched, as in the source (the `modify` is
/// the last statement).
pub fn zonk_vars(st: &mut TypecheckState) -> T<()> {
    // vars <- gets tcVars
    let mut zonked: BTreeMap<i64, Var> = BTreeMap::new();
    for (id, var) in st.tc_vars.iter() {
        // var = fromMaybe (Text.pack ('_':show varId)) varOrigName
        let name = var
            .var_orig_name
            .clone()
            .unwrap_or_else(|| format!("_{}", var.var_id));
        // let unbound _ = prettyError $ vcat [ ... ]
        let unbound = |_x: i64| -> T<Option<Type>> {
            pretty_error(vcat(vec![
                Doc::text("variable ")
                    .append(Doc::text(name.clone()))
                    .append(Doc::text(" has unknown type")),
                Doc::text("    try adding a type signature, like: ")
                    .append(Doc::text(name.clone()))
                    .append(Doc::text(" : T")),
            ]))
        };
        // t <- apply_ unbound resolveHas varType
        let t = apply_(st, &unbound, &resolve_has, &var.var_type)?;
        // return (Var { varType = t, ..})
        zonked.insert(
            *id,
            Var {
                var_type: t,
                var_id: var.var_id,
                var_orig_name: var.var_orig_name.clone(),
            },
        );
    }
    // modify $ \s -> s { tcVars = zonked }
    st.tc_vars = zonked;
    Ok(())
}

// -----------------------------------------------------------------------------
// zonkTcQuery (Unify.hs:292-299)
// -----------------------------------------------------------------------------

/// `zonkTcQuery (TcQuery ty k mv stmts ord) = TcQuery
///    <$> zonkType ty <*> zonkTcPat k <*> mapM zonkTcPat mv
///    <*> mapM zonkTcStatement stmts <*> pure ord` (Unify.hs:292-299).
///
/// Effect order is the applicative's: ty, key, maybe-value, statements,
/// then `pure ord` (no effect). Typecheck.hs:107-108 calls this under a
/// catchError that replaces the error with "query has ambiguous type", so
/// every failure from this subtree is caught there.
pub fn zonk_tc_query(st: &mut TypecheckState, q: &TcQuery) -> T<TcQuery> {
    let TcQuery(ty, k, mv, stmts, ord) = q;
    let ty = zonk_type(st, ty)?;
    let k = zonk_tc_pat(st, k)?;
    let mv = match mv {
        Some(p) => Some(zonk_tc_pat(st, p)?),
        None => None,
    };
    let mut stmts2 = Vec::with_capacity(stmts.len());
    for s in stmts.iter() {
        stmts2.push(zonk_tc_statement(st, s)?);
    }
    Ok(TcQuery(ty, k, mv, stmts2, *ord))
}

// -----------------------------------------------------------------------------
// zonkTcPat (Unify.hs:301-381)
// -----------------------------------------------------------------------------

/// `zonkTcPat` (Unify.hs:301-381) — zonk a TcPat. Leaf Terms pass through
/// unchanged (Byte/Nat/ByteArray/String); Array/Tuple/Alt recurse; a Ref
/// is handed to zonkMatch EXCEPT for the four Typed TcTerm shapes that
/// must be resolved here, because their TcTerm variants are placeholders
/// that zonkTcTerm refuses to see (:432-435):
///
///   * TcPromote: when the zonked type and the zonked inner type are
///     predicates with the same predicate id, the promote disappears
///     (`return e'`); when the type is a predicate and the inner type is
///     anything else, it becomes a fact generator over the predicate's
///     value type with a wildcard value pattern and SeekOnAllFacts
///     (:310-:325). A zonked TyVar on either side is `error "zonkMatch:
///     tyvar"` — a panic, not an ExceptT error.
///   * TcDemote: same predicate-id shortcut; a predicate inner type
///     becomes TcDeref over the ZONKED inner type (:326-:336).
///   * TcStructPat: a zonked RecordTy turns the struct pattern into a
///     positional Tuple, missing fields becoming mkWild of the field type
///     (:337-:348); a zonked SumTy requires exactly one field whose name
///     resolves via lookupField, becoming `Alt index pat` (:349-:354);
///     anything else is the "type error in pattern" message (:355-:361).
///   * TcElementsUnresolved: a zonked SetTy/ArrayTy picks
///     TcElementsOfSet/TcElementsOfArray, else the same message
///     (:362-:379).
///
/// In BOTH "type error in pattern" messages the displayed expected type is
/// the ORIGINAL (unzonked) `ty`, not ty' — the source renders `ty`
/// (:359, :377) — so the port keeps the original binding alive and
/// displays it, not the zonked value.
fn zonk_tc_pat(st: &mut TypecheckState, p: &TcPat) -> T<TcPat> {
    match p {
        // RTS.Byte{} -> return p; RTS.Nat{} -> return p   (:303-:304)
        Term::Byte(_) => Ok(p.clone()),
        Term::Nat(_) => Ok(p.clone()),
        // RTS.Array ts -> RTS.Array <$> mapM zonkTcPat ts   (:305)
        Term::Array(ts) => {
            let mut out = Vec::with_capacity(ts.len());
            for t in ts.iter() {
                out.push(zonk_tc_pat(st, t)?);
            }
            Ok(Term::Array(out))
        }
        // RTS.ByteArray{} -> return p   (:306)
        Term::ByteArray(_) => Ok(p.clone()),
        // RTS.Tuple ts -> RTS.Tuple <$> mapM zonkTcPat ts   (:307)
        Term::Tuple(ts) => {
            let mut out = Vec::with_capacity(ts.len());
            for t in ts.iter() {
                out.push(zonk_tc_pat(st, t)?);
            }
            Ok(Term::Tuple(out))
        }
        // RTS.Alt n t -> RTS.Alt n <$> zonkTcPat t   (:308)
        Term::Alt(n, t) => Ok(Term::Alt(*n, Box::new(zonk_tc_pat(st, t)?))),
        // RTS.String{} -> return p   (:309)
        Term::String(_) => Ok(p.clone()),

        // Ref (MatchExt (Typed ty (TcPromote inner e))) -> ...   (:310-:325)
        Term::Ref(Match::MatchExt(Typed(ty, TcTerm::TcPromote(inner, e)))) => {
            // ty' <- zonkType ty; inner' <- zonkType inner; e' <- zonkTcPat e
            let ty2 = zonk_type(st, ty)?;
            let inner2 = zonk_type(st, inner)?;
            let e2 = zonk_tc_pat(st, e)?;
            // (TyVar{}, _) -> error "zonkMatch: tyvar"
            if matches!(&ty2, Type_::TyVar(_)) {
                panic!("zonkMatch: tyvar");
            }
            // (_, TyVar{}) -> error "zonkMatch: tyvar"
            if matches!(&inner2, Type_::TyVar(_)) {
                panic!("zonkMatch: tyvar");
            }
            // (PredicateTy _ (PidRef _ ref), PredicateTy _ (PidRef _ ref'))
            //   | ref == ref' -> return e'
            // NOTE: the guard compares the PredicateIds (the PidRef's
            // SECOND field), not the Pids.
            let same_predicate = match (&ty2, &inner2) {
                (Type_::PredicateTy(_, p1), Type_::PredicateTy(_, p2)) => {
                    p1.predicate_id == p2.predicate_id
                }
                _ => false,
            };
            if same_predicate {
                return Ok(e2);
            }
            // (PredicateTy _ pidRef@(PidRef _ ref), _other) -> do
            //   PredicateDetails{..} <- getPredicateDetails ref
            //   let vpat = Ref (MatchWild predicateValueType)
            //   return (Ref (MatchExt (Typed ty'
            //     (TcFactGen pidRef e' vpat SeekOnAllFacts))))
            // The PidRef is cloned out of ty2 so the borrow ends before the
            // construction moves ty2 (THE MONAD IMAGE item 3 for the
            // details borrow).
            let pid_ref = match &ty2 {
                Type_::PredicateTy(_, pid_ref) => Some(pid_ref.clone()),
                _ => None,
            };
            match pid_ref {
                Some(pid_ref) => {
                    let value_type = get_predicate_details(st, &pid_ref.predicate_id)
                        .predicate_value_type
                        .clone();
                    let vpat = Term::Ref(Match::MatchWild(value_type));
                    Ok(Term::Ref(Match::MatchExt(Typed(
                        ty2,
                        TcTerm::TcFactGen(
                            pid_ref,
                            Box::new(e2),
                            Box::new(vpat),
                            SeekSection::SeekOnAllFacts,
                        ),
                    ))))
                }
                // _ -> return e'
                None => Ok(e2),
            }
        }

        // Ref (MatchExt (Typed ty (TcDemote inner e))) -> ...   (:326-:336)
        Term::Ref(Match::MatchExt(Typed(ty, TcTerm::TcDemote(inner, e)))) => {
            let ty2 = zonk_type(st, ty)?;
            let inner2 = zonk_type(st, inner)?;
            let e2 = zonk_tc_pat(st, e)?;
            if matches!(&ty2, Type_::TyVar(_)) {
                panic!("zonkMatch: tyvar");
            }
            if matches!(&inner2, Type_::TyVar(_)) {
                panic!("zonkMatch: tyvar");
            }
            // (PredicateTy _ (PidRef _ ref), PredicateTy _ (PidRef _ ref'))
            //   | ref == ref' -> return e'
            let same_predicate = match (&ty2, &inner2) {
                (Type_::PredicateTy(_, p1), Type_::PredicateTy(_, p2)) => {
                    p1.predicate_id == p2.predicate_id
                }
                _ => false,
            };
            if same_predicate {
                return Ok(e2);
            }
            // (_other, PredicateTy{}) ->
            //   return (Ref (MatchExt (Typed ty' (TcDeref inner' e'))))
            if matches!(&inner2, Type_::PredicateTy(..)) {
                return Ok(Term::Ref(Match::MatchExt(Typed(
                    ty2,
                    TcTerm::TcDeref(inner2, Box::new(e2)),
                ))));
            }
            // _ -> return e'
            Ok(e2)
        }

        // Ref (MatchExt (Typed ty (TcStructPat fs))) -> ...   (:337-:361)
        Term::Ref(Match::MatchExt(Typed(ty, TcTerm::TcStructPat(fs)))) => {
            let ty2 = zonk_type(st, ty)?;
            match &ty2 {
                // RecordTy fields -> fmap RTS.Tuple $ forM fields $
                //   \(FieldDef f ty) -> case [p | (g,p) <- fs, f == g] of
                //     [] -> return (mkWild ty)
                //     (x:_) -> zonkTcPat x   (:342-:348)
                Type_::RecordTy(fields) => {
                    let mut out = Vec::with_capacity(fields.len());
                    for fd in fields.iter() {
                        // The first matching pattern wins ((x:_)); field
                        // order is the record's, not the pattern list's.
                        let mut found: Option<&TcPat> = None;
                        for (g, x) in fs.iter() {
                            if g == &fd.field_def_name {
                                found = Some(x);
                                break;
                            }
                        }
                        match found {
                            // [] -> return (mkWild ty)
                            None => out.push(mk_wild(fd.field_def_type.clone())),
                            // (x:_) -> zonkTcPat x
                            Some(x) => out.push(zonk_tc_pat(st, x)?),
                        }
                    }
                    Ok(Term::Tuple(out))
                }
                // SumTy fields -> case fs of
                //   [(name,pat)] | (_, n) :_ <- lookupField name fields ->
                //       RTS.Alt n <$> zonkTcPat pat
                //   _other -> error $ "zonkTcPat: " <> show (displayDefault p)
                //   (:349-:354)
                Type_::SumTy(fields) => match fs.as_slice() {
                    [(name, pat)] => {
                        let found = lookup_field(name, fields);
                        match found.first() {
                            // (_, n) :_ -> do pat' <- zonkTcPat pat
                            //               return (RTS.Alt n pat')
                            Some((_, n)) => {
                                let pat2 = zonk_tc_pat(st, pat)?;
                                Ok(Term::Alt(*n, Box::new(pat2)))
                            }
                            // a singleton pattern naming no field falls to
                            // the same _other arm as a non-singleton list.
                            None => panic!("zonkTcPat: {}", show_doc(&display_default(p))),
                        }
                    }
                    _ => panic!("zonkTcPat: {}", show_doc(&display_default(p))),
                },
                // _other -> prettyError $ nest 4 $ vcat
                //   [ "type error in pattern"
                //   , "pattern: " <> display opts p
                //   , "expected type: " <> display opts ty ]   (:355-:361)
                _ => {
                    let opts = st.tc_display_opts;
                    pretty_error(nest(
                        4,
                        vcat(vec![
                            Doc::text("type error in pattern"),
                            Doc::text("pattern: ").append(p.display(&opts)),
                            Doc::text("expected type: ").append(ty.display(&opts)),
                        ]),
                    ))
                }
            }
        }

        // Ref (MatchExt (Typed ty (TcElementsUnresolved containerTy p))) -> ...
        //   (:362-:379)
        Term::Ref(Match::MatchExt(Typed(
            ty,
            TcTerm::TcElementsUnresolved(container_ty, inner_pat),
        ))) => {
            // ty' <- zonkType ty; containerTy' <- zonkType containerTy;
            // p' <- zonkTcPat p
            let ty2 = zonk_type(st, ty)?;
            let container2 = zonk_type(st, container_ty)?;
            let p2 = zonk_tc_pat(st, inner_pat)?;
            match &container2 {
                // SetTy _ -> return (Ref (MatchExt (Typed ty' (TcElementsOfSet p'))))
                Type_::SetTy(_) => Ok(Term::Ref(Match::MatchExt(Typed(
                    ty2,
                    TcTerm::TcElementsOfSet(Box::new(p2)),
                )))),
                // ArrayTy _ -> return (Ref (MatchExt (Typed ty' (TcElementsOfArray p'))))
                Type_::ArrayTy(_) => Ok(Term::Ref(Match::MatchExt(Typed(
                    ty2,
                    TcTerm::TcElementsOfArray(Box::new(p2)),
                )))),
                // _other -> prettyError $ nest 4 $ vcat [ ... ty ... ]
                _ => {
                    let opts = st.tc_display_opts;
                    pretty_error(nest(
                        4,
                        vcat(vec![
                            Doc::text("type error in pattern"),
                            Doc::text("pattern: ").append(p.display(&opts)),
                            Doc::text("expected type: ").append(ty.display(&opts)),
                        ]),
                    ))
                }
            }
        }

        // Ref m -> Ref <$> zonkMatch m   (:381)
        Term::Ref(m) => Ok(Term::Ref(zonk_match(st, m)?)),
    }
}

// -----------------------------------------------------------------------------
// zonkMatch (Unify.hs:383-404)
// -----------------------------------------------------------------------------

/// `zonkMatch` (Unify.hs:383-404) — zonk every Type in a Match and resolve
/// the bound Vars through tcVars.
///
/// `var (Var _ n _) = do vars <- gets tcVars; case IntMap.lookup n vars of
///   Nothing -> error "zonkMatch"; Just v -> return v` (:400-404): the
/// Match's Var is REPLACED by the tcVars entry, which carries the zonked
/// type; a missing id is a panic ("zonkMatch", no suffix). MatchFid passes
/// through unchanged (it carries no Type).
fn zonk_match(
    st: &mut TypecheckState,
    m: &Match<Typed<TcTerm>, Var>,
) -> T<Match<Typed<TcTerm>, Var>> {
    match m {
        // MatchWild ty -> MatchWild <$> zonkType ty   (:385)
        Match::MatchWild(ty) => Ok(Match::MatchWild(zonk_type(st, ty)?)),
        // MatchNever ty -> MatchNever <$> zonkType ty   (:386)
        Match::MatchNever(ty) => Ok(Match::MatchNever(zonk_type(st, ty)?)),
        // MatchFid{} -> return m   (:387)
        Match::MatchFid(_) => Ok(m.clone()),
        // MatchBind v -> MatchBind <$> var v   (:388)
        Match::MatchBind(v) => Ok(Match::MatchBind(zonk_var(st, v))),
        // MatchVar v -> MatchVar <$> var v   (:389)
        Match::MatchVar(v) => Ok(Match::MatchVar(zonk_var(st, v))),
        // MatchAnd a b -> MatchAnd <$> zonkTcPat a <*> zonkTcPat b   (:390)
        Match::MatchAnd(a, b) => Ok(Match::MatchAnd(
            Box::new(zonk_tc_pat(st, a)?),
            Box::new(zonk_tc_pat(st, b)?),
        )),
        // MatchPrefix s pat -> MatchPrefix s <$> zonkTcPat pat   (:391)
        Match::MatchPrefix(s, pat) => Ok(Match::MatchPrefix(
            s.clone(),
            Box::new(zonk_tc_pat(st, pat)?),
        )),
        // MatchArrayPrefix ty ts all ->
        //   MatchArrayPrefix <$> zonkType ty <*> mapM zonkTcPat ts
        //     <*> zonkTcPat all   (:392-396)
        Match::MatchArrayPrefix(ty, ts, all) => {
            let ty = zonk_type(st, ty)?;
            let mut ts2 = Vec::with_capacity(ts.len());
            for t in ts.iter() {
                ts2.push(zonk_tc_pat(st, t)?);
            }
            let all = zonk_tc_pat(st, all)?;
            Ok(Match::MatchArrayPrefix(ty, ts2, Box::new(all)))
        }
        // MatchExt (Typed ty e) ->
        //   MatchExt <$> (Typed <$> zonkType ty <*> zonkTcTerm e)   (:397-398)
        Match::MatchExt(Typed(ty, e)) => {
            let ty = zonk_type(st, ty)?;
            let e = zonk_tc_term(st, e)?;
            Ok(Match::MatchExt(Typed(ty, e)))
        }
    }
}

/// `var (Var _ n _) = ...` (Unify.hs:400-404) — the where-binding of
/// zonkMatch. Reads tcVars (no write) and CLONES the stored Var, because
/// the source returns the map's value into a new Match.
fn zonk_var(st: &TypecheckState, v: &Var) -> Var {
    match st.tc_vars.get(&v.var_id) {
        // Nothing -> error "zonkMatch"
        None => panic!("zonkMatch"),
        // Just v -> return v
        Some(v) => v.clone(),
    }
}

// -----------------------------------------------------------------------------
// zonkTcTerm (Unify.hs:406-435)
// -----------------------------------------------------------------------------

/// `zonkTcTerm` (Unify.hs:406-435) — zonk a TcTerm's types and patterns.
///
/// Effect order per variant is the applicative's, left to right (e.g.
/// TcIf zonks the cond's Type, then its pattern, then then_/else_). The
/// four placeholder variants are panics: they are resolved by zonkTcPat
/// before a TcTerm is ever handed here (:432-435). The TcDemote panic
/// message says "TcPromote" in the source (:434) — reproduced verbatim,
/// as RULE §2's partial-functions row requires the same message.
fn zonk_tc_term(st: &mut TypecheckState, t: &TcTerm) -> T<TcTerm> {
    match t {
        // TcOr a b -> TcOr <$> zonkTcPat a <*> zonkTcPat b   (:408)
        TcTerm::TcOr(a, b) => Ok(TcTerm::TcOr(
            Box::new(zonk_tc_pat(st, a)?),
            Box::new(zonk_tc_pat(st, b)?),
        )),
        // TcFactGen pid k v sec ->
        //   TcFactGen pid <$> zonkTcPat k <*> zonkTcPat v <*> pure sec
        //   (:409-410)
        TcTerm::TcFactGen(pid, k, v, sec) => {
            let k = zonk_tc_pat(st, k)?;
            let v = zonk_tc_pat(st, v)?;
            Ok(TcTerm::TcFactGen(
                pid.clone(),
                Box::new(k),
                Box::new(v),
                *sec,
            ))
        }
        // TcElementsOfArray a -> TcElementsOfArray <$> zonkTcPat a   (:411)
        TcTerm::TcElementsOfArray(a) => {
            Ok(TcTerm::TcElementsOfArray(Box::new(zonk_tc_pat(st, a)?)))
        }
        // TcQueryGen q -> TcQueryGen <$> zonkTcQuery q   (:412)
        TcTerm::TcQueryGen(q) => Ok(TcTerm::TcQueryGen(Box::new(zonk_tc_query(st, q)?))),
        // TcAll q -> TcAll <$> zonkTcQuery q   (:413)
        TcTerm::TcAll(q) => Ok(TcTerm::TcAll(Box::new(zonk_tc_query(st, q)?))),
        // TcNegation stmts -> TcNegation <$> mapM zonkTcStatement stmts
        //   (:414)
        TcTerm::TcNegation(stmts) => {
            let mut out = Vec::with_capacity(stmts.len());
            for s in stmts.iter() {
                out.push(zonk_tc_statement(st, s)?);
            }
            Ok(TcTerm::TcNegation(out))
        }
        // TcPrimCall op args -> TcPrimCall op <$> mapM zonkTcPat args
        //   (:415)
        TcTerm::TcPrimCall(op, args) => {
            let mut out = Vec::with_capacity(args.len());
            for a in args.iter() {
                out.push(zonk_tc_pat(st, a)?);
            }
            Ok(TcTerm::TcPrimCall(op.clone(), out))
        }
        // TcIf (Typed ty cond) th el -> TcIf
        //   <$> (Typed <$> zonkType ty <*> zonkTcPat cond)
        //   <*> zonkTcPat th <*> zonkTcPat el   (:416-420)
        TcTerm::TcIf { cond, then_, else_ } => {
            let Typed(ty, cond) = &**cond;
            let ty = zonk_type(st, ty)?;
            let cond = zonk_tc_pat(st, cond)?;
            let then_ = zonk_tc_pat(st, then_)?;
            let else_ = zonk_tc_pat(st, else_)?;
            Ok(TcTerm::TcIf {
                cond: Box::new(Typed(ty, cond)),
                then_: Box::new(then_),
                else_: Box::new(else_),
            })
        }
        // TcDeref ty p -> TcDeref <$> zonkType ty <*> zonkTcPat p
        //   (:421-422)
        TcTerm::TcDeref(ty, p) => {
            let ty = zonk_type(st, ty)?;
            let p = zonk_tc_pat(st, p)?;
            Ok(TcTerm::TcDeref(ty, Box::new(p)))
        }
        // TcFieldSelect (Typed ty p) f ->
        //   TcFieldSelect <$> (Typed <$> zonkType ty <*> zonkTcPat p)
        //     <*> pure f   (:423-426)
        TcTerm::TcFieldSelect(typed, f) => {
            let Typed(ty, p) = &**typed;
            let ty = zonk_type(st, ty)?;
            let p = zonk_tc_pat(st, p)?;
            Ok(TcTerm::TcFieldSelect(Box::new(Typed(ty, p)), f.clone()))
        }
        // TcAltSelect (Typed ty p) f -> ...   (:427-430)
        TcTerm::TcAltSelect(typed, f) => {
            let Typed(ty, p) = &**typed;
            let ty = zonk_type(st, ty)?;
            let p = zonk_tc_pat(st, p)?;
            Ok(TcTerm::TcAltSelect(Box::new(Typed(ty, p)), f.clone()))
        }
        // TcElementsOfSet p -> TcElementsOfSet <$> zonkTcPat p   (:431)
        TcTerm::TcElementsOfSet(p) => Ok(TcTerm::TcElementsOfSet(Box::new(zonk_tc_pat(st, p)?))),
        // TcElementsUnresolved{} -> error "zonkTcTerm: TcElementsUnresolved"
        //   -- handled in zonkTcPat   (:432)
        TcTerm::TcElementsUnresolved(..) => panic!("zonkTcTerm: TcElementsUnresolved"),
        // TcPromote{} -> error "zonkTcTerm: TcPromote" -- handled in
        // zonkTcPat   (:433)
        TcTerm::TcPromote(..) => panic!("zonkTcTerm: TcPromote"),
        // TcDemote{} -> error "zonkTcTerm: TcPromote" -- handled in
        // zonkTcPat   (:434)
        // (the source's message names TcPromote for the TcDemote arm —
        // reproduced verbatim)
        TcTerm::TcDemote(..) => panic!("zonkTcTerm: TcPromote"),
        // TcStructPat{} -> error "zonkTcTerm: TcStructPat" -- handled in
        // zonkTcPat   (:435)
        TcTerm::TcStructPat(..) => panic!("zonkTcTerm: TcStructPat"),
    }
}

// -----------------------------------------------------------------------------
// zonkTcStatement (Unify.hs:437-442)
// -----------------------------------------------------------------------------

/// `zonkTcStatement (TcStatement ty l r) =
///   TcStatement <$> zonkType ty <*> zonkTcPat l <*> zonkTcPat r`
/// (Unify.hs:437-442) — type first, then lhs, then rhs.
fn zonk_tc_statement(st: &mut TypecheckState, s: &TcStatement) -> T<TcStatement> {
    let TcStatement(ty, l, r) = s;
    let ty = zonk_type(st, ty)?;
    let l = zonk_tc_pat(st, l)?;
    let r = zonk_tc_pat(st, r)?;
    Ok(TcStatement(ty, l, r))
}

// -----------------------------------------------------------------------------
// show (Doc) — the private flat-branch stand-in (see ASSUMED-IMPORT)
// -----------------------------------------------------------------------------

// TODO(port): `show (displayDefault p)` render parity at zonkTcPat's
// non-singleton sum pattern (Unify.hs:353). `Show (Doc ann)` renders
// through prettyprinter 1.7.1's `renderShowS (layoutPretty
// defaultLayoutOptions doc)` (Internal.hs:2199-2200, :2209-2217) and that
// surface is UNOWNED in the manifest (display.rs:537-546 renders nothing).
// This private copy duplicates the stand-ins in monad.rs:686,
// resolve.rs:1958 and evolve.rs:1186 (no shared owner exists yet) and
// renders the FLAT branch of every group. The divergence is layout, not
// escaping: renderShowS emits text raw (`showString`, :2214), so `go`'s
// raw push agrees with it; but layoutPretty applies the 80-column/ribbon
// layout while this copy is always flat, and its SLine arm emits '\n'
// followed by `i` spaces (:2215) whereas this copy's Line arm drops the
// indentation. When display.rs's renderer lands, delete all four copies
// and route the sites through it.
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
// EXPORT PARITY (the source's list, Unify.hs:9-14, ticked item by item —
// the unit-18 addendum's checklist)
// -----------------------------------------------------------------------------
//   unify          -> pub fn unify
//   apply          -> pub fn apply
//   zonkTcQuery    -> pub fn zonk_tc_query
//   zonkVars       -> pub fn zonk_vars
// All four exports are present, and nothing else is public: unifyError,
// extend, zonkType, resolveHas, apply_, zonkTcPat, zonkMatch, zonkTcTerm
// and zonkTcStatement are private, plus two helpers the source keeps in
// where-bindings (apply_'s `go` as apply_go, zonkMatch's `var` as
// zonk_var) and the private show_doc stand-in. There is no `mod` here and
// no re-export: the parent (query/typecheck.rs) wires `pub mod unify;`.
//
// Marker breakdown (closed set — one open port todo and one performance
// deferral, zero of every other class):
//   1. Doc rendering for zonkTcPat's `show (displayDefault p)` panic text
//      (at show_doc): the flat-branch stand-in for the unowned render
//      surface, fourth private copy after monad.rs:686, resolve.rs:1958
//      and evolve.rs:1186.
//   2. apply_go's pass-through arms deep-clone where the source shares
//      (at apply_go): `Type` has no sharing, so `return t` copies — most
//      expensively a NamedTy's ExpandedType payload and an unresolved
//      HasTy's field map. The fast version needs sharing in `Type`
//      itself; deferred, not fixed here.
// No source-defect reproductions: the two odd panic texts ("zonkMatch:
// tyvar" for both sides of the promote/demote check; "zonkTcTerm:
// TcPromote" for the TcDemote arm) are reproduced verbatim as message
// TEXT, per RULE §2's partial-functions row — they are the source's
// messages, not flagged defects.
// No inventory divergence: this module has no inventory row (INVENTORY in
// the header). No unsafe (RULE §3 vacuously satisfied).
//
// PORT STATUS: confidence=high todos=1
