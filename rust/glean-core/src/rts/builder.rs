// Ported from: glean/hs/Glean/RTS/Builder.hs
// Manifest unit: rust/glean-core/src/rts/builder.rs (manifest.tsv line 10,
// crate glean-core). One source file -> one target file (RULE §4).
//
// ABI truth: glean/rts/ffi.h:320-327 (the glean_builder_* group) and
// glean/rts/ffi.cpp:659-678 (their bodies — read before classifying any
// guard below).
//
// What this module is: the thinnest possible owned handle over the C++ RTS
// byte-writer `facebook::glean::rts::binary::Output` (glean/rts/binary.h:281,
// "A binary buffer which can be written to"). Fact keys/values and query
// prefixes are serialised into it by the push symbols owned by other units;
// per inventory row 61: "Builder consumed by batch construction
// (RTS.hs buildFacts)". The Haskell module is 59 lines of bracket plus five
// foreign imports; the Rust keeps exactly that surface:
//
//   Haskell export      Rust
//   ------------------  ---------------------------------------------
//   Builder(..)         pub struct Builder (wrapping constructor private:
//                       handles enter only via new_builder; the by-value
//                       marshalling seam the Storable newtype gave the FFI
//                       for free becomes Builder::as_ptr for sibling units)
//   withBuilder         with_builder
//   finishBuilder       finish_builder
//   sizeOfBuilder       size_of_builder
//   newBuilder          new_builder
//   freeBuilder         free_builder
//   resetBuilder        reset_builder
//
// INVENTORY-BUG-CANDIDATE row 61 (inventory.tsv line 61, the glean_builder_*
// group row; row id = line number — the convention v0.3 itself uses for
// "rows 120/121", which sit at lines 120/121): target_translation reads
// "cxx bridge", but RULEBOOK §1 (FFI row, amendment A2 — "cxx scope →
// RESOLVED by A2 (extern "C" default, cxx for class bridges)", applied in
// v0.2, after this row was written) makes raw `extern "C"` the default for
// flat C-ABI symbols everywhere, reserving cxx for bridging C++
// classes/inheritance. All five glean_builder_* symbols are flat extern "C"
// functions over an opaque `Output*` (glean/rts/ffi.h:320-327 — ffi.h:30
// `typedef struct Output Output;`): no class method and no inheritance exist
// for a cxx bridge to wrap, which is A2's own evidence ("flat glean_*
// symbols gain nothing from cxx", stress D13). This file therefore declares
// raw externs. SCOPE of the flag — FAMILY-wide, not row-61-only: rows 38
// (Codegen.hs row: "keep the cxx Builder (RTS/Builder.hs)"), 60 (RTS.hs row:
// "push methods on the cxx Builder") and 79 (Set.hs row: "conservative: cxx
// bridge") also mandate cxx for this same builder handle, so one canonical
// ruling is needed across the family — A2's raw extern "C" over the opaque
// handle, no cxx — not a per-row reading of row 61 alone. The rows'
// substantive rulings are followed EXACTLY: abort on OOM per row 61's
// conservative ruling (see allocation_guard_abort), and
// precondition-guards explicit — of which this module's symbols have none,
// verified against the C++ bodies at glean/rts/ffi.cpp:659-678: the only
// exceptions those bodies can raise are allocation failures (`new Output`,
// ffi::clone_bytes' malloc), and per the channel contract in ffi.rs those
// cross the boundary as returned messages (wrap.h:21-32 catches every
// std::exception, bad_alloc included) — so every Err reaching this module
// is allocation-class, and inventory row 61's ruling ("abort on OOM") is
// applied to it via allocation_guard_abort. The precondition-guarded symbols of the builder family (the
// push_value_* arity/type checks) live at sites owned by
// rust/glean-core/src/rts.rs, not here.
//
// SIBLING-UNIT CONTRACT: `sys` is `pub` and `Builder::as_ptr` is the seam,
// because three sibling units declare their own raw externs whose C
// signatures carry the builder handle by value — marshalling the Haskell got
// for free from `newtype Builder = Builder (Ptr Builder) deriving
// (Storable)`:
//   - rust/glean-core/src/rts.rs — glean_push_value_* / glean_push_fact
//     (RTS.hs:268-283, :306-307; inventory RTS.hs row: "push methods on the
//     cxx Builder (see inventory.tsv Builder row)", i.e. on THIS handle)
//   - rust/glean-core/src/rts/set.rs — glean_rtsset_insert / *_build
//     (Set.hs:74-77, :92-97)
//   - rust/glean-core/src/rts/foreign/define.rs — glean_define_fact takes
//     `Builder -> CSize` (Define.hs:116-122; the adjacent foreign import,
//     Define.hs:124-135, is define_batch — not this symbol)
// Those units must obtain the pointer via as_ptr and must not dereference or
// free it; the handle's lifetime is scoped by this module's ownership.

