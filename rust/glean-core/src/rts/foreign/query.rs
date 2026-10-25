// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// Query — execute a compiled bytecode query against the C++ RTS and unpack
// the C++ `QueryResults` struct (`executeCompiled`, `restartCompiled`,
// `interruptRunningQueries`).
//
// Ported from `glean/hs/Glean/RTS/Foreign/Query.hsc` (305 lines).
//
// Manifest unit: rust/glean-core/src/rts/foreign/query.rs
// (migration/manifest.tsv line 94: `glean/hs/Glean/RTS/Foreign/Query.hsc
// -> rust/glean-core/src/rts/foreign/query.rs`, crate glean-core). RULE
// §4: one source file -> one target file; the manifest path is
// authoritative. This line is also the RESOLUTION of the inventory rows'
// TODO(port) — see the INVENTORY block.
//
// ABI truth (read, not assumed):
//   * glean/rts/query.h — `struct QueryResults` :27-40 (the EXACT field
//     list/order/types the `#[repr(C)]` mirror below reproduces) and
//     `enum class Depth { ResultsOnly, ExpandRecursive, ExpandPartial }`
//     :42 (no explicit initializers -> values 0/1/2 by the C++ rule;
//     the hsc's `(#const (int)...` casts, Query.hsc:254-264, read exactly
//     these).
//   * glean/rts/ffi.h (the whole block is `extern "C" {` from :102) —
//     glean_query_execute_compiled :171-187 (`SharedSubroutine* sub` :175,
//     `uint64_t pid` :176, `SharedSubroutine* traverse` :177, ...
//     `uint64_t* expand_pids` :183, `QueryResults** presults` :186; the
//     header names the depth parameter `recursive` :182 — the hsc calls it
//     depth), glean_query_restart_compiled :188-202 (`void* cont` :192,
//     `int64_t cont_size` :193), glean_free_query_results :379.
//   * glean/rts/ffi.cpp — the bodies: glean_interrupt_running_queries
//     :221-223 (defined inside the file's `extern "C" {` block opened at
//     :69; NOT declared in ffi.h — the lookup.rs:344 precedent for
//     ffi.cpp-only symbols), glean_query_execute_compiled :225-268
//     (ffi::wrap; builds the `std::unordered_set` from `expand_pids` when
//     non-null :242-247; writes `*presults` ONLY on success :248),
//     glean_query_restart_compiled :270-311 (same shape, :315),
//     glean_free_query_results :820-822 (ffi::free_ -> wrap_, which
//     swallows every exception, wrap.h:34-45 — infallible). Both query
//     bodies forward 0 -> folly::none for the four maxima (:240-247 /
//     :284-291) — that 0-sentinel mapping is the CALLER's job here, same
//     as the source.
//   * hsthrift `cpp/HsStruct.h` (on disk at
//     ~/glean-build/hsthrift/common/util/cpp/HsStruct.h — the checkout's
//     declared source dep, not under src.parser) — the C++ layouts the
//     mirror reproduces: `HsString` :345-348 (`std::string s_; const
//     char* str; size_t len;`), `HsArray<T>` :444-448 (`std::vector<T>
//     v_; const T* a; size_t n;`), `HsMap<K,V>` :708-713 (`std::vector<K>
//     k_; std::vector<V> v_; const K* keys; const V* values; size_t n;`),
//     all `HS_STRUCT` = plain `struct` (HsStructDefines.h:9-13, no
//     packing), size-uniformity static_asserts :546-551/:876-880. The
//     Haskell peeks read ONLY the cursor fields: `peekDataSize`
//     (Foreign/CPP/HsStruct/Types.hsc:467-471 — `a` + `n`),
//     `peekStrLen` (:225-229 — `str` + `len`), `peekMapWith` (:597-616 —
//     `keys` + `values` + `n`).
//
// LAYOUT PROBE (/tmp/query_probe/: cxx_layout.cpp + rs_layout.rs,
// built + run; outputs identical) (scratch probe, sanctioned by the unit brief): a C++ probe
// transcribing HsStruct.h's member declarations verbatim + query.h:27-40's
// field order, compiled with the tree's g++ (libstdc++, x86-64), and a
// Rust probe of the mirror below (rustc, `offset_of!`) print IDENTICAL
// numbers — sizeof(std::string)=32, std::vector=24, HsString=48,
// HsArray<T>=40 (for BOTH T=uint64_t and T=HsString, per the header's own
// static_asserts), HsMap<u64,u64>=72, QueryResults=456 align 8, and
// offsetof(QueryResults, ...) = 0/40/80/120/160/200/240/280/320/392/400/
// 448; HsString.str=32/.len=40; HsArray.a=24/.n=32; HsMap.keys=48/
// .values=56/.n=64. These are the offsets the hsc `#peek`s compute
// (inventory row 72: "hsc #peek offsets are the proof").
//
// RULE citations:
//   §1 (FFI row, A2 + §2 hsc2hs row): the hsc2hs module becomes manual
//     `extern "C"` declarations; raw extern "C" for flat C-ABI symbols.
//   §2 error-recovery rule: the one guard-kind story in the `//!` block;
//     numeric conversions explicit (`as` with the source's wrap
//     semantics).
//   §3: `unsafe` only inside this glean-core FFI wrapper, where the
//     Haskell was already FFI; every block carries a SAFETY comment.
//   §4: one source file -> one target file; assumed-import regime for
//     the two markers/records this file consumes but does not own.
//   §5: the inventory rows below, followed.
//
// INVENTORY (RULE §5) — the row governing this file, followed EXACTLY:
//   * inventory.tsv row 72 (line 72; duplicated as inventory.ffi.tsv row
//     30): source glean/hs/Glean/RTS/Foreign/Query.hsc, symbols
//     "glean_query_execute_compiled / glean_query_restart_compiled /
//     glean_interrupt_running_queries / glean_free_query_results",
//     OWNED_MUTABLE, target_translation: "manual extern "C" (RULEBOOK §2
//     hsc2hs row); define #[repr(C)] mirror of QueryResults
//     (glean/rts/query.h) — layout MUST match C++ (hsc #peek offsets are
//     the proof); unpack to owned QueryResults then free; HsArray/
//     HsString -> owned Vec/Vec<u8> copies; cont=="" -> None
//     (Query.hsc:243-247)". Every clause is implemented below: the
//     `pub mod sys` mirror (layout-proven), the unpack in
//     `unpack_results`, the `Static`/`using` bracket (Query.hsc:49-50 +
//     :117-134), and the empty-continuation -> None rule. The row's own
//     TODO(port) ("this hsc module is ABSENT from manifest.tsv — q: which
//     target file owns these externs") is RESOLVED by the manifest
//     itself: migration/manifest.tsv line 94 pins
//     `glean/hs/Glean/RTS/Foreign/Query.hsc ->
//     rust/glean-core/src/rts/foreign/query.rs` (the row's evidence
//     column already notes "manifest target exists"). Marked FOLLOWED —
//     no divergence, no INVENTORY-BUG-CANDIDATE.
//   * inventory.sites.tsv row 662: "hsc2hs | glean/hs/Glean/RTS/Foreign/
//     Query.hsc | 1 | hsc2hs binding file | (entire .hsc file)" — the
//     whole-file construct site; the entire file maps onto the manual
//     extern "C" + mirror + unpack image above. Followed.
//
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, module-level): outbound
// rows 1273 (Query.hsc -> Glean/FFI.hs, crate::ffi — the
// `using`/`invoke` pair the hsc imports from there, Query.hsc:37), 1274
// (-> RTS/Foreign/Bytecode.hs, crate::rts::foreign::bytecode), 1275
// (-> RTS/Foreign/Define.hs, crate::rts::foreign::define), 1276 (-> RTS/
// Foreign/Inventory.hs, crate::rts::foreign::inventory), 1277 (-> RTS/
// Foreign/Ownership.hsc, crate::rts::foreign::ownership), 1278 (-> RTS/
// Traverse.hs — the `CompiledTraversal` phantom marker ONLY, see the
// ASSUMED-IMPORT block), 1279 (-> RTS/Types.hs, crate::rts::types —
// Fid/Pid). Inbound rows 544 (db/Glean/Query/Codegen.hs), 637 (db/Glean/
// Query/Transform.hs), 707 (db/Glean/Query/UserQuery.hs) — the db-crate
// consumers of executeCompiled/restartCompiled; they obtain CompiledQuery
// from codegen and call the two functions below. Plus the generated
// thrift `Glean.Types` import (Query.hsc:44, the `Fact` type) and the
// hsthrift kit imports (Query.hsc:34-35 Util.FFI + Foreign.CPP.HsStruct.
// Types — `invoke`/`using` and the HsArray/HsString/HsMap peeks), whose
// Rust landings are crate::ffi and the sys mirror below.
//
// SIBLING-UNIT CONTRACT — `CompiledTraversal` (CORRECTED at review: the
// first draft treated this as an ASSUMED-IMPORT of an unshipped unit,
// but the marker IS ALREADY SHIPPED at rust/glean-core/src/rts/foreign/
// inventory.rs:258-259 (`pub struct CompiledTraversal;`, with the
// relocation TODO at :257 — "move to rts/typecheck.rs (unit 20) and
// rts/traverse.rs (unit 19) when those units ship"). The source imports
// Glean.RTS.Traverse (Query.hsc:42) for this name alone — it appears
// only as the phantom marker of `Maybe (Subroutine CompiledTraversal)`
// (Query.hsc:83/:272). This file consumes the SHIPPED tag from
// inventory.rs (no second owner, no dangling path); when units 19/20
// ship and relocate the tags, THOSE units update their consumers.
//
// ASSUMED-IMPORT (RULE §4 B4) #2 — `glean_thrift::Fact`: NO extension of
// the surface; the first sketch is OWNED by typed/build.rs:104-105
//   pub struct Fact { pub r#type: Id, pub key: Value, pub value: Value }
// and consumed by typed/binary.rs:259-260 and typed/fact.rs:74-92 (the
// `r#type` spelling cross-referenced there; `Value` = thrift
// `typedef binary Value`, glean/if/glean.thrift:143-144, byte-string
// image; `struct Fact { 1: Id type; 2: Value key; 3: Value value }`
// :147-151). The Haskell constructs positionally:
// `Fact pid (hsByteString key) (hsByteString value)` (Query.hsc:207/:211)
// — i.e. field 1 `type` gets the predicate id and fields 2/3 the RAW
// key/value bytes (never decoded). This file constructs the same record
// with named fields and adds nothing to the sketch.
//
// SIBLING-UNIT CONTRACTS (RULE §4; DONE units' real APIs read, never
// re-declared):
//   * crate::ffi (DONE, read): `invoke` (the pinned form `pub unsafe fn
//     invoke(call: impl FnOnce() -> *const c_char) -> Result<(),
//     FfiError>`, ffi.rs:394-402 — the extern call passed AS the
//     closure), `FfiError` (:316-320), `Static` + `using` (:598-625 —
//     the source's `class Static`/`using` bracket that Query.hsc:49-50/
//     :117-134 instantiate; `using` is the bracket, `destroy_static` the
//     finalizer), `with` (:582-585, the ForeignPtr-object borrow used for
//     DefineOwnership), and `unsafe_with_bytes` (:85-87, the source's
//     `Util.FFI.unsafeWithBytes` at Query.hsc:149).
//   * crate::rts::foreign::bytecode (DONE, read): `Subroutine<S>` (the
//     ForeignPtr-backed handle with the `S` phantom, bytecode.rs:259-262),
//     `Subroutine::with_ptr` (:325-332 — the ONLY pointer path; the
//     contract at bytecode.rs:99-112 names THIS file as a consumer of
//     `pub mod sys`'s `SharedSubroutine` for "ffi.h:171+ ... the
//     Query.hsc eval-restart family, `SharedSubroutine* sub`/`traverse`
//     at :175/:177"). The externs below declare `*mut
//     bytecode::sys::SharedSubroutine` (the contract's `*const` spelling
//     was for typecheck.rs's `const SharedSubroutine*` param; the query
//     symbols take non-const `SharedSubroutine*`, ffi.h:175/:177, and
//     with_ptr hands out exactly that).
//   * crate::rts::foreign::define (DONE, read): `CanDefine` +
//     `CanDefine::with_define` (define.rs:355-361 — the `E: From<
//     FfiError>` bound is the trait's; `FfiError` satisfies it for the
//     closures below), so `execute_compiled`/`restart_compiled` are
//     generic over the fact source exactly as the source's
//     `CanDefine a =>` (Query.hsc:93/:137).
//   * crate::rts::foreign::inventory (DONE, read): `Inventory` +
//     `Inventory::as_ptr` (inventory.rs:335-337, the marshalling seam
//     added for the define family; same need here for `Inventory*`,
//     ffi.h:172/:189).
//   * crate::rts::foreign::ownership (DONE, read): `DefineOwnership`
//     (ownership.rs:440-459, a ForeignPtr-backed `Object`) — borrowed
//     through crate::ffi::with, the same route ownership.rs itself uses
//     for `Ownership` (:540).
//   * crate::rts::types (DONE, read): `Fid`/`Pid` — both
//     `#[repr(transparent)]` over i64 (types.rs:121-122/:154-155; the
//     Haskell's `Pid = Pid Int64` at RTS/Types.hs:51 and `Fid = Fid Id`
//     :70 with `type Id = Int64` in the generated module). The u64 values
//     read out of the mirror are bit-cast `as i64` exactly where the
//     Haskell's Storable reads were Int64-typed.
//   * glean_thrift (generated; ASSUMED-IMPORT #2 above): `Fact`.

