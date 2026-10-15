// Ported from: glean/hs/Glean/RTS/Set.hs (98 lines)
// Manifest unit: rust/glean-core/src/rts/set.rs (migration/manifest.tsv
// line 14, crate glean-core). RULE §4 (naming and output paths): one
// source file -> one target file; the manifest path is authoritative.
//
// What this module is: two owned handles over the C++ RTS dedup sets —
// `facebook::glean::rts::BytestringSet` (std::set<folly::fbstring,
// CmpSet>, glean/rts/set.h:20-28) and `facebook::glean::rts::WordSet`
// (std::set<uint64_t>, set.h:30) — plus the ten flat FFI symbols that
// create, fill, serialise and free them (glean/rts/ffi.cpp:585-643).
// The sets deduplicate repeated elements and write their (hence
// deterministic, sorted) contents into a Builder; the two in-tree
// consumers are the JSON batch writer (glean/db/Glean/Write/JSON.hs) and
// the typed binary codec (glean/typed/Glean/Typed/Binary.hs).
//
// ABI truth (READ FROM DISK — the sources below, not this prose, are the
// authority):
//   glean/rts/ffi.cpp:585-643 defines all ten symbols:
//     glean_rtsset_new              :585-587  `BytestringSet**` out-param; `new BytestringSet()`
//     glean_rtsset_insert           :589-591  (set, Output*) — MOVES the Output's contents (:590)
//     glean_rtsset_build            :593-600  (set, Output*) — packed size + raw element bytes
//     glean_rtsset_free             :602-604  (set) -> void; ffi::free_
//     glean_wordrtsset_new          :606-608  `WordSet**` out-param; `new WordSet()`
//     glean_wordrtsset_insert       :610-612  (set, uint64_t)
//     glean_wordrtsset_insert_bytes :614-621  (set, const uint8_t*, size_t); elem per BYTE (:618)
//     glean_wordrtsset_build        :623-630  (set, Output*) — packed size + packed(element) each
//     glean_wordrtsset_build_bytes  :632-639  (set, Output*) — packed size; fixed<uint8_t> each
//     glean_wordrtsset_free         :641-643  (set) -> void; ffi::free_
//   The C++ value types: glean/rts/set.h:20-30 (CmpSet =
//   std::lexicographical_compare over the fbstring's bytes;
//   `BytestringSet`/`WordSet` aliases). The Haskell foreign imports are
//   Set.hs:70-98; both `free` symbols are imported under exact link names
//   glean_rtsset_free / glean_wordrtsset_free (Set.hs:79-80, :97-98) and
//   the Rust externs keep those spellings verbatim (only the wrapper fn
//   names differ, as in the source).
//
// ABI-TRUTH NOTE (grep-verified on disk, 2026-10-15): this unit's brief
// said to read the glean_rtsset_*/glean_wordrtsset_* declarations from
// glean/rts/ffi.h — they are NOT there. The 563-line ffi.h declares its
// neighbours (`glean_factset_*` :268-311, the builder group :320-327,
// `glean_stacked_*` :313-318) and contains zero `rtsset` matches
// (case-insensitive); the ten symbols exist only as the ffi.cpp
// definitions above plus the Haskell imports. The `unsafe extern "C"`
// block below is therefore pinned to the C++ DEFINITIONS (the linkage
// authority) and set.h's type aliases; every type agrees with
// Set.hs:70-98. Reported for the reviewer; no inventory row covers the
// header text itself, and this is not a divergence from row 79 — no
// INVENTORY-BUG-CANDIDATE flag is raised.
//
// INVENTORY (RULE §5): row 79 (migration/inventory.tsv line 79 — the row
// id convention v0.3 itself uses; the ffi shard migration/inventory.ffi.tsv
// row 23 carries the SAME substantive text, MINUS the later in-place A2
// annotation recorded in the row's own target_translation — see the A2
// sub-bullet below) — target_translation, quoted:
// "C++ dedup sets written into a Builder; raw-Ptr wrappers (not
// ForeignPtr); bracketed via withRtsSet" and "conservative: cxx bridge
// [SUPERSEDED by §1/A2: raw extern-C — 2026-10-07] with RAII guard (Drop =
// glean_rtsset_free / glean_wordrtsset_free); TODO(port): confirm dedup ->
// fid assignment order is insertion order so a rebuilt batch produces
// byte-identical ids (needs judge case: same facts via RtsSet vs direct
// Builder define -> same first_id/count)". Followed EXACTLY:
//   * raw-Ptr wrappers, NO ForeignPtr/Object machinery (the row's own
//     words; contrast the ForeignPtr rows for Bytecode/FactSet);
//   * RAII: `Drop` calls the two free symbols; with_rts_set /
//     with_word_rts_set are the source brackets (Set.hs:34-35, :49-50);
//     the Haskell's newRtsSet/freeRtsSet (and Word twins) are unexported
//     and stay private here (they survive as the `new_*` producers +
//     Drop);
//   * raw `extern "C"` per the row's own in-place A2 annotation — the
//     former "cxx bridge" wording is superseded by RULEBOOK §1 (FFI row,
//     A2: raw extern "C" for flat C-ABI symbols). The operative
//     cxx-supersession is recorded OUTSIDE the inventory too:
//     DECISIONS.md 2026-10-07 item 4 annotates rows 38/60/79/61 in place
//     "no code changes required (all four units already shipped
//     extern-C)", and builder.rs's FAMILY-wide row-61 flag names the same
//     four rows for this same builder handle. Hence NO
//     INVENTORY-BUG-CANDIDATE here (unlike the sibling builder.rs's
//     row-61 note, written against the pre-annotation text);
//   * the row's TODO(port) is carried at the three build functions (the
//     points that fix element order), left OPEN — the judge case is
//     unrun.
//
// GUARD CLASSIFICATION, told as ONE story with the sibling units
// (builder.rs's allocation_guard_abort posture and its inventory-row-61
// ruling; foreign/bytecode.rs for the same wrap.h kit, on disk at
// ~/glean-build/hsthrift/common/util/cpp/wrap.h): every fallible symbol
// of this unit is ALLOCATION-GUARD by construction — the ten bodies at
// ffi.cpp:585-643 contain no precondition checks at all, and their only
// failure modes are allocation-class (`new BytestringSet`/`new WordSet`,
// `std::set::insert` node allocation, `Output::packed`/`Output::bytes`
// buffer growth, `moveToFbString`'s terminator alloc). wrap.h:21-32
// turns a caught std::exception — bad_alloc included — into a returned
// const char* (strdup'd, or the static outOfMemory when strdup itself
// fails), so an allocation failure can arrive at a call site as an Err
// value; whether it aborts inside C++ or crosses as a value is
// immaterial to the response — the family's canonical ruling (inventory
// row 61, which builder.rs cites) is ABORT on any Err, reproduced here
// by this file's local allocation_guard_abort. The two free bodies route
// through ffi::free_ (wrap.h:42-45 -> wrap_ :34-40, which swallows every
// exception), so Drop is infallible — the same disposition as the
// builder family's freeBuilder.
//
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, read): outbound — row
// 1289 (Set.hs -> Glean/RTS/Builder.hs) and nothing else in-tree; plus
// the external hsthrift `Util.FFI` import (Set.hs:27), whose Rust
// landing place is crate::ffi (inventory row 122 — this module never
// re-implements the CString protocol). The module imports nothing else
// (Set.hs:22-25: Control.Monad.Catch, Control.Monad.IO.Class, Foreign,
// Foreign.C). Inbound — rows 729 (db/Glean/Write/JSON.hs, call sites
// :319-347) and 1829 (typed/Glean/Typed/Binary.hs, call sites :196-224);
// both were read and are cited at the functions below.
//
// SIBLING-UNIT CONTRACT (RULE §4; DONE units' real APIs read, never
// re-declared):
//   * crate::ffi (ffi.rs) — the pinned `pub unsafe fn invoke(call: impl
//     FnOnce() -> *const c_char) -> Result<(), FfiError>` and `FfiError`
//     (shipped; its header pins the wrap.h channel). The Haskell's
//     invoke shapes used here (one out-param for the two `new`s, none
//     for the rest) collapse to stack locals plus one check per call
//     site — the builder.rs precedent.
//   * crate::rts::builder (builder.rs) — `Builder` with a private handle
//     and `as_ptr() -> *mut sys::Output` (shipped). Its SIBLING-UNIT
//     CONTRACT names this unit explicitly: glean_rtsset_insert / *_build
//     "must obtain the pointer via as_ptr and must not dereference or
//     free it". This file does exactly that — the raw pointer is only
//     ever passed straight back into the C++ calls below. `Output`
//     itself is NOT re-declared: mod sys imports
//     crate::rts::builder::sys::Output (builder.rs keeps its `sys`
//     module pub precisely so builder-carrying sibling externs can name
//     that one type).
//
// Borrow discipline (documented, not incidental): a call that changes the
// object's contents takes the exclusive borrow — the file's own rule, the
// reset_builder discipline builder.rs names for the same handle. The
// inserts MUTATE the C++ set, so they take `&mut RtsSet` /
// `&mut WordRtsSet`; insert_builder also takes `&mut Builder` (it MOVES
// the Output's contents out, "contents mutate, identity survives"); and
// the build_* calls take `&mut Builder` because they append into that
// same Output (packed size + element bytes) — the content-mutating sibling
// writers are all `&mut` on this very handle (rts.rs's with_value /
// encode_value / encode_byte_array, builder.rs's reset_builder). The
// in-tree call shapes hold a `&mut Builder` and reborrow it at the call
// (Write/JSON.hs:319-347, Typed/Binary.hs:216-224), so nothing changes at
// the call sites; the sets themselves are only READ by the builds.

