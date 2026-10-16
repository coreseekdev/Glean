// Ported from: glean/typed/Glean/Typed/Build.hs (139 lines)
// Manifest unit: rust/glean-typed/src/typed/build.rs (crate glean-typed) —
// migration/manifest.tsv row 15. RULE §4 (naming): one source file -> one
// target file; the manifest path is authoritative. The directory
// rust/glean-typed/src/typed/ already holds the sibling file id.rs
// (manifest line 16); this file does not touch it.
//
// INVENTORY (RULE §5): NO row has this file as its col1 subject — grep
// -i 'build.hs' over all six migration/inventory*.tsv returns ZERO
// governing rows (the path occurs only inside other rows' construct/target/
// evidence columns). The closest evidence is migration/inventory.tsv row 43
// (duplicated as inventory.stm.tsv row 30), which names this file's decode
// cache in its construct and target columns:
//   "JSON FactBuilder | Per-import IORef state: nextId, idsRef, owned map —
//    single-threaded builder scope (+ typed/Build.hs:88 decoder memo cache,
//    same shape) | MECHANICAL | ... Build.hs decode cache (newIORef IntMap
//    per decode call, Build.hs:84-88) -> local HashMap<u64,T> &mut. No
//    cross-thread sharing anywhere in either call path"
// The representation CLASS that row rules on is followed exactly: the cache
// is a plain local `HashMap` threaded by `&mut`, with NO lock/atomic (it is
// allocated per top-level `decode` call and never shared across threads).
// One divergence from the row's literal sketch is flagged below (key type).
//
// Row 60 (== inventory.ffi.tsv row 22) rules on the `glean_pop_value_*`/
// ffiBuf construct this file drives: its decode-side sketch replaces the
// shared FFIResultBuf slot with plain locals (PERF-flagged there). This file
// RETAINS the shared slot exactly as the source has it — Build.hs:103
// (`withFFIResultBuf` hands the env its alloca) — and the retention is
// sanctioned by the shipped consumer note at ffi.rs:684-687 ("this machinery
// is module parity plus whatever consumers still choose to use it").
//
// INVENTORY-BUG-CANDIDATE row 43: the row's cache sketch spells the key
// `HashMap<u64,T>`; this port keys both the cache and `serialized` by `i64`.
// The source's `IntMap` key is `Int` — `fromIntegral (fromFid (idOf fid))`
// (Fact.hs:54) — and `Int64 -> Int` is the identity on the 64-bit target, so
// `i64` introduces no reinterpretation where the row's `u64` would invent
// one. The representation class the row actually rules on (local HashMap,
// `&mut`, no synchronisation) is followed as written.
//
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, module-level): the source's
// module edges are outbound rows 1832 (Build.hs -> glean/hs/Glean/FFI.hs) and
// 1833 (Build.hs -> glean/hs/Glean/RTS.hs); inbound are rows 1818 (Typed.hs),
// 1830 (Binary.hs) and 1843 (Fact.hs). Every source import has a disposition:
//   * Glean.FFI (Build.hs:35) -> the DONE unit 8, rust/glean-core/src/ffi.rs:
//     `withFFIResultBuf`, `ffiBuf` and the `FFIResultBuf` type are consumed
//     from there and NEVER re-declared here.
//   * Glean.RTS (Build.hs:36) -> the DONE unit 16, rust/glean-core/src/rts.rs:
//     `RTS.glean_pop_value_selector` (Build.hs:115/:126/:139) is
//     `glean_core::rts::sys::glean_pop_value_selector` (rts.rs:303-307) — the
//     raw pop symbol rts.rs deliberately keeps public for exactly this
//     consumer (rts.rs's SIBLING-UNIT CONTRACT names this file: "where
//     `DecoderEnv` is DEFINED at :41-47; raw pops at :115/:126/:139").
//   * Glean.Types (Build.hs:37) -> the build.rs-generated pinned crate
//     `glean_thrift` (RULE §1 thrift row: generated, never hand-translated):
//     the thrift type `Thrift.Fact` and the `Thrift.Protocol` class
//     `ThriftEnum` (Build.hs:33/:135). See the ASSUMED-IMPORT block below.
//   * External packages (no manifest unit, all pinned): Control.Exception
//     (Exception, throwIO — Build.hs:20), Control.Monad (Build.hs:21),
//     Control.Monad.IO.Class (MonadIO, liftIO — :22), Data.ByteString /
//     Data.ByteString.Unsafe (ByteString, unsafeUseAsCStringLen — :23-24),
//     Data.Dynamic (Dynamic — :25), Data.IntMap (IntMap — :26-27),
//     Data.IORef (IORef — :28), Data.Text (Text — :29-30),
//     Foreign.Marshal.Utils (`with` — :31), Foreign (Ptr/castPtr/plusPtr/
//     minusPtr/peek — :32). Their dispositions are the mapping notes on
//     `DecoderEnv` and the error channel below.
//
// SIBLING-UNIT CONTRACTS (RULE §4; DONE siblings' real APIs read, never
// re-declared):
//   * glean_core::ffi (unit 8, read):
//       - `pub struct FFIResultBuf { words: [u64; 2] }` (ffi.rs:645-647),
//         whose fields are private and whose ONLY constructor is
//         `with_ffi_result_buf` (ffi.rs:661-664). There is no public way to
//         OWN an `FFIResultBuf`, which is why `DecoderEnv.buf` is a
//         `&mut FFIResultBuf` borrowed from that scope (see `DecoderEnv`).
//       - `pub fn with_ffi_result_buf<R>(f: impl FnOnce(&mut FFIResultBuf)
//         -> R) -> R` (ffi.rs:661-664) — the `withFFIResultBuf` twin.
//       - `pub unsafe fn ffi_buf<T: Copy>(buf: &mut FFIResultBuf, fun:
//         impl FnOnce(*mut T) -> *const c_char) -> Result<T, FfiError>`
//         (ffi.rs:701-713) — the invoke-then-read shared-slot helper
//         (`ffiBuf`); `T = usize` here, the selector's slot type.
//       - `FfiError` (ffi.rs:316-320).
//   * glean_core::rts::sys (unit 16, read): `pub fn glean_pop_value_selector(
//     start: *mut *const c_void, end: *const c_void, selector: *mut usize)
//     -> *const c_char` (rts.rs:303-307) — PRECONDITION-GUARD (truncated /
//     malformed varint), returned through `invoke` as `Err(FfiError)`.
//   * This file OWNS `DecoderEnv`/`Decoder`/`DecodingException`/`decodeFail`/
//     `decode`/`decodeWithCache`/`sumD`/`enumD`/`thriftEnumD` for the whole
//     port. rts.rs's own `Decoder<'a>` (rts.rs:700-704) is a DIFFERENT type
//     (a cursor over `&[u8]` for `Glean.RTS`'s `ST` decoder); the two are
//     unrelated despite the shared name.
//
// RULE §3 CRATE-SCOPE: the rulebook restricts `unsafe` to glean-core/
// glean-storage, and this file (glean-typed) is the one non-core exception —
// its escape-hatch scope is the SIBLING-UNIT CONTRACT at rts.rs:123-138,
// which designates build.rs as a raw-pop consumer of `glean_core::rts::sys`
// (named there alongside binary.rs). Every unsafe block below is FFI/
// pointer-machinery under that contract and carries its own in-place SAFETY
// justification; none of it silences the borrow checker.
//
// ASSUMED-IMPORT (RULE §4 B4 — the regime angle/types.rs:52-75 and
// define.rs:208-226 already use for this crate): `glean_thrift` is the
// build.rs-generated pinned crate (RULE §1 thrift row) and does not exist in
// this draft tree yet. Assumed shapes:
//     // glean/if/glean.thrift:147-151
//     pub struct Fact { pub r#type: Id, pub key: Value, pub value: Value }
//     // hsthrift's `class ThriftEnum a` (Thrift/Protocol.hs:49-53),
//     // generated per-enum for the port:
//     pub trait ThriftEnum: Sized {
//         fn to_thrift_enum(i: i64) -> Self;   // `toThriftEnum :: Int -> a`
//     }
// Only the type `Fact` (a cache value) and `to_thrift_enum` (used by
// `thrift_enum_d`) are consumed here; the class's other methods
// (`fromThriftEnum`, `allThriftEnumValues`, `toThriftEnumEither`) have other
// in-tree consumers (Binary.hs:107, DB.hs:92/:121, glass, lsp) and belong to
// whatever unit ports the generated thrift surface. The hsthrift
// `toThriftEnum` is PARTIAL (`errorWithoutStackTrace` on out-of-range, e.g.
// "Thrift.Protocol.ThriftEnum.Bool.toThriftEnum: bad argument",
// Thrift/Protocol.hs:56-59); the Rust impl must panic with the same per-type
// message (RULE §2 partial-functions row).
//
// ---------------------------------------------------------------------------
// Mapping choices for the effectful record (RULE §0: the old code is the spec)
// ---------------------------------------------------------------------------
//   * `Ptr (Ptr ())` -> `*mut *const c_void`; `Ptr ()` -> `*const c_void`.
//     These are exactly the parameter types of the `glean_pop_value_selector`
//     extern this file drives (rts.rs:303-307). `begin` is `*mut` because the
//     C pop writes the advanced cursor back through it.
//   * `IORef (IntMap Dynamic)` -> `&'a mut HashMap<i64, Box<dyn Any>>`
//     (inventory.tsv row 43's `&mut`; `Dynamic` -> `std::any::Any`; key type
//     per the flagged note above).
//   * `IntMap Thrift.Fact` -> `&'a HashMap<i64, Fact>` (a SHARED borrow: the
//     source passes the immutable map by value and shares it structurally
//     across the nested `decodeWithCache` calls at Fact.hs:37/:40, so a
//     borrow — not a clone — is the faithful mapping).
//   * `Text` -> `String` (RULE §1 strings row); `ByteString` -> `&[u8]`.
//   * `IO a` (the `Decoder` monad's effect) -> `Result<a, DecodeError>`
//     (RULE §2 dynamic-exceptions row: a typed error enum per crate). The
//     source's `IO` carries an OPEN exception channel (`throwIO`); the port
//     enumerates every channel reachable from a `Decoder` body in the typed
//     tree in `DecodeError` below.
//
// Marker-grep note: the tally at the foot of the file describes the marker
// counts in prose so raw grep of this file's marker spellings stays exact.

