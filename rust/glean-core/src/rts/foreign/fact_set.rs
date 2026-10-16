// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// FactSet — an owned set of facts which can be looked up by id or by key,
// and the fact-definition surface of the Glean RTS.
//
// Ported from `glean/hs/Glean/RTS/Foreign/FactSet.hs` (202 lines).
//
// Manifest unit: rust/glean-core/src/rts/foreign/fact_set.rs
// (migration/manifest.tsv line 32, crate glean-core). RULE §4: one source
// file -> one target file; the manifest path is authoritative (the
// underscore name is the manifest's, not the module's).
//
// ABI truth (read, not assumed): glean/rts/ffi.h —
//   glean_factset_new              :268      (int64_t first_id, FactSet** facts)
//   glean_factset_free             :269      (FactSet* facts)
//   glean_factset_fact_count       :271      (FactSet*) -> size_t
//   glean_factset_fact_memory      :273      (FactSet*) -> size_t
//   glean_factset_allocated_memory :275      (FactSet*) -> size_t
//   glean_factset_predicateStats   :277-282  (FactSet*, size_t* count,
//                                              int64_t** ids, uint64_t** counts,
//                                              uint64_t** sizes)
//   glean_factset_lookup           :284      (FactSet*) -> Lookup*
//   glean_factset_define           :285      (FactSet*) -> Define*
//   glean_factset_serialize        :287-292  (FactSet*, int64_t* first_id,
//                                              size_t* count, void** facts_data,
//                                              size_t* facts_size)
//   glean_factset_serializeReorder :294-301  (FactSet*, uint64_t* order,
//                                              size_t order_size, ... same outs)
//   glean_factset_append           :303      (FactSet* target, FactSet* source)
//   glean_factset_rebase           :305-311  (FactSet*, const Inventory*,
//                                              const Substitution*,
//                                              LookupCache*, FactSet** result,
//                                              Substitution** out_subst)
//   [glean_factset_first_free_id   : NOT declared in ffi.h — defined in
//     ffi.cpp:506-508 only; see the row-66 flag below]
// and their C++ bodies at glean/rts/ffi.cpp:476-482 (new/free), :484-494
// (the three size getters), :496-504 (predicateStats), :506-508
// (first_free_id), :510-516 (lookup/define), :518-547
// (serialize/serializeReorder), :561-579 (rebase), :581-583 (append), over:
//   * glean/rts/factset.h:112 (`struct FactSet final : public Define` — the
//     FactSet IS the Define and, through Define, the Lookup; the copy
//     ctor/assignment are deleted :209-210), :269-271 (firstFreeId =
//     startingId() + size()), :282-283 (prefix seeks callable from multiple
//     threads but not interleaved with modification);
//   * glean/rts/factset.cpp:295-301 (serialize), :313-323 (serializeReorder),
//     :364-415 (rebase), :417-425 (append);
//   * glean/rts/stats.cpp:23-45 (marshal — the predicateStats array fill);
//   * glean/rts/sanity.h:21-27 (GLEAN_SANITY_CHECK — glog LOG(FATAL),
//     abort inside C++).
// `glean_fact_id_t` is `typedef int64_t` (ffi.h:105), so the id
// parameters are plain `i64`; `size_t` -> `usize` (RULE §2 A5 row).
//
// RULE citations:
//   §1 (FFI row, A2): raw `extern "C"` for flat C-ABI symbols.
//   §2: A5 CSize row — `size_t` <-> `usize`; error-recovery rule — the
//     guard-kind story in the `//!` block and at each function; numeric
//     conversions (the `fromIntegral` at FactSet.hs:66/:85/:104 kept with
//     the same wrap semantics).
//   §3: `unsafe` only inside this glean-core FFI wrapper; every block
//     carries a SAFETY comment.
//   §4: one source file -> one target file; assumed-import regime for the
//     generated glean_thrift crate.
//   §5: inventory rows followed (below); one flagged nuance.
//
// INVENTORY (RULE §5) — two rows govern this file (the same row, duplicated),
// followed EXACTLY:
//   * migration/inventory.tsv row 66 (line 66): "glean/hs/Glean/RTS/Foreign/
//     FactSet.hs | glean_factset_* (new/free/count/memory/allocated_memory/
//     predicateStats/first_free_id/lookup/define/serialize/serializeReorder/
//     rebase/append) | FFI-owned FactSet (ForeignPtr) + malloc'd serialize
//     buffers + mutating append/rebase | OWNED_MUTABLE | cxx bridge
//     [SUPERSEDED by §1/A2: raw extern-C — 2026-10-07]; opaque owned
//     FactSet (Rust Drop = glean_factset_free); serialize/serializeReorder
//     -> Vec<u8> ownership transfer into Thrift.Batch; rebase returns owned
//     (FactSet, Subst); append mutates target in place; C++ error strings ->
//     Result (precondition-guard; C++ OOM still aborts per RTS) | created
//     FactSet.hs:62-63, freed FactSet.hs:145, buffer transfer FactSet.hs:82-89
//     (mkBatch/unsafeMallocedByteString), rebase FactSet.hs:106-119, append
//     FactSet.hs:121-125, imports FactSet.hs:143-202 | confirmed".
//   * migration/inventory.ffi.tsv row 11 (line 11) — the same ruling in the
//     FFI inventory (same symbol list, same target_translation).
//   Followed EXACTLY: opaque owned `FactSet` (Rust Drop = glean_factset_free);
//   serialize/serializeReorder return the malloc'd buffer as an owned
//   `Batch` (the ownership transfer into Thrift.Batch); rebase returns owned
//   `(FactSet, Subst)`; append mutates the target in place; the C++ error
//   strings are surfaced as `Result` (the source's exceptions made
//   explicit). One classification nuance the row does not cover is flagged
//   as the row-66 inventory-bug-candidate at the `sys` block below.
//
// INVENTORY-BUG-CANDIDATE row 66: two row-text nuances (the port follows the
// row's headline target_translation — "C++ error strings -> Result" —
// exactly):
//   (a) the row's symbol list includes `glean_factset_first_free_id`, but
//       ffi.h does NOT declare it — the factset group (ffi.h:268-311) stops
//       at `glean_factset_rebase` (:305). The symbol is defined in
//       ffi.cpp:506-508 and the Haskell imports it directly via `foreign
//       import ccall` (FactSet.hs:165-166), so the port declares the extern
//       from the ffi.cpp definition (the linkage authority) — the same
//       disposition lookup.rs took for `glean_lookup_empty` (a C++ header
//       omission, not a port divergence; the row's symbol list matches the
//       Haskell's imports exactly).
//   (b) the row's guard label — "precondition-guard; C++ OOM still aborts
//       per RTS" — is inaccurate per the ABI bodies: the factset channels
//       carry no precondition check that can fail (ffi.cpp:477, :502-503,
//       :524-529, :540-546, :568-578, :582 — allocation-class only), and
//       wrap.h catches bad_alloc and returns it as a value (wrap.h:21-32),
//       so OOM does NOT abort. The port classifies the channels as
//       allocation-guard and returns `Err` — the disposition the row's
//       target_translation mandates; the label is flagged here per §5.
//
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, module-level): outbound —
// rows 1246 (FactSet.hs -> Glean/FFI.hs, crate::ffi), 1247 (-> RTS/Foreign/
// Define.hs, crate::rts::foreign::define), 1248 (-> RTS/Foreign/Inventory.hs,
// crate::rts::foreign::inventory), 1249 (-> RTS/Foreign/Lookup.hs,
// crate::rts::foreign::lookup), 1250 (-> RTS/Foreign/LookupCache.hs,
// crate::rts::foreign::lookup_cache), 1251 (-> RTS/Foreign/Stacked.hs,
// crate::rts::foreign::stacked), 1252 (-> RTS/Foreign/Stats.hs,
// crate::rts::foreign::stats), 1253 (-> RTS/Foreign/Subst.hs,
// crate::rts::foreign::subst) and 1254 (-> RTS/Types.hs, crate::rts::types),
// plus the thrift-generated `Glean.Types` import (FactSet.hs:45, the `Batch`
// and `PredicateStats` types) whose Rust home is the generated `glean_thrift`
// crate. Inbound — rows 405 (db/Glean/Database/Storage.hs), 414 (db/.../
// Storage/DB.hs), 432 (db/.../Storage/Memory.hs), 457 (db/Glean/Database/
// Types.hs), 486 (db/.../Write/Batch.hs), 704 (db/Glean/Query/UserQuery.hs),
// 1763 (tools/gleancli/GleanCLI/Merge.hs), 1836 (typed/Glean/Typed/
// BuildFact.hs) and 1875 (write/Glean/Write/SendAndRebaseQueue.hs) — the
// consumers of the FactSet handle and its serialize/rebase/renameFacts
// surface. This module imports from every sibling Rust unit named above
// except crate::rts::builder (the source imports no Builder).
//
// SIBLING-UNIT CONTRACTS (RULE §4; DONE crates'/units' real APIs read,
// never re-declared):
//   * crate::ffi (DONE, read): `invoke` (the pinned form
//     `pub unsafe fn invoke(call: impl FnOnce() -> *const c_char)
//     -> Result<(), FfiError>` — the extern call passed AS the closure),
//     `FfiError` (the error channel), and `unsafe_malloced_byte_string`
//     (the buffer transfer — the Rust twin of unsafeMallocedByteString:
//     copy + libc::free).
//   * crate::rts::types (DONE, read): `Fid` (repr(transparent) over the
//     thrift `Id` = i64, types.rs:153-155 — the by-value FFI payload) and
//     `Pid` (repr(transparent) over i64, types.rs:120-122).
//   * crate::rts::foreign::lookup (DONE, read): `CanLookup` — the pinned
//     trait `fn with_lookup<T, E: From<FfiError>>(&self, f: impl
//     FnOnce(*mut c_void) -> Result<T, E>) -> Result<T, E>; fn lookup_name(&self)
//     -> &str` (lookup.rs:122-131) — and the `Lookup` borrowed handle.
//   * crate::rts::foreign::define (DONE, read): `CanDefine` (the pinned
//     trait, define.rs:355-361), `DefineFlags` (re-exported below),
//     `define_batch` (define.rs:619-678) and `DefineBatchError` (the shipped
//     merge enum, define.rs:423-438).
//   * crate::rts::foreign::inventory (DONE, read): `Inventory`, the owned
//     handle; `Inventory::as_ptr` (inventory.rs:335) — the borrowed-pointer
//     seam for the rebase input.
//   * crate::rts::foreign::subst (DONE, read): `Subst`, the owned handle;
//     `Subst::from_raw` (subst.rs:329-331 — the adoption seam for the rebase
//     out-param) and `Subst::as_ptr` (subst.rs:307-309 — the borrowed-pointer
//     seam for the rebase input, landed by the coordinator for this unit).
//   * crate::rts::foreign::lookup_cache (DONE, read): `LookupCache`, the
//     owned handle; `LookupCache::with_ptr` (lookup_cache.rs:337-339) — the
//     scoped-pointer seam for the rebase input.
//   * crate::rts::foreign::stacked (DONE, read): `stacked` (stacked.rs:145-
//     148) — the `Stacked` view constructor (takes `base`/`added` BY VALUE;
//     see the rename_facts note on minding the move) and the `Stacked`
//     type with its CanLookup/CanDefine impls.
//   * crate::rts::foreign::stats (DONE, read): `marshal_predicate_stats`
//     (stats.rs:116-118) — the getter-unpacking helper predicate_stats
//     reuses; NOT re-marshalled here.
//   * glean_thrift (generated; see the ASSUMED-IMPORT block below): `Batch`
//     and `PredicateStats` — imported, never re-declared.
//   * Why `pub mod sys` is public — for sibling consumers declaring their
//     own externs over the same opaque handles (the same convention the
//     sibling units use).
//
// ASSUMED-IMPORT (RULE §4 draft stub policy / assumed-imports — the same
// regime define.rs already pins for this crate): `glean_thrift` is the
// build.rs-generated pinned crate (RULE §1 thrift row: "generated by
// build.rs from if/*.thrift, never hand-translated") and does not exist in
// this draft tree yet. The thrift definition is authoritative
// (glean/if/glean.thrift:159-214):
//   struct Batch {
//     1: Id firstId;                 // -> first_id: Id   (i64)
//     2: i64 count;                  // -> count: i64
//     3: binary facts;               // -> facts: Vec<u8>
//     4: optional list<i64> ids;      // -> ids: Option<Vec<i64>>
//     // 5 owned / 6 dependencies / 7 schema_id: present in the struct,
//     // unused by FactSet.hs (and unused here beyond the mempty/Nothing
//     // fills below).
//   }
// The full spellings assumed here are the generated snake_case form
// (`first_id`, `count`, `facts`, `ids`, `owned`, `dependencies`,
// `schema_id`); the Haskell reads the same fields through the hsthrift
// accessors. The two map fields (`owned`, `dependencies`) are filled with
// `Default::default()` (the source's `pure mempty`); if the generated crate
// spells the map types differently, only the one construction site in
// `mk_batch` changes. (This is the EXACT shape define.rs:208-226 already
// pinned — reused, not re-defined.)

