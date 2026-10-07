// Ported from: glean/hs/Glean/RTS/Foreign/Bytecode.hs
// Manifest unit: rust/glean-core/src/rts/foreign/bytecode.rs
// (migration/manifest.tsv row 18, crate glean-core). RULE §4: one source
// file -> one target file; the manifest path is authoritative.
//
// ABI truth (read, not assumed): glean/rts/ffi.h:381-413 —
//   glean_subroutine_new    :381-392  (const char* error slot + `** sub` out)
//   glean_subroutine_free   :394      (void)
//   glean_subroutine_inspect:396-405  (void; 8 out-params + handle)
//   glean_subroutine_size   :407      (size_t)
//   glean_subroutine_literal:409-413  (void; 2 out-params)
// and their C++ bodies at glean/rts/ffi.cpp:824-892, plus the error
// channel kit at ~/glean-build/hsthrift/common/util/cpp/wrap.h:22-45 (the checkout's declared source dep; not under src.parser) (wrap /
// wrap_ / free_ — the same kit crate::ffi's header pins).
//
// INVENTORY (RULE §5) — rows governing this file, followed:
//   * inventory.tsv row 63 (line 63; duplicated as inventory.ffi.tsv row
//     13): "owned Subroutine (ForeignPtr); inspect = unsafePerformIO read;
//     size = unsafeDupablePerformIO | HIDDEN_EFFECT | cxx bridge
//     [SUPERSEDED by §1/A2: raw extern-C — 2026-10-07] for new/free
//     (owned Subroutine, Drop); inspect ported as PURE fn copying
//     insns/consts into owned Vec<u64> + literals into Vec<Vec<u8>>
//     (source already copies: peekElemOff + copyByteString); size = pure
//     method; reads are read-only so dropping unsafePerformIO is sound per
//     RULEBOOK §2 unsafePerformIO row". Followed EXACTLY: raw extern "C"
//     (the row itself records the A2 supersession inline — this is not a
//     divergence from the row), ForeignPtr-owned Subroutine with the
//     finalizer as Drop, inspect/size as pure fns with owned copies.
//   * inventory.tsv row 64 (line 64): size/inspect are
//     "unsafe(Dupable)PerformIO pure accessors over FFI Subroutine;
//     inspect is THE disassembly surface that verifies the two-pass
//     label-alloc identity check" — followed: both are plain pure Rust
//     fns; inspect's output is the surface RULE §0's bytecode identity
//     gate (A1) compares old-vs-new.
//   * inventory.sites.tsv rows 6-7 (unsafepure :27 import, :96 inspect)
//     and rows 98-102 (the five ffi-import sites) — the same sites rows
//     63/64 rule on; no additional mandate.
//
// INVENTORY-BUG-CANDIDATE row 64 (inventory.tsv line 64), family-scoped
// to include inventory.ffi.tsv row 13: both rows' target_translation
// still carry pre-A2 "cxx" wording ("cxx accessors returning owned
// data"; the ffi-shard copy says "cxx bridge for new/free"), while the
// sibling row for the SAME file and symbols — inventory.tsv row 63 —
// records the supersession inline ("[SUPERSEDED by §1/A2: raw extern-C —
// 2026-10-07]"). RULE §1 (FFI row, amendment A2: "raw extern "C" is the
// default for flat C-ABI symbols everywhere; cxx only when bridging C++
// classes/inheritance") rules these five flat `glean_*` functions over
// the opaque `SharedSubroutine*` (ffi.h:86 — no class method, no
// inheritance for a cxx bridge to wrap, which is A2's own evidence,
// stress D13). This file therefore declares raw extern "C" per row 63's
// superseded ruling + A2, not a cxx bridge. The rows' SUBSTANTIVE
// mandates are followed exactly (owned data copies out of inspect; pure
// accessors; identity-gate surface). One canonical family ruling —
// raw extern "C" over the opaque handle — is what builder.rs already
// requested for the builder family (its INVENTORY-BUG-CANDIDATE row 61).
//
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, module-level): inbound
// edges to this module are rows 398 (Schema/Types.hs), 542 (Query/
// Codegen.hs), 703 (Query/UserQuery.hs), 1229 (RTS/Bytecode/Code.hs),
// 1233 (RTS/Bytecode/Disassemble.hs), 1256 (RTS/Foreign/Inventory.hs),
// 1274 (RTS/Foreign/Query.hsc), 1287 (RTS/Foreign/Typecheck.hs), 1294
// (RTS/Traverse.hs), 1299 (RTS/Typecheck.hs); outbound edges are exactly
// row 1239 -> Glean/FFI.hs plus the external hsthrift Util/FFI kit
// (imported as `Util.FFI`, Bytecode.hs:30 — its Rust landing place is
// crate::ffi per that unit's row 122). This module imports NOTHING from
// the glean-bytecode crate: the Haskell import list (Bytecode.hs:17-30)
// contains no Glean.Bytecode.* — confirmed against edges.tsv, which
// pairs no row between this file and any glean/bytecode/ module (the
// DONE sibling rust/glean-bytecode/src/bytecode/types.rs's header
// records the same fact from its side). The prompt's "Register/Label/
// Literal/Ty" note is therefore resolved by the source: not consumed
// here; those types belong to unit 14's (Code.hs) and the def/ units'
// contracts, not this one.
//
// SIBLING-UNIT CONTRACTS (RULE §4; DONE crates'/units' real APIs read,
// never re-declared):
//   * crate::ffi (DONE, read): `invoke` (the pinned form
//     `pub unsafe fn invoke(call: impl FnOnce() -> *const c_char)
//     -> Result<(), FfiError>` — the extern call passed AS the closure),
//     `FfiError`, `Object`/`ForeignPtr` (the ForeignPtr-with-finalizer
//     machinery this module instantiates), `construct`, and
//     `copy_byte_string` (the copyByteString mapping — not re-implemented
//     here).
//   * unit 14 (glean/hs/Glean/RTS/Bytecode/Code.hs ->
//     rust/glean-core/src/rts/bytecode/code.rs; ASSUMED-IMPORT contract,
//     translated IN PARALLEL — this file was written WITHOUT reading it;
//     owner: rts/bytecode/code.rs): Code.hs:64 imports exactly
//     `Subroutine, subroutine` and calls `subroutine` at :309-316 with
//     (VS.fromListN (length code) code) / finalInputSize / outputs /
//     locals / (reverse csConstants) / (literals from csLiterals ::
//     HashMap ByteString Word64). The Rust seam it codes against:
//     `Subroutine<S>` (generic over its phantom kind marker — Code.hs:261
//     is `IO (Meta, Subroutine t)`, i.e. it is POLYMORPHIC in the marker,
//     so the Rust fn must be generic in S too) and
//     `subroutine::<S>(&[u64], u64, u64, u64, &[u64], &[&[u8]]) ->
//     Subroutine<S>`. If unit 14's literal marshalling produces
//     Vec<Vec<u8>>, the call site builds `&[&[u8]]` by reference — the
//     borrowed shape exists so no copy is forced.
//   * Foreign.Subroutine consumers that spell their OWN externs
//     (ffi.h:108-118 glean_inventory_new's `SharedSubroutine* const*
//     typecheckers/traversals` at :114-115 -> foreign/inventory.rs;
//     ffi.h:415-416 glean_invoke_typechecker's `const
//     SharedSubroutine* typechecker` -> foreign/typecheck.rs; ffi.h:171+
//     glean_query_execute_compiled / the Query.hsc eval-restart family,
//     `SharedSubroutine* sub`/`traverse` at :175/:177 ->
//     foreign/query.rs): `pub mod sys` below is `pub` for exactly that —
//     they declare `*const sys::SharedSubroutine` in their own externs
//     and obtain pointers ONLY through `Subroutine::with_ptr` (never
//     `as_ptr` — a ForeignPtr handle's raw pointer must not be retained
//     past the call, the contract crate::ffi::with already carries; the
//     Haskell siblings reach the same pointer exclusively via Util.FFI's
//     `with`, e.g. Typecheck.hs:41-43 `Ptr (Subroutine CompiledTypecheck)`).
//   * The phantom markers themselves (Typecheck.hs:26
//     `data CompiledTypecheck`, and Code.hs's polymorphic `t`) live with
//     their owners' units, not here — the Haskell module itself defines
//     none.