// RULE §1 (third-party deps row): `thiserror` (error derive) is on the
// whitelist; `Any`/`HashMap`/`c_void` are std. The crate's Cargo.toml is not
// a manifest unit (owned by the crate-root unit, cf. id.rs's MODULE-WIRING
// note).
//
// MODULE-WIRING: `pub mod build;` — the module wiring is deferred to the
// crate-root unit, cf. id.rs.

use std::any::Any;
use std::collections::HashMap;
use std::os::raw::c_void;
use std::sync::Arc;

use glean_core::ffi::{ffi_buf, with_ffi_result_buf, FfiError, FFIResultBuf};
use glean_core::rts::sys::glean_pop_value_selector;
// ASSUMED-IMPORT (RULE §4 B4): see the header block.
use glean_thrift::{Fact, ThriftEnum};

// ---------------------------------------------------------------------------
// DecoderEnv (Build.hs:41-47)
// ---------------------------------------------------------------------------

/// `data DecoderEnv = DecoderEnv { begin :: !(Ptr (Ptr ())), end :: !(Ptr ()),
/// buf :: !FFI.FFIResultBuf, serialized :: IntMap Thrift.Fact,
/// cacheRef :: !(IORef (IntMap Dynamic)) }` (Build.hs:41-47).
///
/// Field-by-field mapping (see the header's "Mapping choices" block):
///   * `begin`/`end` -> the raw input-range pointers, `*mut`/`*const` exactly
///     as `glean_pop_value_selector` takes them. The Haskell `{-# UNPACK #-}`
///     on the pointer fields is the plain by-value raw pointer here.
///   * `buf` -> `&'a mut FFIResultBuf`. The sibling type cannot be owned
///     outside `with_ffi_result_buf` (ffi.rs:645-664), so the env borrows the
///     shared slot from that scope — the same sharing the Haskell got when
///     `withFFIResultBuf` handed the env its alloca.
///   * `serialized` -> `&'a HashMap<i64, Fact>` (shared immutable map).
///   * `cache_ref` -> `&'a mut HashMap<i64, Box<dyn Any>>` (row 43's `&mut`,
///     no lock).
///
/// The one lifetime `'a` is the SHORTEST of the three borrows — the
/// `FFIResultBuf` scope opened in [`decode_with_cache`] — and is what makes
/// [`Decoder`] a higher-ranked reader (see [`Decoder`]).
pub struct DecoderEnv<'a> {
    /// `begin`: the cursor cell the C pops advance in place.
    pub begin: *mut *const c_void,
    /// `end`: one past the end of the input.
    pub end: *const c_void,
    /// `buf`: the shared result slot (`withFFIResultBuf`'s alloca).
    pub buf: &'a mut FFIResultBuf,
    /// `serialized`: nested facts available to `decodeRef`.
    pub serialized: &'a HashMap<i64, Fact>,
    /// `cacheRef`: memo of already-deserialized nested facts.
    pub cache_ref: &'a mut HashMap<i64, Box<dyn Any>>,
}

