// Ported from: glean/hs/Glean/RTS/Foreign/Ownership.hsc (570 lines)
// Manifest unit: rust/glean-core/src/rts/foreign/ownership.rs
// (migration/manifest.tsv line 57, crate glean-core). RULE §4: one source
// file -> one target file; the manifest path is authoritative.
//
// ABI truth (read, not assumed):
//   * glean/rts/ffi.h — iterator frees :429-431; glean_ownership_compute
//     :433-438 (Inventory*, Lookup*, OwnershipUnitIterator* by value,
//     Lookup* base, ComputedOwnership** out); the four finalizers :440-442
//     (Ownership, ComputedOwnership), :483 (Slice), :519 (DefineOwnership);
//     next_set_id :444; get_ownership_set :446-450 (uint32_t uset_id,
//     int* op, OwnershipSet** out); slice_compute :468-475 (uint32_t*
//     unit_ids, size_t size, int exclude, Slice** bases, size_t num_bases,
//     Slice** out); serialize :477-478 (const void** data, size_t* size);
//     deserialize :480-481 (const void*, size_t); make_sliced_stack
//     :485-489; sliced_stack_free :491; new_define_ownership :493-496;
//     define_ownership_subst :498-500; define_ownership_sort_by_owner
//     :502-507 (uint64_t facts, int64_t* order, size_t size,
//     FactOrder* result where 'using FactOrder = HsArray<int64_t>' :94);
//     define_ownership_add_derived :509-517; derived_ownership_compute
//     :521-525; get_ownership_stats :527; the four knob symbols :529-532
//     (void/uint64_t — NO error channel); computed_ownership_unit_count
//     :539-541; augment_ownership_with_acl :549-555.
//   * glean/rts/ffi.cpp — bodies: :932-939 iterator frees; :941-951
//     compute; :953-959 the two finalizers; :961-965 unit_count;
//     :967-1000 augment (the flattening loops, level_offset/group_offset);
//     :1002-1007 next_set_id; :1009-1011 get_fact_owner; :1013-1029
//     get_ownership_set ('new HsArray(std::move(elts))', null out on
//     folly::none); :1031-1046 slice_compute (sorts a local copy of
//     unit_ids); :1048-1055 serialize (binary::Output + clone_bytes
//     release_to); :1057-1062 deserialize (binary::Input over the
//     caller's bytes); :1065-1067 slice free; :1069-1078 make_sliced_stack
//     ('new Sliced<Slices>'); :1080-1082 sliced_stack_free; :1084-1090
//     new_define_ownership; :1092-1096 subst; :1098-1109 sort_by_owner
//     ('*result = vec', an HsArray assignment); :1111-1160 the
//     DependencyIterator; :1162-1180 add_derived
//     (the DependencyIterator over the four parallel arrays);
//     :1182-1184 free; :1186-1194 derived compute; :1196-1198 stats
//     ('*result = own->getStats()'); :1200-1214 the knobs (plain
//     process-global C++ state, not wrapped).
//   * glean/rts/ownership.h:61-71 — 'struct OwnershipStats { uint64_t
//     num_units; size_t units_size; uint64_t num_sets; size_t sets_size;
//     uint64_t num_owner_entries; size_t owners_size; int64_t
//     num_orphan_facts; }' (num_orphan_facts < 0 = "don't know yet").
//     glean/rts/ownership/uset.h:28 — 'constexpr UsetId INVALID_USET =
//     0xffffffff;'; :31 — 'enum SetOp { Or, And };' (Or == 0, And == 1).
//   * hsthrift HsArray (the one C++ container this unit marshals):
//     cpp/HsStruct.h:444-544 — 'struct HsArray<T> { std::vector<T> v_;
//     const T* a = v_.data(); size_t n = v_.size(); }'; the Haskell peek
//     reads exactly 'a' and 'n' (Foreign/CPP/HsStruct/Types.hsc:467-472,
//     'peekDataSize'). The symbols the Haskell's Storable/Marshallable
//     instances call are the kit's own: HsStruct.cpp:240
//     'HS_DEFINE_MARSHALLABLE(HsArrayInt64, HsArray<int64_t>)',
//     :301 'HS_DEFINE_ARRAY_CONSTRUCTIBLE(UInt32, uint32_t)', with the
//     macro bodies at Marshallable.h:14-16, Destructible.h:28-34
//     ('destructible_destruct##Name' / 'destructible_delete##Name') and
//     Constructible.h:28-35 ('constructible_constructDefault##Name').
//     This port calls those kit symbols directly instead of re-deriving
//     the marshalling (see the HsArray section below).
//
// INVENTORY (RULE §5) — rows governing this file, followed with the
// divergences listed below:
//   * inventory.tsv row 71 (line 71; status confirmed). OWNED_MUTABLE.
//     Target translation:
//     "owned types -> Rust guards with Drop = the four free fns;
//     serializeSlice -> Vec<u8> transfer, deserializeSlice borrows input;
//     iterators scoped guards; sliced stack = scoped Lookup view; global
//     knobs (compact_threshold, merge_cache_size) -> once_cell globals or
//     cxx statics". Followed:
//       - the four owned handles are crate::ffi::ForeignPtr wrappers
//         (unsafe impl Object, Drop = the four finalizers) — the same
//         shape bytecode.rs uses for Subroutine;
//       - serialize -> Vec<u8> (copy + free through
//         unsafe_malloced_byte_string), deserialize takes &[u8];
//       - UnitIterator / DerivedFactOwnershipIterator are borrowed
//         Static handles used through crate::ffi::using;
//       - SlicedStack<B> is a scoped CanLookup view (the C++
//         SlicedStack* lives only for the duration of one with_lookup);
//       - the knobs are forwarded to the C++ statics (glean_set_ /
//         glean_get_ownership_*), i.e. the row's "cxx statics" option.
//         The row's TODO(port) asks who reads them in the Rust port and
//         whether cross-process parity is needed. Fact: the Haskell tests
//         do read and write them (glean/test/tests/IncrementalTest.hs:
//         852-858), but tests are not translated (RULE §4), so this port
//         has NO Rust reader yet — the wrappers are exported for
//         parity/future tests and the state stays in the C++ engine. That
//         open item is marked TODO(port) at the knob section (todos=1).
//         The parity question itself is the C++ engine's own, unchanged
//         by the port.
//   * inventory.ffi.tsv row 31 covers the SAME file but is NOT identical
//     to row 71: its classification still says "cxx bridge" without the
//     A2 raw-extern-C supersession, it has no "manifest target exists"
//     evidence, and its status is "unknown (TODO(port): manifest gap +
//     global-knob ownership ...)". It is stale; row 71 and manifest line
//     57 are authoritative. Flagged below (INVENTORY-BUG-CANDIDATE 2).
//   * inventory.sites.tsv row 661: "hsc2hs binding file — (entire .hsc
//     file)". The hsc2hs directives are replaced per RULE §2's hsc2hs
//     row: '#const INVALID_USET' -> INVALID_USET (u32),
//     '#const Or/And' -> the SetOp discriminants, and the
//     '#peek/#size/#alignment OwnershipStats' directives -> the
//     #[repr(C)] OwnershipStats read straight from the C out-param
//     (see the Storable note below). No hsc2hs survives.
//
// INVENTORY-BUG-CANDIDATE (row text only, two items):
//   1. inventory.tsv row 71's prose says "NOT in manifest.tsv", but
//      manifest.tsv line 57 maps this file to
//      rust/glean-core/src/rts/foreign/ownership.rs (the row's EVIDENCE
//      column says "manifest target exists"). The prose is stale; the
//      manifest is authoritative (RULE §4). Flagged for the inventory
//      owner; the port follows the manifest.
//   2. inventory.ffi.tsv row 31 still carries the pre-A2 "cxx bridge"
//      wording, no manifest-target evidence, and status "unknown
//      (TODO(port): manifest gap + global-knob ownership ...)" — stale
//      next to row 71's "confirmed". Flagged for the inventory owner; the
//      port follows row 71 + the manifest.
//
// REPRESENTATION DIVERGENCES (all deliberate, none observable):
//   1. SlicedStack<B> stores the composed lookupName in a String field.
//      The source's 'lookupName (SlicedStack _ base) =
//      "slicedstack:" <> lookupName base' (Ownership.hsc:171) is a pure
//      function returning Text; the Rust CanLookup::lookup_name returns
//      &str BORROWED from self (lookup.rs:130), so the concatenation must
//      be materialised at construction. Same value, same lifetime rule.
//   2. UnitIterator and DerivedFactOwnershipIterator are NOT Copy and
//      compute / compute_derived_ownership take them by &UnitIterator /
//      &DerivedFactOwnershipIterator. The Haskell passes the
//      pointer-sized newtype BY VALUE and still holds the value
//      afterwards for the caller's bracket (Storage/DB.hs:179-180 for the
//      unit iterator, :238-242 for the derived iterator); moving a
//      non-Copy Rust handle would make
//      that free impossible. Borrowing reproduces "use then free" and
//      makes a double free unrepresentable.
//   3. HsArray<T> is read through a #[repr(C)] HsArrayView<T>
//      { std::vector<T>; const T* a; size_t n } whose first field is
//      [usize; 3]. This is the libstdc++/libc++ std::vector layout
//      (three pointers) that the Haskell side also relies on implicitly
//      via hsc2hs's '#peek DummyHsArray, a' — the C++ header's
//      static_assert (HsStruct.h:548-551) only pins that every
//      HsArray<T> has the SAME size, not that the vector is 24 bytes.
//      The assumption is load-bearing for any Rust caller of
//      glean_get_ownership_set / define_ownership_sort_by_owner and is
//      stated here rather than hidden. VERIFIED on the build toolchain
//      (g++ 13, libstdc++, -std=c++20): sizeof(std::vector<uint32_t>) =
//      24, sizeof(HsArray<uint32_t>) = sizeof(HsArray<int64_t>) = 40,
//      alignof = 8, std::is_standard_layout_v<HsArray<...>> = true; with
//      declaration order that pins a at 24 and n at 32. A different
//      stdlib/debug-vector build would change these offsets; the Rust
//      const assert cannot see the C++ side, so this note is the record.
//   4. showOwnershipStats's renderBytes (Ownership.hsc:313-317) comes
//      from hsthrift Util.PrettyPrint (PrettyPrint.hs:58-72) — an
//      external kit module with no manifest unit (the same class as
//      Util.FFI). Its four-branch body is inlined as the private
//      render_bytes below: the same text for non-negative size_t inputs
//      in the C locale, including the lowercase "kiB" and the strict >
//      comparisons.
//   5. 'instance Storable OwnershipStats' with 'poke _ = error
//      "Storable OwnershipStats"' (Ownership.hsc:336) has no Rust
//      counterpart: the port only ever READS the struct from the C
//      out-param. The panic is unreachable by construction (no write
//      path exists), and is recorded here instead of being invented as
//      a method.
//   6. substOwnership takes FactOwnership by value in the source and
//      releases each id vector through unsafeSubstIntervalsAndRelease
//      (Ownership.hsc:357-361); the Rust takes &FactOwnership and maps
//      each vector Id -> Fid (a copy), leaving the caller's map intact.
//      Values are identical; the copy is marked PERF(port) below.
//   7. copy_hs_array treats n > 0 with a null data pointer as empty and
//      debug-asserts the invariant. The Haskell would fault on such an
//      object; a constructed HsArray never has that shape, so the branch
//      is unreachable on valid input and only guards against a layout
//      regression (divergence 3).
//
// PARTIAL FUNCTIONS (RULE §2): exactly ONE error call is reproduced —
// 'error "unknown SetOp"' (Ownership.hsc:296) in get_ownership_set (a C++
// int outside {0,1}) becomes panic!("unknown SetOp"), the same message.
// The Storable OwnershipStats poke is NOT reproduced (divergence 5: no
// write path, no Storable trait), so it is not a second panic. No other
// error/undefined exists in the file.
//
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, module-level): outbound
// rows 1267 (FFI.hs), 1268 (Inventory.hs), 1269 (Lookup.hs), 1270
// (Subst.hs), 1271 (RTS/Types.hs), 1272 (Util/Vector.hs). Inbound rows
// include 141 (db/Glean/Database/ACLOwnership.hs), 156 (Backup.hs), 206
// (CompletePredicates.hs), 257 (Data.hs), 316 (Logger.hs), 333 (Open.hs),
// 345 (Ownership.hs), 408 (Storage.hs), 416 (Storage/DB.hs), 426
// (Storage/LMDB.hs), 441 (Storage/RocksDB.hs), 459 (Types.hs), 489
// (Write/Batch.hs), 518 (Writes.hs), 706 (Query/UserQuery.hs), 1277
// (Query.hsc), 1765 (gleancli/Merge.hs), 1879
// (SendAndRebaseQueue.hs). The storage rows are why Ownership::from_raw
// exists below (glean_rocksdb_get_ownership, storage/ffi.h:149, produces
// an Ownership**).
//
// SIBLING-UNIT CONTRACTS (RULE §4; DONE siblings' real APIs read, never
// re-declared):
//   * crate::ffi (read): invoke (unsafe; closure -> *const c_char ->
//     Result<(), FfiError>), FfiError, Object/ForeignPtr/Destroy/create/
//     with, Static/using, unsafe_malloced_byte_string, unsafe_with_bytes.
//   * crate::rts::foreign::lookup (read): CanLookup (with_lookup<T, E:
//     From<FfiError>>, lookup_name -> &str) and its *mut c_void
//     opaque-pointer convention.
//   * crate::rts::foreign::inventory (read): Inventory::as_ptr
//     (inventory.rs:335-337) — Inventory is a plain pointer handle,
//     not an Object, so compute takes &Inventory and uses as_ptr.
//   * crate::rts::foreign::subst (read): Subst::as_ptr and
//     unsafe_subst_intervals_and_release(Vec<Fid>) ->
//     Result<Vec<Fid>, FfiError> (subst.rs:544-554), used by
//     subst_ownership.
//   * crate::rts::types (read): Fid, Pid, Id.
//   * glean_thrift (generated crate, read): FactDependencies { facts:
//     Vec<i64>, dependencies: Vec<i64> } (glean.thrift:153-156), the
//     type add_derived_owners marshals.
//   * glean-util's util/vector.rs (manifest line 27) is an EMPTY module
//     for vector >= 0.13: the Haskell's unsafeCoerceVector sites become
//     explicit Fid/UsetId mapping below, not a cross-crate call.
//   * sys is pub because sibling units declare externs over the SAME
//     opaque handles (Storage/RocksDB's Ownership**, Storage/DB's
//     ownership plumbing) and because get_ownership_set's OwnershipSet*
//     is an HsArray the kit owns.
//
// RULE §3: unsafe appears only in this FFI wrapper (raw externs,
// ForeignPtr borrows, HsArray reads); every block carries a SAFETY
// comment. RULE §2 error-recovery — the inventory row records no guard
// kind, so each symbol is classified from its C++ body:
//   * ALLOCATION-GUARD (only bad_alloc-class failures cross): compute,
//     compute_derived_ownership, slice, slice_serialize,
//     new_define_ownership, define_ownership_subst, sort_by_owner,
//     add_derived, next_set_id, get_ownership_set, get_ownership_stats,
//     computed_ownership_unit_count, get_fact_owner.
//   * PRECONDITION-GUARD (structural errors cross as messages):
//     slice_deserialize (binary::Input/serialize::get throws on truncated
//     or malformed bytes — slice.cpp:276-284) and
//     augment_ownership_with_acl (acl.cpp:174-179 throws
//     std::runtime_error when a promoted UsetId cannot be resolved).
//     subst_ownership's sibling call Substitution::substIntervals is also
//     precondition-guarded (substitution.h:52-54, "interval out of
//     range").
//   * NO CHANNEL: the four knobs.
// Every const char* symbol goes through crate::ffi::invoke, so the Err arm
// is RETURNED (never aborted) — matching the Haskell, where invoke throws
// FFIError.

