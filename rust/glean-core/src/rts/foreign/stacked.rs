// Ported from: glean/hs/Glean/RTS/Foreign/Stacked.hs (59 lines)
// Manifest unit: rust/glean-core/src/rts/foreign/stacked.rs
// (migration/manifest.tsv line 30, crate glean-core). RULE §4: one source
// file -> one target file; the manifest path is authoritative.
//
// ABI truth (read, not assumed): glean/rts/ffi.h —
//   glean_stacked_lookup_new :313-314  (`Lookup* base, Lookup* added,
//                                        Lookup** stacked` -> const char*)
//   glean_stacked_define_new :316-317  (`Lookup* base, Define* added,
//                                        Define** stacked` -> const char*)
//   glean_stacked_define_free :318     (`Define* stacked`, void)
//   glean_lookup_free        :204      (`Lookup* lookup`, void; declared by
//                                        this unit too — see `sys`)
// and their C++ bodies at glean/rts/ffi.cpp:645-648 (lookup_new),
// :650-653 (define_new), :655-657 (define_free: `ffi::free_`), :313-315
// (glean_lookup_free: `ffi::free_`). Both `_new` bodies are
// `ffi::wrap([=] { *stacked = new Stacked<...>(base, added); })` —
// ALLOCATION-GUARD: the only failure mode is bad_alloc; no precondition
// check exists to fail. The view itself is the C++ class at
// glean/rts/stacked.h: `StackedBase<Iface>` :21-144 (ctor :22-23 records
// `mid = stacked->startingId()`; reads prefer `added`, falling back to
// `base` only below `mid`), `Stacked<Lookup>` :158-161 and
// `Stacked<Define>` :163-... (the latter adds the define override; its own
// source TODO at :171-172 is the C++'s, not re-implemented here). The
// Haskell's `foreign import`s are Stacked.hs:51-59; it imports
// `Util.FFI` (Stacked.hs:19), never Glean.FFI — both kits land in
// crate::ffi per RULE §0's home move.
//
// INVENTORY (RULE §5) — inventory.tsv row 73 (line 73; duplicated as
// inventory.ffi.tsv row 15), followed via its conservative fallback:
//   "temporary virtual Lookup/Define view over (base, added) pair;
//   bracketed lifetime | BORROWED | port as pure composition of two &dyn
//   Lookup / &dyn Define trait objects (no allocation needed);
//   conservative fallback: keep cxx view with Drop =
//   glean_stacked_lookup_free / glean_stacked_define_free; existential
//   Stacked base -> enum/Box<dyn CanLookup> per RULEBOOK §2 GADT row".
//   * The cxx view is KEPT (the row's fallback): the C++ `Stacked` IS the
//     semantics the source buys (`mid`-split reads), so re-composing two
//     Rust trait objects would be a reimplementation, not a port. Its
//     lifetime is bracketed by the two guard types below, exactly the
//     source's `bracket` (Stacked.hs:37-40 and :46-49).
//   * The row's first choice ("pure composition of two &dyn Lookup / &dyn
//     Define trait objects") is UNIMPLEMENTABLE against unit 18's real
//     trait: `CanLookup::with_lookup` is generic over `T`/`E`
//     (lookup.rs:114-121), so the trait is not object-safe and no `&dyn`
//     can exist; same argument for CanDefine's expected twin. This is a
//     property of the DONE sibling, not a port decision.
//   * The existential (`forall base. CanLookup base => ...`, Stacked.hs:25)
//     becomes the two-parameter generic `Stacked<B, A>` (`Box<dyn
//     CanLookup>` is out for the object-safety reason above; RULEBOOK §2
//     GADT row: "Box<dyn Trait> only where genuinely open" — the base is
//     closed over by the smart constructor here). The existential was how
//     the Haskell STORED the base; every in-tree construction passes two
//     concrete types (FactSet.hs:140 `stacked base added`), so the generic
//     is equivalent for the port and exposes the same surface (abstract
//     type, `stacked`, the two trait impls, the combined name). Documented
//     per the unit brief as an implementation detail — no row divergence.
//   * INVENTORY-BUG-CANDIDATE row 73 (row text only): the row's fallback
//     names `glean_stacked_lookup_free` as the lookup view's free
//     function. No such symbol exists anywhere (grep-verified: the only
//     matches in the tree are rows 73 and 15 themselves — ffi.h has no
//     declaration, ffi.cpp no definition). The Haskell frees the lookup
//     view with `glean_lookup_free` (Stacked.hs:39 + :53-54) — the symbol
//     whose body is `ffi::free_` (ffi.cpp:313-315) — and this port calls
//     that same symbol. The port follows the Haskell/ffi.h authority, not
//     the row's typo.
//
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, module-level): outbound —
// rows 1280 (Stacked.hs -> Define.hs: Define(..), CanDefine(..)) and 1281
// (Stacked.hs -> Lookup.hs: Lookup(..), CanLookup(..)). Inbound — rows 334
// (db/Glean/Database/Open.hs), 490 (db/.../Write/Batch.hs), 708
// (db/Glean/Query/UserQuery.hs), 1251 (FactSet.hs:138-141, the
// `renameFacts` construction `stacked base added` at :140) and 1880
// (write/Glean/Write/SendAndRebaseQueue.hs:371, the `snapshot `stacked`
// base `stacked` factSet` chain). This module
// imports NOTHING from a sibling Rust unit beyond crate::ffi and the two
// trait modules named above.
//
// SIBLING-UNIT CONTRACTS (RULE §4; DONE siblings' real APIs read, never
// re-declared):
//   * crate::ffi (DONE, read): `invoke` (the pinned form
//     `pub unsafe fn invoke(call: impl FnOnce() -> *const c_char)
//     -> Result<(), FfiError>` — the extern call passed AS the closure)
//     and `FfiError`.
//   * crate::rts::foreign::lookup (DONE, read): `CanLookup` — the pinned
//     trait `fn with_lookup<T, E: From<FfiError>>(&self, f: impl
//     FnOnce(*mut c_void) -> Result<T, E>) -> Result<T, E>; fn
//     lookup_name(&self) -> &str` (lookup.rs:114-121) — and the
//     `LookupGuard` Drop precedent (lookup.rs:152-163) this file's lookup
//     guard mirrors. The E-generic `?` on `invoke` mirrors the sibling's
//     `with_snapshot` exactly (lookup.rs:193-214).
//   * crate::rts::foreign::define (unit 28, landed in the same batch):
//     `CanDefine` confirmed as `fn with_define<T, E: From<FfiError>>(&self,
//     f: impl FnOnce(*mut c_void) -> Result<T, E>) -> Result<T, E>`
//     (define.rs:355-372) — same shape as this file's import/impl assumed
//     from the Haskell class (Define.hs:44-45), plus the `From` bound the
//     one-review rustc repro proved necessary; reconciled at the impl site
//     below.
//   * Why `pub mod sys` is public — for sibling consumers declaring their
//     own externs over the SAME opaque handles: this module re-declares
//     `glean_lookup_free` (Stacked.hs:53-54 declares its own import too);
//     both declarations are identical and address the same C symbol.

