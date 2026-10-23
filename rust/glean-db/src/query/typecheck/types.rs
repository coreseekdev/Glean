// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// Ported from: glean/db/Glean/Query/Typecheck/Types.hs (139 lines)
// Manifest unit: rust/glean-db/src/query/typecheck/types.rs
// (migration/manifest.tsv line 88, crate glean-db). RULE §4: one source
// file -> one target file; the manifest path is authoritative.

// WHAT THIS FILE IS: the typecheck area's TYPE VOCABULARY — the
// representational-query AST the typechecker produces and the flattener
// consumes: `TcQuery` (5-field positional record: return Type, key pat,
// optional value pat, statements, Ordered), `TcStatement` (Type + lhs/rhs
// pats), the `TcPat` synonym (Term over the Match algebra instantiated at
// `Typed TcTerm`), `TcPred` (the diagnostic (PidRef, span) pair), the
// 15-variant `TcTerm`, the `TypecheckedQuery` synonym, and `lookupField`
// (the record/sum field lookup). Pure data + two Display instances + one
// lookup; no IO, no FFI, no unsafe. The area's first shipped file (rows
// 90 Typecheck/Monad.hs, 91 Unify.hs and 92 Typecheck.hs — manifest lines
// 90/91/92 — grow the area around it; the directory anchor
// src/query/typecheck.rs is row 92's target, created early by THIS unit
// following the row-86/codegen.rs precedent).

// INVENTORY (RULE §5): NO row in any of migration/inventory*.tsv governs
// glean/db/Glean/Query/Typecheck/Types.hs — grep "Typecheck/Types" over
// inventory{,.ffi,.protocol,.sites,.stm,.codeflow}.tsv: zero hits (the
// module's path never occurs). The Typecheck-hitting rows that DO exist
// (inventory.tsv:77/:78 RTS/Foreign/Typecheck.hs FFI, :81 RTS/Typecheck.hs
// mdo labels; the EXC_TAXONOMY rows :13/:54 are Haxl.hs/Glass Main.hs —
// the unit-41 session's cites, not this file's) govern OTHER modules.
// Nothing here is an inventory-owned class: no FFI, no STM, no laziness
// with observable effect (pure AST + a list-comprehension lookup), no
// exceptions, no unsafePerformIO. No divergence from any row (there is
// no row).

// DEPENDENCY TRUTH (migration/depmap/edges.tsv — checked, not assumed;
// 6 edges out, 9 in):
//   OUT :660 Typecheck/Types.hs -> angle/Glean/Angle/Types.hs
//         (Types.hs:24 `import Glean.Angle.Types hiding (Type)`) -> crate
//         glean-angle, `glean_angle::angle::types` (PrimOp, SeekSection,
//         Ordered, FieldName, SrcSpan; the unit-87 precedent imports the
//         same path, codegen/types.rs:128).
//   OUT :661 Typecheck/Types.hs -> angle/Glean/Display.hs
//         (Types.hs:25 `import Glean.Display` — the source ALSO imports
//         Compat.Prettyprinter :19, whose Doc stand-in this crate owns
//         per RULE §1 prettyprinting) -> `glean_angle::display` — one
//         owner, no per-unit forks.
//   OUT :662 Typecheck/Types.hs -> db/Glean/Query/Codegen/Types.hs
//         (Types.hs:22 imports Match(..), Var(..), QueryWithInfo(..),
//         Typed(..)) -> SAME CRATE, `crate::query::codegen::types`
//         (shipped row 87; the within-crate `use crate::` shape is the
//         backend/retry.rs:294 precedent).
//   OUT :663 Typecheck/Types.hs -> hs/Glean/RTS/Term.hs (Types.hs:27
//         `import Glean.RTS.Term as RTS`) -> crate glean-core,
//         `glean_core::rts::term::Term`.
//   OUT :664 Typecheck/Types.hs -> hs/Glean/RTS/Types.hs (Types.hs:26
//         `import Glean.RTS.Types as RTS` — Type, FieldDef, and the
//         RTS-qualified Tuple in the isUnit where-binding) -> crate
//         glean-core, `glean_core::rts::types`.
//   OUT :665 Typecheck/Types.hs -> util/Glean/Util/Some.hs (Types.hs:28
//         `import Glean.Util.Some` — the `Some IsSrcSpan` of TcPred) ->
//         the unit-38 decision record, rust/glean-util/src/util/some.rs:
//         a module that ships ZERO items (row 136's per-use-site rule),
//         so the import dissolves into the TcPred image decided below.
//   IN  :377 Database/Schema.hs, :396 Database/Schema/Types.hs (row 89),
//   IN  :576 Expand.hs, :587 Flatten.hs, :618 Prune.hs (row 93),
//   IN  :646 Typecheck.hs (row 92), :656 Typecheck/Monad.hs (row 90),
//   IN  :672 Typecheck/Unify.hs (row 91), :697 UserQuery.hs
//   — every one of those units imports this vocabulary; none is shipped
//   yet (rows 88/89/90/91/92/93 are queued; unit 87 recorded §2.4's
//   order from 87 as 88, 89, 98, 104, … — this file is 88).
//
// CARGO EDGES: no new crate edge — glean-angle and glean-core are both
// already anticipated by this crate's dependency notes (unit 87's
// CARGO EDGES block, codegen/types.rs:70-81; this file's cross-crate
// `use glean_angle::…` / `use glean_core::…` lines are the cites).