//! C++ RTS bytecode-subroutine handle (`SharedSubroutine`) — create,
//! inspect, size.
//!
//! Port notes: RULE §3 — `unsafe` appears only inside this glean-core FFI
//! wrapper, where the Haskell was already FFI; every block carries a
//! SAFETY comment. Error-recovery classification per RULE §2
//! (error-recovery rule), told as one story with the C++ bodies and the
//! wrap.h kit pinned in crate::ffi: the ONLY symbol here with an error
//! slot is `glean_subroutine_new`, and its body (ffi.cpp:836-852) has no
//! precondition checks at all — every allocation (two std::vector<uint64_t>
//! ranges, the literals vector's strings, make_shared) is
//! allocation-class, so the site is an ALLOCATION-GUARD -> abort (see
//! `allocation_guard_abort`); `inspect`/`size`/`literal` return void/
//! size_t with bodies of pure reads (ffi.cpp:858-892) — no channel, no
//! guard; `free`'s body routes through ffi::free_ (wrap.h:42-45), whose
//! wrap_ swallows every exception (wrap.h:34-40), so Drop is infallible.

use std::marker::PhantomData;
use std::os::raw::c_void;
use std::ptr;

use crate::ffi::{construct, copy_byte_string, invoke, Destroy, FfiError, ForeignPtr, Object};