//! The `Stacked` virtual view — an `added` Lookup/Define stacked on top of
//! a base Lookup, with a combined `"base;added"` name.
//!
//! Port notes: RULE §3 — `unsafe` appears only in this glean-core FFI
//! wrapper, where the Haskell was already FFI; every block carries a
//! SAFETY comment. Error-recovery classification per RULE §2: both `_new`
//! bodies are ALLOCATION-GUARD only (they do `new Stacked<...>` and
//! nothing else — no precondition check can fail), and the errors are
//! returned through the trait methods' `Result` (the source's `invoke`
//! exception made explicit), never aborted — the caller decides, exactly
//! as the sibling `with_snapshot` does.

use std::os::raw::c_void;
use std::ptr;

use crate::ffi::{invoke, FfiError};
use crate::rts::foreign::define::CanDefine;
use crate::rts::foreign::lookup::CanLookup;

/// `data Stacked a = forall base. CanLookup base => Stacked base a Text`
/// (Stacked.hs:25) — "An `a` stacked on top of a base 'CanLookup'"
/// (Stacked.hs:24).
///
/// The existential base is the leading generic parameter `B` and the
/// Haskell's `a` is `A` (see the INVENTORY block at the top of the file
/// for why no `Box<dyn CanLookup>` can exist). Fields are private and
/// [`stacked`] is the only constructor, mirroring the source's export list
/// (`Stacked`, `stacked` — Stacked.hs:9-12).
pub struct Stacked<B, A> {
    base: B,
    added: A,
    name: String,
}

