// Ported from: glean/hs/Glean/RTS/Foreign/LookupCache.hs (188 lines)
// Manifest unit: rust/glean-core/src/rts/foreign/lookup_cache.rs
// (migration/manifest.tsv line 29, crate glean-core). RULE §4: one source
// file -> one target file; the manifest path is authoritative.
//
// ABI truth (read, not assumed): glean/rts/ffi.h —
//   glean_lookupcache_stats_new               :149      (SharedLookupCacheStats** out; const char* slot)
//   glean_lookupcache_stats_free              :150      (void)
//   glean_lookupcache_stats_read_and_reset_counters
//                                             :151-154  (void; uint64_t* values, size_t size — NO error slot)
//   glean_lookupcache_new                     :156-160  (size_t capacity, size_t shards, stats, LookupCache** out)
//   glean_lookupcache_free                    :161      (void)
//   glean_lookupcache_clear                   :162      (const char*)
//   glean_lookupcache_anchor_new              :164-168  (Lookup* base, LookupCache*, int replacementPolicy, Lookup** out)
//   glean_lookupcache_anchor_free             :169      (void)
// `SharedLookupCacheStats` is an opaque `typedef struct` (ffi.h:85).
// C++ bodies (glean/rts/ffi.cpp): :161-165 (stats_new —
// `new SharedLookupCacheStats{make_shared<LookupCache::Stats>()}`; the
// wrapper holds the inner Stats by shared_ptr, which every cache copies,
// so the caches' statistics outlive the wrapper handle), :167-169
// (stats_free), :171-183 (read_and_reset — copy only when
// `size == buffer.size()`, else LOG(ERROR) + zero-fill), :185-194 (new —
// `new LookupCache(Options{capacity, shards}, stats->value)`), :196-198
// (free), :200-202 (clear), :204-215 (anchor_new —
// `new LookupCache::Anchor(cache->anchor(base,
// static_cast<ReplacementPolicy>(replacementPolicy)))`), :217-219
// (anchor_free). The value truth is the C++ side's own headers:
//   * glean/rts/cache.h:125-128 — Anchor::ReplacementPolicy `enum class`
//     { LRU = 0, FIFO = 1 } (the FFI int is that enumerator value);
//   * glean/rts/cache.h:68-89 — Stats::Stat `enum : size_t`,
//     `values[COUNT]`: FIRST_COUNTER = 0 (:69/:71),
//     idByKey_hits .. factById_deletes = 0..10, FIRST_NON_COUNTER = 11
//     (:83), factBytes = 11, factCount = 12 (:85-86), COUNT = 13 (:88);
//   * glean/rts/cache.h:93-98 + glean/rts/cache.cpp:25-34 —
//     readAndResetCounters: [0, FIRST_NON_COUNTER) is read-and-RESET,
//     [FIRST_NON_COUNTER, COUNT) is read only;
//   * glean/rts/cache.h:120-121 + cache.cpp:102-111 — clear() is a pure
//     stats subtraction (the "rebuild the cache" TODO is the source's own);
//   * glean/rts/cache.h:35-37 — "The cache can be used concurrently by
//     multiple threads"; folly::ThreadCachedInt counters (cache.h:91) say
//     the same for Stats (the basis of the Send/Sync impls below).
//
// INVENTORY (RULE §5) — row 70 (migration/inventory.tsv line 70;
// duplicated as inventory.ffi.tsv line 17), quoted and followed:
//   "owned LookupCache + Stats shared across caches; ReplacementPolicy/
//   LAST_COUNTER enums mirror C++ rts/cache.h; read_and_reset writes
//   caller's vector | OWNED_MUTABLE | cxx bridge [SUPERSEDED by §1/A2:
//   raw extern-C — 2026-10-07]; LookupCache + Stats owned (Drop = free
//   fns); Stats shared across caches -> Arc<StatsHandle>; enum
//   DISCRIMINANTS must match cache.h (kept-in-sync comments at
//   LookupCache.hs:32 and :93) — judge case per variant;
//   readStatsAndResetCounters -> caller-owned [u64; STAT_COUNT] via cxx
//   slice; anchor is a scoped guard around base Lookup".
//   Followed EXACTLY, clause by clause:
//   * raw extern "C" (RULE §1/A2): the row's cxx-bridge wording carries
//     the inline supersession ("[SUPERSEDED by §1/A2: raw extern-C —
//     2026-10-07]") in row 70; the eight symbols are flat `glean_*`
//     functions over opaque handles, so this file declares no cxx bridge
//     (see the INVENTORY-BUG-CANDIDATE below for the two phrases that
//     still lack the annotation).
//   * owned handles: `Stats` and `LookupCache` are owning structs whose
//     Drop runs the source's `destroy` (stats_free / free;
//     LookupCache.hs:43/:74). Private raw pointer, neither Clone nor Copy
//     (the Haskell shares a ForeignPtr via `with`, never copies by value).
//   * the sharing decision (the row's "Stats shared across caches ->
//     Arc<StatsHandle>"): `Stats` gets Send + Sync (SAFETY at the impls),
//     so consumers share ONE handle exactly as the Haskell shares one
//     `ForeignPtr Stats` — the natural wrap is `Arc<Stats>` (Env.hs:122
//     creates one stats for the env; Open.hs:648-651 hands it to the
//     writer's cache; Close.hs:141 + Open.hs:501-504 read it). This
//     module keeps `&Stats` in its API, so the Arc stays consumer-side —
//     no global, no clone, no second handle kind.
//   * discriminants: ReplacementPolicy is #[repr(i32)] Lru=0/Fifo=1 (the
//     Haskell `fromEnum` -> C int); Stat is #[repr(u32)] in cache.h order
//     (0..12; STAT_COUNT = 13; LAST_COUNTER = FactByIdDeletes = 10). Both
//     carry the kept-in-sync comments (LookupCache.hs:32/:93) citing
//     cache.h.
//   * read_stats_and_reset_counters: caller-owned 13-slot buffer (the
//     ffi.cpp size check), returns StatValues; the symbol is void, so no
//     Result. (The clause's "via cxx slice" wording lacks row 70's inline
//     supersession annotation; followed per §1/A2 — flagged below.)
//   * anchor: `with_cache` brackets anchor_new .. anchor_free around the
//     base Lookup with an AnchorGuard (the sibling LookupGuard pattern),
//     Lookup named "anchor:<base name>" and freed on every exit path.
//   The row's "judge case per variant" is a verification task against
//   cache.h (the discriminants are pinned by value here and documented on
//   both sides) — noted in the marker breakdown; one
//   INVENTORY-BUG-CANDIDATE is raised below.
//
// INVENTORY-BUG-CANDIDATE row 70 / inventory.ffi.tsv row 17 (the same
// file and symbols, duplicated): the row's "cxx bridge" clause carries
// the A2 supersession inline in inventory.tsv row 70, but TWO pre-A2
// "cxx" phrasings remain unannotated: row 70's readStatsAndResetCounters
// clause ("caller-owned [u64; STAT_COUNT] via cxx slice") and the whole
// ffi-shard duplicate (inventory.ffi.tsv row 17 reads "cxx bridge; ..."
// and "via cxx slice" with no supersession note). This file follows
// RULE §1/A2 — raw `extern "C"` over the flat `glean_*` symbols — which
// is what row 70's annotated clause already rules; the unannotated
// residue is flagged here (the same class bytecode.rs raised for its
// pre-A2 cxx phrasings) so the inventory text can be brought in line.
// No other divergence: the rows' SUBSTANTIVE mandates (owned handles,
// Arc-able sharing, discriminants, caller-owned buffer, anchor guard)
// are followed exactly.
//
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, module-level): outbound —
// rows 1265 (LookupCache.hs -> Glean/FFI.hs) and 1266 (LookupCache.hs ->
// RTS/Foreign/Lookup.hs), plus the external hsthrift Util.FFI kit
// (LookupCache.hs:27, imported as `Util.FFI`) whose Rust landing place is
// crate::ffi (inventory row 122). Inbound — rows 190
// (db/Glean/Database/Close.hs:140 LookupCache.clear), 278 (.../Env.hs:122
// newStats), 332 (.../Open.hs:501-504 read stats + :648-651 new),
// 458 (.../Types.hs:86-87/:274 the Writing + Env fields), 488
// (.../Write/Batch.hs:325 withCache FIFO + :192/:275), 1250
// (RTS/Foreign/FactSet.hs:106/:196 rebase takes the cache), 1306
// (Glean/Write/Stats.hs:126-137 lookupCacheStats/getStat/isCounter + the
// `show c` metric names) and 1878 (write/SendAndRebaseQueue.hs:492-493
// readStatsAndResetCounters + countFailuresAsMisses). This module imports
// nothing from a sibling Rust unit beyond crate::ffi and
// crate::rts::foreign::lookup.
//
// SIBLING-UNIT CONTRACTS (RULE §4; DONE units' real APIs read, never
// re-declared):
//   * crate::ffi (read): `invoke` (the pinned
//     `pub unsafe fn invoke(call: impl FnOnce() -> *const c_char)
//     -> Result<(), FfiError>` — the extern call passed AS the closure)
//     and `FfiError`.
//   * crate::rts::foreign::lookup (unit 18, read): `CanLookup`
//     (with_lookup + lookup_name) and `Lookup` — its `Lookup::new` is
//     ALREADY pub (lookup.rs:69), so the anchor wraps it without a new
//     constructor; LookupGuard (lookup.rs:152-163) is the guard precedent
//     AnchorGuard follows. One signature note: lookup.rs's
//     `with_snapshot`/`with_empty_lookup` and this file's `with_cache` all
//     state `E: From<FfiError>` for the `invoke(...)?` merge (added to
//     lookup.rs in the units 28–31 fix round; this file already had it —
//     semantics unchanged).
//   * Why `pub mod sys` is public — a sibling declares its own externs
//     over the SAME opaque `LookupCache*`: `glean_factset_rebase`'s
//     `LookupCache* cache` (ffi.h:309, RTS/Foreign/FactSet.hs:106/:196),
//     which obtains the pointer via `LookupCache::with_ptr` (never a
//     retained raw pointer — the crate::ffi::with / ForeignPtr::with_raw
//     borrow contract).