/// Raw C-ABI declarations for the glean_subroutine_* group
/// (glean/rts/ffi.h:381-413).
///
/// RULE §1 (FFI row, A2): raw `extern "C"` for flat C-ABI symbols — see
/// the INVENTORY-BUG-CANDIDATE note at the top of this file for the two
/// rows whose pre-A2 "cxx" wording this supersedes.
///
/// Signatures mirror ffi.h EXACTLY. One divergence from the HASKELL
/// foreign import (not from the header), noted per RULE §0 (ABI truth is
/// ffi.h, not the .hs): Bytecode.hs:131-141 declares inspect's
/// inputs/outputs/locals out-params as `Ptr Word64` where the header says
/// `size_t*` (ffi.h:400-402). ABI-compatible on every LP64 target GHC
/// builds for (size_t == 8 == Word64); the port uses the header's
/// `*mut usize` and widens to u64 once, at the field write.
pub mod sys {
    use std::os::raw::{c_char, c_void};

    /// Opaque `SharedSubroutine` (ffi.h:86 `typedef struct
    /// SharedSubroutine SharedSubroutine;` — defined in
    /// glean/rts/subroutine.h as holding a std::shared_ptr<const
    /// Subroutine>). Never constructed, dereferenced, or freed on the
    /// Rust side except through the functions below.
    #[repr(C)]
    pub struct SharedSubroutine {
        _private: [u8; 0],
    }

    unsafe extern "C" {
        // ffi.h:381-392 — `SharedSubroutine** sub` out-param; error slot
        // is allocation-guard (body ffi.cpp:836-852: vectors, strings,
        // make_shared — no precondition checks exist to fail).
        pub fn glean_subroutine_new(
            code: *const u64,
            code_size: usize,
            inputs: usize,
            outputs: usize,
            locals: usize,
            constants_ptr: *const u64,
            constants_size: usize,
            literal_ptrs: *const *const c_void,
            literal_sizes: *const usize,
            literal_count: usize,
            sub: *mut *mut SharedSubroutine,
        ) -> *const c_char;
        // ffi.h:394 — void; body ffi.cpp:854-856 is ffi::free_ (wrap_
        // swallows all exceptions, wrap.h:34-45) — infallible.
        pub fn glean_subroutine_free(sub: *mut SharedSubroutine);
        // ffi.h:396-405 — void, 8 out-params + handle; body ffi.cpp:858-876 is
        // pure reads off the shared Subroutine, no error channel.
        pub fn glean_subroutine_inspect(
            sub: *mut SharedSubroutine,
            code: *mut *const u64,
            code_size: *mut usize,
            inputs: *mut usize,
            outputs: *mut usize,
            locals: *mut usize,
            constants: *mut *const u64,
            constants_size: *mut usize,
            lit_count: *mut usize,
        );
        // ffi.h:407 — size_t; body ffi.cpp:878-880 calls
        // sub->value->size() — pure, infallible.
        pub fn glean_subroutine_size(sub: *mut SharedSubroutine) -> usize;
        // ffi.h:409-413 — void, 2 out-params; body ffi.cpp:882-892 with
        // an explicit in-range test (:887): out-of-range index writes
        // (nullptr, 0). No error channel.
        pub fn glean_subroutine_literal(
            sub: *mut SharedSubroutine,
            index: usize,
            ptr: *mut *const c_void,
            size: *mut usize,
        );
    }
}

