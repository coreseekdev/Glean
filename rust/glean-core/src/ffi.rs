// Ported from: glean/hs/Glean/FFI.hs
// Manifest unit: rust/glean-core/src/ffi.rs (crate glean-core)
//
// The Haskell source is the spec. FFI.hs is the shared marshalling layer
// under every rts/foreign binding (import roster grep-verified: 18 files
// import Glean.FFI plain — RTS/Foreign/{Bytecode,Define,FactSet,Inventory,
// JSON,Lookup,LookupCache,Stats,Subst,Typecheck,Benchmarking}.hs,
// db/{Database/Storage/DB,Database/Storage/LMDB,Database/Storage/RocksDB,
// Database/Write/Batch,Query/UserQuery}.hs,
// write/{Write/SendAndRebaseQueue}.hs, tools/diff/Diff.hs — plus
// qualified imports at RTS.hs:67, db/Query/Codegen.hs, db/Write/JSON.hs,
// typed/{Binary,Build}.hs, and the untranslated test/tests/EncodingTest.hs.
// Stacked.hs and Thrift.hs do NOT import Glean.FFI — Stacked.hs imports
// only hsthrift's Util.FFI). The C ABI it serves (glean/rts/ffi.h,
// ABI truth read for this port) is NOT uniform:
//   * most entry points return `const char*` — null on success, else an
//     error message produced by the hsthrift `wrap.h` C++ kit (on disk at
//     hsthrift/common/util/cpp/wrap.h) from a caught std::exception —
//     C++ raiseError (rts/error.cpp) embeds a stack trace in the text;
//   * pure mutators return void (glean_factset_free, glean_free_subst,
//     glean_set_ownership_*);
//   * simple getters return values directly — glean_subst_offset ->
//     uint64_t; glean_factset_fact_count / glean_builder_size /
//     glean_subroutine_size -> size_t;
//   * two factset accessors return object pointers
//     (glean_factset_lookup -> Lookup*, glean_factset_define -> Define*);
//   * real results of the fallible `const char*` calls are delivered
//     through out-parameters;
//   * byte inputs are `const void*` (read-only — verified on
//     glean_inventory_new / glean_inventory_deserialize / etc. in ffi.h).
//
// RULE §1 (FFI row, A2): raw extern "C" is the DEFAULT for flat C-ABI
// symbols; this module is pure flat-ABI marshalling so it declares no cxx
// bridge of its own. Sibling rts/foreign units follow the per-file
// inventory rows (inventory.tsv/inventory.ffi.tsv FactSet row mandates a
// cxx class bridge; the Lookup row prescribes a Rust trait + Drop-guard
// with NO cxx); those bridges will coexist with these helpers, matching
// the Haskell where FFI.hs and cxx-adjacent bindings also coexist.
//
// RULE §5 + inventory.tsv row 122 (hsthrift/common/util/Util/FFI.hs —
// "unit lives at glean/hs/Glean/FFI.hs"; cited lines FFI.hs:42-43
// unsafeMallocedByteString, :52 withUTF8Text): target_translation is
// followed exactly — "Rust: Vec<u8> from C via slice::from_raw_parts +
// ownership transfer; every FFI export records allocation-guard vs
// precondition-guard". No divergences; no INVENTORY-BUG-CANDIDATE.

//! Marshalling helpers for the C ABI shared by the whole crate.
//!
//! Port notes on the environment this module assumes (restated from the
//! header): errors come back as a `*const c_char` (null = success) and
//! byte inputs are read-only. The old module imported `invoke` from
//! hsthrift `Util.FFI` (the kit lives outside the glean tree, at
//! hsthrift/common/util/ — read from disk for this port); its Rust
//! equivalent — the `invoke` / `FfiError` pair below — is canonically
//! homed HERE because every sibling binding needs it and glean-core is
//! the bottom of that dependency chain (RULE §0: structure-preserving —
//! the dependency shape is kept, only the home moves from an external
//! kit into the ported tree).

use std::ffi::{c_char, c_void, CStr};
use std::sync::{Arc, Mutex};

// RULE §1 (third-party deps row): whitelist — `libc` (raw free) and
// `thiserror` (error derive) are on it.

// ---------------------------------------------------------------------------
// Byte and text marshalling (FFI.hs export list order)
// ---------------------------------------------------------------------------