//! C++ RTS lookup-cache handle (`LookupCache`) and its shared statistics
//! (`SharedLookupCacheStats`, wrapping `LookupCache::Stats`) — create,
//! clear, snapshot/reset, and the scoped [`with_cache`] anchor around a
//! base [`Lookup`].
//!
//! Port notes: RULE §3 — `unsafe` appears only inside this glean-core FFI
//! wrapper (the Haskell was already FFI); every block carries a SAFETY
//! comment. RULE §2 error-recovery classification, per symbol, from the
//! C++ bodies:
//!   * stats_new / lookupcache_new / anchor_new (the three allocating
//!     symbols): ALLOCATION-GUARD only — each body is a plain `new`
//!     (ffi.cpp:161-165 / :185-194 / :204-215) with no precondition check
//!     that raises; the wrap.h message is returned as `Err` (the source's
//!     exception made explicit — a caller may legitimately handle OOM,
//!     e.g. Open.hs:648's cache construction).
//!   * lookupcache_clear: the channel exists but the body is a pure stats
//!     subtraction (ffi.cpp:200-202 + cache.cpp:102-111) — no failure mode;
//!     still returned as `Err` for channel parity.
//!   * stats_free / lookupcache_free / anchor_free: void, `ffi::free_`
//!     (wrap_ swallows every exception) — infallible; used by Drop and
//!     the bracket.
//!   * stats_read_and_reset_counters: VOID — no error channel at all; the
//!     size-mismatch branch is LOG(ERROR) + zero-fill
//!     (ffi.cpp:176-182), and this module always passes exactly the 13
//!     slots the C++ expects, so the branch is unreachable.

