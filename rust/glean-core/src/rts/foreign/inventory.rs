// Ported from: glean/hs/Glean/RTS/Foreign/Inventory.hs
// Manifest unit: rust/glean-core/src/rts/foreign/inventory.rs
// (migration/manifest.tsv row 25, crate glean-core). RULE §4: one source
// file -> one target file; the manifest path is authoritative.
//
// ABI truth (read, not assumed): glean/rts/ffi.h:108-145 and :422-428 —
//   glean_inventory_new         :108-116  (const char* error slot +
//                                           `Inventory**` out)
//   glean_inventory_free        :117      (void)
//   glean_inventory_predicates  :119-122  (const char* + count + `const
//                                           Predicate***` out)
//   glean_inventory_serialize   :124-127  (const char* + data/size out)
//   glean_inventory_deserialize :129-132  (const char* + `Inventory**` out)
//   glean_inventory_equal       :134-137  (const char* + `bool*` out)
//   glean_predicate_unpack      :139-145  (const char* + 4 out-params)
//   glean_validate              :422-428  (const char* ; char/char/size_t +
//                                           Lookup*)
// and their C++ bodies at glean/rts/ffi.cpp:71-151 (:71-95 new, :97-99
// free, :101-111 predicates, :113-119 serialize, :121-130 deserialize,
// :132-144 predicate_unpack, :146-151 equal) and :917-930 (validate),
// over the rts Inventory itself (glean/rts/inventory.h:152-187;
// inventory.cpp:17-22 Predicate::operator==, :24-46 the id-densifying
// constructors, :57-66 predicates(), :68-82 serialize, :84-104
// deserialize) and the Validate option struct (glean/rts/validate.h:20-26;
// validate.cpp:50-187, structural failures raised via rts::error -> the
// wrap.h message channel).
//
// INVENTORY (RULE §5) — row 67 (migration/inventory.tsv line 67),
// followed EXACTLY:
//   "glean/hs/Glean/RTS/Foreign/Inventory.hs | Inventory Eq + new/serialize
//   | unsafePerformIO FFI in Eq instance + C++ owned ptr | OWNED_MUTABLE |
//   owned Rust struct over rts FFI; Eq via explicit PartialEq calling
//   glean_inventory_equal (not derive); no global state | Inventory.hs:95-99
//   instance Eq -> invoke glean_inventory_equal; :183 foreign import;
//   created via glean_inventory_new; serialize/deserialize sites
//   Inventory.hs:118,124 | confirmed".
//   Followed: `Inventory` is an OWNED Rust struct over the rts FFI (its
//   Drop is glean_inventory_free, the source's `destroy`); `Eq` is an
//   EXPLICIT `PartialEq` impl calling glean_inventory_equal — NOT a
//   derive; there is no global state. The five `unsafePerformIO` sites
//   (migration/inventory.sites.tsv rows 8-12: :26 import, :60 new, :96 eq,
//   :118 serialize, :124 deserialize) do not survive — each becomes an
//   ordinary fn, the hidden effect surfaced as an explicit return, per the
//   RULEBOOK §2 unsafePerformIO row. No divergence from the row; no
//   INVENTORY-BUG-CANDIDATE.
//
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, module-level): outbound
// edges are rows 1255 (Glean/FFI.hs -> crate::ffi), 1256
// (RTS/Foreign/Bytecode.hs -> foreign/bytecode), 1257
// (RTS/Foreign/Lookup.hs -> foreign/lookup), 1258 (RTS/Traverse.hs ->
// rts/traverse, unit 19), 1259 (RTS/Typecheck.hs -> rts/typecheck, unit
// 20) and 1260 (RTS/Types.hs -> rts/types), plus the thrift-generated
// `Glean.Types` import (Inventory.hs:36 `PredicateRef(..)`) whose Rust home
// is the generated `glean_thrift` crate. Inbound edges are rows 48, 380,
// 399, 406, 473, 1242, 1248, 1268, 1276, 1678, 1706, 1764, 1876
// (Backend/Local, Database/Schema, Schema/Types, Storage, Validate, Define,
// FactSet, Ownership, Query, Diff, GleanCLI, Merge, SendAndRebaseQueue) —
// consumers of the Inventory handle and its predicates()/validate()/
// serialize() surface.
//
// SIBLING-UNIT CONTRACTS (RULE §4; DONE siblings' real APIs read, never
// re-declared):
//   * crate::ffi (DONE, read): `invoke` (the pinned form
//     `pub unsafe fn invoke(call: impl FnOnce() -> *const c_char)
//     -> Result<(), FfiError>` — the extern call passed AS the closure),
//     `FfiError`, `unsafe_malloced_byte_string` (the Rust twin of
//     unsafePackMallocCStringLen: copy + libc::free — the serialize
//     transfer), `from_utf8` (the `fromUTF8` mapping — the predicate-name
//     decode) and `using_malloced` (the `usingMalloced` bracket — the
//     predicates() array release).
//   * crate::rts::types (DONE, read): `Pid` (#[repr(transparent)] over i64;
//     the by-value id payload, RULE §2 numeric row). (The source imports
//     `Pid(..)` only, Inventory.hs:35 — no `Fid` is used anywhere in this
//     unit; only Pid is imported below.)
//   * crate::rts::foreign::bytecode (DONE, read): `Subroutine<S>` with
//     `with_ptr` — the ONLY door onto the raw `SharedSubroutine*` that
//     glean_inventory_new's typecheckers/traversals arrays carry; the
//     pointers are collected while the `&[CompiledPredicate]` borrow keeps
//     every handle alive (see `new`).
//   * crate::rts::foreign::lookup (DONE, read): `CanLookup` with
//     `with_lookup` — the scoped `Lookup*` the glean_validate symbol takes.
//   * glean_thrift (generated, RULE §1 thrift row — "thrift types are
//     generated by build.rs ... never hand-translated"): `PredicateRef`
//     (glean/if/glean.thrift:55-58 `struct PredicateRef { 1:
//     PredicateName name; 2: Version version; }`; `Version` is `i32` at
//     :39). NOT re-declared here — the Haskell imported it from the
//     generated `Glean.Types` (Inventory.hs:36), so the port imports the
//     generated type for the same reason.