//! Execute a compiled bytecode query against the C++ RTS
//! ([`execute_compiled`] / [`restart_compiled`]) and unpack the returned
//! C++ `QueryResults` struct into owned Rust data; plus
//! [`interrupt_running_queries`].
//!
//! Port notes: RULE §3 — `unsafe` appears only inside this glean-core FFI
//! wrapper, where the Haskell was already FFI (the four foreign imports,
//! Query.hsc:266-305, and the struct-field peeks they enable); every
//! block carries a SAFETY comment. Error-recovery classification per
//! RULE §2 (error-recovery rule):
//!   * `glean_query_execute_compiled` / `glean_query_restart_compiled` —
//!     PRECONDITION-GUARD dominant: the ffi::wrap'd bodies forward to
//!     `executeQuery`/`restartQuery` (ffi.cpp:225-268/:270-311), where the
//!     query engine raises structural `rts::error`s (typecheck failures,
//!     unknown predicates, malformed continuations) that cross the wrap
//!     channel as returned messages; wrap.h merges those with
//!     allocation-class failures into the one channel, and the source
//!     threw FFIError for both — so the port surfaces the channel as
//!     `Err(FfiError)` (the define_batch pattern), never an abort.
//!   * `glean_free_query_results` — void, ffi::free_ body (:820-822),
//!     wrap_ swallows all exceptions: infallible, run from the
//!     `Static`/`using` Drop guard.
//!   * `glean_interrupt_running_queries` — void, no error slot
//!     (ffi.cpp:221-223): infallible.
//! The unpack itself (`unpack_results`) is total over a successfully
//! returned `QueryResults*` — the C++ struct's cursor fields are
//! maintained by the HsStruct `update()` discipline on the C++ side —
//! and performs only owned copies (the row's "HsArray/HsString -> owned
//! Vec/Vec<u8> copies").