//! C++ RTS builder handle (`binary::Output`) — create, size, finish, reset.
//!
//! Port notes: RULE §3 — `unsafe` appears only inside this glean-core FFI
//! wrapper, where the Haskell was already FFI; every block carries a SAFETY
//! comment. Error-recovery classification per RULE §2 (error-recovery rule),
//! aligned with ffi.rs's channel contract so the two units tell ONE story:
//! allocation failures cross the boundary as returned messages (wrap.h:21-32
//! catches every std::exception, bad_alloc included; the C++ bodies at
//! ffi.cpp:659-678 can only fail to allocate, so a crossing Err is
//! allocation-class by construction), and this module applies inventory row
//! 61's ruling ("abort on OOM") to them — see allocation_guard_abort.

use std::os::raw::c_void;
use std::ptr;

// ASSUMED-IMPORT (RULE §4 draft stub policy / assumed-imports) — RESOLVED
// against the shipped sibling: hsthrift's `Util.FFI` is external to this
// checkout (inventory.tsv line 122, Util/FFI row: "the invoke/call error
// protocol itself is hsthrift Util/FFI (external)"). Its Rust landing place
// is the sibling unit glean/hs/Glean/FFI.hs -> rust/glean-core/src/ffi.rs
// (manifest.tsv line 9, unit 8 — SHIPPED and read; this file codes against
// its actual surface, not the earlier assumption). Shipped contract, as
// consumed here:
//   pub unsafe fn invoke(call: impl FnOnce() -> *const c_char)
//     -> Result<(), FfiError>
//     - the extern call itself is passed AS the `call` closure and runs
//       inside invoke
//     - call() == null -> Ok(())   (C++ success convention: ffi::wrap
//                                  returns nullptr, glean/rts/ffi.cpp)
//     - call() != null -> take_error_message reads the NUL-terminated
//       message (the C++ side strdup'd e.what()) and converts with
//       to_string_lossy — i.e. LOSSY at the boundary; RULE §1's no-lossy
//       clause (strings/bytes row: "no lossy conversions at boundaries")
//       is ffi.rs's TODO to resolve, not this file's claim — then frees
//       the buffer with libc::free and returns Err.
// The Haskell `invoke` is used at three arities in the source module alone
// (Builder.hs:36 two out-params, :43 one out-param, :48 none), so the
// external helper is generic over Storable out-params that it allocas and
// peeks; in Rust the arities collapse to explicit stack locals plus this one
// check at each call site. This unit never re-implements the CString
// protocol itself (one canonical mapping per RULE §5 — the Util/FFI row
// counts 17 import sites).
use crate::ffi::{invoke, FfiError};

/// Raw C-ABI declarations for the glean_builder_* group (glean/rts/ffi.h).
///
/// RULE §1 (FFI row, A2): raw `extern "C"` is the default for flat C-ABI
/// symbols; `cxx` only when bridging C++ classes/inheritance. See the
/// INVENTORY-BUG-CANDIDATE note at the top of this file for why this row's
/// "cxx bridge" wording is not followed verbatim.
pub mod sys {
    use std::os::raw::{c_char, c_void};

    /// Opaque `facebook::glean::rts::binary::Output` (ffi.h:30) — the C++
    /// RTS's growable byte buffer. Never constructed, dereferenced, or
    /// freed on the Rust side except through the functions below.
    #[repr(C)]
    pub struct Output {
        _private: [u8; 0],
    }

