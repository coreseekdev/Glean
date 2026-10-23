// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// Ported from: glean/db/Glean/Query/Codegen/Types.hs (395 lines)
// Manifest unit: rust/glean-db/src/query/codegen/types.rs
// (migration/manifest.tsv line 87, crate glean-db). RULE §4: one source
// file -> one target file; the manifest path is authoritative.
//
// WHAT THIS FILE IS: the codegen area's TYPE VOCABULARY — the flattened-
// query AST the compiler consumes after Glean/Query/Flatten.hs. Pure data:
// `QueryWithInfo` (4 fields incl. qiNumVars + qiReturnType), `CgQuery`
// (head Expr + body [CgStatement]), `CgStatement_`/`Generator_` (the
// statement/generator sums), `Match ext var` (the 9-variant pattern
// algebra), the `Pat_/Expr_ = Term (Match () var)` synonyms, `Var`,
// `Typed`, `TransformAndBind`, `Output`; plus the Match Bifunctor/
// Bifoldable walks, `matchVar`/`isVar`, the wildcard test, and ~76 lines
// of pretty-printing (the Display instances). No IO, no FFI, no unsafe;
// row 110 (Codegen.hs, manifest line 110, NOT YET SHIPPED) consumes this
// vocabulary through query/codegen.rs; the sibling record beside it
// (query_regs.rs, row 86) is the syscall table the codegen fills.

// INVENTORY (RULE §5): NO row in any of migration/inventory*.tsv governs
// glean/db/Glean/Query/Codegen/Types.hs — grep "Codegen/Types" over
// inventory{,.ffi,.protocol,.sites,.stm,.codeflow}.tsv: zero hits (the
// module's path never occurs, not even in evidence columns; the
// bytecode-adjacent rows inventory.tsv:37/:39 govern Codegen.hs's OUTPUT,
// not this type vocabulary). Nothing here is an inventory-owned class:
// no FFI, no STM, no laziness-DEPENDENT construct (all fields are strict
// data; Haskell laziness is unobservable in a pure AST), no exceptions,
// no unsafePerformIO. No divergence from any row (there is no row).

// DEPENDENCY TRUTH (migration/depmap/edges.tsv — checked, not assumed;
// edges 550-554 out, 20 in):
//   OUT :550 Codegen/Types.hs -> angle/Glean/Angle/Types.hs
//         (Types.hs:39 imports IsWild(..), PrimOp(..), SeekSection(..))
//         -> crate glean-angle, module path `glean_angle::angle::types`
//         (the shipped precedent: glean-core/src/rts/types.rs:93 imports
//         Type_ from exactly that path).
//   OUT :551 Codegen/Types.hs -> angle/Glean/Display.hs
//         (Types.hs:37 Compat.Prettyprinter + :41 Glean.Display) -> crate
//         glean-angle, `glean_angle::display` — the in-repo Doc algebra
//         (RULE §1 prettyprinting row: one owner, no per-unit forks).
//   OUT :552 Codegen/Types.hs -> bytecode/Glean/Bytecode/Types.hs
//         (Types.hs:40 imports Register, Ty(..)) -> crate glean-bytecode,
//         `glean_bytecode::bytecode::types` (supply.rs:113 precedent).
//         NOTE: the source's `Ty(..)` is DEAD — no Ty constructor occurs
//         anywhere in Types.hs (grep "Ty[,( ]" hits only the generatorEltTy
//         field names); the Rust mirror imports Register/BinaryOutputPtr
//         only and does not carry the stale import forward.
//   OUT :553 Codegen/Types.hs -> hs/Glean/RTS/Term.hs (Types.hs:42) ->
//         crate glean-core, `glean_core::rts::term::Term`.
//   OUT :554 Codegen/Types.hs -> hs/Glean/RTS/Types.hs (Types.hs:43
//         imports Type, Fid, PidRef) -> crate glean-core,
//         `glean_core::rts::types`.
//   IN  :373 Database/Schema.hs, :395 Database/Schema/Types.hs (row 89),
//   IN  :525 BindOrder.hs, :535 Codegen.hs (row 110), :565 Derive.hs,
//   IN  :574 Expand.hs, :584 Flatten.hs, :596 Flatten/Types.hs,
//   IN  :599 Incremental.hs, :609 Opt.hs, :616 Prune.hs (row 93),
//   IN  :624 Reorder.hs, :634 Transform.hs (row 95), :644 Typecheck.hs,
//   IN  :655 Typecheck/Monad.hs, :662 Typecheck/Types.hs (row 88),
//   IN  :670 Typecheck/Unify.hs, :689 UserQuery.hs, :713 Vars.hs,
//   IN  :1559 schema/gen/Glean/Schema/Gen/Main.hs
//   — every one of those units imports this vocabulary; none is shipped
//   yet (rows 88/89/93/95/110 are queued behind this one; §2.4's order
//   from 87 is 88, 89, 98, 104, 105, 117, 125, 148, 177).
//
// CARGO EDGES (recorded here rather than as entries, per the
// glean-db/Cargo.toml convention — Step 4 owns the workspace scaffold):
// this unit is the first glean-db consumer of glean-angle's TYPE surface.
// backend/types.rs's Cargo.toml bullet scoped the glean-angle edge as
// "fingerprint-only" — that sentence is now STALE: depmap edges 550/551
// are module-level imports of Angle/Types.hs and Display.hs, so
// glean-db's anticipated-dependency list needs glean-angle (and this
// file's `use glean_angle::…` lines are the cite). The glean-core edge is
// already anticipated (Cargo.toml's row-52 bullet; storage/db.rs:213-226
// already imports glean_core::…); the glean-bytecode bullet landed with
// row 86 (see query/codegen/query_regs.rs's header NOTE — its "scaffold
// bullet is owed" remark covers this file's import of Register too).

