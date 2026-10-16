// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// Define — the fact-definition surface of the Glean RTS (`Define`,
// `defineFact`, `defineBatch`).
//
// Ported from `glean/hs/Glean/RTS/Foreign/Define.hs` (135 lines).
//
// Manifest unit: rust/glean-core/src/rts/foreign/define.rs
// (migration/manifest.tsv line 28, crate glean-core). RULE §4: one source
// file -> one target file; the manifest path is authoritative.
//
// ABI truth (read, not assumed): glean/rts/ffi.h —
//   glean_define_fact   :218-223  (Define* facts,
//                                  glean_predicate_id_t predicate,
//                                  Output* clause, size_t key_size,
//                                  glean_fact_id_t* id)
//   glean_define_batch  :225-236  (Define* facts, Inventory* inventory,
//                                  int64_t batch_first_id,
//                                  const int64_t* ids, size_t batch_count,
//                                  const void* batch_facts_data,
//                                  size_t batch_facts_size,
//                                  bool trust_refs, bool ignore_redefs,
//                                  Substitution** subst)
// and their C++ bodies at glean/rts/ffi.cpp:355-369 (`glean_define_fact`)
// and :371-395 (`glean_define_batch`), over:
//   * glean/rts/define.h:20-43 — `struct Define : Lookup` with the sentinel
//     contract at :21-24 ("a new id if it's a new fact and Id::INVALID if
//     the set contains a fact with the same type and key but a different
//     value") and the `defineBatch` declaration at :35-43;
//   * glean/rts/define.cpp:20-116 — the `defineBatch` body: `error("invalid
//     base id {} in batch")` :29-31, `error("unknown fact ...")` :61-65,
//     `error("invalid reference to fact ...")` :69-75, `error("invalid fact
//     redefinition of ...")` :99-102, `error("invalid predicate id ...")`
//     :112 (the glog CHECKs at :59/:68 abort inside C++ and never cross);
//   * glean/rts/factset.h:112/:304 + factset.cpp:278-293 — `FactSet`,
//     the primary `Define` implementation under glean_define_fact: `define`
//     returns `Id::invalid()` for a same-key different-value redefinition
//     (:290-291) and raises `error("key too large: ...")` on an oversize
//     key (:279-281).
// `glean_fact_id_t`/`glean_predicate_id_t` are `typedef int64_t`
// (ffi.h:105-106), so the id and predicate parameters are plain `i64`.
//
// RULE citations:
//   §1 (FFI row, A2): raw `extern "C"` for flat C-ABI symbols.
//   §2: error-recovery rule — the full guard-kind story in the `//!` block
//     and at each function; numeric conversions (the `i64 -> CSize`
//     `fromIntegral` at Define.hs:102 kept with the same wrap semantics);
//     deriving-Show -> constructor-shaped Display (A4) for
//     InvalidRedefinition.
//   §3: `unsafe` only inside this glean-core FFI wrapper; every block
//     carries a SAFETY comment.
//   §4: one source file -> one target file; assumed-import regime for the
//     generated glean_thrift crate.
//   §5: inventory rows followed (below); one flagged nuance.
//
// INVENTORY (RULE §5) — two rows govern this file, followed:
//   * migration/inventory.tsv line 65 (row 65): "glean_define_fact /
//     glean_define_batch | invalidFid sentinel -> InvalidRedefinition
//     exception; define_batch writes owned Subst via out-param | GUARD_KIND
//     | ... define_fact -> Result<Fid, InvalidRedefinition> =
//     precondition-guard (id==invalidFid -> err, NOT allocation; RULEBOOK
//     error-recovery rule); define_batch returns owned Subst (Drop); batch
//     ids/fact bytes borrowed for call duration; ids-count mismatch ->
//     explicit error Define.hs:110-113 | ... consumer FactSet.hs:138-141
//     (renameFacts) | confirmed".
//   * migration/inventory.ffi.tsv row 12 (line 12) — the same ruling in the
//     FFI inventory: "cxx bridge; define_fact -> Result<Fid,
//     InvalidRedefinition> = precondition-guard (id==invalidFid -> err, NOT
//     allocation; RULEBOOK error-recovery rule); define_batch returns owned
//     Subst (Drop)". (The "[SUPERSEDED by §1/A2: raw extern-C — 2026-10-07]"
//     annotation on the cxx-bridge wording is carried by inventory.tsv row
//     65's target_translation, quoted above — not by this row; corrected in
//     the units 28–31 fix round.)
//   Followed EXACTLY: `define_fact -> Result<Fid, InvalidRedefinition>`
//   (sentinel precondition-guard, not allocation); `define_batch` returns an
//   owned `Subst` (Drop = glean_free_subst); ids and fact bytes are borrowed
//   for the call duration only; the ids/count mismatch is an explicit error
//   variant (Define.hs:110-113), not a panic. One classification nuance the
//   row does not cover is flagged as
//   `// INVENTORY-BUG-CANDIDATE row 65` at `define_fact`.
//
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, module-level): outbound
// rows 1240 (Define.hs -> Glean/FFI.hs, crate::ffi), 1241 (-> RTS/Builder.hs,
// crate::rts::builder), 1242 (-> RTS/Foreign/Inventory.hs,
// crate::rts::foreign::inventory), 1243 (-> RTS/Foreign/Subst.hs,
// crate::rts::foreign::subst), 1244 (-> RTS/Types.hs, crate::rts::types) and
// 1245 (-> glean/util/Glean/Util/Vector.hs — the `unsafeCoerceVector`
// helper, which DISSOLVES here: Rust's `*const i64` is already the Fid
// layout, no coercion is needed). Inbound rows 485
// (db/Glean/Database/Write/Batch.hs), 1236 (RTS/Foreign/Benchmarking.hs),
// 1247 (RTS/Foreign/FactSet.hs — renameFacts, the row's cited consumer),
// 1275 (RTS/Foreign/Query.hsc), 1280 (RTS/Foreign/Stacked.hs — the
// `CanDefine` import family, unit 30), 1762
// (tools/gleancli/GleanCLI/Merge.hs), 1835
// (typed/Glean/Typed/BuildFact.hs — the sole defineFact call site) and 1874
// (write/Glean/Write/SendAndRebaseQueue.hs). (Inbound list completed in the
// units 28–31 fix round from edges.tsv.)
// Plus the thrift-generated `Glean.Types` import (Define.hs:36, the
// `Batch` type), whose Rust home is the generated `glean_thrift` crate.
//
// SIBLING-UNIT CONTRACTS (RULE §4; DONE crates'/units' real APIs read,
// never re-declared):
//   * crate::ffi (DONE, read): `invoke` (the pinned form
//     `pub unsafe fn invoke(call: impl FnOnce() -> *const c_char)
//     -> Result<(), FfiError>`, ffi.rs:394-402 — the extern call passed AS
//     the closure, which is why no hsthrift-style out-param machinery is
//     declared here) and `FfiError` (ffi.rs:316-320). The alloc-guard
//     helper is NOT in crate::ffi: the family convention is a private
//     per-module `allocation_guard_abort` (builder.rs:215-218,
//     inventory.rs:361-364, subst.rs:326-329) — one is defined below with a
//     symbol-specific doc comment. `foreign import ccall unsafe
//     glean_define_fact` (Define.hs:116-122) and `ccall safe
//     glean_define_batch` (:124-135) both map onto this single `invoke`.
//   * crate::rts::builder (DONE, read): `Builder` with the public
//     `Builder::as_ptr(&self) -> *mut sys::Output` marshalling seam
//     (builder.rs:186-188; its header names THIS unit as a consumer). The
//     extern below types the `Output*` clause as `*mut builder::sys::Output`
//     — the shipped sibling externs' spelling (rts.rs:345-347,
//     set.rs:212-215), which is the same ABI as the unit brief's
//     `*mut c_void` with no cast at the call site. The pointer is borrowed:
//     `define_fact` must not free or retain it (builder.rs's contract).
//   * crate::rts::foreign::inventory (DONE, read): `Inventory`, the owned
//     handle (Drop = glean_inventory_free). Its shipped surface had NO
//     raw-pointer accessor, and `glean_define_batch` takes the `Inventory*`
//     (ffi.h:226) — the same marshalling-seam gap `Builder::as_ptr` fills
//     for the builder family and `Lookup::ptr` (lookup.rs:77-79) for the
//     lookup family. A minimal `Inventory::as_ptr` accessor (safe,
//     borrowed, mirroring `Builder::as_ptr`) was ADDED to inventory.rs by
//     this unit; it is an additive impl block, not a re-declaration.
//   * crate::rts::foreign::subst (DONE, read): `Subst`, the owned handle
//     (Drop = glean_free_subst). define_batch's returned `Substitution*`
//     must be ADOPTED, and the shipped surface had no constructor from a
//     raw pointer (the `ptr` field is private to subst.rs — an inherent
//     impl in this file could not touch it). A minimal `Subst::from_raw`
//     (unsafe; the adoption contract) was ADDED to subst.rs by this unit.
//     subst.rs's own header already names `glean_define_batch`'s
//     `Substitution**` out-param as the reason its `pub mod sys` is public
//     (subst.rs:104-109); this is that consumer landing.
//   * crate::rts::types (DONE, read): `Fid` (repr(transparent) over the
//     thrift `Id` = i64, types.rs:153-155 — the by-value FFI payload),
//     `Pid` (repr(transparent) over i64, types.rs:120-122) and
//     `invalid_fid()` (types.rs:166-169 = `Fid(0)`, the source's
//     `invalidFid`). The Haskell's `Define` newtype `deriving(Storable)`
//     (:39-41) bought by-value pointer marshalling; Rust's handle is passed
//     as the raw `*mut c_void` value through `CanDefine` instead.
//   * glean_thrift (generated; see the ASSUMED-IMPORT block below):
//     `Batch` — imported, never re-declared.
//   * Why `pub mod sys` is public — for sibling consumers declaring their
//     own externs over the same opaque handles: future units take `Define*`
//     out of their own symbols (FactSet.hs's `glean_factset_define`,
//     Benchmarking.hs's `define_each`) and construct this module's `Define`
//     via `Define::new`, and Query.hsc's define step goes through
//     `defineBatch` itself.
//
// SOURCE-ABI DIVERGENCE (RULE §0, noted): the Haskell foreign import types
// `trustRefs`/`ignoreRedef` as `CBool` (Define.hs:132-133, produced by
// `fromBool`, Define.hs:105-106), while the C++ declares `bool` (ffi.h:233-234,
// ffi.cpp:379-380). GHC's `CBool` is CInt-sized (4 bytes) against C++'s
// 1-byte `bool`; the mismatch happens to work because the values are 0/1
// and cross in the low byte of the argument register. It is a Haskell-side
// binding bug; the Rust declaration uses `bool` (1 byte, C99 `_Bool`
// compatible), matching the C++ EXACTLY — deliberately not reproduced
// (there is no observable behaviour to preserve, only an ABI accident).