use std::fmt;
use std::os::raw::c_void;
use std::ptr;

use crate::ffi::{invoke, FfiError};
use crate::rts::foreign::lookup::{CanLookup, Lookup};

/// Raw C-ABI declarations for the `glean_lookupcache_*` group
/// (glean/rts/ffi.h:149-169; bodies glean/rts/ffi.cpp:161-219).
///
/// RULE §1 (FFI row, A2): raw `extern "C"` for flat C-ABI symbols — the
/// inventory row's "cxx bridge" wording was superseded by the row itself
/// (its bracketed amendment) before this file was written. Signatures
/// mirror ffi.h EXACTLY (`size_t` -> `usize` per RULE §2's A5 row; the
/// opaque `SharedLookupCacheStats*` / `LookupCache*` handles are
/// `*mut c_void`, the convention siblings lookup.rs/subst.rs use; the
/// Haskell's `Int` replacement policy is a C `int` -> `i32`).
pub mod sys {
    use std::os::raw::{c_char, c_void};

    // NOTE (RULE §3, family convention): the `unsafe extern "C"` block
    // below is a DECLARATION, not an operation — the same ruling
    // lookup.rs/subst.rs/bytecode.rs state for theirs. It introduces no
    // unsafe behaviour of its own, carries no SAFETY comment, and is NOT
    // counted in the SAFETY tally at the foot of this file; every USE
    // below carries its own.
    unsafe extern "C" {
        // ffi.h:149 — `SharedLookupCacheStats**` out-param; body
        // ffi.cpp:161-165 is `new SharedLookupCacheStats{shared_ptr(new
        // LookupCache::Stats)}` — ALLOCATION-GUARD only (no precondition
        // check exists to fail).
        pub fn glean_lookupcache_stats_new(stats: *mut *mut c_void) -> *const c_char;
        // ffi.h:150 — void; body ffi.cpp:167-169 is ffi::free_ (wrap_
        // swallows every exception, so nothing unwinds across) —
        // infallible.
        pub fn glean_lookupcache_stats_free(stats: *mut c_void);
        // ffi.h:151-154 — VOID, no error slot. Body ffi.cpp:171-183:
        // copies when `size == buffer.size()` (= Stats::COUNT = 13), else
        // LOG(ERROR) + zero-fills. `values` is caller-owned; the caller
        // must pass exactly 13 and gets the counters reset, the two sums
        // not (cache.cpp:25-34).
        pub fn glean_lookupcache_stats_read_and_reset_counters(
            stats: *mut c_void,
            values: *mut u64,
            size: usize,
        );
        // ffi.h:156-160 — `LookupCache**` out-param; body ffi.cpp:185-194
        // is `new LookupCache(Options{capacity, shards}, stats->value)` —
        // ALLOCATION-GUARD only. `size_t` capacity/shards -> usize (A5).
        pub fn glean_lookupcache_new(
            capacity: usize,
            shards: usize,
            stats: *mut c_void,
            cache: *mut *mut c_void,
        ) -> *const c_char;
        // ffi.h:161 — void; body ffi.cpp:196-198 is ffi::free_ —
        // infallible.
        pub fn glean_lookupcache_free(cache: *mut c_void);
        // ffi.h:162 — body ffi.cpp:200-202 is `cache->clear()`: a pure
        // stats subtraction (cache.cpp:102-111; the "rebuild the cache" TODO
        // in that body is the source's own) — no failure mode; the
        // channel exists but nothing raises.
        pub fn glean_lookupcache_clear(cache: *mut c_void) -> *const c_char;
        // ffi.h:164-168 — `Lookup**` out-param; body ffi.cpp:204-215 is
        // `new LookupCache::Anchor(cache->anchor(base,
        // static_cast<ReplacementPolicy>(replacementPolicy)))` —
        // ALLOCATION-GUARD only. `int` replacementPolicy -> i32, the
        // Haskell's `fromEnum`; an out-of-range int is UB in the
        // static_cast, reachable only by not passing a
        // ReplacementPolicy discriminant.
        pub fn glean_lookupcache_anchor_new(
            base: *mut c_void,
            cache: *mut c_void,
            replacement_policy: i32,
            anchor: *mut *mut c_void,
        ) -> *const c_char;
        // ffi.h:169 — void; body ffi.cpp:217-219 is ffi::free_ over the
        // Anchor that was `new`ed at ffi.cpp:210 (a raw pointer — the
        // C++ body takes ownership at construction) —
        // infallible.
        pub fn glean_lookupcache_anchor_free(anchor: *mut c_void);
    }
}