// DERIVING PARITY (every Haskell deriving clause has a disposition; none
// dropped — derive sites: Types.hs:69, :90, :181, :199, :253, :259, :310):
//   * Show -> #[derive(Debug)] (RULE §2 deriving-Show row / the port-wide
//     Debug-vs-Show convention, angle/types.rs:77-89): the constructor
//     TAGS carry (`MatchBind(..)`, `CgDisjunction(..)`), payload
//     punctuation is Rust's. Grep of every consumer module (edges
//     :373-:713 + :1559) finds NO direct `show` consumer of CgQuery/
//     CgStatement_/Generator_/Match/Var/Typed/TransformAndBind — the
//     diagnostic surface this vocabulary feeds is the DISPLAY instances
//     below (UserQuery.hs:986/:989/:992 and Reorder.hs:138 render
//     `displayDefault qiQuery`; Vars.hs:195 renders `displayVerbose` of a
//     CgStatement; Flatten.hs:71 renders the pre-flatten TcQuery), plus
//     the composite derives term.rs:100-113 already documented (a derived
//     Show on these types would only leak into other derived Shows, which
//     have no consumer either). Tag parity is therefore RULE §2's
//     constructor-shaped default and Debug is the stand-in.
//   * Eq (Generator_ :181, Match :253) -> derived PartialEq/Eq (contextual
//     bounds on Ext/V, exactly Haskell's derived-Eq context). Var's Eq is
//     HAND-WRITTEN in the source (:201-202, varId only) — manual impl
//     below, PidRef-style (rts/types.rs:194-215 documentation shape).
//   * Functor/Foldable/Traversable on CgStatement_ (:90), Generator_
//     (:181), Match (:253), Typed (:310): no Rust derive exists.
//     Consumed-surface analysis over every consumer module: ZERO uses of
//     the var-only walks. Every structural traversal in the tree
//     hand-recurses instead (Vars.hs:82-110 `VarsOf` instances,
//     Vars.hs:136-186 freshWild/reWild, Codegen.hs:133-141 findOutputs,
//     Incremental.hs:74-103, Reorder.hs:846-870, Typecheck.hs:1009-1035,
//     Opt.hs:219/:608 — mapM over LISTS of statements, not over the
//     statement type). The walks that DO hit Matches go through Term's
//     ref positions via the Bifunctor trio below (Codegen.hs:1228,
//     Typecheck.hs:1029, Prune.hs:230). No map/fold helpers are shipped
//     for CgStatement_/Generator_/Typed; a later unit whose source
//     fmap/folds one of them derives the helper at its own file (the
//     source's derives are mechanical to re-express).
//   * QueryWithInfo (:48-53) derives NOTHING in the source — no Rust
//     derives here either (structure-preserving).
//   * Clone is added on every data type that carries a source deriving
//     clause (Haskell values are freely copyable; the term.rs:127
//     precedent) — QueryWithInfo, which derives nothing, gets none.

// The source's export list re-exports SeekSection(..) and PrimOp(..)
// (Types.hs:22/:25) — the two names travel with this module's exports, so
// the Rust mirror re-exports them under their own names (selective, not a
// glob; RULE §4). `pub use` is also this file's import of them (the enum
// fields below carry both).
pub use glean_angle::angle::types::{PrimOp, SeekSection};
use glean_angle::display::{
    align, enclose_sep, hang, hsep, nest, parens, punctuate, sep, Display, DisplayOpts, Doc,
};
use glean_bytecode::bytecode::types::{BinaryOutputPtr, Register};
use glean_core::rts::term::Term;
use glean_core::rts::types::{Fid, PidRef, Type};

// -----------------------------------------------------------------------------
// A Query with flat generators, ready for compilation (Types.hs:45-53)
// -----------------------------------------------------------------------------

/// `type CodegenQuery = QueryWithInfo CgQuery` (Types.hs:46).
pub type CodegenQuery = QueryWithInfo<CgQuery>;

/// `data QueryWithInfo q = QueryWithInfo {..}` (Types.hs:48-53) — a
/// generic record: q is TcQuery pre-flattening, CgQuery after.
/// The source derives nothing on this record; neither do we (DERIVING
/// PARITY above). Haskell `Int` -> `i64` (the RULE §1 numeric row as
/// applied crate-wide, e.g. database/trace.rs:74).
pub struct QueryWithInfo<Q> {
    pub qi_query: Q,
    /// maximum index of any Var + 1 (source haddock, Types.hs:50)
    pub qi_num_vars: i64,
    /// see Note [query result] in Flatten.hs (source haddock, Types.hs:51)
    pub qi_generator: Option<Generator>,
    pub qi_return_type: Type,
}

// -----------------------------------------------------------------------------
// Flattened Query types (Types.hs:55-122)
// -----------------------------------------------------------------------------

/// `data CgQuery = CgQuery {..} deriving Show` (Types.hs:59-69) — a query
/// produces a finite series of terms.
///
/// `flatQueryHead` cannot contain MatchWild or MatchBind (source haddock).
/// Variables bound by each body statement scope over the statements that
/// follow it (source haddock).
#[derive(Clone, Debug)]
pub struct CgQuery {
    /// For each result produced by the query body, build this term.
    pub flat_query_head: Expr,
    /// A sequence of statements, like a list comprehension.
    pub flat_query_body: Vec<CgStatement>,
}

/// `data CgStatement_ var ... deriving (Show, Functor, Foldable,
/// Traversable)` (Types.hs:79-90) — a statement is either a single
/// generator `pat = gen` or one of the composite forms below.
///
/// The type variable is spelled `V` because `Var` is the concrete
/// instantiation (`type CgStatement = CgStatement_ Var`, Types.hs:93) and
/// a generic parameter of the same name would shadow the struct.
#[derive(Clone, Debug)]
pub enum CgStatement_<V> {
    /// `CgStatement (Pat_ var) (Generator_ var)` — a single generator.
    CgStatement(Pat_<V>, Generator_<V>),
    CgAllStatement(V, Expr_<V>, Vec<CgStatement_<V>>),
    CgNegation(Vec<CgStatement_<V>>),
    /// For rationale, see Note [why do we have sequential composition?]
    /// below the enum (source comment, Types.hs:84).
    CgDisjunction(Vec<Vec<CgStatement_<V>>>),
    CgConditional {
        cond: Vec<CgStatement_<V>>,
        then_: Vec<CgStatement_<V>>,
        else_: Vec<CgStatement_<V>>,
    },
}

/// `type CgStatement = CgStatement_ Var` (Types.hs:93).
pub type CgStatement = CgStatement_<Var>;

// Note [why do we have sequential composition?]
// (Types.hs:95-122, transcribed verbatim — RULE §0: the source's own
// rationale travels with the type that needs it.)
//
// The issue is that queries for sum types can't necessarily be handled
// by nested generators. Consider
//
//   v = cxx1.FunctionName (name(cxx1.Name "xyz" ) | operator(cxx1.Name "+"))
//
// If we flattened this into nested generators we would get
//
//   x = cxx1.Name "xyz"
//   y = cxx1.Name "+"
//   z = cxx1.FunctionName (name x | operator y)
//
// Now suppose there is no name xyz. This query will match nothing,
// because the generator for cxx1.Name "xyz" would be empty.  (even if
// the generator matched, flattening out the generators like this will
// test too many combinations and do too much work).
//
// With sequential composition of queries we can do it like this:
//
//  n = (name x where x = cxx1.Name "xyz") |
//      (operator x where x = cxx1.Name "+")
//  v = cxx1.FunctionName n
//
// (Note that this query won't work if you write it because we can't
// typecheck the sub-query "name x where ...", but we can generate the
// AST for it in the JSON query compiler.)

/// `type Generator = Generator_ Var` (Types.hs:124).
pub type Generator = Generator_<Var>;