use std::collections::{BTreeMap, BTreeSet};
use std::os::raw::c_void;
use std::ptr;

use glean_thrift::Fact;

use crate::ffi::{FfiError, Static, invoke, unsafe_with_bytes, using, with};
use crate::rts::foreign::bytecode::Subroutine;
use crate::rts::foreign::define::CanDefine;
use crate::rts::foreign::inventory::Inventory;
use crate::rts::foreign::ownership::DefineOwnership;
use crate::rts::types::{Fid, Pid};

// The phantom marker: SHIPPED at foreign/inventory.rs:258-259 (see the
// SIBLING-UNIT CONTRACT above).
use crate::rts::foreign::inventory::CompiledTraversal;

/// Raw C-ABI declarations and the `#[repr(C)]` mirror for the query
/// family (glean/rts/ffi.h:171-202/:379 + glean/rts/query.h:27-42).
///
/// RULE §1 (FFI row, A2) + §2 (hsc2hs row): manual `extern "C"`
/// declarations replacing the hsc2hs foreign imports (Query.hsc:266-305);
/// the mirror is the `#peek` target (inventory row 72: "layout MUST match
/// C++ — hsc #peek offsets are the proof"; proven identical by the
/// layout probe recorded in the header).
///
/// Signatures mirror ffi.h EXACTLY (the "ABI truth is ffi.h, not the .hs"
/// convention of bytecode.rs/define.rs), with the family's handle
/// modelling: `Inventory*`/`Define*`/`DefineOwnership*` are opaque
/// `c_void` pointers (the define.rs:233-235 / lookup.rs / ownership.rs
/// convention — never dereferenced on this side); the two subroutine
/// handles are the bytecode-sys typed pointers. One Haskell-vs-header
/// divergence, per that convention: the hsc types restart's `cont_size`
/// as `CSize` (Query.hsc:289) where the header says `int64_t` (ffi.h:
/// 193) — ABI-identical on every LP64 target (both 8 bytes; the value is
/// a ByteString length, non-negative by construction); the port uses the
/// header's `i64` and converts with the same bit pattern. Guard kinds
/// are recorded at each symbol; the call sites below own the
/// dispositions.
pub mod sys {
    use std::os::raw::{c_char, c_void};

    use crate::rts::foreign::bytecode::sys as bytecode_sys;

    // NOTE (RULE §3, family convention, define.rs:248-252): the `unsafe
    // extern "C"` block below is a DECLARATION, not an operation — it
    // introduces no unsafe behaviour of its own, carries no SAFETY
    // comment, and is NOT counted in the SAFETY tally at the foot of
    // this file; every USE below carries its own.

    /// The libstdc++ `std::vector<T>` placeholder: three pointers
    /// (`_M_start`, `_M_finish`, `_M_end_of_storage`, bits/stl_vector.h).
    /// Size 24 / align 8 on the target ABI — probe-verified. NEVER
    /// touched on this side: it is the C++-owned storage that `HsArray`'s
    /// cursor fields (`a`/`n`) point into.
    #[repr(C)]
    pub struct CxxVector<T> {
        _start: *mut T,
        _finish: *mut T,
        _end_of_storage: *mut T,
    }

    /// The libstdc++ `std::string` placeholder: pointer + length + the
    /// 16-byte SSO buffer (`_M_dataplus`, `_M_string_length`,
    /// `_M_local_buf`, bits/basic_string.h). Size 32 / align 8 —
    /// probe-verified. NEVER touched on this side.
    #[repr(C)]
    pub struct CxxString {
        _dataplus: *mut c_char,
        _length: usize,
        _local_buf: [u8; 16],
    }

    /// Mirror of hsthrift `HsArray<T>` (cpp/HsStruct.h:444-448):
    /// `std::vector<T> v_; const T* a; size_t n;` — size 40 / align 8 for
    /// EVERY T (the header's own `sizeof(HsArray<HsString>) ==
    /// sizeof(DummyHsArray)` static_assert, :548-550). The Haskell peeks
    /// read only `a` + `n` (`peekDataSize`, Types.hsc:467-471); the
    /// vector member is opaque.
    #[repr(C)]
    pub struct HsArray<T> {
        /// C++ `std::vector<T> v_` — opaque storage (see [`CxxVector`]).
        pub v_: CxxVector<T>,
        /// C++ `const T* a` — element cursor the peeks read.
        pub a: *const T,
        /// C++ `size_t n` — element count the peeks read.
        pub n: usize,
    }

    /// Mirror of hsthrift `HsString` (cpp/HsStruct.h:345-348):
    /// `std::string s_; const char* str; size_t len;` — size 48 /
    /// align 8. The Haskell peeks read only `str` + `len`
    /// (`peekStrLen`, Types.hsc:225-229).
    #[repr(C)]
    pub struct HsString {
        /// C++ `std::string s_` — opaque storage (see [`CxxString`]).
        pub s_: CxxString,
        /// C++ `const char* str` — bytes cursor the peeks read.
        pub str: *const c_char,
        /// C++ `size_t len` — byte count the peeks read.
        pub len: usize,
    }

    /// Mirror of hsthrift `HsMap<K, V>` (cpp/HsStruct.h:708-713):
    /// `std::vector<K> k_; std::vector<V> v_; const K* keys; const V*
    /// values; size_t n;` — size 72 / align 8 for every K/V pair (the
    /// header's uniform-size static_asserts, :876-880). The Haskell
    /// peeks read only `keys` + `values` + `n` (`peekMapWith`,
    /// Types.hsc:597-616).
    #[repr(C)]
    pub struct HsMap<K, V> {
        /// C++ `std::vector<K> k_` — opaque.
        pub k_: CxxVector<K>,
        /// C++ `std::vector<V> v_` — opaque.
        pub v_: CxxVector<V>,
        /// C++ `const K* keys` — key cursor the peeks read.
        pub keys: *const K,
        /// C++ `const V* values` — value cursor the peeks read.
        pub values: *const V,
        /// C++ `size_t n` — entry count the peeks read.
        pub n: usize,
    }

    /// Mirror of `facebook::glean::rts::QueryResults` (query.h:27-40) —
    /// field order/types EXACTLY as the C++ struct; `uint64_t` stays
    /// `u64` here (the Int64-typed reads the Haskell performed are the
    /// unpack's `as i64` bit-casts, not the layout). Layout proven
    /// identical by the probe recorded in the file header (456 bytes,
    /// align 8).
    #[repr(C)]
    pub struct QueryResults {
        pub fact_ids: HsArray<u64>,
        pub fact_pids: HsArray<u64>,
        pub fact_keys: HsArray<HsString>,
        pub fact_values: HsArray<HsString>,
        pub nested_fact_ids: HsArray<u64>,
        pub nested_fact_pids: HsArray<u64>,
        pub nested_fact_keys: HsArray<HsString>,
        pub nested_fact_values: HsArray<HsString>,
        /// C++ `HsMap<uint64_t, uint64_t>` (query.h:36).
        pub stats: HsMap<u64, u64>,
        pub elapsed_ns: u64,
        /// C++ `HsString continuation` (query.h:38).
        pub continuation: HsString,
        pub result_bytes: u64,
    }