//! C++ RTS dedup-set handles (`BytestringSet`/`WordSet`) — create,
//! insert, build into a Builder, free.
//!
//! Port notes: RULE §3 — `unsafe` appears only inside this glean-core FFI
//! wrapper, where the Haskell was already FFI; every block carries a
//! SAFETY comment. Error-recovery classification per RULE §2
//! (error-recovery rule), aligned with crate::ffi::invoke's channel note
//! and the sibling units so the family tells ONE story (see the GUARD
//! CLASSIFICATION block above): every Err at every site here is
//! allocation-class and aborts via allocation_guard_abort.

use std::ptr;

use crate::ffi::{invoke, FfiError};
use crate::rts::builder::Builder;

/// Raw C-ABI declarations for the glean_rtsset_* / glean_wordrtsset_*
/// groups (glean/rts/ffi.cpp:585-643 — see the ABI-TRUTH NOTE above for
/// why ffi.h is not the citation here).
///
/// RULE §1 (FFI row, A2): raw `extern "C"` is the default for flat C-ABI
/// symbols; row 79's pre-A2 "cxx bridge" wording is superseded by the
/// row's own in-place annotation (DECISIONS.md 2026-10-07 item 4).
///
/// Private: no sibling unit declares externs over these handles (the
/// builder-carrying sibling externs take `Output*`, never `*RtsSet` or
/// `*WordRtsSet` — Set.hs is the only declarer, Set.hs:70-98), so
/// unlike builder.rs::sys there is no cross-unit consumer to expose.
/// The Haskell's `Storable` instances on the two pointer newtypes
/// (Set.hs:32, :47) had no use beyond the out-param marshalling that
/// Util.FFI's invoke performed generically; the Rust collapse below
/// needs no equivalent.
mod sys {
    use std::os::raw::c_char;