/// `data SubroutineCode = SubroutineCode {..}` (Bytecode.hs:32-50) — the
/// owned snapshot returned by [`inspect`]. Source field order kept
/// (subInsns, subInputs, subOutputs, subLocals, subConstants,
/// subLiterals); names snake_cased per house style.
///
/// No deriving clause in the source — no derives here (RULE §0).
pub struct SubroutineCode {
    /// Instructions. (Source comment: "Instructions".)
    pub sub_insns: Vec<u64>,
    /// Number of inputs. (Source comment: "Number of inputs".)
    pub sub_inputs: u64,
    /// Number of outputs. (Source comment: "Number of outputs".)
    pub sub_outputs: u64,
    /// Number of local registers. (Source comment: "Number of local
    /// registers".)
    pub sub_locals: u64,
    /// Constants (copied into local registers at beginning). (Source
    /// comment carried verbatim in substance.)
    pub sub_constants: Vec<u64>,
    /// Literals. RULE §1 (strings/bytes row): `Vec<u8>` for ByteString.
    pub sub_literals: Vec<Vec<u8>>,
}

/// `newtype Subroutine s = Subroutine (ForeignPtr (Subroutine s))`
/// (Bytecode.hs:55).
///
/// The `S` parameter mirrors the source's phantom: "The s parameter
/// describes the kind of subroutine this is (e.g. CompiledTypecheck,
/// CompiledQuery), to avoid us mixing up the two" (Bytecode.hs:52-54).
/// It is `PhantomData<fn() -> S>` so the marker imposes no auto-trait
/// bound of its own — `Subroutine<S>` is Send+Sync exactly as
/// `ForeignPtr` is (crate::ffi's unsafe impls, matching GHC ForeignPtr
/// shareability); a bare `PhantomData<S>` would wrongly inherit
/// whatever S is.
///
/// Ownership/sharing (inventory row 63: "owned Subroutine
/// (ForeignPtr)"): the handle wraps `crate::ffi::ForeignPtr` — cheap to
/// clone, finalizer runs at most once, `release` available — the exact
/// GHC ForeignPtr semantics the source relies on when it stores one
/// subroutine in an Inventory and hands the same handle to every later
/// invoke (edges into Inventory.hs/Query.hsc above). This module's
/// ported `instance Object` (Bytecode.hs:57-60) is the `unsafe impl
/// Object` below.
pub struct Subroutine<S> {
    fp: ForeignPtr<Subroutine<S>>,
    marker: PhantomData<fn() -> S>,
}

// `instance Object (Subroutine s) where wrap = Subroutine; unwrap
// (Subroutine p) = p; destroy = glean_subroutine_free`
// (Bytecode.hs:57-60).
//
// SAFETY (trait contract, crate::ffi::Object): destroy() is the correct C
// finalizer for pointers wrap() adopts — glean_subroutine_free over the
// SharedSubroutine* the constructor's out-param produced — and wrap/
// unwrap round-trip over the same clone-shared ForeignPtr (unwrap clones
// the Arc, GHC's ForeignPtr sharing; nothing re-wraps a foreign pointer
// behind the type's back: the ONLY door onto a live handle is
// crate::ffi::create inside `subroutine` below, mirroring the Haskell
// where `construct` was the only newForeignPtr site).
unsafe impl<S> Object for Subroutine<S> {
    /// `destroy = glean_subroutine_free` — the FunPtr finalizer
    /// (Bytecode.hs:128-129 `foreign import ccall unsafe
    /// "&glean_subroutine_free"`) becomes a Rust fn pointer of
    /// `Destroy<Subroutine<S>>` shape (crate::ffi::Destroy's documented
    /// convention). The generic shim exists because Destroy is typed over
    /// *mut Subroutine<S> while the C symbol takes *mut
    /// SharedSubroutine; the cast is the pointer-identity the Haskell got
    /// for free from `Ptr (Subroutine s)` being the C pointer type — no
    /// Subroutine<S> Rust value ever lives at that address.
    fn destroy() -> Destroy<Self> {
        unsafe extern "C" fn glean_subroutine_free_finalizer<S>(p: *mut Subroutine<S>) {
            // SAFETY: p is the SharedSubroutine* adopted at construction
            // (the only pointer ever wrapped); ForeignPtr's cell
            // guarantees at-most-once execution, and the C++ body
            // (ffi.cpp:854-856, ffi::free_ -> wrap_) cannot throw across.
            unsafe { sys::glean_subroutine_free(p.cast()) };
        }
        glean_subroutine_free_finalizer::<S>
    }