    unsafe extern "C" {
        // ffi.h:171-187 — the depth parameter is spelled `recursive` in
        // the header (:182); the hsc calls it depth (Query.hsc:277). Body
        // ffi.cpp:225-268: ffi::wrap around executeQuery; builds the
        // expand set only when `expand_pids` is non-null (:242-247);
        // writes `*presults` ONLY on success (:267). PRECONDITION-GUARD
        // dominant (query-engine structural errors) + allocation-class
        // failures merged by wrap.h — surfaced as Err by the caller.
        pub fn glean_query_execute_compiled(
            inventory: *mut c_void,
            facts: *mut c_void,
            ownership: *mut c_void,
            sub: *mut bytecode_sys::SharedSubroutine,
            pid: u64,
            traverse: *mut bytecode_sys::SharedSubroutine,
            max_results: u64,
            max_bytes: u64,
            max_time_ms: u64,
            max_set_size: u64,
            depth: u64,
            expand_pids: *mut u64,
            num_expand_pids: u64,
            want_stats: u64,
            presults: *mut *mut QueryResults,
        ) -> *const c_char;

        // ffi.h:188-202 — `cont_size` is `int64_t` in the header (:193);
        // the hsc imported it as CSize (SOURCE-vs-HEADER divergence noted
        // at the block). Body ffi.cpp:270-311, same shape as execute.
        pub fn glean_query_restart_compiled(
            inventory: *mut c_void,
            facts: *mut c_void,
            ownership: *mut c_void,
            cont: *mut c_void,
            cont_size: i64,
            max_results: u64,
            max_bytes: u64,
            max_time_ms: u64,
            max_set_size: u64,
            depth: u64,
            expand_pids: *mut u64,
            num_expand_pids: u64,
            want_stats: u64,
            presults: *mut *mut QueryResults,
        ) -> *const c_char;

        // ffi.cpp:221-223 — NOT declared in ffi.h; defined inside
        // ffi.cpp's `extern "C" {` block (:69). Body is a straight call
        // of interruptRunningQueries(): no error slot, infallible.
        pub fn glean_interrupt_running_queries();

        // ffi.h:379 — void; body ffi.cpp:820-822 is ffi::free_ (wrap_
        // swallows all exceptions) — infallible; the Static finalizer.
        pub fn glean_free_query_results(results: *mut QueryResults);
    }
}

// `depth_ResultsOnly = (#const (int)facebook::glean::rts::Depth::ResultsOnly)`
// etc. (Query.hsc:254-264). `enum class Depth { ResultsOnly,
// ExpandRecursive, ExpandPartial }` (query.h:42) has no explicit
// initializers, so the `(int)` casts read 0/1/2; the constants pin those
// values to the header.
const DEPTH_RESULTS_ONLY: u64 = 0;
const DEPTH_EXPAND_RECURSIVE: u64 = 1;
const DEPTH_EXPAND_PARTIAL: u64 = 2;

/// `data Depth = ResultsOnly | ExpandRecursive | ExpandPartial (Set Pid)`
/// (Query.hsc:53-59) — "Specifies the depth to which we expand nested
/// facts in the results" (source doc).
///
/// The `Set Pid` maps to `BTreeSet<Pid>`: Data.Set is ordered, and
/// `Set.toList` — the marshal point (Query.hsc:181) — is ASCENDING, so
/// the ordered set IS the source representation, and BTreeSet's ascending
/// iteration reproduces the wire order of `expand_pids` exactly (the C++
/// side folds the array into an unordered_set, so the order is not
/// semantically load-bearing there — kept ascending anyway, bug-for-bug
/// on the bytes handed over).
///
/// No deriving clause in the source — no derives here (RULE §0).
pub enum Depth {
    /// Don't expand any nested facts in the results. (Source doc.)
    ResultsOnly,
    /// Expand all nested facts recursively. (Source doc.)
    ExpandRecursive,
    /// Only expand nested facts with these Pids. (Source doc.)
    ExpandPartial(BTreeSet<Pid>),
}

/// `data QueryRuntimeOptions = QueryRuntimeOptions {..}` (Query.hsc:61-68).
/// Field names snake_cased per house style; no deriving clause in the
/// source — no derives here.
pub struct QueryRuntimeOptions {
    /// `queryMaxResults :: Maybe Int64` — `Nothing` marshals as the 0
    /// sentinel (`fromMaybe 0`, Query.hsc:107; the C++ side maps 0 back
    /// to folly::none, ffi.cpp:240-241).
    pub query_max_results: Option<i64>,
    /// `queryMaxBytes :: Maybe Int64` (same 0-sentinel, Query.hsc:108).
    pub query_max_bytes: Option<i64>,
    /// `queryMaxTimeMs :: Maybe Int64` (same 0-sentinel, Query.hsc:109).
    pub query_max_time_ms: Option<i64>,
    /// `queryMaxSetSize :: Maybe Int64` (same 0-sentinel, Query.hsc:110).
    pub query_max_set_size: Option<i64>,
    /// `queryDepth :: Depth` (Query.hsc:66).
    pub query_depth: Depth,
    /// `queryWantStats :: Bool` (Query.hsc:67; marshals as 1/0,
    /// Query.hsc:132).
    pub query_want_stats: bool,
}

/// `data CompiledQuery = CompiledQuery {..}` (Query.hsc:79-90).
///
/// The subroutine's phantom names the query record ITSELF
/// (`compiledQuerySub :: Subroutine CompiledQuery`, Query.hsc:80 — the
/// record's own type, recursively): the shipped `Subroutine<S>` is
/// generic over any marker (bytecode.rs:239-262), so the field is
/// `Subroutine<CompiledQuery>` with no separate marker type. The
/// traversal subroutine's phantom names the traversal type
/// (`CompiledTraversal`, the ASSUMED-IMPORT marker above).
pub struct CompiledQuery {
    /// `compiledQuerySub :: Subroutine CompiledQuery` (Query.hsc:80).
    pub compiled_query_sub: Subroutine<CompiledQuery>,
    /// `compiledQueryResultPid :: Maybe Pid` — "If all the result facts
    /// have the same Pid, this is it." (source doc, Query.hsc:81-82.)
    /// Drives BOTH the `pid` argument (Query.hsc:123) and the unpack's
    /// pid-skip optimization (:188-192).
    pub compiled_query_result_pid: Option<Pid>,
    /// `compiledQueryResultTraversal :: Maybe (Subroutine
    /// CompiledTraversal)` — "The query engine needs to know how to
    /// traverse the query results to expand nested facts. If the result
    /// is not an existing predicate type then we have to pass in a
    /// bespoke CompiledTraversal subroutine." (source doc, Query.hsc:83-87.)
    pub compiled_query_result_traversal: Option<Subroutine<CompiledTraversal>>,
    /// `compiledQueryFullScans :: [Pid]` — "whether the query performs a
    /// full predicate scan" (source doc, Query.hsc:88-89). Carried for
    /// record parity: the two FFI calls below never read it (Query.hsc
    /// :92-174 use only the other three fields).
    pub compiled_query_full_scans: Vec<Pid>,
}