/// `unsafeWithBytes s f = BS.unsafeUseAsCStringLen s $ \(p,n) ->
/// f (castPtr p) (fromIntegral n)`.
///
/// Hands the bytes' pointer and length to `f` for the duration of the
/// call only. Like the Haskell, the pointer is NOT NUL-terminated (this
/// is `unsafeUseAsCStringLen`, not `useAsCString`).
// RULE §2 (A7 row — "a safe signature must not launder an unbounded C++
// write"): this stays a SAFE function because nothing here writes: the
// pointer is derived from a live `&[u8]` borrow (borrow checking pins
// validity for the call, the analogue of GHC pinning the ByteString),
// and the C side takes these buffers as `const void*` (verified in
// glean/rts/ffi.h). Misuse requires a *later* separate `unsafe`
// dereference of an escaped pointer, which carries its own obligation —
// the same shape as `slice::as_ptr`. Contract: `f` must not retain the
// pointer beyond the call (the "unsafe" the Haskell name warns about).
pub fn unsafe_with_bytes<R>(s: &[u8], f: impl FnOnce(*const c_void, usize) -> R) -> R {
    f(s.as_ptr() as *const c_void, s.len())
}

/// `copyByteString p n = BS.packCStringLen (castPtr p, fromIntegral n)`.
/// Copies `n` bytes from C-owned memory into a fresh owned buffer.
///
/// SAFETY: `p` must be valid for reads of `n` bytes.
pub unsafe fn copy_byte_string(p: *const c_void, n: usize) -> Vec<u8> {
    // SAFETY: caller contract above; the slice is only read.
    unsafe { std::slice::from_raw_parts(p as *const u8, n) }.to_vec()
}

/// `unsafeMallocedByteString p n =
/// BS.unsafePackMallocCStringLen (castPtr p, fromIntegral n)` — takes
/// ownership of a malloc'd C buffer as an owned byte vector.
///
/// Ownership semantics (RULE §5, inventory.tsv row 122): the buffer is
/// consumed — read via `slice::from_raw_parts`, then released with
/// `libc::free`. After this call the caller must not use `p` again.
///
/// Guard kind (RULE §2 error-recovery rule): a null `p` with `n > 0` is a
/// caller-side PRECONDITION violation; C++ allocation failure never
/// surfaces here as a null return — it aborts inside the C++ side
/// (allocation-guard; same classification as inventory.ffi.tsv's FactSet
/// row: "C++ OOM still aborts per RTS"). The null case is NOT
/// crash-equivalent, for the record: Haskell's
/// unsafePackMallocCStringLen(nullPtr, n) does NOT dereference at
/// construction — it wraps the pointer and the fault is deferred to the
/// first use of the bytes — whereas slice::from_raw_parts(null, n) here
/// is immediate UB when `to_vec` dereferences it. The no-null-check
/// posture is kept regardless (bug-for-bug, RULE §2 BUG rule posture:
/// the Haskell crashes on use, we crash at copy).
///
/// SAFETY: `p` must be a live malloc'd (libc-heap) buffer of at least `n`
/// readable bytes, valid to pass to `free`, and not retained elsewhere.
pub unsafe fn unsafe_malloced_byte_string(p: *const c_void, n: usize) -> Vec<u8> {
    // SAFETY: caller contract above.
    let v = unsafe { std::slice::from_raw_parts(p as *const u8, n) }.to_vec();
    // Transfer of ownership of the C allocation back to the allocator.
    // SAFETY: `p` came from malloc and is freed exactly once, here.
    unsafe { libc::free(p as *mut c_void) };
    v
}

/// `unsafeMallocedVector p n`: ForeignPtr-with-finalizerFree over a
/// malloc'd array of `Storable` elements — same ownership story as
/// [`unsafe_malloced_byte_string`], elementwise.
// PERF(port): copies the buffer (`to_vec`) then frees; the zero-copy form
// `Vec::from_raw_parts(p.cast(), n, n)` is only sound if the Rust global
// allocator is the system malloc AND the C side malloc'd exactly n
// elements' worth — an allocator-coupling contract we do not take on the
// first pass. Revisit only if the judge shows marshalling cost.
///
/// SAFETY: `p` must be a live malloc'd buffer of `n` elements of `T`
/// (properly aligned for `T` — malloc's guarantee covers fundamental
/// alignment), not retained elsewhere.
pub unsafe fn unsafe_malloced_vector<T: Copy>(p: *mut T, n: usize) -> Vec<T> {
    // SAFETY: caller contract above; elements are only read (T: Copy).
    let v = unsafe { std::slice::from_raw_parts(p, n) }.to_vec();
    // SAFETY: malloc'd pointer freed exactly once, here.
    unsafe { libc::free(p as *mut c_void) };
    v
}

/// `withUTF8Text = unsafeWithBytes . Text.encodeUtf8`.
// RULE §1 (strings/bytes row): `Text` -> `&str`. Haskell `Text` is
// UTF-16 internally so `encodeUtf8` is a real encode; a Rust `&str` is
// already the UTF-8 encoding, so `s.as_bytes()` produces byte-identical
// input with no allocation. No NUL terminator is appended (the Haskell
// didn't).
pub fn with_utf8_text<R>(s: &str, f: impl FnOnce(*const c_void, usize) -> R) -> R {
    unsafe_with_bytes(s.as_bytes(), f)
}