    /// `wrap = Subroutine` (Bytecode.hs:58).
    fn wrap(fp: ForeignPtr<Self>) -> Self {
        Subroutine {
            fp,
            marker: PhantomData,
        }
    }

    /// `unwrap (Subroutine p) = p` (Bytecode.hs:59) — the shared pointer.
    fn unwrap(&self) -> ForeignPtr<Self> {
        self.fp.clone()
    }
}

impl<S> Subroutine<S> {
    /// Util.FFI `with sub $ \p_sub -> ...` — every sibling consumer's
    /// ONLY pointer path (Typecheck.hs:41-43, Inventory.hs, Query.hsc
    /// all go through `with`; none retains the pointer).
    ///
    /// RULE §2 (A7 row: "a safe signature must not launder an unbounded
    /// C++ write"): this stays an `unsafe fn` because the contract is
    /// retention-bounded, exactly as crate::ffi::with's is — `f` gets the
    /// raw pointer valid only for the call; a safe signature would hide
    /// that `f` could stash it past the ForeignPtr's lifetime.
    ///
    /// SAFETY: `f` must not retain or free the pointer beyond the call;
    /// `self` must not be concurrently `release()`d (crate::ffi's
    /// with_raw contract, which this delegates to).
    pub unsafe fn with_ptr<R>(&self, f: impl FnOnce(*mut sys::SharedSubroutine) -> R) -> R {
        // SAFETY: delegated from this fn's own contract above; the inner
        // Arc clone keeps the finalizer duty alive for the whole call
        // (that is with_raw's own doing).
        unsafe {
            self.fp.with_raw(|p: *mut Subroutine<S>| f(p.cast()))
        }
    }
}

/// RULE §2 (error-recovery rule): "allocation-failure ≠ structural
/// error. Every FFI null return in the inventory records which kind:
/// allocation-guard (abort) or precondition-guard (record + partial
/// result)." `glean_subroutine_new`'s body (ffi.cpp:836-852) contains no
/// precondition check that can fail — its only failure modes are
/// bad_alloc-class throws (two vector ranges, the literals' strings,
/// make_shared), which wrap.h converts into a returned message
/// (wrap.h:26-28; strdup-failure into the static outOfMemory, :28) — so
/// the Err arm is allocation-class and the row-sanctioned response is
/// ABORT. The Haskell shape at this site was `construct $ invoke $ ...`:
/// the FFIError escapes as an exception and kills the calling path
/// uncaught (no `catch` exists anywhere on the compile/typecheck routes
/// that build subroutines — grep over RTS/Typecheck.hs,
/// RTS/Bytecode/Code.hs, Query/Codegen.hs), so abort reproduces the
/// source's observable outcome. abort() rather than panic! so no
/// unwinding runs first — same shape as the DONE sibling
/// rts/builder.rs's allocation_guard_abort (a house-pattern match, not
/// provenance: the ruling is RULE §2 + the body just cited).
fn allocation_guard_abort(e: FfiError) -> ! {
    eprintln!("glean: C++ subroutine allocation failure: {}", e);
    std::process::abort()
}