    use crate::rts::builder::sys::Output;

    /// Opaque C++ `facebook::glean::rts::BytestringSet` (set.h:28; the
    /// Haskell calls the handle `RtsSet`, Set.hs:31). Never constructed,
    /// dereferenced or freed on the Rust side except through the
    /// functions below.
    #[repr(C)]
    pub struct BytestringSet {
        _private: [u8; 0],
    }

    /// Opaque C++ `facebook::glean::rts::WordSet` (set.h:30; the Haskell
    /// handle is `WordRtsSet`, Set.hs:46).
    #[repr(C)]
    pub struct WordSet {
        _private: [u8; 0],
    }

    // NOTE (RULE §3, family convention): the `unsafe extern "C"` block
    // below is a DECLARATION, not an operation — the same ruling rts.rs
    // states for its own `pub mod sys` block, and the same unmarked
    // practice bytecode.rs:168 and ffi.rs:326 follow for theirs. It
    // introduces no unsafe behaviour of its own, carries no SAFETY
    // comment, and is NOT counted in the SAFETY tally at the foot of this
    // file; every USE below carries its own.
    unsafe extern "C" {
        // ffi.cpp:585-587 — `BytestringSet**` out-param; error slot is
        // allocation-guard (`new BytestringSet`, :586).
        pub fn glean_rtsset_new(set: *mut *mut BytestringSet) -> *const c_char;
        // ffi.cpp:589-591 — MOVES `out->moveToFbString()` into the set
        // (:590); allocation-class failure only.
        pub fn glean_rtsset_insert(set: *mut BytestringSet, out: *mut Output) -> *const c_char;
        // ffi.cpp:593-600 — writes packed size + raw element bytes into
        // `out`; allocation-class only (Output growth).
        pub fn glean_rtsset_build(set: *mut BytestringSet, out: *mut Output) -> *const c_char;
        // ffi.cpp:602-604 — ffi::free_ (infallible; wrap.h:42-45).
        pub fn glean_rtsset_free(set: *mut BytestringSet);

        // ffi.cpp:606-608 — `WordSet**` out-param (`new WordSet`, :607).
        pub fn glean_wordrtsset_new(set: *mut *mut WordSet) -> *const c_char;
        // ffi.cpp:610-612 — uint64_t element (C++ definition is the ABI
        // truth; the Haskell import's CULong is 64-bit on every LP64
        // target — Set.hs:85-86). Allocation-class only (set node).
        pub fn glean_wordrtsset_insert(set: *mut WordSet, value: u64) -> *const c_char;
        // ffi.cpp:614-621 — one uint64 element per input BYTE (:618);
        // `size_t` -> usize per RULE §2 (A5 row). Allocation-class only.
        pub fn glean_wordrtsset_insert_bytes(
            set: *mut WordSet,
            data: *const u8,
            size: usize,
        ) -> *const c_char;
        // ffi.cpp:623-630 — packed(size) then packed(element) each.
        pub fn glean_wordrtsset_build(set: *mut WordSet, out: *mut Output) -> *const c_char;
        // ffi.cpp:632-639 — packed(size) then fixed<uint8_t>(element)
        // each (LOW byte; see build_word_set_bytes below).
        pub fn glean_wordrtsset_build_bytes(set: *mut WordSet, out: *mut Output) -> *const c_char;
        // ffi.cpp:641-643 — ffi::free_ (infallible).
        pub fn glean_wordrtsset_free(set: *mut WordSet);
    }
}

