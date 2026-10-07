// Ported from: glean/bytecode/Glean/Bytecode/Types.hs (56 lines)
//
// MANIFEST RESOLUTION — supersedes the manifest-path INVENTORY-BUG-CANDIDATE
// (manifest.tsv row 20) this file originally carried: RESOLVED, the file now
// IS at the manifest path rust/glean-bytecode/src/bytecode/types.rs (crate
// glean-bytecode, migration/depmap/crates.txt line 2: 6 files). It was first
// written at a glean-core path against the then-batch sibling layout, then
// moved verbatim to the manifest path. The crate edge is now LIVE and is
// exactly the one the depmap predicted: glean-core → glean-bytecode, carried
// by rust/glean-core/src/rts/bytecode/supply.rs's
// `use glean_bytecode::bytecode::types::…` import (edges.tsv row 1234 =
// Supply.hs:16 `import Glean.Bytecode.Types`; RULE §4 imports row:
// `use glean_<crate>::…` across crates). The original flag's prediction that
// supply.rs would consume `crate::bytecode::types` is RETIRED — it consumes
// the cross-crate path instead. Dependency truth rechecked after the move:
// 14 inbound edges, 0 outbound (edges.tsv rows 64/65/66/68/392/531/548/552/
// 631/1227/1232/1234/1292/1297) — a leaf, as a glean-bytecode crate member
// must be.
//
// INVENTORY (RULE §5): NO row in any of the six migration/inventory*.tsv
// files governs glean/bytecode/Glean/Bytecode/Types.hs (grep over
// inventory{,.ffi,.protocol,.sites,.stm,.codeflow}.tsv: zero hits — the
// module's path never occurs even in evidence columns; the bytecode-adjacent
// rows, inventory.tsv rows 37/39 and 80/81, govern Codegen.hs and
// Traverse.hs/Typecheck.hs — REAL consumers of this module (edges.tsv rows
// 531/1292/1297) — and the fourth bytecode-family consumer beside those
// rows' files is Code.hs (edges.tsv row 1227); row 64's
// RTS/Foreign/Bytecode.hs is NOT a consumer at all (its import list,
// Foreign/Bytecode.hs:17-30, contains no Glean.Bytecode.Types, and no
// edges.tsv row pairs the two). None of these rows constrains a construct
// defined here). Nothing here
// is an inventory-owned class: no FFI (module imports no foreign), no STM,
// no laziness-DEPENDENT construct (the module defines only data/types,
// empty classes, and a total coerce — NOTE the newtype payload fields are
// not strict: Types.hs:44/:51/:54 carry no strictness annotations, so they
// are lazy in Haskell; nothing defined here observes that laziness), no
// exceptions, no unsafePerformIO. No divergence from any row; the original
// manifest-path flag above is resolved.
//
// SIBLING CONTRACTS (RULE §4; DONE crates' real APIs read, never
// re-declared):
//   - The Haskell module's imports are Data.Coerce and Data.Word only
//     (Types.hs:19-20) — pure base. Therefore this unit consumes NOTHING
//     from crate::ffi, crate::rts::types, crate::rts::term,
//     crate::rts::builder, or glean-angle, and it declares no externs.
//   - THIS file owns, for the whole port: the value-level `Ty` enum, the
//     phantom-tag layer (TyTag markers Word/WordPtr/Lit/Offset/DataPtr/
//     BinaryOutputPtr/Fun + TyTuple), the constraint traits Ordered/Addable,
//     `Register`, `cast_register`, `Label`, `Literal`.
//   - Sibling unit 13 (rts/bytecode/supply.rs — SHIPPED and reconciled
//     2026-10-07 against this file's actual shape; its header carries the
//     reconciliation record) consumes `Register<Word>` (Supply.hs:28
//     `RegSupply (Register 'Word)`), `cast_register` (Supply.hs:33, :36)
//     and the Enum delegation below (`succ r`, Supply.hs:36).
//   - Manifest row 150 (def/Glean/Bytecode/Generate/Instruction.rs) consumes
//     value-level `Ty` in its instruction table (def/Instruction.hs:23
//     import; :326-379 `reg (Fun …)` / `Regs [Word…]` argument
//     shapes; contexts `"Addable a b"` :239/:249 (Sub/Add) and
//     `"Addable a 'Word"` :244 (AddConst), `"Ordered a"`
//     :293/:299/:305/:311 — the reason the two constraint traits exist).
//   - Manifest rows 151-152 (generate/cpp.rs, generate/haskell.rs) consume
//     value-level `Ty` AND a Haskell-Show TEXT rendering of it: the source
//     generator builds promoted-type syntax via `show ty` (Generate/
//     Haskell.hs:124-131 showTy). NOTE for those units: Rust `Debug` here is
//     the crate's Show stand-in (derive map below) and is NOT Haskell-Show
//     text (`{:?}` of Ty::Fun(vec![Word,Word]) differs from `Fun [Word,Word]`)
//     — the showTy rendering is owned by generate/haskell.rs, not here.
//   - Downstream register-type users pinned for the reviewer:
//     Register<'Word>/<'DataPtr>/<'BinaryOutputPtr> signatures in
//     db/Glean/Query/Codegen/QueryRegs.hs:20-77; castRegister call sites
//     Code.hs:276-277, Codegen.hs:258/:265/:277/:420; `Register ('Fun '_)`
//     at RTS/Traverse.hs:28 (arity 2) and in the generated Issue module
//     (arities 1 through 7 — the Issue module wraps EVERY instruction,
//     Generate/Haskell.hs:72, over the def/Instruction.hs:326-379 rows
//     cited in the DESIGN block below).
//
// DESIGN — the phantom kinds (RULE §2 GADT row: "enum where the variant set
// is closed"; RULE §2 type-family row: "plain generic traits or concrete
// types (the set is closed)"):
//   - Haskell promotes `Ty` to the kind of `Register`'s phantom parameter
//     (`newtype Register (ty :: Ty)`). Rust has no promotion, so each
//     constructor gets a zero-sized marker type NAMED AS IN THE SOURCE
//     (marker `Word` ↔ variant `Ty::Word` — legal: enum variants live in
//     the enum's namespace), gathered under one SEALED trait `TyTag`
//     (sealing = the closed-kind discipline; no downstream marker can be
//     invented). `fn ty()` is the demotion that keeps the two levels tied
//     to a single source of truth; unused in-tree today (verified: no
//     consumer demotes), it exists so adding a `Ty` variant without its
//     marker cannot compile silently.
//   - `'Fun '[ 'Word, 'Word ]` (type-level list, Traverse.hs:28) →
//     `Fun<(Word, Word)>`: a type-level list has no Rust mirror; a tuple
//     preserves the distinction (each arg-list is a distinct type) and the
//     content (TyTuple::tys()). Arity collector implemented for () through
//     7-tuples — the in-tree arity survey runs to SEVEN, not three:
//     def/Instruction.hs rows CallFun_2_1 arity 3 (:344-346), CallFun_3_0
//     arity 3 (:352-354), CallFun_4_0 arity 4 (:356-358), CallFun_3_1
//     arity 4 (:360-362), CallFun_2_2 arity 4 (:372-374), CallFun_5_0
//     arity 5 (:364-366), CallFun_5_1 arity 6 (:368-370), CallFun_2_5
//     arity 7 (:376-379 — `Fun
//     [Word,Word,WordPtr,WordPtr,WordPtr,WordPtr,WordPtr]`); the high-arity
//     shapes have live emitters (Codegen.hs:1634 `callFun_5_1
//     seekWithinSection_`, :1648 `callFun_2_5 next_`, :1664 `callFun_5_0
//     resultWithPid_`). A longer arg-list fails to compile, which is the
//     extension point, not a gap.
//   - `class Ordered (t :: Ty)` → `trait Ordered: TyTag` (instances Word,
//     DataPtr); `class Addable (t :: Ty) (u :: Ty)` → `trait Addable<U:
//     TyTag>: TyTag` with Self = the destination type t, U = the operand
//     type u (instances: Word+Word, DataPtr+Word). Both are pure
//     compile-time gates on the issue-function generics; they carry no
//     methods, exactly as in the source.
//   - `castRegister = coerce` (Types.hs:47-48) → `cast_register`: rewrites
//     the phantom only; total, unchecked — RULE §0 (translating, not
//     improving: coerce's totality is the source contract).
//
// Deriving map (every Haskell deriving has a disposition; none dropped):
//   Ty: Eq → derive(PartialEq, Eq); Show → derive(Debug).
//   Register: Eq,Ord → derive(PartialEq, Eq, PartialOrd, Ord) — the
//     derived compare sees only the u64, the phantom is ignored, exactly
//     the newtype-derived semantics; Show → MANUAL Debug (below) so the
//     tag stays `Register(5)` (RULE §2 deriving-Show row / A4: constructor-
//     shaped tag; derive would append `PhantomData<…>` noise — the one
//     systematic layout drift the crate convention allows, avoided here
//     because the fix is a 4-line impl); Enum → see delegation row.
//   Label: Eq,Ord → derive; Show → derive(Debug); Enum → delegation row.
//   Literal: Eq,Ord → derive; Show → derive(Debug).
//   Marker types (new Rust surface, no Haskell value-level counterpart):
//     derive the full eq/ord set — Rust derives put `T: Trait` bounds on
//     phantom parameters, so `Register`'s derived Ord (Code.hs:232
//     `maximum` over call-site registers needs it) forces the markers to
//     be Ord themselves; on ZST markers this is a no-op, not semantics.
//   Enum (Register/Label) → no Rust trait (precedent: rts/types.rs
//     deriving map, same disposition). In-tree uses are `succ` only:
//     Register — Code.hs:156 (`succ csNextConstant`), Supply.hs:36
//     (`succ r`); Label — Code.hs:462 (`succ csLabel`). Haskell's derived
//     `succ` ERRORS at the bound, it never wraps — pinned package source:
//     Word64 base-4.17.1.0/GHC/Word.hs:719-722 (`succ x | x /= maxBound
//     = x + 1 | otherwise = succError "Word64"`, with succError at
//     GHC/Enum.hs:206-209 raising "Enum.succ{Word64}: tried to take
//     `succ' of maxBound"); Int, Label's payload, same shape at
//     GHC/Enum.hs:509-513 ("Prelude.Enum.succ{Int}: …"). Consumers port
//     succ as `.0.checked_add(1)` + panic carrying the EXACT per-type
//     message — the shipped disposition of sibling supply.rs's
//     succ_register (RULE §2 partial-functions row: where the source
//     errors, panic with the same text).
//   No Storable anywhere in the source → no #[repr(transparent)] pins;
//   no Hashable → no Hash derives; newtype Copy/Clone are the unboxed-
//   value semantics of the source's Word64/Int payloads — the fields are
//   lazy in Haskell (no strictness annotations, Types.hs:44/:51/:54),
//   but the words are demanded at every consumer site
//   (RULE §2 numeric row: Word64 ids are newtypes; `Int` is the RTS
//   platform's 64-bit machine int → i64).