// DERIVING PARITY (every Haskell deriving clause has a disposition; none
// dropped — derive sites: Types.hs:34, :48, :80):
//   * Show (TcQuery :34, TcStatement :48, TcTerm :80) -> #[derive(Debug)]
//     (RULE §2 deriving-Show row; the port-wide Debug-vs-Show convention,
//     angle/types.rs:77-89 / codegen/types.rs:83-98). Grep of the
//     consumers (the 9 IN edges) finds NO direct `show` of TcQuery/
//     TcStatement/TcTerm: every diagnostic renders the DISPLAY instances
//     instead (UserQuery.hs:674/:982/:986/:989/:992
//     `show (displayDefault qiQuery)`; Flatten.hs:71
//     `show (displayVerbose qiQuery)`; the TcQuery error path Typecheck.hs
//     :92 (withExceptT, twin :131) `Text.pack . show` shows the rendered ERROR Doc, not the
//     query). Tag parity is RULE §2's constructor-shaped default; Debug
//     is the stand-in.
//   * No Eq/Ord/Hash on any of the three (the source derives none) — the
//     port adds none.
//   * Clone is added on every data type carrying a source deriving clause
//     (Haskell values are freely copyable; the term.rs:127 /
//     codegen/types.rs:119-121 precedent) — TcQuery, TcStatement, TcTerm.
//     The aliases: TcPat/TcPred inherit Clone through their components
//     (Term/Match/Typed/Var all Clone — term.rs:129, codegen/types.rs
//     :322/:377/:573; PidRef Clone — rts/types.rs:185-190; SrcSpan
//     Clone — angle/types.rs:401); TypecheckedQuery does NOT implement
//     Clone — its QueryWithInfo carrier derives nothing (codegen/
//     types.rs:145-148), exactly like the source.
//   * TcTerm's record-constructor fields (`cond`/`then_`/`else_`,
//     Types.hs:66) keep their source names as the struct-variant fields
//     (`else_` is a legal Rust identifier; no mangling).

use crate::query::codegen::types::{is_wild, Match, QueryWithInfo, Typed, Var};
use glean_angle::angle::types::{FieldName, Ordered, PrimOp, SeekSection, SrcSpan};
use glean_angle::display::{
    cat, hang, hsep, nest, parens, punctuate, sep, Display, DisplayOpts, Doc,
};
use glean_core::rts::term::Term;
use glean_core::rts::types::{FieldDef, PidRef, Type};

// -----------------------------------------------------------------------------
// TcQuery (Types.hs:30-45)
// -----------------------------------------------------------------------------

/// The typechecking phase turns 'ParsedPat' (source-level terms) into
/// 'TcPat' (representational terms), but it doesn't flatten nested
/// generators, so it leaves the query as a 'SourceQuery'.
/// (source haddock, Types.hs:30-32 — transcribed.)
///
/// `data TcQuery = TcQuery Type TcPat (Maybe TcPat) [TcStatement] Ordered
/// deriving Show` (Types.hs:33-34) — a 5-field POSITIONAL record (no
/// field names in the source), so the Rust image is a tuple struct in the
/// TransformAndBind shape (codegen/types.rs:559-565). Fields, in order:
/// return type, key pattern, optional value pattern, statements, and the
/// `Ordered` flag whose syntax has no display yet (the source's own TODO,
/// transcribed in the Display below).
#[derive(Clone, Debug)]
pub struct TcQuery(pub Type, pub TcPat, pub Option<TcPat>, pub Vec<TcStatement>, pub Ordered);

/// `instance Display TcQuery` (Types.hs:36-45):
///
/// ```haskell
/// display opts (TcQuery _ key maybeVal stmts _ord) = case stmts of
///   [] -> head
///   _ ->
///     hang 2 $ sep $ head <+>
///       "where" : punctuate ";" (map (display opts) stmts)
///       -- TODO: syntax for ordering
///   where
///  head = display opts key <>
///    maybe mempty (\val -> " -> " <> display opts val) maybeVal
/// ```
///
/// The `head` local keeps its source name (`head` is not a Rust keyword);
/// `<+>` is `append_space`, `<>` is `append`, `mempty` is `Doc::empty`
/// (never reached — the None arm returns `head` unchanged, which IS
/// `mempty <> head`). The empty-statement arm renders `head` alone. The
/// `-- TODO: syntax for ordering` line is the SOURCE's own comment
/// (Types.hs:42), transcribed with its Haskell comment marker so it is
/// not mistaken for a port marker; the `_ord` field feeds no display arm,
/// exactly as in the source.
impl Display for TcQuery {
    fn display(&self, opts: &DisplayOpts) -> Doc {
        let key = self.1.display(opts);
        let head = match &self.2 {
            // maybe mempty (\val -> " -> " <> display opts val) maybeVal
            Some(val) => key.append(Doc::text(" -> ")).append(val.display(opts)),
            None => key,
        };
        match self.3.as_slice() {
            [] => head,
            stmts => {
                // (head <+> "where") : punctuate ";" (map (display opts) stmts)
                //  — `<+>` binds tighter than `:`: ONE leading element.
                let mut docs = vec![head.append_space(Doc::text("where"))];
                docs.extend(punctuate(
                    Doc::text(";"),
                    stmts.iter().map(|s| s.display(opts)).collect(),
                ));
                hang(2, sep(docs))
                // -- TODO: syntax for ordering   (source, Types.hs:42)
            }
        }
    }
}