/// Create a subroutine (Bytecode.hs:62-89):
///
/// * subroutine
/// *   :: V.Vector Word64   -- ^ instructions
/// *   -> Word64 -> Word64 -> Word64   -- inputs / outputs / locals
/// *   -> [Word64]          -- ^ constants
/// *   -> [ByteString]      -- ^ literals
/// *   -> IO (Subroutine s)
///
/// Marshalling (Bytecode.hs:71-89): `V.unsafeWith code` -> the slice's
/// own (ptr, len) (a &[u64] borrows exactly what unsafeWith pinned);
/// `withArrayLen consts` -> consts.as_ptr()/len(); `withMany (\s ->
/// unsafeWithBytes s . curry) lits` -> all literal pointers held open
/// SIMULTANEOUSLY and collected into two parallel arrays. crate::ffi's
/// `with_many` header cites this very site (Bytecode.hs:74) for why its
/// stricter signature cannot carry the curry'd transliteration; this
/// port sidesteps it the way that note prescribes — collecting from
/// storage that outlives the call: the `&[&[u8]]` parameter holds every
/// borrow for the whole fn body, and the two Vecs below are plain
/// arrays of the already-valid pointers. The pointers are non-null even
/// for empty literals (&[u8] borrows, never dangling), matching
/// unsafeWithBytes' ByteString guarantee.
///
/// Guard kind: ALLOCATION-GUARD -> abort (see `allocation_guard_abort`);
/// on the (impossible-here) Err the out-param is still null because the
/// C++ body writes `*sub` only after every allocation succeeds
/// (ffi.cpp:845).
///
/// Generic in `S` because the source is: Code.hs:261 calls it
/// polymorphically (`IO (Meta, Subroutine t)`).
pub fn subroutine<S>(
    code: &[u64],
    inputs: u64,
    outputs: u64,
    locals: u64,
    consts: &[u64],
    lits: &[&[u8]],
) -> Subroutine<S> {
    let lit_ptrs: Vec<*const c_void> = lits.iter().map(|s| s.as_ptr().cast()).collect();
    let lit_sizes: Vec<usize> = lits.iter().map(|s| s.len()).collect();
    let mut sub: *mut sys::SharedSubroutine = ptr::null_mut();
    // RULE §2 (FFI `CSize` returns row, A5 — protocol noted): all length
    // conversions here are unsigned->unsigned. inputs/outputs/locals are
    // the source's Word64 narrowed to the header's size_t via `as` — the
    // exact `fromIntegral ... :: CSize` truncation the Haskell performed
    // (Bytecode.hs:82-84), total on LP64, truncating identically on any
    // narrower size_t. No signed value is ever `as`-cast.
    let result: Result<Subroutine<S>, FfiError> = unsafe {
        // SAFETY: `construct`/`create` adopt exactly the live
        // uniquely-owned handle the C side produced on success — the
        // mk closure returns the out-param's value only after invoke
        // reports success, and null-init means a failed call cannot
        // smuggle a dangling handle into the ForeignPtr.
        construct(|| {
            // SAFETY: every pointer argument is valid for the duration
            // of the call — code/consts borrowed slices,
            // lit_ptrs/lit_sizes Vecs of pointers into the lits borrows
            // (all still alive; the Vecs outlive this closure), and
            // `sub` is an initialised stack out-param. The extern's
            // parameter order mirrors ffi.h:381-392 exactly. invoke
            // takes the extern call AS its closure and owns the error
            // string's release (crate::ffi::invoke contract).
            unsafe {
                invoke(|| {
                    sys::glean_subroutine_new(
                        code.as_ptr(),
                        code.len(),
                        inputs as usize,
                        outputs as usize,
                        locals as usize,
                        consts.as_ptr(),
                        consts.len(),
                        lit_ptrs.as_ptr(),
                        lit_sizes.as_ptr(),
                        lits.len(),
                        &mut sub,
                    )
                })?
            };
            Ok(sub.cast())
        })
    };
    match result {
        Ok(sub) => sub,
        // Allocation-class by the body's construction; abort per RULE §2
        // and the classification above.
        Err(e) => allocation_guard_abort(e),
    }
}