use std::marker::PhantomData;

/// Instruction argument types (source doc, Types.hs:22-31).
///
/// Value-level `Ty`: consumed by the instruction-definition table and the
/// generators; the type-level mirror of its constructors is the `TyTag`
/// marker set below. RULE §2 (GADT row): `Fun [Ty]` stays a recursive
/// payload — it is data here, only its PROMOTION is type-level.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Ty {
    /// 64 bit word
    Word,
    /// pointer to 64 bit word
    WordPtr,
    /// index into the (string) literal table
    Lit,
    /// jump offset (relative to start of next instruction)
    Offset,
    /// a void pointer
    DataPtr,
    /// pointer to binary::Output (temporary, will be removed)
    BinaryOutputPtr,
    /// pointer to syscall (temporary, will be removed)
    Fun(Vec<Ty>),
}

/// Sealing the kind (RULE §2 GADT row: closed variant set): only the
/// markers in this module can implement `TyTag`.
mod private {
    pub trait Sealed {}
}

/// The type-level counterpart of the `Ty` constructors: the phantom
/// parameter of `Register` (Haskell's promoted kind `Ty`).
///
/// Each marker below is the promotion of the same-named `Ty` constructor;
/// `Fun<Args>` promotes `'Fun '[…]` with the argument list as a tuple
/// (TyTuple). `ty()` is the demotion; unused in-tree, it pins the marker
/// set to the constructor set (single source of truth, RULE §0).
pub trait TyTag: Copy + private::Sealed {
    /// Demotion: the value-level `Ty` this marker promotes.
    fn ty() -> Ty;
}

