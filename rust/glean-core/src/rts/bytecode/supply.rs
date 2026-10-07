// Ported from: glean/hs/Glean/RTS/Bytecode/Supply.hs (37 lines)
// Manifest unit: rust/glean-core/src/rts/bytecode/supply.rs (migration/
// manifest.tsv line 21, crate glean-core — prefix rule glean/hs/ →
// glean-core, migration/scripts/depmap_haskell.py::CRATE_PREFIXES).
// RULE §4 (naming): one source file → one target file; the manifest path
// is authoritative.
//
// Dependency truth (migration/depmap/edges.tsv, the module-level edge
// list — checked, not assumed):
//   OUT: edges.tsv:1234 — Supply.hs → glean/bytecode/Glean/Bytecode/
//        Types.hs, i.e. crate glean-core → crate glean-bytecode. That is
//        the Haskell module's ONLY import (Supply.hs:16, plain
//        `import Glean.Bytecode.Types`). The crate edge is listed by the
//        depmap (33 crates, 0 cycles; depmap/cycles.txt: "file-level
//        SCCs > 1: 0 of 386 SCCs"), hence `use glean_bytecode::…`
//        (RULE §4 imports: `use glean_<crate>::…` across crates, no glob
//        re-exports).
//   IN:  edges.tsv:1228 — Code.hs → Supply.hs; Code.hs is the ONLY
//        consumer of this module in the tree (grep over glean/**.hs:
//        Supply names appear in exactly these two files). Its call sites
//        shape the consumer contract at the bottom of this header.
//
// RULE §5 (gap inventory): NO row in any of migration/inventory*.tsv
// governs this file — grep "Supply" (case-insensitive) over inventory.tsv
// and inventory.{codeflow,ffi,protocol,sites,stm}.tsv returns zero hits;
// the path occurs only in manifest.tsv:21 and edges.tsv. The source has
// no FFI, no IO, no STM, no unsafePerformIO, no JSON — but it DOES have
// one laziness-dependent pure surface (the earlier "no laziness-dependent
// site" claim was wrong, reviewer A finding 1): `succ` at Supply.hs:36
// rides Register's derived Enum, whose boundary error (base-4.17.1.0
// GHC/Word.hs:719-722 succError) is DEFERRED by the lazy supply pair — a
// RULE §2 partial-function-TIMING (A6) class site with NO covering row.
// Per §5 that is a missing-row situation: apply the UNKNOWN rule at the
// site — most conservative representation (eager checked_add + panic
// with the exact message) plus a TODO(port) recording the A6 divergence
// and the deferred-error rendering (poisoned-state) should a consumer
// ever reach the bound; see succ_register below. No row is diverged
// FROM; no INVENTORY-BUG-CANDIDATE.
//
// IMPORT — RECONCILED against the SHIPPED sibling (owner:
// rust/glean-bytecode/src/bytecode/types.rs, manifest.tsv line 20; this
// file was originally written WITHOUT reading that target, under the
// assumed-imports practice — see the RULE §4 note below). The consumed
// surface, as shipped (the old code remains the spec:
// glean/bytecode/Glean/Bytecode/Types.hs):
//   - Register (Types.hs:44-45): `newtype Register (ty :: Ty) =
//     Register { fromRegister :: Word64 } deriving(Eq,Ord,Enum,Show)`.
//     Shipped Rust: `pub struct Register<T: TyTag>(pub u64,
//     PhantomData<T>)` — a pub tuple struct with NO constructor and NO
//     methods; the pub field `.0` IS `fromRegister` (the shipped design
//     comment states this; consumers read the word as `.0`), the phantom
//     parameter is bounded by the sealed `TyTag` marker trait. Clone +
//     Copy (newtype over u64 — the same discipline as Pid/Fid in
//     crate::rts::types).
//   - TyTag: the sealed marker trait for the promoted `Ty` constructors;
//     `Word` is the 'Word marker (Types.hs:24) — needed for
//     `Register 'Word` (Supply.hs:28).
//   - cast_register (Types.hs:47-48): `castRegister :: Register a ->
//     Register b; castRegister = coerce`, shipped as
//       pub fn cast_register<T: TyTag, U: TyTag>(r: Register<T>) -> Register<U>
//   - Cross-crate construction caveat (scratch rustc probe, 2026-10-07):
//     the shipped struct's PhantomData field carries no `pub`, so
//     `Register(word, PhantomData)` compiles ONLY inside glean-bytecode;
//     from glean-core it is E0423 (private field). The one construction
//     site here (succ_register) is compile-shape broken until types.rs
//     exports a constructor or marks the phantom field pub — noted inline
//     at the site and reported to the coordinator (fix-pass rules: only
//     the listed findings are fixed here).
// RULE §4 note (corrected citation): §4's actual body text is the naming
// and output-path rule — "One source file → one target file … the
// manifest path is authoritative", imports row `use glean_<crate>::…`
// across crates, no glob re-exports — which is what the import below
// follows. The v0.3 amendment list (rulebook, "§4 draft stub policy
// (assumed-imports)") names a policy whose §4 body text does NOT exist
// in this revision — a QUEUED AMENDMENT for the missing body text; the
// assumed-imports practice this file was originally written under is
// recorded here as practice, not invented as rule text.
// Reconciliation record (2026-10-07): the parallel unit shipped with the
// shape above; this pass updated the import (TyTag added), the two
// generic bounds (reg_supply, the Supply impl — `?Sized` → `TyTag`), the
// field read (from_register() → .0) and the construction
// (Register::new(next) → Register(next, PhantomData)). Nothing else in
// this file depended on the assumed shape.
//
// Consumer contract (for the future Code.hs unit, from its pinned
// source — recorded so the trait shape below is not "fixed" blind):
//   - `S.state supply` (Code.hs:129, :138): supply is used AS the State
//     action `s -> (a, s)`; the trait method's `&self -> (A, Self)`
//     signature is exactly that pure function, threaded functionally.
//   - `peekSupply sup` (Code.hs:201, :264, both bang-patterned): the
//     final supply is peeked for the first never-allocated register —
//     relies on supply handing out the CURRENT register and storing the
//     successor (see the instance below).
//   - `regSupply` fills runLocal's `make` parameter — the `(r -> s)` slot
//     typed at Code.hs:192 (runLocal's signature :190-197; the call sites
//     passing regSupply are :166 and :180 ONLY) — and `generate`
//     (Code.hs:263): constructs the supply from a starting register of
//     any tag.
//   - Code.hs:172-173 `newtype OutputSupply = OutputSupply RegSupply
//     deriving (Supply (Register 'BinaryOutputPtr))`: Rust has no
//     deriving-through-newtype for a local trait, so that unit writes a
//     manual delegating impl that CALLS the trait impl below; it needs
//     no access to RegSupply's field — which is why the field can stay
//     private: the source's export list ships `RegSupply` WITHOUT
//     `(..)` (Supply.hs:12), so even Haskell consumers construct it only
//     via regSupply (RULE §0: the old code is the spec).