/// `size :: Subroutine s -> Int` (Bytecode.hs:91-93):
///
/// * size sub = unsafeDupablePerformIO $ with sub $ \p_sub ->
/// *   fromIntegral <$> glean_subroutine_size p_sub
///
/// Inventory row 63: "size = pure method; reads are read-only so
/// dropping unsafePerformIO is sound per RULEBOOK §2 unsafePerformIO
/// row" — the unsafeDupablePerformIO hid an FFI read behind a pure
/// signature; the port is an ordinary pure fn performing that one read.
/// (`unsafeDupablePerformIO` vs `unsafePerformIO`: dupable permits
/// concurrent duplicate execution; the read is idempotent over an
/// immutable handle, so the distinction is unobservable — noted, RULE
/// §0.) CSize -> `usize` directly (A5: unsigned both sides, no cast at
/// all); the source's `fromIntegral` to Int was the same no-op shape on
/// LP64.
pub fn size<S>(sub: &Subroutine<S>) -> usize {
    // SAFETY: the pointer is a live SharedSubroutine* (with_ptr's
    // contract) and the C++ body (ffi.cpp:878-880) is a pure size_t
    // read — no error channel exists to check.
    unsafe { sub.with_ptr(|p| sys::glean_subroutine_size(p)) }
}

/// `inspect :: Subroutine s -> SubroutineCode` (Bytecode.hs:95-113) —
/// THE disassembly surface feeding the RULE §0 (A1) bytecode identity
/// gate (inventory row 64). Pure in the source via unsafePerformIO;
/// ported as a pure fn with owned copies, per row 63: "inspect ported as
/// PURE fn copying insns/consts into owned Vec<u64> + literals into
/// Vec<Vec<u8>> (source already copies: peekElemOff + copyByteString)".
pub fn inspect<S>(sub: &Subroutine<S>) -> SubroutineCode {
    let mut insns_ptr: *const u64 = ptr::null();
    let mut insns_size: usize = 0;
    let mut inputs: usize = 0;
    let mut outputs: usize = 0;
    let mut locals: usize = 0;
    let mut consts_ptr: *const u64 = ptr::null();
    let mut consts_size: usize = 0;
    let mut lit_count: usize = 0;
    // SAFETY: all nine out-params are initialised stack locals valid for
    // the duration; the pointer is a live SharedSubroutine* (with_ptr);
    // the C++ body (ffi.cpp:858-876) only READS the shared Subroutine
    // and writes the out-params — no error channel. The source's `invoke
    // $` around this void-returning call (Bytecode.hs:100) was
    // Util.FFI's vacuous IO () instance; there is no const char* slot to
    // route through crate::ffi::invoke.
    unsafe {
        sub.with_ptr(|p| {
            sys::glean_subroutine_inspect(
                p,
                &mut insns_ptr,
                &mut insns_size,
                &mut inputs,
                &mut outputs,
                &mut locals,
                &mut consts_ptr,
                &mut consts_size,
                &mut lit_count,
            )
        });
    }
    // V.generateM (fromIntegral insns_size) $ peekElemOff insns_ptr
    // (Bytecode.hs:101-102) — element-for-element read into an owned
    // vector; the guarded helper below is the same shape.
    //
    // SAFETY: these are inspect's
    // out-params, read off the still-live handle immediately above —
    // non-null and readable for the full count whenever the count is
    // non-zero, which is copy_u64s' caller contract; the with_ptr scope
    // above has closed, but the handle is borrowed for this whole fn
    // (`sub: &Subroutine<S>`), so the C++ vectors it points into cannot
    // have been freed.
    let insns = unsafe { copy_u64s(insns_ptr, insns_size) };
    let consts = unsafe { copy_u64s(consts_ptr, consts_size) };
    // forM [1 .. lit_count] $ \i -> do { (lit_ptr, lit_size) <- invoke $
    // glean_subroutine_literal p_sub (i-1); copyByteString lit_ptr
    // lit_size } (Bytecode.hs:103-105) — the [1..n] then (i-1) dance is
    // 0-based iteration; lit_count == 0 gives no iterations on both
    // sides.
    let mut lits = Vec::with_capacity(lit_count);
    for i in 0..lit_count {
        let mut lit_ptr: *const c_void = ptr::null();
        let mut lit_size: usize = 0;
        // SAFETY: the pointer is live (with_ptr), `i < lit_count` was
        // taken from inspect's own lit_count on the same immutable
        // handle, and lit_ptr/lit_size are initialised out-params; the
        // body (ffi.cpp:882-892) writes either (literals[i].data(),
        // size) — in range — or (nullptr, 0) out of range, which this
        // loop cannot request. No error channel (void), so the source's
        // `invoke $` was again the vacuous instance.
        unsafe {
            sub.with_ptr(|p| {
                sys::glean_subroutine_literal(p, i, &mut lit_ptr, &mut lit_size)
            });
        }
        // copyByteString lit_ptr lit_size (Bytecode.hs:105) via the
        // crate-canonical mapping (crate::ffi::copy_byte_string — not
        // re-implemented, RULE §5 one-mapping-per-site). The size == 0
        // guard is REQUIRED for soundness where the Haskell needed none:
        // slice::from_raw_parts demands a non-null pointer even at len
        // 0, whereas BS.packCStringLen (nullptr, 0) is simply the empty
        // ByteString; observable behaviour is identical (empty Vec) on
        // every path, including the body's unreachable out-of-range
        // (nullptr, 0) write.
        lits.push(if lit_size == 0 {
            Vec::new()
        } else {
            // SAFETY: in range with size > 0, lit_ptr is
            // literals[i].data() — non-null and valid for reads of
            // lit_size bytes for as long as the handle lives (with_ptr
            // keeps it alive through the copy). copy_byte_string's
            // caller contract is thereby met.
            unsafe { copy_byte_string(lit_ptr, lit_size) }
        });
    }
    // The usize out-params widen to the record's Word64 fields
    // (Bytecode.hs:37-43) — `as u64`, total on LP64 (the header's
    // size_t); identical to the source's peek on Ptr Word64.
    SubroutineCode {
        sub_insns: insns,
        sub_inputs: inputs as u64,
        sub_outputs: outputs as u64,
        sub_locals: locals as u64,
        sub_constants: consts,
        sub_literals: lits,
    }
}