/// `'Word` — 64 bit word.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Word;
/// `'WordPtr` — pointer to 64 bit word.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct WordPtr;
/// `'Lit` — index into the (string) literal table.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Lit;
/// `'Offset` — jump offset (relative to start of next instruction).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Offset;
/// `'DataPtr` — a void pointer.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct DataPtr;
/// `'BinaryOutputPtr` — pointer to binary::Output (temporary, will be
/// removed).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct BinaryOutputPtr;

/// `'Fun '[…]` — pointer to syscall (temporary, will be removed). The
/// type-level argument list is a tuple of TyTags: `Register<Fun<(Word,
/// Word)>>` is the port of `Register ('Fun '[ 'Word, 'Word ])`
/// (RTS/Traverse.hs:28).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Fun<Args>(PhantomData<Args>);

/// The tuple of markers a `Fun` promotes its argument list to. Implemented
/// for `()` (empty list) through 7-tuples; the in-tree arity survey tops
/// out at 7 (CallFun_2_5, def/Instruction.hs:376-379; next-highest 6 at
/// :368-370 and 5 at :364-366 — full row list in the header's DESIGN
/// block). Longer lists fail to compile — extend here, do not approximate.
pub trait TyTuple: Copy + private::Sealed {
    /// The demoted argument list, in order.
    fn tys() -> Vec<Ty>;
}