//! C++ RTS ownership handles: Ownership / ComputedOwnership / Slice /
//! DefineOwnership, the unit iterators, slice serialization, ownership-set
//! introspection, ACL augmentation, and the fact-ownership map.
//!
//! Port notes: the four owned handles are crate::ffi::ForeignPtr wrappers
//! (clone-shared, finalizer at most once); the two iterators are borrowed
//! Static handles; the SlicedStack lookup view brackets its C++ stack for
//! exactly one with_lookup call. See the header block for the divergences
//! and the two inventory flags.

use std::collections::HashMap;
use std::ffi::c_void;
use std::mem::MaybeUninit;
use std::os::raw::c_int;
use std::ptr;

use crate::ffi::{
    create, invoke, unsafe_malloced_byte_string, unsafe_with_bytes, with, Destroy, FfiError,
    ForeignPtr, Object, Static,
};
use crate::rts::foreign::inventory::Inventory;
use crate::rts::foreign::lookup::CanLookup;
use crate::rts::foreign::subst::Subst;
use crate::rts::types::{Fid, Id, Pid};
use glean_thrift::FactDependencies;

/// 'constexpr UsetId INVALID_USET = 0xffffffff' (ownership/uset.h:28).
/// The value glean_get_fact_owner writes for "no owner"; the Haskell
/// compares against the hsc2hs #const (Ownership.hsc:274-276).
const INVALID_USET: u32 = 0xffff_ffff;