// ---------------------------------------------------------------------------
// The Decoder reader monad (Build.hs:49-68)
// ---------------------------------------------------------------------------

/// `newtype Decoder a = Decoder { runDecoder :: DecoderEnv -> IO a }`
/// (Build.hs:49-53).
///
/// Rust has no `Functor`/`Applicative`/`Monad` traits, so the source's three
/// instances become the inherent combinators [`Decoder::map`] (Functor),
/// [`Decoder::pure`] + the free [`ap`] (Applicative) and
/// [`Decoder::and_then`] (Monad); [`Decoder::run_decoder`] is the record
/// field `runDecoder`.
///
/// The `IO` in `DecoderEnv -> IO a` is an OPEN exception channel in the
/// source; the port pins it to [`DecodeError`], so a `Decoder<A>` is a reader
/// of `&mut DecoderEnv` into `Result<A, DecodeError>`.
///
/// The env is taken by `&mut` because the source's `buf` and `cacheRef`
/// fields are mutable cells (an `FFIResultBuf` pointer and an `IORef`) and
/// the `>>=` instance threads ONE env through both halves (Build.hs:66-68).
/// A raw pointer would also work but would launder the borrow (RULE §3
/// forbids `unsafe` for that), so the borrow is kept and `DecoderEnv`'s
/// `FFIResultBuf`-scope lifetime is abstracted with `for<'a>`: a `Decoder` is
/// a value that runs against the env of ANY `with_ffi_result_buf` scope.
///
/// REPRESENTATION (reusable, not single-use): the source's `Decoder` is a
/// REUSABLE function value — `runDecoder` may be called any number of times,
/// and the in-tree consumers rely on it (Binary.hs:177-179 runs one decoder
/// `size` times against the same env; Haxl/DataSource/Glean/Common.hs:129
/// shares one across a `mapM_`). The closure is therefore `Fn` (not
/// `FnOnce`), [`Decoder::run_decoder`] borrows `&self`, and the combinators
/// borrow their decoders. The handle is an `Arc` so a combinator can clone
/// its share out of the borrowed decoder and still satisfy the `'static`
/// bound — a `Decoder` is a cheaply clonable handle to a reusable function,
/// and [`decode_with_cache`] takes it by reference so one decoder can serve
/// many decodes.
pub struct Decoder<A> {
    run: Arc<dyn for<'a> Fn(&mut DecoderEnv<'a>) -> Result<A, DecodeError>>,
}