/// `stacked :: (CanLookup base, CanLookup a) => base -> a -> Stacked a`
/// (Stacked.hs:27-30) — stack `added` on top of `base`.
///
/// `name = lookupName base <> ";" <> lookupName l` (Stacked.hs:30),
/// computed before the handles move into the struct (Rust ownership takes
/// the Haskell's shared immutable values by value here — the unit brief's
/// pinned signature).
pub fn stacked<B: CanLookup, A: CanLookup>(base: B, added: A) -> Stacked<B, A> {
    let name = format!("{};{}", base.lookup_name(), added.lookup_name());
    Stacked { base, added, name }
}

/// Guard that frees the C++ lookup view with the source's finalizer
/// (`bracket ... glean_lookup_free`, Stacked.hs:37-40) — the mirror of
/// lookup.rs's `LookupGuard` (lookup.rs:152-163).
struct StackedLookupGuard(*mut c_void);

impl Drop for StackedLookupGuard {
    fn drop(&mut self) {
        // SAFETY: `self.0` is the `Lookup*` a successful
        // `glean_stacked_lookup_new` produced — the only construction site
        // is `with_lookup` below, which creates this guard only after the
        // call reported success. `glean_lookup_free` is the corresponding
        // free function (ffi.cpp:313-315), exactly the source's bracket
        // release. The guard is neither Copy nor Clone, so this runs once.
        unsafe { sys::glean_lookup_free(self.0) };
    }
}

/// Guard that frees the C++ define view with the source's finalizer
/// (`bracket ... glean_stacked_define_free`, Stacked.hs:46-49).
struct StackedDefineGuard(*mut c_void);

impl Drop for StackedDefineGuard {
    fn drop(&mut self) {
        // SAFETY: `self.0` is the `Define*` a successful
        // `glean_stacked_define_new` produced — the only construction site
        // is `with_define` below, which creates this guard only after the
        // call reported success. `glean_stacked_define_free` is the
        // corresponding free function (ffi.cpp:655-657, `ffi::free_`). The
        // guard is neither Copy nor Clone, so this runs once.
        unsafe { sys::glean_stacked_define_free(self.0) };
    }
}

