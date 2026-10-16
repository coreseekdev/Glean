// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// Lookup — fact lookup handle for the Glean RTS.
//
// Ported from `glean/hs/Glean/RTS/Foreign/Lookup.hs` (109 lines).
//
// ABI truth (read, not assumed):
//   glean/rts/ffi.h:147      — glean_snapshot_new
//   glean/rts/ffi.h:204      — glean_lookup_free
//   glean/rts/ffi.h:206      — glean_lookup_starting_id
//   glean/rts/ffi.h:207      — glean_lookup_first_free_id
//   glean/rts/ffi.h:209-215  — glean_lookup_fact
//   glean/rts/ffi.cpp:154-159 — snapshot_new body
//   glean/rts/ffi.cpp:313-365 — lookup_free, lookup_empty, starting_id,
//   first_free_id, fact bodies
//
// RULE citations:
//   §1: Raw `extern "C"` for flat C-ABI symbols.
//   §2: A5 CSize row — `size_t` ↔ `usize`. A7 — unsafe for unbounded C++ writes.
//   §3: unsafe only inside glean-core FFI wrappers.
//   §4: One source file → one target file.
//
// INVENTORY (RULE §5) — migration/inventory.tsv line 69:
//   "Rust trait mirroring glean/rts/ffi.h Lookup vtable + owned snapshot handle
//   (Drop = glean_lookup_free); lookup_fact -> Option<Fact> with Vec<u8>
//   transfer from malloc'd buffers (unsafeMallocedByteString); pid==0 -> None
//   is precondition semantics, keep; empty/snapshot lifetime = scoped guard"
//   Followed EXACTLY: CanLookup trait, LookupGuard (Drop = glean_lookup_free),
//   lookup_fact returns Option, pid==0 -> None, bracket for empty/snapshot.
//
// INVENTORY-BUG-CANDIDATE row 69 (row text only): the row's symbol list
// includes `glean_lookup_empty`, but ffi.h does not declare it — only
// ffi.cpp defines it (ffi.cpp:317-319). The Haskell imports it directly via
// `foreign import ccall` (Lookup.hs:87-88), so the port declares the extern
// too (sys module below, with its ABI-TRUTH NOTE). This is a C++ header
// omission, not a port divergence; flagged for the inventory owner.
//
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, module-level):
//   Inbound: the Haskell imports Glean.RTS.Types (Fid), Glean.FFI
//   (unsafeMallocedByteString), Util.FFI (invoke). Outbound: this module is
//   a leaf — no other Rust module
//   depends on it yet; future consumers will be in glean-db.
//
// SIBLING-UNIT CONTRACTS (RULE §4; DONE crates'/units' real APIs read,
// never re-declared):
//   * crate::ffi's `invoke` (unsafe fn, closure returns *const c_char,
//     Result<(), FfiError>)
//   * crate::ffi's `FfiError` (the error channel)
//   * crate::ffi's `unsafe_malloced_byte_string` (buffer transfer)
//   * crate::rts::types::Fid (the fact ID type)
//   * Why `pub mod sys` is public — for sibling consumers declaring their own
//     externs.
//
// UNIT-18 ADDENDUM (units 28–31 fix round, 2026-10-16):
//   * `with_can_lookup` (Lookup.hs:47-49, exported at :12) was missing from
//     the first draft; restored below (§4 completeness — every source export
//     accounted for).
//   * `CanLookup::with_lookup` (and the two scoped helpers) gained the
//     `E: From<FfiError>` bound: the `invoke(...)?` sites need it, and the
//     reviewer of units 30+31 proved with rustc repros that omitting it is a
//     hard compile error (E0277), while an impl may not add it itself
//     (E0276). The trait doc below carries the load-bearing statement.
//   * The five invoke blocks this round's audit found without SAFETY
//     comments got them, and the trailer's unsafe count was corrected
//     (it said 8, the file has 10).

//! Lookup — fact lookup handle for the Glean RTS.

use std::os::raw::{c_char, c_void};
use std::ptr;

use crate::ffi::{invoke, unsafe_malloced_byte_string, FfiError};
use crate::rts::types::Fid;

/// A reference to a thing we can look up facts in.
///
/// This is a borrowed handle — the caller owns the C++ `Lookup`.
/// The `Lookup` struct does NOT have a `Drop` impl.
pub struct Lookup {
    ptr: *mut c_void,
    name: String,
}