impl<A: 'static> Decoder<A> {
    /// `Decoder` constructor (Build.hs:51) — the source's exported
    /// constructor (`Decoder(..)`, Build.hs:12).
    pub fn new<F>(run: F) -> Decoder<A>
    where
        F: for<'a> Fn(&mut DecoderEnv<'a>) -> Result<A, DecodeError> + 'static,
    {
        Decoder { run: Arc::new(run) }
    }

    /// `runDecoder` (Build.hs:52) — the source's record field. Borrows
    /// `&self`: the decoder is a reusable function and may run any number of
    /// times (see the representation note on [`Decoder`]).
    pub fn run_decoder<'a>(&self, env: &mut DecoderEnv<'a>) -> Result<A, DecodeError> {
        (self.run)(env)
    }

    /// `instance Functor Decoder where fmap f (Decoder g) = Decoder $ \env ->
    /// fmap f (g env)` (Build.hs:55-56). Borrows `self` (the source's `g` is
    /// a reusable function); `f` is `Fn` for the same reason.
    #[inline]
    pub fn map<B: 'static>(&self, f: impl Fn(A) -> B + 'static) -> Decoder<B> {
        let run = Arc::clone(&self.run);
        Decoder::new(move |env: &mut DecoderEnv<'_>| {
            let a = run(env)?;
            Ok(f(a))
        })
    }

    /// `instance Monad Decoder where Decoder m >>= k = Decoder $ \env -> do
    /// { a <- m env; runDecoder (k a) env }` (Build.hs:63-68). The SAME env
    /// is reborrowed for both halves — that is what the source's shared
    /// cursor cell / `IORef` require. Borrows `self`; `k` is `Fn` (the
    /// source's `k :: a -> Decoder b` is a reusable function).
    #[inline]
    pub fn and_then<B: 'static>(&self, k: impl Fn(A) -> Decoder<B> + 'static) -> Decoder<B> {
        let run = Arc::clone(&self.run);
        Decoder::new(move |env: &mut DecoderEnv<'_>| {
            let a = run(&mut *env)?;
            k(a).run_decoder(env)
        })
    }

    /// `pure x = Decoder $ \_ -> pure x` (Build.hs:59) — `Applicative.pure`.
    /// `A: Clone`: the closure is `Fn` (the decoder is reusable), so each run
    /// must produce a FRESH `A` — the source's `pure x` re-runs `return x`
    /// per decode, and `Clone` is the minimal way to reproduce that.
    pub fn pure(x: A) -> Decoder<A>
    where
        A: Clone,
    {
        Decoder::new(move |_env: &mut DecoderEnv<'_>| Ok(x.clone()))
    }
}