/// 'newtype UnitIterator = UnitIterator (Ptr UnitIterator)' (Ownership.hsc:
/// 81) — an opaque OwnershipUnitIterator* produced by the DB layer.
///
/// Not Copy: the source passes the pointer-sized value to compute and
/// then frees it in a bracket (using), so a Rust move would lose the
/// free. Use from_raw + using, and borrow it for compute.
#[repr(transparent)]
#[derive(Debug)]
pub struct UnitIterator(*mut c_void);

impl UnitIterator {
    /// Adopt an OwnershipUnitIterator*.
    ///
    /// SAFETY: ptr must be a live iterator from the C ABI (never already
    /// freed); the returned handle owns it — destroy_static must run
    /// exactly once (use crate::ffi::using).
    pub unsafe fn from_raw(ptr: *mut c_void) -> UnitIterator {
        UnitIterator(ptr)
    }

    /// Borrowed raw pointer for the compute extern.
    pub fn as_ptr(&self) -> *mut c_void {
        self.0
    }
}

impl Static for UnitIterator {
    fn destroy_static(&self) {
        // SAFETY: self.0 is the live iterator adopted by from_raw;
        // glean_ownership_unit_iterator_free (ffi.cpp:932-934, ffi::free_)
        // consumes it and cannot throw across (wrap_).
        unsafe { sys::glean_ownership_unit_iterator_free(self.0) }
    }
}

/// 'newtype UnitId = UnitId Word32' (Ownership.hsc:88) — a leaf ownership
/// unit (one concrete file path).
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnitId(pub u32);

/// 'newtype UsetId = UsetId Word32' (Ownership.hsc:92) — a node in the
/// ownership DAG.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UsetId(pub u32);

/// 'firstUsetId = UsetId 0' (Ownership.hsc:95-96).
pub const FIRST_USET_ID: UsetId = UsetId(0);

// ---------------------------------------------------------------------------
// Owned handles (Haskell class Object)
// ---------------------------------------------------------------------------

/// 'newtype Ownership = Ownership (ForeignPtr Ownership)' (Ownership.hsc:
/// 98) — the ownership data of one DB.
pub struct Ownership {
    fp: ForeignPtr<Ownership>,
}

// SAFETY: destroy() is the correct finalizer for the pointers wrap()
// adopts — glean_ownership_free over an Ownership* produced by
// glean_rocksdb_get_ownership (storage/ffi.h:149) — and wrap/unwrap
// round-trip over the same clone-shared ForeignPtr. The only door onto a
// live handle is create() (inside from_raw / compute callers).
unsafe impl Object for Ownership {
    fn destroy() -> Destroy<Self> {
        destroy_ownership
    }

    fn wrap(fp: ForeignPtr<Self>) -> Self {
        Ownership { fp }
    }

    fn unwrap(&self) -> ForeignPtr<Self> {
        self.fp.clone()
    }
}

unsafe extern "C" fn destroy_ownership(p: *mut Ownership) {
    // SAFETY: p is the Ownership* adopted at construction; ForeignPtr's
    // cell guarantees at-most-once execution, and the body
    // (ffi.cpp:953-955, ffi::free_ -> wrap_) cannot throw across.
    unsafe { sys::glean_ownership_free(p.cast()) }
}

impl Ownership {
    /// Adopt a raw Ownership* — the seam for storage units whose externs
    /// return an Ownership** out-param (glean_rocksdb_get_ownership,
    /// storage/ffi.h:149; the Haskell side wraps it with the Object
    /// instance, Ownership.hsc:98-103).
    ///
    /// SAFETY: ptr must be a live, uniquely-owned Ownership* from a
    /// successful C ABI call (never null, never already adopted or freed
    /// elsewhere). Ownership transfers to the returned handle: its
    /// finalizer (glean_ownership_free) runs exactly once.
    pub unsafe fn from_raw(ptr: *mut c_void) -> Ownership {
        // SAFETY: delegated to create's contract, which from_raw's doc
        // states verbatim.
        unsafe { create(ptr.cast()) }
    }
}

/// 'newtype ComputedOwnership = ComputedOwnership (ForeignPtr
/// ComputedOwnership)' (Ownership.hsc:105) — the result of compute /
/// computeDerivedOwnership.
pub struct ComputedOwnership {
    fp: ForeignPtr<ComputedOwnership>,
}

// SAFETY: as Ownership — destroy is glean_computed_ownership_free
// (ffi.cpp:957-959), the matching finalizer for the ComputedOwnership*
// that glean_ownership_compute / glean_derived_ownership_compute return.
unsafe impl Object for ComputedOwnership {
    fn destroy() -> Destroy<Self> {
        destroy_computed_ownership
    }

    fn wrap(fp: ForeignPtr<Self>) -> Self {
        ComputedOwnership { fp }
    }

    fn unwrap(&self) -> ForeignPtr<Self> {
        self.fp.clone()
    }
}

unsafe extern "C" fn destroy_computed_ownership(p: *mut ComputedOwnership) {
    // SAFETY: p is the ComputedOwnership* adopted at construction;
    // ForeignPtr runs this at most once; the body (ffi.cpp:957-959,
    // ffi::free_ -> wrap_) cannot throw across.
    unsafe { sys::glean_computed_ownership_free(p.cast()) }
}

/// 'newtype Slice = Slice (ForeignPtr Slice)' (Ownership.hsc:127) — a
/// slice view of a DB over a set of units.
pub struct Slice {
    fp: ForeignPtr<Slice>,
}

// SAFETY: as Ownership — destroy is glean_slice_free (ffi.cpp:1065-1067),
// the matching finalizer for the Slice* that glean_slice_compute /
// glean_slice_deserialize return.
unsafe impl Object for Slice {
    fn destroy() -> Destroy<Self> {
        destroy_slice
    }

    fn wrap(fp: ForeignPtr<Self>) -> Self {
        Slice { fp }
    }

    fn unwrap(&self) -> ForeignPtr<Self> {
        self.fp.clone()
    }
}

unsafe extern "C" fn destroy_slice(p: *mut Slice) {
    // SAFETY: p is the Slice* adopted at construction; ForeignPtr runs
    // this at most once; the body (ffi.cpp:1065-1067, ffi::free_ ->
    // wrap_) cannot throw across.
    unsafe { sys::glean_slice_free(p.cast()) }
}

/// 'newtype DefineOwnership = DefineOwnership (ForeignPtr
/// DefineOwnership)' (Ownership.hsc:182) — the builder for a batch's
/// fact ownership.
pub struct DefineOwnership {
    fp: ForeignPtr<DefineOwnership>,
}

// SAFETY: as Ownership — destroy is glean_define_ownership_free
// (ffi.cpp:1182-1184), the matching finalizer for the DefineOwnership*
// that glean_new_define_ownership returns.
unsafe impl Object for DefineOwnership {
    fn destroy() -> Destroy<Self> {
        destroy_define_ownership
    }

    fn wrap(fp: ForeignPtr<Self>) -> Self {
        DefineOwnership { fp }
    }

    fn unwrap(&self) -> ForeignPtr<Self> {
        self.fp.clone()
    }
}

unsafe extern "C" fn destroy_define_ownership(p: *mut DefineOwnership) {
    // SAFETY: p is the DefineOwnership* adopted at construction;
    // ForeignPtr runs this at most once; the body (ffi.cpp:1182-1184,
    // ffi::free_ -> wrap_) cannot throw across.
    unsafe { sys::glean_define_ownership_free(p.cast()) }
}

// ---------------------------------------------------------------------------
// compute / slice / serialization
// ---------------------------------------------------------------------------

/// 'compute inv l base iter' (Ownership.hsc:112-125): compute the
/// ownership of a DB from its unit iterator.
///
/// Guard kind: the C++ body (ffi.cpp:941-951) is a pure
/// computeOwnership + release; its only failure mode is allocation
/// (wrap.h), so the Err arm is allocation-class and returned.
pub fn compute<D, B>(
    inventory: &Inventory,
    db: &D,
    base: Option<&B>,
    iter: &UnitIterator,
) -> Result<ComputedOwnership, FfiError>
where
    D: CanLookup + ?Sized,
    B: CanLookup + ?Sized,
{
    // The source nests withLookup db INSIDE with inventory and the base
    // INSIDE both (Ownership.hsc:120-125); nesting here is the same.
    db.with_lookup(|lookup_ptr| match base {
        None => compute_with(inventory, lookup_ptr, iter, ptr::null_mut()),
        Some(base) => {
            base.with_lookup(|base_ptr| compute_with(inventory, lookup_ptr, iter, base_ptr))
        }
    })
}