// -----------------------------------------------------------------------------
// TcStatement (Types.hs:47-51)
// -----------------------------------------------------------------------------

/// `data TcStatement = TcStatement Type TcPat TcPat deriving Show`
/// (Types.hs:47-48) — positional, tuple struct like TcQuery. The leading
/// Type is the statement's predicate type (unread by the Display).
#[derive(Clone, Debug)]
pub struct TcStatement(pub Type, pub TcPat, pub TcPat);

/// `instance Display TcStatement where
///    display opts (TcStatement _ lhs rhs) = displayStatement opts lhs rhs`
/// (Types.hs:50-51) — the SHARED displayStatement (Angle/Types.hs:
/// 1000-1008: `| isWild lhs = display opts rhs | otherwise = hang 2 $
/// sep [display opts lhs <+> "=", display opts rhs]`).
///
/// That function is generic over `IsWild pat`, and the lhs here is a
/// `TcPat = Term (Match …)` — the very instantiation whose `IsWild (Term
/// (Match ext v))` instance (Codegen/Types.hs:207-210) the unit-87
/// orphan ruling proved UNPORTABLE (E0117; probe-pinned, see the block
/// above [`is_wild`], codegen/types.rs:591-628 — the free fn is the
/// instance's image). The shipped
/// `glean_angle::angle::types::display_statement` requires
/// `Pat: IsWild + Display` (angle/types.rs:2265-2270), which the foreign
/// `Term` cannot satisfy, so the call cannot be made from this crate.
/// The body is therefore replicated HERE over [`is_wild`] — the
/// show_byte_string precedent (codegen/types.rs:839-851): a shared
/// function whose class dispatch collapsed to a free function is
/// mirrored locally, recorded rather than silent. (The alternative —
/// relaxing display_statement's bound in glean-angle to take a
/// wildcard-test closure — is glean-angle's owner's call, not this
/// unit's; flagged for review.)
fn display_tc_statement(lhs: &TcPat, rhs: &TcPat, opts: &DisplayOpts) -> Doc {
    if is_wild(lhs) {
        rhs.display(opts)
    } else {
        hang(
            2,
            sep(vec![
                lhs.display(opts).append_space(Doc::text("=")),
                rhs.display(opts),
            ]),
        )
    }
}

impl Display for TcStatement {
    fn display(&self, opts: &DisplayOpts) -> Doc {
        display_tc_statement(&self.1, &self.2, opts)
    }
}

// -----------------------------------------------------------------------------
// TcPat and TcPred (Types.hs:53-54)
// -----------------------------------------------------------------------------

/// `type TcPat = Term (Match (Typed TcTerm) Var)` (Types.hs:53) — a Term
/// whose Ref positions carry the Match algebra instantiated at
/// `Typed TcTerm` (row 87's enum documents this instantiation:
/// codegen/types.rs:421-426) with `Var` variables. The Display chain
/// recurses through the shipped impls: Term's Display calls the Ref's
/// Match Display (term.rs:548 shape), MatchExt hands to Typed's Display
/// (codegen/types.rs:581-589), Typed hands to [`TcTerm`]'s below.
pub type TcPat = Term<Match<Typed<TcTerm>, Var>>;