/// `data ReplacementPolicy = LRU | FIFO` (LookupCache.hs:33-36) — the
/// cache's eviction policy. The Haskell's own note (LookupCache.hs:32):
/// "This must be kept in sync with the ReplacementPolicy in rts/cache.h".
//
// KEPT IN SYNC with glean/rts/cache.h:125-128
// (`enum class ReplacementPolicy { LRU /*= 0*/, FIFO /*= 1*/ }`). The
// discriminants are load-bearing: [`with_cache`] passes `self as i32`
// (the Haskell's `fromEnum`) through to the C++ `static_cast` at
// ffi.cpp:211-213, so 0 must stay LRU and 1 FIFO.
//
// Rust variant names are CamelCase (`Lru`, `Fifo`) where Haskell/src
// spell them upper-case (`LRU`, `FIFO`); no consumer in the tree renders
// this type (the source's derived Show has no call site), so no Display
// is provided and the tag spelling is not observable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash)]
#[repr(i32)]
pub enum ReplacementPolicy {
    /// Evict least-recently-used entries (LookupCache.hs:34; cache.h:126).
    Lru = 0,
    /// Evict the oldest entries. Faster with many concurrent clients
    /// (LookupCache.hs:35; cache.h:127).
    Fifo = 1,
}

/// `newtype LookupCache = LookupCache (ForeignPtr LookupCache)`
/// (LookupCache.hs:38) — an OWNED LRU/FIFO fact cache (cache.h:24-53),
/// "pair[ed] with a base Lookup for looking up facts" via [`with_cache`].
/// Inventory row 70: "OWNED_MUTABLE"; "LookupCache + Stats owned (Drop =
/// free fns)". The C++ object is documented as usable concurrently by
/// multiple threads (cache.h:35-37), hence the Send + Sync impls below.
pub struct LookupCache {
    /// Opaque `LookupCache*` from `glean_lookupcache_new` (the only
    /// construction site; the field is private). Non-null for every value
    /// this module produces; never dereferenced on the Rust side (only
    /// handed back to the C functions in `sys`).
    ptr: *mut c_void,
}

// SAFETY: `LookupCache` is a thin owner of an opaque C++ `LookupCache*`
// (ffi.h:49); the pointer is only ever handed back to the C functions in
// `sys` — as `LookupCache*` or the `base` handle of an Anchor — and the
// C++ object is explicitly built for concurrent use ("The cache can be
// used concurrently by multiple threads", cache.h:35-37; the LRU list and
// hash maps carry their own locks). Moving a handle between threads
// transfers no Rust-side state and the C++ object has no per-thread
// identity — the same shareability GHC's ForeignPtr provides and
// crate::ffi's ForeignPtr Send/Sync impls (ffi.rs:455-463) record.
unsafe impl Send for LookupCache {}
// SAFETY: as Send. `&LookupCache` exposes only `with_ptr` (a borrow-scoped
// pointer view) and the `&self` methods in `sys`; the anchor's C++ side
// keeps its OWN strong reference to the shared inner stats, not to this
// Rust handle, and every use is the C++ object's own thread-safe path.
unsafe impl Sync for LookupCache {}

impl Drop for LookupCache {
    fn drop(&mut self) {
        // SAFETY: `self.ptr` is non-null and originates from a successful
        // `glean_lookupcache_new` (the field is private and `new` is the
        // only constructor), so it is the live uniquely-owned handle the
        // C++ side produced. `LookupCache` is neither Copy nor Clone, so
        // this runs exactly once and cannot double-free; the body
        // (ffi.cpp:196-198) is ffi::free_, whose wrap_ swallows every
        // exception — nothing can unwind across. (An Anchor created over
        // this cache must be gone first: Rust's borrow of the cache in
        // `with_cache` and the C++ anchor lifetime line up, so by Drop
        // time no anchor retains the pointer.)
        unsafe { sys::glean_lookupcache_free(self.ptr) };
    }
}

impl LookupCache {
    /// Borrow the raw `LookupCache*` for the duration of `f` — the Rust
    /// landing of the source's `Object` instance, whose `with`/`unwrap`
    /// is how consumers hand the pointer to their own foreign imports
    /// (RTS/Foreign/FactSet.hs:110 passes it to `glean_factset_rebase`'s
    /// `LookupCache*` param, ffi.h:309).
    ///
    /// Borrow contract (the crate::ffi::with / ForeignPtr::with_raw
    /// shape): `f` must not retain the pointer past the call. The C++
    /// `Anchor` constructor DOES retain it for the anchor's lifetime —
    /// that is sound only because the caller's `&LookupCache` borrow
    /// outlives the anchor, which is exactly how `with_cache` uses it
    /// here and how the Haskell's `with cache $ \p -> bracket anchor ...`
    /// nests.
    pub fn with_ptr<R>(&self, f: impl FnOnce(*mut c_void) -> R) -> R {
        f(self.ptr)
    }
}

/// Guard that frees the C++ `Anchor` on drop — the source's
/// `bracket (invoke $ glean_lookupcache_anchor_new ...)
/// glean_lookupcache_anchor_free` (LookupCache.hs:59-64). Private, like
/// the sibling lookup.rs::LookupGuard (lookup.rs:121-133).
struct AnchorGuard(*mut c_void);

impl Drop for AnchorGuard {
    fn drop(&mut self) {
        // SAFETY: `self.0` is a valid pointer to a C++
        // `LookupCache::Anchor` produced by `glean_lookupcache_anchor_new`
        // and never freed elsewhere (the guard is the only owner);
        // `glean_lookupcache_anchor_free` (ffi.cpp:217-219, `ffi::free_`
        // over the anchor `new`ed at ffi.cpp:210) is the matching
        // free function. The guard is not Copy/Clone, so this runs once.
        unsafe { sys::glean_lookupcache_anchor_free(self.0) };
    }
}