/// `data QueryResults = QueryResults {..}` (Query.hsc:70-77) — the OWNED
/// record the row's "unpack to owned QueryResults then free" mandates.
///
/// Field mappings (all snake_cased per house style):
///   * `queryResultsFacts :: Vector (Fid, Thrift.Fact)` ->
///     `Vec<(Fid, Fact)>` — the zip of the C++ fact_ids/fact_pids/
///     fact_keys/fact_values arrays (or the pid-known 3-array zip).
///   * `queryResultsStats :: Maybe (Map Int64 Int64)` ->
///     `Option<BTreeMap<i64, i64>>` — Data.Map -> BTreeMap (the house
///     convention; the C++ member is `HsMap<uint64_t, uint64_t>`,
///     query.h:36, and the Haskell's Int64-typed peeks are the bit-casts
///     below).
///   * `queryResultsElapsedNs :: Word64` -> `u64` (direct).
///   * `queryResultsCont :: Maybe ByteString` -> `Option<Vec<u8>>` —
///     `None` iff the continuation is EMPTY (Query.hsc:243-247, the row's
///     "cont==\"\" -> None").
///   * `queryResultsResultBytes :: Int64` -> `i64` — the C++
///     `uint64_t` bit-cast (the Haskell peek was Int64-typed).
pub struct QueryResults {
    /// `queryResultsFacts` (Query.hsc:71).
    pub query_results_facts: Vec<(Fid, Fact)>,
    /// `queryResultsNestedFacts` (Query.hsc:72).
    pub query_results_nested_facts: Vec<(Fid, Fact)>,
    /// `queryResultsStats` (Query.hsc:73).
    pub query_results_stats: Option<BTreeMap<i64, i64>>,
    /// `queryResultsElapsedNs` (Query.hsc:74).
    pub query_results_elapsed_ns: u64,
    /// `queryResultsCont` (Query.hsc:75).
    pub query_results_cont: Option<Vec<u8>>,
    /// `queryResultsResultBytes` (Query.hsc:76).
    pub query_results_result_bytes: i64,
}

/// `newtype Results = Results (Ptr Results) deriving(Storable)`
/// (Query.hsc:46-47) with `instance Static Results where destroyStatic =
/// glean_free_query_results` (:49-50) — the raw bracket handle, NOT in
/// the source's export list, private here too.
struct Results(*mut sys::QueryResults);

impl Static for Results {
    fn destroy_static(&self) {
        // SAFETY: `self.0` is the live `QueryResults*` the C++ call wrote
        // on success (ffi.cpp:248/:292-309 — written ONLY on success, so the
        // bracket is constructed exactly once per successful call and
        // never for a failed one: `using`'s mk runs after the `?`);
        // glean_free_query_results (ffi.cpp:820-822, ffi::free_ -> wrap_)
        // is the matching finalizer and cannot throw across.
        unsafe { sys::glean_free_query_results(self.0) }
    }
}

/// `executeCompiled :: CanDefine a => Inventory -> Maybe DefineOwnership
/// -> a -> CompiledQuery -> QueryRuntimeOptions -> IO QueryResults`
/// (Query.hsc:92-99).
///
/// Marshalling (Query.hsc:100-134), bracket for bracket:
///   * `withDefine facts $ \facts_ptr` -> `facts.with_define` (the
///     CanDefine trait's pointer scope);
///   * `with inventory $ \inventory_ptr` -> `inventory.as_ptr()` (the
///     shipped seam; the handle is borrowed for the whole call);
///   * `with compiledQuerySub $ \sub_ptr` and `withTraversal` ->
///     `Subroutine::with_ptr` extractions — the source's nested `with`
///     brackets are held open by BORROW here: each handle is a borrowed
///     parameter that outlives the FFI call, so the extracted raw pointer
///     is valid for exactly the call (the inventory.rs:404-419
///     reasoning, per pointer, in the SAFETY comments);
///   * `maybe ($ nullPtr) with ownership` -> [`with_ownership_ptr`];
///   * the four `max*` = `fromIntegral (fromMaybe 0 ...)` (Query.hsc:
///     107-110) — the explicit `as u64` bit-casts below;
///   * `withDepth` -> [`with_depth`];
///   * `using (invoke $ \presults -> glean_query_execute_compiled ...)
///     (unpackResults ...)` (Query.hsc:117-134) -> the `invoke` + `using`
///     pair at the bottom.
///
/// Guard kind: PRECONDITION-GUARD dominant (see the `//!` block) — the
/// invoke channel is surfaced as `Err(FfiError)`, the source's thrown
/// FFIError made explicit; nothing is freed on the Err path because the
/// C++ writes `*presults` only on success (ffi.cpp:248).
pub fn execute_compiled(
    inventory: &Inventory,
    ownership: Option<&DefineOwnership>,
    facts: &impl CanDefine,
    compiled: &CompiledQuery,
    options: &QueryRuntimeOptions,
) -> Result<QueryResults, FfiError> {
    facts.with_define(|facts_ptr| {
        let inventory_ptr = inventory.as_ptr();
        // `maxr = fromIntegral (fromMaybe 0 queryMaxResults) :: Word64`
        // (Query.hsc:107-110) — i64 -> u64 bit-cast, the source's
        // fromIntegral; the C++ maps 0 back to folly::none.
        let maxr = options.query_max_results.unwrap_or(0) as u64;
        let maxb = options.query_max_bytes.unwrap_or(0) as u64;
        let maxt = options.query_max_time_ms.unwrap_or(0) as u64;
        let maxs = options.query_max_set_size.unwrap_or(0) as u64;
        // `maybe 0 (fromIntegral . fromPid) compiledQueryResultPid`
        // (Query.hsc:123) — fromPid :: Int64 -> fromIntegral :: Word64.
        let pid = compiled
            .compiled_query_result_pid
            .map_or(0, |p| p.0 as u64);
        // `(if queryWantStats then 1 else 0)` (Query.hsc:132).
        let want_stats = if options.query_want_stats { 1 } else { 0 };
        with_ownership_ptr(ownership, |ownership_ptr| {
            // SAFETY: the pointer is read out of the live handle behind
            // the borrowed `compiled.compiled_query_sub` (with_ptr's
            // contract) and used only for the FFI call within this
            // function, which the borrow outlives — the same
            // held-open-by-borrow extraction inventory.rs:404-419
            // performs for the source's withMany-held brackets. Not
            // freed, not retained past the call.
            let sub_ptr = unsafe { compiled.compiled_query_sub.with_ptr(|p| p) };
            let traversal_ptr = match &compiled.compiled_query_result_traversal {
                // `withTraversal = case ... of Nothing -> ($ nullPtr)`
                // (Query.hsc:111-113).
                None => ptr::null_mut(),
                // SAFETY: as the sub_ptr extraction, for the traversal
                // handle borrowed from `compiled` — valid through the FFI
                // call below; the C++ side maps a null traverse to no
                // traversal (ffi.cpp:245: `traverse ? traverse->value :
                // nullptr`).
                Some(sub) => unsafe { sub.with_ptr(|p| p) },
            };
            with_depth(&options.query_depth, |depth, expand_pids, num_expand_pids| {
                let mut presults: *mut sys::QueryResults = ptr::null_mut();
                // SAFETY: every argument is valid for the call —
                // inventory_ptr/facts_ptr/ownership_ptr from the live
                // borrowed handles (as_ptr / with_define /
                // with_ownership_ptr), sub_ptr/traversal_ptr extracted
                // above from handles the borrows keep alive, expand_pids
                // owned by with_depth's closure scope (its Vec lives
                // across this call), presults an initialised stack
                // out-param. The extern's parameter order mirrors
                // ffi.h:171-187 exactly. `invoke` takes the extern call
                // AS its closure and owns the error string's release
                // (crate::ffi::invoke contract); on Err the C++ left
                // *presults untouched (ffi.cpp:292-309 writes only on success), so there is nothing
                // to free.
                unsafe {
                    invoke(|| {
                        sys::glean_query_execute_compiled(
                            inventory_ptr,
                            facts_ptr,
                            ownership_ptr,
                            sub_ptr,
                            pid,
                            traversal_ptr,
                            maxr,
                            maxb,
                            maxt,
                            maxs,
                            depth,
                            expand_pids,
                            num_expand_pids,
                            want_stats,
                            &mut presults,
                        )
                    })
                }?;
                // `using (invoke $ ...) (unpackResults ...)` — the
                // Static bracket (Query.hsc:117-134): free on every exit
                // path, unpack under it.
                Ok(using(
                    || Results(presults),
                    |results| {
                        unpack_results(
                            options.query_want_stats,
                            compiled.compiled_query_result_pid,
                            results,
                        )
                    },
                ))
            })
        })
    })
}