fn compute_with(
    inventory: &Inventory,
    lookup: *mut c_void,
    iter: &UnitIterator,
    base: *mut c_void,
) -> Result<ComputedOwnership, FfiError> {
    let mut result: *mut c_void = ptr::null_mut();
    // SAFETY: inventory.as_ptr() is the live Inventory* the borrow
    // guarantees; lookup and base are the live Lookup* the enclosing
    // with_lookup calls guarantee for this closure; iter.as_ptr() is the
    // live iterator borrowed for the call; result is an initialised
    // stack out-param. invoke owns the error string's release.
    unsafe {
        invoke(|| {
            sys::glean_ownership_compute(
                inventory.as_ptr(),
                lookup,
                iter.as_ptr(),
                base,
                &mut result,
            )
        })?;
        // SAFETY: on success result is a fresh, uniquely-owned
        // ComputedOwnership* (ffi.cpp:948-949); the ? returned early
        // otherwise.
        Ok(create(result.cast()))
    }
}

/// 'slice ownership bases units exclude' (Ownership.hsc:135-147): build
/// a slice over the given units (or all but them, when exclude).
pub fn slice(
    ownership: &Ownership,
    bases: &[Slice],
    units: &[UnitId],
    exclude: bool,
) -> Result<Slice, FfiError> {
    let mut base_ptrs: Vec<*mut c_void> = Vec::with_capacity(bases.len());
    // SAFETY: ownership is a live handle for the whole call; each base
    // pointer is collected from a Slice that outlives the call, and the
    // C++ body only reads the array (ffi.cpp:1040-1042).
    unsafe {
        with(ownership, |own_ptr| {
            for base in bases {
                // SAFETY: base outlives this call; with_raw's contract is
                // that the pointer is not retained past the closure — it
                // is only stored in base_ptrs for the call below.
                with(base, |p| base_ptrs.push(p.cast()));
            }
            let mut result: *mut c_void = ptr::null_mut();
            // SAFETY: own_ptr is the live Ownership*; units is a
            // repr(transparent) [UnitId] (u32) valid for units.len()
            // reads (the body copies it, ffi.cpp:1040); base_ptrs holds
            // base_ptrs.len() live Slice*; result is an initialised
            // stack out-param.
            invoke(|| {
                sys::glean_slice_compute(
                    own_ptr.cast(),
                    units.as_ptr().cast::<u32>(),
                    units.len(),
                    exclude as c_int,
                    base_ptrs.as_ptr(),
                    base_ptrs.len(),
                    &mut result,
                )
            })?;
            // SAFETY: on success result is a fresh, uniquely-owned Slice*
            // (ffi.cpp:1043-1044); the ? returned early otherwise.
            Ok(create(result.cast()))
        })
    }
}

/// 'serializeSlice slice' (Ownership.hsc:149-153): serialize a slice to
/// bytes, taking ownership of the malloc'd buffer.
pub fn serialize_slice(slice: &Slice) -> Result<Vec<u8>, FfiError> {
    // SAFETY: slice is a live handle for the call.
    unsafe {
        with(slice, |p_slice| {
            let mut data: *const c_void = ptr::null();
            let mut size: usize = 0;
            // SAFETY: p_slice is the live Slice*; data/size are
            // initialised stack out-params. The body allocates a clone
            // (ffi.cpp:1051-1053) and releases it to us on success only.
            invoke(|| sys::glean_slice_serialize(p_slice.cast(), &mut data, &mut size))?;
            // A zero-length clone may hand back null; from_raw_parts
            // requires non-null even for n == 0, and a null pointer means
            // there is nothing to free.
            if data.is_null() {
                return Ok(Vec::new());
            }
            // PERF(port): the Haskell's unsafeMallocedByteString
            // (Glean/FFI.hs:42-44, BS.unsafePackMallocCStringLen) WRAPS
            // the malloc'd buffer without copying; this helper copies and
            // frees. A zero-copy form needs a Vec-from-malloc allocator
            // contract (the family note lives in ffi.rs).
            // SAFETY: data is non-null and points at size malloc'd bytes
            // owned by this call (clone_bytes.release_to); the helper
            // copies and frees them.
            Ok(unsafe_malloced_byte_string(data, size))
        })
    }
}

/// 'deserializeSlice bs' (Ownership.hsc:155-159): deserialize a slice,
/// borrowing the input bytes for the duration of the call.
pub fn deserialize_slice(bytes: &[u8]) -> Result<Slice, FfiError> {
    unsafe_with_bytes(bytes, |data, len| {
        let mut result: *mut c_void = ptr::null_mut();
        // SAFETY: data is valid for len bytes for the whole closure; the
        // body builds a binary::Input over it (ffi.cpp:1058-1062) and
        // does not retain it; result is an initialised stack out-param.
        unsafe { invoke(|| sys::glean_slice_deserialize(data, len, &mut result)) }?;
        // SAFETY: on success result is a fresh, uniquely-owned Slice*.
        Ok(unsafe { create(result.cast()) })
    })
}

/// 'data SlicedStack base = SlicedStack [Slice] base' (Ownership.hsc:161)
/// with the CanLookup instance at :170-180.
///
/// The C++ SlicedStack* is bracketed inside with_lookup (the source's
/// bracket, Ownership.hsc:174-179), so this Rust value owns only the
/// slices and the base.
pub struct SlicedStack<B> {
    slices: Vec<Slice>,
    base: B,
    name: String,
}

/// 'slicedStack = SlicedStack' (Ownership.hsc:163-168).
pub fn sliced_stack<B: CanLookup>(slices: Vec<Slice>, base: B) -> SlicedStack<B> {
    // Divergence 1 (header): materialise the composed name.
    let name = format!("slicedstack:{}", base.lookup_name());
    SlicedStack { slices, base, name }
}

struct SlicedStackGuard(*mut c_void);

impl Drop for SlicedStackGuard {
    fn drop(&mut self) {
        // SAFETY: self.0 is the SlicedStack* glean_make_sliced_stack
        // produced (ffi.cpp:1074-1076); the guard is created only on
        // success, and frees it exactly once (ffi::free_).
        unsafe { sys::glean_sliced_stack_free(self.0) }
    }
}

impl<B: CanLookup> CanLookup for SlicedStack<B> {
    fn lookup_name(&self) -> &str {
        &self.name
    }

    fn with_lookup<T, E: From<FfiError>>(
        &self,
        f: impl FnOnce(*mut c_void) -> Result<T, E>,
    ) -> Result<T, E> {
        self.base.with_lookup(|p_base| {
            let mut slice_ptrs: Vec<*mut c_void> = Vec::with_capacity(self.slices.len());
            for slice in &self.slices {
                // SAFETY: slice outlives this call; the pointer is only
                // stored for the make_sliced_stack call below and is not
                // retained past it.
                unsafe { with(slice, |p| slice_ptrs.push(p.cast())) };
            }
            let mut stack: *mut c_void = ptr::null_mut();
            // SAFETY: p_base is the live Lookup* the enclosing
            // with_lookup guarantees; slice_ptrs holds slice_ptrs.len()
            // live Slice*; stack is an initialised stack out-param.
            unsafe {
                invoke(|| {
                    sys::glean_make_sliced_stack(
                        p_base,
                        slice_ptrs.len(),
                        slice_ptrs.as_ptr(),
                        &mut stack,
                    )
                })
            }?;
            // The source's bracket: free on every exit path from here.
            let _guard = SlicedStackGuard(stack);
            f(stack)
        })
    }
}

// ---------------------------------------------------------------------------
// DefineOwnership operations
// ---------------------------------------------------------------------------

/// 'newDefineOwnership ownership (Fid first_id)' (Ownership.hsc:189-193).
pub fn new_define_ownership(
    ownership: &Ownership,
    first_id: Fid,
) -> Result<DefineOwnership, FfiError> {
    // SAFETY: ownership is a live handle for the call.
    unsafe {
        with(ownership, |ownership_ptr| {
            let mut result: *mut c_void = ptr::null_mut();
            // SAFETY: ownership_ptr is the live Ownership*; result is an
            // initialised stack out-param; the body is 'new
            // DefineOwnership' (ffi.cpp:1088-1089).
            invoke(|| {
                sys::glean_new_define_ownership(ownership_ptr.cast(), first_id.0, &mut result)
            })?;
            // SAFETY: on success result is a fresh, uniquely-owned
            // DefineOwnership*; the ? returned early otherwise.
            Ok(create(result.cast()))
        })
    }
}