/// `withCache base cache replacement_policy f` (LookupCache.hs:49-65) —
/// "Pair the 'LookupCache' with a base 'Lookup' for looking up facts. It
/// is the user's responsibility to ensure that the 'Lookup's are morally
/// the same throughout the cache's lifetime (such as different instances
/// of the same database)."
///
/// Bracket: `glean_lookupcache_anchor_new` (ffi.cpp:204-215), wrap the
/// anchor in a [`Lookup`] named `"anchor:<lookupName base>"` (the source's
/// `Lookup p ("anchor:" <> lookupName base)`), call `f`, and free the
/// anchor on every exit path via [`AnchorGuard`]. `fromEnum
/// replacement_policy` is the `#[repr(i32)]` cast; the C++ body casts the
/// int straight back to its own enum.
///
/// `E: From<FfiError>`: the anchor allocation's error slot (ALLOCATION-
/// GUARD only) must merge into the caller's error type; see the
/// SIBLING-UNIT CONTRACTS note at the top of the file. (The sibling
/// lookup.rs scoped helpers state the same bound since the units 28–31
/// fix round, where the reviewer proved with rustc repros that the
/// `invoke(...)?` merge is a hard compile requirement.)
pub fn with_cache<T, E: From<FfiError>>(
    base: &impl CanLookup,
    cache: &LookupCache,
    policy: ReplacementPolicy,
    f: impl FnOnce(&Lookup) -> Result<T, E>,
) -> Result<T, E> {
    // `withLookup base $ \base_lookup -> with cache $ \cache_ptr -> ...`:
    // the CanLookup borrow pins the base pointer for the call and the
    // `cache` borrow keeps the cache handle alive for the whole anchor
    // lifetime (the C++ Anchor retains `cache.ptr`).
    base.with_lookup(|base_ptr| {
        let mut anchor: *mut c_void = ptr::null_mut();
        // SAFETY: `base_ptr` is the live Lookup pointer CanLookup pins for
        // this call, `cache.ptr` is a live owned handle borrowed for its
        // duration, `anchor` is an initialised stack out-param, and the
        // extern order mirrors ffi.h:164-168. invoke owns the error
        // string's release. If the call fails, `?` returns early before
        // the guard exists — no leak, and the source's bracket would have
        // thrown before entering `f` the same way.
        unsafe {
            invoke(|| {
                sys::glean_lookupcache_anchor_new(
                    base_ptr,
                    cache.ptr,
                    policy as i32,
                    &mut anchor,
                )
            })
        }?;
        let _guard = AnchorGuard(anchor);
        // SAFETY (doc-level): on success `anchor` is the live
        // uniquely-owned `Lookup*` the C++ side produced; the guard above
        // owns it for the rest of the call. `Lookup` is a borrowed wrapper
        // with no Drop (lookup.rs:60-65), so `f` cannot free it early.
        let lookup = Lookup::new(anchor, format!("anchor:{}", base.lookup_name()));
        f(&lookup)
    })
}

/// `newtype Stats = Stats (ForeignPtr Stats)` (LookupCache.hs:69) — the
/// statistics object. The source's doc (LookupCache.hs:67-68): "The 'Stat'
/// object can be shared between different 'LookupCache's which will
/// accumulate their statistics into it". Inventory row 70: "OWNED_MUTABLE";
/// "shared across caches -> Arc<StatsHandle>" — see the sharing decision
/// in the file header.
pub struct Stats {
    /// Opaque `SharedLookupCacheStats*` from `glean_lookupcache_stats_new`
    /// (the only construction site; the field is private). Non-null for
    /// every value this module produces; never dereferenced on the Rust
    /// side.
    ptr: *mut c_void,
}

// SAFETY: `Stats` is a thin owner of the opaque C++ wrapper
// `SharedLookupCacheStats` ({ shared_ptr<LookupCache::Stats> },
// ffi.cpp:163); the pointer is only handed back to the C functions in
// `sys`, which take it as `SharedLookupCacheStats*`. The pointee's values
// are `folly::ThreadCachedInt<uint64_t>` (cache.h:91), built for
// concurrent accumulation across threads, and the per-cache `shared_ptr`
// copies (ffi.cpp:192) mean no Rust-side state is transferred by moving
// the handle. This matches GHC's unconditionally shareable ForeignPtr and
// crate::ffi's ForeignPtr Send/Sync impls (ffi.rs:455-463), and is what
// the row-70 `Arc<Stats>` sharing shape needs.
unsafe impl Send for Stats {}
// SAFETY: as Send; `&Stats` exposes no Rust-visible interior state — only
// the `sys` calls (read_and_reset_counters, and `new`, which copies the
// shared_ptr), all of which are the C++ side's own thread-safe paths.
unsafe impl Sync for Stats {}

impl Drop for Stats {
    fn drop(&mut self) {
        // SAFETY: `self.ptr` is non-null and originates from a successful
        // `glean_lookupcache_stats_new` (the field is private and
        // `new_stats` is the only constructor), so it is the live
        // uniquely-owned wrapper the C++ side produced. `Stats` is neither
        // Copy nor Clone, so this runs exactly once; the body
        // (ffi.cpp:167-169) is ffi::free_, whose wrap_ swallows every
        // exception. Freeing the wrapper does NOT kill the inner Stats:
        // every cache carries its own shared_ptr copy (ffi.cpp:192), so
        // dropping this handle only ends the reading side — which is why
        // the row's "shared across caches" works even though the Rust
        // handle is owned.
        unsafe { sys::glean_lookupcache_stats_free(self.ptr) };
    }
}