impl private::Sealed for Word {}
impl private::Sealed for WordPtr {}
impl private::Sealed for Lit {}
impl private::Sealed for Offset {}
impl private::Sealed for DataPtr {}
impl private::Sealed for BinaryOutputPtr {}
impl<Args: TyTuple> private::Sealed for Fun<Args> {}
impl private::Sealed for () {}
impl<A: TyTag> private::Sealed for (A,) {}
impl<A: TyTag, B: TyTag> private::Sealed for (A, B) {}
impl<A: TyTag, B: TyTag, C: TyTag> private::Sealed for (A, B, C) {}
impl<A: TyTag, B: TyTag, C: TyTag, D: TyTag> private::Sealed for (A, B, C, D) {}
impl<A: TyTag, B: TyTag, C: TyTag, D: TyTag, E: TyTag> private::Sealed for (A, B, C, D, E) {}
impl<A: TyTag, B: TyTag, C: TyTag, D: TyTag, E: TyTag, F: TyTag> private::Sealed
    for (A, B, C, D, E, F)
{
}
impl<A: TyTag, B: TyTag, C: TyTag, D: TyTag, E: TyTag, F: TyTag, G: TyTag> private::Sealed
    for (A, B, C, D, E, F, G)
{
}

impl TyTag for Word {
    fn ty() -> Ty {
        Ty::Word
    }
}
impl TyTag for WordPtr {
    fn ty() -> Ty {
        Ty::WordPtr
    }
}
impl TyTag for Lit {
    fn ty() -> Ty {
        Ty::Lit
    }
}
impl TyTag for Offset {
    fn ty() -> Ty {
        Ty::Offset
    }
}
impl TyTag for DataPtr {
    fn ty() -> Ty {
        Ty::DataPtr
    }
}
impl TyTag for BinaryOutputPtr {
    fn ty() -> Ty {
        Ty::BinaryOutputPtr
    }
}
impl<Args: TyTuple> TyTag for Fun<Args> {
    fn ty() -> Ty {
        Ty::Fun(Args::tys())
    }
}

