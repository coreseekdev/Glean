// Ported from: glean/hs/Glean/RTS.hs (310 lines)
// Manifest unit: rust/glean-core/src/rts.rs (migration/manifest.tsv line 13,
// crate glean-core). RULE §4: one source file -> one target file; the
// manifest path is authoritative.
//
// ABI truth (read, not assumed):
//   * glean/rts/ffi.h — the id typedefs at :105-106 (glean_fact_id_t /
//     glean_predicate_id_t are plain int64_t: by-value ids carry no struct);
//     the push group at :329-338; the pop group at :340-369;
//     glean_push_fact at :371-372; glean_string_demangle_trusted at
//     :374-377.
//   * glean/rts/ffi.cpp — the pop_value helper at :58-66 (the `*start`
//     write-back happens ON SUCCESS ONLY, after the lambda returns) and
//     this unit's extern bodies at :680-818 (:680-715 push, :717-800 pop,
//     :802-811 push_fact, :813-818 demangle; the :820-821
//     glean_free_query_results that follows belongs to another unit).
//   * glean/rts/binary.h — Input::wantError want :119-128 ("truncated
//     input: expected {} bytes, got {}"), fixed :134-143, packed :145-156
//     (folly varint; "invalid packed value" on a malformed one), bytes
//     :200-206; Output::packed :345-352, bytes :372-377 (size 0 is a
//     no-op), expect :379-381, mangleString :383-387.
//   * glean/rts/string.cpp — untrustedChunks :36-81 (the structural errors
//     "invalid NUL in mangled string" :72, "truncated terminator in
//     mangled string" :75, "invalid UTF-8 string" :78), trustedChunks
//     :95-111 (glog CHECK at :102, noexcept — abort-class),
//     skipTrustedString :115-121 (returns (mangled size, demangled size),
//     the latter computed as `size - nuls - 1`), demangleTrustedString
//     :123-130 (writes the demangled bytes into the caller's buffer; the
//     write length is a function of the INPUT, never of any buffer-length
//     parameter), mangleString :132-146 (every NUL byte of the input is
//     written as `00 01`; a `00 00` terminator is appended).
//   * the error kit (hsthrift, on disk at
//     ~/glean-build/hsthrift/common/util/cpp/wrap.h): wrap (:21-32;
//     `outOfMemory` :18, `unknownError` :19) turns EVERY failure — a
//     caught std::exception INCLUDING bad_alloc (via a strdup'd e.what(),
//     falling back to the static `outOfMemory`; the static `unknownError`
//     on catch(...)) — into a returned message, null on success; rts::error
//     (glean/rts/error.cpp raiseError) embeds a stack trace in the text;
//     ffi::clone_bytes (memory.h:86-87, over clone_array :77-83) is the
//     malloc'd ownership-transfer helper the pop bodies use.
//     crate::ffi::invoke reads and releases exactly this protocol.
//
// INVENTORY (RULE §5) — rows governing this file, followed exactly:
//   * inventory.tsv row 58 (line 58; duplicated as inventory.ffi.tsv row
//     21): "fromValue | pure fn implemented as unsafePerformIO FFI decode
//     (decodeValue over Decoder) | HIDDEN_EFFECT | RULEBOOK §2
//     unsafePerformIO row: pure Rust fn Value -> Vec<u8>; decoder becomes
//     a cursor struct over the encoded bytes with explicit EOF/extra-bytes
//     error (DecodingException parity, same message shape "extra N bytes
//     at end of value")". Followed: from_value is an ordinary pure fn (the
//     unsafePerformIO does not survive), Decoder is a cursor struct, and
//     the trailing check raises DecodingException with the exact
//     "extra {} bytes at end of value" text.
//   * inventory.tsv row 59 (line 59): fromValue's unsafePerformIO wrap —
//     "explicit pure fn Value -> Vec<u8> calling the builder bridge; no
//     hidden effect survives". Followed: the builder bridge is
//     with_value + rts/builder.rs::finish_builder.
//   * inventory.tsv row 60 (line 60; duplicated as inventory.ffi.tsv row
//     22): "stateful cursor Decoder over input bytes (Ptr advanced
//     in-place via FFI); dBytes/dString take ownership of malloc'd
//     buffers; bytes_ref/trusted_string_ref BORROW decoder lifetime;
//     shared FFIResultBuf slot trick | OWNED_MUTABLE | encode side: push
//     methods on the cxx Builder (see inventory.tsv Builder row); decode
//     side: Rust Decoder<'a> {cursor, end} borrowing the input &[u8];
//     dByteStringRef/dTrustedStringRef -> borrowed &[u8] (lifetime =
//     decoder), dBytes/dString -> Vec<u8> (ownership transfer); demangle
//     writes into caller buffer -> &mut [u8] -> usize; ffiBuf/ffiBuf2
//     shared-slot becomes plain locals (fast path only if the judge shows
//     decode cost)". Followed, with two representation divergences flagged
//     in the INVENTORY-BUG-CANDIDATE block below. The row's "cxx Builder"
//     wording is superseded by RULE §1/A2 exactly as rts/builder.rs's
//     family-wide flag records (rows 60/61/38/79 mandate one canonical
//     ruling for this same handle) — cited, not re-flagged here.
//
// INVENTORY-BUG-CANDIDATE row 60: the row's target_translation sketches a
// `Decoder<'a> {cursor, end}` and "demangle writes into caller buffer ->
// &mut [u8] -> usize"; the port diverges twice, both deliberate:
//   1. demangle stays an `unsafe fn` taking a raw `*mut u8` buffer. RULE
//      §2's A7 row ("a safe signature must not launder an unbounded C++
//      write") names demangle as its worked case (stress D11), and the
//      reason is load-bearing: demangleTrustedString writes the DEMANGLED
//      size (string.cpp:123-130), a function of the input's NUL layout —
//      the function takes no buffer-length parameter and checks nothing
//      about the buffer, so a `&mut [u8]` sketch would still check
//      nothing and would force slice-minting at the raw call site
//      (Query/JSON.hs:377-383 allocates exactly the size a prior
//      dTrustedStringRef returned, inside a Buffer.alloc closure). The
//      unsafe fn + SAFETY contract is the A7-mandated surface.
//   2. Decoder carries a third field, `input: &'a [u8]`, alongside the
//      two cursor words. This is the same borrow the row's "borrowing the
//      input &[u8]" clause mandates, stored rather than only used to
//      initialise the cursors: it is what lets d_trusted_string_ref return
//      a `ByteStringRef<'a>` borrow (row: "borrowed &[u8] (lifetime =
//      decoder)") by SLICING the live input in bounds. d_byte_string_ref
//      satisfies the same row clause the other way — it MINTs its
//      `ByteStringRef::from_raw` over the C pointer the pop handed back,
//      which is sound because the C side validated the range before
//      returning it (ffi.cpp:745-753). The {cursor, end} sketch still
//      drives every C call; no observable difference.
//   (The row's remaining clauses — borrowed refs, Vec transfers, plain
//   locals instead of the shared FFIResultBuf slot — are followed as
//   written; the locals replacement is the row's own PERF-flagged
//   instruction, marked at dffi below.)
//
// SIBLING-UNIT CONTRACT (RULE §4; DONE siblings' real APIs read, never
// re-declared):
//   * crate::ffi (read): `invoke` (the pinned
//     `pub unsafe fn invoke(call: impl FnOnce() -> *const c_char)
//     -> Result<(), FfiError>` — the extern call passed AS the closure),
//     `FfiError`, and `unsafe_malloced_byte_string` (the Rust twin of
//     unsafePackMallocCStringLen: copy + libc::free, used by
//     adopt_malloced_bytes). This file never re-implements the CString
//     protocol.
//   * crate::rts::builder (read): `Builder::as_ptr` is the marshalling
//     seam for every push symbol below; `with_builder` is the scope
//     (Drop = free) with_value wraps; `finish_builder` is the owned-bytes
//     transfer from_value ends in. The push family's abort ruling
//     (INVENTORY-BUG-CANDIDATE row 61 there, per row 61's "abort on OOM")
//     is mirrored by this file's private allocation_guard_abort.
//   * crate::rts::types (read): Fid/Pid/lowest_fid/lowest_pid are
//     re-exported below for export-list parity (RTS.hs:11-12); Rep is the
//     decode-side type; crate::rts::term (read): Term/Value.
//   * `sys` is `pub` and header-exact because the raw externs are part of
//     the source module's own export list (RTS.hs:11-44 exports 17 of the
//     module's 20 foreign imports; the three that are NOT exported —
//     glean_pop_value_bytes_ref (:293), glean_pop_value_trusted_string_ref
//     (:299) and glean_string_demangle_trusted (:309) — are internal to
//     the d* wrappers here). `sys` is therefore a DELIBERATE superset: it
//     holds those three internal symbols alongside every export-list
//     symbol, because the export-list symbols are what consumers call
//     directly with their own cursors and result slots:
//       - rust/glean-typed/src/typed/binary.rs (Typed/Binary.hs, which
//         CONSUMES the `DecoderEnv` type at Typed/Binary.hs:114-131 — the
//         type is DEFINED at Typed/Build.hs:41-47; pushes at :103-104/:113/
//         :120-121/:130/:155/:163/:175/:234, raw pops at :115/:123-124/
//         :132/:158/:166/:178/:201/:212/:226/:236);
//       - rust/glean-typed/src/typed/build.rs (Typed/Build.hs, where
//         `DecoderEnv` is DEFINED at :41-47; raw pops at :115/:126/:139);
//       - the db crate's consumers (Write/JSON.hs:180-181 calls
//         glean_push_fact through Util.FFI.invoke; Query/JSON.hs:151-267
//         drives the d* methods; Query/Transform.hs:238/:249/:312-313
//         uses mangle_string/from_value/to_value).
//     Those units must obtain the builder pointer via as_ptr, must not
//     dereference or free it, and must keep this file's per-symbol guard
//     dispositions.