//! C++ RTS fact-definition surface — define one fact (`define_fact`) or a
//! whole thrift `Batch` (`define_batch`) into a `Define`.
//!
//! Port notes: RULE §3 — `unsafe` appears only inside this glean-core FFI
//! wrapper, where the Haskell was already FFI; every block carries a SAFETY
//! comment. Error-recovery classification per RULE §2 (error-recovery
//! rule), told as one story with the C++ bodies:
//!   * `glean_define_fact` — the invalid-id SENTINEL is the guard
//!     (Define.hs:59-60): `Define::define` returns `Id::invalid()` for a
//!     same-key different-value redefinition (define.h:21-24), a
//!     PRECONDITION-GUARD surfaced as `Err(InvalidRedefinition)`. The invoke
//!     Err channel (allocation-class, plus the `FactSet::define`
//!     key-too-large precondition read at factset.cpp:279-281) is aborted
//!     on — see `define_fact` and the `INVENTORY-BUG-CANDIDATE row 65`
//!     flag.
//!   * `glean_define_batch` — PRECONDITION-GUARD: `defineBatch`
//!     (define.cpp:20-116) raises structural errors ("invalid base id",
//!     "unknown fact ...", "invalid reference to fact ...", "invalid fact
//!     redefinition ...", "invalid predicate id") that cross the wrap
//!     channel as values; surfaced as `Err(DefineBatchError::Ffi)`, the
//!     Haskell's thrown `FFIError` made explicit. The ids/count mismatch is
//!     a Haskell-side guard (Define.hs:110-113) surfaced as
//!     `Err(DefineBatchError::Mismatch)` BEFORE any FFI call is made.
//!   * The returned `Subst` is the C++-allocated `Substitution`, adopted
//!     (the row: "define_batch returns owned Subst (Drop)"); the batch's
//!     ids and fact bytes are borrowed from the caller's `Batch` for the
//!     call duration only.