//! C++ RTS inventory handle (`Inventory`) — build from compiled
//! predicates, enumerate predicates, (de)serialize, compare, validate.
//!
//! Port notes: RULE §3 — `unsafe` appears only inside this glean-core FFI
//! wrapper, where the Haskell was already FFI; every block carries a
//! SAFETY comment. Error-recovery classification per RULE §2
//! (error-recovery rule), told with the C++ bodies:
//!   * `new`'s body (ffi.cpp:71-95) has NO precondition check — its only
//!     failure mode is allocation (the reserve, the per-name string copy,
//!     `new Inventory`), so its Err arm is ALLOCATION-class -> abort
//!     ([`allocation_guard_abort`]): the source's uncaught FFIError out of
//!     a pure `unsafePerformIO` made deterministic.
//!   * `predicates`/`serialize` bodies (ffi.cpp:101-119) are likewise
//!     allocation-only; `deserialize` (ffi.cpp:121-130 over
//!     inventory.cpp:84-104) adds a PRECONDITION class (a malformed byte
//!     range makes `serialize::get` throw). All three surface the outcome
//!     as `Result` — the `unsafePerformIO` hidden effect made explicit
//!     (RULE §2); the caller decides the disposition (the source threw).
//!   * `validate`'s body (ffi.cpp:917-930 over validate.cpp:50-187)
//!     reports structural failures (invalid fact, type mismatch, key
//!     mismatch, out-of-order enumeration) through `rts::error` —
//!     PRECONDITION-GUARD -> `Result`.
//!   * `glean_inventory_equal`'s body (ffi.cpp:146-151 over
//!     inventory.cpp:17-22) is a pure comparison (`first_id`/`preds`,
//!     allocation-free); its Err arm is unreachable in practice and aborts
//!     rather than fabricating a bool (the source's pure `==` would throw,
//!     i.e. crash).

use std::os::raw::c_void;
use std::ptr;

use glean_thrift::PredicateRef;

use crate::ffi::{from_utf8, invoke, unsafe_malloced_byte_string, using_malloced, FfiError};
use crate::rts::foreign::bytecode::Subroutine;
use crate::rts::foreign::lookup::CanLookup;
use crate::rts::types::Pid;

/// Raw C-ABI declarations for the glean_inventory_* / glean_predicate_unpack
/// / glean_validate group (glean/rts/ffi.h:108-145, :422-428).
///
/// RULE §1 (FFI row, A2): raw `extern "C"` for flat C-ABI symbols.
///
/// Signatures mirror ffi.h EXACTLY. `size_t` -> `usize` (A5 CSize row,
/// unsigned both sides); `int64_t` -> `i64`; `int32_t` -> `i32`; `char`
/// (used as a 0/1 flag by glean_validate) -> `i8`; `bool` -> `bool`; the
/// opaque `Inventory`/`Predicate`/`SharedSubroutine` handles -> `c_void`
/// pointers (only ever passed through, never dereferenced on this side).
/// Guard kinds per RULE §2 are recorded at each symbol.
pub mod sys {
    use std::os::raw::{c_char, c_void};