/// `data Generator_ var ... deriving (Eq, Show, Functor, Foldable,
/// Traversable)` (Types.hs:127-181) — a generator produces a finite
/// series of terms.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Generator_<V> {
    /// Enumerate facts of a predicate matching a pattern (Types.hs:128-150).
    /// The left-hand side can be any pattern that matches the fact ID
    /// (e.g. `1234 = pred _` looks up a specific fact — source haddock).
    FactGenerator {
        /// Predicate to search
        generator_pid: PidRef,
        /// match the fact key
        generator_pat: Pat_<V>,
        /// match the fact value
        generator_value: Pat_<V>,
        /// restrict the range of fact ids this generator can produce.
        generator_seek_section: SeekSection,
    },
    /// Generate values (Types.hs:153).
    TermGenerator(Expr_<V>),
    /// Produces facts of a derived predicate. The values of this
    /// generator are fact IDs that refer to facts in the temporary
    /// Environment produced by a query. (Types.hs:155-162.)
    DerivedFactGenerator {
        generator_pid: PidRef,
        generator_key: Expr_<V>,
        generator_value: Expr_<V>,
    },
    /// Produces all elements of an array (Types.hs:164-168).
    ArrayElementGenerator {
        /// type of the elements (source comment)
        generator_elt_ty: Type,
        generator_array: Expr_<V>,
    },
    /// Produce all elements of a set (Types.hs:170-174).
    SetElementGenerator {
        generator_elt_ty: Type,
        generator_set: Expr_<V>,
    },
    /// Types.hs:176-180.
    PrimCall {
        prim_op: PrimOp,
        prim_op_args: Vec<Expr_<V>>,
        prim_op_ret_type: Type,
    },
}

/// `type Pat = Pat_ Var` (Types.hs:183).
pub type Pat = Pat_<Var>;
/// `type Expr = Expr_ Var` (Types.hs:184).
pub type Expr = Expr_<Var>;

/// `type Pat_ var = Term (Match () var)` (Types.hs:186) — the `()` stays:
/// `MatchExt`'s placeholder parameter is instantiated with the unit type,
/// whose `MatchExt` payload is uninhabited-in-practice (RULE §0
/// structure-preserving; see [`Match::MatchExt`]).
// `Pat_` is NOT in the source's export list (:10-29); pub is forced by
// Rust's private-in-public rule through the exported `Pat` synonym below.
pub type Pat_<V> = Term<Match<(), V>>;
/// `type Expr_ var = Term (Match () var)` (Types.hs:187) — the same type
/// as [`Pat_`]; both synonyms are kept because the source keeps both.
pub type Expr_<V> = Term<Match<(), V>>;

/// `type Output = Typed (Register 'BinaryOutputPtr)` (Types.hs:189) — the
/// promoted `'BinaryOutputPtr` kind maps to the shipped zero-sized marker
/// `glean_bytecode::bytecode::types::BinaryOutputPtr` (types.rs:215, the
/// sealed TyTag layer).
pub type Output = Typed<Register<BinaryOutputPtr>>;

/// `isVar :: Pat -> Maybe Var` (Types.hs:191-193) — inspect a pattern's
/// single Ref; consumers pattern-bind the result (Opt.hs:360/:474/:481/
/// :492, Reorder.hs:360/:699), so the shared-reference shape is the
/// faithful inspector (no clone the source does not do).
pub fn is_var(pat: &Pat) -> Option<&Var> {
    match pat {
        Term::Ref(v) => match_var(v),
        _ => None,
    }
}

/// `data Var = Var {..} deriving Show` (Types.hs:195-199).
///
/// Eq is NOT derived: `Var _ x _ == Var _ y _ = x == y` (Types.hs:201-202)
/// — equality is by varId ONLY; varType and varOrigName are ignored. Not
/// a clippy-allow situation (we do not derive and silence); the manual
/// impl IS the source's semantics, documented PidRef-style
/// (rts/types.rs:194-215). No Ord (the source derives none); no Hash (no
/// Hashable instance in the source).
#[derive(Clone, Debug)]
pub struct Var {
    pub var_type: Type,
    /// `{-# UNPACK #-} !Int` (Types.hs:197) -> i64 (LP64; RULE §1 numeric
    /// row; the unboxing pragma is a GHC heap-layout detail, dropped as in
    /// term.rs:71-74).
    pub var_id: i64,
    pub var_orig_name: Option<String>,
}

impl PartialEq for Var {
    fn eq(&self, other: &Self) -> bool {
        // Var _ x _ == Var _ y _  = x == y   (Types.hs:202)
        self.var_id == other.var_id
    }
}

impl Eq for Var {}

/// `instance Display Var where
///    display _ (Var _ v nm) = pretty (fromMaybe "" nm) <> "_" <> pretty v`
/// (Types.hs:204-205) — `pretty` on Text and on Int; the opts are ignored.
impl Display for Var {
    fn display(&self, _opts: &DisplayOpts) -> Doc {
        Doc::text(self.var_orig_name.as_deref().unwrap_or(""))
            .append(Doc::text("_"))
            .append(Doc::text(self.var_id.to_string()))
    }
}

// -----------------------------------------------------------------------------
// Match (Types.hs:211-253)
// -----------------------------------------------------------------------------