/// `fromUTF8 p n = Text.decodeUtf8With Text.lenientDecode <$> copyByteString
/// p n`.
// RULE §1 (strings/bytes row: "No lossy conversions at boundaries;
// Text.Encoding errors in source become explicit Result"): the no-lossy
// rule governs boundaries where the SOURCE errors — this source
// explicitly chose `lenientDecode` (U+FFFD replacement, never an error),
// so `from_utf8_lossy` is the faithful mapping, not a violation.
///
/// SAFETY: `p` must be valid for reads of `n` bytes.
pub unsafe fn from_utf8(p: *const c_void, n: usize) -> String {
    // SAFETY: delegated to copy_byte_string's caller contract.
    let bytes = unsafe { copy_byte_string(p, n) };
    String::from_utf8_lossy(&bytes).into_owned()
}

/// `unsafeMallocedUTF8 p n = Text.decodeUtf8With Text.lenientDecode <$>
/// unsafeMallocedByteString p n`.
///
/// SAFETY: `p` must be a live malloc'd buffer of `n` readable bytes,
/// freed by this call.
pub unsafe fn unsafe_malloced_utf8(p: *const c_void, n: usize) -> String {
    // SAFETY: delegated to unsafe_malloced_byte_string's caller contract.
    let bytes = unsafe { unsafe_malloced_byte_string(p, n) };
    String::from_utf8_lossy(&bytes).into_owned()
}

// ---------------------------------------------------------------------------
// Scopes over malloc'd pointers
// ---------------------------------------------------------------------------

/// `usingMalloced p action = action \`finally\` free p` — run `action`
/// with `p` alive, then free `p` on every exit path (normal or unwind —
/// a Drop guard is Rust's `finally`).
///
/// SAFETY: `p` must be a live malloc'd pointer, not freed or retained
/// elsewhere, and must not be freed again after this returns.
pub unsafe fn using_malloced<R>(p: *mut c_void, action: impl FnOnce() -> R) -> R {
    struct FreeOnDrop(*mut c_void);
    impl Drop for FreeOnDrop {
        fn drop(&mut self) {
            // SAFETY: the guard is constructed only inside using_malloced,
            // whose contract guarantees the pointer is a live malloc'd
            // allocation freed exactly once (here, on scope exit or unwind).
            unsafe { libc::free(self.0) };
        }
    }
    let _guard = FreeOnDrop(p);
    action()
}

/// `usingManyMalloced ps action = foldr usingMalloced action ps`.
///
/// `foldr` nests the brackets head-outward, so the finalizers run in
/// REVERSE list order on exit (p_n .. p_1). The recursive form below
/// creates one stack guard per element in order; locals drop in reverse
/// construction order — exactly the foldr's free order. (Free order is
/// observable in principle; kept bug-for-bug even though the buffers are
/// independent.)
///
/// SAFETY: as [`using_malloced`], for every pointer in `ps`.
pub unsafe fn using_many_malloced<R>(ps: &[*mut c_void], action: impl FnOnce() -> R) -> R {
    fn go<R>(rest: &[*mut c_void], action: &mut dyn FnMut() -> R) -> R {
        match rest.split_first() {
            None => action(),
            Some((p, rest)) => {
                // SAFETY: per-pointer contract carried from
                // using_many_malloced's caller.
                unsafe { using_malloced(*p, || go(rest, action)) }
            }
        }
    }
    go(ps, &mut action)
}