impl<A: 'static> Clone for Decoder<A> {
    /// Clones the `Arc` share — a cheap handle copy, not a copy of the
    /// decoder's behaviour (see [`Decoder`]'s representation note). This is
    /// what lets the combinators and `sum_d`/`enum_d` clone their captured
    /// decoders per run while staying `Fn`.
    fn clone(&self) -> Self {
        Decoder { run: Arc::clone(&self.run) }
    }
}

/// `(<*>) = ap` (Build.hs:61): `ap mf mx = mf >>= \f -> mx >>= \x ->
/// return (f x)`. Kept a FREE function (the source's operator) so the
/// argument order matches `mf <*> mx`. Borrows both decoders (the source's
/// `mf`/`mx` are reusable function values); the `Arc` shares are cloned out
/// of the borrows — same-module private-field access — so the result is
/// `'static` and owns its shares.
pub fn ap<A, B, F>(mf: &Decoder<F>, mx: &Decoder<A>) -> Decoder<B>
where
    F: Fn(A) -> B + 'static,
    A: 'static,
    B: 'static,
{
    let mf_run = Arc::clone(&mf.run);
    let mx_run = Arc::clone(&mx.run);
    Decoder::new(move |env: &mut DecoderEnv<'_>| {
        let f = mf_run(env)?;
        let x = mx_run(env)?;
        Ok(f(x))
    })
}

// ---------------------------------------------------------------------------
// DecodingException / decodeFail / the error channel (Build.hs:72-80)
// ---------------------------------------------------------------------------

/// `newtype DecodingException = DecodingException { decodingException ::
/// Text }` (Build.hs:72) with `instance Show DecodingException where show =
/// Text.unpack . decodingException` (Build.hs:74) — the RAW message, no
/// quotes (RULE §1 strings row: `Text` -> `String`).
///
/// The source's explicit raw `Show` becomes thiserror's `Display`
/// (`#[error("{decoding_exception}")]`), matching the sibling
/// `glean_core::rts::DecodingException` (rts.rs:706-723), whose header already
/// cites THIS file's raw `show` as the parity target. The derived `Debug` is
/// the port-wide stand-in for the type's (overridden) `Show` (RULE §2
/// deriving-Show row).
#[derive(Debug, thiserror::Error)]
#[error("{decoding_exception}")]
pub struct DecodingException {
    /// `decodingException` — the source's record field/accessor (exported via
    /// `DecodingException(..)`, Build.hs:14).
    pub decoding_exception: String,
}

/// The port's typed exception channel for [`Decoder`] (RULE §2
/// dynamic-exceptions row). The source's `IO` raises an open set; every
/// channel reachable from a `Decoder` body in the typed tree is enumerated:
///   * `Ffi(FfiError)` — `GleanFFIError` from the FFI pop calls (Build.hs:50
///     doc: "Can usually throw 'GleanFFIError'").
///   * `Decoding(DecodingException)` — `decodeFail` and the trailing
///     extra-bytes check (Build.hs:79-80, :106-108).
///   * `ErrorCall(String)` — the plain `Control.Exception.ErrorCall` a
///     decoder body can raise (Fact.hs:58 `throwIO $ ErrorCall "decodeRef:
///     wrong type"`), kept as the raw message.
///
/// Consumers that need only the first two channels can use this enum
/// directly; a consumer with an additional channel either maps the error or
/// extends this enum (the source's channel is open, the port's is typed).
#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    #[error(transparent)]
    Ffi(#[from] FfiError),
    #[error(transparent)]
    Decoding(#[from] DecodingException),
    /// `Control.Exception.ErrorCall`; `Display` is the raw message
    /// (`displayException` parity).
    #[error("{0}")]
    ErrorCall(String),
}

/// `decodeFail :: Text -> IO a` (Build.hs:79-80) — `throwIO .
/// DecodingException`. The source throws into the ambient `IO`; the port
/// returns the `Err` arm of the monad's channel (its callers are decoder
/// bodies, which propagate it with `?`).
pub fn decode_fail<A>(msg: impl Into<String>) -> Result<A, DecodeError> {
    Err(DecodeError::Decoding(DecodingException {
        decoding_exception: msg.into(),
    }))
}

// ---------------------------------------------------------------------------
// decode / decodeWithCache (Build.hs:82-109)
// ---------------------------------------------------------------------------

/// `decode decoder v = do { cache <- liftIO $ newIORef IntMap.empty;
/// decodeWithCache IntMap.empty cache decoder v }` (Build.hs:86-89). The
/// `MonadIO m` result is the explicit `Result<_, DecodeError>` here (the
/// typed channel; see the header's mapping notes).
pub fn decode<A: 'static>(decoder: Decoder<A>, v: &[u8]) -> Result<A, DecodeError> {
    let serialized: HashMap<i64, Fact> = HashMap::new();
    let mut cache: HashMap<i64, Box<dyn Any>> = HashMap::new();
    decode_with_cache(&serialized, &mut cache, &decoder, v)
}