impl Lookup {
    /// Create a new `Lookup` from a raw pointer and a name.
    pub fn new(ptr: *mut c_void, name: impl Into<String>) -> Self {
        Self {
            ptr,
            name: name.into(),
        }
    }

    /// Get the raw pointer to the C++ `Lookup`.
    pub fn ptr(&self) -> *mut c_void {
        self.ptr
    }

    /// Get the name of the lookup.
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// The empty lookup — matches any fact.
///
/// This is a unit struct; the C++ side allocates the actual lookup.
pub struct EmptyLookup;

/// Class of things we can look up facts in.
///
/// `E: From<FfiError>` on the callback-bearing methods is load-bearing, not
/// decorative: every implementation of this trait reaches the C++ vtable
/// through `invoke(...)?`, whose error channel is `FfiError`; the `?` needs
/// `E: From<FfiError>` to merge. Rust has no implicit bounds, so the bound
/// is stated here (trait-level, so implementations cannot add it themselves
/// — E0276). Confirmed by two independent reviews with rustc repros
/// (migration fix round, units 28–31 batch).
pub trait CanLookup {
    /// Call `f` with the raw pointer to the C++ `Lookup`.
    fn with_lookup<T, E: From<FfiError>>(
        &self,
        f: impl FnOnce(*mut c_void) -> Result<T, E>,
    ) -> Result<T, E>;

    /// Get the name of the lookup.
    fn lookup_name(&self) -> &str;
}

impl CanLookup for Lookup {
    fn with_lookup<T, E: From<FfiError>>(
        &self,
        f: impl FnOnce(*mut c_void) -> Result<T, E>,
    ) -> Result<T, E> {
        f(self.ptr)
    }

    fn lookup_name(&self) -> &str {
        &self.name
    }
}

impl CanLookup for EmptyLookup {
    fn with_lookup<T, E: From<FfiError>>(
        &self,
        f: impl FnOnce(*mut c_void) -> Result<T, E>,
    ) -> Result<T, E> {
        with_empty_lookup(|lookup| f(lookup.ptr))
    }