//! C++ RTS fact-set handle (`FactSet`) — an owned set of facts that can be
//! looked up by id or by key, and the fact-definition surface.
//!
//! Port notes: RULE §3 — `unsafe` appears only inside this glean-core FFI
//! wrapper, where the Haskell was already FFI; every block carries a SAFETY
//! comment. The source's ForeignPtr/`with`/`construct`/`create`/`destroy`
//! machinery (FactSet.hs:48-63) dissolves into Rust ownership: the `Drop`
//! below runs `glean_factset_free` exactly once, and every method borrows
//! `&self` (the Haskell's `with`) or `&mut self` (the one mutating method,
//! `append`). Error-recovery classification per RULE §2 (error-recovery
//! rule), told as one story with the C++ bodies:
//!   * `glean_factset_new` — ALLOCATION-GUARD only (the body is
//!     `*facts = new FactSet(...)`, ffi.cpp:477 — no precondition check
//!     exists to fail); returned as `Err`.
//!   * the three size getters and `first_free_id` — NO error channel (the
//!     externs return `size_t`/`int64_t` directly, ffi.h:271-275,
//!     ffi.cpp:506-508); plain `usize`/`Fid` returns, no `Result`.
//!   * `glean_factset_predicateStats` — ALLOCATION-GUARD only (a DenseMap
//!     walk + three `malloc_array`s, stats.cpp:23-45); returned as `Err`
//!     via the shipped `marshal_predicate_stats`.
//!   * `glean_factset_serialize`/`serializeReorder` — ALLOCATION-GUARD
//!     only (a pure iteration + byte copy, factset.cpp:295-323, plus the
//!     buffer release); returned as `Err`.
//!   * `glean_factset_rebase` — the body (ffi.cpp:561-579) carries
//!     GLEAN_SANITY_CHECKs (glog LOG(FATAL) — abort inside C++ and never
//!     cross as a value, sanity.h:21-27) plus allocation-class (the two
//!     `new`s at ffi.cpp:574-575); the wrap channel is returned as `Err`.
//!   * `glean_factset_append` — ALLOCATION-GUARD only (a vector insert +
//!     key merge; the `assert(appendable(other))` at factset.cpp:418 is a
//!     C `assert` — aborts in debug builds, compiled out under NDEBUG);
//!     returned as `Err`. The C++ body MOVES the source's facts
//!     (ffi.cpp:581-583) — see `append`.