/// `instance CanLookup a => CanLookup (Stacked a)` (Stacked.hs:32-40) — the
/// lookup view: reads prefer `added`, `base` serves only ids below the
/// added part's starting id (C++ StackedBase, glean/rts/stacked.h:24-138,
/// the Lookup overrides).
impl<B: CanLookup, A: CanLookup> CanLookup for Stacked<B, A> {
    /// `withLookup (Stacked base added _) f = withLookup base $ \p_base ->
    /// withLookup added $ \p_added -> bracket (invoke $
    /// glean_stacked_lookup_new p_base p_added) glean_lookup_free f`
    /// (Stacked.hs:34-40). The `E: From<FfiError>` bound is repeated from the
    /// trait declaration (lookup.rs:114-121) — required for the
    /// `invoke(...)?` below.
    fn with_lookup<T, E: From<FfiError>>(
        &self,
        f: impl FnOnce(*mut c_void) -> Result<T, E>,
    ) -> Result<T, E> {
        self.base.with_lookup(|p_base| {
            self.added.with_lookup(|p_added| {
                let mut stacked: *mut c_void = ptr::null_mut();

                // If `invoke` fails, the `?` returns before the guard is
                // created — no leak (the same early-return shape
                // lookup.rs's `with_snapshot` uses).
                // SAFETY: `p_base`/`p_added` are live `Lookup*`s (the two
                // with_lookup borrows keep them so for the whole call);
                // `stacked` is an initialised stack out-param; the extern
                // order mirrors ffi.h:313-314. The body (ffi.cpp:645-648)
                // is allocation-class only.
                unsafe {
                    invoke(|| sys::glean_stacked_lookup_new(p_base, p_added, &mut stacked))
                }?;

                let _guard = StackedLookupGuard(stacked);
                f(stacked)
            })
        })
    }

    /// `lookupName (Stacked _ _ name) = name` (Stacked.hs:33) — the
    /// combined name [`stacked`] computed.
    fn lookup_name(&self) -> &str {
        &self.name
    }
}

/// `instance CanDefine a => CanDefine (Stacked a)` (Stacked.hs:42-49).
///
/// NOTE the source's asymmetry, preserved verbatim: the BASE is taken as a
/// Lookup even on the define path (`withLookup base`, Stacked.hs:44) — the
/// C++ view is a `Stacked<Define>` whose base parameter is a `Lookup*`
/// (ffi.h:317), and reads of base facts go through Lookup.
//
// Reconciliation CLOSED: `CanDefine` landed with unit 28 in the same
// batch, and its trait shape is the assumed one plus the `From` bound —
// `fn with_define<T, E: From<FfiError>>(&self, f: impl FnOnce(*mut
// c_void) -> Result<T, E>) -> Result<T, E>` (define.rs:355-372;
// Define.hs:44-45 is the Haskell class). The bound was added to BOTH
// traits in the units 28–31 fix round: the `invoke(...)?` below is
// `E`-generic and needs `E: From<FfiError>`. No marker remains open here.
impl<B: CanLookup, A: CanDefine> CanDefine for Stacked<B, A> {
    /// `withDefine (Stacked base added _) f = withLookup base $ \p_base ->
    /// withDefine added $ \p_added -> bracket (invoke $
    /// glean_stacked_define_new p_base p_added) glean_stacked_define_free f`
    /// (Stacked.hs:43-49). The `E: From<FfiError>` bound is repeated from the
    /// trait declaration (define.rs) — required for the `invoke(...)?` below.
    fn with_define<T, E: From<FfiError>>(
        &self,
        f: impl FnOnce(*mut c_void) -> Result<T, E>,
    ) -> Result<T, E> {
        self.base.with_lookup(|p_base| {
            self.added.with_define(|p_added| {
                let mut stacked: *mut c_void = ptr::null_mut();

                // Same early-return shape as `with_lookup` above.
                // SAFETY: `p_base` is a live `Lookup*` (the with_lookup
                // borrow), `p_added` a live `Define*` (the with_define
                // borrow); `stacked` is an initialised stack out-param; the
                // extern order mirrors ffi.h:316-317. The body
                // (ffi.cpp:650-653) is allocation-class only.
                unsafe {
                    invoke(|| sys::glean_stacked_define_new(p_base, p_added, &mut stacked))
                }?;

                let _guard = StackedDefineGuard(stacked);
                f(stacked)
            })
        })
    }
}