/// `type TcPred = (PidRef, Some IsSrcSpan)` (Types.hs:54).
///
/// THE EXISTENTIAL COLLAPSE — decided by a PRODUCER-FLOW survey, not
/// intuition. The unit-38 decision record (rust/glean-util/src/util/
/// some.rs, the zero-item decision record for RULE §2's GADT/existential
/// row, citing inventory row 136 "closed enum at each use site")
/// prescribes "enum-at-use-site in the query/typecheck units" for the
/// instance set {SrcSpan @ Angle/Types.hs:875, SpanAngleDSL @
/// typed/Glean/Query/Angle.hs:98} and mandates "span tracking must NOT be
/// dropped"; its TODO(port) acknowledges the survey can be refined. The
/// record's set is the tree-wide INSTANCE survey (which instances exist);
/// THIS site's question is which spans can be WRAPPED here — and the
/// typechecker fixes one concrete span per run, at its top-level caller:
///
/// * The constructor: `tcFactGenerator` (Typecheck.hs:688-717) appends
///   `(pidRef, Some span) : tcPreds s` (:715) where `predSpan :: Maybe s`
///   and everything above it is `IsSrcSpan s`-polymorphic.
/// * Caller A — `typecheck` (Typecheck.hs:78-116) ← UserQuery.hs:980
///   (compileAngleQuery): the query is `resolved` from
///   `resolveQuery parsed`, `parsed = Angle.parseQuery source`
///   (UserQuery.hs:969); the qualified `Angle` is the PARSER
///   (UserQuery.hs:56 `import qualified Glean.Angle.Parser as Angle`),
///   whose `parseQuery :: ByteString -> Either String SourceQuery`
///   (Angle/Parser.y:422) returns `SourceQuery' SrcSpan SrcSpan`
///   (Angle/Types.hs:645) -> s = **SrcSpan**.
/// * Caller B — `typecheckDeriving` (Typecheck.hs:119-126) ←
///   Database/Schema.hs:881: the query is
///   `predicateDefDeriving (predicateSchema details)` and
///   `predicateSchema :: PredicateDef' SrcSpan ()`
///   (Database/Schema/Types.hs:168) -> s = **SrcSpan**.
///   (The SAME existential is stored by `tcPromote :: [(Type, Type, Some
///   IsSrcSpan)]` — Typecheck/Monad.hs:106, wrapped by addPromote
///   Typecheck.hs:1243-1245 — and unpacked polymorphically by
///   resolvePromote Typecheck.hs:1248-1269, which runs under both
///   callers: Typecheck.hs:96 and :169. The sibling units owning
///   Monad.hs/Typecheck.hs inherit the decision below verbatim.)
/// * The negative result: the OTHER IsSrcSpan instance, SpanAngleDSL
///   (`data SpanAngleDSL = DSL`, typed/Glean/Query/Angle.hs:95, instance
///   :98-103), belongs to the programmatic query-builder DSL — and that
///   DSL never reaches this site: its only production consumer renders
///   the built query to TEXT before any typecheck, `query = angleData .
///   display . sig` (backend-api/Glean/Query/Thrift/Internal.hs:81-82),
///   discarding the spans; the server re-parses the text with the parser
///   (SrcSpan). Grepping every importer of Glean.Query.Typecheck, the
///   ONLY entry-point calls are UserQuery.hs:980 and Schema.hs:881
///   (Prune.hs:29/Derive.hs:50/Schema/Gen/Main.hs:59 take `tcQueryDeps`
///   only; the test modules — test/tests/Angle/*.hs — are not translated,
///   the record's own classification).
/// * The consumer: getPredDiags (UserQuery.hs:706-731) reads
///   `(PidRef _ (PredicateId predRef _), Some span)` and renders
///   `show (pretty span)` into a diagnostic string (:725-731);
///   tcPredExist (:713) reads only the PidRef. No consumer ever needs
///   the span POLYMORPHICALLY across the two flows — each flow's list is
///   homogeneous in its one span type.
///
/// The closed set of what can flow into THIS site is therefore the
/// SINGLETON {SrcSpan}. Row 136's rule maps a site's closed variant set;
/// an enum over one reachable variant is the concrete type plus dead
/// weight — the unit-87 refusal shape (no speculative unconsumed API
/// with a fabricated justification, codegen/types.rs:450-462). The image
/// is the concrete type, below. Span tracking is NOT dropped (the
/// record's mandate): the full `SrcSpan` travels in the pair, Display
/// intact (angle/types.rs:1967-1974, `spanStart - spanEnd`).
///
/// REOPEN CONDITION (the contract with rows 90/91/92): if any unit finds
/// a production path instantiating the typechecker at another IsSrcSpan,
/// this image reopens to the closed enum. The archaeology for that day
/// is on file: SpanAngleDSL is a payload-less singleton — startLoc/
/// endLoc/mkSpan all discard the value and return DSL (Angle.hs:99-103),
/// so a FIELDLESS enum variant would be its exact image, and its struct
/// is module-PRIVATE in the port (rust/glean-typed/src/query/angle.rs:
/// 407-447, `struct SpanAngleDSL;`) — fieldless means the enum never
/// needs to name the private type; the "angle DSL" render (Angle.hs:105)
/// would be owned by that enum's Display arm, cross-referenced to
/// glean-typed's angle.rs:439-447.
pub type TcPred = (PidRef, SrcSpan);

// -----------------------------------------------------------------------------
// TcTerm (Types.hs:56-80)
// -----------------------------------------------------------------------------