use std::os::raw::c_void;
use std::ptr;

use glean_thrift::Batch;

use crate::ffi::{invoke, FfiError};
use crate::rts::builder::Builder;
use crate::rts::foreign::inventory::Inventory;
use crate::rts::foreign::subst::Subst;
use crate::rts::types::{invalid_fid, Fid, Pid};

// ASSUMED-IMPORT (RULE §4 draft stub policy / assumed-imports — the same
// regime types.rs and inventory.rs already use for this crate):
// `glean_thrift` is the build.rs-generated pinned crate (RULE §1 thrift
// row: "generated by build.rs from if/*.thrift, never hand-translated")
// and does not exist in this draft tree yet. The thrift definition is
// authoritative (glean/if/glean.thrift:159-214):
//   struct Batch {
//     1: Id firstId;                 // -> first_id: Id   (i64)
//     2: i64 count;                  // -> count: i64
//     3: binary facts;               // -> facts: Vec<u8>
//     4: optional list_i64 ids;      // -> ids: Option<Vec<i64>>
//     // 5 owned / 6 dependencies / 7 schema_id: present in the struct,
//     // unused by Define.hs (and unused here).
//   }
// The full spellings assumed here are the generated snake_case form
// (`first_id`, `count`, `facts`, `ids`); the Haskell reads the same fields
// through the hsthrift accessors `Thrift.batch_firstId` (Define.hs:100),
// `Thrift.batch_count` (:102,:110), `Thrift.batch_facts` (:96) and
// `Thrift.batch_ids` (:109).