/// `newtype RtsSet = RtsSet (Ptr RtsSet) deriving (Storable)` (Set.hs:31-32)
/// — a pointer handle to the C++ `BytestringSet` (set.h:28). Inventory
/// row 79: "raw-Ptr wrappers (not ForeignPtr)".
///
/// Ownership: allocated by the private `new_rts_set` (the source's
/// `newRtsSet`, Set.hs:37-38) and released exactly once by `Drop` (the
/// source's `freeRtsSet`, Set.hs:79-80). Not `Copy`, `Clone`, `Send` or
/// `Sync` (conservative, matching builder.rs): the C++ set is
/// unsynchronised mutable state, and the source only ever uses one
/// inside its `withRtsSet` bracket on one thread — no call site in the
/// tree moves one across threads (edges.tsv inbound rows 729/1829 only).
///
/// `#[repr(transparent)]` keeps the by-value C marshalling story of the
/// Storable newtype available: the Rust value IS the `BytestringSet*`.
#[repr(transparent)]
pub struct RtsSet(*mut sys::BytestringSet);

impl Drop for RtsSet {
    fn drop(&mut self) {
        // SAFETY: self.0 is non-null and originates from a successful
        // glean_rtsset_new (the only construction site — the field is
        // private and no other constructor exists), so it is a live
        // handle the C++ side owns. RtsSet is neither Copy nor Clone, so
        // this runs exactly once per handle and cannot double-free. The
        // body (ffi.cpp:602-604) is ffi::free_, whose wrap_ swallows
        // every exception (wrap.h:34-45) — nothing can unwind across.
        unsafe { sys::glean_rtsset_free(self.0) };
    }
}

/// RULE §2 (error-recovery rule): "allocation-failure ≠ structural error.
/// Every FFI null return in the inventory records which kind:
/// allocation-guard (abort) or precondition-guard (record + partial
/// result)." Every site below states its kind; all ten bodies at
/// ffi.cpp:585-643 can only fail to allocate (there are no precondition
/// checks), and the family's canonical ruling (inventory row 61,
/// followed by the sibling builder.rs) is abort-on-Err for this handle
/// family. Same shape as builder.rs / foreign/bytecode.rs: one local
/// copy per file — the house pattern (NOT a shared re-export).
/// abort() rather than panic! so that no unwinding runs first.
fn allocation_guard_abort(e: FfiError) -> ! {
    eprintln!("glean: C++ RTS set allocation failure: {}", e);
    std::process::abort()
}

/// Haskell (Set.hs:37-38): `newRtsSet = FFI.invoke glean_rtsset_new` —
/// the one-out-param invoke arity (Util.FFI allocas the `Ptr RtsSet` slot
/// and peeks it after the call); in Rust the arity collapses to the stack
/// local below plus one check (builder.rs precedent). Not exported by the
/// source module; private here too.
fn new_rts_set() -> RtsSet {
    let mut out: *mut sys::BytestringSet = ptr::null_mut();
    // SAFETY: `out` is a valid, initialised stack out-param for the
    // duration of the call. On the success path the C++ side stores a
    // freshly `new`-allocated BytestringSet (ffi.cpp:586); the null
    // initialiser means a failed call cannot smuggle a dangling handle
    // into RtsSet. invoke takes the extern call itself as its `call`
    // closure; the returned pointer follows crate::ffi::invoke's contract
    // (null on success, else a message it reads and frees via
    // hs_ffi_free_error).
    if let Err(e) = unsafe { invoke(|| sys::glean_rtsset_new(&mut out)) } {
        // `new BytestringSet` (ffi.cpp:586) is allocation-class; abort per
        // the guard story above.
        allocation_guard_abort(e);
    }
    debug_assert!(
        !out.is_null(),
        "glean_rtsset_new returned success with a null set"
    );
    RtsSet(out)
}