/// `data TcTerm … deriving Show` (Types.hs:56-80) — the 16-variant
/// representational term. Variant order is the source's declaration
/// order. The source's own per-variant comments are transcribed verbatim
/// (RULE §0: rationale travels with the type).
///
/// THE BOXES (and why they land HERE): TcPat =
/// `Term<Match<Typed<TcTerm>, Var>>` closes a size cycle with NO
/// indirection in the shipped links — Term's `Ref(R)` is inline
/// (term.rs:130-140), Match's `MatchExt(Ext)` is inline
/// (codegen/types.rs:426) and Typed is an inline pair — so
/// TcTerm -> TcPat -> Match -> MatchExt(Typed<TcTerm>) -> TcTerm is
/// E0072-infinite unless TcTerm breaks it (probe-pinned on a 3-crate
/// scratch, /tmp/u88_probe2: the unboxed enum fails E0072 exactly as the
/// analysis predicts; the boxed shape below compiles and resolves the
/// full Display chain through the shipped generic impls). The direct
/// pattern/query payload fields are therefore `Box`-ed — the unit-87
/// disposition verbatim (codegen/types.rs:364-369): "the Haskell fields
/// were lazy pointers anyway — the same disposition as Term::Alt's Box".
/// Placement notes, both probe-pinned:
///   * the `Typed TcPat` positions become `Box<Typed<TcPat>>` (the
///     whole field was the lazy pointer in the source). NOT
///     `Typed<Box<TcPat>>`: Typed's delegating Display impl requires
///     `X: Display` and the owned Display trait has NO `Box<T>`
///     blanket impl (adding one would be an orphan impl), so
///     `Typed<Box<TcPat>>` breaks the `displayAtom opts pat` call on
///     the whole Typed that the source's TcAltSelect arm makes —
///     `Box<Typed<TcPat>>` keeps it alive through method auto-deref.
///   * `Vec` positions stay plain (already heap-indirected);
///     TcQuery/TcStatement keep their inline TcPat fields (finite once
///     TcTerm is). Rows 90/91/92 construct with `Box::new` at the
///     pointer positions.
#[derive(Clone, Debug)]
pub enum TcTerm {
    TcOr(Box<TcPat>, Box<TcPat>),
    TcFactGen(PidRef, Box<TcPat>, Box<TcPat>, SeekSection),
    TcElementsOfArray(Box<TcPat>),
    TcElementsOfSet(Box<TcPat>),
    TcElementsUnresolved(Type, Box<TcPat>),
    TcQueryGen(Box<TcQuery>),
    TcAll(Box<TcQuery>),
    TcNegation(Vec<TcStatement>),
    TcPrimCall(PrimOp, Vec<TcPat>),
    /// `TcIf { cond :: Typed TcPat, then_ :: TcPat, else_ :: TcPat }`
    /// (Types.hs:66) — the one record constructor; field names kept; the
    /// pat positions Box-ed per the block above.
    TcIf {
        cond: Box<Typed<TcPat>>,
        then_: Box<TcPat>,
        else_: Box<TcPat>,
    },
    /// `TcDeref Type TcPat` — pat.* : if pat has predicate type,
    /// evaluates to the key(s). (source comment, Types.hs:68)
    TcDeref(Type, Box<TcPat>),
    TcFieldSelect(Box<Typed<TcPat>>, FieldName),
    TcAltSelect(Box<Typed<TcPat>>, FieldName),
    /// `TcPromote Type TcPat` — Typed B (TcPromote A P) ("promote pat P
    /// from type A to type B")
    ///   - P : A, and either
    ///     - A == B, or
    ///     - B = P, where P : A for some predicate P
    /// Turns into either nothing or TcFactGen after typechecking
    /// (source comment, Types.hs:72-76).
    TcPromote(Type, Box<TcPat>),
    TcDemote(Type, Box<TcPat>),
    /// `TcStructPat [(FieldName, TcPat)]` — An unresolved pattern
    /// matching a record or sum type. (source comment, Types.hs:79)
    TcStructPat(Vec<(FieldName, TcPat)>),
}

/// The TcStructPat Display's `fields` where-binding (Types.hs:122-123):
///   fields = sep . punctuate "," . map field
///   field (name, pat) = pretty name <+> "=" <+> display opts pat
fn struct_pat_fields(fs: &[(FieldName, TcPat)], opts: &DisplayOpts) -> Doc {
    sep(punctuate(
        Doc::text(","),
        fs.iter()
            .map(|(name, pat)| {
                Doc::text(name.as_str())
                    .append_space(Doc::text("="))
                    .append_space(pat.display(opts))
            })
            .collect(),
    ))
}