    unsafe extern "C" {
        // ffi.h:108-116 — `Inventory** inventory` out-param; error slot is
        // ALLOCATION-guard (body ffi.cpp:80-94: reserve, string copies,
        // `new Inventory` — no precondition check exists to fail). The
        // typecheckers/traversals arrays carry `SharedSubroutine*`
        // (ffi.h:114-115), modelled as opaque `*const c_void` per this
        // unit's contract (see `new`).
        pub fn glean_inventory_new(
            count: usize,
            ids: *const i64,
            name_ptrs: *const *const c_void,
            name_sizes: *const usize,
            versions: *const i32,
            typecheckers: *const *const c_void,
            traversals: *const *const c_void,
            inventory: *mut *mut c_void,
        ) -> *const c_char;

        // ffi.h:117 — void; body ffi.cpp:97-99 is ffi::free_ (wrap_
        // swallows all exceptions, wrap.h:34-45) — infallible.
        pub fn glean_inventory_free(inventory: *mut c_void);

        // ffi.h:119-122 — `size_t* count` + `const Predicate***`
        // out-params; body ffi.cpp:101-111 builds a malloc'd array of
        // `const Predicate*` (ffi::clone_array). ALLOCATION-guard (no
        // precondition check).
        pub fn glean_inventory_predicates(
            inventory: *mut c_void,
            count: *mut usize,
            predicates: *mut *mut c_void,
        ) -> *const c_char;

        // ffi.h:124-127 — `const void** data` + `size_t* size` out-params;
        // body ffi.cpp:113-119 is Inventory::serialize (inventory.cpp:68-82)
        // + ffi::clone_bytes(...).release_to. ALLOCATION-guard.
        pub fn glean_inventory_serialize(
            inventory: *mut c_void,
            data: *mut *const c_void,
            size: *mut usize,
        ) -> *const c_char;

        // ffi.h:129-132 — `Inventory** inventory` out-param; body
        // ffi.cpp:121-130 is Inventory::deserialize (inventory.cpp:84-104).
        // BOTH classes: the byte-range reads validate structurally
        // (PRECONDITION) and the vector/subroutine allocations can fail
        // (ALLOCATION); wrap.h merges both into the one error slot.
        pub fn glean_inventory_deserialize(
            data: *const c_void,
            size: usize,
            inventory: *mut *mut c_void,
        ) -> *const c_char;

        // ffi.h:134-137 — `bool* result` out-param; body ffi.cpp:146-151 is
        // `*result = *first == *second` (inventory.cpp:17-22,
        // Inventory::operator== inventory.h:176-178). No allocation, no
        // precondition; the error slot is effectively unreachable.
        pub fn glean_inventory_equal(
            first: *const c_void,
            second: *const c_void,
            result: *mut bool,
        ) -> *const c_char;

        // ffi.h:139-145 — 4 out-params; body ffi.cpp:132-144 reads
        // predicate->id/name/version. No allocation, no precondition — the
        // error slot is effectively unreachable.
        pub fn glean_predicate_unpack(
            predicate: *mut c_void,
            id: *mut i64,
            name: *mut *const c_void,
            name_size: *mut usize,
            version: *mut i32,
        ) -> *const c_char;

        // ffi.h:422-428 — `char typecheck`/`char keys` are 0/1 flags,
        // `size_t limit`, `Lookup* lookup`; body ffi.cpp:917-930 fills a
        // Validate and runs validate.cpp:50-187. PRECONDITION-guard
        // (structural validation errors -> rts::error); ALLOCATION can also
        // occur inside the workers, merged into the same slot by wrap.h.
        pub fn glean_validate(
            inventory: *const c_void,
            typecheck: i8,
            keys: i8,
            limit: usize,
            lookup: *mut c_void,
        ) -> *const c_char;
    }
}