/// Raw C-ABI declarations for glean_define_fact / glean_define_batch
/// (glean/rts/ffi.h:218-236).
///
/// RULE §1 (FFI row, A2): raw `extern "C"` for flat C-ABI symbols.
///
/// Signatures mirror ffi.h EXACTLY, with the family's handle modelling:
/// `Define*`/`Inventory*`/`Substitution**` are opaque `c_void` pointers
/// (never dereferenced on this side — the inventory.rs/subst.rs/lookup.rs
/// convention); the builder's `Output*` clause is the sibling-typed
/// `*mut builder::sys::Output` (rts.rs:345-347, set.rs:212-215); `size_t`
/// -> `usize` (A5 CSize row, unsigned both sides); the id parameters are
/// `i64`/`*const i64`/`*mut i64` (`glean_fact_id_t` = int64_t, ffi.h:105).
/// The `bool` parameters are documented at the block (SOURCE-ABI DIVERGENCE
/// note in the header). Guard kinds per RULE §2 are recorded at each
/// symbol; the actual dispositions live at the call sites below.
pub mod sys {
    use std::os::raw::{c_char, c_void};

    use crate::rts::builder::sys::Output;

    // NOTE (RULE §3, family convention): the `unsafe extern "C"` block
    // below is a DECLARATION, not an operation — the same ruling subst.rs
    // and set.rs state for theirs. It introduces no unsafe behaviour of its
    // own, carries no SAFETY comment, and is NOT counted in the SAFETY tally
    // at the foot of this file; every USE below carries its own.
    unsafe extern "C" {
        // ffi.h:218-223 — `id` is the `glean_fact_id_t*` out-param; body
        // ffi.cpp:355-369 (over Define::define, define.h:29-30; the
        // FactSet implementation at factset.cpp:278-293). `assert(key_size
        // <= clause->size())` (ffi.cpp:362) aborts inside C++ in debug
        // builds and never crosses as a value. PRECONDITION-GUARD via the
        // `Id::invalid()` sentinel (returned, not raised) — see
        // `define_fact`; the error slot carries allocation-class failures
        // plus the key-too-large `error()` (flagged there).
        pub fn glean_define_fact(
            facts: *mut c_void,
            predicate: i64,
            clause: *mut Output,
            key_size: usize,
            id: *mut i64,
        ) -> *const c_char;

        // ffi.h:225-236 — `subst` is the `Substitution**` out-param; body
        // ffi.cpp:371-395 (`*subst = new Substitution(defineBatch(...))`)
        // over define.cpp:20-116. PRECONDITION-GUARD (the structural
        // `error()` sites listed in the ABI-truth block) plus whatever
        // allocation-class error crosses wrap.h — surfaced by the caller,
        // not aborted. `ids` is `*const i64`: `glean_fact_id_t` = int64_t
        // and the body only reads (define.cpp:108-110). `trust_refs` /
        // `ignore_redefs` are `bool` per the C++ EXACTLY — NOT the
        // Haskell's CBool (SOURCE-ABI DIVERGENCE in the header).
        pub fn glean_define_batch(
            facts: *mut c_void,
            inventory: *mut c_void,
            batch_first_id: i64,
            ids: *const i64,
            batch_count: usize,
            batch_facts_data: *const c_void,
            batch_facts_size: usize,
            trust_refs: bool,
            ignore_redefs: bool,
            subst: *mut *mut c_void,
        ) -> *const c_char;
    }
}

/// `newtype Define = Define (Ptr Define)` (Define.hs:39-41) — "A reference
/// to a thing we can define facts in".
///
/// BORROWED handle. The Haskell wraps a bare GHC `Ptr` (not a ForeignPtr —
/// no finalizer attached); the C++ `Define` is owned by whoever created it
/// (in this tree a `FactSet`, via `glean_factset_define`; or a Stacked view
/// over a base and an added set — FactSet.hs:138-141 passes
/// `stacked base added`). This struct therefore has NO `Drop` and never
/// frees the pointer — the caller keeps ownership, exactly as the Haskell
/// `Define` does.
///
/// `deriving(Storable)` note: the Haskell got the by-value pointer
/// marshalling for free from the newtype; here the pointer is likewise the
/// value (`CanDefine` hands out the raw `*mut c_void`), and this wrapper
/// exists to give `CanDefine` its concrete implementing type.
///
/// Not `Send`/`Sync`/`Clone`/`Copy` (conservative representation): the
/// underlying Define (a FactSet or a stacked view) is mutable state, and
/// the Haskell's Storable newtype made no sharing claim beyond what GHC
/// happened to permit. `Builder` takes the same posture (builder.rs:168-172).
pub struct Define {
    /// The borrowed `Define*`. Never null for a value this crate
    /// constructs; never dereferenced on the Rust side (only handed back to
    /// the C functions in `sys`).
    ptr: *mut c_void,
}

impl Define {
    /// Wrap a raw `Define*` — the door a sibling unit uses to adopt the
    /// pointer its own symbol produced (the future FactSet unit's
    /// `glean_factset_define` returns `Define*`). BORROWED: the caller keeps
    /// ownership; this type will never free the pointer.
    pub fn new(ptr: *mut c_void) -> Self {
        Define { ptr }
    }