/// `instance Display TcTerm` (Types.hs:82-123) — the load-bearing literal
/// surface: ".*" (deref), ". field" / ".? field" (selects), "[..]"
/// (elements-of), "elements" (set), "all" (query quantifier), "!" (negation),
/// ";" (statement punctuate), "|" (or), "^" (promote), "↓" U+2193
/// (demote — kept BYTE-faithful, UTF-8 e2 86 93 as in the source,
/// Types.hs:118), "->" (fact generator + key/value arrow), "where"
/// (via the TcQuery instance), and the nest-2/hang-2 structure,
/// clause for clause. `pretty` on FieldName/String is `Doc::text`
/// (display.rs's documented mapping — raw, unquoted).
impl Display for TcTerm {
    fn display(&self, opts: &DisplayOpts) -> Doc {
        match self {
            // display opts (TcOr a b) = display opts a <+> "|" <+> display opts b
            TcTerm::TcOr(a, b) => a
                .display(opts)
                .append_space(Doc::text("|"))
                .append_space(b.display(opts)),
            // display opts (TcDeref _ pat) = displayAtom opts pat <> ".*"
            TcTerm::TcDeref(_, pat) => pat.display_atom(opts).append(Doc::text(".*")),
            // display opts (TcFieldSelect (Typed _ pat) field) =
            //   displayAtom opts pat <> "." <> pretty field
            // (the Box<Typed<..>> payload needs the `&**` let — Box
            // patterns are unstable; the destructure shape is the
            // source's.)
            TcTerm::TcFieldSelect(typed, field) => {
                let Typed(_, pat) = &**typed;
                pat.display_atom(opts)
                    .append(Doc::text("."))
                    .append(Doc::text(field.as_str()))
            }
            // display opts (TcAltSelect pat field) =
            //   displayAtom opts pat <> ".?" <> pretty field
            // (NOTE: pat is the WHOLE Typed — not destructured; the
            // displayAtom lands on Typed's delegating override,
            // codegen/types.rs:586-588, reached through the Box's
            // method auto-deref.)
            TcTerm::TcAltSelect(pat, field) => pat
                .display_atom(opts)
                .append(Doc::text(".?"))
                .append(Doc::text(field.as_str())),
            // display opts (TcIf (Typed _ cond) then_ else_) = sep
            //   [ nest 2 $ sep ["if", displayAtom opts cond ]
            //   , nest 2 $ sep ["then", displayAtom opts then_]
            //   , nest 2 $ sep ["else", displayAtom opts else_]
            //   ]
            TcTerm::TcIf { cond, then_, else_ } => {
                let Typed(_, cond) = &**cond;
                sep(vec![
                    nest(2, sep(vec![Doc::text("if"), cond.display_atom(opts)])),
                    nest(2, sep(vec![Doc::text("then"), then_.display_atom(opts)])),
                    nest(2, sep(vec![Doc::text("else"), else_.display_atom(opts)])),
                ])
            }
            // display opts (TcFactGen pid kpat vpat _range)
            //   | isWild vpat || isUnit vpat =
            //     nest 2 $ sep [display opts pid, displayAtom opts kpat]
            //   | otherwise =
            //     nest 2 $ sep
            //       [ display opts pid
            //       , displayAtom opts kpat
            //       , "->" <+> displayAtom opts vpat ]
            //   where
            //   isUnit (RTS.Tuple []) = True
            //   isUnit _ = False
            // (Types.hs:94-104. isWild is the free fn [`is_wild`] — row
            // 87 pre-declared exactly this consumption, codegen/types.rs:
            // 1027-1033; isUnit is the Generator-display-local where-
            // binding re-expressed as a match guard, the codegen/types.rs:
            // 768 shape (&**vpat: the payload is Box<TcPat>, and a match
            // guard cannot move out of the Box). "->" <+> … is ONE list
            // element.)
            TcTerm::TcFactGen(pid, kpat, vpat, _range) => {
                let is_unit = matches!(&**vpat, Term::Tuple(xs) if xs.is_empty());
                if is_wild(vpat) || is_unit {
                    nest(2, sep(vec![pid.display(opts), kpat.display_atom(opts)]))
                } else {
                    nest(
                        2,
                        sep(vec![
                            pid.display(opts),
                            kpat.display_atom(opts),
                            Doc::text("->").append_space(vpat.display_atom(opts)),
                        ]),
                    )
                }
            }
            // display opts (TcElementsOfArray arr) = displayAtom opts arr <> "[..]"
            TcTerm::TcElementsOfArray(arr) => {
                arr.display_atom(opts).append(Doc::text("[..]"))
            }
            // display opts (TcElementsOfSet set) =
            //   "elements" <+> parens (display opts set)
            TcTerm::TcElementsOfSet(set) => {
                Doc::text("elements").append_space(parens(set.display(opts)))
            }
            // display opts (TcElementsUnresolved _ pat) =
            //   displayAtom opts pat <> "[..]"
            TcTerm::TcElementsUnresolved(_, pat) => {
                pat.display_atom(opts).append(Doc::text("[..]"))
            }
            // display opts (TcQueryGen q) = parens (display opts q)
            TcTerm::TcQueryGen(q) => parens(q.display(opts)),
            // display opts (TcAll query)
            //   = "all" <+> "(" <> display opts query <> ")"
            // ("all" is append_space-separated; the parens are `<>`-glued
            // to the query — one Doc, no separator inside.)
            TcTerm::TcAll(query) => Doc::text("all").append_space(
                Doc::text("(").append(query.display(opts)).append(Doc::text(")")),
            ),
            // display opts (TcNegation q) =
            //   "!" <> parens (sep (punctuate ";" (map (display opts) q)))
            TcTerm::TcNegation(q) => Doc::text("!").append(parens(sep(punctuate(
                Doc::text(";"),
                q.iter().map(|s| s.display(opts)).collect(),
            )))),
            // display opts (TcPrimCall op args) =
            //   hsep (display opts op : map (displayAtom opts) args)
            TcTerm::TcPrimCall(op, args) => hsep(
                std::iter::once(op.display(opts))
                    .chain(args.iter().map(|a| a.display_atom(opts))),
            ),
            // display opts (TcPromote _ pat) = "^" <> displayAtom opts pat
            TcTerm::TcPromote(_, pat) => Doc::text("^").append(pat.display_atom(opts)),
            // display opts (TcDemote _ pat) = "↓" <> displayAtom opts pat
            // (U+2193 DOWNWARDS ARROW, UTF-8 e2 86 93 — byte-faithful.)
            TcTerm::TcDemote(_, pat) => Doc::text("\u{2193}").append(pat.display_atom(opts)),
            // display opts (TcStructPat fs) =
            //   cat [ nest 2 $ cat [ "{", fields fs], "}"]
            //   where
            //   fields = sep . punctuate "," . map field
            //   field (name, pat) = pretty name <+> "=" <+> display opts pat
            // (cat exists in the owned algebra — display.rs:397; the
            // inner cat glues "{" to the fields with NO grouping.)
            TcTerm::TcStructPat(fs) => cat(vec![
                nest(2, cat(vec![Doc::text("{"), struct_pat_fields(fs, opts)])),
                Doc::text("}"),
            ]),
        }
    }