/// 'substDefineOwnership define subst' (Ownership.hsc:195-199).
pub fn subst_define_ownership(define: &DefineOwnership, subst: &Subst) -> Result<(), FfiError> {
    // SAFETY: define is a live handle for the call; subst.as_ptr() is
    // its live Substitution*, borrowed (never freed here).
    unsafe {
        with(define, |define_ptr| {
            invoke(|| sys::glean_define_ownership_subst(define_ptr.cast(), subst.as_ptr()))
        })
    }
}

/// 'addDerivedOwners base define (Pid pid) deps' (Ownership.hsc:201-234).
///
/// The Haskell's 'when (not $ null deps)' is the early return; the
/// four parallel arrays are the unzip4 of (facts ptr, facts len, deps
/// ptr, deps len) per FactDependencies.
pub fn add_derived_owners<B: CanLookup + ?Sized>(
    base: &B,
    define: &DefineOwnership,
    pid: Pid,
    deps: &[FactDependencies],
) -> Result<(), FfiError> {
    if deps.is_empty() {
        return Ok(());
    }

    let mut facts_ptrs: Vec<*const i64> = Vec::with_capacity(deps.len());
    let mut facts_sizes: Vec<usize> = Vec::with_capacity(deps.len());
    let mut deps_ptrs: Vec<*const i64> = Vec::with_capacity(deps.len());
    let mut deps_sizes: Vec<usize> = Vec::with_capacity(deps.len());
    for dep in deps {
        // SAFETY: each vector is owned by deps, which outlives the C call
        // below; the pointers are read-only there (the DependencyIterator
        // copies them, ffi.cpp:1126-1149).
        facts_ptrs.push(dep.facts.as_ptr());
        facts_sizes.push(dep.facts.len());
        deps_ptrs.push(dep.dependencies.as_ptr());
        deps_sizes.push(dep.dependencies.len());
    }

    // SAFETY: define is a live handle for the call.
    unsafe {
        with(define, |define_ptr| {
            base.with_lookup(|base_ptr| {
                // SAFETY: base_ptr is the live Lookup* the enclosing
                // with_lookup guarantees; define_ptr is the live
                // DefineOwnership*; the four arrays are valid for
                // deps.len() reads; pid.0 is the source's fromIntegral
                // (Int64 -> uint64_t, two's complement).
                invoke(|| {
                    sys::glean_define_ownership_add_derived(
                        base_ptr,
                        define_ptr.cast(),
                        pid.0 as u64,
                        deps.len(),
                        facts_ptrs.as_ptr(),
                        facts_sizes.as_ptr(),
                        deps_ptrs.as_ptr(),
                        deps_sizes.as_ptr(),
                    )
                })
            })
        })
    }
}

/// 'newtype DerivedFactOwnershipIterator = ... (Ptr ...)' (Ownership.hsc:
/// 237) — an opaque iterator produced by the DB layer.
#[repr(transparent)]
#[derive(Debug)]
pub struct DerivedFactOwnershipIterator(*mut c_void);

impl DerivedFactOwnershipIterator {
    /// Adopt a DerivedFactOwnershipIterator*.
    ///
    /// SAFETY: ptr must be a live iterator from the C ABI (never already
    /// freed); the returned handle owns it — destroy_static must run
    /// exactly once (use crate::ffi::using).
    pub unsafe fn from_raw(ptr: *mut c_void) -> DerivedFactOwnershipIterator {
        DerivedFactOwnershipIterator(ptr)
    }

    /// Borrowed raw pointer for the compute extern.
    pub fn as_ptr(&self) -> *mut c_void {
        self.0
    }
}

impl Static for DerivedFactOwnershipIterator {
    fn destroy_static(&self) {
        // SAFETY: self.0 is the live iterator adopted by from_raw;
        // glean_derived_fact_ownership_iterator_free (ffi.cpp:936-939,
        // ffi::free_) consumes it and cannot throw across (wrap_).
        unsafe { sys::glean_derived_fact_ownership_iterator_free(self.0) }
    }
}

/// 'computeDerivedOwnership ownership base iter' (Ownership.hsc:244-254).
pub fn compute_derived_ownership<B: CanLookup + ?Sized>(
    ownership: &Ownership,
    base: Option<&B>,
    iter: &DerivedFactOwnershipIterator,
) -> Result<ComputedOwnership, FfiError> {
    // SAFETY: ownership is a live handle for the call.
    unsafe {
        with(ownership, |ownership_ptr| match base {
            None => derived_compute_with(ownership_ptr, ptr::null_mut(), iter),
            Some(base) => {
                base.with_lookup(|base_ptr| derived_compute_with(ownership_ptr, base_ptr, iter))
            }
        })
    }
}

fn derived_compute_with(
    ownership: *mut Ownership,
    base: *mut c_void,
    iter: &DerivedFactOwnershipIterator,
) -> Result<ComputedOwnership, FfiError> {
    let mut result: *mut c_void = ptr::null_mut();
    // SAFETY: ownership is the live Ownership* the enclosing with
    // guarantees; base is a live Lookup* or null (the source's nullPtr);
    // iter.as_ptr() is the live iterator borrowed for the call; result is
    // an initialised stack out-param.
    unsafe {
        invoke(|| {
            sys::glean_derived_ownership_compute(ownership.cast(), base, iter.as_ptr(), &mut result)
        })?;
        // SAFETY: on success result is a fresh, uniquely-owned
        // ComputedOwnership* (ffi.cpp:1191-1192).
        Ok(create(result.cast()))
    }
}

// ---------------------------------------------------------------------------
// HsArray marshalling (the kit's C++ container)
// ---------------------------------------------------------------------------

/// Layout of the kit's HsArray<T> (HsStruct.h:444-451): std::vector<T>
/// followed by the cached data pointer and size. See divergence 3.
#[repr(C)]
struct HsArrayView<T> {
    /// std::vector<T> is three pointers in both libstdc++ and libc++.
    vector: [usize; 3],
    a: *const T,
    n: usize,
}

const _: () = assert!(std::mem::size_of::<HsArrayView<u8>>() == 5 * std::mem::size_of::<usize>());

/// Copy the elements out of a constructed HsArray (the Haskell's
/// 'hsArrayStorable <$> peek' — Ownership.hsc:267, :293).
///
/// SAFETY: p must point at a constructed HsArray<T> whose a/n fields are
/// valid for n reads (the kit's invariant after construction/assignment).
unsafe fn copy_hs_array<T: Copy>(p: *mut HsArrayView<T>) -> Vec<T> {
    // SAFETY: p is a constructed HsArray per this fn's contract; a/n were
    // written by the C++ constructor/assignment.
    let view = unsafe { &*p };
    // Divergence 7: a constructed HsArray never has n > 0 with a null
    // data pointer; catch a layout regression in debug builds instead of
    // silently returning an empty vector.
    debug_assert!(view.n == 0 || !view.a.is_null(), "corrupt HsArray view");
    if view.n == 0 {
        Vec::new()
    } else {
        // SAFETY: a is valid for n elements (the HsArray invariant).
        unsafe { std::slice::from_raw_parts(view.a, view.n) }.to_vec()
    }
}

/// Destructor guard for the stack-allocated HsArray<int64_t> of
/// define_ownership_sort_by_owner (the Haskell's withDefaultCxxObject
/// bracket, Foreign/CPP/HsStruct/Utils.hs:35-43).
struct Int64ArrayGuard(*mut HsArrayView<i64>);

impl Drop for Int64ArrayGuard {
    fn drop(&mut self) {
        // SAFETY: self.0 is the stack object constructed by
        // constructible_constructDefaultHsArrayInt64; destruct is the
        // matching placement-destructor call (Destructible.h:29-31).
        unsafe { sys::destructible_destructHsArrayInt64(self.0.cast()) }
    }
}

/// Deleter guard for the heap HsArray<uint32_t> returned by
/// get_ownership_set (the Haskell's 'delete arr_ptr',
/// Ownership.hsc:286-288).
struct UInt32ArrayGuard(*mut HsArrayView<u32>);

impl Drop for UInt32ArrayGuard {
    fn drop(&mut self) {
        // SAFETY: self.0 is the HsArray* the C++ allocated with new
        // (ffi.cpp:1026); delete is the matching destructor +
        // operator delete (Destructible.h:32-34).
        unsafe { sys::destructible_deleteHsArrayUInt32(self.0.cast()) }
    }
}