/// `data CompiledPredicate = CompiledPredicate { compiledPid :: Pid,
/// compiledRef :: PredicateRef, compiledTypecheck :: Subroutine
/// CompiledTypecheck, compiledTraversal :: Subroutine CompiledTraversal }`
/// (Inventory.hs:47-52). Field names snake_cased per house style; `ref` is
/// the source's `compiledRef` and is a Rust keyword, so it is spelled
/// `r#ref` (the raw identifier is the field's name).
///
/// No deriving clause in the source — no derives here (RULE §0).
pub struct CompiledPredicate {
    /// `compiledPid :: Pid` — the predicate's id.
    pub pid: Pid,
    /// `compiledRef :: PredicateRef` — name + version.
    pub r#ref: PredicateRef,
    /// `compiledTypecheck :: Subroutine CompiledTypecheck`.
    pub typecheck: Subroutine<CompiledTypecheck>,
    /// `compiledTraversal :: Subroutine CompiledTraversal`.
    pub traversal: Subroutine<CompiledTraversal>,
}

/// `data CompiledTypecheck` — the phantom kind tag of the typechecking
/// subroutine (Haskell: `Glean.RTS.Typecheck`, Typecheck.hs:26), and
/// `data CompiledTraversal` — the tag of the traversal subroutine
/// (Haskell: `Glean.RTS.Traverse`, Traverse.hs:25). `Subroutine` is
/// polymorphic in this marker ("to avoid us mixing up the two",
/// Bytecode.hs:52-54), so both tags are needed here for `CompiledPredicate`
/// to name its two fields' kinds.
///
/// TODO: move to rts/typecheck.rs (unit 20) and rts/traverse.rs (unit 19) when those units ship
pub struct CompiledTypecheck;
pub struct CompiledTraversal;

/// `data Validate = Validate { validateTypecheck :: Bool, validateKeys ::
/// Bool, validateLimit :: Maybe Int }` (Inventory.hs:128-132) — mirrored by
/// the C++ `struct Validate` (glean/rts/validate.h:20-24).
///
/// RULE §2 (A5 CSize row): the source's `Maybe Int` is `Option<usize>` —
/// the limit is a `size_t` bound (validate.cpp:54-56), unsigned on both
/// sides; the `None` -> `maxBound` collapse happens at the call in
/// [`validate`], exactly where the Haskell's `maybe maxBound fromIntegral`
/// did it.
pub struct Validate {
    /// `validateTypecheck` — run the typechecker over every fact.
    pub typecheck: bool,
    /// `validateKeys` — check `idByKey` against each fact's id.
    pub keys: bool,
    /// `validateLimit` — cap on facts validated; `None` means unlimited.
    pub limit: Option<usize>,
}

/// `instance Default Validate where def = Validate { validateTypecheck =
/// True, validateKeys = True, validateLimit = Nothing }`
/// (Inventory.hs:134-139).
impl Default for Validate {
    fn default() -> Self {
        Validate {
            typecheck: true,
            keys: true,
            limit: None,
        }
    }
}

/// `newtype Inventory = Inventory (ForeignPtr Inventory)` (Inventory.hs:39)
/// — an owned handle to the C++ `rts::Inventory`.
///
/// The source's `instance Object Inventory` (Inventory.hs:41-44) supplied
/// `destroy = glean_inventory_free`; the Rust counterpart is the `Drop`
/// below (inventory row 67: "owned Rust struct over rts FFI"). The handle
/// is immutable — every method borrows `&self` — and is shared across the
/// crate by reference (its consumers, the inbound edges in the header).
pub struct Inventory {
    /// The owned `rts::Inventory*` from glean_inventory_new /
    /// glean_inventory_deserialize. Never null for a live value; never
    /// dereferenced on the Rust side.
    ptr: *mut c_void,
}

// The C++ `Inventory` is immutable after construction (the Haskell comment
// at Inventory.hs:59: "inventories are immutable"). The source's
// `ForeignPtr`-backed `newtype Inventory` is unconditionally `Send + Sync`;
// the raw-pointer port must match that capability — concurrent reads via
// `glean_inventory_predicates` / `glean_inventory_serialize` only read.
unsafe impl Send for Inventory {}
unsafe impl Sync for Inventory {}

// `destroy = glean_inventory_free` (Inventory.hs:44).
impl Drop for Inventory {
    fn drop(&mut self) {
        // SAFETY: `self.ptr` is a live `Inventory*` produced by
        // glean_inventory_new (ffi.cpp:93) or glean_inventory_deserialize
        // (ffi.cpp:126) and adopted exactly once by this struct;
        // glean_inventory_free (ffi.cpp:97-99, ffi::free_ -> wrap_, which
        // swallows every exception, wrap.h:34-45) is the matching finalizer
        // and cannot throw across the boundary.
        unsafe { sys::glean_inventory_free(self.ptr) }
    }
}