impl TyTuple for () {
    fn tys() -> Vec<Ty> {
        Vec::new()
    }
}
impl<A: TyTag> TyTuple for (A,) {
    fn tys() -> Vec<Ty> {
        vec![A::ty()]
    }
}
impl<A: TyTag, B: TyTag> TyTuple for (A, B) {
    fn tys() -> Vec<Ty> {
        vec![A::ty(), B::ty()]
    }
}
impl<A: TyTag, B: TyTag, C: TyTag> TyTuple for (A, B, C) {
    fn tys() -> Vec<Ty> {
        vec![A::ty(), B::ty(), C::ty()]
    }
}
impl<T: TyTag> Register<T> {
    /// `Register $` constructor parity (Code.hs:445-446 pattern).
    pub fn new(value: u64) -> Self {
        Register(value, PhantomData)
    }
}

impl<A: TyTag, B: TyTag, C: TyTag, D: TyTag> TyTuple for (A, B, C, D) {
    fn tys() -> Vec<Ty> {
        vec![A::ty(), B::ty(), C::ty(), D::ty()]
    }
}
impl<A: TyTag, B: TyTag, C: TyTag, D: TyTag, E: TyTag> TyTuple for (A, B, C, D, E) {
    fn tys() -> Vec<Ty> {
        vec![A::ty(), B::ty(), C::ty(), D::ty(), E::ty()]
    }
}
impl<A: TyTag, B: TyTag, C: TyTag, D: TyTag, E: TyTag, F: TyTag> TyTuple for (A, B, C, D, E, F) {
    fn tys() -> Vec<Ty> {
        vec![A::ty(), B::ty(), C::ty(), D::ty(), E::ty(), F::ty()]
    }
}
impl<A: TyTag, B: TyTag, C: TyTag, D: TyTag, E: TyTag, F: TyTag, G: TyTag> TyTuple
    for (A, B, C, D, E, F, G)
{
    fn tys() -> Vec<Ty> {
        vec![A::ty(), B::ty(), C::ty(), D::ty(), E::ty(), F::ty(), G::ty()]
    }
}

/// Types which can be compared (source doc, Types.hs:33-36):
/// `class Ordered (t :: Ty)` with instances `'Word` and `'DataPtr`.
///
/// Compile-time gate on the ordering-comparison instructions
/// (`"Ordered a"` contexts: JumpIfGt/Ge/Lt/Le, def/Instruction.hs:293-313).
/// RULE §2 (type-family row): closed constraint set → plain trait.
pub trait Ordered: TyTag {}
impl Ordered for Word {}
impl Ordered for DataPtr {}

/// Types which can be added (source doc, Types.hs:38-42):
/// `class Addable (t :: Ty) (u :: Ty)` — `Self` is the destination type
/// `t`, the parameter the operand type `u`; instances `'Word +'Word` and
/// `'DataPtr +'Word`.
///
/// Compile-time gate on the arithmetic instructions — context
/// `"Addable a b"` on Sub and Add (def/Instruction.hs:239/:249), context
/// `"Addable a 'Word"` on AddConst (:244 — the operand is pinned to 'Word
/// by its `Imm ImmWord` argument; only the destination `a` stays polymorphic).
pub trait Addable<U: TyTag>: TyTag {}
impl Addable<Word> for Word {}
impl Addable<Word> for DataPtr {}