use std::os::raw::{c_char, c_void};
use std::ptr;

use glean_thrift::{Batch, PredicateStats};

use crate::ffi::{invoke, unsafe_malloced_byte_string, FfiError};
use crate::rts::foreign::define::{define_batch, CanDefine, DefineBatchError};
use crate::rts::foreign::inventory::Inventory;
use crate::rts::foreign::lookup::CanLookup;
use crate::rts::foreign::lookup_cache::LookupCache;
use crate::rts::foreign::stacked::stacked;
use crate::rts::foreign::stats::marshal_predicate_stats;
use crate::rts::foreign::subst::Subst;
use crate::rts::types::{Fid, Pid};

pub use crate::rts::foreign::define::DefineFlags;

/// Raw C-ABI declarations for the `glean_factset_*` group (glean/rts/ffi.h:
/// 268-311; `glean_factset_first_free_id` from ffi.cpp:506-508 — see the
/// row-66 flag in the header).
///
/// RULE §1 (FFI row, A2): raw `extern "C"` for flat C-ABI symbols.
///
/// Signatures mirror ffi.h EXACTLY, with the family's handle modelling:
/// `FactSet*`/`Inventory*`/`Substitution*`/`LookupCache*` are opaque
/// `c_void` pointers (never dereferenced on this side — the
/// inventory.rs/subst.rs/lookup_cache.rs convention); `size_t` -> `usize`
/// (A5 CSize row, unsigned both sides); the id parameters are `i64`/`*const
/// i64`/`*mut i64` (`glean_fact_id_t` = int64_t, ffi.h:105). The three size
/// getters and `first_free_id` return `size_t`/`int64_t` DIRECTLY — no
/// CString error channel (the Haskell's `IO Int`/`IO Fid` from these is a
/// total pure read). Guard kinds per RULE §2 are recorded at each symbol;
/// the actual dispositions live at the call sites below.
pub mod sys {
    use std::os::raw::{c_char, c_void};

    // NOTE (RULE §3, family convention): the `unsafe extern "C"` block
    // below is a DECLARATION, not an operation — the same ruling lookup.rs,
    // define.rs, subst.rs and stacked.rs state for theirs. It introduces no
    // unsafe behaviour of its own, carries no SAFETY comment, and is NOT
    // counted in the SAFETY tally at the foot of this file; every USE below
    // carries its own.
    unsafe extern "C" {
        // ffi.h:268 — `FactSet**` out-param; body ffi.cpp:476-478 is
        // `*facts = new FactSet(Id::fromThrift(first_id))` — ALLOCATION-GUARD
        // only (no precondition check exists to fail).
        pub fn glean_factset_new(first_id: i64, facts: *mut *mut c_void) -> *const c_char;

        // ffi.h:269 — void; body ffi.cpp:480-482 is ffi::free_ (wrap_
        // swallows every exception, wrap.h:34-45) — infallible.
        pub fn glean_factset_free(facts: *mut c_void);

        // ffi.h:271 — size_t, NO error channel; body ffi.cpp:484-486 is
        // `facts->size()` — a pure read.
        pub fn glean_factset_fact_count(facts: *mut c_void) -> usize;

        // ffi.h:273 — size_t, NO error channel; body ffi.cpp:488-490 is
        // `facts->factMemory()` — a pure read.
        pub fn glean_factset_fact_memory(facts: *mut c_void) -> usize;

        // ffi.h:275 — size_t, NO error channel; body ffi.cpp:492-494 is
        // `sizeof(FactSet) + facts->allocatedMemory()` — a pure read.
        pub fn glean_factset_allocated_memory(facts: *mut c_void) -> usize;

        // ffi.h:277-282 — the four out-params (count + three malloc'd
        // arrays); body ffi.cpp:496-504 is `ffi::wrap([=] {
        // marshal(facts->predicateStats(), count, ids, counts, sizes); })`
        // over stats.cpp:23-45 — ALLOCATION-GUARD only (the DenseMap walk
        // and three malloc_arrays; no precondition check exists to fail).
        // NOTE: the Haskell imports this symbol `ccall safe` (FactSet.hs:157)
        // — the ONLY safe import in the file; the safe/unsafe distinction has
        // no Rust analogue in the `invoke` protocol (stats.rs:29-31 records
        // the same note for this very getter).
        pub fn glean_factset_predicateStats(
            facts: *mut c_void,
            count: *mut usize,
            ids: *mut *mut i64,
            counts: *mut *mut u64,
            sizes: *mut *mut u64,
        ) -> *const c_char;

        // NOT in ffi.h — ffi.cpp:506-508 only (see the INVENTORY-BUG-
        // CANDIDATE row 66 flag in the header). Body: `return
        // facts->firstFreeId().toThrift();` — int64_t, NO error channel; a
        // pure read (factset.h:269-271: `startingId() + size()`).
        pub fn glean_factset_first_free_id(facts: *mut c_void) -> i64;

        // ffi.h:284 — body ffi.cpp:510-512 is `return facts;` — the FactSet
        // IS the Lookup (factset.h:112 `struct FactSet final : public
        // Define`, and Define : Lookup). Direct return, NO error channel.
        pub fn glean_factset_lookup(facts: *mut c_void) -> *mut c_void;

        // ffi.h:285 — body ffi.cpp:514-516 is `return facts;` — the FactSet
        // IS the Define. Direct return, NO error channel.
        pub fn glean_factset_define(facts: *mut c_void) -> *mut c_void;

        // ffi.h:287-292 — the four out-params; body ffi.cpp:518-530 over
        // factset.cpp:295-301 (`FactSet::serialize`). The buffer is
        // released to the caller (`s.facts.moveBytes().release_to(...)`,
        // ffi.cpp:528) — ownership transfer. ALLOCATION-GUARD only.
        pub fn glean_factset_serialize(
            facts: *mut c_void,
            first_id: *mut i64,
            count: *mut usize,
            facts_data: *mut *mut c_void,
            facts_size: *mut usize,
        ) -> *const c_char;

        // ffi.h:294-301 — `order` is `uint64_t*` (the C++ reads it as
        // `folly::Range<const uint64_t*>`, ffi.cpp:541-542); body
        // ffi.cpp:532-547 over factset.cpp:313-323. The buffer is released
        // to the caller — ownership transfer. ALLOCATION-GUARD only. (The
        // Haskell's import types `order` as `Ptr Int64` — a
        // representation-identical pun the source itself performs; see the
        // note at `serialize_reorder`.)
        pub fn glean_factset_serializeReorder(
            facts: *mut c_void,
            order: *const u64,
            order_size: usize,
            first_id: *mut i64,
            count: *mut usize,
            facts_data: *mut *mut c_void,
            facts_size: *mut usize,
        ) -> *const c_char;

        // ffi.h:305-311 — the two out-params; body ffi.cpp:561-579 over
        // factset.cpp:364-415. The body carries GLEAN_SANITY_CHECKs
        // (glog LOG(FATAL) — abort inside C++, sanity.h:21-27) plus
        // allocation-class; both out-params are adopted on success.
        pub fn glean_factset_rebase(
            facts: *mut c_void,
            inventory: *const c_void,
            subst: *const c_void,
            cache: *mut c_void,
            result: *mut *mut c_void,
            out_subst: *mut *mut c_void,
        ) -> *const c_char;

        // ffi.h:303 — body ffi.cpp:581-583 is
        // `target->append(std::move(*source))` over factset.cpp:417-425 —
        // the C++ MOVES the source's facts into the target (the source is
        // left empty). ALLOCATION-GUARD only (the body is a vector insert +
        // key merge; the `assert(appendable(other))` at factset.cpp:418 is
        // a glog CHECK — abort inside C++).
        pub fn glean_factset_append(target: *mut c_void, source: *mut c_void) -> *const c_char;
    }
}