/// `newtype StatValues = StatValues (V.Vector Word64)` (LookupCache.hs:78)
/// — "An approximate snapshot of the values in a 'Stats' object". The
/// field is private: the source exports `StatValues` without its
/// constructor. The length is ALWAYS [`STAT_COUNT`] by construction
/// ([`StatValues::zero`], [`StatValues::combine`],
/// [`read_stats_and_reset_counters`], [`count_failures_as_misses`] all
/// preserve it), which is what makes [`get_stat`]'s index total.
#[derive(Clone, Debug)]
pub struct StatValues(Vec<u64>);

impl StatValues {
    /// `mempty = StatValues $ V.replicate sTAT_COUNT 0`
    /// (LookupCache.hs:85) — the `Monoid` identity. (Rust has no `Monoid`;
    /// this is the explicit fn of the §2 unsafePerformIO-row pattern the
    /// sibling subst.rs row-74 records.)
    pub fn zero() -> StatValues {
        StatValues(vec![0; STAT_COUNT])
    }

    /// `StatValues old <> StatValues new` (LookupCache.hs:80-82):
    /// per-index, counters (`isCounter (toEnum i)`) ADD, everything else
    /// takes `new`. `V.izipWith` truncates to the shorter vector; both
    /// sides are always `STAT_COUNT` here, so the index agreement ALL_STATS
    /// encodes is structural. `Word64` `+` wraps in the source; held
    /// bug-for-bug with `wrapping_add` (RULE §2 numeric-conversions row).
    pub fn combine(&self, new: &StatValues) -> StatValues {
        let values = ALL_STATS
            .iter()
            .map(|&stat| {
                let i = stat as usize;
                if is_counter(stat) {
                    self.0[i].wrapping_add(new.0[i])
                } else {
                    new.0[i]
                }
            })
            .collect();
        StatValues(values)
    }
}

/// `instance Show StatValues` (LookupCache.hs:87-91): `intercalate ", "`
/// over `enumFrom minBound`, each rendered `"name: value"` — ALL 13 stats,
/// counters or not, in discriminant order. The exact tag spellings are
/// load-bearing: `show` is the surface Glean/Write/Stats.hs:132 builds
/// metric names from (`show c`), so the `Display` below renders the
/// Haskell tags verbatim.
impl fmt::Display for StatValues {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        for &stat in ALL_STATS.iter() {
            if !first {
                f.write_str(", ")?;
            }
            first = false;
            write!(f, "{}: {}", stat, get_stat(self, stat))?;
        }
        Ok(())
    }
}

/// `data Stat = IdByKey_hits | ... | Fact_count` (LookupCache.hs:94-110).
/// The Haskell's own note (LookupCache.hs:93): "This must be kept in sync
/// with the stats in rts/cache.h".
//
// KEPT IN SYNC with glean/rts/cache.h:68-89 (`Stats::Stat`,
// `values[COUNT]`): index order is IdByKey_hits = 0 (FIRST_COUNTER),
// ... FactById_deletes = 10 (LAST_COUNTER), factBytes = 11,
// factCount = 12 (FIRST_NON_COUNTER .. COUNT). The indices are
// load-bearing: they address both the C++ `values[]` array and the
// snapshot buffer the two sides agree on.
//
// Variant names are the Haskell constructors CamelCased; the Display
// impl below renders the EXACT Haskell tag text, because
// Glean/Write/Stats.hs:132 builds metric names from `map toLower (show c)`
// ("glean.db.write.cache.idbykey_hits", ...) and Logging output uses the
// same text — the RULE §2 Show-tag-parity surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd)]
#[repr(u32)]
pub enum Stat {
    IdByKeyHits = 0,
    IdByKeyMisses = 1,
    IdByKeyFailures = 2,
    IdByKeyDeletes = 3,
    TypeByIdHits = 4,
    TypeByIdMisses = 5,
    TypeByIdFailures = 6,
    FactByIdHits = 7,
    FactByIdMisses = 8,
    FactByIdFailures = 9,
    FactByIdDeletes = 10,
    // Slightly ugly names because we want them to be nice on ODS
    // (LookupCache.hs:107) — kept in the tag text.
    FactBytes = 11,
    FactCount = 12,
}

impl fmt::Display for Stat {
    /// `show` (LookupCache.hs:110, derived) — the constructor-shaped tag
    /// (RULE §2 Show-tag row); see the enum's doc for why the exact text
    /// is load-bearing.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Stat::IdByKeyHits => "IdByKey_hits",
            Stat::IdByKeyMisses => "IdByKey_misses",
            Stat::IdByKeyFailures => "IdByKey_failures",
            Stat::IdByKeyDeletes => "IdByKey_deletes",
            Stat::TypeByIdHits => "TypeById_hits",
            Stat::TypeByIdMisses => "TypeById_misses",
            Stat::TypeByIdFailures => "TypeById_failures",
            Stat::FactByIdHits => "FactById_hits",
            Stat::FactByIdMisses => "FactById_misses",
            Stat::FactByIdFailures => "FactById_failures",
            Stat::FactByIdDeletes => "FactById_deletes",
            Stat::FactBytes => "Fact_bytes",
            Stat::FactCount => "Fact_count",
        })
    }
}

/// `sTAT_COUNT = fromEnum (maxBound :: Stat) + 1` (LookupCache.hs:112-113)
/// — 13, the C++ `Stats::COUNT` (cache.h:88). This is the size the
/// read-and-reset call MUST pass (ffi.cpp:176).
pub const STAT_COUNT: usize = 13;