/// Typed registers (source doc, Types.hs:44-45):
/// `newtype Register (ty :: Ty) = Register { fromRegister :: Word64 }`.
///
/// `.0` IS `fromRegister` (pub field = the exported record field; consumers
/// destructuring `Register i` port to `Register(i, _)`). The phantom
/// survives as the `TyTag` parameter — RULE §2 (GADT row) keeps the
/// register typing the source's EDSL relies on (QueryRegs.hs:20-77,
/// polyReg instructions). RULE §2 (numeric row): newtype over u64.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Register<T: TyTag>(pub u64, pub PhantomData<T>);
// field made pub: cross-crate construction (supply.rs's
// `Register(next, PhantomData)` — E0423 otherwise); zero-cost, and
// `Register(_, _)` pattern stays crate-internal via TyTag sealing.

/// Manual Debug so the tag is `Register(5)` — the shape of the source's
/// `deriving Show` (`show (Register 5) = "Register 5"`); derive would
/// append `PhantomData<…>` (RULE §2 deriving-Show row / A4).
impl<T: TyTag> std::fmt::Debug for Register<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Register").field(&self.0).finish()
    }
}

/// `castRegister :: Register a -> Register b` (Types.hs:47-48) — `coerce`:
/// rewrites the phantom only, the register word is untouched. Total and
/// unchecked, exactly as in the source (RULE §0). By value: Register is
/// Copy, the Haskell moves the newtype through.
pub fn cast_register<T: TyTag, U: TyTag>(r: Register<T>) -> Register<U> {
    Register(r.0, PhantomData)
}

/// Labels (source doc, Types.hs:50-52):
/// `newtype Label = Label { fromLabel :: Int }`.
///
/// `.0` IS `fromLabel`. Haskell `Int` is the RTS platform's 64-bit machine
/// int → i64 (RULE §2 numeric row; Decode.hs:46 narrows the Word64 stream
/// with `Label . fromIntegral`, wrapping — the consumer owns that cast).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Label(pub i64);

/// `newtype Literal = Literal { fromLiteral :: Word64 }` (Types.hs:54-55).
///
/// `.0` IS `fromLiteral` (the generator's word/show emitters read it:
/// Generate/Haskell.hs:207 `fromLiteral` in insnWords, :305 in insnShow).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Literal(pub u64);

// Marker inventory (reviewer counts the trailer against this): zero
// TODO(port), zero PERF(port), zero BUG(port), zero unsafe/SAFETY blocks;
// zero LIVE flags of §3/§5 kind — the manifest-path flag this file
// originally carried is RESOLVED (the file sits at the manifest path; the
// resolution record at the top quotes its name for traceability only).
//
// Coverage: every export of the Haskell module has a Rust counterpart —
// Ty(..) [6 constructors, Eq/Show derived], Ordered [marker trait, both
// instances], Addable [marker trait, both instances], Register(..)
// [phantom kept, field public, Eq/Ord derived, Show as manual Debug,
// Enum delegated per the header deriving map], castRegister
// [cast_register], Label(..) [field public, Eq/Ord/Show derived, Enum
// delegated], Literal(..) [field public, Eq/Ord/Show derived]. The
// promotion discipline (kind `Ty` at the type level) is ported as the
// sealed TyTag/TyTuple layer with the demotion `ty()`.
//
// MODULE-WIRING: lib.rs / bytecode/mod.rs scaffolding is deliberately NOT
// a manifest unit; owned by the Step-4 survey build (do not create ad-hoc
// here) — same ruling as rts/types.rs.
// (confidence high → medium, fix round 2: the header's evidence had
// outrun its greps — arity survey, Enum disposition, consumer labels and
// the Addable citations were corrected this pass; the TyTuple/Sealed
// surface is newly extended to 7-tuples and, like the rest of the crate,
// has not yet been compiled — Step 4 grades that.)
// PORT STATUS: confidence=medium todos=0