/// `newtype FactSet = FactSet (ForeignPtr FactSet)` (FactSet.hs:48) — an
/// OWNED handle to the C++ `rts::FactSet` (factset.h:112-358).
///
/// The source's `instance Object FactSet` (FactSet.hs:50-53) supplied
/// `wrap = FactSet`, `unwrap`, and `destroy = glean_factset_free`; the
/// ForeignPtr/`with`/`construct`/`create` machinery dissolves into Rust
/// ownership — the `Drop` below runs the finalizer exactly once, and every
/// method borrows `&self` (the Haskell's `with`) or `&mut self` (the one
/// mutating method, `append`). The `Object` instance also gave `release ::
/// FactSet -> IO ()` (Glean/FFI.hs:84-85 — used at UserQuery.hs:461,
/// Batch.hs:234/:246/:286); the Rust landing is plain `drop(facts)` (the
/// four call sites all own the handle, so the mapping is exact — the family's
/// owned handles dissolve `Object` into `Drop`; only `Subroutine` impls it,
/// bytecode.rs:276). Inventory row 66: "FFI-owned FactSet (ForeignPtr)";
/// "opaque owned FactSet (Rust Drop = glean_factset_free)".
///
/// The C++ `FactSet` is movable but not copyable (factset.h:209-210 deletes
/// the copy ctor/assignment); the Rust handle is likewise neither Clone nor
/// Copy.
///
/// Not `Send`/`Sync` (conservative representation — the sibling `Define`
/// posture, define.rs:310-313): the handle is MUTABLE state. `append`
/// mutates through `&mut self`, and `define_batch(&*facts, ...)` mutates
/// through `&self` — the `CanDefine::with_define` impl below hands out the
/// raw pointer to the C++ define surface — and the C++ contract
/// (factset.h:282-283) forbids interleaving modification with reads.
/// Sharing `&FactSet` across threads would let fully safe code race the C++
/// object, so `Sync` is withheld; `Send` is withheld with it for the same
/// conservative posture (a handle that cannot be shared cannot introduce a
/// cross-thread race through this type either). The Haskell's ForeignPtr
/// newtype is unconditionally `Send + Sync`; that shareability is
/// deliberately NOT reproduced — a documented divergence where the Rust
/// surface is stricter, which is the sound direction.
pub struct FactSet {
    /// Opaque `FactSet*` from `glean_factset_new` / `glean_factset_rebase`.
    /// Non-null for every value this module produces; never dereferenced on
    /// the Rust side (only handed back to the C functions in `sys`).
    ptr: *mut c_void,
}

// `Send`/`Sync` are deliberately NOT implemented — see the struct doc above
// (the handle is mutable state; the sibling `Define` takes the same
// conservative posture, define.rs:310-313).

impl Drop for FactSet {
    fn drop(&mut self) {
        // SAFETY: `self.ptr` is non-null and originates from a successful
        // `glean_factset_new` (ffi.cpp:477) or `glean_factset_rebase`
        // (ffi.cpp:574) call, or the `from_raw` seam below (the construction
        // sites; the field stays private and every door carries the same
        // live-handle contract), so it is a live handle the C++ side owns.
        // `FactSet` is neither Copy nor Clone, so this runs exactly once per
        // handle and cannot double-free. The body (ffi.cpp:480-482) is
        // ffi::free_, whose wrap_ swallows every exception (wrap.h:34-45) —
        // nothing can unwind across.
        unsafe { sys::glean_factset_free(self.ptr) };
    }
}

impl FactSet {
    /// Adopt a raw `FactSet*` into the owned `FactSet` handle — the door for
    /// the `glean_factset_rebase` `FactSet**` out-param (ffi.h:310), the
    /// twin of the shipped `Subst::from_raw` (subst.rs:329-331).
    ///
    /// SAFETY: `ptr` must be a live, uniquely-owned `FactSet*` from a
    /// successful C ABI call (never null, never already adopted or freed
    /// elsewhere). Ownership transfers to the returned handle: `Drop`
    /// (glean_factset_free) will free `ptr` exactly once.
    pub unsafe fn from_raw(ptr: *mut c_void) -> FactSet {
        FactSet { ptr }
    }

    /// Borrow the raw `FactSet*` for the duration of an extern call — the
    /// Rust landing of the source's `Object` instance, whose `with`/`unwrap`
    /// is how consumers hand the pointer to their own foreign imports
    /// (FactSet.hs:108-111 passes it to `glean_factset_rebase`'s `FactSet*`
    /// param, ffi.h:306). Mirrors the shipped `Inventory::as_ptr`
    /// (inventory.rs:335) / `Subst::as_ptr` (subst.rs:307-309) seams. The
    /// pointer is BORROWED: it stays owned by this `FactSet`, remains valid
    /// until drop, and must never be freed by the caller.
    pub fn as_ptr(&self) -> *mut c_void {
        self.ptr
    }
}