/// `data Match ext var ... deriving (Eq, Functor, Foldable, Traversable,
/// Show)` (Types.hs:211-253) — the pattern algebra a [`Term`]'s Ref
/// positions carry.
///
/// Generic parameters: the source's `ext var` — `V` instead of `Var` for
/// the second (the struct [`Var`] is the common instantiation; a generic
/// parameter of the same name would shadow it).
///
/// `Box` on the three direct `Term` fields is the mechanical Rust
/// requirement for recursion (Match contains `Term<Match<...>>`, Term's
/// `Ref` is inline): without it the enum's size would be infinite. The
/// Haskell fields were lazy pointers anyway — the same disposition as
/// [`Term::Alt`]'s Box (term.rs:76-79). The Vec fields are already
/// heap-indirected.
///
/// Deriving parity: Eq -> derived PartialEq/Eq (ext and V gain the
/// contextual bounds, exactly Haskell's derived context; Type/Fid carry
/// their own Eq). Show -> derive(Debug) (DERIVING PARITY above).
/// Functor/Foldable/Traversable have no Rust derive; the consumed walk
/// surface is the Bifunctor/Bifoldable pair below. Clone added
/// (term.rs:127 precedent).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Match<Ext, V> {
    /// Always matches (Types.hs:212-213).
    MatchWild(Type),

    /// Never matches (Types.hs:215-216).
    MatchNever(Type),

    /// Match a literal Fact ID (Types.hs:218-219).
    MatchFid(Fid),

    /// Match a value and bind it to this variable (Types.hs:221-222).
    MatchBind(V),

    /// Match a value bound earlier. Note that this means we can have
    /// patterns like pred(X,X), where the second X matches the value
    /// bound by the first X. (Types.hs:224-227.)
    MatchVar(V),

    /// Match two patterns simultaneously. Fails if either of them fails
    /// to match, and binds the MatchBinds from both sides if both
    /// patterns match. Not valid in expressions, only patterns.
    /// (Types.hs:229-232.)
    MatchAnd(Box<Term<Match<Ext, V>>>, Box<Term<Match<Ext, V>>>),

    /// Match a prefix of a string. The bytes are the prefix of the string
    /// (utf-8 encoded — source comment, Types.hs:234-237).
    MatchPrefix(Vec<u8>, Box<Term<Match<Ext, V>>>),

    /// Match a prefix of an array. Note that this may occur in a position
    /// that requires us to build the term instead of matching, so we have
    /// an additional field that matches the whole array. Typically this
    /// will be either a wildcard, if we never need to build a term
    /// involving this array, or a variable which will be bound to the
    /// array when it is matched. (Types.hs:239-248.)
    MatchArrayPrefix(
        /// The type of elements (source comment)
        Type,
        /// The prefix (source comment)
        Vec<Term<Match<Ext, V>>>,
        /// matches the whole array (source comment)
        Box<Term<Match<Ext, V>>>,
    ),

    /// placeholder for extending this type (Types.hs:250-251). The
    /// `Match () var` instantiations keep Ext = `()` (see [`Pat_`]); the
    /// Typecheck instantiation is `Typed TcTerm` (Typecheck/Types.hs:53,
    /// row 88's unit) and the codegen's own is `TransformAndBind` +
    /// `Output` (Codegen.hs:1228, row 110's unit).
    MatchExt(Ext),
}

// -----------------------------------------------------------------------------
// The Bifunctor trio — consumed-surface analysis (RULE §2 B6 row governs
// the walk shape: FULL tree, no shallow copies)
// -----------------------------------------------------------------------------
//
// Rust has no std Bifunctor and no Applicative, so the three Haskell
// instances (Types.hs:261-302) are decided per-consumer, in the house
// shape of the DONE siblings (angle/types.rs:1458 Type_::bimap /
// :1267 SourcePat_::bimap — the B6-row canonical implementations — and
// term.rs:154/:238 map/fold_map):
//
//   * Bifunctor `bimap` — CONSUMED: Codegen.hs:1228
//     `inlineVars vars term = fmap (bimap (error "inlineVars") toOutput)`
//     with `term :: Term (Match () Var)` -> `Term (Match TransformAndBind
//     Output)` — the Term-level fmap applies bimap at every Ref. Shipped
//     as [`Match::bimap`] (ext AND var both evolve).
//   * Bifoldable `bifoldMap` — CONSUMED: Typecheck.hs:1029
//     `overPat pat = foldMap (bifoldMap overTyped (const mempty)) pat`
//     over `TcPat = Term (Match (Typed TcTerm) Var)` (Typecheck/Types.hs:
//     :53) — the Term-level foldMap folds over the Refs. Shipped as
//     [`Match::bifold_map`].
//   * Bitraversable `bitraverse` — its ONE consumer is Prune.hs:230
//     `renamePat = traverse (bitraverse renameTyped renameVar)` (edge
//     :616, row 93's unit, NOT YET SHIPPED), and it runs in `R a = State
//     S a` (Prune.hs:202) — a STATEFUL applicative with no error
//     dimension. There is no Applicative image in Rust, and the house
//     Result-specialised shape (term.rs:198 try_map) has no matching
//     consumer here (no Result anywhere in the instantiation), so shipping
//     a speculative `try_bimap` would be unconsumed API with a fabricated
//     justification. The bitraverse instance is therefore NOT imaged; the
//     consuming unit ports its own stateful traversal exactly as the
//     Vars.hs:136 carve-out established for Term (term.rs:192-197), which
//     is recorded in this file's SIBLING-UNIT CONTRACTS as row 93's
//     obligation. (Not omitted silently — this block IS the record.)
//
// Both shipped walks cover the FULL tree — MatchAnd's both sides,
// MatchPrefix's tail, MatchArrayPrefix's prefix list AND whole-array term
// (the source's own walk shape, Types.hs:269-273/:283-286; B6).

impl<Ext, V> Match<Ext, V> {
    /// Bifunctor — `instance Bifunctor Match where bimap f g = ...`
    /// (Types.hs:261-273), full tree (B6): MatchAnd's both sides,
    /// MatchPrefix's tail, MatchArrayPrefix's prefix list and whole-array
    /// term all recurse; MatchWild/MatchNever/MatchFid carry no ext/var
    /// and pass through; MatchExt evolves the ext, MatchVar/MatchBind the
    /// var. Closures are `&mut` per the sibling house style
    /// (angle/types.rs:1458, term.rs:154).
    pub fn bimap<Ext2, V2, F, G>(self, f: &mut F, g: &mut G) -> Match<Ext2, V2>
    where
        F: FnMut(Ext) -> Ext2,
        G: FnMut(V) -> V2,
    {
        match self {
            Match::MatchWild(x) => Match::MatchWild(x),
            Match::MatchNever(x) => Match::MatchNever(x),
            Match::MatchFid(x) => Match::MatchFid(x),
            Match::MatchExt(ext) => Match::MatchExt(f(ext)),
            Match::MatchVar(var) => Match::MatchVar(g(var)),
            Match::MatchBind(var) => Match::MatchBind(g(var)),
            // MatchAnd a b -> MatchAnd (fmap (bimap f g) a)
            //                       (fmap (bimap f g) b)
            Match::MatchAnd(a, b) => Match::MatchAnd(
                Box::new((*a).map(&mut |m| m.bimap(f, g))),
                Box::new((*b).map(&mut |m| m.bimap(f, g))),
            ),
            // MatchPrefix b term -> MatchPrefix b $ fmap (bimap f g) term
            Match::MatchPrefix(bytes, term) => {
                Match::MatchPrefix(bytes, Box::new((*term).map(&mut |m| m.bimap(f, g))))
            }
            // MatchArrayPrefix ty prefix all ->
            //   MatchArrayPrefix ty ((fmap.fmap) (bimap f g) prefix)
            //     (fmap (bimap f g) all)
            Match::MatchArrayPrefix(ty, prefix, all) => Match::MatchArrayPrefix(
                ty,
                prefix
                    .into_iter()
                    .map(|t| t.map(&mut |m| m.bimap(f, g)))
                    .collect(),
                Box::new((*all).map(&mut |m| m.bimap(f, g))),
            ),
        }
    }