    fn lookup_name(&self) -> &str {
        "lookup:empty"
    }
}

/// Guard that frees the C++ `Lookup` on drop.
struct LookupGuard(*mut c_void);

impl Drop for LookupGuard {
    fn drop(&mut self) {
        // SAFETY: `self.0` is a valid pointer to a C++ `Lookup` allocated by
        // `glean_lookup_empty` or `glean_snapshot_new`. `glean_lookup_free`
        // is the corresponding free function.
        unsafe {
            sys::glean_lookup_free(self.0);
        }
    }
}

/// Call `f` with an empty lookup.
///
/// Bracket: call `glean_lookup_empty`, wrap in `Lookup`, call `f`, then `glean_lookup_free`.
/// The `E: From<FfiError>` bound is required by the `invoke(...)?` below (the
/// same bound `CanLookup::with_lookup` states; see its note).
pub fn with_empty_lookup<T, E: From<FfiError>>(
    f: impl FnOnce(&Lookup) -> Result<T, E>,
) -> Result<T, E> {
    let mut ptr: *mut c_void = ptr::null_mut();

    // If `invoke` fails, the `?` operator returns early before the guard is
    // created — no leak.
    // SAFETY: `ptr` is an initialised stack out-param; `glean_lookup_empty`
    // (body ffi.cpp:317-319) allocates the EmptyLookup and stores it in
    // `*ptr` on success. `invoke` takes the extern call itself as its
    // `call` closure and owns the error string's release.
    unsafe { invoke(|| sys::glean_lookup_empty(&mut ptr)) }?;

    let guard = LookupGuard(ptr);
    let lookup = Lookup::new(ptr, "lookup:empty");
    f(&lookup)
}

/// Call `f` with a snapshot lookup restricted to facts up to `boundary`.
///
/// Bracket: call `glean_snapshot_new`, wrap in `Lookup`, call `f`, then `glean_lookup_free`.
/// The `E: From<FfiError>` bound is required by the `invoke(...)?` below (the
/// same bound `CanLookup::with_lookup` states; see its note).
pub fn with_snapshot<T, E: From<FfiError>>(
    base: &impl CanLookup,
    boundary: Fid,
    f: impl FnOnce(&Lookup) -> Result<T, E>,
) -> Result<T, E> {
    base.with_lookup(|base_ptr| {
        let mut ptr: *mut c_void = ptr::null_mut();

        // If `invoke` fails, the `?` operator returns early before the guard is
        // created — no leak.
        // SAFETY: `base_ptr` is the live `Lookup*` the enclosing
        // `with_lookup` borrow guarantees for this closure; `ptr` is an
        // initialised stack out-param; `glean_snapshot_new` (body
        // ffi.cpp:154-159) allocates the Snapshot and stores it on success.
        // `invoke` itself owns the error string's release.
        unsafe { invoke(|| sys::glean_snapshot_new(base_ptr, boundary.0, &mut ptr)) }?;

        let guard = LookupGuard(ptr);
        let lookup = Lookup::new(ptr, base.lookup_name());
        f(&lookup)
    })
}

/// `withCanLookup :: CanLookup lookup => lookup -> (Lookup -> IO a) -> IO a`
/// (Lookup.hs:47-49): hand `f` a `Lookup` value built from the abstract
/// lookup's pointer and name.
///
/// * withCanLookup lookup f =
/// *   withLookup lookup $ \p -> f (Lookup p (lookupName lookup))
///
/// The `E: From<FfiError>` bound is transitive: this function has no invoke
/// of its own, but delegates to `CanLookup::with_lookup`, whose signature
/// carries the bound (see the trait doc above).
///
/// UNIT-18 ADDENDUM (units 28–31 fix round): this combinator is exported by
/// the source (Lookup.hs:12) and used at Open.hs:172/:181/:201/:203/:361/
/// :426, Batch.hs:190 and RenameBench.hs:30; the first draft omitted it.
/// Restored here for §4 completeness (one source file → one target file,
/// every export accounted for).
pub fn with_can_lookup<T, E: From<FfiError>>(
    lookup: &impl CanLookup,
    f: impl FnOnce(&Lookup) -> Result<T, E>,
) -> Result<T, E> {
    lookup.with_lookup(|ptr| {
        let lookup = Lookup::new(ptr, lookup.lookup_name());
        f(&lookup)
    })
}

/// Get the starting fact ID of the lookup.
pub fn starting_id(lookup: &impl CanLookup) -> Result<Fid, FfiError> {
    lookup.with_lookup(|ptr| {
        let mut id: i64 = 0;
        // SAFETY: `ptr` is the live `Lookup*` `with_lookup` guarantees for
        // this closure; `id` is an initialised stack out-param;
        // `glean_lookup_starting_id` (body ffi.cpp:321-323) writes it on
        // success. `invoke` owns the error string's release.
        unsafe { invoke(|| sys::glean_lookup_starting_id(ptr, &mut id)) }?;
        Ok(Fid(id))
    })
}

/// Get the first free fact ID of the lookup.
pub fn first_free_id(lookup: &impl CanLookup) -> Result<Fid, FfiError> {
    lookup.with_lookup(|ptr| {
        let mut id: i64 = 0;
        // SAFETY: `ptr` is the live `Lookup*` `with_lookup` guarantees for
        // this closure; `id` is an initialised stack out-param;
        // `glean_lookup_first_free_id` (body ffi.cpp:325-327) writes it on
        // success. `invoke` owns the error string's release.
        unsafe { invoke(|| sys::glean_lookup_first_free_id(ptr, &mut id)) }?;
        Ok(Fid(id))
    })
}

/// Look up a fact by ID.
///
/// Returns `None` if the fact is not found (type_ == 0).
/// Otherwise returns `Some((type_, key_bytes, value_bytes))`.
pub fn lookup_fact(
    lookup: &impl CanLookup,
    fid: Fid,
) -> Result<Option<(i64, Vec<u8>, Vec<u8>)>, FfiError> {
    lookup.with_lookup(|ptr| {
        // The C++ parameter is `int64_t* type` — the type discriminator, not
        // a predicate ID. The Haskell names it `pid` but that is misleading.
        let mut type_: i64 = 0;
        let mut key_ptr: *mut c_void = ptr::null_mut();
        let mut key_size: usize = 0;
        let mut val_ptr: *mut c_void = ptr::null_mut();
        let mut val_size: usize = 0;

        // SAFETY: `ptr` is the live `Lookup*` `with_lookup` guarantees for
        // this closure; the four out-params are initialised stack locals;
        // `glean_lookup_fact` (body ffi.cpp:329-353) fills them on success —
        // on a found fact it mallocs the key/value buffers (transferred
        // below), on a miss it leaves `type_ == 0`. `invoke` owns the error
        // string's release.
        unsafe {
            invoke(|| {
                sys::glean_lookup_fact(
                    ptr,
                    fid.0,
                    &mut type_,
                    &mut key_ptr,
                    &mut key_size,
                    &mut val_ptr,
                    &mut val_size,
                )
            })
        }?;

        if type_ == 0 {
            // Fact not found — free the empty buffers.
            if !key_ptr.is_null() {
                // SAFETY: `key_ptr` is a valid pointer allocated by `malloc`.
                unsafe { libc::free(key_ptr as *mut libc::c_void) };
            }
            if !val_ptr.is_null() {
                // SAFETY: `val_ptr` is a valid pointer allocated by `malloc`.
                unsafe { libc::free(val_ptr as *mut libc::c_void) };
            }
            return Ok(None);
        }

        // SAFETY: `key_ptr` and `val_ptr` are valid pointers to `key_size` and
        // `val_size` bytes respectively, allocated by `malloc` on the C++ side.
        // If ptr is null or size is 0, returns an empty Vec.
        let key = unsafe { unsafe_malloced_byte_string(key_ptr as *const c_void, key_size) };
        let val = unsafe { unsafe_malloced_byte_string(val_ptr as *const c_void, val_size) };

        Ok(Some((type_, key, val)))
    })
}

pub mod sys {
    use std::os::raw::{c_char, c_void};