/// `instance CanLookup FactSet where lookupName _ = "factset"; withLookup x f
/// = with x $ f . glean_factset_lookup` (FactSet.hs:55-57).
///
/// `glean_factset_lookup` RETURNS the `Lookup*` directly (ffi.cpp:510-512 —
/// `return facts;`, the FactSet IS the Lookup) — there is NO error channel,
/// so the callback's `Result` is always `Ok` and the `E: From<FfiError>`
/// bound the trait carries (lookup.rs:122-131) is never exercised here.
impl CanLookup for FactSet {
    fn with_lookup<T, E: From<FfiError>>(
        &self,
        f: impl FnOnce(*mut c_void) -> Result<T, E>,
    ) -> Result<T, E> {
        // SAFETY: `self.ptr` is a live `FactSet*` (the handle contract above);
        // `glean_factset_lookup` (ffi.h:284, body ffi.cpp:510-512) returns
        // it unchanged — the C++ `FactSet` IS the `Lookup` (factset.h:112).
        // The returned pointer is valid for as long as `self` is borrowed.
        let lookup = unsafe { sys::glean_factset_lookup(self.ptr) };
        f(lookup)
    }

    fn lookup_name(&self) -> &str {
        "factset"
    }
}

/// `instance CanDefine FactSet where withDefine x f = with x $ f .
/// glean_factset_define` (FactSet.hs:59-60).
///
/// As with `CanLookup`: `glean_factset_define` RETURNS the `Define*`
/// directly (ffi.cpp:514-516 — `return facts;`) — NO error channel.
impl CanDefine for FactSet {
    fn with_define<T, E: From<FfiError>>(
        &self,
        f: impl FnOnce(*mut c_void) -> Result<T, E>,
    ) -> Result<T, E> {
        // SAFETY: `self.ptr` is a live `FactSet*`; `glean_factset_define`
        // (ffi.h:285, body ffi.cpp:514-516) returns it unchanged — the C++
        // `FactSet` IS the `Define` (factset.h:112). Valid for the borrow.
        let define = unsafe { sys::glean_factset_define(self.ptr) };
        f(define)
    }
}

/// Blanket impl: a shared reference to a `CanLookup` is itself `CanLookup`
/// (the reference borrows the underlying lookup for the call). This is what
/// lets `rename_facts` stack `base` (a `&impl CanLookup`) and `&added`
/// without moving the owned handles — the Haskell's `stacked base added`
/// shares the ForeignPtrs (Stacked.hs:27-30); the Rust port borrows.
impl<L: CanLookup> CanLookup for &L {
    fn with_lookup<T, E: From<FfiError>>(
        &self,
        f: impl FnOnce(*mut c_void) -> Result<T, E>,
    ) -> Result<T, E> {
        L::with_lookup(*self, f)
    }

    fn lookup_name(&self) -> &str {
        L::lookup_name(*self)
    }
}

/// Blanket impl: a shared reference to a `CanDefine` is itself `CanDefine`.
impl<D: CanDefine> CanDefine for &D {
    fn with_define<T, E: From<FfiError>>(
        &self,
        f: impl FnOnce(*mut c_void) -> Result<T, E>,
    ) -> Result<T, E> {
        D::with_define(*self, f)
    }
}

/// `new :: Fid -> IO FactSet` (FactSet.hs:62-63):
///
/// * new next_id = construct $ invoke $ glean_factset_new next_id
///
/// The hsthrift invoke's `Ptr (Ptr FactSet)` out-param collapses to the
/// stack local `facts` (the crate idiom, ffi.rs:379-385); `construct`'s
/// `mask_` window has no Rust analogue (ffi.rs:558-566) and nothing here
/// can unwind between the C++ allocation and the adoption.
///
/// Guard kind: ALLOCATION-GUARD only — the body (ffi.cpp:476-478) is
/// `*facts = new FactSet(Id::fromThrift(first_id))` with no precondition
/// check that can fail; the wrap.h message is returned as `Err` (the
/// source's exception made explicit — a caller may legitimately handle OOM).
pub fn new(next_id: Fid) -> Result<FactSet, FfiError> {
    let mut facts: *mut c_void = ptr::null_mut();
    // SAFETY: `facts` is an initialised stack out-param; the extern order
    // mirrors ffi.h:268. The body (ffi.cpp:476-478) stores the fresh
    // handle on success. invoke takes the extern call itself as its `call`
    // closure and owns the error string's release.
    unsafe { invoke(|| sys::glean_factset_new(next_id.0, &mut facts)) }?;
    // SAFETY: on success `facts` is the live, uniquely-owned `FactSet*` the
    // C++ body allocated (ffi.cpp:477 — the only writer of the out-param);
    // `?` above returned early on failure so a null cannot be wrapped.
    // Ownership moves into the returned handle, whose Drop runs
    // glean_factset_free exactly once — the source's `construct` + `Object`
    // instance made explicit (FactSet.hs:50-53, :62-63).
    Ok(unsafe { FactSet::from_raw(facts) })
}

/// `factCount :: FactSet -> IO Int` (FactSet.hs:65-66):
///
/// * factCount facts = fromIntegral <$> with facts glean_factset_fact_count
///
/// The extern returns `size_t` DIRECTLY (ffi.h:271) — there is NO error
/// channel (no CString return), so the Haskell's `IO Int` is a total pure
/// read and the Rust port returns plain `usize` (RULE §2 A5 CSize row:
/// `size_t` ↔ `usize`, unsigned both sides; the source's
/// `fromIntegral :: CSize -> Int` was a signed reinterpretation,
/// intentionally not reproduced — counts never approach the boundary).
/// No `Result` — there is nothing to fail.
pub fn fact_count(&self) -> usize {
    // SAFETY: `self.ptr` is a live `FactSet*`; the body (ffi.cpp:484-486) is
    // `facts->size()` — a pure read with no error channel.
    unsafe { sys::glean_factset_fact_count(self.ptr) }
}

/// `factMemory :: FactSet -> IO Int` (FactSet.hs:68-69) — the number of
/// bytes occupied by the facts (factset.h:248-251). Same shape as
/// `fact_count`: the extern returns `size_t` directly (ffi.h:273), no error
/// channel, plain `usize`.
pub fn fact_memory(&self) -> usize {
    // SAFETY: `self.ptr` is a live `FactSet*`; the body (ffi.cpp:488-490) is
    // `facts->factMemory()` — a pure read with no error channel.
    unsafe { sys::glean_factset_fact_memory(self.ptr) }
}