//! `Glean.RTS` — the RTS term codec: encode a [`Value`] into the C++ byte
//! writer ([`with_value`]/[`from_value`]), decode values back through the
//! borrow-scoped [`Decoder`] ([`to_value`]), plus the borrowed byte/string
//! reference types ([`ByteStringRef`], [`MangledStringRef`],
//! [`mangle_string`], [`demangle`]) shared with the JSON encoders and the
//! raw `sys` symbols the typed decoder units call directly.
//!
//! Port notes: RULE §3 — `unsafe` appears only where the Haskell was
//! already FFI (the raw `sys` externs, the raw-pointer refs, the decoder's
//! cursor calls); every block carries a SAFETY comment. RULE §2
//! error-recovery rule, one classification per direction:
//!   * PUSH symbols (encode side): the bodies at ffi.cpp:680-715 contain
//!     no precondition checks; their only failure mode is allocation
//!     (Output growth), which wrap.h returns as a message. Inventory row
//!     61's family ruling ("allocation-guard: abort on OOM") applies via
//!     row 60's "push methods on the cxx Builder (see inventory.tsv
//!     Builder row)" — so every push Err aborts
//!     ([`allocation_guard_abort`]), the same disposition rts/builder.rs
//!     records for its own calls on the same handle.
//!   * POP symbols (decode side): structural errors (truncation, malformed
//!     varint, bad mangled strings) are returned as messages — the
//!     precondition-guard class — and surface as `Err(FfiError)`, never an
//!     abort. The two ownership-transfer pops (bytes, string) can ALSO
//!     fail by allocation (the clone/demangle mallocs); wrap.h merges both
//!     classes into one channel, and the source threw FFIError for both,
//!     so the port returns both as values (classified per symbol below).
//!   * glean_pop_value_trusted_string_ref and glean_string_demangle_trusted
//!     have NO error slot (ffi.h:360/:374; noexcept CHECK-bodied) —
//!     malformed input aborts inside C++, exactly as in the source, so
//!     their wrappers have no Result to return.

use std::mem::MaybeUninit;
use std::os::raw::{c_char, c_void};
use std::ptr;

use crate::ffi::{invoke, unsafe_malloced_byte_string, FfiError};
use crate::rts::builder::{self, Builder};
use crate::rts::term::{Term, Value};
use crate::rts::types::Rep;

// Haskell export-list parity (RTS.hs:11-12): `Fid(..), lowestFid,
// Pid(..), lowestPid` are re-exported here; the types themselves stay
// owned by rts/types.rs (RULE §4: never re-declare a sibling's type).
pub use crate::rts::types::{lowest_fid, lowest_pid, Fid, Pid};

/// Raw C-ABI declarations for the RTS push/pop/demangle symbols
/// (glean/rts/ffi.h:329-377). Part of this module's public surface because
/// the Haskell module re-exports most of its foreign imports (RTS.hs:11-44
/// — 17 of the 20; the three internal ones are named in the SIBLING-UNIT
/// CONTRACT at the top of this file) and the typed decoder units call the
/// symbols directly — see that contract.
///
/// RULE §1 (FFI row, A2): raw `extern "C"` for flat C-ABI symbols.
/// Signatures mirror ffi.h EXACTLY (`glean_fact_id_t`/`glean_predicate_id_t`
/// are int64_t at :105-106; `unsigned char` -> `u8`; `size_t` -> `usize`
/// per the A5 CSize row — unsigned both sides). Guard kinds per RULE §2
/// are recorded at each symbol; `wrap.h` returns every C++ failure,
/// bad_alloc included, as a message through the error slot, and the
/// no-slot symbols (trusted_string_ref, demangle) are CHECK/noexcept
/// abort-class.
pub mod sys {
    use std::os::raw::{c_char, c_void};

    use crate::rts::builder::sys::Output;