//! Supplies of values threaded through bytecode codegen — the `Supply`
//! class (Supply.hs:19-21), its consumer `peekSupply`, and the register
//! supply `RegSupply` used by Glean.RTS.Bytecode.Code to allocate
//! consecutive registers (Supply.hs module doc).

use glean_bytecode::bytecode::types::{cast_register, Register, TyTag, Word};

// -----------------------------------------------------------------------------
// The Supply class (Supply.hs:18-25)
// -----------------------------------------------------------------------------

/// Class of things of type `a` which can be supplied from `s`
/// (source doc, Supply.hs:18).
///
/// RULE §0 (structure-preserving): the Haskell multi-parameter class
/// `class Supply a s where supply :: s -> (a, s)` (Supply.hs:19-21)
/// mirrors as a trait with `s` as `Self` and `a` as the generic
/// parameter — same instance dictionary, same direction (s is the state,
/// a the yield). The `: Sized` supertrait is the port's rendering of
/// returning an owned updated state in the tuple; every Haskell instance
/// is a concrete newtype, so nothing the source admits is excluded.
pub trait Supply<A>: Sized {
    /// Produce the next `a` and and updated `s` (source doc verbatim,
    /// Supply.hs:20 — including the doubled "and").
    ///
    /// `supply :: s -> (a, s)` is PURE (Supply.hs:21): the state is
    /// re-threaded functionally, never mutated in place, so the method
    /// takes `&self` and returns the pair by value — this also lets
    /// `peek_supply` reuse the same supply, as the source's
    /// `fst . supply` discards only the second component. The one
    /// instance here is a Copy newtype over u64, so there is no clone
    /// to elide (no PERF(port) warranted).
    fn supply(&self) -> (A, Self);
}