/// `lAST_COUNTER = FactById_deletes` (LookupCache.hs:115-116) — the last
/// counter index (10). Everything after it is a SUM the C++ side never
/// resets (FIRST_NON_COUNTER, cache.h:83).
pub const LAST_COUNTER: Stat = Stat::FactByIdDeletes;

/// `isCounter x = x <= lAST_COUNTER` (LookupCache.hs:120-121) — "Counters
/// are values which we want to bump in ODS; the others we want to set."
pub fn is_counter(s: Stat) -> bool {
    s <= LAST_COUNTER
}

/// `[minBound .. maxBound]` over [`Stat`] — the enumeration the Haskell's
/// derived `Enum`+`Bounded` provides (LookupCache.hs:110). The Show
/// instance iterates it (:88) and Glean/Write/Stats.hs:133 builds one
/// metric per element, so consumers need it too. Discriminant order, with
/// the indexing invariant `ALL_STATS[i] as u32 == i` that `combine` and
/// `get_stat` rely on.
pub const ALL_STATS: [Stat; STAT_COUNT] = [
    Stat::IdByKeyHits,
    Stat::IdByKeyMisses,
    Stat::IdByKeyFailures,
    Stat::IdByKeyDeletes,
    Stat::TypeByIdHits,
    Stat::TypeByIdMisses,
    Stat::TypeByIdFailures,
    Stat::FactByIdHits,
    Stat::FactByIdMisses,
    Stat::FactByIdFailures,
    Stat::FactByIdDeletes,
    Stat::FactBytes,
    Stat::FactCount,
];

/// `getStat (StatValues as) c = as V.! fromEnum c` (LookupCache.hs:123-124)
/// — index the snapshot. The source's `V.!` would error on an out-of-range
/// index; here the index is total because every `StatValues` carries
/// exactly `STAT_COUNT` elements by construction (the "length == COUNT"
/// invariant both sides maintain) — the panic is unreachable through this
/// module's API, so no BUG-class marker is raised.
pub fn get_stat(values: &StatValues, stat: Stat) -> u64 {
    values.0[stat as usize]
}

/// `newStats = construct $ invoke glean_lookupcache_stats_new`
/// (LookupCache.hs:126-127) — a fresh, empty statistics object.
///
/// Guard kind: ALLOCATION-GUARD only — the body (ffi.cpp:161-165) is a
/// plain `new SharedLookupCacheStats{...}` with no precondition check;
/// returned as `Err` (the source's exception made explicit — the caller
/// handles it, e.g. Open.hs:648-651 wraps cache construction in the write
/// setup path).
pub fn new_stats() -> Result<Stats, FfiError> {
    let mut ptr: *mut c_void = ptr::null_mut();
    // SAFETY: `ptr` is an initialised stack out-param valid for the call;
    // the extern's parameter order mirrors ffi.h:149. invoke owns the
    // error string's release.
    unsafe { invoke(|| sys::glean_lookupcache_stats_new(&mut ptr)) }?;
    // SAFETY (doc-level): on success `ptr` is the fresh live
    // uniquely-owned `SharedLookupCacheStats*` the C++ side produced; the
    // `?` above returned early on failure, so a null cannot be wrapped.
    Ok(Stats { ptr })
}

/// `readStatsAndResetCounters` (LookupCache.hs:131-140) — "Obtain an
/// approximate snapshot of the values in 'Stats' and reset all counters
/// to 0 (but not sums like total cache size)."
///
/// The symbol is VOID — no error channel (ffi.h:151-154) — so this
/// returns no `Result`: the caller-owned buffer is filled
/// unconditionally, exactly as the Haskell's `VM.new`/`unsafeFreeze` pair
/// was. The buffer is exactly `STAT_COUNT` (13): the C++ body copies only
/// when `size == buffer.size()` and otherwise LOG(ERROR)s + zero-fills
/// (ffi.cpp:176-182), so 13 keeps us on the copy branch. Counters
/// [0, FIRST_NON_COUNTER) are read-and-reset; the two sums are read only
/// (cache.cpp:25-34).
pub fn read_stats_and_reset_counters(stats: &Stats) -> StatValues {
    let mut values = vec![0u64; STAT_COUNT];
    // SAFETY: `stats.ptr` is a live handle (borrowed for the call),
    // `values` owns STAT_COUNT initialised u64 slots, and the extern —
    // which mirrors ffi.h:151-154 and returns nothing — writes exactly
    // that many because `size == 13` selects the copy branch.
    unsafe {
        sys::glean_lookupcache_stats_read_and_reset_counters(
            stats.ptr,
            values.as_mut_ptr(),
            values.len(),
        );
    }
    StatValues(values)
}

/// `countFailuresAsMisses` (LookupCache.hs:145-151) — "Replace miss counts
/// with failure counts. This is useful when the cache is being used
/// without a base DB, e.g. in SendAndRebaseQueue. In that case there will
/// always be zero cache misses, and we're interested in failures."
///
/// The Haskell's `vec V.// [...]` updates the three `*_misses` slots from
/// the CORRESPONDING `*_failures` slot of the ORIGINAL snapshot (its
/// offsets read `stats`, not the evolving copy) — the same reads here.
pub fn count_failures_as_misses(stats: &StatValues) -> StatValues {
    let mut values = stats.0.clone();
    values[Stat::IdByKeyMisses as usize] = get_stat(stats, Stat::IdByKeyFailures);
    values[Stat::TypeByIdMisses as usize] = get_stat(stats, Stat::TypeByIdFailures);
    values[Stat::FactByIdMisses as usize] = get_stat(stats, Stat::FactByIdFailures);
    StatValues(values)
}