/// `allocatedMemory :: FactSet -> IO Int` (FactSet.hs:71-73) — the
/// (approximate) total bytes allocated by the FactSet (factset.h:192-195,
/// :253). Same shape: `size_t` directly (ffi.h:275), no error channel,
/// plain `usize`.
pub fn allocated_memory(&self) -> usize {
    // SAFETY: `self.ptr` is a live `FactSet*`; the body (ffi.cpp:492-494) is
    // `sizeof(FactSet) + facts->allocatedMemory()` — a pure read with no
    // error channel.
    unsafe { sys::glean_factset_allocated_memory(self.ptr) }
}

/// `predicateStats :: FactSet -> IO [(Pid, Thrift.PredicateStats)]`
/// (FactSet.hs:75-77):
///
/// * predicateStats facts = with facts
/// *   $ marshalPredicateStats . glean_factset_predicateStats
///
/// The source's composition is preserved EXACTLY: the shipped
/// `marshal_predicate_stats` (stats.rs:116-118) is called with the extern as
/// its getter — no re-marshalling here. The getter's `ccall safe`
/// distinction (FactSet.hs:157) has no Rust analogue (stats.rs:29-31).
///
/// Guard kind: ALLOCATION-GUARD only — the body (ffi.cpp:496-504 over
/// stats.cpp:23-45) is a DenseMap walk + three `malloc_array`s with no
/// precondition check; returned as `Err` (the source's exception made
/// explicit).
pub fn predicate_stats(&self) -> Result<Vec<(Pid, PredicateStats)>, FfiError> {
    marshal_predicate_stats(|count, ids, counts, sizes| {
        // SAFETY: `self.ptr` is a live `FactSet*`; the four out-params are
        // the initialised stack locals `marshal_predicate_stats` declares
        // (stats.rs:119-122); the extern order mirrors ffi.h:277-282. The
        // body (ffi.cpp:496-504) writes the filled count and the three
        // malloc'd arrays on success. (invoke — called by
        // marshal_predicate_stats — takes the getter AS its `call` closure
        // and owns the error string's release.)
        unsafe { sys::glean_factset_predicateStats(self.ptr, count, ids, counts, sizes) }
    })
}

/// `firstFreeId :: FactSet -> IO Fid` (FactSet.hs:79-80):
///
/// * firstFreeId facts = with facts glean_factset_first_free_id
///
/// The extern returns `int64_t` DIRECTLY (ffi.cpp:506-508) — NO error
/// channel, so the Rust port returns plain `Fid` (the source's `Fid` is
/// repr(transparent) over the thrift `Id` = i64, types.rs:153-155, so the
/// raw i64 crosses directly). No `Result`.
pub fn first_free_id(&self) -> Fid {
    // SAFETY: `self.ptr` is a live `FactSet*`; the body (ffi.cpp:506-508)
    // is `facts->firstFreeId().toThrift()` — a pure read (factset.h:269-271:
    // `startingId() + size()`) with no error channel.
    Fid(unsafe { sys::glean_factset_first_free_id(self.ptr) })
}

/// `mkBatch :: IO (Fid, CSize, Ptr (), CSize) -> IO Thrift.Batch`
/// (FactSet.hs:82-90) — the source's private serialize helper, ported as a
/// private fn (the source does not export it).
///
/// * mkBatch fn = mask_ $ do
/// *   (Fid first_id, count, facts_data, facts_size) <- fn
/// *   Thrift.Batch first_id (fromIntegral count)
/// *     <$> unsafeMallocedByteString facts_data facts_size
/// *     <*> pure Nothing
/// *     <*> pure mempty
/// *     <*> pure mempty
/// *     <*> pure Nothing
///
/// The four out-params are declared here and handed to the extern through
/// `get` (the source's `fn`); the `mask_` window (between the C++ allocation
/// and the adoption) has no Rust analogue — nothing here can unwind between
/// the extern call and the transfer below (ffi.rs:558-566). The `Batch`
/// construction fills `ids = None` / `owned = mempty` / `dependencies =
/// mempty` / `schema_id = None` exactly as the source's `pure Nothing` /
/// `pure mempty` do (the ASSUMED-IMPORT block pins the field spellings).
///
/// PERF(port): the buffer transfer below copies — `unsafe_malloced_byte_string`
/// (ffi.rs:121-128) does `slice::from_raw_parts(...).to_vec()` + `libc::free`,
/// while the source's `unsafeMallocedByteString` was zero-copy
/// (`BS.unsafePackMallocCStringLen`, Glean/FFI.hs:42-44 — a ForeignPtr
/// finalizerFree wrap). The crate's shared transfer helper is the
/// slow-but-faithful twin; the copy is the price of the crate-wide ownership
/// transfer. Flagged per RULEBOOK §5 (slow-but-faithful copy).
fn mk_batch(
    get: impl FnOnce(*mut i64, *mut usize, *mut *mut c_void, *mut usize) -> *const c_char,
) -> Result<Batch, FfiError> {
    let mut first_id: i64 = 0;
    let mut count: usize = 0;
    let mut facts_data: *mut c_void = ptr::null_mut();
    let mut facts_size: usize = 0;

    // SAFETY: the four out-params are initialised stack locals; `get` is the
    // extern call — for the two in-tree consumers, `glean_factset_serialize`
    // / `glean_factset_serializeReorder` (ffi.h:287-301, bodies
    // ffi.cpp:518-547), whose C contract is to write the filled count and
    // the released malloc'd buffer through them on success. invoke takes the
    // extern call itself as its `call` closure and owns the error string's
    // release.
    unsafe { invoke(|| get(&mut first_id, &mut count, &mut facts_data, &mut facts_size)) }?;

    // SAFETY: the `?` above returned early on any error, so on this path
    // `facts_data`/`facts_size` are the malloc'd ownership transfer
    // (ffi.cpp:528/:545 — `release_to`), exactly
    // `unsafe_malloced_byte_string`'s contract: it reads exactly
    // `facts_size` bytes and frees the buffer. Not retained anywhere else on
    // this side.
    let facts = unsafe { unsafe_malloced_byte_string(facts_data as *const c_void, facts_size) };

    Ok(Batch {
        first_id,
        // RULE §2 (numeric row): the source's `fromIntegral count`
        // (CSize -> Int64) — `as i64` keeps the same two's-complement wrap
        // for a huge count (exactly what the Haskell's `fromIntegral ::
        // CSize -> Int64` did).
        count: count as i64,
        facts,
        ids: None,
        owned: Default::default(),
        dependencies: Default::default(),
        schema_id: None,
    })
}

/// `serialize :: FactSet -> IO Thrift.Batch` (FactSet.hs:92-95):
///
/// * serialize facts =
/// *   with facts $ \facts_ptr -> do
/// *   mkBatch $ invoke $ glean_factset_serialize facts_ptr
///
/// Guard kind: ALLOCATION-GUARD only (the body is `facts->serialize()` —
/// factset.cpp:295-301, a pure iteration + byte copy — plus the buffer
/// release; no precondition check). Returned as `Err`.
pub fn serialize(&self) -> Result<Batch, FfiError> {
    mk_batch(|first_id, count, facts_data, facts_size| {
        // SAFETY: `self.ptr` is a live `FactSet*`; the four out-params are
        // the initialised stack locals `mk_batch` declares; the extern order
        // mirrors ffi.h:287-292. The body (ffi.cpp:518-530) writes the
        // filled count and the released malloc'd buffer on success.
        // (invoke — called by `mk_batch` — takes the getter AS its `call`
        // closure and owns the error string's release.)
        unsafe { sys::glean_factset_serialize(self.ptr, first_id, count, facts_data, facts_size) }
    })
}