/// Haskell (Set.hs:34-35):
/// `withRtsSet = bracket (liftIO newRtsSet) (liftIO . freeRtsSet)`.
///
/// The `MonadMask` bracket guarantees the free on every exception path;
/// `RtsSet`'s `Drop` is that guarantee here, including panic-unwind out
/// of `f` (Rust has no async exceptions, so the mask-class concerns of
/// crate::ffi::construct's note do not arise). The Haskell's generality
/// over `(MonadMask m, MonadIO m)` collapses to the generic return `R`;
/// the closure gets `&mut RtsSet` for the duration of the bracket
/// (exclusive access — the with_builder shape, and what the `&mut`-taking
/// inserts below compose with).
pub fn with_rts_set<R>(f: impl FnOnce(&mut RtsSet) -> R) -> R {
    let mut set = new_rts_set();
    f(&mut set)
}

/// Haskell (Set.hs:40-41):
/// `insertBuilder set builder = FFI.invoke $ glean_rtsset_insert set builder`.
///
/// The C body is `set->insert(out->moveToFbString())` (ffi.cpp:590), and
/// `moveToFbString` (binary.cpp:57-71) MOVES the builder's contents out:
/// the malloced branch NUL-terminates, hands the heap block to the
/// fbstring and `markEmpty()`s the Output (:58-67); the small/RefMem
/// branch COPIES (:68-70) and leaves the Output intact. The source's
/// usage pattern resets the builder after every insert — `insertBuilder
/// tb; resetBuilder tb` in a loop (Write/JSON.hs:344-346,
/// Typed/Binary.hs:221-223) — so both branches discharge identically at
/// the pattern level; the move is nevertheless real and takes `&mut
/// Builder` (the reset_builder precedent: "contents mutate, identity
/// survives") plus `&mut RtsSet` (insertion mutates the set).
///
/// Guard kind: ALLOCATION-GUARD -> abort (std::set::insert node +
/// moveToFbString's allocations; no precondition checks in :589-591).
pub fn insert_builder(set: &mut RtsSet, builder: &mut Builder) {
    // SAFETY: `set.0` is a live non-null handle (RtsSet's invariant);
    // `builder` is a live handle, and as_ptr only borrows it — the C++
    // call moves the Output's CONTENTS, never the Output object itself,
    // so the handle stays valid and owned by the Builder, exactly per
    // builder.rs's SIBLING-UNIT CONTRACT ("must not dereference or free
    // it"). invoke takes the extern call itself as its `call` closure;
    // the returned pointer follows crate::ffi::invoke's contract.
    if let Err(e) = unsafe { invoke(|| sys::glean_rtsset_insert(set.0, builder.as_ptr())) } {
        // Allocation-class only (ffi.cpp:590); abort per the guard story.
        allocation_guard_abort(e);
    }
}

/// Haskell (Set.hs:43-44):
/// `buildSet set builder = FFI.invoke $ glean_rtsset_build set builder`.
///
/// Append-only: the C body writes `out->packed(set->size())` then each
/// element's raw bytes (`out->bytes(v.data(), v.size())`,
/// ffi.cpp:593-600); the SET is only read (`&RtsSet`). The call does
/// change the Builder's contents, so it takes the exclusive `&mut
/// Builder` — the file's own rule above, the same posture as the
/// content-mutating sibling writers on this handle (rts.rs's with_value /
/// encode_value / encode_byte_array; builder.rs's reset_builder).
///
/// Guard kind: ALLOCATION-GUARD -> abort (Output::packed/bytes growth
/// only).
// TODO(port): row 79's open question — confirm that dedup via
// BytestringSet, then buildSet, reproduces the fid assignment order of a
// batch defined DIRECTLY through the Builder, so a rebuilt batch
// produces byte-identical ids (judge case: same facts via RtsSet vs
// direct Builder define -> same first_id/count). Source evidence while
// this stays OPEN: the order these bodies emit is deterministic —
// BytestringSet is std::set with CmpSet = std::lexicographical_compare
// over the fbstring's bytes (set.h:20-28), so iteration is sorted and
// deduplicated — but the batch-level id question is out of this file's
// scope and the judge case has NOT run.
pub fn build_set(set: &RtsSet, builder: &mut Builder) {
    // SAFETY: `set.0` is a live non-null handle; `builder` is live and
    // as_ptr only borrows it (this call appends to the buffer and never
    // frees or replaces the Output). invoke takes the extern call itself
    // as its `call` closure; the returned pointer follows
    // crate::ffi::invoke's contract.
    if let Err(e) = unsafe { invoke(|| sys::glean_rtsset_build(set.0, builder.as_ptr())) } {
        // Allocation-class only (ffi.cpp:594-599); abort per the guard
        // story.
        allocation_guard_abort(e);
    }
}