/// `withMany with as f` — the classic bracket-collector: run `with` over
/// every element, all brackets held open simultaneously, and hand `f` the
/// collected results (Haskell: CPS recursion `go bs (a:as) = with a $ \\b
/// -> go (b:bs) as`, so `f` receives them in input order).
///
/// The `with` callback `k` is the continuation and MUST be invoked
/// exactly once (same contract the Haskell version silently relies on).
///
/// CONTRACT DIVERGENCE from the Haskell type, on purpose: withMany is
/// fully polymorphic in `b` — the collected list can hold anything the
/// bracket hands the continuation, with no lifetime tie to the inputs.
/// This signature is stricter: `B: 'a` and every borrow passed to `k`
/// must be a `&'a B` valid for the WHOLE call, i.e. the handed-out
/// referents must outlive `f`. Pointer-carrying values satisfy it (a raw
/// pointer imposes no borrow of its own; store the produced values where
/// they outlive `f` and hand out `&'a` borrows of that storage);
/// borrowing a local built INSIDE `with` does not type-check — sibling
/// porters will hit exactly this at the in-tree precedent
/// Bytecode.hs:74 (`withMany (\s -> unsafeWithBytes s . curry) lits`):
/// the direct transliteration curry's `(p, n)` into a closure-local
/// tuple, and the Rust port must collect from storage that lives at
/// least `'a` instead.
// RULE §2 (partial functions row) does not apply — the once-contract is
// enforced structurally here: `k` is an ordinary FnMut and a second call
// would see an empty Option and panic loudly rather than corrupt state.
pub fn with_many<'a, A: 'a, B: 'a, R>(
    with: impl Fn(&'a A, &'a mut dyn FnMut(&'a B) -> R) -> R,
    items: &'a [A],
    f: impl FnOnce(&[&'a B]) -> R,
) -> R {
    fn go<'a, A, B, R>(
        with: &dyn Fn(&'a A, &'a mut dyn FnMut(&'a B) -> R) -> R,
        rest: &'a [A],
        acc: &mut Vec<&'a B>,
        f: &mut dyn FnMut(&[&'a B]) -> R,
    ) -> R {
        match rest.split_first() {
            None => f(acc),
            Some((a, rest)) => with(a, &mut |b| {
                acc.push(b);
                go(with, rest, acc, f)
            }),
        }
    }
    // `f` is FnOnce but `go` needs FnMut at the leaf; the Option+take
    // wrapper is FnMut and panics (never happens: the leaf is reached
    // exactly once by the CPS contract above).
    let mut f = Some(f);
    let mut acc = Vec::with_capacity(items.len());
    go(
        &with,
        items,
        &mut acc,
        &mut move |bs: &[&'a B]| {
            let f = f.take().expect("with_many leaf invoked twice");
            f(bs)
        },
    )
}

// ---------------------------------------------------------------------------
// The invoke error protocol (hsthrift Util.FFI replacement)
// ---------------------------------------------------------------------------

/// The error carried by the C ABI's `const char*` return: non-null means
/// an error message (with stack trace — see rts/error.cpp raiseError)
/// that this side reads and releases.
// RULE §2 (dynamic exceptions row): typed error carrier; hsthrift's
// exception type is external and unknowable, so the port gives the crate
// its own.
// hsthrift `Util.FFI` parity — VERIFIED against the kit, which is on
// disk (hsthrift/common/util/Util/FFI.hs + cpp/{wrap.h,ffi.cpp}):
//   * message: GHC's ffiErrorMessage reads the buffer via
//     `unsafeWithForeignPtr fp peekCString` — the message verbatim; this
//     port reads via CStr, identical for the ASCII/UTF-8 text the kit
//     produces, and Display shows it verbatim, matching
//     `instance Show FFIError where show = ffiErrorMessage`.
//   * release: the Haskell attaches `newForeignPtr hs_ffi_free_error`
//     (GC-finalized); we release eagerly through the SAME
//     `hs_ffi_free_error` (see take_error_message) — identical release
//     semantics modulo finalization timing.
// The former TODO(port) on this block is closed by that verification.
#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct FfiError {
    pub message: String,
}

/// The kit's own error releaser (hsthrift/common/util/cpp/ffi.cpp:25-30):
/// pointer-compares against wrap.h's two STATIC error strings and frees
/// everything else. Freeing an error with bare libc::free is UB — see
/// take_error_message.
unsafe extern "C" {
    fn hs_ffi_free_error(err: *const c_char);
}

/// Read, adopt and release a C error string, per the ABI: null = success.
// SAFETY: `err` is only dereferenced when non-null.
unsafe fn take_error_message(err: *const c_char) -> FfiError {
    // Decode note: hsthrift reads the message with GHC's peekCString,
    // which decodes using the CURRENT LOCALE's round-trip semantics —
    // locale-dependent, not byte-exact by construction. `to_string_lossy`
    // (U+FFFD replacement) is the closest deterministic Rust analog; the
    // kit's messages are ASCII in practice, so the difference is
    // unobservable on live paths.
    // SAFETY: non-null per the caller; a NUL-terminated C string from the
    // wrap.h kit — either a strdup'd buffer (caught-std::exception path)
    // or one of the kit's two statics. Read to an owned copy first.
    unsafe {
        let message = CStr::from_ptr(err).to_string_lossy().into_owned();
        // THE UB FIX: the error is NOT always heap-allocated. wrap.h's
        // catch(...) path returns the STATIC `unknownError` and its
        // strdup-failure path returns the STATIC `outOfMemory`
        // (wrap.h:26-31); libc::free on either is undefined behavior.
        // hs_ffi_free_error guards both statics by pointer comparison and
        // frees everything else — the exact protocol the Haskell relies
        // on via `newForeignPtr hs_ffi_free_error` — so the free is
        // routed through it, never done inline.
        hs_ffi_free_error(err);
        FfiError { message }
    }
}

/// hsthrift `Util.FFI.invoke`, at the only instance FFI.hs itself uses
/// (`invoke :: IO CString -> IO ()`, applied to the fully-applied call in
/// `ffiBuf`/`ffiBuf2`). Runs the call; a non-null return is converted to
/// `Err(FfiError)` after reading and releasing it.
///
/// Guard kind (RULE §2 error-recovery rule): the Err arm is
/// guard-kind-POLYMORPHIC — its kind is whatever the CALLEE's inventory
/// row records, not this wrapper's. Nothing aborts on the C++ side of
/// this boundary: wrap.h converts EVERY failure into a returned
/// `const char*` — a caught std::exception (including bad_alloc) becomes
/// a strdup'd message, a failed strdup becomes the static "out of
/// memory", catch(...) becomes the static "unknown error" — so
/// allocation failures DO cross this boundary as values, and whether a
/// given Err is a precondition- or allocation-guard is a per-callee fact
/// (see each unit's inventory row, e.g. inventory.ffi.tsv Define.hs:
/// "precondition-guard (id == invalidFid -> err, NOT allocation)").
// SIBLING-UNIT CONTRACT: the canonical shape consumers adapt to is THIS
// signature, pinned here —
//   `pub unsafe fn invoke(call: impl FnOnce() -> *const c_char)
//      -> Result<(), FfiError>`
// (builder.rs is being fixed separately to conform to it). hsthrift's
// other `invoke` instances marshal curried out-parameters into result
// tuples (e.g. `(Fid, CSize, Ptr) <- invoke $ glean_serialize_subst ...`
// at RTS/Foreign/Subst.hs:57-59); that class-over-arities trick has no
// Rust analogue. Consumer idiom for the sibling units: declare
// out-params as locals, call through THIS `invoke`, then read the
// locals — exactly what inventory.ffi.tsv's target_translations already
// prescribe (e.g. Diff.hs row: "Result<(u64,u64,u64)>", Define.hs row:
// "Result<Fid, InvalidRedefinition>").
///
/// SAFETY: `call`'s returned pointer, when non-null, must be a
/// NUL-terminated error string from the wrap.h kit — a strdup'd buffer
/// on the caught-std::exception path, or one of the kit's two statics
/// (`outOfMemory` when strdup itself fails, `unknownError` on
/// catch(...)) — and must not be retained or released elsewhere:
/// release happens here via `hs_ffi_free_error`, which guards the
/// statics and frees only the strdup'd buffers.
pub unsafe fn invoke(call: impl FnOnce() -> *const c_char) -> Result<(), FfiError> {
    let err = call();
    if err.is_null() {
        Ok(())
    } else {
        // SAFETY: non-null per the check; take_error_message's contract.
        Err(unsafe { take_error_message(err) })
    }
}

// ---------------------------------------------------------------------------
// Object: ForeignPtr-managed C objects (Haskell class Object)
// ---------------------------------------------------------------------------

/// `type Destroy a = FunPtr (Ptr a -> IO ())` — the C finalizer. Rust fn
/// pointers replace the FunPtr indirection (RULE §2 hsc2hs row: the
/// consumer-side convention is manual `extern "C"` declarations, whose
/// fn items coerce into this type, e.g. `&glean_free_subst`-style
/// finalizers declared as `unsafe extern "C" { fn glean_free_subst(...) }`).
pub type Destroy<T> = unsafe extern "C" fn(*mut T);

/// The shared cell behind [`ForeignPtr`]: GHC ForeignPtr semantics — the
/// finalizer runs AT MOST ONCE, either explicitly ([`release`]) or when
/// the last handle drops.
struct ForeignPtrInner<T: Object> {
    /// The live pointer, or null once finalized. Guarded so `finalize` is
    /// idempotent under racing handles.
    ptr: Mutex<*mut T>,
}

impl<T: Object> Drop for ForeignPtrInner<T> {
    fn drop(&mut self) {
        // Last handle gone: run the finalizer unless already finalized.
        // SAFETY: exclusive access in Drop; the value was swapped to null
        // by any earlier finalize, so destroy runs at most once, on the
        // live pointer create() adopted.
        let p = *self
            .ptr
            .get_mut()
            .expect("ForeignPtr finalizer mutex locked at drop");
        if !p.is_null() {
            unsafe { (T::destroy())(p) };
        }
    }
}

/// The GHC `ForeignPtr a` analogue: a cheap, cloneable, shared handle to
/// a C object whose finalizer runs at most once.
pub struct ForeignPtr<T: Object> {
    inner: Arc<ForeignPtrInner<T>>,
}

// Manual impls (a derive would wrongly require T: Clone/Send/Sync):
impl<T: Object> Clone for ForeignPtr<T> {
    fn clone(&self) -> Self {
        ForeignPtr {
            inner: Arc::clone(&self.inner),
        }
    }
}

// SAFETY: the raw pointer is never dereferenced on the Rust side — it is
// only handed to T::destroy() (whose soundness the Object impl asserts)
// and to C functions that receive it as a pointer. The Haskell ForeignPtr
// is likewise always shareable; per-type thread-safety of the underlying
// C object is the C++ side's own contract.
unsafe impl<T: Object> Send for ForeignPtr<T> {}
// SAFETY: as Send — &ForeignPtr hands out no data race surface (the only
// interior mutability is the idempotent-finalizer Mutex).
unsafe impl<T: Object> Sync for ForeignPtr<T> {}

impl<T: Object> ForeignPtr<T> {
    /// `newForeignPtr destroy ptr` (private: `create` is the only door,
    /// as in the Haskell where nothing else calls newForeignPtr).
    ///
    /// SAFETY: `ptr` must be a live, uniquely-owned C object of type T;
    /// ownership transfers (T::destroy() will be called on it at most
    /// once).
    unsafe fn new(ptr: *mut T) -> Self {
        ForeignPtr {
            inner: Arc::new(ForeignPtrInner {
                ptr: Mutex::new(ptr),
            }),
        }
    }

    /// `unsafeWithForeignPtr`: borrow the raw pointer, keeping the handle
    /// (and hence the finalizer) alive across the call.
    ///
    /// SAFETY: `f` must not retain the pointer beyond the call. This is
    /// the "unsafe" ForeignPtr accessor: a concurrent explicit
    /// `release()` during `f` leaves the pointer dangling — caller-beware
    /// exactly as in GHC's unsafeWithForeignPtr (the one corner the
    /// source itself left racy; not reproduced beyond keeping it the
    /// caller's obligation).
    pub unsafe fn with_raw<R>(&self, f: impl FnOnce(*mut T) -> R) -> R {
        // touchForeignPtr analogue: this Arc clone keeps the cell (and its
        // finalizer duty) alive for the whole call even if the caller's
        // other handles go away inside `f`.
        let _keep = Arc::clone(&self.inner);
        let p = *self
            .inner
            .ptr
            .lock()
            .expect("ForeignPtr mutex poisoned");
        // A finalized-then-used object is a dangling use in the Haskell
        // too; here it surfaces as null in debug builds rather than a
        // stale pointer (RULE §0: not a behavior change on live paths).
        debug_assert!(!p.is_null(), "ForeignPtr used after release");
        f(p)
    }

    /// `finalizeForeignPtr`: run the finalizer now, at most once.
    fn finalize(&self) {
        let p = self
            .inner
            .ptr
            .lock()
            .expect("ForeignPtr mutex poisoned")
            .swap(std::ptr::null_mut());
        if !p.is_null() {
            // SAFETY: p was the live adopted pointer, swapped out under
            // the lock so this runs at most once.
            unsafe { (T::destroy())(p) };
        }
    }
}

/// `class Object a where wrap :: ForeignPtr a -> a; unwrap :: a ->
/// ForeignPtr a; destroy :: Destroy a`.
///
/// SAFETY: the impl must ensure `destroy()` is the correct C finalizer
/// for pointers wrapped by `wrap`, and that `wrap`/`unwrap` round-trip
/// over the same handle (each wrapped pointer is uniquely owned by the
/// produced value).
pub unsafe trait Object: Sized {
    /// `destroy`: this type's C finalizer.
    fn destroy() -> Destroy<Self>;
    /// `wrap`: build the owning handle around the foreign pointer.
    fn wrap(fp: ForeignPtr<Self>) -> Self;
    /// `unwrap`: the shared foreign pointer behind the handle.
    fn unwrap(&self) -> ForeignPtr<Self>;
}

/// `create = fmap wrap . newForeignPtr destroy`.
///
/// SAFETY: `ptr` must be a live, uniquely-owned C object of type T;
/// ownership transfers — T::destroy() runs at most once, at Drop or
/// [`release`], whichever is first.
pub unsafe fn create<T: Object>(ptr: *mut T) -> T {
    // SAFETY: create's own contract is new()'s contract, verbatim.
    T::wrap(unsafe { ForeignPtr::new(ptr) })
}

/// `release = finalizeForeignPtr . unwrap` — destroy the object now
/// instead of at Drop (used to not wait for GC; call sites e.g.
/// db/Glean/Query/UserQuery.hs:461 on FactSet).
///
/// SAFETY: no use of `x` (this thread or another) may be in flight or
/// happen afterwards; the pointer is dead when this returns.
pub unsafe fn release<T: Object>(x: &T) {
    x.unwrap().finalize()
}

/// `construct mk = mask_ $ mk >>= create`.
// RULE §0 (translating, not improving): Haskell's mask_ closes an async-
// exception window between the C call allocating the object and the
// ForeignPtr attaching (a leak window). Rust has no async exceptions and
// `mk` is a single closure that returns only after the extern call
// completes, so the window does not exist and no masking analogue is
/// needed. The signature absorbs hsthrift invoke's Result because the
/// source's `construct $ invoke $ ...` composition lets the FFI exception
/// propagate straight through construct (mask_ does not catch).
///
/// SAFETY: on success `mk` must return a live, uniquely-owned C object
/// (that is [`create`]'s contract).
pub unsafe fn construct<T: Object>(
    mk: impl FnOnce() -> Result<*mut T, FfiError>,
) -> Result<T, FfiError> {
    let ptr = mk()?;
    // SAFETY: mk's success value is create's contract.
    Ok(unsafe { create(ptr) })
}

/// `with x f = unsafeWithForeignPtr (unwrap x) f`.
///
/// SAFETY: as [`ForeignPtr::with_raw`] — `f` must not retain the
/// pointer, and `x` must not be concurrently released.
pub unsafe fn with<T: Object, R>(x: &T, f: impl FnOnce(*mut T) -> R) -> R {
    // SAFETY: delegated to with_raw's contract, which is with's contract.
    unsafe { x.unwrap().with_raw(f) }
}

// ---------------------------------------------------------------------------
// Static: bracket-managed C objects without ForeignPtr (Haskell class
// Static + Constructor)
// ---------------------------------------------------------------------------

/// `class Static a where destroyStatic :: a -> IO ()` — for C values held
/// as raw pointers and freed by an explicit bracket (instances e.g.
/// RTS/Foreign/JSON.hs:54 Document, db/.../Storage/DB.hs:287 Container).
///
/// Impl note: the trait method is safe; each impl's body owns its unsafe
/// C free call and its `// SAFETY:` justification.
pub trait Static: Sized {
    fn destroy_static(&self);
}

/// `newtype Constructor a = Constructor (IO a)` — a deferred construction
/// action (no call sites in the tree — grep-verified; export-only, kept
/// for module parity).
pub struct Constructor<T> {
    mk: Box<dyn FnOnce() -> T>,
}

/// `constructor = Constructor`.
pub fn constructor<T: 'static>(mk: impl FnOnce() -> T + 'static) -> Constructor<T> {
    Constructor { mk: Box::new(mk) }
}

/// `using mk = bracket mk destroyStatic` — acquire, run, destroy on every
/// exit path. A Drop guard IS bracket under unwind.
pub fn using<T: Static, R>(mk: impl FnOnce() -> T, f: impl FnOnce(&T) -> R) -> R {
    struct StaticGuard<T: Static>(T);
    impl<T: Static> Drop for StaticGuard<T> {
        fn drop(&mut self) {
            self.0.destroy_static();
        }
    }
    let guard = StaticGuard(mk());
    f(&guard.0)
}

/// `withStatic (Constructor mk) = using mk` — no call sites in the tree
/// (grep-verified; export-only, kept for module parity).
pub fn with_static<T: Static + 'static, R>(c: Constructor<T>, f: impl FnOnce(&T) -> R) -> R {
    let Constructor { mk } = c;
    using(move || mk(), f)
}

// ---------------------------------------------------------------------------
// Sharing buffers to hold the result of FFI calls
// ---------------------------------------------------------------------------

/// `newtype FFIResultBuf = FFIResultBuf (Ptr Word64)` over an alloca of
/// two Word64. Representation note (RULE §0): the Rust struct holds the
/// two words by value — `[u64; 2]` on the caller's stack IS the alloca;
/// the C side still receives a pointer into it. The source's allocaBytes
/// memory is uninitialized; we zero-init — the values are always
/// overwritten by the call before they are read, so the initial value is
/// unobservable.
pub struct FFIResultBuf {
    words: [u64; 2],
}

/// `wORD64_SIZE = sizeOf (undefined :: Word64)`.
const WORD64_SIZE: usize = std::mem::size_of::<u64>();

impl FFIResultBuf {
    fn slot_ptr(&mut self) -> *mut u64 {
        self.words.as_mut_ptr()
    }
}

/// `withFFIResultBuf f = allocaBytes (wORD64_SIZE * 2) $ f .
/// FFIResultBuf` — a temporary 2-word buffer, avoiding a per-call
/// allocation for high-volume/low-overhead calls.
pub fn with_ffi_result_buf<R>(f: impl FnOnce(&mut FFIResultBuf) -> R) -> R {
    let mut buf = FFIResultBuf { words: [0; 2] };
    f(&mut buf)
}

// RULE §2 (partial functions row: where the source would `error`,
// panic/assert with the same timing): the source's `assert (sizeOf ...
// <= wORD64_SIZE)` is Haskell `Control.Exception.assert` — CHECKED ONLY
// WHEN NOT OPTIMISED ("will silently segfault when optimisation is on",
// FFI.hs NOTE). `debug_assert!` is its exact Rust counterpart: live in
/// debug, compiled out in release. In release an oversized T therefore
/// reads past the slot — the same silent failure the source documents.
// BUG(port): the release-mode overread is reproduced bug-for-bug (source
// FFI.hs:126-127 NOTE). Repro: build with debug assertions off (release)
// and call ffi_buf::<T> (or ffi_buf2) with a Copy type T whose
// size_of::<T>() > 8 — the debug_assert! is compiled out, no panic
// fires, and the `ptr::read` below the invoke copies size_of::<T>()
// bytes out of an 8-byte slot inside the 16-byte [u64; 2] stack buffer,
// clobbering the neighboring slot (or running past the buffer entirely
// for T > 16 bytes — the source's "silently segfault"). A size check
// that survives optimization ships only as a flagged post-parity change
// (RULE §2 BUG rule).
///
/// Consumer note (inventory.ffi.tsv RTS.hs row): the Decoder's
/// shared-buffer slot "becomes plain locals" on the Rust decode side
/// (PERF-flagged there), so this machinery is module parity plus whatever
/// consumers still choose to use it.

/// `ffiBuf (FFIResultBuf buf) fun = assert ... $ invoke (fun (castPtr
/// buf)) >> peek (castPtr buf)` — run an invoke-style call writing its
/// result into the shared buffer's first slot, then read it back.
// RULE §2 (FFI CSize row, A5): lengths are `usize` end to end here (born
// from slice::len), so the signed-negative-wraps-to-huge protocol is
// never exercised on this path — no `as usize` of a signed value.
///
/// SAFETY: `T` must fit in one Word64 (`size_of::<T>() <= 8`, debug-
/// asserted — see the release-mode note above), and `fun` must treat the
/// slot pointer as an out-param valid for one `T`; `fun`'s returned
/// pointer follows [`invoke`]'s contract.
#[inline]
pub unsafe fn ffi_buf<T: Copy>(
    buf: &mut FFIResultBuf,
    fun: impl FnOnce(*mut T) -> *const c_char,
) -> Result<T, FfiError> {
    debug_assert!(std::mem::size_of::<T>() <= WORD64_SIZE);
    let slot = buf.slot_ptr().cast::<T>();
    // SAFETY: fun's contract above; invoke's error contract.
    unsafe { invoke(|| fun(slot)) }?;
    // SAFETY: the call succeeded, so it wrote one T into the slot; T is
    // Copy and at most one word big (debug-asserted), and the slot is
    // u64-aligned, which covers any align of a <=8-byte type.
    Ok(unsafe { std::ptr::read(slot as *const T) })
}

/// `ffiBuf2 (FFIResultBuf buf) fun = ...` — two results, one per word.
///
/// SAFETY: as [`ffi_buf`], for both `A` (first slot) and `B` (second
/// slot, `buf \`plusPtr\` WORD64_SIZE`).
#[inline]
pub unsafe fn ffi_buf2<A: Copy, B: Copy>(
    buf: &mut FFIResultBuf,
    fun: impl FnOnce(*mut A, *mut B) -> *const c_char,
) -> Result<(A, B), FfiError> {
    debug_assert!(std::mem::size_of::<A>() <= WORD64_SIZE);
    debug_assert!(std::mem::size_of::<B>() <= WORD64_SIZE);
    let base = buf.slot_ptr();
    let p = base.cast::<A>();
    let q = base.add(1).cast::<B>(); // buf `plusPtr` WORD64_SIZE
    // SAFETY: as ffi_buf, for both slots.
    unsafe { invoke(|| fun(p, q)) }?;
    // SAFETY: both slots written by the successful call; Copy, <=1 word
    // each, u64-aligned slots.
    Ok(unsafe { (std::ptr::read(p as *const A), std::ptr::read(q as *const B)) })
}

// Marker breakdown (closed set — 1 open TODO(port), 1 BUG(port),
// 1 PERF(port)):
//   1. TODO(port) below: Object/ForeignPtr machinery vs the cxx bridges
//      some rts/foreign inventory rows mandate — integration decision
//      deferred to Step-4 consolidation.
//   2. BUG(port) above (on ffi_buf/ffi_buf2's release-mode overread):
//      source defect reproduced bug-for-bug with a repro line, per the
//      RULE §2 BUG rule; any fix ships as a flagged post-parity change.
//   (The round-2 TODO(port) on FfiError's hsthrift parity is CLOSED:
//    verified against the kit on disk — message read matches
//    ffiErrorMessage; release via hs_ffi_free_error.)
// The single PERF(port) (on unsafe_malloced_vector; the same copy+free
// shape backs unsafe_malloced_byte_string) is perf-only, not a todo.
// TODO(port): if the cxx bridges prescribed for the rts/foreign units
// (the cxx-mandating inventory.ffi.tsv rows, e.g. FactSet — the Lookup
// row prescribes a trait + Drop-guard with no cxx) end up owning those
// objects' lifetimes (cxx UniquePtr), the Object/ForeignPtr machinery
// here may lose its implementors; keep until Step 4 decides, then either
// wire the cxx-backed types through `impl Object` or record the
// machinery's retirement in the inventory.
// PORT STATUS: confidence=high todos=1