/// Raw C-ABI declarations for the `glean_stacked_*` group plus the shared
/// `glean_lookup_free` (Stacked.hs:51-59 declares its own imports for all
/// four — the reason this module re-declares `glean_lookup_free` instead of
/// reaching into lookup.rs's `sys`; both declarations address the same C
/// symbol with identical types).
///
/// RULE §1 (FFI row, A2): raw `extern "C"` for flat C-ABI symbols. The
/// handles are the opaque `Lookup`/`Define` pointers (ffi.h), modelled as
/// `*mut c_void` — the same convention unit 18/28's `sys` use. Signatures
/// mirror ffi.h EXACTLY (`size_t`-free here; no id parameters).
pub mod sys {
    use std::os::raw::{c_char, c_void};

    // NOTE (RULE §3, family convention): the `unsafe extern "C"` block
    // below is a DECLARATION, not an operation — the same ruling lookup.rs
    // and subst.rs state for theirs. It introduces no unsafe behaviour of
    // its own, carries no SAFETY comment, and is NOT counted in the SAFETY
    // tally at the foot of this file.
    unsafe extern "C" {
        /// `glean/rts/ffi.h:313-314` — body ffi.cpp:645-648; the C++ ctor
        /// records `mid = added->startingId()` (stacked.h:22-23). No
        /// precondition check -> allocation-class only.
        pub fn glean_stacked_lookup_new(
            base: *mut c_void,
            added: *mut c_void,
            stacked: *mut *mut c_void,
        ) -> *const c_char;

        /// `glean/rts/ffi.h:316-317` — body ffi.cpp:650-653. NOTE the base
        /// is a `Lookup*` even on this define path (see the CanDefine impl).
        pub fn glean_stacked_define_new(
            base: *mut c_void,
            added: *mut c_void,
            stacked: *mut *mut c_void,
        ) -> *const c_char;

        /// `glean/rts/ffi.h:318` — body ffi.cpp:655-657 is `ffi::free_`
        /// (void; infallible).
        pub fn glean_stacked_define_free(stacked: *mut c_void);

        /// `glean/rts/ffi.h:204` — body ffi.cpp:313-315 is `ffi::free_`
        /// (void; infallible). The SAME symbol lookup.rs's `sys` declares;
        /// re-declared here because Stacked.hs declares its own import
        /// (Stacked.hs:53-54) and Rust externs may be declared per-module.
        pub fn glean_lookup_free(lookup: *mut c_void);
    }
}

// Marker breakdown (closed set — no open port TODOs, no source-defect
// reproductions, no performance deferrals, 1 inventory-bug-candidate):
//   * no open port TODOs: the one carried question — reconciling the
//     assumed CanDefine shape against unit 28 — was CLOSED in the same
//     batch: define.rs:355-361 confirms the exact assumed shape (see the
//     reconciliation note at the impl).
//   * no source-defect reproductions: no bug-class flag is needed. (The
//     C++ `Stacked<Define>` has its own source TODO about lookup order,
//     stacked.h:171-172 — C++ behaviour we CALL, not code re-implemented
//     on the Rust side.)
//   * no performance deferrals: the one allocation per view is the C++
//     factory's own `new Stacked<...>` — the source's allocation, kept;
//     no Rust copy.
//   * 1 inventory-bug-candidate: rows 73/15 name
//     `glean_stacked_lookup_free` as the lookup view's free function — no
//     such symbol exists (grep-verified). The port calls
//     `glean_lookup_free` per the Haskell (Stacked.hs:39 + :53-54) and
//     ffi.h:204 / ffi.cpp:313-315. Row-text defect only; no port
//     divergence (the generic-vs-Box deviation is documented in the
//     INVENTORY block as an implementation detail, per the unit brief).
// SAFETY count: 4 unsafe blocks, each carrying a `// SAFETY:` comment —
// the two Drop guards (glean_lookup_free / glean_stacked_define_free) and
// the two `_new` invokes in with_lookup/with_define. (The `pub mod sys`
// `unsafe extern` block is a declaration, not counted — family
// convention.)
// PORT STATUS: confidence=high todos=0