    extern "C" {
        // ffi.h:320 — `*builder` out-param; error string is
        // allocation-guard (body: `new Output`, ffi.cpp:659-661).
        pub fn glean_new_builder(builder: *mut *mut Output) -> *const c_char;
        // ffi.h:321 — void, infallible (plain delete, ffi.cpp:663-665).
        pub fn glean_free_builder(builder: *mut Output);
        // ffi.h:323 — size_t, infallible (NOT wrapped in ffi::wrap on the
        // C++ side, ffi.cpp:667-669).
        pub fn glean_builder_size(builder: *mut Output) -> usize;
        // ffi.h:325 — `data`/`size` out-params; on success ownership of the
        // malloc'd clone transfers to the caller
        // (ffi::clone_bytes(builder->bytes()).release_to, ffi.cpp:671-674).
        pub fn glean_finish_builder(
            builder: *mut Output,
            data: *mut *mut c_void,
            size: *mut usize,
        ) -> *const c_char;
        // ffi.h:327 — move-assigns a fresh binary::Output (ffi.cpp:676-678);
        // error slot exists but the body is allocation-guard in principle
        // only (vector move-assign does not throw).
        pub fn glean_reset_builder(builder: *mut Output) -> *const c_char;
    }
}

/// Glean.RTS.Builder.Builder (Builder.hs:28-29):
/// `newtype Builder = Builder (Ptr Builder) deriving (Storable)` — a handle
/// to the C++ `binary::Output` buffer.
///
/// Ownership: allocated by `new_builder` (glean_new_builder) and released
/// exactly once by `Drop` (== `free_builder` / glean_free_builder) — the
/// Drop-equals-free shape the inventory prescribes for every owned RTS FFI
/// handle (FactSet / Subroutine / LookupCache rows). The handle is not
/// `Copy`, `Clone`, `Send` or `Sync`: the C++ buffer is unsynchronised
/// mutable state, and the source only ever moves a Builder within its
/// `withBuilder` bracket on one thread (conservative representation; no
/// call site in the tree moves one across threads).
///
/// `#[repr(transparent)]` keeps the by-value C marshalling story of the
/// Storable newtype available: the Rust value IS the `Output*`.
#[repr(transparent)]
pub struct Builder(*mut sys::Output);

impl Builder {
    /// Raw `Output*` accessor — the marshalling seam described in the
    /// SIBLING-UNIT CONTRACT at the top of this file. Not in the Haskell
    /// export list; added because raw externs spell the by-value handle
    /// `*mut sys::Output` and the unwrapping must live once, here, instead
    /// of per-site. The pointer is borrowed: it stays owned by this Builder
    /// and the buffer it points to remains valid until drop.
    pub fn as_ptr(&self) -> *mut sys::Output {
        self.0
    }
}

impl Drop for Builder {
    fn drop(&mut self) {
        // SAFETY: self.0 is non-null and originates from a successful
        // glean_new_builder (the only way a Builder is constructed — the
        // field is private and no other constructor exists), so it is a
        // live handle the C++ side owns. Builder is neither Copy nor Clone,
        // so this runs exactly once per handle and cannot double-free.
        unsafe { sys::glean_free_builder(self.0) };
    }
}

/// RULE §2 (error-recovery rule): "allocation-failure ≠ structural error.
/// Every FFI null return in the inventory records which kind:
/// allocation-guard (abort) or precondition-guard (record + partial
/// result)." Guard kind here, told as ONE story with ffi.rs's channel
/// contract: allocation failures cross the boundary as returned messages
/// (wrap.h:21-32; the C++ bodies at ffi.cpp:659-678 can only fail to
/// allocate), so any Err reaching this helper is allocation-class — and
/// inventory row 61 rules "abort on OOM (source dies via RTS OOM)"
/// for this family, so the port aborts instead of returning a catchable
/// error, mirroring an unrecoverable RTS death. The source emits no
/// diagnostic before dying; we keep the C++ message on stderr because it is
/// free at this point. abort() rather than panic! so that no unwinding
/// runs first.
fn allocation_guard_abort(e: FfiError) -> ! {
    eprintln!("glean: C++ builder allocation failure: {}", e);
    std::process::abort()
}