/// `restartCompiled :: CanDefine a => Inventory -> Maybe DefineOwnership
/// -> a -> Maybe Pid -> QueryRuntimeOptions -> ByteString -> IO
/// QueryResults` (Query.hsc:136-144) — the ByteString is the serialized
/// `thrift::internal::QueryCont` (source doc, Query.hsc:143).
///
/// Marshalling as [`execute_compiled`], plus `unsafeWithBytes
/// serializedCont $ \cont_ptr cont_size` (Query.hsc:149) ->
/// `crate::ffi::unsafe_with_bytes` (the bytes are borrowed for the call
/// only; the C++ side reads them as the serialized continuation,
/// ffi.cpp:307-308's `cont`/`cont_size` forwarding). The unpack's `Maybe
/// Pid` is THIS function's `pid` parameter (Query.hsc:174) — there is no
/// CompiledQuery here.
pub fn restart_compiled(
    inventory: &Inventory,
    ownership: Option<&DefineOwnership>,
    facts: &impl CanDefine,
    pid: Option<Pid>,
    options: &QueryRuntimeOptions,
    serialized_cont: &[u8],
) -> Result<QueryResults, FfiError> {
    let maxr = options.query_max_results.unwrap_or(0) as u64;
    let maxb = options.query_max_bytes.unwrap_or(0) as u64;
    let maxt = options.query_max_time_ms.unwrap_or(0) as u64;
    let maxs = options.query_max_set_size.unwrap_or(0) as u64;
    let want_stats = if options.query_want_stats { 1 } else { 0 };
    facts.with_define(|facts_ptr| {
        let inventory_ptr = inventory.as_ptr();
        with_ownership_ptr(ownership, |ownership_ptr| {
            // `unsafeWithBytes serializedCont $ \cont_ptr cont_size ->
            // ...` (Query.hsc:149) — the (ptr, len) pair is exactly
            // unsafe_with_bytes'; the length is `usize` here and crosses
            // the header's `int64_t` (ffi.h:193) with the same bit
            // pattern the source's `fromIntegral :: CSize` produced
            // (non-negative by construction).
            unsafe_with_bytes(serialized_cont, |cont_ptr, cont_size| {
                with_depth(&options.query_depth, |depth, expand_pids, num_expand_pids| {
                    let mut presults: *mut sys::QueryResults = ptr::null_mut();
                    // SAFETY: as execute_compiled's call — every pointer
                    // valid for the duration (cont_ptr is the borrowed
                    // serialized_cont, alive for this closure), presults
                    // an initialised stack out-param; the extern's
                    // parameter order mirrors ffi.h:188-202 exactly.
                    // invoke owns the error string; on Err *presults is
                    // untouched (ffi.cpp:292-309).
                    unsafe {
                        invoke(|| {
                            sys::glean_query_restart_compiled(
                                inventory_ptr,
                                facts_ptr,
                                ownership_ptr,
                                cont_ptr as *mut c_void,
                                cont_size as i64,
                                maxr,
                                maxb,
                                maxt,
                                maxs,
                                depth,
                                expand_pids,
                                num_expand_pids,
                                want_stats,
                                &mut presults,
                            )
                        })
                    }?;
                    Ok(using(
                        || Results(presults),
                        |results| unpack_results(options.query_want_stats, pid, results),
                    ))
                })
            })
        })
    })
}

/// `interruptRunningQueries :: IO ()` (Query.hsc:251-252) — the C++ body
/// (ffi.cpp:221-223) is a plain call with no error slot.
pub fn interrupt_running_queries() {
    // SAFETY: the symbol takes no arguments and returns nothing
    // (ffi.cpp:221-223); infallible on this boundary.
    unsafe { sys::glean_interrupt_running_queries() }
}

/// `withDepth :: Depth -> ((Word64, Ptr Word64, Word64) -> IO a) -> IO a`
/// (Query.hsc:176-183).
///
/// `ResultsOnly`/`ExpandRecursive` pass `nullPtr` and 0; `ExpandPartial
/// pids` builds `[ fromIntegral (fromPid p) | p <- Set.toList pids ]`
/// (Query.hsc:181) and `withArrayLen`s it. Data.Set's `toList` is
/// ASCENDING, and BTreeSet's iteration is ascending, so the array handed
/// to C++ carries the same ascending pid order as the source (`as u64`
/// on the Int64 `fromPid` — the same bit pattern fromIntegral produced).
/// The Vec is a closure-scope local kept alive across `f` — withArrayLen's
/// bracket property. An EMPTY ExpandPartial set yields a non-null
/// dangling-but-aligned pointer with len 0 (Vec's empty as_mut_ptr),
/// which the C++ treats as an empty set exactly as it treated the
/// Haskell's 0-byte withArrayLen buffer (ffi.cpp:242-247 checks the
/// POINTER, then iterates 0 elements) — same observable behaviour.
fn with_depth<R>(depth: &Depth, f: impl FnOnce(u64, *mut u64, u64) -> R) -> R {
    match depth {
        // Query.hsc:178: f (depth_ResultsOnly, nullPtr, 0)
        Depth::ResultsOnly => f(DEPTH_RESULTS_ONLY, ptr::null_mut(), 0),
        // Query.hsc:179: f (depth_ExpandRecursive, nullPtr, 0)
        Depth::ExpandRecursive => f(DEPTH_EXPAND_RECURSIVE, ptr::null_mut(), 0),
        // Query.hsc:180-183.
        Depth::ExpandPartial(pids) => {
            let mut list: Vec<u64> = pids.iter().map(|p| p.0 as u64).collect();
            // `withArrayLen list $ \len p -> ...` — len is the element
            // count, widened to the header's uint64_t (the source's
            // `fromIntegral len`).
            let len = list.len() as u64;
            f(DEPTH_EXPAND_PARTIAL, list.as_mut_ptr(), len)
        }
    }
}