    unsafe extern "C" {
        // ffi.h:329 — push. Body ffi.cpp:680-682 (`builder->fixed<unsigned
        // char>`). ALLOCATION-GUARD only: no precondition check exists;
        // Output::fixed -> bytes -> grab -> alloc can bad_alloc.
        pub fn glean_push_value_byte(builder: *mut Output, val: u8) -> *const c_char;
        // ffi.h:330-331 — push. Body ffi.cpp:684-687 (`builder->bytes`).
        // ALLOCATION-GUARD; size == 0 is a no-op (binary.h:372-377).
        pub fn glean_push_value_bytes(
            builder: *mut Output,
            data: *const c_void,
            size: usize,
        ) -> *const c_char;
        // ffi.h:332 — push. Body ffi.cpp:689-691 (`builder->packed`).
        // ALLOCATION-GUARD (packed allocates kMaxVarintLength64).
        pub fn glean_push_value_nat(builder: *mut Output, val: u64) -> *const c_char;
        // ffi.h:333 — push. Body ffi.cpp:693-695 (`builder->packed`).
        // ALLOCATION-GUARD.
        pub fn glean_push_value_array(builder: *mut Output, size: usize) -> *const c_char;
        // ffi.h:334 — push. Body ffi.cpp:697-699 (`builder->packed`; the
        // dedup itself lives in the rts-set units, not here).
        // ALLOCATION-GUARD.
        pub fn glean_push_value_set(builder: *mut Output, size: usize) -> *const c_char;
        // ffi.h:335 — push. Body ffi.cpp:701-703 (`builder->packed`).
        // ALLOCATION-GUARD.
        pub fn glean_push_value_selector(builder: *mut Output, selector: usize)
            -> *const c_char;
        // ffi.h:336-337 — push. Body ffi.cpp:705-711 (Output::mangleString
        // -> rts::mangleString, string.cpp:132-146). ALLOCATION-GUARD (the
        // mangled copy grows the buffer).
        pub fn glean_push_value_string(
            builder: *mut Output,
            data: *const c_void,
            size: usize,
        ) -> *const c_char;
        // ffi.h:338 — push. Body ffi.cpp:713-715
        // (`builder->packed(Id::fromThrift(fact))`). ALLOCATION-GUARD.
        pub fn glean_push_value_fact(builder: *mut Output, fact: i64) -> *const c_char;
        // ffi.h:340-341 — pop. Body ffi.cpp:717-722 (`input.fixed<uint8_t>`)
        // through pop_value :58-66 (cursor written back on success only).
        // PRECONDITION-GUARD: want(1) failure = "truncated input: expected
        // {} bytes, got {}" (binary.h:119-128); no allocation.
        pub fn glean_pop_value_byte(
            start: *mut *const c_void,
            end: *const c_void,
            byte: *mut u8,
        ) -> *const c_char;
        // ffi.h:342-343 — pop. Body ffi.cpp:724-729 (`input.packed<u64>`).
        // PRECONDITION-GUARD ("invalid packed value", binary.h:145-156).
        pub fn glean_pop_value_nat(
            start: *mut *const c_void,
            end: *const c_void,
            nat: *mut u64,
        ) -> *const c_char;
        // ffi.h:344-345 — pop. Body ffi.cpp:731-736 (`input.packed<size_t>`).
        // PRECONDITION-GUARD; the returned size is validated by the
        // CONSUMER's element decodes, not here.
        pub fn glean_pop_value_array(
            start: *mut *const c_void,
            end: *const c_void,
            size: *mut usize,
        ) -> *const c_char;
        // ffi.h:346-347 — pop. Body ffi.cpp:738-743; same shape as array.
        // PRECONDITION-GUARD.
        pub fn glean_pop_value_set(
            start: *mut *const c_void,
            end: *const c_void,
            size: *mut usize,
        ) -> *const c_char;
        // ffi.h:348-352 — pop. Body ffi.cpp:745-753 (`input.bytes(size)`,
        // borrowed pointer into the input range). PRECONDITION-GUARD only
        // (truncation; no allocation).
        pub fn glean_pop_value_bytes_ref(
            start: *mut *const c_void,
            end: *const c_void,
            size: usize,
            bytes: *mut *const c_void,
        ) -> *const c_char;
        // ffi.h:353-357 — pop. Body ffi.cpp:755-763 (`input.bytes(size)` +
        // ffi::clone_bytes ownership transfer). BOTH classes: the range
        // check is PRECONDITION; the clone is ALLOCATION (memory.h:86-87,
        // over clone_array :77-83; wrap.h returns the bad_alloc). On
        // success `*bytes`/size are the malloc'd clone, size 0 included
        // (malloc(0) may be non-null).
        pub fn glean_pop_value_bytes(
            start: *mut *const c_void,
            end: *const c_void,
            size: usize,
            bytes: *mut *mut c_void,
        ) -> *const c_char;
        // ffi.h:358-359 — pop. Body ffi.cpp:765-772 (`input.packed<size_t>`).
        // PRECONDITION-GUARD.
        pub fn glean_pop_value_selector(
            start: *mut *const c_void,
            end: *const c_void,
            selector: *mut usize,
        ) -> *const c_char;
        // ffi.h:360 — NOT wrap'd; returns size_t directly (NO error slot).
        // Body ffi.cpp:786-791: skipTrustedString scans the input in place
        // (string.cpp:95-121, noexcept + glog CHECK on malformed input).
        // ABORT-CLASS on malformed input; on success advances `*start` by
        // the MANGLED size and returns the DEMANGLED size (`size - nuls - 1`).
        pub fn glean_pop_value_trusted_string_ref(
            start: *mut *const c_void,
            end: *const c_void,
        ) -> usize;
        // ffi.h:361-365 — pop. Body ffi.cpp:774-784
        // (`input.demangleUntrustedString` into a fresh Output, then
        // moveBytes().release_to — binary.cpp:44-55). BOTH classes: the
        // mangled-string validation is PRECONDITION (string.cpp:72/:75/:78
        // errors) and the fresh Output's growth is ALLOCATION. On success
        // `*bytes`/size are a malloc'd ownership transfer.
        pub fn glean_pop_value_string(
            start: *mut *const c_void,
            end: *const c_void,
            bytes: *mut *mut c_void,
            size: *mut usize,
        ) -> *const c_char;
        // ffi.h:366-369 — pop. Body ffi.cpp:793-800
        // (`input.packed<Id>().toThrift()`; toThrift/fromThrift are the
        // identity over the raw i64, id.h:123-129). PRECONDITION-GUARD.
        pub fn glean_pop_value_fact(
            start: *mut *const c_void,
            end: *const c_void,
            fact: *mut i64,
        ) -> *const c_char;
        // ffi.h:371-372 — push (header spells the 3rd param `data`; the
        // Haskell call site names it `clause`). Body ffi.cpp:802-811:
        // Fact::serialize's copy is ALLOCATION-GUARD; the body ALSO has a
        // structural CHECK_GE(clause->size(), key_size) (:805) — a CHECK
        // abort inside C++, never a returned error, so the slot is
        // allocation-class in practice (the source's `invoke` saw only the
        // slot). Consumers: Write/JSON.hs:180-181.
        pub fn glean_push_fact(
            builder: *mut Output,
            pid: i64,
            clause: *mut Output,
            key_size: usize,
        ) -> *const c_char;
        // ffi.h:374-377 — NO error slot, returns size_t. Body
        // ffi.cpp:813-818 (demangleTrustedString, string.cpp:123-130,
        // noexcept + CHECK): writes the demangled bytes into the caller's
        // buffer, whose length is the caller's obligation (see demangle
        // below; RULE §2 A7). ABORT-CLASS on malformed input.
        pub fn glean_string_demangle_trusted(
            start: *const u8,
            size: usize,
            buffer: *mut u8,
        ) -> usize;
    }
}

// ---------------------------------------------------------------------------
// Encoding: Value -> C++ Builder (RTS.hs:72-102)
// ---------------------------------------------------------------------------

/// RULE §2 (error-recovery rule): "allocation-failure != structural error.
/// Every FFI null return in the inventory records which kind:
/// allocation-guard (abort) or precondition-guard (record + partial
/// result)." The PUSH symbols are allocation-guard: their bodies
/// (ffi.cpp:680-715) contain no precondition check, and their only
/// failure mode is Output growth (folly::allocate_sys_buffer throws
/// bad_alloc, which wrap.h returns as a message). Inventory row 60 sends
/// the encode side to the Builder row's ruling, and row 61 rules "abort
/// on OOM" for that family — the disposition rts/builder.rs records for
/// its own calls and its private helper. This is that helper's mirror
/// (builder.rs's `allocation_guard_abort` is private there): same message
/// text, same abort-not-panic choice so no unwinding runs first.
fn allocation_guard_abort(e: FfiError) -> ! {
    eprintln!("glean: C++ builder allocation failure: {}", e);
    std::process::abort()
}

/// Run one push-symbol call through crate::ffi::invoke; an Err aborts per
/// the classification above. Not `unsafe`: each call site builds the
/// actual extern call inside its own `unsafe` block (with its SAFETY
/// note); this helper only routes the returned pointer through invoke's
/// documented contract.
fn invoke_push(call: impl FnOnce() -> *const c_char) {
    // SAFETY: `call` runs exactly one of the push symbols, constructed by
    // the caller with a live builder handle; a non-null return follows
    // crate::ffi::invoke's contract (a wrap.h-produced NUL-terminated
    // message, read and released by invoke).
    if let Err(e) = unsafe { invoke(call) } {
        allocation_guard_abort(e);
    }
}