/// The `V.generateM n (peekElemOff p)` twin for the two u64 arrays
/// inspect hands back (Bytecode.hs:101-102).
///
/// The `n == 0` early-out is a soundness guard Rust needs and the
/// Haskell did not: an empty std::vector's `data()` may legally be null
/// (ffi.cpp writes `sub->value->code.data()` unconditionally,
/// ffi.cpp:868), and `slice::from_raw_parts(null, 0)` is UB — while
/// `V.generateM 0` never touches the pointer and yields the empty
/// vector. Same empty result, no dereference.
///
/// SAFETY: when `n > 0`, `p` must be non-null and valid for reads of `n`
/// u64 elements (inspect's out-params on a live handle: pointers into
/// the shared Subroutine's own vectors, kept alive by the caller's
/// with_ptr).
unsafe fn copy_u64s(p: *const u64, n: usize) -> Vec<u64> {
    if n == 0 {
        Vec::new()
    } else {
        // SAFETY: caller contract above; elements only read.
        unsafe { std::slice::from_raw_parts(p, n) }.to_vec()
    }
}

// Marker breakdown (closed set — 0 TODO(port), 0 BUG(port), 0
// PERF(port), 1 INVENTORY-BUG-CANDIDATE):
//   * The one open-looking question — the two inventory rows' pre-A2
//     "cxx" wording — is resolved IN the inventory itself (row 63's
//     inline supersession) plus RULE §1/A2, and flagged above as
//     INVENTORY-BUG-CANDIDATE row 64 (family: inventory.ffi.tsv row 13)
//     per RULE §5 rather than silently interpreted; nothing remains
//     open, so there is no TODO(port) to count.
//   * No BUG(port): the Haskell's Ptr Word64 typing of inspect's
//     size_t* out-params is ABI-identical on every supported (LP64)
//     target — no repro exists, so it is documented at the extern, not
//     flagged.
//   * No PERF(port): every copy here (insns/consts/literals) is
//     source-mandated (row 63: "source already copies"), and the two
//     parallel literal arrays are the minimum marshalling the C ABI's
//     array-of-pointers contract requires.
// SAFETY marker count: 11 unsafe blocks, each carrying a `// SAFETY:`
// comment — finalizer 1, with_ptr 1, subroutine 2 (construct + invoke),
// size 1, inspect 5 (extern + two copy_u64s calls + literal extern +
// copy_byte_string), copy_u64s' own slice read 1. (The `unsafe impl
// Object`, `unsafe fn with_ptr` and `unsafe fn copy_u64s` additionally
// carry doc-level SAFETY contracts, per crate::ffi's house style.)
// PORT STATUS: confidence=high todos=0