    /// Get the raw pointer to the C++ `Define` (the `Lookup::ptr` precedent,
    /// lookup.rs:77-79). Borrowed: valid only while the owner keeps it
    /// alive, and never to be freed by the caller.
    pub fn ptr(&self) -> *mut c_void {
        self.ptr
    }
}

/// `class CanDefine a where withDefine :: a -> (Define -> IO b) -> IO b`
/// (Define.hs:43-45) — "Class of things we can define facts in".
///
/// Rust shape mirrors the sibling `CanLookup` (lookup.rs:114-121): the
/// callback receives the raw pointer and returns `Result<T, E>`, with `E`
/// chosen by the caller — the analogue of the Haskell's `IO b`
/// polymorphism. (Rust cannot express `Define -> IO b` as a scoped borrow
/// of a `Define` value because the externs consume the raw pointer
/// directly.)
///
/// `E: From<FfiError>` is load-bearing: stacking implementations
/// (rts/foreign/stacked.rs) reach the C++ factories through
/// `invoke(...)?`, whose error channel is `FfiError`; the `?` needs
/// `E: From<FfiError>` to merge. The bound is stated at the trait because
/// implementations may not add it themselves (E0276). Confirmed by two
/// independent reviews with rustc repros (migration fix round, units 28–31
/// batch).
pub trait CanDefine {
    /// Call `f` with the raw pointer to the C++ `Define`.
    fn with_define<T, E: From<FfiError>>(
        &self,
        f: impl FnOnce(*mut c_void) -> Result<T, E>,
    ) -> Result<T, E>;
}

/// `instance CanDefine Define where withDefine l f = f l` (Define.hs:47-48).
impl CanDefine for Define {
    fn with_define<T, E: From<FfiError>>(
        &self,
        f: impl FnOnce(*mut c_void) -> Result<T, E>,
    ) -> Result<T, E> {
        f(self.ptr)
    }
}

/// `newtype InvalidRedefinition = InvalidRedefinition String` with
/// `deriving(Show, Typeable)` and `instance Exception` (Define.hs:50-53).
///
/// The `Exception` instance becomes a `thiserror::Error` impl; the derived
/// `Show` becomes `Debug` (the port-wide stand-in), and `Display` keeps the
/// constructor-shaped tag around the carried message per RULE §2
/// (deriving-Show row / A4: `InvalidRedefinition(..)`, never invented
/// prose). The one message this module raises is the source's literal:
/// "invalid fact redefinition" (Define.hs:60).
#[derive(Debug, thiserror::Error)]
#[error("InvalidRedefinition({0:?})")]
pub struct InvalidRedefinition(pub String);

/// `data DefineFlags = DefineFlags { trustRefs :: Bool, ignoreRedef ::
/// Bool }` (Define.hs:63-75). No deriving clause in the source — no derives
/// here (RULE §0); the only instance is `Default` (below).
pub struct DefineFlags {
    /// `trustRefs` — "Whether to trust that fact references have the right
    /// type. If the facts have already been typechecked then this can be
    /// set to True to improve performance." (source doc, verbatim.)
    pub trust_refs: bool,
    /// `ignoreRedef` — "When True, if we see a redefinition of a key/value
    /// fact where the value is different, just discard the fact. When
    /// False, redefinition is an error. Redefinition may safely arise in
    /// cases where we have a cache of remote facts and the values have been
    /// rebased with respect to different caches: the values may be literally
    /// different but semantically identical." (source doc, verbatim.)
    pub ignore_redef: bool,
}

/// `instance Default DefineFlags where def = DefineFlags { trustRefs =
/// False, ignoreRedef = False }` (Define.hs:77-81).
impl Default for DefineFlags {
    fn default() -> Self {
        DefineFlags {
            trust_refs: false,
            ignore_redef: false,
        }
    }
}

