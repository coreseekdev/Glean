// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// Id-like phantom-tagged newtypes — PidOf / IdOf — plus the Pid / Fid
// re-export from Glean.RTS.Types.
//
// Ported from: glean/typed/Glean/Typed/Id.hs (41 lines)
// Manifest unit: rust/glean-typed/src/typed/id.rs
// (migration/manifest.tsv line 16, crate glean-typed). RULE §4: one source
// file -> one target file; the manifest path is authoritative.
//
// FIRST FILE of the glean-typed crate. The crate root (glean/typed/Glean/
// Typed.hs, manifest row 37) and the crate's Cargo.toml/lib.rs do not exist
// yet, so this file has no in-crate siblings to read. Its two dependencies
// are DONE crates read below: glean-core's rts::types (Pid/Fid) and
// glean-angle's display (Doc/Display). The crate-root module wiring
// (`pub mod id;` in typed.rs) is owned by unit 37, not here (MODULE-WIRING).
//
// INVENTORY (RULE §5): NO inventory row governs this file — grep
// "Glean/Typed/Id" across all six migration/inventory*.tsv returns zero rows
// (the path appears only in migration/depmap/edges.tsv:1846 and
// manifest.tsv:16). The file is a leaf of the dependency graph: one outbound
// module edge (Id.hs -> Glean/RTS/Types.hs) and five inbound edges
// (edges.tsv:1821/1831/1840/1844/1851 — Typed.hs, Binary.hs, BuildFact.hs,
// Fact.hs, Predicate.hs). The §5-class construct here — the phantom-tagged id
// newtypes — is not enumerated in any inventory, so there is no row to follow
// and no divergence; the source itself is the spec (RULE §0). No
// INVENTORY-BUG-CANDIDATE.
//
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, module-level): the source's
// only module edge is outbound row 1846 (Id.hs -> glean/hs/Glean/RTS/Types.hs,
// crate glean-core) — `import Glean.RTS.Types` (Id.hs:24). The remaining
// imports are external packages, all outside the Glean edge graph (boot/hsthrift
// packages, pinned deps): Control.DeepSeq (Id.hs:18), Data.Hashable (Id.hs:19),
// Compat.Prettyprinter (Id.hs:20, hsthrift common-util), Data.Vector.Primitive
// (Id.hs:21), Foreign.Storable (Id.hs:22). Their dispositions are the deriving
// map below. No in-crate edge.
//
// CROSS-CRATE NOTE (unit brief correction): the brief sketches the re-export
// as `crate::rts::types`, which presumes this file lives in glean-core. It
// does not — the manifest (RULE §4, authoritative) puts it in glean-typed, and
// glean-typed has no `rts` module. Pid/Fid therefore cross the crate boundary
// as `glean_core::rts::types` (module edge edges.tsv:1846). The import path
// is the only consequence; the source's re-export is followed exactly.
//
// SIBLING-UNIT CONTRACTS (RULE §4; DONE crates' real APIs read, never
// re-declared):
//   * glean_core::rts::types (DONE, read): `Pid` (#[repr(transparent)] over
//     i64, types.rs:120-122) and `Fid` (#[repr(transparent)] over the thrift
//     Id = i64, types.rs:153-155), both Copy + Hash + Debug + Display. The
//     source re-exports `Pid(..), Fid(..)` from Glean.RTS.Types (Id.hs:14) —
//     mirrored by the `pub use` below. Neither type is re-declared here.
//   * glean_angle::display (DONE, read): `Doc` (display.rs:109-138), the
//     `Display` trait and its `display(&self, &DisplayOpts) -> Doc` method
//     (display.rs:612-625), `DisplayOpts` (display.rs:570). This file
//     implements `Display` for PidOf/IdOf against that trait and never
//     re-declares it; the delegation below calls Pid/Fid's own Display impls
//     (rts/types.rs:127-133, :158-164) rather than re-rendering.
//   * glean-bytecode's `Register` (read): the house precedent for a
//     phantom-tagged newtype — derive(Clone, Copy, PartialEq, Eq, PartialOrd,
//     Ord) plus a MANUAL Debug that drops the `PhantomData` field noise
//     (rust/glean-bytecode/src/bytecode/types.rs:373-386). Followed here for
//     the same reason.
//
// Deriving map (every Haskell deriving has a disposition; none silently
// dropped):
//   Eq/Ord/Hashable -> derive(PartialEq, Eq, PartialOrd, Ord, Hash). PidOf
//     takes Eq+Ord (Id.hs:30); IdOf takes Eq+Ord+Hashable (Id.hs:38). The
//     derive adds a `P: …` bound on the phantom parameter (Rust derives bound
//     every type param appearing in a field type); the source's stock
//     instances carry NO constraint on `p` (it is phantom). Harmless and
//     same-as-sibling: the compare/hash never touch `P`, and every in-tree
//     `P` is a predicate type that already has these instances — the exact
//     no-op the Register derivation relies on
//     (rust/glean-bytecode/src/bytecode/types.rs:126-130).
//   Show -> MANUAL Debug: the port-wide stand-in for `deriving Show`
//     (rts/types.rs:62-67, RULE §2 deriving-Show row / A4). Derive would
//     append `PhantomData<fn() -> P>` to the tag; the manual impl keeps the
//     constructor-shaped record `PidOf { pid_of: Pid(5) }` (Register
//     precedent, bytecode/types.rs:379-386). The snake_case field name is the
//     systematic crate-wide layout drift.
//   Pretty instance -> impl Display (header: Pretty vs Display): the source
//     has no separate Display instance; `pretty` is the rendering path and
//     the port folds every Pretty into Display (rts/types.rs:82-89). Both
//     instances delegate to the wrapped Pid/Fid's Display, exactly as
//     `pretty (PidOf p) = pretty p` / `pretty (IdOf f) = pretty f` do.
//   NFData -> nothing: a strict-evaluation strategy token, not semantics;
//     Rust values are strict by construction (rts/types.rs:79-80).
//   Storable -> #[repr(transparent)] over the wrapped scalar: the layout the
//     source's Storable-derived instance guaranteed for by-value marshalling
//     (rts/types.rs:68-70). The INSTANCE itself has no Rust counterpart (no
//     Storable trait) and neither wrapper has an FFI call site (both are
//     phantom-tagged typed-layer values), so the by-value FFI half of the
//     mapping dissolves; the layout pin is kept. The family's Storable mapping
//     also adds Copy — deliberately NOT added here: Haskell values are all
//     implicitly copyable, but the port follows the source's instance list
//     exactly (which has no Copy/Clone deriving) and does not add Copy/Clone
//     beyond it (Pid/Fid are Copy, so the accessors return by value).
//   Prim (Data.Vector.Primitive) -> nothing: Rust's Vec<PidOf<P>> is already
//     monomorphic; the class existed only to permit unboxed vectors of the id
//     (rts/types.rs:76-78).
//
// Pretty vs Display: the source's only rendering instance is `Pretty`; the
// port has no Pretty trait — prettyprinter's Doc surface is owned by
// glean-angle's display.rs and the single rendering path is the Display trait
// — so each Pretty instance folds into the corresponding Display impl, the
// same folding Pid/Fid themselves use (rts/types.rs:82-89). Nothing is lost:
// for these wrappers Display IS pretty by definition.
//
// MODULE-WIRING: lib.rs / Cargo.toml / the crate-root `typed.rs` module
// declarations are deliberately NOT a manifest unit; owned by unit 37
// (Glean/Typed.hs). Not created here.