/// RULE §2 (error-recovery rule): "allocation-failure != structural error.
/// Every FFI null return in the inventory records which kind:
/// allocation-guard (abort) or precondition-guard (record + partial
/// result)." glean_inventory_new's body (ffi.cpp:80-94) contains no
/// precondition check — its only failure modes are bad_alloc-class throws
/// (the reserve, each name's std::string copy, `new Inventory`), which
/// wrap.h converts into a returned message. So the Err arm is
/// allocation-class and the row-sanctioned response is ABORT. The Haskell
/// shape at this site was `construct $ invoke $ ...` under
/// `unsafePerformIO` (Inventory.hs:60,74): the FFIError escaped as an
/// exception and killed the calling path uncaught (no `catch` exists on
/// any inventory-construction route), so abort reproduces the source's
/// observable outcome. abort() rather than panic! so no unwinding runs
/// first — the same shape as the sibling units' allocation_guard_abort.
fn allocation_guard_abort(e: FfiError) -> ! {
    eprintln!("glean: C++ inventory allocation failure: {}", e);
    std::process::abort()
}

/// Create a new `Inventory` from compiled predicates (Inventory.hs:58-93).
///
/// * new :: [CompiledPredicate] -> Inventory
///
/// Marshalling (Inventory.hs:60-93): the source brackets every predicate
/// with `withMany predicate ps`, encodes each `predicateRef_name` with
/// `withUTF8Text`, obtains each subroutine's pointer with `with`, and
/// collects the six parallel arrays with `unzip6` before `withArray`-ing
/// them all into glean_inventory_new. In Rust the bracketing dissolves into
/// plain owned Vecs alive for the whole call: `names` holds every UTF-8
/// name buffer, and the `&[CompiledPredicate]` borrow keeps every
/// `Subroutine` handle — and hence every `SharedSubroutine*` collected from
/// it — alive across the FFI call. The source's `!n = length ps` is
/// `predicates.len()`.
///
/// Guard kind: ALLOCATION-GUARD -> abort (see `allocation_guard_abort`);
/// on the (unreachable) Err the out-param is still null because the C++
/// body writes `*inventory` only after every allocation succeeds
/// (ffi.cpp:93).
pub fn new(predicates: &[CompiledPredicate]) -> Inventory {
    let ids: Vec<i64> = predicates.iter().map(|p| p.pid.0).collect();
    // withUTF8Text (predicateRef_name compiledRef): a Rust String is
    // already UTF-8, so its bytes are the encoding the Haskell produced
    // (crate::ffi::with_utf8_text's note) — no NUL terminator is appended.
    let names: Vec<Vec<u8>> = predicates
        .iter()
        .map(|p| p.r#ref.name.as_bytes().to_vec())
        .collect();
    let name_ptrs: Vec<*const c_void> = names.iter().map(|n| n.as_ptr() as *const c_void).collect();
    let name_sizes: Vec<usize> = names.iter().map(|n| n.len()).collect();
    let versions: Vec<i32> = predicates.iter().map(|p| p.r#ref.version).collect();

    let mut typecheckers: Vec<*const c_void> = Vec::with_capacity(predicates.len());
    let mut traversals: Vec<*const c_void> = Vec::with_capacity(predicates.len());
    for p in predicates {
        // SAFETY: with_ptr hands out the live `SharedSubroutine*` behind the
        // handle. The pointer is stored, not freed or dereferenced, and it
        // stays valid for the whole `new` call because `p` — and hence the
        // `Subroutine` owning that SharedSubroutine — is borrowed from
        // `predicates`, which outlives the glean_inventory_new call below.
        // This is the source's `with compiledTypecheck $ \tc_ptr -> ...`
        // held open by `withMany` across the same call.
        let tc = unsafe { p.typecheck.with_ptr(|ptr| ptr) };
        // SAFETY: as the typecheck pointer above, for compiledTraversal.
        let tr = unsafe { p.traversal.with_ptr(|ptr| ptr) };
        typecheckers.push(tc as *const c_void);
        traversals.push(tr as *const c_void);
    }

    let mut inventory: *mut c_void = ptr::null_mut();
    // SAFETY: every pointer argument is valid for the duration of the call —
    // ids/name_ptrs/name_sizes/versions/typecheckers/traversals are Vecs
    // alive for the whole call, each name pointer is a live UTF-8 buffer in
    // `names`, and `inventory` is an initialised stack out-param. The
    // parameter order mirrors ffi.h:108-116 exactly. invoke takes the extern
    // call AS its closure and owns the error string's release.
    let result = unsafe {
        invoke(|| {
            sys::glean_inventory_new(
                predicates.len(),
                ids.as_ptr(),
                name_ptrs.as_ptr(),
                name_sizes.as_ptr(),
                versions.as_ptr(),
                typecheckers.as_ptr(),
                traversals.as_ptr(),
                &mut inventory,
            )
        })
    };
    match result {
        Ok(()) => Inventory { ptr: inventory },
        // Allocation-class by the body's construction; abort per RULE §2
        // and the classification above.
        Err(e) => allocation_guard_abort(e),
    }
}

/// Return all predicates from an `Inventory` (Inventory.hs:106-115):
///
/// * predicates inventory = with inventory $ \inventory_ptr -> do
/// *   (count, preds) <- invoke $ glean_inventory_predicates inventory_ptr
/// *   usingMalloced preds $ forM [1 .. count] $ \i -> do
/// *     p <- peekElemOff preds $ fromIntegral i - 1
/// *     (pid, name_ptr, name_size, version) <- invoke $ glean_predicate_unpack p
/// *     ref <- PredicateRef <$> fromUTF8 name_ptr name_size
/// *                        <*> pure (fromIntegral version)
///
///     return (pid, ref)
///
/// The source's `[1 .. count]` then `i - 1` is 0-based iteration over the
/// `count`-element array; `usingMalloced preds` releases that array on every
/// exit path (crate::ffi::using_malloced is that bracket). The name is
/// decoded with `fromUTF8` (crate::ffi::from_utf8); the version is the raw
/// `i32`.
///
/// Guard kind: the array build is ALLOCATION-guard and each unpack is
/// allocation-free; the source threw on Err, so the port returns `Result`
/// and the caller decides.
pub fn predicates(&self) -> Result<Vec<(Pid, PredicateRef)>, FfiError> {
    let mut count: usize = 0;
    let mut preds: *mut c_void = ptr::null_mut();
    // SAFETY: `self.ptr` is a live `Inventory*`; `count`/`preds` are
    // initialised stack out-params; the body (ffi.cpp:101-111) writes a
    // malloc'd array of `const Predicate*` and its count. invoke owns the
    // error string.
    unsafe { invoke(|| sys::glean_inventory_predicates(self.ptr, &mut count, &mut preds)) }?;

    // `preds` is the array base (a `const Predicate**`); each element is a
    // `const Predicate*` borrowed from the inventory, valid for as long as
    // `&self` is.
    let array = preds as *const *const c_void;

    // The read pass, kept as a named closure so its unsafe operations sit
    // OUTSIDE using_malloced's own unsafe block (no nested `unsafe`). It
    // returns a Result so the `?` unwinds through the guard below — no leak
    // on the unpack error path.
    let read_all = || -> Result<Vec<(Pid, PredicateRef)>, FfiError> {
        let mut out = Vec::with_capacity(count);
        for i in 0..count {
            // SAFETY: `i < count`, and the array holds `count` valid element
            // pointers (glean_inventory_predicates' out-params), each a
            // `const Predicate*` into the still-borrowed inventory.
            let p = unsafe { *array.add(i) };
            let mut pid: i64 = 0;
            let mut name: *const c_void = ptr::null();
            let mut name_size: usize = 0;
            let mut version: i32 = 0;
            // SAFETY: `p` is a live `Predicate*` (above); the four
            // out-params are initialised stack locals; the body
            // (ffi.cpp:132-144) only reads the predicate. invoke owns the
            // error string.
            unsafe {
                invoke(|| {
                    sys::glean_predicate_unpack(
                        p as *mut c_void,
                        &mut pid,
                        &mut name,
                        &mut name_size,
                        &mut version,
                    )
                })
            }?;
            // SAFETY: on success `name`/`name_size` are the predicate's own
            // name buffer (ffi.cpp:140-141, `predicate->name.data()` /
            // `.size()`), which lives in the inventory borrowed by `&self` —
            // readable for `name_size` bytes for the rest of this call.
            // from_utf8's contract is met.
            let name = unsafe { from_utf8(name, name_size) };
            out.push((Pid(pid), PredicateRef { name, version }));
        }
        Ok(out)
    };

    // SAFETY: `preds` is the live malloc'd array glean_inventory_predicates
    // just transferred ownership of; using_malloced frees it (libc::free) on
    // every exit path, matching the source's `usingMalloced preds`.
    unsafe { using_malloced(preds, read_all) }
}

/// `serialize :: Inventory -> ByteString` (Inventory.hs:117-121):
///
/// * serialize inventory = unsafePerformIO $
/// *   with inventory $ \inventory_ptr -> do
/// *     (bytes, size) <- invoke $ glean_inventory_serialize inventory_ptr
/// *     unsafeMallocedByteString bytes size
///
/// The source's `unsafePerformIO` does not survive: the hidden effect is an
/// explicit `Result` (inventory row 67 / RULE §2 unsafePerformIO row). On
/// success the C side has transferred a malloc'd buffer
/// (ffi::clone_bytes(...).release_to, ffi.cpp:118), adopted here with
/// `unsafe_malloced_byte_string` (copy + libc::free) exactly as
/// `unsafeMallocedByteString` did.
pub fn serialize(&self) -> Result<Vec<u8>, FfiError> {
    let mut data: *const c_void = ptr::null();
    let mut size: usize = 0;
    // SAFETY: `self.ptr` is a live `Inventory*`; `data`/`size` are
    // initialised stack out-params; the body (ffi.cpp:113-119) writes the
    // malloc'd buffer and its size. invoke owns the error string.
    unsafe { invoke(|| sys::glean_inventory_serialize(self.ptr, &mut data, &mut size)) }?;
    // SAFETY: on success `data`/`size` are the malloc'd ownership transfer
    // (ffi.cpp:118); unsafe_malloced_byte_string reads exactly `size` bytes
    // and frees the buffer, and folds the (null, 0) empty case.
    Ok(unsafe { unsafe_malloced_byte_string(data, size) })
}

/// `deserialize :: ByteString -> Inventory` (Inventory.hs:123-126):
///
/// * deserialize s = unsafePerformIO $
/// *   unsafeWithBytes s $ \p n ->
/// *   construct $ invoke $ glean_inventory_deserialize p n
///
/// As with `serialize`, the `unsafePerformIO` becomes an explicit `Result`.
/// `unsafeWithBytes` hands the slice's own pointer/length for the call only
/// — Rust's borrow of `bytes` pins exactly that. The C body
/// (ffi.cpp:121-130 over inventory.cpp:84-104) reads the byte range and
/// writes a new `Inventory*` on success.
pub fn deserialize(bytes: &[u8]) -> Result<Inventory, FfiError> {
    let mut inventory: *mut c_void = ptr::null_mut();
    // SAFETY: `bytes` is a live borrow (borrow checking pins it for the
    // call); `inventory` is an initialised stack out-param; the body reads
    // the byte range and writes a new Inventory (ffi.cpp:125-129). invoke
    // owns the error string.
    unsafe {
        invoke(|| {
            sys::glean_inventory_deserialize(
                bytes.as_ptr() as *const c_void,
                bytes.len(),
                &mut inventory,
            )
        })
    }?;
    Ok(Inventory { ptr: inventory })
}

/// `validate :: CanLookup a => Inventory -> Validate -> a -> IO ()`
/// (Inventory.hs:141-150):
///
/// * validate inventory Validate{..} lookupable =
/// *   with inventory $ \inventory_ptr ->
/// *   withLookup lookupable $ \lookup ->
/// *   invoke $ glean_validate
/// *     inventory_ptr
/// *     (if validateTypecheck then 1 else 0)
/// *     (if validateKeys then 1 else 0)
/// *     (maybe maxBound fromIntegral validateLimit)
/// *     lookup
///
/// `withLookup` is the sibling's `CanLookup::with_lookup` (foreign/lookup.rs),
/// which scopes the raw `Lookup*` to the closure; the booleans cross as the
/// C `char` 0/1 flags and the limit as `usize`, with `None` collapsing to
/// `usize::MAX` (`maxBound`) at exactly this point.
pub fn validate(&self, options: &Validate, lookup: &impl CanLookup) -> Result<(), FfiError> {
    let typecheck = if options.typecheck { 1i8 } else { 0i8 };
    let keys = if options.keys { 1i8 } else { 0i8 };
    // maybe maxBound fromIntegral validateLimit (Inventory.hs:149):
    // maxBound :: CSize == usize::MAX; the C++ default matches
    // (validate.h:23 std::numeric_limits<size_t>::max()).
    let limit = options.limit.unwrap_or(usize::MAX);
    lookup.with_lookup(|lookup_ptr| {
        // SAFETY: `self.ptr` is a live `const Inventory*`; `lookup_ptr` is
        // the scoped `Lookup*` with_lookup guarantees valid for the closure;
        // the body (ffi.cpp:917-930 over validate.cpp:50-187) runs the
        // validation and reports structural failures through rts::error.
        // invoke owns the error string.
        unsafe { invoke(|| sys::glean_validate(self.ptr, typecheck, keys, limit, lookup_ptr)) }
    })
}

/// `instance Eq Inventory` (Inventory.hs:95-100):
///
/// * a == b = unsafePerformIO $
/// *   with a $ \a_ptr ->
/// *   with b $ \b_ptr -> do
/// *     r <- invoke $ glean_inventory_equal a_ptr b_ptr
/// *     return $ r /= 0
///
/// NOT a derive (inventory row 67): equality is the C++ Inventory's own
/// (inventory.cpp:17-22 over inventory.h:176-178 — first_id and the
/// predicate vector), reached through glean_inventory_equal, whose `bool`
/// out-param is the source's `r`. The `unsafePerformIO` does not survive.
impl PartialEq for Inventory {
    fn eq(&self, other: &Self) -> bool {
        let mut result: bool = false;
        // SAFETY: both pointers are live `const Inventory*` (owned by the
        // two structs); `result` is an initialised stack out-param; the body
        // (ffi.cpp:146-151 -> inventory.cpp:17-22) only reads and compares.
        // invoke owns the error string.
        match unsafe {
            invoke(|| sys::glean_inventory_equal(self.ptr, other.ptr, &mut result))
        } {
            Ok(()) => result,
            // Unreachable in practice: the comparison allocates nothing and
            // has no precondition. The source's pure `==` would throw an
            // FFIError here (a crash); abort rather than fabricate a bool.
            Err(e) => allocation_guard_abort(e),
        }
    }
}

// The C++ comparison (inventory.cpp:17-22) is an equivalence relation —
// reflexive, symmetric, transitive (compares first_id and the predicate
// vector). The Haskell's `instance Eq Inventory` (Inventory.hs:95) requires
// it; the Rust PartialEq impl above is the evidence.
impl Eq for Inventory {}

// `Clone` is deliberately NOT implemented: the source's ForeignPtr allows
// cloning, but every in-crate consumer shares `&Inventory` (the header's
// DEPENDENCY TRUTH section documents the edges). A `Clone` impl would need
// a refcounted handle (the crate's `ForeignPtr<Inventory>` pattern) — if a
// future consumer needs owned sharing, switch the `ptr` field to
// `ForeignPtr<Inventory>` (Clone + Send + Sync for free). Documented per
// RULE §5 (no silent divergence).
// Marker breakdown (closed set — 1 TODO(port), 0 BUG(port), 0
// PERF(port), 0 INVENTORY-BUG-CANDIDATE):
//   * 1 TODO(port): the two phantom kind tags CompiledTypecheck/
//     CompiledTraversal are defined here only until units 19 (rts/traverse.rs)
//     and 20 (rts/typecheck.rs) ship the modules that own them (source
//     definitions Typecheck.hs:26 / Traverse.hs:25); the note above them
//     records the move. Nothing else in the source is left open.
//   * 0 BUG(port): no source defect is reproduced. (The Haskell's `Eq`
//     throwing on an FFIError — impossible for an allocation-free
//     comparison — is documented at the impl, not reproduced as a bug.)
//   * 0 PERF(port): every copy here is source-mandated (serialize's
//     malloc'd buffer transfer via unsafe_malloced_byte_string; the name
//     decode via from_utf8; the two subroutine-pointer arrays are the
//     minimum marshalling the C ABI's array-of-pointers contract requires).
//   * 0 INVENTORY-BUG-CANDIDATE: inventory row 67 is followed exactly
//     (owned struct over rts FFI; explicit PartialEq calling
//     glean_inventory_equal; no global state). One port decision differs
//     from the unit brief but not from any row — `PredicateRef` is imported
//     from the generated `glean_thrift` crate rather than re-declared
//     locally, because RULEBOOK §1 (thrift row: "never hand-translated")
//     and RULE §4 ("never re-declare a sibling's type") require it and the
//     Haskell imported it from generated `Glean.Types` (Inventory.hs:36).
// SAFETY marker count: 14 unsafe blocks, each carrying a `// SAFETY:`
// comment — Drop 1, new 3 (two with_ptr + one invoke), predicates 5 (the
// glean_inventory_predicates invoke, the array deref, the
// glean_predicate_unpack invoke, from_utf8, and the using_malloced bracket),
// serialize 2 (invoke + unsafe_malloced_byte_string), deserialize 1
// (invoke), validate 1 (invoke), PartialEq 1 (invoke). Two further
// `unsafe impl Send/Sync for Inventory` (each with its own SAFETY
// justification: the C++ Inventory is immutable after construction).
// PORT STATUS: confidence=high todos=1