/// Haskell (Builder.hs:31-32):
/// `withBuilder = bracket (liftIO newBuilder) (liftIO . freeBuilder)`.
///
/// The `MonadMask` bracket guarantees free on exception; here `Builder`'s
/// `Drop` performs the release on every exit path, including panic-unwind
/// out of `f` — the same acquire/release/exception-safety shape. (The
/// Haskell's `forall m. (MonadMask m, MonadIO m)` generality over
/// transformer stacks corresponds to the generic return value `R`.)
pub fn with_builder<R>(f: impl FnOnce(&mut Builder) -> R) -> R {
    let mut builder = new_builder();
    f(&mut builder)
}

/// Haskell (Builder.hs:42-43): `newBuilder = FFI.invoke glean_new_builder`
/// — the one-out-param invoke arity (allocad `Ptr Builder` slot, peeked
/// after the call).
pub fn new_builder() -> Builder {
    let mut out: *mut sys::Output = ptr::null_mut();
    // SAFETY: `out` is a valid, initialised stack out-param for the
    // duration of the call. On the success path the C++ side stores a
    // freshly `new`-allocated Output (ffi.cpp:659-661); the null
    // initialiser means a failed call cannot smuggle a dangling handle
    // into Builder. invoke takes the extern call itself as its `call`
    // closure; the returned pointer follows crate::ffi::invoke's contract
    // (null on success, else a malloc'd NUL-terminated message it frees).
    if let Err(e) = unsafe { invoke(|| sys::glean_new_builder(&mut out)) } {
        // `new Output` can only fail on OOM (ffi.cpp:660) — which crosses
        // as a wrap.h message; allocation-class, aborts per row 61.
        allocation_guard_abort(e);
    }
    debug_assert!(
        !out.is_null(),
        "glean_new_builder returned success with a null builder"
    );
    Builder(out)
}

/// Haskell (Builder.hs:34-37):
///
/// * finishBuilder b = do
/// *   (p,n) <- FFI.invoke $ glean_finish_builder b
/// *   BS.unsafePackMallocCStringLen (castPtr p, fromIntegral n)
///
/// `unsafePackMallocCStringLen` is the zero-copy ownership transfer of the
/// malloc'd clone; the Rust twin is the crate-canonical helper
/// `crate::ffi::unsafe_malloced_byte_string` — the shipped mapping for RULE
/// §1 (strings/bytes row: `Vec<u8>` for `ByteString`) and inventory line 122
/// (Util/FFI row: "Rust: Vec<u8> from C via slice::from_raw_parts +
/// ownership transfer"): slice view + copy + `libc::free`, with no
/// assumption that the Rust global allocator is the C++ one.
// PERF(port): copies the buffer and frees the original (helper shape); the
// zero-copy `Vec::from_raw_parts(data.cast(), size, size)` form — the
// Haskell's literal twin — was rejected on allocator-coupling, per ffi.rs's
// own precedent comment on unsafe_malloced_vector: it is only sound if the
// Rust global allocator IS the system malloc AND the C side malloc'd
// exactly `size` bytes. Revisit only if the judge shows marshalling cost.
pub fn finish_builder(builder: &Builder) -> Vec<u8> {
    let mut data: *mut c_void = ptr::null_mut();
    let mut size: usize = 0;
    // SAFETY: `data`/`size` are valid, fully-initialised stack out-params
    // for the duration of the call; `builder.as_ptr()` is a live non-null
    // Output handle which the call only reads (it clones the buffer). The
    // signature mirrors ffi.h:325 exactly. invoke takes the extern call
    // itself as its `call` closure; the returned pointer follows
    // crate::ffi::invoke's contract.
    if let Err(e) =
        unsafe { invoke(|| sys::glean_finish_builder(builder.as_ptr(), &mut data, &mut size)) }
    {
        // ffi::clone_bytes' malloc is the only failure (ffi.cpp:672) —
        // which crosses as a wrap.h message; allocation-class, aborts
        // per row 61.
        allocation_guard_abort(e);
    }
    if data.is_null() {
        // malloc(0) may legally return NULL (clone of an empty buffer —
        // e.g. finish_builder straight after reset_builder). The Haskell
        // twin is unsafePackMallocCStringLen (nullPtr, 0) == empty
        // ByteString with nothing to free.
        debug_assert_eq!(size, 0);
        return Vec::new();
    }
    // SAFETY: on success the C++ side transferred ownership of exactly
    // `size` malloc'd bytes to us (release_to, ffi.cpp:672-673); the
    // pointer is non-null (guarded above) and readable for `size` bytes —
    // unsafe_malloced_byte_string's caller contract. The helper copies
    // into the Vec and then releases the C allocation with `libc::free`
    // unconditionally: no allocator identity is assumed (a Vec drop would
    // deallocate through Rust's global allocator, which need not be the
    // system malloc), and a non-null malloc(0) result (size 0) is still
    // freed rather than leaked. The Builder is unaffected (it holds its
    // own copy of the bytes).
    unsafe { crate::ffi::unsafe_malloced_byte_string(data, size) }
}