/// `newtype WordRtsSet = WordRtsSet (Ptr WordRtsSet) deriving (Storable)`
/// (Set.hs:46-47) — a pointer handle to the C++ `WordSet` (set.h:30).
/// Same ownership story as [`RtsSet`] (Drop == the source's
/// `freeWordRtsSet`, Set.hs:97-98; `#[repr(transparent)]`; no
/// Copy/Clone/Send/Sync — conservative, matching builder.rs).
#[repr(transparent)]
pub struct WordRtsSet(*mut sys::WordSet);

impl Drop for WordRtsSet {
    fn drop(&mut self) {
        // SAFETY: self.0 is non-null and originates from a successful
        // glean_wordrtsset_new (the only construction site — private
        // field, no other constructor), so it is a live handle the C++
        // side owns. WordRtsSet is neither Copy nor Clone, so this runs
        // exactly once. The body (ffi.cpp:641-643) is ffi::free_
        // (wrap.h:34-45 swallows every exception) — nothing unwinds
        // across.
        unsafe { sys::glean_wordrtsset_free(self.0) };
    }
}

/// Haskell (Set.hs:52-53):
/// `newWordRtsSet = FFI.invoke glean_wordrtsset_new` — same collapse as
/// [`new_rts_set`]; unexported by the source module, private here.
fn new_word_rts_set() -> WordRtsSet {
    let mut out: *mut sys::WordSet = ptr::null_mut();
    // SAFETY: `out` is a valid, initialised stack out-param for the
    // duration of the call; on success the C++ side stores a freshly
    // `new`-allocated WordSet (ffi.cpp:607), and the null initialiser
    // means a failed call cannot smuggle a dangling handle into
    // WordRtsSet. invoke takes the extern call itself as its `call`
    // closure; the returned pointer follows crate::ffi::invoke's
    // contract.
    if let Err(e) = unsafe { invoke(|| sys::glean_wordrtsset_new(&mut out)) } {
        // `new WordSet` (ffi.cpp:607) is allocation-class; abort per the
        // guard story above.
        allocation_guard_abort(e);
    }
    debug_assert!(
        !out.is_null(),
        "glean_wordrtsset_new returned success with a null set"
    );
    WordRtsSet(out)
}

/// Haskell (Set.hs:49-50):
/// `withWordRtsSet = bracket (liftIO newWordRtsSet) (liftIO . freeWordRtsSet)`.
/// The RAII twin of [`with_rts_set`]; in-tree brackets: Write/JSON.hs:319-322,
/// :327-329, :332-337 and Typed/Binary.hs:196-199, :207-210.
pub fn with_word_rts_set<R>(f: impl FnOnce(&mut WordRtsSet) -> R) -> R {
    let mut set = new_word_rts_set();
    f(&mut set)
}

/// Haskell (Set.hs:55-57):
///
/// * insertWordRtsSet :: Integral n => WordRtsSet -> n -> IO ()
/// * insertWordRtsSet set elem =
/// *   FFI.invoke $ glean_wordrtsset_insert set (fromIntegral elem)
///
/// The C ABI takes uint64_t (ffi.cpp:610; the Haskell's CULong, Set.hs:85-86,
/// is 64-bit on every LP64 target — same class of .hs-vs-C type as
/// bytecode.rs documents for size_t). The Rust surface takes `elem: u64`
/// directly (conservative per the unit brief; RULE §2 numeric-conversions
/// row: the call site reproduces the source's fromIntegral wrap with an
/// explicit cast). In-tree callers, all widening/wrapping signed payloads:
///   * Write/JSON.hs:335-336 passes `J.Int n` — an Int64
///     (Foreign/JSON.hs:88) with no sign check in the source — so
///     `n as u64` at the call site is Haskell's fromIntegral two's-
///     complement wrap (e.g. -1 -> 2^64-1).
///   * Typed/Binary.hs:198/:209 pass `unByte x` / `unNat x` — payloads of
///     the thrift types `typedef byte Byte` / `typedef i64 Nat`
///     (glean/if/glean.thrift:97, :100; Int8/Int64 images under
///     hs.newtype), so `as u64` (sign-extending) is the same wrap.
///
/// Guard kind: ALLOCATION-GUARD -> abort (std::set::insert node; no
/// precondition checks in :610-612).
pub fn insert_word_rts_set(set: &mut WordRtsSet, elem: u64) {
    // SAFETY: `set.0` is a live non-null handle (WordRtsSet's invariant).
    // invoke takes the extern call itself as its `call` closure; the
    // returned pointer follows crate::ffi::invoke's contract.
    if let Err(e) = unsafe { invoke(|| sys::glean_wordrtsset_insert(set.0, elem)) } {
        // Allocation-class only (ffi.cpp:611); abort per the guard story.
        allocation_guard_abort(e);
    }
}