/// RTS.hs:72-75: `withValue v f = withBuilder $ \b -> do
///   encodeValue b v
///   f b`.
///
/// The acquire/encode/hand-over/release scope is rts/builder.rs's
/// `with_builder` (its Drop is the source's bracket); this fn adds only
/// the encode step. The value is taken by reference — the Haskell shared
/// it and encoding only reads it (RULE §0: sharing-preserving).
pub fn with_value<R>(v: &Value, f: impl FnOnce(&mut Builder) -> R) -> R {
    builder::with_builder(|b| {
        encode_value(b, v);
        f(b)
    })
}

/// RTS.hs:77-90 — `encodeValue :: Builder -> Value -> IO ()` (not in the
/// source's export list; private here too).
///
/// Guard kind per arm: every push is ALLOCATION-GUARD -> abort (see
/// `invoke_push`); Tuple adds no bytes of its own (field order only), and
/// ByteArray delegates to [`encode_byte_array`].
fn encode_value(b: &mut Builder, v: &Value) {
    match v {
        // encodeValue b (Byte n) = FFI.call $ glean_push_value_byte b n
        Term::Byte(n) => {
            // SAFETY: `b` is a live Builder (the &mut borrow outlives the
            // call); the push only appends to its buffer; invoke's
            // contract covers the returned pointer.
            invoke_push(|| unsafe { sys::glean_push_value_byte(b.as_ptr(), *n) });
        }
        // encodeValue b (Nat n) = FFI.call $ glean_push_value_nat b n
        Term::Nat(n) => {
            // SAFETY: as the Byte arm — live builder, append-only call.
            invoke_push(|| unsafe { sys::glean_push_value_nat(b.as_ptr(), *n) });
        }
        // encodeValue b (Array xs) = do
        //   FFI.call $ glean_push_value_array b (fromIntegral $ length xs)
        //   mapM_ (encodeValue b) xs
        Term::Array(xs) => {
            // RULE §2 (A5): length :: Int -> CSize is usize -> size_t; no
            // signed value is ever cast (the source's fromIntegral was the
            // same no-op).
            // SAFETY: as the Byte arm; the length is the slice's own.
            invoke_push(|| unsafe { sys::glean_push_value_array(b.as_ptr(), xs.len()) });
            for x in xs {
                encode_value(b, x);
            }
        }
        // encodeValue b (ByteArray xs) = encodeByteArray b xs
        Term::ByteArray(xs) => encode_byte_array(b, xs),
        // encodeValue b (Tuple xs) = mapM_ (encodeValue b) xs
        Term::Tuple(xs) => {
            for x in xs {
                encode_value(b, x);
            }
        }
        // encodeValue b (Alt n x) = do
        //   FFI.call $ glean_push_value_selector b (fromIntegral n)
        //   encodeValue b x
        Term::Alt(n, x) => {
            // RULE §2 (A5): Word64 -> CSize via `as` — the source's
            // fromIntegral; identity on LP64 (selector values in practice
            // come from dSelector-sized data).
            // SAFETY: as the Byte arm.
            invoke_push(|| unsafe { sys::glean_push_value_selector(b.as_ptr(), *n as usize) });
            encode_value(b, x);
        }
        // encodeValue b (String s) = FFI.unsafeWithBytes s $ \p n ->
        //   FFI.call $ glean_push_value_string b (castPtr p) n
        Term::String(s) => {
            // SAFETY: as the Byte arm; `s.as_ptr()`/`s.len()` are the
            // buffer/len pair unsafeWithBytes pinned (the pointer is
            // non-null even for the empty string; size 0 makes the C++ a
            // terminator-only write).
            invoke_push(|| unsafe {
                sys::glean_push_value_string(b.as_ptr(), s.as_ptr() as *const c_void, s.len())
            });
        }
        // encodeValue b (Ref id) = FFI.call $ glean_push_value_fact b id
        Term::Ref(id) => {
            // Fid is #[repr(transparent)] over i64 (types.rs) — the
            // by-value marshalling the Haskell's Storable bought.
            // SAFETY: as the Byte arm.
            invoke_push(|| unsafe { sys::glean_push_value_fact(b.as_ptr(), id.0) });
        }
    }
}

/// RTS.hs:92-96:
/// `encodeByteArray b xs = BS.unsafeUseAsCStringLen xs $ \(p,n) -> do
///    FFI.call $ glean_push_value_array b (fromIntegral n)
///    FFI.call $ glean_push_value_bytes b (castPtr p) (fromIntegral n)`.
///
/// Both pushes carry the same `n` (the length prefix then the bytes); both
/// are ALLOCATION-GUARD -> abort via `invoke_push`.
pub fn encode_byte_array(b: &mut Builder, xs: &[u8]) {
    // SAFETY: `b` is a live Builder; the push appends the length prefix;
    // invoke's contract covers the returned pointer.
    invoke_push(|| unsafe { sys::glean_push_value_array(b.as_ptr(), xs.len()) });
    // SAFETY: as above; `xs` is borrowed for the call and the C++ side
    // copies (Output::bytes -> memcpy) before returning.
    invoke_push(|| unsafe {
        sys::glean_push_value_bytes(b.as_ptr(), xs.as_ptr() as *const c_void, xs.len())
    });
}

/// RTS.hs:101-102: `fromValue val = unsafePerformIO $ withValue val
/// finishBuilder`.
///
/// Inventory rows 58/59: pure Rust fn `Value -> Vec<u8>`; the
/// unsafePerformIO does NOT survive (the FFI calls are ordinary calls —
/// "no hidden effect survives"), and the one fallible step (a push) aborts
/// inside `invoke_push` per the builder-family ruling (the source's
/// uncaught RTS OOM death, made deterministic). Consumes the value like
/// the source (call sites hand over freshly built ones —
/// Query/Transform.hs:249/:312-313).
pub fn from_value(val: Value) -> Vec<u8> {
    with_value(&val, |b| builder::finish_builder(b))
}

/// RTS.hs:98-99: `toValue ty bs = runST $ withDecoder bs $ \d ->
/// decodeValue d ty`.
///
/// Pure signature preserved (the source ran it in ST): the cursor reads
/// are read-only over the borrowed bytes, and structural failures come
/// back as [`DecodeError`] values (row 58's "explicit EOF/extra-bytes
/// error") rather than exceptions.
pub fn to_value(ty: &Rep<Pid>, bs: &[u8]) -> Result<Value, DecodeError> {
    with_decoder(bs, |d| decode_value(d, ty))
}

// ---------------------------------------------------------------------------
// Byte/string references (RTS.hs:104-133)
// ---------------------------------------------------------------------------

/// RTS.hs:104-108: `data ByteStringRef = ByteStringRef
///   { byteStringRefPtr :: !(Ptr Word8), byteStringRefSize :: !CSize }`.
///
/// RULE §1 (strings/bytes row) + row 60: the C (pointer, length) pair
/// becomes a borrowed slice — `&[u8]` for borrowed data (`Vec<u8>` for
/// owned). `as_ptr`/`len` recover the C view on the same terms the
/// Haskell wrapped (including the empty case, where the C sites can have
/// a null pointer — Query/JSON.hs:389-390 documents exactly that).
/// `Copy` mirrors the source's plain unboxed pair; the borrow keeps every
/// construction honest about the memory staying alive.
#[derive(Clone, Copy)]
pub struct ByteStringRef<'a> {
    pub bytes: &'a [u8],
}

impl<'a> ByteStringRef<'a> {
    /// The pointer half of the C pair (`byteStringRefPtr`, RTS.hs:106).
    /// NOT guaranteed non-null for an empty reference (see the struct
    /// doc); callers passing it to C must respect `len() == 0`.
    pub fn as_ptr(&self) -> *const u8 {
        self.bytes.as_ptr()
    }