/// `decodeWithCache serialized cache decoder v` (Build.hs:91-109).
///
/// `BS.unsafeUseAsCStringLen v $ \(p,n) -> ...`: borrowing `v` pins the
/// buffer for the call exactly as GHC pinned the `ByteString`; `p` is the
/// base and `end = p `plusPtr` n` is one past the end.
/// `Marshal.with begin $ \pbegin -> ...` is the stack cursor cell: `begin`
/// holds `p` and `pbegin` addresses it, so the C pops can advance it in
/// place. `withFFIResultBuf` opens the shared result-slot scope the env
/// borrows.
///
/// The trailing check reproduces `when (pos /= end)` (Build.hs:106-108): an
/// unconsumed tail raises `DecodingException` with the byte-identical
/// message `"extra <n> bytes at end of value"` (the same text rts.rs uses at
/// its own decoder).
pub fn decode_with_cache<A: 'static>(
    serialized: &HashMap<i64, Fact>,
    cache_ref: &mut HashMap<i64, Box<dyn Any>>,
    decoder: &Decoder<A>,
    v: &[u8],
) -> Result<A, DecodeError> {
    // unsafeUseAsCStringLen v: (p, n) = (v.as_ptr(), v.len()).
    let p = v.as_ptr() as *const c_void;
    // end = p `plusPtr` n.
    // SAFETY: `p` is the base of `v`'s allocation; `p.add(v.len())` is the
    // one-past-the-end pointer of that same allocation, valid to form and
    // never dereferenced.
    let end = unsafe { v.as_ptr().add(v.len()) } as *const c_void;
    // Marshal.with begin $ \pbegin -> ...: a stack cell seeded with `p`,
    // whose address is `pbegin`.
    let mut begin: *const c_void = p;
    let pbegin: *mut *const c_void = &mut begin;
    with_ffi_result_buf(move |buf| {
        let mut env = DecoderEnv {
            begin: pbegin,
            end,
            buf,
            serialized,
            cache_ref,
        };
        let x = decoder.run_decoder(&mut env)?;
        // pos <- peek pbegin.
        // SAFETY: `pbegin` points at the live `begin` local in the enclosing
        // frame; every C pop writes back only a cursor inside `[p, end]` on
        // success, so this read is a valid pointer read.
        let pos = unsafe { *pbegin };
        if pos != end {
            // extra = show (end `minusPtr` pos). The difference is
            // non-negative on every reachable path (the C pops never advance
            // past `end`), so the `usize` subtraction matches the Haskell's
            // Int `minusPtr` without its unreachable negative branch (the
            // same reasoning rts.rs:781-786 records).
            let extra = end as usize - pos as usize;
            return Err(DecodeError::Decoding(DecodingException {
                decoding_exception: format!("extra {} bytes at end of value", extra),
            }));
        }
        Ok(x)
    })
}

// ---------------------------------------------------------------------------
// sumD / enumD / thriftEnumD (Build.hs:112-139)
// ---------------------------------------------------------------------------

/// `index` from `sumD`'s where-clause (Build.hs:119-121): the `sel`-th
/// alternative, or `empty` when `sel` runs past the end. Note that `index 0
/// []` also yields `empty` — the source's first two equations require a
/// non-empty list, so only `index _ []` matches the empty case.
fn sum_index<B>(sel: usize, alts: Vec<Decoder<B>>, empty: Decoder<B>) -> Decoder<B> {
    let mut i = sel;
    let mut it = alts.into_iter();
    loop {
        match it.next() {
            Some(x) => {
                if i == 0 {
                    return x;
                }
                i -= 1;
            }
            None => return empty,
        }
    }
}