    /// `displayAtom opts pat = case pat of
    ///    TcOr{} -> parens (display opts pat)
    ///    TcFactGen{} -> parens (display opts pat)
    ///    TcPrimCall{} -> parens (display opts pat)
    ///    TcQueryGen{} -> parens (display opts pat)
    ///    TcNegation{} -> display opts pat
    ///    TcIf{} -> parens (display opts pat)
    ///    _ -> display opts pat`
    /// (Types.hs:125-132 — the parens discipline, exact: the five
    /// "atomic-needing" variants get parens(display); TcNegation is
    /// listed WITHOUT parens and the catchall covers the remaining nine;
    /// the two same-body arms are kept separate to mirror the source's
    /// case shape.)
    fn display_atom(&self, opts: &DisplayOpts) -> Doc {
        match self {
            TcTerm::TcOr(..)
            | TcTerm::TcFactGen(..)
            | TcTerm::TcPrimCall(..)
            | TcTerm::TcQueryGen(..)
            | TcTerm::TcIf { .. } => parens(self.display(opts)),
            TcTerm::TcNegation(_) => self.display(opts),
            _ => self.display(opts),
        }
    }
}

// -----------------------------------------------------------------------------
// TypecheckedQuery (Types.hs:134)
// -----------------------------------------------------------------------------

/// `type TypecheckedQuery = QueryWithInfo TcQuery` (Types.hs:134) — the
/// generic record from row 87 (codegen/types.rs:148-155); the Haskell
/// builds it at Typecheck.hs:116 with `QueryWithInfo q tcNextVar Nothing
/// ty`.
pub type TypecheckedQuery = QueryWithInfo<TcQuery>;

// -----------------------------------------------------------------------------
// lookupField (Types.hs:136-139)
// -----------------------------------------------------------------------------

/// `lookupField :: FieldName -> [RTS.FieldDef] -> [(Type, Word64)]
/// lookupField fieldName fields =
///   [ (ty, n) | (FieldDef name ty, n) <- zip fields [0..]
///   , name == fieldName ]`
/// (Types.hs:136-139) — index fields from 0, keep name matches, project
/// (type, index).
///
/// The Rust image is `fields.iter().enumerate().filter_map(...)` over
/// the same shape: zip [0..] is `.enumerate()`, the guard is the
/// `if`, the projection is the tuple. `Word64` -> `u64` (the RULE §2
/// numeric row; an index, not an ID — no newtype).
///
/// The (Type, u64) pairs are BORROWED (`&Type`): every consumer takes
/// list HEADS and only reads — Typecheck.hs:459/:470
/// `(fieldTy,_):_ -> … Typed fieldTy sel …` (wraps into a Typed, which
/// the owning site can clone from the shared ref),
/// Typecheck.hs:544/:550 `(ty, n):_ -> RTS.Alt n <$> typecheckPattern
/// ctx ty pat` / `RTS.Alt n (mkWild ty)`, Unify.hs:350
/// `(_, n):_ -> RTS.Alt n pat'`, Flatten.hs:380 `(_,n):_ -> return n`.
/// The shared-reference shape is the faithful inspector — no clone the
/// source does not do (the [`is_var`] precedent, codegen/types.rs:
/// 303-312); `n as u64` widens usize losslessly.
pub fn lookup_field<'a>(field_name: &str, fields: &'a [FieldDef]) -> Vec<(&'a Type, u64)> {
    fields
        .iter()
        .enumerate()
        .filter_map(|(n, f)| {
            if f.field_def_name == field_name {
                Some((&f.field_def_type, n as u64))
            } else {
                None
            }
        })
        .collect()
}