    /// The size half of the C pair (`byteStringRefSize`, RTS.hs:107).
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// `byteStringRefSize == 0`.
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Rebuild the reference from the C (pointer, size) pair — the
    /// constructor the Haskell export `ByteStringRef(..)` exposed.
    ///
    /// SAFETY: `p` must be valid for reads of `n` bytes for `'a` when
    /// `n > 0`; for `n == 0` any pointer is accepted and folded to the
    /// empty slice (slice::from_raw_parts demands non-null even at length
    /// 0; the null-with-0 pair the C side can produce is normalized here,
    /// with the same observable value the Haskell's lazy
    /// unsafePackMallocCStringLen(nullPtr, 0) had).
    pub unsafe fn from_raw(p: *const u8, n: usize) -> Self {
        // SAFETY: caller contract above; the empty case never touches `p`.
        ByteStringRef {
            bytes: if n == 0 {
                &[]
            } else {
                unsafe { std::slice::from_raw_parts(p, n) }
            },
        }
    }
}

/// RTS.hs:110-113: `unsafeWithByteStringRef s f =
///   FFI.unsafeWithBytes s $ \p n -> f $ ByteStringRef (castPtr p) n`.
///
/// The name keeps the source's warning; the signature is SAFE for the
/// same reason crate::ffi::unsafe_with_bytes is (its RULE §2/A7 note):
/// the reference derives from a live `&[u8]` borrow, so Rust's borrow
/// checking pins validity for the call exactly as GHC pinned the
/// ByteString. The one obligation left is the name's: `f` must not retain
/// the reference beyond the call.
pub fn unsafe_with_byte_string_ref<R>(s: &[u8], f: impl FnOnce(ByteStringRef<'_>) -> R) -> R {
    f(ByteStringRef { bytes: s })
}

/// RTS.hs:115-117: `derefByteString (ByteStringRef p n) =
///   FFI.copyByteString (castPtr p) n` — copy the referenced bytes into an
/// owned buffer. The IO shell was the FFI copy itself; the port is a
/// plain copy (`Vec<u8>` per RULE §1).
pub fn deref_byte_string(r: ByteStringRef<'_>) -> Vec<u8> {
    r.bytes.to_vec()
}

/// RTS.hs:119-123: `newtype MangledStringRef =
///   MangledStringRef { mangledBytes :: ByteStringRef }` — the byte range
///   INCLUDING the terminator (source comment carried).
#[derive(Clone, Copy)]
pub struct MangledStringRef<'a> {
    pub mangled_bytes: ByteStringRef<'a>,
}

/// RTS.hs:125-129: `unsafeWithMangledStringRef s f =
///   unsafeWithByteStringRef s $ f . MangledStringRef`.
///
/// Same SAFE-signature reasoning as [`unsafe_with_byte_string_ref`]; the
/// wrapper adds no obligations.
pub fn unsafe_with_mangled_string_ref<R>(
    s: &[u8],
    f: impl FnOnce(MangledStringRef<'_>) -> R,
) -> R {
    unsafe_with_byte_string_ref(s, |r| f(MangledStringRef { mangled_bytes: r }))
}

/// RTS.hs:131-133: `mangleString s = BS.intercalate "\0\1" (BS.split 0 s)
///   <> "\0\0"` — "Yield the internal representation of a UTF-8 string".
///
/// Read operationally: `split 0` cuts at every NUL byte, so `intercalate`
/// inserts the two-byte separator `00 01` exactly at each NUL, and the
/// terminator `00 00` is appended; the empty input yields just `00 00`.
/// Byte-for-byte this is the C++ `rts::mangleString` the push symbol uses
/// (string.cpp:132-146), which is the representation the decoder's
/// trusted path expects.
pub fn mangle_string(s: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len() + 2);
    for &b in s {
        if b == 0 {
            out.push(0);
            out.push(1);
        } else {
            out.push(b);
        }
    }
    out.push(0);
    out.push(0);
    out
}

/// RTS.hs:243-245: `demangle (MangledStringRef (ByteStringRef p n)) q =
///   glean_string_demangle_trusted p n q`.
///
/// Writes the demangled bytes into `q` and returns how many were written —
/// the DEMANGLED size, a function of the input's NUL layout. The buffer
/// length is not a parameter of the C function and is never checked
/// against anything: that unbounded write is exactly why this is an
/// `unsafe fn` (RULE §2's A7 row names demangle as its worked case; see
/// the INVENTORY-BUG-CANDIDATE row 60 block at the top of the file). The
/// buffer must be sized from a prior d_trusted_string_ref on the same
/// range (Query/JSON.hs:377-383 allocates exactly that size and only
/// asserts the equality AFTER the write).
///
/// SAFETY: `r` must be a well-formed TRUSTED mangled string (interior NUL
/// separators followed by `01`, `00 00` terminator before the range end)
/// — malformed input is a C++ glog CHECK, i.e. process death, not an
/// error value (string.cpp:95-121, noexcept; the source had no channel
/// here either). `q` must be valid for writes of the demangled size. No
/// memory is retained on either side.
pub unsafe fn demangle(r: &MangledStringRef<'_>, q: *mut u8) -> usize {
    // SAFETY: caller contract above; the C++ side only reads the range and
    // writes the caller's buffer; the symbol has no error channel.
    unsafe {
        sys::glean_string_demangle_trusted(r.mangled_bytes.as_ptr(), r.mangled_bytes.len(), q)
    }
}

// ---------------------------------------------------------------------------
// Decoding (RTS.hs:135-266)
// ---------------------------------------------------------------------------

/// RTS.hs:135-140: `data Decoder s = Decoder
///   { decoderPtr :: !(Ptr (Ptr ())), decoderEnd :: !(Ptr ()),
///     decoderBuf :: !FFI.FFIResultBuf }`.
///
/// The Haskell's `s` is the ST state token that threads decoders through
/// `runST` and keeps borrowed refs from escaping (RTS.hs:148-164); the
/// Rust lifetime `'a` is that discipline: refs obtained from a decoder
/// borrow the input, and the borrow checker stops them outliving it (row
/// 60: "borrowed &[u8] (lifetime = decoder)"). Fields are private and
/// [`with_decoder`] is the only constructor — the export list says
/// `Decoder`, not `Decoder(..)`.
///
/// `cursor` is the shared cursor cell's contents (the `decoderPtr`
/// pointee, updated through the C calls on success only); `end` is the
/// one-past-end bound; `input` is the live borrow the cursors point into
/// (see INVENTORY-BUG-CANDIDATE row 60, deviation 2).
pub struct Decoder<'a> {
    cursor: *const c_void,
    end: *const c_void,
    input: &'a [u8],
}

/// RTS.hs:142-145: `newtype DecodingException = DecodingException String
///   deriving newtype Show`; `instance Exception DecodingException`.
///
/// Display is the RAW message (no quotes): the message is a diagnostic
/// that surfaces through error-reporting paths, and the crate's canonical
/// error surface displays messages verbatim (crate::ffi::FfiError: its
/// Display "shows it verbatim, matching instance Show FFIError") — one
/// story for both channels of [`DecodeError`]. (RULE §0 note: the
/// Haskell's `deriving newtype Show` would render the quoted String, but
/// the two in-tree uses both want the RAW text — glean/test/tests/Schema/
/// Lib.hs:96 pattern-matches `RTS.DecodingException e` and binds the raw
/// string `e`, and the sibling Typed/Build.hs:72-76 gives its OWN
/// DecodingException `show = Text.unpack . decodingException` (raw). No
/// in-tree consumer ever sees the quoted form, so raw Display is the
/// parity choice.)
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct DecodingException(pub String);