/// `new capacity shards stats = with stats $ construct . invoke .
/// glean_lookupcache_new (fromIntegral capacity) (fromIntegral shards)`
/// (LookupCache.hs:153-157) — create a cache that accumulates into
/// `stats` (the C++ copies the inner shared_ptr, so the cache's counters
/// outlive this Rust `Stats` handle; see `Stats::drop`).
///
/// Guard kind: ALLOCATION-GUARD only (ffi.cpp:185-194 — `new LookupCache`
/// + its touched-buffer arrays); returned as `Err`.
pub fn new(capacity: u64, shards: usize, stats: &Stats) -> Result<LookupCache, FfiError> {
    let mut ptr: *mut c_void = ptr::null_mut();
    // RULE §2 (A5/numeric row): `fromIntegral capacity` is Word64 -> CSize
    // — `as usize`, identity on LP64 and truncating on 32-bit, the
    // source's own wrap semantics; `shards` is already the C size type
    // (the Haskell's `Int` conversion happened at the consumer boundary,
    // e.g. Open.hs:649-650).
    // SAFETY: `stats.ptr` is a live handle (borrowed for the call), `ptr`
    // is an initialised stack out-param, and the extern order mirrors
    // ffi.h:156-160. invoke owns the error string's release.
    unsafe {
        invoke(|| {
            sys::glean_lookupcache_new(capacity as usize, shards, stats.ptr, &mut ptr)
        })
    }?;
    // SAFETY (doc-level): on success `ptr` is the fresh live
    // uniquely-owned `LookupCache*`; the `?` above returned early on
    // failure, so a null cannot be wrapped.
    Ok(LookupCache { ptr })
}

/// `clear cache = with cache $ invoke . glean_lookupcache_clear`
/// (LookupCache.hs:159-160) — reset the cache's statistics bookkeeping
/// (cache.cpp:102-111 subtracts the current fact bytes/count; the source's
/// own TODO about rebuilding the cache contents is preserved there).
///
/// Guard kind: the body has no failure mode (a pure subtraction); the
/// `Err` arm exists for channel parity and is unreachable in practice.
pub fn clear(cache: &LookupCache) -> Result<(), FfiError> {
    // SAFETY: `cache.ptr` is a live handle borrowed for the call; the
    // extern mirrors ffi.h:162. invoke owns the error string's release.
    unsafe { invoke(|| sys::glean_lookupcache_clear(cache.ptr)) }
}

// Marker breakdown (closed set — no open port TODOs, no source-defect
// reproductions, no performance deferrals, 1 inventory-bug-candidate):
//   * no open port TODOs: every source question is answered by the pinned rows
//     and the C++ bodies (the error-recovery classification per symbol is
//     in the module doc; the sharing decision is the row's own
//     Arc<StatsHandle> guidance, realized as Send + Sync + `&Stats` API
//     with `Arc<Stats>` left to consumers). The row's "judge case per
//     variant" is a Step-6 verification task against cache.h (the
//     discriminants are pinned by value here and the kept-in-sync
//     comments cite both sides), not an open port question — the same
//     disposition the sibling subst.rs gives row 74's "judge case needed".
//   * no source-defect reproductions: no source defect is reproduced. Every quirk read on
//     this path (the clear() TODO, the static_cast of the replacement
//     policy, the size-mismatch LOG+zero-fill branch, Word64 counter
//     wrap) is the behaviour of the C++ we CALL or is held faithfully
//     (`wrapping_add`), not code re-implemented on the Rust side.
//   * no performance deferrals: no performance deferral — every allocation here is
//     source-mandated (the snapshot buffer, the source's `VM.new`).
//   * 1 inventory-bug-candidate (row 70 / inventory.ffi.tsv row 17): the
//     two unannotated pre-A2 "cxx" phrasings — row 70's "via cxx slice"
//     clause and the ffi-shard duplicate's "cxx bridge" + "via cxx
//     slice" — flagged in the header block (this file follows §1/A2 raw
//     extern-C per row 70's annotated clause, the same class bytecode.rs
//     raised for its pre-A2 cxx phrasings). Row 70 is otherwise followed
//     clause by clause (see the INVENTORY block).
//     Two reviewer notes that are NOT row divergences: (a) `with_cache`
//     states the `E: From<FfiError>` bound the `?` on the anchor error
//     requires (the sibling lookup.rs helpers state it too since the
//     units 28–31 fix round; semantics unchanged); (b) `LookupCache::with_ptr` is
//     the Rust landing of the source's `Object`/`with` instance, needed
//     by the FactSet rebase consumer (RTS/Foreign/FactSet.hs:110), not
//     covered by any row and not a divergence from one.
// SAFETY count: 12 unsafe constructs, each with a SAFETY comment — the
// two `unsafe impl Send/Sync for LookupCache` 2, its Drop 1, `with_cache`
// 1 (the anchor invoke), the two `unsafe impl Send/Sync for Stats` 2, its
// Drop 1, new_stats 1 (invoke), read_stats_and_reset_counters 1 (the
// no-slot extern call), new 1 (invoke), clear 1 (invoke), AnchorGuard's
// Drop 1. Three further doc-level SAFETY comments justify the safe struct
// constructions after the new_stats / new / with_cache success paths. The
// `pub mod sys` `unsafe extern` block is a declaration, not an operation
// (lookup.rs/subst.rs/bytecode.rs precedent), so it is not counted.
// PORT STATUS: confidence=high todos=0