// SIBLING-UNIT CONTRACTS (RULE §4; shipped surfaces read, never
// re-declared; unshipped units recorded with their cites):
//   * SHIPPED rust/glean-db/src/query/codegen/types.rs (row 87) owns
//     Match<Ext, V> (:378), Var (:323), Typed<X> (:574), QueryWithInfo<Q>
//     (:148), the free fn is_wild (:628) and the Display chain this
//     file's pattern displays recurse through (Match's
//     `impl<Ext: Display> Display for Match<Ext, Var>` :910; Typed's
//     delegating :581-589 — the TcAltSelect/TcIf arms lean on the
//     display_atom override). Row 87 pre-declared this file's
//     consumption (codegen/types.rs:1027-1033): is_wild generic over
//     Ext/V serves the TcFactGen arm; isUnit is our own Tuple-[] guard.
//   * SHIPPED rust/glean-core/src/rts/term.rs owns Term<R> (:130) — the
//     Tuple variant the isUnit guard matches (term.rs:135), and the generic
//     `impl<R: Display> Display for Term<R>` that dispatches the Ref
//     displays (:548 shape).
//   * SHIPPED rust/glean-core/src/rts/types.rs owns PidRef (:185, Display
//     :218 — the TcFactGen `display opts pid` prefix), Type (:246) and
//     FieldDef (:250 = FieldDef_<(), PidRef, ExpandedType>, whose
//     field_def_name : Name = String and field_def_type : Type are
//     lookup_field's projections).
//   * SHIPPED rust/glean-angle/src/angle/types.rs owns PrimOp (:667,
//     Display :2283 — the TcPrimCall head), SeekSection (:607 — stored,
//     unread by every display arm exactly as the source's `_range`),
//     Ordered (:438 — stored, unread: the source's own "TODO: syntax for
//     ordering"), FieldName (:372 = String), SrcSpan (:401) and
//     display_statement (:2265 — UNCALLABLE for Term-typed patterns, see
//     the block above [`display_tc_statement`]).
//   * SHIPPED rust/glean-angle/src/display.rs owns the whole Doc algebra
//     used here: append (`<>`), append_space (`<+>`), nest (:325), hang
//     (:342), hsep (:367), sep (:391), cat (:397 — the TcStructPat arm's
//     combinator EXISTS, no display.rs gap), punctuate (:420), parens
//     (:466), and the Display trait (:612) with the display_atom default
//     (overridden here). EVERY combinator the source instance uses
//     exists — grep-verified per combinator.
//   * SHIPPED rust/glean-typed/src/query/angle.rs (unit 44's area) owns
//     the PRIVATE SpanAngleDSL (:407-447) — named by the TcPred
//     archaeology above, never imported here (it is not nameable from
//     this crate; the reopen condition documents the fieldless-variant
//     image if the day comes).
//   * UNSHIPPED row 90 (Typecheck/Monad.hs -> query/typecheck/monad.rs):
//     `tcPreds :: [TcPred]` (Monad.hs:107) and
//     `tcPromote :: [(Type, Type, Some IsSrcSpan)]` (:106) — BOTH carry
//     this file's TcPred image: the second's span component is the same
//     concrete SrcSpan (one typechecker run, one span type — the
//     archaeology above; addPromote/resolvePromote run under the same
//     two SrcSpan-instantiated callers). The row-38 record's
//     "enum-at-use-site in the query/typecheck units" is consumed by the
//     singleton-flow refinement above, not silently dropped.
//   * UNSHIPPED rows 91/92 (Unify.hs, Typecheck.hs) and the Flatten/
//     Prune/Expand/UserQuery/Schema consumers (edges :576-:697): the
//     TcTerm/TcPat vocabulary above; lookup_field's borrowed pairs are
//     the heads they pattern-bind; row 92 owns the src/query/typecheck.rs
//     anchor this unit created (its header records the handoff).
//   * THE BOXES, for the constructors among those rows: TcTerm's direct
//     pat/query payload fields are Box<TcPat>/Box<TcQuery> (and the
//     Typed-wrapped ones Typed<Box<TcPat>>) — see the enum's box block
//     for the E0072 cycle and the unit-87 lazy-pointer disposition.
//     Construction inserts Box::new at those positions; every other
//     field keeps the source shape.
//
// Marker breakdown (closed set — zero of every class): no open port
// todos — every open-looking question was closed against shipped
// surfaces or the source (the TcPred existential collapse is
// evidence-complete for the current tree, with the reopen condition on
// file; the display_statement replication follows from the unit-87
// orphan ruling and is flagged for review in prose, not as a port todo);
// the "TODO: syntax for ordering" string inside the TcQuery Display is
// the SOURCE's own comment (Types.hs:42), transcribed with its Haskell
// comment marker — not a port marker. No performance deferrals — nothing
// here is perf-critical beyond the source's shape. No source-defect
// reproductions — the source has no defects in this vocabulary. No
// unsafe code (RULE §3 vacuously satisfied).
// PORT STATUS: confidence=high todos=0