/// `maybe ($ nullPtr) with ownership $ \ownership_ptr -> ...`
/// (Query.hsc:105/:150) — None passes the null pointer; Some borrows the
/// DefineOwnership handle for the duration of `f` (crate::ffi::with's
/// scope), exactly the Haskell bracket.
fn with_ownership_ptr<R>(ownership: Option<&DefineOwnership>, f: impl FnOnce(*mut c_void) -> R) -> R {
    match ownership {
        None => f(ptr::null_mut()),
        // SAFETY: `with` keeps the ForeignPtr's finalizer duty alive for
        // the whole call (with_raw's Arc keep-alive); the pointer is used
        // only inside f and never freed or retained.
        Some(o) => unsafe { with(o, |p: *mut DefineOwnership| f(p as *mut c_void)) },
    }
}

/// `unpackResults :: Bool -> Maybe Pid -> Results -> IO QueryResults`
/// (Query.hsc:185-249) — the row's field-by-field unpack. Infallible
/// over a successfully returned struct (no channel in the source either:
/// the peeks cannot fail on a live QueryResults).
///
/// The pid-skip optimization (Query.hsc:188-192, "-- don't marshal the
/// pids if we know what they are"): the C++ `fact_pids` array is read
/// ONLY when `maybe_pid` is Nothing — the source skipped the peek by
/// substituting `HsArray Vector.empty`; the port skips the read the same
/// way (the array's cursor is never dereferenced on the known-pid path).
fn unpack_results(
    want_stats: bool,
    maybe_pid: Option<Pid>,
    results: &Results,
) -> QueryResults {
    // SAFETY: `results.0` is the live `QueryResults*` owned by the
    // `using` bracket (destroy_static has not run — the guard drops only
    // after this function returns), written by the C++ side on success;
    // the shared reference never mutates it.
    let r = unsafe { &*results.0 };
    // The peeks (Query.hsc:187-202) — field order kept. Each read is an
    // owned copy; see the helpers' contracts.
    // SAFETY: r's fact_ids cursor was written by the C++ side (HsArray
    // update() discipline); the bracket keeps the struct alive for the
    // whole unpack.
    let fact_ids = unsafe { hs_array_u64(&r.fact_ids) };
    // The maybePid optimization (:188-192) — read fact_pids ONLY when the
    // pid is unknown.
    let fact_pids = if maybe_pid.is_none() {
        // SAFETY: as fact_ids, for r.fact_pids.
        Some(unsafe { hs_array_u64(&r.fact_pids) })
    } else {
        // The source's `return (HsArray Vector.empty)`: no peek at all.
        None
    };
    // SAFETY: as fact_ids, for r.fact_keys (HsArray<HsString> elements
    // copied out as owned Vec<u8>).
    let fact_keys = unsafe { hs_array_bytes(&r.fact_keys) };
    // SAFETY: as fact_ids, for r.fact_values.
    let fact_values = unsafe { hs_array_bytes(&r.fact_values) };
    // SAFETY: as fact_ids, for r.nested_fact_ids (:195-196).
    let nested_fact_ids = unsafe { hs_array_u64(&r.nested_fact_ids) };
    // SAFETY: as fact_ids, for r.nested_fact_pids (:197-198).
    let nested_fact_pids = unsafe { hs_array_u64(&r.nested_fact_pids) };
    // SAFETY: as fact_ids, for r.nested_fact_keys (:199-200).
    let nested_fact_keys = unsafe { hs_array_bytes(&r.nested_fact_keys) };
    // SAFETY: as fact_ids, for r.nested_fact_values (:201-202).
    let nested_fact_values = unsafe { hs_array_bytes(&r.nested_fact_values) };

    // `mkResultFact pid id key value = (Fid (fromIntegral (id::Word64)),
    // Fact pid (hsByteString key) (hsByteString value))` (Query.hsc:
    // 205-207) — the known-pid form: `id :: Word64` bit-cast to the
    // thrift `Id` (i64), field 1 `type` = the fixed pid (an Int64 `Pid`
    // in the source). Consumes the already-copied bytes by value, as the
    // source's Fact construction did.
    let mk_result_fact =
        |pid: i64, id: &u64, key: Vec<u8>, value: Vec<u8>| -> (Fid, Fact) {
            (
                Fid(*id as i64),
                Fact {
                    r#type: pid,
                    key,
                    value,
                },
            )
        };
    // `mkFact id pid key value = ...` (Query.hsc:209-211) — the 4-array
    // form: pid read out of fact_pids. The Haskell's elements were
    // Int64-typed (the generated `Fact`'s `fact_type :: Id = Int64`), so
    // the raw u64 is bit-cast `as i64`.
    let mk_fact = |id: &u64, pid: &u64, key: Vec<u8>, value: Vec<u8>| -> (Fid, Fact) {
        (
            Fid(*id as i64),
            Fact {
                r#type: *pid as i64,
                key,
                value,
            },
        )
    };

    // `resultFacts = case maybePid of ...` (Query.hsc:213-222):
    // zipWith4 / zipWith3 -> truncating iterator zips (Vector.zipWithN
    // truncates to the shortest input; the arrays are C++-side
    // equal-length by construction, and the truncation semantics are
    // kept anyway).
    let result_facts: Vec<(Fid, Fact)> = match (maybe_pid, fact_pids) {
        // Nothing -> Vector.zipWith4 mkFact ids pids keys values.
        (None, Some(pids)) => fact_ids
            .iter()
            .zip(pids.iter())
            .zip(fact_keys)
            .zip(fact_values)
            .map(|(((id, pid), key), value)| mk_fact(id, pid, key, value))
            .collect(),
        // Just (Pid pid) -> Vector.zipWith3 (mkResultFact pid) ids keys
        // values (:219-222).
        (Some(Pid(pid)), None) => fact_ids
            .iter()
            .zip(fact_keys)
            .zip(fact_values)
            .map(|((id, key), value)| mk_result_fact(pid, id, key, value))
            .collect(),
        // Unreachable by construction (fact_pids was read iff maybe_pid
        // is None); the source's lazy Vectors made the mismatched
        // combination unrepresentable too. Panic rather than guess.
        _ => unreachable!("fact_pids was read iff maybe_pid is None"),
    };

    // `nestedFacts = Vector.zipWith4 mkFact ...` (Query.hsc:224-228).
    let nested_facts: Vec<(Fid, Fact)> = nested_fact_ids
        .iter()
        .zip(nested_fact_pids.iter())
        .zip(nested_fact_keys)
        .zip(nested_fact_values)
        .map(|(((id, pid), key), value)| mk_fact(id, pid, key, value))
        .collect();

    // `stats <- if wantStats then Just . hsMap <$> (# peek ..., stats) p
    // else return Nothing` (Query.hsc:230-233) — the C++ stats member is
    // read ONLY when wantStats.
    let stats = if want_stats {
        // SAFETY: r.stats' keys/values/n cursors were written by the C++
        // side (HsMap update() discipline); owned copy.
        Some(unsafe { hs_map_u64(&r.stats) })
    } else {
        None
    };
    // `elapsed_ns <- (# peek ..., elapsed_ns) p` (Query.hsc:234) —
    // uint64_t, direct.
    let elapsed_ns = r.elapsed_ns;
    // `cont <- (# peek ..., continuation) p` (Query.hsc:235) — an
    // HsString, read as owned bytes.
    // SAFETY: r.continuation's str/len cursors were written by the C++
    // side; owned copy.
    let cont_bytes = unsafe { hs_string_bytes(&r.continuation) };
    // `result_bytes <- (# peek ..., result_bytes) p` (Query.hsc:236) —
    // the C++ uint64_t read through the Int64-typed field: bit-cast.
    let result_bytes = r.result_bytes as i64;

    QueryResults {
        query_results_facts: result_facts,
        query_results_nested_facts: nested_facts,
        query_results_stats: stats,
        query_results_elapsed_ns: elapsed_ns,
        // `queryResultsCont = let contBytes = hsByteString cont in
        // if ByteString.length contBytes == 0 then Nothing else Just
        // contBytes` (Query.hsc:243-247) — the row's "cont=="" -> None".
        query_results_cont: if cont_bytes.is_empty() {
            None
        } else {
            Some(cont_bytes)
        },
        query_results_result_bytes: result_bytes,
    }
}