    /// Bifoldable — `instance Bifoldable Match where bifoldMap f g = ...`
    /// (Types.hs:275-286), full tree (B6), left-to-right visit order
    /// exactly as the source's `<>` chain: MatchExt's ext, MatchVar/
    /// MatchBind's var, then MatchAnd's both sides, MatchPrefix's tail,
    /// and MatchArrayPrefix's prefix list (in list order) followed by the
    /// whole-array term. Leaves (MatchWild/MatchNever/MatchFid)
    /// contribute nothing.
    pub fn bifold_map<A, F, G>(&self, acc: &mut A, f: &mut F, g: &mut G)
    where
        F: FnMut(&Ext, &mut A),
        G: FnMut(&V, &mut A),
    {
        match self {
            Match::MatchWild(_) | Match::MatchNever(_) | Match::MatchFid(_) => {}
            Match::MatchExt(ext) => f(ext, acc),
            Match::MatchVar(var) | Match::MatchBind(var) => g(var, acc),
            // MatchAnd a b -> foldMap (bifoldMap f g) a <> foldMap (bifoldMap f g) b
            Match::MatchAnd(a, b) => {
                a.fold_map(acc, &mut |m, acc| m.bifold_map(acc, f, g));
                b.fold_map(acc, &mut |m, acc| m.bifold_map(acc, f, g));
            }
            // MatchPrefix _ term -> foldMap (bifoldMap f g) term
            Match::MatchPrefix(_, term) => {
                term.fold_map(acc, &mut |m, acc| m.bifold_map(acc, f, g));
            }
            // MatchArrayPrefix _ty pre all ->
            //   (foldMap.foldMap) (bifoldMap f g) pre <> foldMap (bifoldMap f g) all
            Match::MatchArrayPrefix(_ty, pre, all) => {
                for t in pre {
                    t.fold_map(acc, &mut |m, acc| m.bifold_map(acc, f, g));
                }
                all.fold_map(acc, &mut |m, acc| m.bifold_map(acc, f, g));
            }
        }
    }
}

/// `matchVar :: Match ext var -> Maybe var` (Types.hs:304-307) — the var
/// of a MatchVar or MatchBind. Consumers inspect (Reorder.hs:323,
/// Opt.hs:635), so the shared-reference shape mirrors `is_var`.
pub fn match_var<Ext, V>(m: &Match<Ext, V>) -> Option<&V> {
    match m {
        Match::MatchVar(v) | Match::MatchBind(v) => Some(v),
        _ => None,
    }
}

/// `data TransformAndBind = TransformAndBind Type Output deriving Show`
/// (Types.hs:257-259) — bind a value of one type into an output of a
/// different type by first performing a type transformation (source
/// haddock). This is the `ext` instantiation of [`Match`] in the codegen
/// proper (Codegen.hs:1228 `inlineVars`).
#[derive(Clone, Debug)]
pub struct TransformAndBind(pub Type, pub Output);

/// `data Typed x = Typed Type x deriving (Show, Functor, Traversable,
/// Foldable)` (Types.hs:309-310). The Functor/Foldable/Traversable
/// derivings have no consumers (grep of every consumer module — the
/// `renameTyped`/`overTyped` sites, Prune.hs:232-234 and
/// Typecheck.hs:1031-1035, hand-pattern-match; DERIVING PARITY above), so
/// no walk helpers ship. Clone added (term.rs:127 precedent).
#[derive(Clone, Debug)]
pub struct Typed<X>(pub Type, pub X);

/// `instance Display x => Display (Typed x) where
///    display opts (Typed _ x) = display opts x
///    displayAtom opts (Typed _ x) = displayAtom opts x`
/// (Types.hs:312-314) — both methods delegate, including the atom
/// override (NOT folded into the default: the source overrides it).
impl<X: Display> Display for Typed<X> {
    fn display(&self, opts: &DisplayOpts) -> Doc {
        self.1.display(opts)
    }

    fn display_atom(&self, opts: &DisplayOpts) -> Doc {
        self.1.display_atom(opts)
    }
}

// -----------------------------------------------------------------------------
// IsWild — the instance that could not be (Types.hs:207-210)
// -----------------------------------------------------------------------------
//
// The source declares `instance IsWild (Term (Match ext v))` with
// `isWild (Ref MatchWild{}) = True; isWild _ = False`. The Rust image the
// unit brief proposed — `impl<Ext, V> IsWild for Term<Match<Ext, V>>` —
// is ILLEGAL, probe-pinned (rustc, 3-crate scratch /tmp/u87_probe —
// crate `pa` defines IsWild, crate `pb` defines Term, crate `pc` holds
// the local Match and the impl attempts):
//   * `impl<Ext, V> IsWild for Term<Match<Ext, V>>` -> E0117 "only traits
//     defined in the current crate can be implemented for types defined
//     outside of the crate ... impl doesn't have any local type before
//     any uncovered type parameters" — a LOCAL type inside a FOREIGN
//     type's arguments does not make the self type local (the same rule
//     that forbids `impl Display for Vec<LocalType>`); Match's locality
//     covers Ext/V, but nothing makes `Term<...>` local.
//   * fully monomorphic `impl IsWild for Term<Match<(), Var>>` -> the
//     same E0117 (concreteness does not help).
//   * an inherent `impl<Ext, V> Term<Match<Ext, V>> { fn is_wild }` ->
//     E0116 (inherent impls must live in the type's crate, and
//     glean-core cannot name this crate's Match).
// A newtype wrapper over Term would repair coherence but rewrite every
// consumer's type — rejected (RULE §0). The faithful image of the
// consumed surface is this free function, generic over Ext/V exactly as
// the source instance is generic over ext/v:
//   * Codegen/Types.hs:349 (the FactGenerator Display below) calls
//     `isWild vpat` on a Pat;
//   * Typecheck/Types.hs:95 (row 88's unit) repeats the same call shape
//     on a TcPat — the generic Ext/V parameters serve it;
//   * Codegen.hs:615/:1088/:1492/:1516 call `all isWild chunks` on
//     QueryChunks — that is Codegen.hs's OWN instance (Codegen.hs:1219-
//     1222, row 110's unit), not this one;
//   * no consumer in the tree uses IsWild POLYMORPHICALLY (grep of
//     `isWild` over db/hs/angle: every site names a concrete type), so
//     the trait dispatch itself is unconsumed surface and nothing here
//     needs the trait import at all.
pub fn is_wild<Ext, V>(pat: &Term<Match<Ext, V>>) -> bool {
    // isWild (Ref MatchWild{}) = True; isWild _ = False
    matches!(pat, Term::Ref(Match::MatchWild(_)))
}

// -----------------------------------------------------------------------------
// Pretty-printing (Types.hs:316-395)
// -----------------------------------------------------------------------------