/// Haskell (Builder.hs:39-40): `sizeOfBuilder = glean_builder_size` — the
/// only symbol in the group with no error slot (no ffi::wrap on the C++
/// side), hence infallible.
pub fn size_of_builder(builder: &Builder) -> usize {
    // RULE §2 (FFI `CSize` returns row, A5): CSize maps to `usize` with the
    // wrap semantics noted bug-for-bug — here the C type is size_t
    // (unsigned) both sides, so the value passes through unchanged and no
    // `as` cast on a signed value exists anywhere in this module. The
    // Haskell's `fromIntegral n` (CSize -> Int) had the same no-op shape.
    //
    // SAFETY: `builder.as_ptr()` is a live non-null Output handle; the C++
    // body (ffi.cpp:667-669) is a pure size read with no failure mode.
    unsafe { sys::glean_builder_size(builder.as_ptr()) }
}

/// Haskell (Builder.hs:45-48), doc comment carried over verbatim:
///
/// * -- | release the current contents of the Builder and reinitialise it
/// * -- to empty.  A bit cheaper than freeing and allocating a new Builder,
///
/// (source comment ends mid-sentence — preserved as written.)
pub fn reset_builder(builder: &mut Builder) {
    // SOURCE-ABI DIVERGENCE (RULE §0, noted): the Haskell foreign import
    // typed glean_reset_builder as `Builder -> IO ()` (Builder.hs:58-59) —
    // silently discarding the real ABI's `const char*` error return
    // (ffi.h:327). The port checks the slot and aborts on any Err
    // (allocation_guard_abort) — deliberately on the safe side of the
    // source, and behaviour-identical on every path the source could
    // exercise: the C++ body (ffi.cpp:676-678, move-assigning a fresh
    // binary::Output) cannot actually throw, and allocation failures abort
    // inside C++ without crossing anyway.
    //
    // SAFETY: `builder.as_ptr()` is a live non-null Output handle; the C++
    // side move-assigns a fresh binary::Output() over it (ffi.cpp:676-678),
    // freeing the previous buffer but keeping the handle itself valid —
    // which is why this takes &mut (contents mutate, identity survives)
    // and not self by value. invoke takes the extern call itself as its
    // `call` closure; the returned pointer follows crate::ffi::invoke's
    // contract.
    if let Err(e) = unsafe { invoke(|| sys::glean_reset_builder(builder.as_ptr())) } {
        // The body's move-assign cannot actually throw (ffi.cpp:676-678),
        // so this path is unreachable in practice; kept because the C ABI
        // reserves the slot, and any Err that does cross (a wrap.h
        // message) is treated per row 61's ruling.
        allocation_guard_abort(e);
    }
}

/// Haskell (Builder.hs:52-53): `freeBuilder :: Builder -> IO ()`.
///
/// Consuming `builder` by value makes this an explicit early release: the
/// `Drop` impl performs the call, and the type system prevents a second
/// free. (`freeBuilder` is exported by the source module but has no direct
/// callers — `withBuilder`'s bracket is the only internal use — it is kept
/// for surface parity.)
pub fn free_builder(builder: Builder) {
    drop(builder)
}

// PORT STATUS: confidence=medium todos=0