/// `sumD empty alts` (Build.hs:113-121): pop a selector and run the chosen
/// alternative, or `empty` when the selector is out of range. Can raise
/// `GleanFFIError` (`FfiError`).
pub fn sum_d<B: 'static>(empty: Decoder<B>, alts: Vec<Decoder<B>>) -> Decoder<B> {
    Decoder::new(move |env: &mut DecoderEnv<'_>| {
        let begin = env.begin;
        let end = env.end;
        // sel <- FFI.ffiBuf buf $ RTS.glean_pop_value_selector begin end
        let sel = unsafe {
            // SAFETY: `env.buf` is the live shared slot borrowed from the
            // enclosing `with_ffi_result_buf` scope; `begin`/`end` are the
            // input range `decode_with_cache` installed; `ffi_buf`'s contract
            // (ffi.rs:701-713) is exactly this out-slot call and it owns the
            // error string.
            ffi_buf(&mut *env.buf, |slot| {
                // SAFETY: one pop against the range above, writing exactly
                // one `usize` into `slot` before returning null on success.
                unsafe { glean_pop_value_selector(begin, end, slot) }
            })
        }?;
        // let Decoder f = index sel alts; f env. The closure is `Fn` (the
        // decoder is reusable), so the captured `alts`/`empty` are cloned per
        // run — `Arc` share clones, not deep copies.
        sum_index(sel, alts.clone(), empty.clone()).run_decoder(env)
    })
}

/// The Rust stand-in for the source's `(Enum a, Bounded a)` constraint on
/// [`enum_d`] (RULE §2 typeclass row: "plain generic traits or concrete
/// types"). `MIN_BOUND`/`MAX_BOUND` are `ord minBound`/`ord maxBound` where
/// `ord x = fromIntegral (fromEnum x)` (Build.hs:132); `from_enum` is
/// `toEnum`.
///
/// `Clone` (beyond `Sized`) is required by [`enum_d`]'s use of
/// [`Decoder::pure`]: the source's `return $ toEnum ...` re-runs per decode,
/// so the value must be producible fresh on every run — `Clone` is the
/// minimal such bound (the only in-tree impl, `bool`, is `Clone`).
pub trait BoundedEnum: Sized + Clone {
    /// `ord minBound`.
    const MIN_BOUND: i64;
    /// `ord maxBound`.
    const MAX_BOUND: i64;
    /// `toEnum` — only ever called in `[MIN_BOUND, MAX_BOUND]` by
    /// [`enum_d`]'s range check, so the partiality of Haskell's `toEnum`
    /// (which `enumD` never reaches) is not reproduced.
    fn from_enum(i: i64) -> Self;
}

/// base's `instance Enum Bool` / `Bounded Bool`, the only `enumD` consumer in
/// the tree (Binary.hs:148 `enumD fail` for `instance Type Bool`).
impl BoundedEnum for bool {
    const MIN_BOUND: i64 = 0;
    const MAX_BOUND: i64 = 1;
    fn from_enum(i: i64) -> Self {
        i != 0
    }
}

/// `enumD unknown` (Build.hs:124-132): pop a selector; if it is at most
/// `ord maxBound - ord minBound` decode `toEnum (sel + ord minBound)`,
/// otherwise run `unknown`. Can raise `GleanFFIError`.
pub fn enum_d<A: BoundedEnum + 'static>(unknown: Decoder<A>) -> Decoder<A> {
    Decoder::new(move |env: &mut DecoderEnv<'_>| {
        let begin = env.begin;
        let end = env.end;
        // SAFETY: as `sum_d` — the live shared slot plus the installed input
        // range; `ffi_buf` owns the returned error string.
        let sel = unsafe {
            ffi_buf(&mut *env.buf, |slot| {
                // SAFETY: one selector pop writing exactly one `usize` into
                // `slot` before returning null on success.
                unsafe { glean_pop_value_selector(begin, end, slot) }
            })
        }?;
        // if sel <= ord maxBound - ord minBound then ... else unknown.
        // `ord minBound`/the range are `CSize` (Word64) values in the source,
        // so the arithmetic below is computed in `u64` to match it
        // (Build.hs:127-128): the bounds are formed with the same wrap the
        // source's `fromIntegral` into `Word64` has, and the subtraction/
        // addition wrap exactly as the source's unsigned `CSize` arithmetic
        // does (the only in-tree impl, `bool`, has non-negative bounds, so
        // the behaviour is unchanged there).
        let ord_min = A::MIN_BOUND as u64 as usize;
        let range = (A::MAX_BOUND as u64).wrapping_sub(A::MIN_BOUND as u64) as usize;
        let dec = if sel <= range {
            // toEnum $ fromIntegral $ sel + ord minBound.
            Decoder::pure(A::from_enum(sel.wrapping_add(ord_min) as i64))
        } else {
            // Cloned per run: the closure is `Fn` (see `sum_d` above).
            unknown.clone()
        };
        dec.run_decoder(env)
    })
}