    extern "C" {
        /// `glean/rts/ffi.h:204`
        pub fn glean_lookup_free(lookup: *mut c_void);

        /// `glean/rts/ffi.cpp:317-319`
        /// NOTE: This symbol is NOT declared in ffi.h — only defined in ffi.cpp.
        /// The Haskell source imports it directly via `foreign import ccall`.
        pub fn glean_lookup_empty(lookup: *mut *mut c_void) -> *const c_char;

        /// `glean/rts/ffi.h:147` (defined in ffi.cpp:154-159)
        pub fn glean_snapshot_new(
            base: *mut c_void,
            boundary: i64,
            snapshot: *mut *mut c_void,
        ) -> *const c_char;

        /// `glean/rts/ffi.h:206` (defined in ffi.cpp:321-323)
        pub fn glean_lookup_starting_id(lookup: *mut c_void, id: *mut i64) -> *const c_char;

        /// `glean/rts/ffi.h:207` (defined in ffi.cpp:325-327)
        pub fn glean_lookup_first_free_id(lookup: *mut c_void, id: *mut i64) -> *const c_char;

        /// `glean/rts/ffi.h:209-215` (defined in ffi.cpp:329-353)
        pub fn glean_lookup_fact(
            lookup: *mut c_void,
            id: i64,
            type_: *mut i64,
            key: *mut *mut c_void,
            key_size: *mut usize,
            value: *mut *mut c_void,
            value_size: *mut usize,
        ) -> *const c_char;
    }
}

// Marker breakdown (closed set — no open port TODOs, no source-defect
// reproductions, no performance deferrals, 1 inventory-bug-candidate):
//   * no open port TODOs: everything is clear from the source; the one
//     open-looking question (glean_lookup_empty not in ffi.h) is resolved by
//     the ABI truth (the symbol exists in ffi.cpp and the Haskell imports it
//     directly).
//   * no source-defect reproductions: none.
//   * no performance deferrals: none.
//   * 1 inventory-bug-candidate: the glean_lookup_empty symbol is not
//     declared in ffi.h (only in ffi.cpp). This is a C++ header omission, not
//     a Rust-side issue. Flag it.
// SAFETY count: 10 unsafe blocks, all covered by SAFETY comments — Drop 1,
// with_empty_lookup 1, with_snapshot 1, starting_id 1, first_free_id 1,
// lookup_fact 5 (the invoke, the two libc::free calls in the not-found
// branch, and the two unsafe_malloced_byte_string transfers, which share
// one comment). with_can_lookup adds none. Recounted in the units 28–31 fix
// round; the pre-fix trailer claimed 8 and the five invoke sites lacked
// their SAFETY comments.
// PORT STATUS: confidence=high todos=0