/// `instance Display CgQuery` (Types.hs:319-324).
///
/// The source's CgAllStatement arm calls this instance on a CONSTRUCTED
/// temporary (`display opts (CgQuery expr stmts)`, Types.hs:333); to call
/// the same code from that arm without cloning the statement list, the
/// instance delegates to [`display_cg_query_parts`] and the arm calls it
/// with its own borrowed fields — the instance is a pure function of the
/// two fields, so the output is identical by construction.
impl Display for CgQuery {
    fn display(&self, opts: &DisplayOpts) -> Doc {
        display_cg_query_parts(&self.flat_query_head, &self.flat_query_body, opts)
    }
}

/// The shared body of [`Display for CgQuery`] (Types.hs:320-324):
///   display opts (CgQuery expr []) = display opts expr
///   display opts (CgQuery expr stmts) =
///     hang 2 $ sep $
///       display opts expr <+> "where" : punctuate ";" (map (display opts) stmts)
/// (`<+>` binds tighter than `:` — the head and "where" are ONE element.)
fn display_cg_query_parts(head: &Expr, body: &[CgStatement], opts: &DisplayOpts) -> Doc {
    match body {
        [] => head.display(opts),
        stmts => hang(
            2,
            sep(
                std::iter::once(head.display(opts).append_space(Doc::text("where"))).chain(
                    punctuate(
                        Doc::text(";"),
                        stmts.iter().map(|s| s.display(opts)).collect(),
                    ),
                ),
            ),
        ),
    }
}

/// The CgStatement display's `where doStmts` binding (Types.hs:343-345):
///   doStmts stmts = hang 2 $
///     sep [sep ("(" : punctuate ";" (map (display opts) stmts)), ")"]
fn do_stmts(stmts: &[CgStatement], opts: &DisplayOpts) -> Doc {
    let mut inner = vec![Doc::text("(")];
    inner.extend(punctuate(
        Doc::text(";"),
        stmts.iter().map(|s| s.display(opts)).collect(),
    ));
    hang(2, sep(vec![sep(inner), Doc::text(")")]))
}

/// `instance Display CgStatement` (Types.hs:326-345) — the flattened-query
/// debug/compare surface (consumed via `displayVerbose` in Vars.hs:195's
/// reWildStatement error and through the CgQuery instance).
impl Display for CgStatement {
    fn display(&self, opts: &DisplayOpts) -> Doc {
        match self {
            // CgStatement pat gen ->
            //   hang 2 $ sep [display opts pat <+> "=", display opts gen]
            CgStatement_::CgStatement(pat, gen) => hang(
                2,
                sep(vec![
                    pat.display(opts).append_space(Doc::text("=")),
                    gen.display(opts),
                ]),
            ),
            // CgAllStatement var expr stmts -> hang 2 $ sep
            //   [display opts var <+> "=", "all (",
            //    display opts (CgQuery expr stmts), ")"]
            // (Types.hs:330-335; the CgQuery display is invoked on the
            // parts — see [`Display for CgQuery`].)
            CgStatement_::CgAllStatement(var, expr, stmts) => hang(
                2,
                sep(vec![
                    var.display(opts).append_space(Doc::text("=")),
                    Doc::text("all ("),
                    display_cg_query_parts(expr, stmts, opts),
                    Doc::text(")"),
                ]),
            ),
            // CgNegation stmts -> "!" <> doStmts stmts
            CgStatement_::CgNegation(stmts) => Doc::text("!").append(do_stmts(stmts, opts)),
            // CgDisjunction stmtss -> sep (punctuate " |" (map doStmts stmtss))
            CgStatement_::CgDisjunction(stmtss) => sep(punctuate(
                Doc::text(" |"),
                stmtss.iter().map(|ss| do_stmts(ss, opts)).collect(),
            )),
            // CgConditional cond then_ else_ -> sep
            //   [ nest 2 $ sep ["if", doStmts cond ]
            //   , nest 2 $ sep ["then", doStmts then_]
            //   , nest 2 $ sep ["else", doStmts else_]
            //   ]
            CgStatement_::CgConditional { cond, then_, else_ } => sep(vec![
                nest(2, sep(vec![Doc::text("if"), do_stmts(cond, opts)])),
                nest(2, sep(vec![Doc::text("then"), do_stmts(then_, opts)])),
                nest(2, sep(vec![Doc::text("else"), do_stmts(else_, opts)])),
            ]),
        }
    }
}

/// `instance Display Generator` (Types.hs:347-369).
impl Display for Generator {
    fn display(&self, opts: &DisplayOpts) -> Doc {
        match self {
            // display opts (FactGenerator pref kpat vpat section)
            //   | isWild vpat || isUnit vpat =
            //       hang 2 (sep [pred, display opts kpat])
            //   | otherwise =
            //       hang 2 (sep [pred, display opts kpat <+> "->",
            //                    display opts vpat])
            //   where
            //   pred = display opts pref <> prettySection
            //   prettySection = case section of
            //     SeekOnAllFacts -> ""
            //     SeekOnBase -> "<base>"
            //     SeekOnStacked -> "<stacked>"
            //   isUnit (Tuple []) = True
            //   isUnit _ = False
            // (Types.hs:348-359; `pred` is `<>`, NOT `<+>` — no space
            // before the section text.)
            Generator_::FactGenerator {
                generator_pid,
                generator_pat,
                generator_value,
                generator_seek_section,
            } => {
                let pretty_section = match generator_seek_section {
                    SeekSection::SeekOnAllFacts => Doc::text(""),
                    SeekSection::SeekOnBase => Doc::text("<base>"),
                    SeekSection::SeekOnStacked => Doc::text("<stacked>"),
                };
                let pred = generator_pid.display(opts).append(pretty_section);
                let is_unit = matches!(generator_value, Term::Tuple(xs) if xs.is_empty());
                if is_wild(generator_value) || is_unit {
                    hang(2, sep(vec![pred, generator_pat.display(opts)]))
                } else {
                    hang(
                        2,
                        sep(vec![
                            pred,
                            generator_pat.display(opts).append_space(Doc::text("->")),
                            generator_value.display(opts),
                        ]),
                    )
                }
            }
            // display opts (TermGenerator q) = display opts q
            Generator_::TermGenerator(q) => q.display(opts),
            // display opts (DerivedFactGenerator pid k (Tuple [])) =
            //   hang 2 (sep [display opts pid <> "<-", displayAtom opts k ])
            // (Types.hs:361-362 — the value-pattern match on `Tuple []`
            // becomes a match guard on the same shape.)
            Generator_::DerivedFactGenerator {
                generator_pid,
                generator_key,
                generator_value: Term::Tuple(xs),
            } if xs.is_empty() => hang(
                2,
                sep(vec![
                    generator_pid.display(opts).append(Doc::text("<-")),
                    generator_key.display_atom(opts),
                ]),
            ),
            // display opts (DerivedFactGenerator pid k v) =
            //   display opts pid <> "<- (" <>
            //     displayAtom opts k <> " -> " <> displayAtom opts v <> ")"
            // (Types.hs:363-365.)
            Generator_::DerivedFactGenerator {
                generator_pid,
                generator_key,
                generator_value,
            } => generator_pid
                .display(opts)
                .append(Doc::text("<- ("))
                .append(generator_key.display_atom(opts))
                .append(Doc::text(" -> "))
                .append(generator_value.display_atom(opts))
                .append(Doc::text(")")),
            // display opts (ArrayElementGenerator _ arr) =
            //   display opts arr <> "[..]"
            Generator_::ArrayElementGenerator {
                generator_array: arr,
                ..
            } => arr.display(opts).append(Doc::text("[..]")),
            // display opts (SetElementGenerator _ set) =
            //   "elements" <+> display opts set
            Generator_::SetElementGenerator {
                generator_set: set, ..
            } => Doc::text("elements").append_space(set.display(opts)),
            // display opts (PrimCall op args _) =
            //   hsep (display opts op : map (displayAtom opts) args)
            Generator_::PrimCall {
                prim_op,
                prim_op_args,
                ..
            } => hsep(
                std::iter::once(prim_op.display(opts))
                    .chain(prim_op_args.iter().map(|a| a.display_atom(opts))),
            ),
        }
    }
}