/// `thriftEnumD` (Build.hs:135-139): pop a selector and convert it with the
/// thrift enum's `toThriftEnum . fromIntegral` (an `Int`-valued conversion).
/// The conversion is hsthrift's PARTIAL `toThriftEnum`: an out-of-range
/// selector panics inside the impl with the source's per-type message (RULE
/// §2 partial-functions row; see the ASSUMED-IMPORT block).
pub fn thrift_enum_d<A: ThriftEnum + 'static>() -> Decoder<A> {
    Decoder::new(|env: &mut DecoderEnv<'_>| {
        let begin = env.begin;
        let end = env.end;
        // SAFETY: as `sum_d` — the live shared slot plus the installed input
        // range; `ffi_buf` owns the returned error string.
        let sel = unsafe {
            ffi_buf(&mut *env.buf, |slot| {
                // SAFETY: one selector pop writing exactly one `usize` into
                // `slot` before returning null on success.
                unsafe { glean_pop_value_selector(begin, end, slot) }
            })
        }?;
        // toThriftEnum . fromIntegral  (CSize -> Int).
        Ok(A::to_thrift_enum(sel as i64))
    })
}

// TODO(port): `Decoder<A>`'s closure is `'static` — the representation is
// an `Arc`-shared `Fn` closure (see `Decoder`), so the bound propagates to
// EVERY type parameter of every decoder constructor and combinator, not only
// to decoders that capture a borrow. Every in-tree decoder is built from
// `'static` pieces (the `Type` instance methods in Binary.hs, generated
// code), so no consumer needs a non-`'static` decoder today; if one appears
// (a decoder capturing a borrow), give `Decoder` a lifetime parameter
// instead of widening this bound.

// Coverage: every export of the Haskell module has a Rust counterpart —
// Decoder(..) (Build.hs:12) -> `Decoder<A>` + `Decoder::new` (constructor) +
// `Decoder::run_decoder` (the `runDecoder` field) + the three instance
// combinators `map`/`pure`/`and_then` and the free `ap`; DecoderEnv(..)
// (Build.hs:13) -> `DecoderEnv<'a>` with its five `pub` fields;
// DecodingException(..)/decodeFail (Build.hs:14) -> `DecodingException` +
// `decode_fail`; decode (Build.hs:15) -> `decode`; decodeWithCache
// (Build.hs:16) -> `decode_with_cache`; sumD/enumD/thriftEnumD (Build.hs:17)
// -> `sum_d`/`enum_d`/`thrift_enum_d`. Port-added scaffolding with no source
// export: `DecodeError` (the typed exception channel, RULE §2
// dynamic-exceptions row), the `BoundedEnum` trait + its `bool` impl (the
// `(Enum a, Bounded a)` stand-in), and the private `sum_index`.
//
// Marker breakdown (closed set — one open port todo, no source-defect
// reproductions, no performance deferrals, one inventory-bug-candidate):
//   * one open port todo (at the foot of the code): the `'static` bound that
//     the reusable-`Fn` representation puts on every decoder type
//     parameter — no consumer needs a borrowed capture today; the fix, if
//     one appears, is a lifetime parameter on the type.
//   * no source-defect reproductions: the source has no defect here — the
//     only partiality (`toThriftEnum` out of range, and base's `toEnum`) is
//     reached only through the impl's own panic, which the port keeps.
//   * no performance deferrals: the shared result slot is the source's own
//     shape, and `serialized`/the cache are borrowed rather than cloned, so
//     nothing here is slower than the Haskell.
//   * one inventory-bug-candidate: the key-type note on row 43 at the head of
//     the file (the row's `u64` sketch realised as `i64`).
// SAFETY count: 8 unsafe constructs — all FFI/pointer-machinery, none to
// silence the borrow checker (RULE §3); each carries its own soundness
// justification in place: the one-past-the-end pointer formation and the
// cursor-cell read in `decode_with_cache`, and the `ffi_buf`-plus-extern pair
// in each of `sum_d`, `enum_d` and `thrift_enum_d`. There are no other unsafe
// constructs in this file.
// PORT STATUS: confidence=high todos=1