/// `serializeReorder :: FactSet -> Vector Int64 -> IO Thrift.Batch`
/// (FactSet.hs:97-104):
///
/// * serializeReorder facts order =
/// *   with facts $ \facts_ptr -> do
/// *   unsafeWith order $ \order_ptr -> do
/// *     mkBatch $ invoke $ glean_factset_serializeReorder
/// *       facts_ptr
/// *       order_ptr
/// *       (fromIntegral (Vector.length order))
///
/// The `VS.unsafeWith order` borrow is the plain `&[i64]` borrow (the
/// laid-out i64 IS the element layout; the C++ only reads the range,
/// ffi.cpp:541-542). NOTE the source's own type pun, preserved: the Haskell
/// passes `order_ptr :: Ptr Int64` where the C++ declares `uint64_t*`
/// (ffi.h:296) — a representation-identical reinterpretation (the values
/// are fact ids, non-negative). The port mirrors the source's parameter
/// type (`&[i64]`, the Haskell's `Vector Int64`) and the header's ABI
/// (`*const u64`); the `.cast()` at the call site is the same no-op
/// reinterpretation the source itself performs. `fromIntegral
/// (Vector.length order)` is `order.len()` (usize both sides).
///
/// Guard kind: ALLOCATION-GUARD only (factset.cpp:313-323 — the `assert`
/// at :317-318 is a C `assert`, NDEBUG-dependent). Returned as `Err`.
pub fn serialize_reorder(&self, order: &[i64]) -> Result<Batch, FfiError> {
    mk_batch(|first_id, count, facts_data, facts_size| {
        // SAFETY: `self.ptr` is a live `FactSet*`; `order` is a live
        // `&[i64]` whose laid-out i64 IS the element layout (the `.cast()`
        // to `*const u64` is the same representation-identical pun the
        // source performs — see the note above); the four out-params are
        // the initialised stack locals `mk_batch` declares; the extern order
        // mirrors ffi.h:294-301. The body (ffi.cpp:532-547) writes the
        // filled count and the released malloc'd buffer on success.
        // (invoke — called by `mk_batch` — takes the getter AS its `call`
        // closure and owns the error string's release.)
        unsafe {
            sys::glean_factset_serializeReorder(
                self.ptr,
                order.as_ptr().cast::<u64>(),
                order.len(),
                first_id,
                count,
                facts_data,
                facts_size,
            )
        }
    })
}

/// `rebase :: Inventory -> Subst -> LookupCache -> FactSet -> IO (FactSet,
/// Subst)` (FactSet.hs:106-119):
///
/// * rebase inventory subst cache facts =
/// *   with inventory $ \inventory_ptr ->
/// *   with subst $ \subst_ptr ->
/// *   with cache $ \cache_ptr ->
/// *   with facts $ \facts_ptr ->
/// *   mask_ $ do
/// *     (p_factset, p_subst) <- invoke $
/// *       glean_factset_rebase
/// *         facts_ptr
/// *         inventory_ptr
/// *         subst_ptr
/// *         cache_ptr
/// *     (,) <$> create p_factset <*> create p_subst
///
/// The four `with` borrows dissolve into the four handle accessors
/// (`as_ptr`/`with_ptr` — the shipped seams); the two `create`s are the two
/// `from_raw` adoptions below. The `mask_` window has no Rust analogue
/// (ffi.rs:558-566).
///
/// Guard kind: the body (ffi.cpp:561-579 over factset.cpp:364-415) carries
/// GLEAN_SANITY_CHECKs (glog `LOG(FATAL)` — abort inside C++ and never
/// cross as a value, sanity.h:21-27) plus allocation-class (the two
/// `new`s at ffi.cpp:574-575); the wrap channel is returned as `Err` (the
/// source's exception made explicit). Both out-params are adopted on
/// success: the `FactSet*` via `FactSet::from_raw`, the `Substitution*` via
/// the shipped `Subst::from_raw` (subst.rs:329-331).
pub fn rebase(
    inventory: &Inventory,
    subst: &Subst,
    cache: &LookupCache,
    facts: &FactSet,
) -> Result<(FactSet, Subst), FfiError> {
    let mut result: *mut c_void = ptr::null_mut();
    let mut out_subst: *mut c_void = ptr::null_mut();
    // `with cache $ \cache_ptr ->` (FactSet.hs:110) — the shipped
    // `LookupCache::with_ptr` seam (lookup_cache.rs:337-339) scopes the raw
    // `LookupCache*` to the closure; the `&cache` borrow keeps the handle
    // alive for the whole call.
    cache.with_ptr(|cache_ptr| {
        // SAFETY: `facts.as_ptr()`/`inventory.as_ptr()`/`subst.as_ptr()`/
        // `cache_ptr` are live handles (each borrow keeps its owner alive for
        // the call); `result`/`out_subst` are initialised stack out-params;
        // the extern order mirrors ffi.h:305-311. The body (ffi.cpp:568-578)
        // stores the two fresh handles on success. invoke takes the extern
        // call itself as its `call` closure and owns the error string's
        // release.
        unsafe {
            invoke(|| {
                sys::glean_factset_rebase(
                    facts.as_ptr(),
                    inventory.as_ptr(),
                    subst.as_ptr(),
                    cache_ptr,
                    &mut result,
                    &mut out_subst,
                )
            })
        }
    })?;
    // SAFETY: the `?` above returned early on any error, so on this path
    // `result`/`out_subst` are the live, uniquely-owned handles the C++ body
    // just allocated (ffi.cpp:574-575 — the only writers of the two
    // out-params). That is exactly the two `from_raw` adoption contracts:
    // ownership moves into the returned handles (whose Drops call
    // glean_factset_free / glean_free_subst exactly once), and the raw
    // pointers are not used again on this side.
    Ok(unsafe { (FactSet::from_raw(result), Subst::from_raw(out_subst)) })
}