/// The decode-side error channel: C-ABI errors ([`crate::ffi::FfiError`] —
/// the pop symbols' precondition/allocation messages) and this module's
/// own [`DecodingException`] (extra bytes). Consumers with their own error
/// type need not use it — `with_decoder` is generic over `E`.
#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    #[error(transparent)]
    Ffi(#[from] FfiError),
    #[error(transparent)]
    Decoding(#[from] DecodingException),
}

/// RTS.hs:147-164: `withDecoder bs f = unsafeIOToST $
///   FFI.withFFIResultBuf $ \ !buf -> FFI.unsafeWithBytes bs $ \start len ->
///   alloca $ \pstart -> do
///     poke pstart start
///     let end = start `plusPtr` fromIntegral len
///     res <- unsafeSTToIO $ f Decoder { decoderPtr = pstart
///                                     , decoderEnd = end
///                                     , decoderBuf = buf }
///     pos <- peek pstart
///     when (pos /= end) $
///       let extra = show (end `minusPtr` pos) in
///       throwIO $ DecodingException $
///         "extra " <> extra <> " bytes at end of value"
///     return res`.
///
/// The alloca'd cursor cell and the FFIResultBuf dissolve into the
/// Decoder's fields (row 60's "shared FFIResultBuf slot trick" clause; the
/// per-call result slot is marked at `dffi`). The trailing check runs only
/// when `f` returned Ok — the source's exception flow skips the `when`
/// when `f` throws — and the message is byte-identical, `show`-formatted
/// byte count included.
///
/// E is generic so nested decoders compose with the CONSUMER's error type
/// (Query/JSON.hs:151-152 nests `withDecoder` for key and value inside its
/// own monad): E must absorb this module's two channels, `From<FfiError>`
/// and `From<DecodingException>`; with `E = DecodeError` both are
/// automatic.
pub fn with_decoder<'a, R, E>(
    bs: &'a [u8],
    f: impl FnOnce(&mut Decoder<'a>) -> Result<R, E>,
) -> Result<R, E>
where
    E: From<FfiError> + From<DecodingException>,
{
    let cursor = bs.as_ptr() as *const c_void;
    // SAFETY: one-past-end pointer arithmetic on a slice's own base is a
    // valid pointer (same allocation or one past its end); no access.
    let end = unsafe { bs.as_ptr().add(bs.len()) } as *const c_void;
    let mut dec = Decoder {
        cursor,
        end,
        input: bs,
    };
    let res = f(&mut dec)?;
    // extra = show (end `minusPtr` pos) (RTS.hs:160-163). The difference is
    // non-negative on every reachable path: the C pop bodies only advance
    // the cursor inside [start, end] (every read goes through
    // binary::Input::want/bytes, binary.h:119-128/:200-206), so the usize
    // arithmetic matches the Haskell's Int `minusPtr` without its
    // unreachable negative branch.
    let extra = (dec.end as usize) - (dec.cursor as usize);
    if extra != 0 {
        return Err(E::from(DecodingException(format!(
            "extra {} bytes at end of value",
            extra
        ))));
    }
    Ok(res)
}

impl<'a> Decoder<'a> {
    /// RTS.hs:166-171 (`dffi`): run one pop whose result lands in a single
    /// out-slot, then read the slot — the Rust twin of `FFI.ffiBuf`'s
    /// invoke-then-read shape. The slot is a fresh `MaybeUninit` local.
    // PERF(port): row 60: the ffiBuf/ffiBuf2 shared-slot becomes plain
    // locals, the fast shared-slot version deferred until a judge shows
    // decode cost. That fast version is a reused slot owned by the
    // Decoder — or crate::ffi::FFIResultBuf — removing the per-call local;
    // dffi/dffi2 are the combined site.
    fn dffi<R>(
        &mut self,
        f: unsafe extern "C" fn(
            *mut *const c_void,
            *const c_void,
            *mut R,
        ) -> *const c_char,
    ) -> Result<R, FfiError> {
        let mut slot = MaybeUninit::<R>::uninit();
        let mut start = self.cursor;
        // SAFETY: `slot` is a live stack out-param, `start` an initialised
        // stack copy of the cursor (the C side writes through `&mut`),
        // and `start`/`end` are the input range with_decoder installed;
        // `f` is one of the pop symbols whose C contract is exactly this
        // pointer pair + out-slot. The returned pointer follows
        // crate::ffi::invoke's contract (null on success or a released
        // wrap.h message).
        unsafe { invoke(|| f(&mut start, self.end, slot.as_mut_ptr())) }?;
        // The C++ pop_value helper writes *start back ON SUCCESS ONLY
        // (ffi.cpp:58-66), so an Err leaves the cursor where it was — the
        // Haskell's shared pstart cell saw exactly the same.
        self.cursor = start;
        // SAFETY: the call reported success, so it wrote exactly one R
        // into the initialised slot (every pop body assigns its out-param
        // before returning null); R is owned by value.
        Ok(unsafe { slot.assume_init() })
    }

    /// RTS.hs:173-180 (`dffi2`): the two-slot variant (only
    /// `glean_pop_value_string` uses it). Same local-slot note as [`Self::dffi`].
    fn dffi2<A, B>(
        &mut self,
        f: unsafe extern "C" fn(
            *mut *const c_void,
            *const c_void,
            *mut A,
            *mut B,
        ) -> *const c_char,
    ) -> Result<(A, B), FfiError> {
        let mut slot_a = MaybeUninit::<A>::uninit();
        let mut slot_b = MaybeUninit::<B>::uninit();
        let mut start = self.cursor;
        // SAFETY: as `dffi`, for both out-slots; `f` is the string pop
        // whose C contract is (start, end, void** bytes, size_t* size).
        unsafe {
            invoke(|| {
                f(
                    &mut start,
                    self.end,
                    slot_a.as_mut_ptr(),
                    slot_b.as_mut_ptr(),
                )
            })
        }?;
        // Success-only write-back, as `dffi`.
        self.cursor = start;
        // SAFETY: both slots were written by the successful call; read
        // once each, by value.
        Ok(unsafe { (slot_a.assume_init(), slot_b.assume_init()) })
    }

    /// RTS.hs:182-184: `dByte = dffi glean_pop_value_byte` — next byte.
    pub fn d_byte(&mut self) -> Result<u8, FfiError> {
        self.dffi(sys::glean_pop_value_byte)
    }

    /// RTS.hs:186-188: `dNat = dffi glean_pop_value_nat` — packed natural.
    pub fn d_nat(&mut self) -> Result<u64, FfiError> {
        self.dffi(sys::glean_pop_value_nat)
    }

    /// RTS.hs:190-192: `dArray = dffi glean_pop_value_array` — array
    /// length prefix.
    pub fn d_array(&mut self) -> Result<usize, FfiError> {
        self.dffi(sys::glean_pop_value_array)
    }

    /// RTS.hs:194-195: `dSet = dffi glean_pop_value_set` — set length
    /// prefix.
    pub fn d_set(&mut self) -> Result<usize, FfiError> {
        self.dffi(sys::glean_pop_value_set)
    }

    /// RTS.hs:197-202: `dByteStringRef d n = do
    ///   p <- dffi (\s e -> glean_pop_value_bytes_ref s e n) d
    ///   return $ ByteStringRef p n`.
    ///
    /// The reference borrows the decoder's input (row 60: "dByteStringRef
    /// -> borrowed &[u8] (lifetime = decoder)"): the C side hands back a
    /// pointer INTO the range with_decoder borrows, never a copy.
    pub fn d_byte_string_ref(&mut self, n: usize) -> Result<ByteStringRef<'a>, FfiError> {
        let mut p: *const c_void = ptr::null();
        let mut start = self.cursor;
        // SAFETY: `p`/`start` are initialised stack out-params; start/end
        // are the input range; the captured `n` is the C call's size
        // argument (ffi.cpp:745-753 borrows `input.bytes(size)` — with the
        // truncation precondition). invoke owns the error string.
        unsafe { invoke(|| sys::glean_pop_value_bytes_ref(&mut start, self.end, n, &mut p)) }?;
        self.cursor = start;
        // SAFETY: on success `*bytes` is `input.bytes(n).data()`
        // (ffi.cpp:750-752) — a pointer into the input this decoder
        // borrows for 'a, having passed the want(n) range check, so it is
        // readable for `n` bytes for 'a (from_raw folds n == 0 to the
        // empty slice regardless of the pointer).
        Ok(unsafe { ByteStringRef::from_raw(p as *const u8, n) })
    }

    /// RTS.hs:204-209: `dBytes d n = do
    ///   -- TODO: reimplement based on dByteStringRef
    ///   p <- dffi (\s e -> glean_pop_value_bytes s e n) d
    ///   unsafeIOToST $ FFI.unsafeMallocedByteString (castPtr p) n`.
    // PERF(port): the source's own TODO (RTS.hs:207) — the fast version
    // reimplements this on top of d_byte_string_ref (borrow + Rust-side
    // copy), skipping the C++ malloc + libc::free round trip. Kept on the
    // C path for byte-identical ownership semantics until the judge shows
    // decode cost.
    pub fn d_bytes(&mut self, n: usize) -> Result<Vec<u8>, FfiError> {
        let mut p: *mut c_void = ptr::null_mut();
        let mut start = self.cursor;
        // SAFETY: `p`/`start` are initialised stack out-params; start/end
        // are the input range; the captured `n` sizes the range copy
        // (ffi.cpp:755-763 — truncation precondition + clone_bytes
        // allocation, both returned via the slot). invoke owns the error
        // string.
        unsafe { invoke(|| sys::glean_pop_value_bytes(&mut start, self.end, n, &mut p)) }?;
        self.cursor = start;
        // SAFETY: on success `*bytes`/`n` are the malloc'd ownership
        // transfer (ffi::clone_bytes(...).release(), ffi.cpp:761) —
        // adopt_malloced_bytes' contract; size 0 included.
        Ok(unsafe { adopt_malloced_bytes(p, n) })
    }

    /// RTS.hs:211-213: `dSelector = dffi glean_pop_value_selector` — sum
    /// selector.
    pub fn d_selector(&mut self) -> Result<usize, FfiError> {
        self.dffi(sys::glean_pop_value_selector)
    }

    /// RTS.hs:215-229:
    /// `dTrustedStringRef Decoder{..} = unsafeIOToST $ do
    ///   start <- peek decoderPtr
    ///   n <- glean_pop_value_trusted_string_ref decoderPtr decoderEnd
    ///   end <- peek decoderPtr
    ///   return (MangledStringRef { mangledBytes = ByteStringRef
    ///     { byteStringRefPtr = castPtr start
    ///     , byteStringRefSize = fromIntegral $ end `minusPtr` start } }, n)`.
    ///
    /// The Haskell peeks the cursor before and after because the C
    /// function advances it in place; the port computes the same two
    /// offsets and slices the live input between them — in-bounds
    /// indexing, so an arithmetic slip is a panic, never UB. The slice
    /// spans the MANGLED range including the terminator (as the source's
    /// ByteStringRef did), while the returned size is the DEMANGLED size
    /// (`size - nuls - 1`, string.cpp:120).
    ///
    /// No Result: the symbol has no error slot (ffi.h:360) and malformed
    /// input is a C++ CHECK abort inside the call (abort-class), exactly
    /// as in the source.
    pub fn d_trusted_string_ref(&mut self) -> (MangledStringRef<'a>, usize) {
        let base = self.input.as_ptr() as usize;
        let start = (self.cursor as usize) - base;
        // SAFETY: self.cursor/self.end point into self.input (with_decoder
        // is the only constructor and every C pop advances only within
        // [start, end]); the call reads the mangled string, writes the
        // advanced cursor back through `&mut self.cursor`, and returns the
        // demangled size. No error channel exists.
        let n = unsafe { sys::glean_pop_value_trusted_string_ref(&mut self.cursor, self.end) };
        let end = (self.cursor as usize) - base;
        // In-bounds by the C++ advance contract; a panic (not UB) if that
        // ever breaks. `&self.input[..]` reborrows the 'a slice behind the
        // &mut self, so the reference outlives the borrow of the decoder —
        // the row's "lifetime = decoder".
        let mangled_bytes = ByteStringRef {
            bytes: &self.input[start..end],
        };
        (MangledStringRef { mangled_bytes }, n)
    }

    /// RTS.hs:232-237: `dString d = do
    ///   -- TODO: reimplement based on dStringRef
    ///   (p,n) <- dffi2 glean_pop_value_string d
    ///   unsafeIOToST $ FFI.unsafeMallocedByteString (castPtr p) n`.
    // PERF(port): the source's own TODO (RTS.hs:235) — the fast version
    // demangles in Rust over the borrowed trusted range instead of the
    // C++ fresh-Output + moveBytes path. Kept on the C path for
    // byte-identical validation errors (string.cpp:72/:75/:78) until the
    // judge shows decode cost.
    pub fn d_string(&mut self) -> Result<Vec<u8>, FfiError> {
        let (p, n) = self.dffi2(sys::glean_pop_value_string)?;
        // SAFETY: on success `p`/`n` are the malloc'd demangled bytes
        // (ffi.cpp:779-782: demangleUntrustedString into a fresh Output,
        // then output.moveBytes().release_to) — adopt_malloced_bytes'
        // contract; the empty case (0-length output) is folded there.
        Ok(unsafe { adopt_malloced_bytes(p, n) })
    }

    /// RTS.hs:239-241: `dFact = dffi glean_pop_value_fact` — fact id.
    /// The C writes the raw i64 (id.h:123-129 toThrift is the identity);
    /// `Fid` is the transparent wrapper.
    pub fn d_fact(&mut self) -> Result<Fid, FfiError> {
        self.dffi(sys::glean_pop_value_fact).map(Fid)
    }
}