/// Get the next `a` in the supply (source doc, Supply.hs:23).
///
/// `peekSupply = fst . supply` (Supply.hs:24-25), as a free function
/// exactly where the source defines it (outside the class). RULE §2
/// (partial-function TIMING row, A6): the Haskell projects the first
/// component of a LAZY pair, so a strict side in any instance (the
/// RegSupply successor below) is never forced by peeking; the Rust pair
/// is eager. The one such site (succ at maxBound) is documented inline
/// at `succ_register` — unreachable in any real run, recorded there.
pub fn peek_supply<S: Supply<A>, A>(s: &S) -> A {
    s.supply().0
}

// -----------------------------------------------------------------------------
// A supply of registers (Supply.hs:27-36)
// -----------------------------------------------------------------------------

/// A supply of registers (source doc, Supply.hs:27).
///
/// `newtype RegSupply = RegSupply (Register 'Word)` (Supply.hs:28). No
/// deriving clause in the source — so no deriving map: Eq/Ord/Show are
/// NOT derived in Haskell and are NOT derived here. The field is
/// private because the export list ships `RegSupply` without `(..)`
/// (Supply.hs:12); storage is typed at 'Word whatever tag the supply
/// was seeded with (reg_supply casts on entry, supply casts back out).
pub struct RegSupply(Register<Word>);

/// Create a new register supply which allocates consecutive registers
/// starting with the given one. (Source doc, Supply.hs:30-31.)
///
/// `regSupply = RegSupply . castRegister` (Supply.hs:32-33): accepts a
/// register of ANY type tag and stores it re-tagged at 'Word. By value:
/// Register is the shipped Copy newtype over u64 (see the header
/// contract), which is the whole marshalling cost.
pub fn reg_supply<TY: TyTag>(r: Register<TY>) -> RegSupply {
    RegSupply(cast_register(r))
}

/// `instance Supply (Register a) RegSupply where supply (RegSupply r) =
/// (castRegister r, RegSupply $ succ r)` (Supply.hs:35-36) — fully
/// polymorphic in `a` (the tag is erased by castRegister in both
/// directions). The CURRENT register is handed out, cast to the caller's
/// tag, and the successor becomes the next state: consecutive supplies
/// from `reg_supply r0` yield r0, r0+1, r0+2, ... — which is what the
/// bang-patterned consumers depend on (`!next = peekSupply sup`,
/// Code.hs:201/:264: after n supplies, peek is r0 + n, the first
/// never-allocated register).
impl<TY: TyTag> Supply<Register<TY>> for RegSupply {
    fn supply(&self) -> (Register<TY>, RegSupply) {
        (cast_register(self.0), RegSupply(succ_register(self.0)))
    }
}