/// 'defineOwnershipSortByOwner define count order' (Ownership.hsc:256-270).
///
/// count is the Haskell Int (i64 here, passed as uint64_t facts via the
/// same fromIntegral bit copy); order is the caller's array; the result
/// is the sorted order.
pub fn define_ownership_sort_by_owner(
    define: &DefineOwnership,
    count: i64,
    order: &[i64],
) -> Result<Vec<i64>, FfiError> {
    // SAFETY: define is a live handle for the call.
    unsafe {
        with(define, |define_ptr| {
            let mut arr = MaybeUninit::<HsArrayView<i64>>::uninit();
            let arr_ptr = arr.as_mut_ptr();
            // SAFETY: arr_ptr is stack storage of the right size and
            // alignment; constructDefault placement-news an empty
            // HsArray (Constructible.h:28-30), exactly the Haskell's
            // withDefaultCxxObject.
            sys::constructible_constructDefaultHsArrayInt64(arr_ptr.cast());
            let _guard = Int64ArrayGuard(arr_ptr);
            // SAFETY: define_ptr is live; order is valid for order.len()
            // reads (the body copies it, ffi.cpp:1105-1106); arr_ptr is
            // the constructed HsArray the body assigns to.
            invoke(|| {
                sys::glean_define_ownership_sort_by_owner(
                    define_ptr.cast(),
                    count as u64,
                    order.as_ptr(),
                    order.len(),
                    arr_ptr.cast(),
                )
            })?;
            // SAFETY: the call assigned an HsArray; copy_hs_array's
            // contract is satisfied by that assignment.
            Ok(copy_hs_array(arr_ptr))
        })
    }
}

// ---------------------------------------------------------------------------
// Fact owner / ownership sets
// ---------------------------------------------------------------------------

/// 'getFactOwner lookup (Fid fid)' (Ownership.hsc:272-278).
pub fn get_fact_owner<A: CanLookup + ?Sized>(
    lookup: &A,
    fid: Fid,
) -> Result<Option<UsetId>, FfiError> {
    lookup.with_lookup(|p_lookup| {
        let mut uset_id: u32 = 0;
        // SAFETY: p_lookup is the live Lookup* the enclosing with_lookup
        // guarantees; uset_id is an initialised stack out-param.
        unsafe { invoke(|| sys::glean_get_fact_owner(p_lookup, fid.0, &mut uset_id)) }?;
        // The source compares against the hsc2hs #const (Ownership.hsc:
        // 274-276): INVALID_USET means "no owner", not an error.
        if uset_id == INVALID_USET {
            Ok(None)
        } else {
            Ok(Some(UsetId(uset_id)))
        }
    })
}

/// 'data SetOp = Or | And' (Ownership.hsc:280-281; C++ Or == 0, And == 1
/// at ownership/uset.h:31). The source's derived Enum is represented by
/// the repr(i32) discriminants; no toEnum consumer exists in the tree.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SetOp {
    Or = 0,
    And = 1,
}

/// 'getOwnershipSet ownership usetid' (Ownership.hsc:283-298).
///
/// Bracket: the C++ returns a heap HsArray or null; null is Nothing, and
/// the array is deleted on every exit path (the source's bracket).
pub fn get_ownership_set(
    ownership: &Ownership,
    uset_id: UsetId,
) -> Result<Option<(SetOp, Vec<UsetId>)>, FfiError> {
    // SAFETY: ownership is a live handle for the call.
    unsafe {
        with(ownership, |ownership_ptr| {
            let mut op: c_int = 0;
            let mut arr: *mut c_void = ptr::null_mut();
            // SAFETY: ownership_ptr is live; op/arr are initialised stack
            // out-params; the body sets arr to null for a missing set
            // (ffi.cpp:1020-1022).
            invoke(|| {
                sys::glean_get_ownership_set(ownership_ptr.cast(), uset_id.0, &mut op, &mut arr)
            })?;
            if arr.is_null() {
                return Ok(None);
            }
            let arr = arr.cast::<HsArrayView<u32>>();
            let _guard = UInt32ArrayGuard(arr);
            // SAFETY: arr is the heap HsArray the body constructed
            // (ffi.cpp:1026); the guard above keeps it alive until after
            // the copy.
            let values = copy_hs_array(arr);
            let op = match op {
                0 => SetOp::Or,
                1 => SetOp::And,
                // RULE §2 partial-function row: the source's
                // 'error "unknown SetOp"' (Ownership.hsc:296), verbatim.
                _ => panic!("unknown SetOp"),
            };
            // PERF(port): the Haskell's unsafeCoerceVector (Word32 ->
            // UsetId) is zero-copy; the repr(transparent) map allocates.
            Ok(Some((op, values.into_iter().map(UsetId).collect())))
        })
    }
}

// ---------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------

/// 'data OwnershipStats = OwnershipStats { ... }' (Ownership.hsc:300-308),
/// mirroring ownership.h:61-71 exactly. The Haskell's Storable instance
/// reads the same seven fields via hsc2hs; here the C++ writes the whole
/// struct into our out-param (ffi.cpp:1196-1198), so a repr(C) struct is
/// the direct image (see the header's hsc2hs note).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OwnershipStats {
    pub num_units: u64,
    pub units_size: usize,
    pub num_sets: u64,
    pub sets_size: usize,
    pub num_owner_entries: u64,
    pub owners_size: usize,
    /// < 0 means "don't know yet" (ownership.h:71).
    pub num_orphan_facts: i64,
}

/// 'showOwnershipStats' (Ownership.hsc:310-320).
pub fn show_ownership_stats(stats: &OwnershipStats) -> String {
    let mut out = format!(
        "{} units ({}), {} sets ({}), {} owners ({})",
        stats.num_units,
        render_bytes(stats.units_size as i64),
        stats.num_sets,
        render_bytes(stats.sets_size as i64),
        stats.num_owner_entries,
        render_bytes(stats.owners_size as i64),
    );
    if stats.num_orphan_facts >= 0 {
        out.push_str(&format!(", {} orphan facts", stats.num_orphan_facts));
    }
    out
}

/// hsthrift Util.PrettyPrint.renderBytes (PrettyPrint.hs:58-72), inlined
/// (header divergence 4): the same four branches, the same strict >
/// comparisons, and the same lowercase "kiB".
fn render_bytes(b: i64) -> String {
    let bf = b as f64;
    let kb = 1024.0;
    let mb = 1024.0 * kb;
    let gb = 1024.0 * mb;
    if bf > gb {
        format!("{:.2} GiB", bf / gb)
    } else if bf > mb {
        format!("{:.2} MiB", bf / mb)
    } else if bf > kb {
        format!("{:.2} kiB", bf / kb)
    } else {
        format!("{} bytes", b)
    }
}

/// 'getOwnershipStats ownership' (Ownership.hsc:338-341).
pub fn get_ownership_stats(ownership: &Ownership) -> Result<OwnershipStats, FfiError> {
    // SAFETY: ownership is a live handle for the call.
    unsafe {
        with(ownership, |ownership_ptr| {
            let mut stats = MaybeUninit::<OwnershipStats>::uninit();
            // SAFETY: ownership_ptr is live; stats is stack storage of
            // the repr(C) layout; the body assigns the whole struct on
            // success (ffi.cpp:1196-1198).
            invoke(|| sys::glean_get_ownership_stats(ownership_ptr.cast(), stats.as_mut_ptr()))?;
            // SAFETY: the call succeeded, so every field was written.
            Ok(stats.assume_init())
        })
    }
}

/// 'nextUsetId ownership' (Ownership.hsc:343-346).
pub fn next_uset_id(ownership: &Ownership) -> Result<UsetId, FfiError> {
    // SAFETY: ownership is a live handle for the call.
    unsafe {
        with(ownership, |ownership_ptr| {
            let mut result: u32 = 0;
            // SAFETY: ownership_ptr is live; result is an initialised
            // stack out-param.
            invoke(|| sys::glean_ownership_next_set_id(ownership_ptr.cast(), &mut result))?;
            Ok(UsetId(result))
        })
    }
}

// ---------------------------------------------------------------------------
// Fact ownership (batch-level map)
// ---------------------------------------------------------------------------

/// 'newtype FactOwnership = FactOwnership { ownershipUnits :: HashMap
/// UnitName (Vector Id) }' (Ownership.hsc:352-355) — exactly the same
/// shape as Batch.owned in glean.thrift (UnitName is bytestring).
#[derive(Debug, Clone, Default)]
pub struct FactOwnership {
    pub ownership_units: HashMap<Vec<u8>, Vec<Id>>,
}