/// The `hsArray` read for the u64 arrays — `peekDataSize` (Types.hsc:
/// 467-471: `a` + `n`) + `peekElemOffWith` per element.
///
/// The `n == 0` early-out is the soundness guard the copy_u64s precedent
/// established (bytecode.rs:574-595): an empty std::vector's `data()`
/// may be null and `slice::from_raw_parts(null, _)` is UB, while the
/// Haskell's `generateM 0` never touched the pointer — same empty slice,
/// no dereference.
///
/// SAFETY: `arr.n > 0` implies `arr.a` is non-null and valid for reads
/// of `arr.n` `u64` elements (the C++ side maintains `a = v_.data()` /
/// `n = v_.size()` under the HsArray update() discipline, and the struct
/// is kept alive by the caller's bracket for the whole read).
unsafe fn hs_array_u64(arr: &sys::HsArray<u64>) -> &[u64] {
    if arr.n == 0 {
        return &[];
    }
    // SAFETY: caller contract above; elements only read.
    unsafe { std::slice::from_raw_parts(arr.a, arr.n) }
}

/// The `hsArray` read over `HsArray<HsString>` — each element peeked via
/// `peekStrLen` (Types.hsc:225-229) and `packCStringLen`ed (:380, the
/// `hsByteString` of the source) into an OWNED Vec<u8> (the row's
/// "HsArray/HsString -> owned Vec/Vec<u8> copies").
///
/// SAFETY: `arr.n > 0` implies `arr.a` is non-null and valid for reads
/// of `arr.n` `HsString` elements (48 bytes each, the C++ layout the
/// mirror reproduces), each element's `str`/`len` a maintained
/// (str, len) pair over the element's own std::string storage.
unsafe fn hs_array_bytes(arr: &sys::HsArray<sys::HsString>) -> Vec<Vec<u8>> {
    let mut out = Vec::with_capacity(arr.n);
    for i in 0..arr.n {
        // SAFETY: i < arr.n, in range per the caller contract above.
        let elem = unsafe { &*arr.a.add(i) };
        // SAFETY: elem's (str, len) is a maintained pair over the C++-owned
        // string storage; hs_string_bytes performs the owned copy.
        out.push(unsafe { hs_string_bytes(elem) });
    }
    out
}

/// The single-`HsString` read — `peekStrLen` + `packCStringLen`
/// (Types.hsc:225-229/:380): copy `len` bytes from `str` into an owned
/// Vec<u8>.
///
/// The `len == 0` early-out is the same soundness guard as
/// hs_array_u64's (an empty std::string's data() may be null in theory;
/// packCStringLen(nullptr, 0) was simply the empty ByteString).
///
/// SAFETY: `s.len > 0` implies `s.str` is non-null and valid for reads
/// of `s.len` bytes (the C++ side maintains str = s_.data() / len =
/// s_.size() under the HsString update() discipline).
unsafe fn hs_string_bytes(s: &sys::HsString) -> Vec<u8> {
    if s.len == 0 {
        return Vec::new();
    }
    // SAFETY: caller contract above; the bytes are only read, into the
    // owned copy this returns.
    unsafe { std::slice::from_raw_parts(s.str as *const u8, s.len) }.to_vec()
}

/// The `hsMap` read — `peekMapWith Map.empty Map.insert` (Types.hsc:
/// 597-616): iterate `(keys[i], values[i])` for `i in 0..n`, inserting
/// into the map. The Haskell's `Data.Map.insert` makes a LATER duplicate
/// key override an earlier one; BTreeMap::insert is the same. The
/// Int64-typed peeks over the C++ `HsMap<uint64_t, uint64_t>` are the
/// two `as i64` bit-casts.
///
/// SAFETY: `m.n > 0` implies `m.keys`/`m.values` are non-null and valid
/// for reads of `m.n` u64 elements each (the C++ HsMap update()
/// discipline), kept alive by the caller's bracket.
unsafe fn hs_map_u64(m: &sys::HsMap<u64, u64>) -> BTreeMap<i64, i64> {
    if m.n == 0 {
        return BTreeMap::new();
    }
    // SAFETY: caller contract above; the keys array only read.
    let keys = unsafe { std::slice::from_raw_parts(m.keys, m.n) };
    // SAFETY: caller contract above; the values array only read.
    let values = unsafe { std::slice::from_raw_parts(m.values, m.n) };
    keys.iter()
        .zip(values.iter())
        .map(|(k, v)| (*k as i64, *v as i64))
        .collect()
}

// Marker breakdown (closed set — 0 TODO(port), 0 BUG(port), 0
// PERF(port), 0 INVENTORY-BUG-CANDIDATE):
//   * The inventory rows' TODO(port) (target-file ownership) is RESOLVED
//     by manifest.tsv line 94 and recorded in the INVENTORY block — it is
//     not an open marker of this file.
//   * no source-defect reproductions — no source defect reproduced. The two
//     Haskell-vs-header type notes (restart's CSize vs int64_t; the
//     Int64-typed peeks over uint64_t C++ fields) are ABI-identical
//     bit-pattern conversions, documented at the sites, not defects.
//   * no performance deferrals — every copy in unpack_results is source-mandated
//     (the row's "owned Vec/Vec<u8> copies"; the Haskell's packCStringLen
//     copied too), and the pid-skip optimization is the source's own.
//   * No INVENTORY-BUG-CANDIDATE: row 72 (== ffi row 30) is followed
//     clause by clause; sites row 662 rules the whole file.
// SAFETY marker count: 24 unsafe operations, each carrying its own
// inline `// SAFETY:` comment — destroy_static 1 (the free call);
// execute_compiled 3 (sub_ptr extraction, traversal_ptr extraction, the
// invoke call); restart_compiled 1 (the invoke call);
// interrupt_running_queries 1; with_ownership_ptr 1 (the
// crate::ffi::with borrow); unpack_results 11 (the &*results.0
// projection, hs_array_u64 x4 — fact_ids, fact_pids, nested_fact_ids,
// nested_fact_pids — hs_array_bytes x4 — fact_keys, fact_values,
// nested_fact_keys, nested_fact_values — hs_map_u64, hs_string_bytes);
// the four peek helpers 6 (hs_array_u64's from_raw_parts;
// hs_array_bytes' element add() and its inner hs_string_bytes call;
// hs_string_bytes' from_raw_parts; hs_map_u64's two from_raw_parts).
// The four `unsafe fn` peek helpers additionally carry doc-level
// SAFETY contracts (the `/// SAFETY:` caller obligations), per
// crate::ffi's house style. The `pub mod sys` `unsafe extern` block is
// a declaration, not an operation (define.rs:248-252 precedent), so it
// is not counted.
// PORT STATUS: confidence=high todos=0