// The MatchPrefix Display renders `show str` — Haskell `show` on
// ByteString. The pinned probe (GHC 9.4.7 at
// ~/glean-build/ghc-9.4.7-install/bin/ghc — the migration's pinned
// compiler): show (pack "ab") == "\"ab\"", newline byte -> "ab\n",
// bytes 0xC3 0xA9 -> "\195\169" (DECIMAL per byte, the Latin-1
// unpackChars form), quote/backslash -> "\"" "\\\\" — exactly the form
// [`show_byte_string`] below renders (it is term.rs's byte-exact port of
// bytestring-0.11.5.2's Show + base-4.17.1.0 GHC.Show, see term.rs:259-278
// for the full pin; the shape was re-confirmed on the pinned GHC during
// implementation — the standing pin is term.rs:259-278, quoted below). The copy stands because term.rs's fn is module-private and cannot
// be imported across the crate edge — mirroring the Haskell, where each
// instance independently calls `show` (term.rs:276-278).
fn show_byte_string(bytes: &[u8]) -> String {
    // asciiTab — GHC.Show.hs:428-434 (indices 0..32).
    const ASCII_TAB: [&str; 33] = [
        "NUL", "SOH", "STX", "ETX", "EOT", "ENQ", "ACK", "BEL", "BS", "HT", "LF", "VT", "FF", "CR",
        "SO", "SI", "DLE", "DC1", "DC2", "DC3", "DC4", "NAK", "SYN", "ETB", "CAN", "EM", "SUB",
        "ESC", "FS", "GS", "RS", "US", "SP",
    ];
    // protectEsc inspects the first char of the rendered REST, which is
    // the next input byte's first output char (a raw printable or a
    // backslash), or the closing quote at end-of-string — so the guard
    // reduces to a test on the next byte, as below.
    let mut out = String::with_capacity(bytes.len() + 2);
    out.push('"');
    for (i, &b) in bytes.iter().enumerate() {
        let next_is_digit = bytes.get(i + 1).is_some_and(u8::is_ascii_digit);
        let next_is_h = bytes.get(i + 1) == Some(&b'H');
        match b {
            0x5C => out.push_str("\\\\"), // '\\'
            0x22 => out.push_str("\\\""), // '"'
            0x7F => out.push_str("\\DEL"),
            b if (0x20..0x7F).contains(&b) => out.push(b as char),
            0x07 => out.push_str("\\a"),
            0x08 => out.push_str("\\b"),
            0x0C => out.push_str("\\f"),
            0x0A => out.push_str("\\n"),
            0x0D => out.push_str("\\r"),
            0x09 => out.push_str("\\t"),
            0x0B => out.push_str("\\v"),
            // '\SO': protectEsc (== 'H') (GHC.Show.hs:384). When the
            // guard fires, protectEsc prepends the TWO-CHAR sequence
            // `\&` before the rest (GHC.Show.hs:422-424) — never `;`.
            0x0E => {
                out.push_str("\\SO");
                if next_is_h {
                    out.push_str("\\&");
                }
            }
            // c > '\DEL': \DDD decimal + protectEsc isDec (GHC.Show.hs:373)
            b if b > 0x7F => {
                out.push('\\');
                out.push_str(&(b as u32).to_string());
                if next_is_digit {
                    out.push_str("\\&");
                }
            }
            b => {
                out.push('\\');
                out.push_str(ASCII_TAB[b as usize]);
            }
        }
    }
    out.push('"');
    out
}

/// `instance Display ext => Display (Match ext Var)` (Types.hs:371-395) —
/// monomorphic in the var exactly as the source is; the display paths of
/// patterns (every `display opts pat` on a `Term (Match ext Var)`) run
/// through Term's Display, which calls this at every Ref (term.rs:548).
impl<Ext: Display> Display for Match<Ext, Var> {
    fn display(&self, opts: &DisplayOpts) -> Doc {
        match self {
            // display opts (MatchAnd l r) =
            //   display opts l <+> "@" <+> display opts r
            Match::MatchAnd(l, r) => l
                .display(opts)
                .append_space(Doc::text("@"))
                .append_space(r.display(opts)),
            // display opts (MatchBind v@(Var ty _ _)) =
            //   display opts v  <> ":" <> display opts ty
            Match::MatchBind(v) => v
                .display(opts)
                .append(Doc::text(":"))
                .append(v.var_type.display(opts)),
            // display opts (MatchPrefix str rest) =
            //   display opts (show str) <> ".." <> display opts rest
            // (`show str` is a STRING displayed raw-unquoted by Display
            // String — the quotes come from show itself.)
            Match::MatchPrefix(bytes, rest) => Doc::text(show_byte_string(bytes))
                .append(Doc::text(".."))
                .append(rest.display(opts)),
            // display opts (MatchExt ext) = display opts ext
            Match::MatchExt(ext) => ext.display(opts),
            // display opts (MatchArrayPrefix _ty pre all) =
            //   case all of
            //     Ref MatchWild{} -> arr
            //     _ -> arr <+> "@" <+> display opts all
            //   where
            //   arr = align $ encloseSep "[" "..]" "," $ map (display opts) pre
            // (Types.hs:378-383. display.rs's enclose_sep doc already
            // lists this site — Codegen/Types.hs:383 — among its verified
            // consumers.)
            Match::MatchArrayPrefix(_ty, pre, all) => {
                let arr = align(enclose_sep(
                    Doc::text("["),
                    Doc::text("..]"),
                    Doc::text(","),
                    pre.iter().map(|t| t.display(opts)),
                ));
                match &**all {
                    Term::Ref(Match::MatchWild(_)) => arr,
                    _ => arr
                        .append_space(Doc::text("@"))
                        .append_space(all.display(opts)),
                }
            }
            // display opts other = displayAtom opts other
            // (the remaining variants: MatchWild, MatchNever, MatchFid,
            // MatchVar)
            other => other.display_atom(opts),
        }
    }