/// 'substOwnership subst (FactOwnership owned)' (Ownership.hsc:357-361):
/// apply the substitution to every unit's ids.
pub fn subst_ownership(
    subst: &Subst,
    ownership: &FactOwnership,
) -> Result<FactOwnership, FfiError> {
    let mut ownership_units = HashMap::with_capacity(ownership.ownership_units.len());
    for (unit, ids) in &ownership.ownership_units {
        // Divergence 6: the source's unsafeCoerceVector is zero-copy and
        // consumes the vector; this port takes &FactOwnership, so the
        // Id -> Fid map allocates a fresh Vec.
        // PERF(port): consume FactOwnership (or transmute the
        // repr(transparent) Vec) to avoid the copy.
        let fids: Vec<Fid> = ids.iter().map(|&id| Fid(id)).collect();
        let substituted = subst.unsafe_subst_intervals_and_release(fids)?;
        ownership_units.insert(
            unit.clone(),
            substituted.into_iter().map(|fid| fid.0).collect(),
        );
    }
    Ok(FactOwnership { ownership_units })
}

/// 'unionOwnership = FactOwnership . fmap VS.concat . foldr
/// (HashMap.unionWith (<>) . fmap (: []) . ownershipUnits) HashMap.empty'
/// (Ownership.hsc:363-369).
///
/// foldr + unionWith (<>) keeps the EARLIER list first on duplicate keys
/// (unionWith applies f left right, and the left map is the earlier
/// element); iterating left-to-right and extending reproduces that order.
pub fn union_ownership(ownerships: Vec<FactOwnership>) -> FactOwnership {
    let mut ownership_units: HashMap<Vec<u8>, Vec<Id>> = HashMap::new();
    for ownership in ownerships {
        for (unit, ids) in ownership.ownership_units {
            ownership_units.entry(unit).or_default().extend(ids);
        }
    }
    FactOwnership { ownership_units }
}

// ---------------------------------------------------------------------------
// ACL augmentation
// ---------------------------------------------------------------------------

/// 'computedOwnershipUnitCount own' (Ownership.hsc:374-383).
pub fn computed_ownership_unit_count(ownership: &ComputedOwnership) -> Result<u32, FfiError> {
    // SAFETY: ownership is a live handle for the call.
    unsafe {
        with(ownership, |ownership_ptr| {
            let mut result: u32 = 0;
            // SAFETY: ownership_ptr is live; result is an initialised
            // stack out-param (the body writes sets_.getFirstId(),
            // ffi.cpp:961-965).
            invoke(|| sys::glean_computed_ownership_unit_count(ownership_ptr.cast(), &mut result))?;
            Ok(result)
        })
    }
}

/// 'augmentWithACL own assignments' (Ownership.hsc:391-414): AND each
/// fact's existing owner with the CNF built from the per-unit ACL levels.
///
/// The source flattens (unitId, levels, groups) into the four parallel
/// arrays unit_ids / level_counts / group_counts / group_ids; the C++
/// rebuilds them with two running offsets (ffi.cpp:975-998).
pub fn augment_with_acl(
    ownership: &ComputedOwnership,
    assignments: &[(u32, Vec<Vec<u32>>)],
) -> Result<(), FfiError> {
    // The source's 'augmentWithACL _ [] = return ()' short-circuit.
    if assignments.is_empty() {
        return Ok(());
    }

    let mut unit_ids: Vec<u32> = Vec::with_capacity(assignments.len());
    let mut level_counts: Vec<usize> = Vec::new();
    let mut group_counts: Vec<usize> = Vec::new();
    let mut group_ids: Vec<u32> = Vec::new();
    for (unit, levels) in assignments {
        unit_ids.push(*unit);
        level_counts.push(levels.len());
        for level in levels {
            group_counts.push(level.len());
            group_ids.extend_from_slice(level);
        }
    }

    // SAFETY: ownership is a live handle for the call.
    unsafe {
        with(ownership, |ownership_ptr| {
            // SAFETY: ownership_ptr is live; the four vectors are valid
            // for the counts passed (unit_ids.len() units, the two count
            // arrays parallel to them, group_ids.len() groups); the body
            // only reads them (ffi.cpp:975-998).
            invoke(|| {
                sys::glean_augment_ownership_with_acl(
                    ownership_ptr.cast(),
                    unit_ids.as_ptr(),
                    level_counts.as_ptr(),
                    unit_ids.len(),
                    group_counts.as_ptr(),
                    group_ids.as_ptr(),
                )
            })
        })
    }
}

// ---------------------------------------------------------------------------
// Process-global knobs (forwarded to the C++ statics)
// ---------------------------------------------------------------------------

// TODO(port): no Rust consumer reads or writes these knobs yet. The
// Haskell tests do (IncrementalTest.hs:852-858), but tests are not
// translated (RULE §4); the process-global state stays in the C++ engine
// and the inventory row's "who owns the settings" question remains open
// for the inventory owner (inventory.tsv row 71 / inventory.ffi.tsv row
// 31).
/// 'setOwnershipCompactThreshold' (Ownership.hsc:542-543).
pub fn set_ownership_compact_threshold(threshold: u64) {
    // SAFETY: the symbol takes a plain uint64_t and has no failure mode
    // (ffi.cpp:1200-1202).
    unsafe { sys::glean_set_ownership_compact_threshold(threshold) }
}

/// 'getOwnershipCompactThreshold' (Ownership.hsc:545-546).
pub fn get_ownership_compact_threshold() -> u64 {
    // SAFETY: pure getter over C++ state (ffi.cpp:1204-1206).
    unsafe { sys::glean_get_ownership_compact_threshold() }
}

/// 'setOwnershipMergeCacheSize' (Ownership.hsc:554-555).
pub fn set_ownership_merge_cache_size(size: u64) {
    // SAFETY: the symbol takes a plain uint64_t and has no failure mode
    // (ffi.cpp:1208-1210).
    unsafe { sys::glean_set_ownership_merge_cache_size(size) }
}

/// 'getOwnershipMergeCacheSize' (Ownership.hsc:557-558).
pub fn get_ownership_merge_cache_size() -> u64 {
    // SAFETY: pure getter over C++ state (ffi.cpp:1212-1214).
    unsafe { sys::glean_get_ownership_merge_cache_size() }
}

// ---------------------------------------------------------------------------
// Raw C ABI
// ---------------------------------------------------------------------------

/// Raw C-ABI declarations for the glean_ownership_* / slice_* /
/// define_ownership_* groups plus the hsthrift HsArray kit symbols.
///
/// RULE §1 (FFI row, A2): raw extern "C" for flat C-ABI symbols.
/// Signatures are ABI-identical to ffi.h, with the const/signedness
/// narrowing noted per symbol: unit_ids and order and the four add_derived
/// arrays are only read by the bodies (uint32_t* -> *const u32,
/// int64_t*/uint64_t** -> *const i64), and the opaque handles are
/// *mut c_void (the sibling lookup.rs/subst.rs convention).
///
/// The three HsArray symbols are NOT in glean/rts/ffi.h: they are the
/// hsthrift kit's own externs (Destructible.h:28-33, Constructible.h:
/// 27-32) that the Haskell's derived instances bind (Types.hsc:792-794,
/// :800-801); they are declared here because this unit is their only
/// consumer.
pub mod sys {
    use std::ffi::c_void;
    use std::os::raw::{c_char, c_int};

    use super::OwnershipStats;