use std::marker::PhantomData;

use glean_angle::display::{Display, DisplayOpts, Doc};

// The source re-exports `Pid(..), Fid(..)` from Glean.RTS.Types (Id.hs:14).
// Its Rust home is the DONE glean-core crate, so the re-export is cross-crate
// (RULE §4: named re-exports only, no glob re-exports across crates).
pub use glean_core::rts::types::{Fid, Pid};

// ---------------------------------------------------------------------------
// PidOf
// ---------------------------------------------------------------------------

/// `newtype PidOf p = PidOf { pidOf :: Pid } deriving(Eq, Ord, Show)`
/// (Id.hs:29-30) — tie a `Pid` to a predicate phantom type `p`.
///
/// The phantom `p` is a marker only: it is carried by `PhantomData` and is
/// never compared or hashed, so the instances are exactly the wrapped `Pid`'s.
/// `PhantomData<fn() -> P>` (not `PhantomData<P>`) keeps the wrapper's
/// auto-traits (`Send`/`Sync`/`Unpin`) independent of `P` — the same reasoning
/// as the sibling `Subroutine<S>` marker
/// (rust/glean-core/src/rts/foreign/bytecode.rs:245-249).
#[derive(PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PidOf<P> {
    pid_of: Pid,
    marker: PhantomData<fn() -> P>,
}

impl<P> PidOf<P> {
    /// `PidOf` constructor (Id.hs:29) — the source's exported constructor
    /// (`PidOf(..)`, Id.hs:15). The Haskell positional field is the record
    /// field `pidOf`; construction is a `new` helper so the private phantom
    /// stays encapsulated.
    pub fn new(pid: Pid) -> Self {
        PidOf {
            pid_of: pid,
            marker: PhantomData,
        }
    }