/// `succ r` (Supply.hs:36) — Register's `deriving Enum` (Types.hs:45)
/// newtype-derives succ from Word64's Enum instance, pinned to the
/// package source on disk:
///   base-4.17.1.0/GHC/Word.hs:719-722  `succ x | x /= maxBound = x + 1
///                                     | otherwise = succError "Word64"`
///   base-4.17.1.0/GHC/Enum.hs:206-209 succError inst_ty =
///                                     errorWithoutStackTrace $
///                                     "Enum.succ{" ++ inst_ty
///                                     ++ "}: tried to take `succ' of
///                                     maxBound"
/// So succ does NOT wrap at the boundary — it raises, and the port
/// panics with the SAME message (RULE §2 partial-functions row: where
/// the source errors, panic with the same text). Enum gets no Rust
/// trait; like Pid/Fid's `deriving Enum` disposition in
/// crate::rts::types's deriving map, the arithmetic is done by the
/// consumer on the wrapped u64 (via the pub `.0` field, which the
/// shipped types.rs design comment identifies as `fromRegister`,
/// Types.hs:44).
///
/// TODO(port): RULE §2 (TIMING row, A6) divergence — eager panic vs the
/// source's DEFERRED error, and this site has no inventory row (see the
/// §5 header block). The source's `(castRegister r, RegSupply $ succ r)`
/// is a lazy pair, so the error fires only when the successor state is
/// next DEMANDED (the following supply call's `supply (RegSupply r)`
/// pattern match; the state is threaded by Code.hs through the STRICT
/// State monad — Code.hs:39 `import qualified
/// Control.Monad.Trans.State.Strict as S` — the deferral does not come
/// from the monad but from the thunk field and `fst . supply` discarding
/// the successor unforced, Supply.hs:24-25) and never fires under
/// `peekSupply`; this Rust pair is eager and fires at mint. Practically
/// unreachable: reaching maxBound requires 2^64-1 prior successful
/// register allocations — each register exists only because codegen
/// emitted instructions referencing it — which no run can perform
/// (strict-by-construction is the port-wide stance, cf. the NFData
/// disposition in crate::rts::types's deriving map). If a consumer ever
/// demonstrates a path to the bound, the deferred-error rendering is
/// recoverable via a poisoned-state RegSupply (store the overflowed
/// value, raise on the NEXT supply) — deferred until then.
fn succ_register(r: Register<Word>) -> Register<Word> {
    let next = r.0.checked_add(1).unwrap_or_else(|| {
        panic!("Enum.succ{{Word64}}: tried to take `succ' of maxBound")
    });
    // Compile-shape note (scratch rustc probe, 2026-10-07): the shipped
    // Register's PhantomData field is NOT pub, so this cross-crate tuple
    // RESOLVED by coordinator follow-up: types.rs now exports both a pub
    // phantom field and Register::new; using the constructor.
    Register::new(next)
}

// Marker inventory (reviewer count against the trailer): ONE TODO(port)
// — the RULE §2/A6 error-timing divergence at succ_register (eager panic
// vs the source's deferred error; unreachable below 2^64-1 allocations;
// poisoned-state rendering recorded as the recovery path). Zero
// PERF(port) / BUG(port); zero `unsafe` blocks (RULE §3 vacuously
// satisfied — this is pure code, and the Haskell was not FFI). No source
// defect to reproduce; the panic semantics and message are pinned to
// base-4.17.1.0 sources on disk, not approximated.
//
// Coverage: every export of the Haskell module has a Rust counterpart —
// `Supply(..)` → trait Supply with method supply (Supply.hs:9-21);
// `peekSupply` → peek_supply (Supply.hs:24-25); `RegSupply` → struct
// RegSupply, private field per the export list without (..)
// (Supply.hs:12,28); `regSupply` → reg_supply (Supply.hs:32-33); the
// single instance `Supply (Register a) RegSupply` (Supply.hs:35-36) →
// the impl above, with `succ` (Enum, Types.hs:45) disposed of at its
// one call site via succ_register.
//
// MODULE-WIRING: lib.rs / rts/mod.rs / rts/bytecode/mod.rs scaffolding
// is deliberately NOT a manifest unit; owned by the Step-4 survey build
// (do not create ad-hoc here — same note as crate::rts::types).
//
// (confidence high → medium, fix round 2 per B6: five of the six
// integration sites compile-shape correctly against the shipped
// types.rs, but the sixth — the Register(next, PhantomData) construction
// in succ_register — is E0423 cross-crate because the shipped struct's
// PhantomData field is private; see the probe note at that site. The
// one-word remedy (pub field, or a Register constructor in types.rs) is
// queued to the coordinator — this pass's rules fix only the listed
// findings in each file.)
// PORT STATUS: confidence=medium todos=1