    // NOTE (RULE §3, family convention): the unsafe extern "C" block
    // below is a DECLARATION, not an operation — the same ruling subst.rs
    // and bytecode.rs state for theirs. It introduces no unsafe behaviour
    // of its own, carries no SAFETY comment, and is NOT counted in the
    // SAFETY tally at the foot of this file; every USE below carries its
    // own.
    unsafe extern "C" {
        // ffi.h:429 — void; body ffi.cpp:932-934 is ffi::free_.
        pub fn glean_ownership_unit_iterator_free(iter: *mut c_void);
        // ffi.h:431 — void; body ffi.cpp:936-939 is ffi::free_.
        pub fn glean_derived_fact_ownership_iterator_free(iter: *mut c_void);
        // ffi.h:433-438 — allocation-class only (computeOwnership +
        // release); no precondition check.
        pub fn glean_ownership_compute(
            inventory: *mut c_void,
            lookup: *mut c_void,
            iter: *mut c_void,
            base_lookup: *mut c_void,
            result: *mut *mut c_void,
        ) -> *const c_char;
        // ffi.h:440 — void; body ffi.cpp:953-955 is ffi::free_.
        pub fn glean_ownership_free(own: *mut c_void);
        // ffi.h:442 — void; body ffi.cpp:957-959 is ffi::free_.
        pub fn glean_computed_ownership_free(own: *mut c_void);
        // ffi.h:444 — pure nextSetId read.
        pub fn glean_ownership_next_set_id(
            ownership: *mut c_void,
            result: *mut u32,
        ) -> *const c_char;
        // ffi.h:446-450 — a missing set writes a null result (not an
        // error); the array is heap-allocated with new.
        pub fn glean_get_ownership_set(
            ownership: *mut c_void,
            uset_id: u32,
            op: *mut c_int,
            result: *mut *mut c_void,
        ) -> *const c_char;
        // ffi.h:452-453 — pure lookup read; INVALID_USET is a value.
        pub fn glean_get_fact_owner(
            lookup: *mut c_void,
            fact: i64,
            uset_id: *mut u32,
        ) -> *const c_char;
        // ffi.h:468-475 — the body copies unit_ids into a local vector
        // (ffi.cpp:1040) and reads the bases array, so both are const
        // here (ABI-identical).
        pub fn glean_slice_compute(
            ownership: *mut c_void,
            unit_ids: *const u32,
            unit_ids_size: usize,
            exclude: c_int,
            bases: *const *mut c_void,
            num_bases: usize,
            result: *mut *mut c_void,
        ) -> *const c_char;
        // ffi.h:477-478 — the body allocates a clone and releases it to
        // the caller (ffi.cpp:1051-1053).
        pub fn glean_slice_serialize(
            slice: *mut c_void,
            data: *mut *const c_void,
            size: *mut usize,
        ) -> *const c_char;
        // ffi.h:480-481 — PRECONDITION-GUARD: the body only reads the
        // input bytes, but Slice::deserialize's serialize::get throws on
        // truncated/malformed input (slice.cpp:276-284).
        pub fn glean_slice_deserialize(
            data: *const c_void,
            size: usize,
            slice: *mut *mut c_void,
        ) -> *const c_char;
        // ffi.h:483 — void; body ffi.cpp:1065-1067 is ffi::free_.
        pub fn glean_slice_free(slice: *mut c_void);
        // ffi.h:485-489 — allocates a Sliced<Slices>; the slices array is
        // only read (ffi.cpp:1074-1076).
        pub fn glean_make_sliced_stack(
            lookup: *mut c_void,
            count: usize,
            slices: *const *mut c_void,
            sliced: *mut *mut c_void,
        ) -> *const c_char;
        // ffi.h:491 — void; body ffi.cpp:1080-1082 is ffi::free_.
        pub fn glean_sliced_stack_free(sliced: *mut c_void);
        // ffi.h:493-496 — allocation-class only (new DefineOwnership).
        pub fn glean_new_define_ownership(
            own: *mut c_void,
            first_id: i64,
            result: *mut *mut c_void,
        ) -> *const c_char;
        // ffi.h:498-500 — define->subst; allocation-class only.
        pub fn glean_define_ownership_subst(
            define: *mut c_void,
            subst: *const c_void,
        ) -> *const c_char;
        // ffi.h:502-507 — order is copied into a local vector
        // (ffi.cpp:1105-1106); result is an HsArray<int64_t>* (FactOrder).
        pub fn glean_define_ownership_sort_by_owner(
            define: *mut c_void,
            facts: u64,
            order: *const i64,
            size: usize,
            result: *mut c_void,
        ) -> *const c_char;
        // ffi.h:509-517 — the DependencyIterator only reads the four
        // arrays (ffi.cpp:1126-1149).
        pub fn glean_define_ownership_add_derived(
            lookup: *mut c_void,
            define: *mut c_void,
            pid: u64,
            fids_size: usize,
            facts_lists: *const *const i64,
            facts_lists_sizes: *const usize,
            deps_lists: *const *const i64,
            deps_lists_sizes: *const usize,
        ) -> *const c_char;
        // ffi.h:519 — void; body ffi.cpp:1182-1184 is ffi::free_.
        pub fn glean_define_ownership_free(define: *mut c_void);
        // ffi.h:521-525 — allocation-class only (computeDerivedOwnership
        // + release).
        pub fn glean_derived_ownership_compute(
            own: *mut c_void,
            base_lookup: *mut c_void,
            iter: *mut c_void,
            result: *mut *mut c_void,
        ) -> *const c_char;
        // ffi.h:527 — the body assigns the whole struct (ffi.cpp:
        // 1196-1198); no precondition check.
        pub fn glean_get_ownership_stats(
            own: *mut c_void,
            result: *mut OwnershipStats,
        ) -> *const c_char;
        // ffi.h:529-532 — plain process-global state; no error channel.
        pub fn glean_set_ownership_compact_threshold(threshold: u64);
        pub fn glean_get_ownership_compact_threshold() -> u64;
        pub fn glean_set_ownership_merge_cache_size(size: u64);
        pub fn glean_get_ownership_merge_cache_size() -> u64;
        // ffi.h:539-541 — pure sets_.getFirstId() read.
        pub fn glean_computed_ownership_unit_count(
            own: *mut c_void,
            result: *mut u32,
        ) -> *const c_char;
        // ffi.h:549-555 — PRECONDITION-GUARD: the body only reads the
        // four arrays (ffi.cpp:975-998), but augmentOwnershipWithACL
        // throws when a promoted UsetId cannot be resolved
        // (acl.cpp:174-179).
        pub fn glean_augment_ownership_with_acl(
            ownership: *mut c_void,
            unit_ids: *const u32,
            level_counts: *const usize,
            num_units: usize,
            group_counts: *const usize,
            group_ids: *const u32,
        ) -> *const c_char;

        // hsthrift kit symbols (see the module doc above). All three are
        // noexcept on the C++ side (Destructible.h:12-24,
        // Constructible.h:10-22).
        pub fn constructible_constructDefaultHsArrayInt64(p: *mut c_void);
        pub fn destructible_destructHsArrayInt64(p: *mut c_void);
        pub fn destructible_deleteHsArrayUInt32(p: *mut c_void);
    }
}

// Marker breakdown (closed set — 1 TODO(port), 0 BUG(port), 3 PERF(port),
// 2 INVENTORY-BUG-CANDIDATE):
//   * 1 TODO(port): the knobs have no Rust consumer yet — the Haskell
//     tests read them (IncrementalTest.hs:852-858) but tests are not
//     translated. See the marker above set_ownership_compact_threshold.
//     The manifest absence is answered by manifest line 57.
//   * 0 BUG(port): no source defect is reproduced. The one reproduced
//     partial function keeps its message; every quirk read on this path
//     (the C++ ownership engine's own allocation failures, the null
//     result for a missing set) is the behaviour of the C++ we CALL.
//   * 3 PERF(port): serialize_slice (the Haskell packs the malloc'd
//     buffer zero-copy through unsafePackMallocCStringLen; this port
//     copies through unsafe_malloced_byte_string), subst_ownership
//     (Id -> Fid map), and get_ownership_set (u32 -> UsetId map). The
//     HsArray -> Vec copies ARE source-mandated (the Haskell peeks and
//     copies at exactly those points).
//   * 2 INVENTORY-BUG-CANDIDATE: row 71's stale "NOT in manifest.tsv"
//     prose, and row 31's stale pre-A2/unknown status (details in the
//     header block).
// SAFETY count: 35 unsafe blocks + 13 unsafe declarations (48 unsafe
// constructs; 54 unsafe OPERATIONS in the itemized list below), with 64
// // SAFETY: comment lines — the
// two iterator destroy_static calls 2; the four Object finalizer shims 4;
// Ownership::from_raw 1; compute_with (invoke + create) 2; slice (with +
// nested with + invoke + create) 4; serialize_slice (with + invoke +
// unsafe_malloced_byte_string) 3; deserialize_slice (invoke + create) 2;
// SlicedStack with_lookup (nested with + invoke) 2; SlicedStackGuard drop
// 1; new_define_ownership (with + invoke + create) 3; subst_define (with +
// invoke) 2; add_derived_owners (with + invoke) 2; derived compute (with +
// invoke + create) 3; copy_hs_array 2; Int64ArrayGuard drop 1;
// UInt32ArrayGuard drop 1; sort_by_owner (with + construct + invoke) 3;
// get_fact_owner (invoke) 1; get_ownership_set (with + invoke) 2;
// get_ownership_stats (with + invoke + assume_init) 3; next_uset_id (with
// + invoke) 2; unit_count (with + invoke) 2; augment (with + invoke) 2;
// the four knob wrappers 4. The ONE unsafe extern "C" block (plus the
// four unsafe extern "C" fn finalizer shims) is a declaration, not an
// operation, and is not counted in the 54 (subst.rs precedent).
// PORT STATUS: confidence=high todos=1