/// `append :: FactSet -> FactSet -> IO ()` (FactSet.hs:121-125):
///
/// * append target source =
/// *   with target $ \target_ptr ->
/// *   with source $ \source_ptr ->
/// *   invoke $ glean_factset_append target_ptr source_ptr
///
/// MUTATION DEVIATION (documented): the Haskell's two `with`s hand out
/// immutable ForeignPtrs and encode no mutability, but the C++ body
/// (ffi.cpp:581-583) is `target->append(std::move(*source))` — it MOVES the
/// source's facts into the target (factset.cpp:417-425), leaving the source
/// empty. The honest Rust receiver is `&mut self` for the target (the
/// inventory row: "append mutates target in place"); the source stays
/// `&FactSet`, mirroring the source's own immutable-handle posture (the
/// Haskell's `with source` likewise hides the drain — the source is simply
/// never used again at the call sites). The C++ `assert(appendable(other))`
/// (factset.cpp:418) is a C `assert` — aborts in debug builds, compiled out
/// under NDEBUG.
///
/// Guard kind: ALLOCATION-GUARD only (a vector insert + key merge; no
/// precondition check that crosses as a value). Returned as `Err`.
pub fn append(&mut self, source: &FactSet) -> Result<(), FfiError> {
    // SAFETY: `self.ptr`/`source.ptr` are live `FactSet*`s (the `&mut self`
    // / `&FactSet` borrows keep them alive for the call); the extern order
    // mirrors ffi.h:303. The body (ffi.cpp:581-583) moves the source's
    // facts into the target. invoke takes the extern call itself as its
    // `call` closure and owns the error string's release.
    unsafe { invoke(|| sys::glean_factset_append(self.ptr, source.ptr)) }
}

/// Errors of [`rename_facts`] — the Rust join of the two exception sources
/// the source function's two FFI calls had: the `glean_factset_new` invoke
/// channel (`FfiError`) and the `defineBatch` channel (`DefineBatchError`,
/// the shipped define.rs merge of the C++ `defineBatch` structural errors and
/// the Haskell-side ids/count guard).
///
/// The Haskell's `renameFacts` ran both in `IO` and let both exceptions
/// propagate; the port returns them (RULE §2 error-recovery rule: the
/// crossing C++ errors are precondition-class records, not aborts — the
/// caller decides, and SendAndRebaseQueue.hs:372-392 relies on `ignoreRedef`
/// to absorb redefinitions rather than die). Same merge-enum precedent as
/// the shipped `DefineBatchError` (define.rs:423-438).
#[derive(Debug, thiserror::Error)]
pub enum RenameFactsError {
    /// The `glean_factset_new` invoke channel (allocation-class).
    #[error("{0}")]
    Ffi(#[from] FfiError),
    /// The `defineBatch` structural errors (define.cpp:29-31, :61-65, :69-75,
    /// :99-102, :112) plus the Haskell-side ids/count guard
    /// ("mismatch between count and ids.size in batch", Define.hs:112-113).
    #[error("{0}")]
    DefineBatch(#[from] DefineBatchError),
}

/// `renameFacts :: CanLookup l => Inventory -> l -> Fid -> Thrift.Batch ->
/// DefineFlags -> IO (FactSet, Subst)` (FactSet.hs:129-141) — "Prepare a
/// Thrift batch for writing into the database by renaming and deduplicating
/// facts."
///
/// * renameFacts inventory base next batch flags = do
/// *   added <- new next
/// *   subst <- defineBatch (stacked base added) inventory batch flags
/// *   return (added, subst)
///
/// The source's two-step shape is preserved EXACTLY: `added` is created
/// FIRST (the `new next` at FactSet.hs:139), then `stacked base added`
/// borrows both handles for the duration of `define_batch` (the borrow ends
/// when `define_batch` returns), and `(added, subst)` is returned — the
/// `added` FactSet outlives the stacked view, so returning it afterwards is
/// fine. The Rust signatures express this: `added` is created first, the
/// stacked view is built with `&added` (a borrow — the two blanket impls
/// above let `&impl CanLookup`/`&FactSet` be stacked without moving the
/// owned handles, mirroring the Haskell's ForeignPtr sharing), the view is
/// consumed by the `define_batch` call, and `added` is moved into the
/// returned tuple.
///
/// `E: From<FfiError> + From<DefineBatchError>` is load-bearing: `new` and
/// `define_batch` reach the C++ through `invoke(...)?`, whose error channel
/// is `FfiError`; the `?` needs the `From` bounds to merge. (The sibling
/// traits state the same `E: From<FfiError>` bound — lookup.rs:122-131 /
/// define.rs:355-361 — because an impl cannot add it itself, E0276.)
pub fn rename_facts<E: From<FfiError> + From<DefineBatchError>>(
    inventory: &Inventory,
    base: &impl CanLookup,
    next: Fid,
    batch: &Batch,
    flags: &DefineFlags,
) -> Result<(FactSet, Subst), E> {
    let added = new(next)?;
    let subst = define_batch(&stacked(base, &added), inventory, batch, flags)?;
    Ok((added, subst))
}

// Marker breakdown (closed set — no open port TODOs, no source-defect
// reproductions, 1 performance deferral, 1 inventory-bug-candidate):
//   * no open port TODOs: everything is clear from the source and the pinned
//     rows; the one open-looking question (glean_factset_first_free_id not in
//     ffi.h) is resolved by the ABI truth (the symbol exists in ffi.cpp and
//     the Haskell imports it directly) and flagged in the header.
//   * no source-defect reproductions: no source defect is reproduced. The
//     one source quirk read on this path — the Haskell's `Ptr Int64` vs the
//     C++'s `uint64_t*` for serializeReorder's `order` (FactSet.hs:97-104 vs
//     ffi.h:296) — is a representation-identical pun the source itself
//     performs; the port mirrors it (documented at serialize_reorder), not
//     reproduces a defect.
//   * 1 performance deferral: the serialize buffer transfer copies
//     (unsafe_malloced_byte_string does copy + libc::free, ffi.rs:121-128)
//     where the source's unsafeMallocedByteString was zero-copy
//     (unsafePackMallocCStringLen, Glean/FFI.hs:42-44) — the crate's shared
//     transfer helper is the slow-but-faithful twin; flagged at mk_batch.
//   * 1 inventory-bug-candidate: two row-66 nuances, both flagged in the
//     header — (a) glean_factset_first_free_id is not declared in ffi.h
//     (only defined in ffi.cpp:506-508), a C++ header omission (the same
//     class lookup.rs flagged for glean_lookup_empty); (b) the row's guard
//     label ("precondition-guard; C++ OOM still aborts per RTS") is
//     inaccurate per the ABI bodies — the channels are allocation-class and
//     wrap.h returns bad_alloc as a value — the port returns `Err` per the
//     row's headline "error strings -> Result" and flags the label here.
// SAFETY count: 18 unsafe constructs, each with a SAFETY comment — 17
// blocks: Drop 1, CanLookup::with_lookup 1, CanDefine::with_define 1, new 2
// (invoke + from_raw), fact_count 1, fact_memory 1, allocated_memory 1,
// first_free_id 1, predicate_stats 1 (the extern call inside the
// marshal_predicate_stats closure), serialize 1 (the extern call inside the
// mk_batch closure), serialize_reorder 1 (same), mk_batch 2 (invoke +
// unsafe_malloced_byte_string), rebase 2 (invoke + the two from_raw
// adoptions), append 1; plus the `unsafe fn from_raw` seam (its SAFETY is
// the adoption contract in its doc comment). (The `pub mod sys`
// `unsafe extern` block is a declaration, not counted — family convention.)
// PORT STATUS: confidence=high todos=0</longcat_think>