/// Adopt the malloc'd ownership transfer the TWO transfer pops
/// (`glean_pop_value_bytes`, `glean_pop_value_string`) hand back, with the
/// empty-case guard rts/builder.rs::finish_builder established for the
/// same clone_bytes/moveBytes family: a null pointer with size 0 (an empty
/// clone on platforms whose malloc(0) may return null) is the empty Vec —
/// never handed to slice::from_raw_parts.
///
/// SAFETY: `p`/`n` must be a pair the C side just transferred ownership
/// of (malloc'd, exactly `n` readable bytes when `p` is non-null, or the
/// (NULL, 0) empty case), not retained elsewhere.
unsafe fn adopt_malloced_bytes(p: *mut c_void, n: usize) -> Vec<u8> {
    if p.is_null() {
        // malloc(0) may legally return NULL; nothing to free and no bytes
        // to copy — mirror of finish_builder's guard.
        debug_assert_eq!(n, 0);
        return Vec::new();
    }
    // SAFETY: caller contract above; unsafe_malloced_byte_string copies
    // into the Vec and releases the C allocation with libc::free (its own
    // contract, crate::ffi).
    unsafe { unsafe_malloced_byte_string(p, n) }
}

/// RTS.hs:247-266 — `decodeValue :: Decoder s -> Rep Pid -> ST s (Term Fid)`
/// (not in the source's export list; private here too).
///
/// Shape notes, per arm:
///   * ArrayRep: `size <- dArray d` (a CSize), then ByteRep elements take
///     the dBytes fast path; every other element decodes recursively
///     `replicateM (fromIntegral size)` times — and `fromIntegral` is the
///     CSize→Int conversion, so a size ≥ 2^63 wraps NEGATIVE and replicates
///     ZERO times (see the arm). The size is NOT pre-validated — a huge
///     size fails on the first truncated element (precondition-class) —
///     so the port pushes into an EMPTY Vec rather than trusting it with
///     with_capacity.
///   * TupleRep: fixed arity from the TYPE — with_capacity(tys.len()) is
///     bounded by the type itself.
///   * SumRep: `sel <- dSelector d; Alt (fromIntegral sel) <$> decodeValue
///     d (tys !! fromIntegral sel)`. A bad index panics with the literal
///     `Prelude.!!` text (GHC 9.4.7): "negative index" is reachable
///     because `fromIntegral` reinterprets a selector above
///     maxBound::Int64 as negative — the same `as i64` here.
///   * SetRep: `Array . uniq <$> replicateM ...` — uniq = `Set.toList .
///     Set.fromList` (Util/List.hs:58-59): sort-then-dedup under the
///     element Ord (Term's derived constructor-order Ord on both sides),
///     NOT insertion order.
///   * StringRep -> owned demangled bytes via dString; PredicateRep _ ->
///     Term::Ref via dFact (the predicate id in the representative is
///     ignored, exactly as the source's `PredicateRep _`).
fn decode_value(dec: &mut Decoder<'_>, ty: &Rep<Pid>) -> Result<Value, DecodeError> {
    match ty {
        // ByteRep -> Byte <$> dByte d
        Rep::ByteRep => Ok(Term::Byte(dec.d_byte()?)),
        // NatRep -> Nat <$> dNat d
        Rep::NatRep => Ok(Term::Nat(dec.d_nat()?)),
        // ArrayRep elty -> do { size <- dArray d; case elty of
        //   ByteRep -> ByteArray <$> dBytes d size
        //   _ -> Array <$> replicateM (fromIntegral size) (decodeValue d elty) }
        Rep::ArrayRep(elty) => {
            let size = dec.d_array()?;
            match &**elty {
                Rep::ByteRep => Ok(Term::ByteArray(dec.d_bytes(size)?)),
                _ => {
                    // RULE §2 (numeric-conversions): the source's
                    // `replicateM (fromIntegral size)` (RTS.hs:257) converts
                    // the CSize to Int, so a size ≥ 2^63 wraps NEGATIVE and
                    // replicateM on a negative count replicates ZERO times —
                    // an EMPTY Array, with no element decode attempted and
                    // the cursor untouched. Reproduced bug-for-bug with the
                    // explicit `as i64`; the loop runs only when count > 0.
                    let count = size as i64;
                    let mut xs = Vec::new();
                    if count > 0 {
                        for _ in 0..count {
                            xs.push(decode_value(dec, elty)?);
                        }
                    }
                    Ok(Term::Array(xs))
                }
            }
        }
        // TupleRep tys -> Tuple <$> mapM (decodeValue d) tys
        Rep::TupleRep(tys) => {
            let mut xs = Vec::with_capacity(tys.len());
            for t in tys {
                xs.push(decode_value(dec, t)?);
            }
            Ok(Term::Tuple(xs))
        }
        // SumRep tys -> do { sel <- dSelector d;
        //   Alt (fromIntegral sel) <$> decodeValue d (tys !! fromIntegral sel) }
        Rep::SumRep(tys) => {
            let sel = dec.d_selector()?;
            // RULE §2 (A5 note): the explicit `as` trampoline of the
            // source's `fromIntegral sel` (CSize -> Int); wraps above
            // maxBound::Int64 into the negative branch, as in the source.
            let idx = sel as i64;
            if idx < 0 {
                // Literal `!!` panic text, byte for byte.
                panic!("Prelude.!!: negative index");
            }
            let ty = tys
                .get(idx as usize)
                .unwrap_or_else(|| panic!("Prelude.!!: index too large"));
            Ok(Term::Alt(sel as u64, Box::new(decode_value(dec, ty)?)))
        }
        // SetRep elty -> do { size <- dSet d;
        //   Array . uniq <$> replicateM (fromIntegral size) (decodeValue d elty) }
        Rep::SetRep(elty) => {
            let size = dec.d_set()?;
            // Same RULE §2 (numeric-conversions) ruling as ArrayRep: the
            // source's `replicateM (fromIntegral size)` (RTS.hs:264) is the
            // CSize→Int conversion, so a size ≥ 2^63 wraps NEGATIVE and
            // replicates ZERO times — the EMPTY `uniq []` Array, with no
            // element decode attempted. Reproduced bug-for-bug.
            let count = size as i64;
            let mut xs = Vec::new();
            if count > 0 {
                for _ in 0..count {
                    xs.push(decode_value(dec, elty)?);
                }
            }
            xs.sort();
            xs.dedup();
            Ok(Term::Array(xs))
        }
        // StringRep -> String <$> dString d
        Rep::StringRep => Ok(Term::String(dec.d_string()?)),
        // PredicateRep _ -> Ref <$> dFact d
        Rep::PredicateRep(_) => Ok(Term::Ref(dec.d_fact()?)),
    }
}