/// Haskell (Set.hs:59-61):
///
/// * insertBytesRtsSet :: Integral n => WordRtsSet -> Ptr Word8 -> n -> IO ()
/// * insertBytesRtsSet set bytes size =
/// *   FFI.invoke $ glean_wordrtsset_insert_bytes set bytes (fromIntegral size)
///
/// The C body inserts one uint64 element per input BYTE
/// (`set->insert(data[i])`, ffi.cpp:617-619) — preserved exactly by
/// calling the C++ function itself, not re-encoding.
///
/// Surface choice (documented per the unit brief): SAFE `bytes: &[u8]`,
/// not a raw pointer plus a separately stored size. The C body reads
/// EXACTLY `size` bytes from `data` (ffi.cpp:615-621:
/// `for (auto i = 0; i < size; i++)`), so with a `&[u8]` argument the
/// SLICE IS the bound — there is no unbounded read to launder and RULE
/// §2's A7 row does not mandate `unsafe` here. `&mut WordRtsSet` matches
/// [`insert_word_rts_set`] (insertion mutates the set). The two in-tree
/// call shapes hand over the pointer/length halves of a LIVE buffer,
/// which is precisely a slice, so they pass through unchanged:
///   * Write/JSON.hs:319-322: `insertBytesRtsSet rtsset p n` where
///     `J.String (J.ByteStringRef p n)` destructures a `ByteStringRef`
///     { Ptr Word8, CSize } (RTS.hs:105-108) — a borrow of a live string,
///     so `&buf[..n]` (the slice's own length) is the faithful image.
///   * Write/JSON.hs:327-329: `insertBytesRtsSet rtsset (castPtr ptr) len`
///     where (ptr, len) come from unsafeWithBytes (Glean/FFI.hs:35-37;
///     len :: CSize), after the base64 decode — the same slice.
/// In both, pointer and length come from ONE live borrow (never a
/// separately stored CSize), so the slice is the lossless image and no
/// numeric conversion is needed (RULE §2's A5 row: no signed cast).
///
/// RULE §3: `unsafe` stays confined to this glean-core FFI wrapper — the
/// extern call below is the only unsafe operation, in its own block.
///
/// Guard kind: ALLOCATION-GUARD -> abort (one std::set::insert node per
/// byte; no precondition checks in :614-621). `size == 0` inserts nothing
/// (the C loop body never executes), matching the Haskell where the call
/// never touched the pointer either.
pub fn insert_bytes_rts_set(set: &mut WordRtsSet, bytes: &[u8]) {
    // SAFETY: `set.0` is a live non-null handle (WordRtsSet's invariant);
    // `bytes.as_ptr()`/`bytes.len()` hand the C++ the same (pointer, size)
    // pair the Haskell did, and the C loop reads EXACTLY that many bytes
    // from that slice (ffi.cpp:617-618) — an empty slice reads nothing.
    // invoke takes the extern call itself as its `call` closure; the
    // returned pointer follows crate::ffi::invoke's contract.
    if let Err(e) = unsafe {
        invoke(|| sys::glean_wordrtsset_insert_bytes(set.0, bytes.as_ptr(), bytes.len()))
    } {
        // Allocation-class only (ffi.cpp:617-619 inserts); abort per the
        // guard story.
        allocation_guard_abort(e);
    }
}

/// Haskell (Set.hs:63-64):
/// `buildWordSet set builder = FFI.invoke $ glean_wordrtsset_build set builder`.
///
/// Append-only: the C body writes `out->packed(set->size())` then
/// `out->packed(v)` for each element (ffi.cpp:623-630); the set is only
/// read. The call does change the Builder's contents, so it takes
/// `&mut Builder` — same posture as [`build_set`].
///
/// Guard kind: ALLOCATION-GUARD -> abort (Output growth only).
// TODO(port): the row-79 open question recorded at build_set applies
// identically here — confirm that WordSet dedup + buildWordSet
// reproduces the fid assignment order of a batch defined directly
// through the Builder (judge case: same facts via RtsSet vs direct
// Builder define -> same first_id/count). Source evidence while OPEN:
// WordSet is std::set<uint64_t> (ascending, deduplicated — set.h:30), so
// this body's iteration order is deterministic; the batch-level id
// question is out of this file's scope and the judge case has NOT run.
pub fn build_word_set(set: &WordRtsSet, builder: &mut Builder) {
    // SAFETY: `set.0` is a live non-null handle; `builder` is live and
    // as_ptr only borrows it (append-only into the buffer). invoke takes
    // the extern call itself as its `call` closure; the returned pointer
    // follows crate::ffi::invoke's contract.
    if let Err(e) = unsafe { invoke(|| sys::glean_wordrtsset_build(set.0, builder.as_ptr())) } {
        // Allocation-class only (ffi.cpp:624-628); abort per the guard
        // story.
        allocation_guard_abort(e);
    }
}