/// Errors of [`define_batch`] — the Rust join of the two exception sources
/// the source function had: the `invoke` FFI channel (`FfiError`) and the
/// Haskell-side ids/count guard (the `throwIO $ Thrift.Exception "mismatch
/// between count and ids.size in batch"`, Define.hs:112-113).
///
/// The Haskell threw both; the port returns them (RULE §2 error-recovery
/// rule: the crossing C++ errors are precondition-class records, not
/// aborts — the caller decides, and SendAndRebaseQueue.hs:372-392 relies on
/// `ignoreRedef` to absorb redefinitions rather than die).
#[derive(Debug, thiserror::Error)]
pub enum DefineBatchError {
    /// The FFI channel: the C++ `defineBatch` structural errors
    /// (define.cpp:29-31, :61-65, :69-75, :99-102, :112 — "invalid base id",
    /// "unknown fact ...", "invalid reference to fact ...", "invalid fact
    /// redefinition ...", "invalid predicate id") plus any allocation-class
    /// error, all delivered by wrap.h as a returned message.
    #[error("{0}")]
    Ffi(#[from] FfiError),
    /// `mismatch between count and ids.size in batch` (Define.hs:112-113) —
    /// thrown Haskell-side when the batch's optional `ids` list length does
    /// not equal `count`; no C++ call is made in that case. Message kept
    /// byte-for-byte.
    #[error("mismatch between count and ids.size in batch")]
    Mismatch,
}

/// RULE §2 (error-recovery rule): "allocation-failure != structural error.
/// Every FFI null return in the inventory records which kind:
/// allocation-guard (abort) or precondition-guard (record + partial
/// result)." For `glean_define_fact` the row records the precondition guard
/// as the invalidFid sentinel (surfaced as `Err` at `define_fact`, NOT
/// here); the invoke Err channel that reaches this helper is the source's
/// uncaught-exception path, so abort reproduces the source's observable
/// outcome (the only defineFact call site,
/// glean/typed/Glean/Typed/BuildFact.hs:155, has no catch). One nuance —
/// the channel also carries `FactSet::define`'s key-too-large precondition
/// message — is flagged at `define_fact`. The helper is deliberately NOT
/// exported: the family keeps one private `allocation_guard_abort` per
/// module (builder.rs:215, inventory.rs:361, subst.rs:326). abort() rather
/// than panic! so no unwinding runs first.
fn allocation_guard_abort(e: FfiError) -> ! {
    eprintln!("glean: C++ fact definition failure: {}", e);
    std::process::abort()
}

/// The closure-local error type for [`define_fact`]. It exists solely so the
/// `E`-generic closure passed to `CanDefine::with_define` can carry BOTH the
/// source's `InvalidRedefinition` guard and the abort-class invoke error: the
/// trait method requires `E: From<FfiError>` (the merge bound; see the
/// trait doc), and `InvalidRedefinition` must NOT grow such an impl itself —
/// that would let an allocation failure masquerade as an invalid
/// redefinition. `From<FfiError>` here only feeds the match just outside,
/// which aborts the FFI arm with the same disposition the guard-kind note
/// documents. Not exported; the public signature keeps the row's
/// `Result<Fid, InvalidRedefinition>`.
enum DefineFactFailure {
    /// The invoke Err channel (allocation-class plus the flagged
    /// key-too-large nuance) — aborted, never returned.
    Ffi(FfiError),
    /// The row's PRECONDITION-GUARD: `id == invalidFid`, returned.
    Invalid(InvalidRedefinition),
}

impl From<FfiError> for DefineFactFailure {
    fn from(e: FfiError) -> Self {
        DefineFactFailure::Ffi(e)
    }
}

/// `defineFact :: CanDefine a => a -> Pid -> Builder -> CSize -> IO Fid`
/// (Define.hs:55-61):
///
/// * defineFact facts pred clause key_size =
/// *   withDefine facts $ \facts_ptr -> do
/// *     id <- invoke $ glean_define_fact facts_ptr pred clause key_size
/// *     when (id == invalidFid) $
/// *       throwIO $ InvalidRedefinition "invalid fact redefinition"
/// *     return id
///
/// The hsthrift invoke arity (`Ptr Fid` out-param allocated and peeked by
/// the class machinery) collapses to the stack local `id`, the crate idiom
/// (ffi.rs:379-385). `clause` is borrowed (the Haskell's Builder is pinned
/// by its bracket); `key_size` is the source's CSize already (usize).
///
/// Guard kind (RULE §2, told with the ABI bodies — the whole story):
///   * PRECONDITION-GUARD — the sentinel `id == invalidFid`. This is the
///     row's ruling: "id==invalidFid -> err, NOT allocation". Read off
///     define.h:21-24, `Id::INVALID` means the set already contains a fact
///     with the same type and key but a different value; factset.cpp:290-291
///     returns it for exactly that case. Surfaced as
///     `Err(InvalidRedefinition("invalid fact redefinition"))`, the
///     source's exception made explicit.
///   * The invoke Err channel is ABORTED (allocation_guard_abort): the
///     row/brief disposition. Bodies read while classifying: ffi.cpp:362's
///     `assert` aborts inside C++ and never crosses; `new`/fact insertion
///     inside `FactSet::define` is allocation-class; and — the nuance the
///     row does not record — factset.cpp:279-281 raises `error("key too
///     large: ...")`, a PRE-CONDITION message that DOES cross the wrap
///     channel as a value. It is reachable only when the caller built a
///     clause whose key exceeds `Fact::MAX_KEY_SIZE` (fact.h:33, asserted
///     in `Fact::Clause::from`; the asserts compile out under NDEBUG, so
///     the message is reachable in release builds), i.e. an
///     upstream-invariant violation;
///     the source would die of the uncaught FFIError at BuildFact.hs:155,
///     so abort reproduces the observable outcome. Flagged below.
//
// INVENTORY-BUG-CANDIDATE row 65: the row pins `define_fact ->
// Result<Fid, InvalidRedefinition>` and describes the guard as
// "precondition-guard (id==invalidFid -> err, NOT allocation)". ABI-body
// reading (pinned ground truth) shows the symbol's invoke error channel is
// not allocation-only: FactSet::define's `error("key too large")`
// (factset.cpp:279-281) is a precondition-class message the row's
// single-variant error type cannot carry. This port follows the row and the
// unit brief (abort on the whole invoke Err channel; disposition documented
// above) and flags the gap for the inventory owner: either widen the row's
// type to a two-variant error (`Ffi(FfiError) | InvalidRedefinition`) or
// record the abort disposition explicitly.
pub fn define_fact(
    facts: &impl CanDefine,
    pid: Pid,
    clause: &Builder,
    key_size: usize,
) -> Result<Fid, InvalidRedefinition> {
    let outcome = facts.with_define(|facts_ptr| {
        let mut id: i64 = 0;
        // SAFETY: `facts_ptr` is the Define* `with_define` guarantees valid
        // for this closure (borrowed, never freed or retained here);
        // `clause` is a live Builder (builder.rs: as_ptr only borrows it)
        // whose Output* handle outlives the call; `pid.0` is the
        // transparent i64 the header's glean_predicate_id_t expects;
        // `key_size` the size_t; `id` an initialised stack out-param. The
        // extern order mirrors ffi.h:218-223; the body (ffi.cpp:355-369)
        // writes `*id` on success. invoke takes the extern call itself as
        // its `call` closure and owns the error string's release.
        // The `?` merges the invoke channel into `DefineFactFailure::Ffi`
        // (its From<FfiError> impl above); the match just outside aborts it.
        unsafe {
            invoke(|| {
                sys::glean_define_fact(facts_ptr, pid.0, clause.as_ptr(), key_size, &mut id)
            })
        }?;
        if id == invalid_fid().0 {
            return Err(DefineFactFailure::Invalid(InvalidRedefinition(
                "invalid fact redefinition".to_string(),
            )));
        }
        Ok(Fid(id))
    });
    match outcome {
        Ok(id) => Ok(id),
        // The row/brief disposition for this symbol's invoke Err channel —
        // see the guard-kind note above (and the INVENTORY-BUG-CANDIDATE
        // flag it references).
        Err(DefineFactFailure::Ffi(e)) => allocation_guard_abort(e),
        Err(DefineFactFailure::Invalid(e)) => Err(e),
    }
}

/// `defineBatch :: CanDefine a => a -> Inventory -> Thrift.Batch ->
/// DefineFlags -> IO Subst` (Define.hs:85-114) — "Prepare a Thrift batch
/// for writing into the database by renaming and deduplicating facts."
///
/// Transliteration notes (the source's bracket nesting at Define.hs:92-106
/// dissolves into ordinary borrows):
///   * `withDefine facts $ \p_facts -> with inventory $ \p_inventory ->`
///     — `facts.with_define` + `inventory.as_ptr()` (the seam this unit
///     added to inventory.rs; BORROWED, valid for the call, never freed).
///   * `withIds $ \ids_ptr ->` — Define.hs:107-114. `Some(ids)` is passed
///     as the i64 buffer; the Haskell's `VS.unsafeWith (unsafeCoerceVector
///     ids)` borrow is the plain `Vec<i64>` borrow (no coercion needed: the
///     laid-out i64 IS the `glean_fact_id_t` layout, and
///     glean/util/Glean/Util/Vector.hs's helper dissolves, edge 1245).
///     `None` passes null. The count check is `fromIntegral (VS.length ids) ==
///     Thrift.batch_count batch` (Define.hs:110): `ids.len() as i64 ==
///     batch.count`; on mismatch the source `throwIO`s BEFORE the FFI call,
///     so this port returns `Err(DefineBatchError::Mismatch)` before the
///     call too (and the early return inside the closure exits through
///     `with_define`'s bracket exactly as the throw unwound through
///     `withDefine`'s).
///   * `unsafeWithBytes (Thrift.batch_facts batch) $ \facts_ptr facts_size
///     ->` (:96) — the `Vec<u8>` borrow pins the buffer for the call; the
///     C++ only reads it (`const void*`, ffi.h:231).
///   * `construct $ invoke $ glean_define_batch ...` (:97-106) — the
///     `Substitution**` out-param becomes the stack local `subst`, adopted
///     with `Subst::from_raw` on success (the seam this unit added to
///     subst.rs); `construct`'s mask window has no Rust analogue
///     (ffi.rs:558-566) and nothing here can unwind between the C++ alloc
///     and the adoption.
///   * `(Fid $ Thrift.batch_firstId batch)` (:100) — `Fid(Id)` is
///     repr(transparent) over i64 (types.rs:153-155), so the raw Id crosses
///     directly as the extern's `i64`. `(fromIntegral $ Thrift.batch_count
///     batch)` (:102) — RULE §2 numeric row: `as usize` keeps the source's
///     two's-complement wrap for a negative count (exactly what the
///     Haskell's `fromIntegral :: i64 -> CSize` did).
///   * `(fromBool trustRefs)` / `(fromBool ignoreRedef)` (:105-106) — Rust
///     passes `bool` directly (see the SOURCE-ABI DIVERGENCE note in the
///     header).
///
/// Guard kind: PRECONDITION-GUARD, returned (never aborted): the C++
/// `defineBatch` reports every structural failure in the ABI-truth list
/// through the wrap channel as a value, and the Haskell propagated them as
/// `FFIError`; `Err(DefineBatchError::Ffi)` is that exception made
/// explicit. The `Mismatch` variant is the source's own Haskell-side
/// `throwIO`. The returned `Subst` owns the C++ `Substitution`
/// (Drop = glean_free_subst).
pub fn define_batch(
    facts: &impl CanDefine,
    inventory: &Inventory,
    batch: &Batch,
    flags: &DefineFlags,
) -> Result<Subst, DefineBatchError> {
    facts.with_define(|facts_ptr| {
        // withIds (Define.hs:107-114) — check BEFORE the call; see the
        // transliteration notes above.
        let ids_ptr: *const i64 = match &batch.ids {
            Some(ids) => {
                if ids.len() as i64 == batch.count {
                    ids.as_ptr()
                } else {
                    return Err(DefineBatchError::Mismatch);
                }
            }
            None => ptr::null(),
        };

        let mut subst: *mut c_void = ptr::null_mut();
        // SAFETY: `facts_ptr` is the Define* with_define guarantees for
        // this closure (borrowed, never freed or retained); `inventory` is
        // a live &Inventory, so its `as_ptr()` handle is valid for the
        // call; `ids_ptr` points into `batch.ids`'s i64 buffer (or is null
        // when the batch has none) and `batch.facts`'s bytes are likewise
        // borrowed from `batch`, which outlives the call — both inputs are
        // read-only on the C++ side (const void*/const int64_t*); `subst`
        // is an initialised stack out-param. The extern order mirrors
        // ffi.h:225-236; the body (ffi.cpp:371-395, `*subst = new
        // Substitution(defineBatch(...))`) stores the fresh handle on
        // success. invoke takes the extern call itself as its `call`
        // closure and owns the error string's release.
        unsafe {
            invoke(|| {
                sys::glean_define_batch(
                    facts_ptr,
                    inventory.as_ptr(),
                    batch.first_id,
                    ids_ptr,
                    batch.count as usize,
                    batch.facts.as_ptr() as *const c_void,
                    batch.facts.len(),
                    flags.trust_refs,
                    flags.ignore_redef,
                    &mut subst,
                )
            })
        }?;

        // SAFETY: the `?` above returned early on any error, so on this
        // path `subst` is the live, uniquely-owned `Substitution*` the C++
        // body just allocated (ffi.cpp:383 — the only writer of the
        // out-param). That is exactly `Subst::from_raw`'s adoption
        // contract: ownership moves into the returned handle (whose Drop
        // calls glean_free_subst exactly once), and the raw pointer is not
        // used again on this side.
        Ok(unsafe { Subst::from_raw(subst) })
    })
}

// Marker breakdown (closed set — no open port TODOs, no source-defect
// reproductions, no performance deferrals, 1 inventory-bug-candidate):
//   * no open port TODOs: nothing is left open. The `glean_thrift::Batch`
//     shape is an ASSUMED-IMPORT under the v0.3 §4 draft-stub policy
//     (documented in the header, the same regime types.rs/inventory.rs use
//     for the generated crate), and the two sibling seams this unit needs
//     (`Inventory::as_ptr`, `Subst::from_raw`) were added to the sibling
//     files (SIBLING-UNIT CONTRACTS above) rather than left dangling.
//   * no source-defect reproductions: no source defect is reproduced. The
//     one source defect read on this path — the Haskell's CBool-vs-C++-bool
//     foreign-import mismatch (Define.hs:132-133 vs ffi.h:233-234) — is
//     deliberately NOT reproduced: the Rust declares `bool`, matching the
//     C++ exactly (see the SOURCE-ABI DIVERGENCE note).
//   * no performance deferrals: both functions hand the C the caller's
//     existing buffers (no copies), and the returned `Subst` is the
//     C++-allocated object itself, adopted — no serialization round-trip
//     (the fast
//     version IS the source's version here).
//   * 1 inventory-bug-candidate: row 65's guard classification misses the
//     `FactSet::define` key-too-large precondition error that crosses
//     `glean_define_fact`'s invoke channel as a value (details at
//     `define_fact`). Disposition kept per the row/brief (abort); flag
//     raised for the inventory owner.
// SAFETY count: 3 unsafe constructs, each with a SAFETY comment —
// `define_fact` 1 (the invoke, whose comment also covers the `id`
// out-param read afterward) and `define_batch` 2 (the invoke + the
// `Subst::from_raw` adoption). (The `unsafe extern "C"` block is a
// declaration, not counted.)
// PORT STATUS: confidence=high todos=0