// Marker breakdown (closed set — no open port TODOs, no source-defect
// reproductions, 3 performance deferrals, 1 inventory-bug-candidate):
//   * 3 performance deferrals: dffi/dffi2 (combined, the reused-slot note;
//     row 60's own instruction), d_bytes (source TODO RTS.hs:207), d_string
//     (source TODO RTS.hs:235). No open question remains in any of them —
//     all three name the fast version and the deferral condition.
//   * 1 inventory-bug-candidate (row 60, top of file): the two deliberate
//     representation divergences (demangle's unsafe raw-pointer surface
//     per A7; the Decoder's third `input` field carrying the row's own
//     input borrow). The row's "cxx Builder" wording is cancelled out by
//     the family-wide flag in rts/builder.rs (cited, not duplicated).
//   * no open TODOs of the port kind: every open-looking question in the
//     source (the two reimplementation TODOs) is a performance-class
//     deferral, not an unknown; rows 58/59/60 close the
//     unsafePerformIO/ownership questions.
//   * no source-defect reproductions: no bug-class flag is needed in this
//     unit — the `!!` panics, the extra-bytes message and the
//     fromIntegral-wrap-to-empty-array in decode_value are faithful
//     behaviours, not defects; the A7 restoration of demangle's unsafe
//     signature is an A7 mandate, flagged above, not a bug reproduction.
// SAFETY marker count: 23 unsafe blocks, each carrying an inline safety
// comment — encode 9 (six pushes in encode_value + the two in
// encode_byte_array + invoke_push's `invoke`), refs 2 (from_raw's slice
// read; demangle's extern call), decode 12 (with_decoder's `add`; dffi's
// invoke + assume_init; dffi2's invoke + assume_init; d_trusted_string_ref's
// extern call; d_byte_string_ref's invoke + from_raw; d_bytes' invoke +
// adopt; d_string's adopt; adopt_malloced_bytes' transfer call). The
// `pub mod sys` `unsafe extern` block is a declaration, not an operation
// (bytecode.rs precedent), so it is not counted; `unsafe fn` from_raw /
// demangle / adopt_malloced_bytes additionally carry doc-level safety
// contracts.
// Coverage: every export of the Haskell module has a Rust counterpart —
// Fid(..)/lowestFid + Pid(..)/lowestPid (re-exports above),
// withValue/fromValue/toValue/encodeByteArray, mangleString,
// ByteStringRef(..)/unsafeWithByteStringRef/derefByteString,
// MangledStringRef(..)/unsafeWithMangledStringRef/demangle, Decoder,
// DecodingException(..), withDecoder, dByte/dNat/dTrustedStringRef/dString/
// dArray/dSet/dByteStringRef/dBytes/dSelector/dFact, and the twenty raw
// push/pop/push_fact/demangle foreign imports (sys). encodeValue and
// decodeValue are private here as they are unexported there.
// PORT STATUS: confidence=high todos=0