    /// `pidOf` record-field read (Id.hs:29) — the source's field accessor.
    pub fn pid_of(&self) -> Pid {
        self.pid_of
    }
}

/// Manual `Debug` (deriving map, Show row): the source's `deriving Show`
/// renders `PidOf {pidOf = Pid 5}`; derive would append the `PhantomData`
/// field (Register precedent, bytecode/types.rs:379-386). The snake_case field
/// name is the crate-wide layout drift.
impl<P> std::fmt::Debug for PidOf<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PidOf").field("pid_of", &self.pid_of).finish()
    }
}

/// `instance Pretty (PidOf p) where pretty (PidOf p) = pretty p`
/// (Id.hs:32-33), folded into `Display` (header: Pretty vs Display). The
/// wrapped `Pid`'s Display renders `{<decimal>}` (rts/types.rs:124-133) —
/// exactly what `pretty p` did on the wrapped id.
impl<P> Display for PidOf<P> {
    fn display(&self, opts: &DisplayOpts) -> Doc {
        self.pid_of.display(opts)
    }
}

// ---------------------------------------------------------------------------
// IdOf
// ---------------------------------------------------------------------------

/// `newtype IdOf p = IdOf { idOf :: Fid } deriving(Eq, Ord, Show, Hashable,
/// NFData, Storable, Prim)` (Id.hs:37-38) — the id of a fact of a given
/// predicate type `p` (phantom type parameter). NOT the Id/Fid/Pid of the
/// definition of predicate `p` (source doc, Id.hs:35-36).
///
/// The phantom `p` is a marker only; `PhantomData<fn() -> P>` keeps the
/// wrapper's auto-traits independent of `P` (same as `PidOf` above).
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct IdOf<P> {
    id_of: Fid,
    marker: PhantomData<fn() -> P>,
}

impl<P> IdOf<P> {
    /// `IdOf` constructor (Id.hs:37) — the source's exported constructor
    /// (`IdOf(..)`, Id.hs:15); the record field is `idOf`.
    pub fn new(id: Fid) -> Self {
        IdOf {
            id_of: id,
            marker: PhantomData,
        }
    }

    /// `idOf` record-field read (Id.hs:37) — the source's field accessor.
    pub fn id_of(&self) -> Fid {
        self.id_of
    }
}

/// Manual `Debug` (deriving map, Show row) — see `PidOf` above.
impl<P> std::fmt::Debug for IdOf<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IdOf").field("id_of", &self.id_of).finish()
    }
}

/// `instance Pretty (IdOf p) where pretty (IdOf f) = pretty f`
/// (Id.hs:40-41), folded into `Display` (header: Pretty vs Display). The
/// wrapped `Fid`'s Display renders `{<decimal>}` (rts/types.rs:157-164).
impl<P> Display for IdOf<P> {
    fn display(&self, opts: &DisplayOpts) -> Doc {
        self.id_of.display(opts)
    }
}

// Coverage: every export of the Haskell module has a Rust counterpart —
// Pid(..)/Fid(..) (Id.hs:14) -> the two `pub use` re-exports; PidOf(..)
// (Id.hs:15) -> `PidOf` + `PidOf::new` (constructor) + `PidOf::pid_of`
// (record field); IdOf(..) (Id.hs:15) -> `IdOf` + `IdOf::new` + `IdOf::id_of`.
// Every instance is ported: PidOf Eq/Ord/Show/Pretty, IdOf Eq/Ord/Show/
// Hashable/Pretty, and the deriving map disposes of NFData/Storable/Prim.
//
// Marker breakdown (closed set — no open port TODOs, no source-defect
// reproductions, no performance deferrals, no inventory-bug-candidates):
//   * no open port TODOs: every source construct has a disposition — the two
//     newtypes, their constructors and field reads, the Eq/Ord/Show/Hashable/
//     Pretty instances, and the Pid/Fid re-export. NFData/Prim/Storable
//     dissolve per the deriving map; the family convention
//     (rts/types.rs:68-80) settles them, so no open question remains.
//   * no source-defect reproductions: the source has no defect here — a
//     41-line newtype file whose only oddity is the stale
//     "does not import Glean modules" comment at Id.hs:11 (a comment, not
//     behaviour).
//   * no performance deferrals: the wrappers are zero-cost by construction
//     (one scalar plus a ZST phantom).
//   * no inventory-bug-candidates: no row exists (see the INVENTORY block).
// SAFETY count: 0 unsafe constructs — this unit is pure safe Rust; there is
// no unsafe code and therefore no soundness comments.
// PORT STATUS: confidence=high todos=0