/// Haskell (Set.hs:66-68):
/// `buildWordSetBytes set builder =
/// FFI.invoke $ glean_wordrtsset_build_bytes set builder`.
///
/// Append-only: `out->packed(set->size())` then `out->fixed<uint8_t>(v)`
/// for each element (ffi.cpp:632-639). `fixed<uint8_t>` narrows the
/// uint64_t element to its LOW byte at the call (binary.h:361-366) and
/// writes that one byte — that low-byte write is the INTENDED encoding
/// mechanism for this set, preserved exactly by calling the C++ function.
/// Every element fed by [`insert_bytes_rts_set`] is <= 0xFF, so that path
/// is lossless; the in-tree Byte path feeds `unByte x` (Int8) through
/// [`insert_word_rts_set`] (Typed/Binary.hs:198; Write/JSON.hs:336), so
/// NEGATIVE byte payloads sign-extend above 0xFF BY DESIGN and land on the
/// same low-byte write — truncation there is the intended encoding, not a
/// mixing-only corner, and nothing is lost that the source would not have
/// lost. Set only read; the call changes the Builder's contents, so
/// `&mut Builder` (same posture as [`build_set`]).
///
/// Guard kind: ALLOCATION-GUARD -> abort (Output growth only).
// TODO(port): the row-79 open question recorded at build_set applies
// identically here — confirm that WordSet dedup + buildWordSetBytes
// reproduces the fid assignment order of a batch defined directly
// through the Builder (judge case: same facts via RtsSet vs direct
// Builder define -> same first_id/count). Source evidence while OPEN:
// WordSet is std::set<uint64_t> (ascending, deduplicated — set.h:30), so
// this body's iteration order is deterministic; the batch-level id
// question is out of this file's scope and the judge case has NOT run.
pub fn build_word_set_bytes(set: &WordRtsSet, builder: &mut Builder) {
    // SAFETY: `set.0` is a live non-null handle; `builder` is live and
    // as_ptr only borrows it (append-only into the buffer). invoke takes
    // the extern call itself as its `call` closure; the returned pointer
    // follows crate::ffi::invoke's contract.
    if let Err(e) =
        unsafe { invoke(|| sys::glean_wordrtsset_build_bytes(set.0, builder.as_ptr())) }
    {
        // Allocation-class only (ffi.cpp:633-637); abort per the guard
        // story.
        allocation_guard_abort(e);
    }
}

// Marker breakdown (closed set — 3 open port TODOs (ONE question), no
// source-defect reproductions, no performance deferrals, 10 inline safety
// comments):
//   * 3 open port TODOs — ONE question (inventory row 79), recorded at
//     each of the three build entry points (build_set, build_word_set,
//     build_word_set_bytes) because each independently fixes the element
//     order written into the Builder; the judge case is unrun. No other
//     open TODO remains: row 79's "cxx bridge" wording is superseded in
//     the row itself (A2 annotation; DECISIONS.md 2026-10-07 item 4 —
//     "annotated in-place, no code changes required"), so it is not
//     carried as an open item — and no inventory-bug-candidate flag is
//     needed: item 4 plus builder.rs's FAMILY-wide row-61 flag (which
//     names rows 38/60/79/61 for this same handle) are the canonical
//     cxx-supersession record.
//   * no source-defect reproductions — every source quirk read on this
//     path (moveToFbString's copy-on-small-branch, build_bytes' low-byte
//     truncation, set iteration order) is the behaviour of the C++ we
//     CALL, not code re-implemented on the Rust side — it passes through
//     verbatim, so there is no port divergence to repro.
//   * no performance deferrals — this module performs no copies of its
//     own; the one copy that can happen (small/RefMem moveToFbString)
//     happens in the source-side C++ being reused.
// SAFETY count: 10 unsafe blocks, each with an inline safety comment —
// new_rts_set 1, RtsSet::drop 1, insert_builder 1, build_set 1,
// new_word_rts_set 1, WordRtsSet::drop 1, insert_word_rts_set 1,
// insert_bytes_rts_set 1, build_word_set 1, build_word_set_bytes 1. That
// is the whole tally: the `unsafe extern "C"` block is a DECLARATION (see
// the note above it), and insert_bytes_rts_set takes a `&[u8]` — the
// slice is the C++ loop's bound — so it is a safe fn carrying no
// doc-level safety contract.
// PORT STATUS: confidence=high todos=3