    // displayAtom _ (MatchWild _) = "_"
    // displayAtom _ (MatchNever _) = "()"
    // displayAtom _ (MatchFid fid) = pretty fid
    // displayAtom opts (MatchVar v) = display opts v
    // displayAtom opts (MatchPrefix str rest) =
    //   pretty (show str) <> ".." <> display opts rest
    // displayAtom opts other@MatchAnd{} = parens (display opts other)
    // displayAtom opts other@MatchBind{} = parens (display opts other)
    // displayAtom opts other@MatchArrayPrefix{} = parens (display opts other)
    // displayAtom opts (MatchExt ext) = displayAtom opts ext
    // (Types.hs:386-395.)
    fn display_atom(&self, opts: &DisplayOpts) -> Doc {
        match self {
            Match::MatchWild(_) => Doc::text("_"),
            Match::MatchNever(_) => Doc::text("()"),
            Match::MatchFid(fid) => fid.display(opts),
            Match::MatchVar(v) => v.display(opts),
            Match::MatchPrefix(bytes, rest) => Doc::text(show_byte_string(bytes))
                .append(Doc::text(".."))
                .append(rest.display(opts)),
            // parens (display opts other) — display.rs's parens is plain
            // concatenation, NO grouping (the source groups nothing here).
            Match::MatchAnd(..) | Match::MatchBind(_) | Match::MatchArrayPrefix(..) => {
                parens(self.display(opts))
            }
            Match::MatchExt(ext) => ext.display_atom(opts),
        }
    }
}

// SIBLING-UNIT CONTRACTS (RULE §4; shipped surfaces read, never
// re-declared; unshipped units recorded with their cites):
//   * SHIPPED rust/glean-core/src/rts/term.rs owns `Term` (row 12) and
//     its map/fold_map/try_map walks (term.rs:154/:238/:198) — bimap/
//     bifold_map above are DEFINED IN TERMS of Term::map/Term::fold_map,
//     exactly as the Haskell composes `fmap (bimap f g)` over the Term.
//     term.rs's Display for Term<R: Display> is the dispatcher that lands
//     every pattern display in this file's Match instance (term.rs:548
//     `r.display(opts)`); its show_byte_string (:279) is the source of
//     the third-copy rationale for this file's own copy.
//   * SHIPPED rust/glean-core/src/rts/types.rs owns Fid (:155, Display
//     :158 = `pretty fid`), PidRef (:185, Eq-by-pid-only :194-215 — the
//     documentation shape Var's Eq mirrors), and `type Type =
//     Type_<(), PidRef, ExpandedType>` (:246). Display for PidRef
//     (:218) delegates to the PredicateId — the FactGenerator/DerivedFact
//     `pred` prefix in the Generator instance rides it.
//   * SHIPPED rust/glean-angle/src/angle/types.rs owns IsWild (:1917),
//     PrimOp (:667, Display :2283), SeekSection (:607) and Display for
//     Type_ (:1672, needed by MatchBind's `display opts ty`). Its
//     SourcePat_/Type_ bimap/bifold_map (:1267/:1360/:1458/:1493) are the house
//     shape the trio above follows. NOTE: IsWild is deliberately NOT
//     imported — this crate cannot legally implement it for the foreign
//     Term (see the block above [`is_wild`]); the free fn is the image.
//   * SHIPPED rust/glean-angle/src/display.rs owns the whole Doc algebra
//     used here: append (`<>`, :202/:210), append_space (`<+>`, :217),
//     nest (:325), align (:331), hang (:342), hsep (:367), sep (:391),
//     punctuate (:420), enclose_sep (:444 — its doc ALREADY lists
//     Codegen/Types.hs:383 among verified consumers), parens (:466), and
//     the Display trait (:612) with the displayAtom default. EVERY
//     combinator the source instance uses exists — no display.rs gap, no
//     fallback substitutions (grep-verified per combinator).
//   * SHIPPED rust/glean-bytecode/src/bytecode/types.rs owns Register
//     (:374) and the BinaryOutputPtr marker (:215) behind Output.
//   * UNSHIPPED row 88 (Typecheck/Types.hs -> query/typecheck/types.rs):
//     its `TcPat = Term (Match (Typed TcTerm) Var)` (Types.hs:53)
//     instantiates Match's ext with Typed TcTerm; its Display-clause twin
//     at Typecheck/Types.hs:95 calls isWild/isUnit on a TcPat — it
//     consumes [`is_wild`] (generic over Ext/V) and, for isUnit, its own
//     Tuple-[] check (the source's isUnit is a Generator-display-local
//     where-binding, NOT a shared export).
//   * UNSHIPPED row 93 (Prune.hs -> query/prune.rs): the ONE Bitraversable
//     consumer (Prune.hs:230, in `R a = State S a`) must port its own
//     stateful Match walk — no bitraverse image ships here (see the trio
//     block above for the full reasoning).
//   * UNSHIPPED row 110 (Codegen.hs -> query/codegen.rs): consumes bimap
//     via inlineVars (Codegen.hs:1228, the `Match TransformAndBind
//     Output` instantiation), the Output alias, and QueryWithInfo's four
//     fields; it also defines its own IsWild instance for QueryChunk
//     (Codegen.hs:1219-1222) — unrelated to this file.
//   * The 20 IN edges (DEPENDENCY TRUTH) name the remaining consumer
//     units; none may assume a var-only map/fold helper on
//     CgStatement_/Generator_/Typed exists — the source's Functor/
//     Foldable/Traversable derivings on those types have zero consumers
//     (DERIVING PARITY), and any later unit that finds one adds the
//     helper at its own file.

// Marker breakdown (closed set — zero of every class): no open port
// todos — every open-looking question was closed against shipped
// surfaces, the depmap, or the pinned compiler probe (the IsWild orphan
// ruling, the ByteString-show form, the bitraverse non-image); the two
// cross-unit obligations that fall OUT of those closures (row 93's
// stateful walk, row 88's is_wild consumption) are recorded in
// SIBLING-UNIT CONTRACTS with their cites, per the query_regs.rs prose
// convention. No performance deferrals — nothing here is perf-critical
// beyond the source's own shape. No source-defect reproductions — the
// source has no defects in this vocabulary. No unsafe code (RULE §3
// vacuously satisfied), so no safety contracts to carry.
// PORT STATUS: confidence=high todos=0
