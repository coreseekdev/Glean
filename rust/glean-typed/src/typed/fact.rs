// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// Fact — decoding serialized thrift Facts into typed predicate values
// (decodeFact) and resolving fact references with the decode cache
// (decodeRef).
//
// Ported from `glean/typed/Glean/Typed/Fact.hs` (66 lines).
//
// Manifest unit: rust/glean-typed/src/typed/fact.rs
// (migration/manifest.tsv line 35, crate glean-typed). RULE §4: one source
// file -> one target file; the manifest path is authoritative.
//
// INVENTORY (RULE §5): NO inventory row governs this file — grep
// "Typed/Fact" across all six migration/inventory*.tsv returns ZERO rows,
// and the broader substring "Fact.hs" occurs only as part of "BuildFact.hs"
// in inventory.tsv row 118 (== inventory.stm.tsv row 31), whose subject is
// glean/write/Glean/Write/Async.hs and whose construct rules on
// BuildFact's Facts IORefs — not this module. The source is the spec
// (RULE §0). No INVENTORY-BUG-CANDIDATE. The one inventory text that
// touches this file's neighborhood is build.rs's row-43 note
// (build.rs:32-38), whose key-type flag (`i64`, not `u64`) cites this
// file's Fact.hs:54 as its evidence; the shipped pin
// `HashMap<i64, _>` is followed here unchanged.
//
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, module-level): the
// source's in-tree module edges are outbound rows 1842 (Fact.hs ->
// Typed/Binary.hs), 1843 (-> Typed/Build.hs), 1844 (-> Typed/Id.hs),
// 1845 (-> Typed/Predicate.hs); inbound is row 1820 (Glean/Typed.hs, the
// crate root's re-export). The qualified thrift import (Fact.hs:25
// `import Glean.Types as Thrift`) rides the generated-crate edge the way
// BuildFact's does (module-level edges do not list Glean.Types). Every
// source import has a disposition:
//   * Control.Exception (ErrorCall, throwIO — Fact.hs:14) -> the
//     pre-pinned `DecodeError::ErrorCall` channel (build.rs:357-359 pins
//     the variant citing this file's Fact.hs:58 — "pinned IN ADVANCE"
//     for this unit; this file is that pin's promised consumer).
//   * Control.Monad.IO.Class (MonadIO, liftIO — Fact.hs:15) -> the
//     Result channel (Mapping choices below).
//   * Data.Dynamic (Dynamic, toDyn, fromDynamic — Fact.hs:16) -> the
//     shipped cache pin: `toDyn f` = `Box::new(f)` unsize-coerced to
//     `Box<dyn Any>`; `fromDynamic dyn` = `downcast_ref::<P>()`, whose
//     None arm IS the Fact.hs:58 wrong-type ErrorCall.
//   * Data.IntMap / Data.IORef (Fact.hs:17-19) -> the build.rs/binary.rs
//     pin `HashMap<i64, _>` with `&mut` for the IORef: `IntMap.lookup` =
//     `get`, `IntMap.insert` = `insert` (plain overwrite on both sides —
//     no combining function anywhere in this file), `readIORef` = the
//     map's state at the `get`, `modifyIORef'` = the in-place `insert`.
//     The row-43 key-type note is owned by build.rs:32-38 and is not
//     re-flagged here.
//   * Glean.Typed.Build (Fact.hs:21) -> crate::typed::build (SHIPPED,
//     manifest line 15): Decoder/DecoderEnv/runDecoder/decodeWithCache.
//   * Glean.Typed.Binary (Fact.hs:22) -> crate::typed::binary (SHIPPED,
//     line 17): the Type class (decodeRtsValue/constantRtsValue).
//   * Glean.Typed.Id (Fact.hs:23) -> crate::typed::id (SHIPPED, line
//     16): IdOf — and `fromFid`, which reaches this module through the
//     `Fid(..)` re-export (Id.hs:14; the selector is defined at
//     glean/hs/Glean/RTS/Types.hs:70 `newtype Fid = Fid { fromFid :: Id }`).
//   * Glean.Typed.Predicate (Fact.hs:24) -> crate::typed::predicate
//     (SHIPPED, line 33): Predicate (mkFact), justId.
// In-tree consumers of this module's exports (the evidence for the
// `MonadIO m` mapping): the GENERATED per-schema instances
// (glean/schema/gen/Glean/Schema/Gen/Haskell.hs:327-328 emit
// `decodeRtsValue = Glean.decodeRef` and `decodeAsFact =
// Glean.decodeFact`), Haxl's data source (Haxl/DataSource/Glean/
// Common.hs:118-121, m at IO via liftIO), and the bench pair
// (glean/bench/TypedBench.hs:54/:59, m at IO — the NOINLINE/INLINE
// poly/mono arms the source's own INLINE pragma exists to serve).
//
// ASSUMED-IMPORT (RULE §4 B4) — RESOLVED against the shipped sketches, NO
// extension of the surface: `glean_thrift::Fact`'s first sketch is OWNED
// by typed/build.rs:104-105
//     // glean/if/glean.thrift:147-151
//     pub struct Fact { pub r#type: Id, pub key: Value, pub value: Value }
// and typed/binary.rs consumed it (binary.rs:45-49 header note;
// binary.rs:259-260's `let glean_thrift::Fact { key, .. } = fact`,
// passing `key` as decode_with_cache's `v: &[u8]` — pinning `Value`'s
// deref-coercible byte image). This file cross-references both sketches
// and adds nothing: it consumes `{ key, value, .. }` only. Thrift truth
// (glean/if/glean.thrift): `typedef binary Value` (:143-144, NO
// hs.newtype — a plain alias, Haskell image ByteString, Rust image
// deref-coercible to `&[u8]`), and `struct Fact { 1: Id type; 2: Value
// key; 3: Value value }` (:147-151). NOTE (unit-brief correction): the
// brief spelled the struct `1: Id pid; 2: Byte key; 3: Value value` —
// the schema says field 1 is `type` and field 2 is `Value`, not `Byte`;
// the source REQUIRES both `k` and `v` to be byte-strings (both feed
// decodeWithCache's ByteString parameter), so the brief's `Byte` reading
// could not type-check against Fact.hs:36-40 in any case. The shipped
// sketch (`r#type`) matches the schema and is followed; field 1 is
// ignored here exactly as the source's `_pid` ignores it (Fact.hs:36).
//
// SIBLING-UNIT CONTRACTS (RULE §4; DONE units' real APIs read, never
// re-declared):
//   * crate::typed::build (SHIPPED, read): `Decoder<A>` (:237) — a
//     reusable `Arc`-shared `Fn` handle — with `Decoder::new` (:244) and
//     `run_decoder(&self, env: &mut DecoderEnv<'a>) -> Result<A,
//     DecodeError>` (:254); `DecoderEnv<'a>` (:188) with pub fields
//     begin/end/buf/serialized/cache_ref (:190-198 — this unit reads only
//     `serialized` and `cache_ref`, never the raw pointers);
//     `decode_with_cache(serialized: &HashMap<i64, Fact>, cache_ref:
//     &mut HashMap<i64, Box<dyn Any>>, decoder: &Decoder<A>, v: &[u8]) ->
//     Result<A, DecodeError>` (:414-419); `DecodeError` (:365) whose
//     `ErrorCall(String)` variant (:373) was pinned in advance for THIS
//     unit (its doc bullet at :357-359 cites Fact.hs:58).
//   * crate::typed::binary (SHIPPED, read): `pub trait Type<A: 'static>`
//     (:235) — `decode_rts_value() -> Decoder<A>` (:242) and
//     `constant_rts_value() -> Option<A>` default None (:245-247) are
//     the two members decode_fact's arms dispatch through. THE OVERRIDE
//     CONTRACT: `decode_as_fact` (:253-261) is the default every
//     NON-predicate type keeps (Binary.hs:84-85 — it destructures only
//     `key`), while "predicate types override it with 'decodeFact'"
//     (Binary.hs:74-75; restated in binary.rs:249-252's method doc).
//     THIS unit is that override's function: the generated instance is
//     the eta-reduced pass-through `decodeAsFact = Glean.decodeFact`
//     (Gen/Haskell.hs:328; paired with `decodeRtsValue =
//     Glean.decodeRef` at :327), so `decode_fact`'s parameter list
//     matches the override's exactly (`&HashMap`, `&mut HashMap`,
//     `&IdOf<P>`, `&Fact` -> Result) and the generated override body is
//     the same pass-through. Also consumed: `impl Type<IdOf<P>> for
//     IdOf<P>` (:706) — decode_ref's `(fid :: IdOf p) <- runDecoder
//     decodeRtsValue env` (Fact.hs:52) is that instance's
//     decode_rts_value (Binary.hs:235-236, binary.rs:716-726), whose
//     `P: Type<P>` bound the Predicate supertrait supplies.
//   * crate::typed::predicate (SHIPPED, read): `pub trait Predicate:
//     Sized + 'static + Type<Self>` (:268) with associated types
//     KeyType (:271) / ValueType (:287) and `mk_fact(id: IdOf<Self>,
//     key: Option<Self::KeyType>, value: Option<Self::ValueType>) ->
//     Self` (:295-299); `just_id(x: IdOf<P>) -> P` (:336-338,
//     Predicate.hs:85-86). Its header's STRUCTURE CHOICE (:174-179)
//     NAMES THIS UNIT as the where-clause landing for the source's
//     inexpressible superclasses `Type (KeyType p)` / `Type (ValueType
//     p)` (Predicate.hs:64): both functions below carry exactly
//     `P::KeyType: Type<P::KeyType>, P::ValueType: Type<P::ValueType>`;
//     the first superclass `Type p` IS the supertrait and needs no
//     clause.
//   * crate::typed::id + glean_core::rts::types (SHIPPED, read):
//     `IdOf<P>::new(Fid)` (id.rs:205) and `.id_of() -> Fid`
//     (id.rs:213); `Fid(pub Id)` (rts/types.rs:153-155) where `Id` is
//     the plain thrift synonym `pub type Id = i64` (the assumed pin at
//     rts/types.rs:50, re-exported at :101) — ONE newtype layer, so
//     `fid.id_of().0` IS the i64: the image of `fromIntegral (fromFid
//     (idOf fid))` (Fact.hs:54 — `fromFid` is Fid's record selector,
//     then `fromIntegral`'s Int64 -> Int identity on LP64, the ruling
//     build.rs:34-38 records citing this very line). Fid is Copy
//     (derive at rts/types.rs:153-155; the id.rs:98-101 commentary notes
//     it), which is what makes the id reconstruction below a
//     field read rather than a clone.
//
// ---------------------------------------------------------------------------
// Mapping choices (RULE §0: the old code is the spec)
// ---------------------------------------------------------------------------
//   * `IntMap Thrift.Fact` -> `&HashMap<i64, Fact>` (a SHARED borrow: the
//     source passes the map by value and shares it structurally across
//     the nested decodeWithCache calls at Fact.hs:37/:40);
//     `IORef (IntMap Dynamic)` -> `&mut HashMap<i64, Box<dyn Any>>`
//     (inventory row 43's `&mut`, build.rs:130-132). No second IntMap
//     image is introduced.
//   * `m p` with `MonadIO m` -> `Result<P, DecodeError>`. decodeFact is
//     polymorphic in m, but every in-tree instantiation is IO-shaped
//     (`liftIO` at Common.hs:118-121; `IO` at TypedBench.hs:54/:59; the
//     generated override's `m a` is Binary.hs's decodeAsFact channel,
//     which binary.rs:253-258 already pinned to this same Result), and
//     the monad's only effects on this path are decodeWithCache /
//     decodeRef effects, all of which live in the pinned `DecodeError`
//     channel. The `liftIO`s in decodeRef's body dissolve accordingly:
//     the Decoder monad's underlying IO IS the Result channel
//     (build.rs:138-142).
//   * `Thrift.Fact _pid k v` -> destructure `Fact { key: k, value: v,
//     .. }` on `&Fact` (the binary.rs:259 precedent); `k`/`v` reach
//     decode_with_cache's `v: &[u8]` by the same deref coercion the
//     default decode_as_fact uses for `key`.
//   * `mkFact fid ...` (the Haskell shares the immutable `fid` into the
//     pure application for free) -> decode_fact takes `fid: &IdOf<P>` —
//     matching the generated override's `&IdOf<A>` parameter
//     (binary.rs:256) so that override stays the source's eta-reduced
//     pass-through — and performs the ONE reconstruction
//     `IdOf::new(fid.id_of())` at the mk_fact call (Fid: Copy,
//     id.rs:98-101). id.rs's deliberate no-Copy/no-Clone ruling for the
//     IdOf wrapper itself is respected: nothing clones an IdOf.
//   * Haskell's free sharing of the decoded fact between the cache and
//     the return (the miss arm caches `toDyn f` AND returns `f`), and
//     between a cache entry and every hit (`fromDynamic` hands out the
//     shared value) -> `P: Clone` on `decode_ref` ONLY (decode_fact
//     duplicates nothing: its key/value results move straight into
//     mk_fact). Each `p.clone()` is one duplication the Haskell got for
//     free; the precedent is build.rs `Decoder::pure`'s `A: Clone`
//     (:285-291 — "Clone is the minimal way to reproduce that"). See
//     the TODO at `decode_ref`.
//   * Effect order (probe-verified): decodeFact's applicative chain runs
//     the key-decode effects FIRST, then the value arm — and the
//     `constantRtsValue = Just` shortcut runs NO value effects at all.
//     Verified with a scratch GHC 9.4.7 probe of the exact shape
//     (`mkFact-stand-in fid <$> (Just <$> dec "KEY") <*> (Just <$> case
//     c of Just val -> return val; Nothing -> dec "VAL")`): the Just arm
//     logs `["dec KEY"]` only; the Nothing arm logs `["dec KEY","dec
//     VAL"]`; a key-side failure surfaces as `user error (failed@KEY)`
//     with the value decode never entered. The generic
//     Monad-derived Applicative behind this (`(<*>) = ap`,
//     Build.hs:58-61, quoted at build.rs:306-307 — `ap mf mx = mf >>=
//     \f -> mx >>= \x -> return (f x)`) and IO's instance are both
//     left-to-right. The Rust sequencing below (`let key = ...?;` then
//     the value match, then the pure mk_fact) is that order.
//   * Laziness: the pure layer the Haskell defers (the mkFact
//     constructor application, the `Just` wrappers, `toDyn f`'s box)
//     hides NO effects — `k`/`v` are total constructor applications
//     whose only partial piece (thriftEnum's `toThriftEnum`) already ran
//     inside the decode effects — so the eager Rust image is observably
//     identical (the bench's own whnf-sufficiency note,
//     TypedBench.hs:61-62, corroborates that the pure layer is inert).
//   * The snapshot read (`cache <- liftIO $ readIORef cacheRef`,
//     Fact.hs:53) versus the insert (`modifyIORef' cacheRef $ \cache ->
//     IntMap.insert id (toDyn f) cache`, :64-65): in the `&mut` image
//     the "snapshot" is simply the map's state at the `get` — the lookup
//     sees the pre-decode state; the insert applies to the same cell
//     AFTER decode_fact's nested cache mutations (a stale-copy insert is
//     structurally impossible under `&mut`), and its POSITION — strictly
//     between decode_fact completing and the return — is preserved
//     exactly.
//   * `{-# INLINE decodeFact #-}` (Fact.hs:35) -> `#[inline]` (the
//     pragma-to-attribute image; behavior-neutral: the source pragma
//     exists so call sites such as TypedBench's mono arm monomorphise,
//     and Rust generics already monomorphise per instantiation).
//     decodeRef carries no pragma in the source and none here.
//   * `Typeable p` (Fact.hs:49) -> no extra bound: it exists solely to
//     support toDyn/fromDynamic, whose Rust image needs exactly
//     `P: 'static` — already carried by the Predicate supertrait
//     (predicate.rs:268).
//
// MODULE-WIRING: `pub mod fact;` — deferred to the crate-root unit
// (cf. id.rs/build.rs/build_fact.rs's MODULE-WIRING notes).

use std::any::Any;
use std::collections::HashMap;

use crate::typed::binary::Type;
use crate::typed::build::{decode_with_cache, DecodeError, Decoder, DecoderEnv};
use crate::typed::id::IdOf;
use crate::typed::predicate::{just_id, Predicate};

// ASSUMED-IMPORT (RULE §4 B4): see the header block — the build.rs-owned
// sketch, consumed unchanged.
use glean_thrift::Fact;

// ---------------------------------------------------------------------------
// decodeFact (Fact.hs:27-40)
// ---------------------------------------------------------------------------

/// Decode a serialized fact, possibly with nested facts (source doc,
/// Fact.hs:27).
///
/// `decodeFact :: (Predicate p, MonadIO m) => IntMap Thrift.Fact -> IORef
/// (IntMap Dynamic) -> IdOf p -> Thrift.Fact -> m p` (Fact.hs:28-34).
/// Parameter mapping: the serialized map by shared borrow, the cache by
/// `&mut` (row 43), `IdOf p` by `&IdOf<P>` (the generated override's
/// shape — see the header's OVERRIDE CONTRACT), `Thrift.Fact` by `&Fact`,
/// and `m p` by `Result<P, DecodeError>` (the header's Mapping choices).
/// The source's `Type (KeyType p)` / `Type (ValueType p)` superclasses
/// (via `Predicate p`) are the two where clauses — the landing
/// predicate.rs's header names for this unit.
///
/// Body order, mirroring `mkFact fid <$> (Just <$> decodeWithCache … k)
/// <*> (Just <$> case constantRtsValue of …)` (Fact.hs:36-40) exactly:
/// the KEY decode runs first, then the value arm — `Just val -> return
/// val` short-circuits with no decode effects, `Nothing` decodes `v` —
/// and the pure `mk_fact fid (Just key) (Just value)` applies last
/// (probe-cited in the header; the `<$>`/`<*>` chain is left-to-right in
/// both the generic Monad-derived Applicative and IO).
#[inline] // {-# INLINE decodeFact #-} (Fact.hs:35) — see Mapping choices.
pub fn decode_fact<P>(
    serialized: &HashMap<i64, Fact>,
    cache_ref: &mut HashMap<i64, Box<dyn Any>>,
    fid: &IdOf<P>,
    fact: &Fact,
) -> Result<P, DecodeError>
where
    P: Predicate,
    P::KeyType: Type<P::KeyType>,
    P::ValueType: Type<P::ValueType>,
{
    // (Thrift.Fact _pid k v)  (Fact.hs:36): field 1 (`type`, the source's
    // _pid) is ignored via `..`; k/v bind as the two Value references.
    let Fact { key: k, value: v, .. } = fact;
    // Just <$> decodeWithCache serialized cache decodeRtsValue k
    // (Fact.hs:37) — `decodeRtsValue` at `KeyType p`.
    let key = decode_with_cache(
        serialized,
        cache_ref,
        &<P::KeyType as Type<P::KeyType>>::decode_rts_value(),
        k,
    )?;
    // Just <$> case constantRtsValue of  (Fact.hs:38-40) — the
    // Just-shortcut: `constantRtsValue` is a total pure class method
    // (Binary.hs:62-64), so evaluating it here has no effects; its
    // evaluation point relative to the key decode is unobservable and
    // this order (after the key decode) is the source's own forcing
    // point (the case is forced only when the value arm runs).
    let value = match <P::ValueType as Type<P::ValueType>>::constant_rts_value() {
        // Just val -> return val
        Some(val) => val,
        // Nothing -> decodeWithCache serialized cache decodeRtsValue v
        None => decode_with_cache(
            serialized,
            cache_ref,
            &<P::ValueType as Type<P::ValueType>>::decode_rts_value(),
            v,
        )?,
    };
    // mkFact fid (Just key) (Just value) — the pure application, after
    // both decodes. The Haskell shared the immutable `fid` in for free;
    // the port's one reconstruction reads the Copy payload out and
    // rewraps (id.rs keeps IdOf itself non-Copy by design).
    Ok(P::mk_fact(IdOf::new(fid.id_of()), Some(key), Some(value)))
}

// ---------------------------------------------------------------------------
// decodeRef (Fact.hs:42-66)
// ---------------------------------------------------------------------------

/// Decode a fact reference inside a fact (source doc, Fact.hs:42-46):
///
/// * If the nested fact is in the cache, then return it
/// * If the serialized fact is available, we decode it and add it to the
///   cache
/// * Otherwise, we simply construct the empty fact with `justId`.
///
/// `decodeRef :: forall p. (Predicate p, Typeable p) => Decoder p`
/// (Fact.hs:48-50). `Typeable p` needs no bound (header, Mapping
/// choices); the KeyType/ValueType where clauses are decode_fact's (the
/// source gets them from `Predicate p`'s superclasses). The `P: Clone`
/// bound is port-added — see the TODO below.
pub fn decode_ref<P>() -> Decoder<P>
where
    P: Predicate + Clone,
    P::KeyType: Type<P::KeyType>,
    P::ValueType: Type<P::ValueType>,
{
    // TODO(port): the `P: Clone` bound is port-added; the source needed
    // only `Typeable p` because Haskell shares the cached fact freely
    // (fromDynamic returns the shared value; the miss arm caches `toDyn
    // f` and returns the same `f`). The port's cache OWNS its
    // `Box<dyn Any>` copy, so every duplication is a real clone — the
    // build.rs `Decoder::pure` precedent (build.rs:285-291) for exactly
    // this sharing-vs-Clone situation. The only `Predicate` impls are
    // the generated per-schema records (Gen/Haskell.hs:327-328) — plain
    // data in the Haskell, where sharing made every field freely
    // copyable. Scratch-rustc-verified ripple: a `#[derive(Clone)]` on
    // a record embedding an `IdOf` field needs `IdOf<P>: Clone`, which
    // the shipped id.rs deliberately WITHHOLDS (id.rs:98-101, following
    // the source's deriving list). If the generated Rust records keep
    // embedded IdOf fields, one of three cross-unit resolutions applies:
    // id.rs revisits its no-Clone ruling; the generated records
    // manual-impl Clone via the Copy-Fid reconstruction; or this bound
    // is revisited (e.g. an Arc-in-cache pin — which would change the
    // SHIPPED `DecoderEnv.cache_ref` image, build.rs:198). Not this
    // file's to take.
    Decoder::new(|env: &mut DecoderEnv<'_>| {
        // (fid :: IdOf p) <- runDecoder decodeRtsValue env  (Fact.hs:52)
        // — decodeRtsValue at `IdOf p`: the `impl Type<IdOf<P>> for
        // IdOf<P>` instance (binary.rs:706-731), which pops the fact id.
        let fid: IdOf<P> = <IdOf<P> as Type<IdOf<P>>>::decode_rts_value().run_decoder(env)?;
        // cache <- liftIO $ readIORef cacheRef  (Fact.hs:53) — the
        // snapshot read: the lookup below sees the cache's state here,
        // before any of decode_fact's nested insertions.
        // let id = fromIntegral (fromFid (idOf fid))  (Fact.hs:54)
        let id = fid.id_of().0;
        // case IntMap.lookup id cache of  (Fact.hs:55-58)
        match env.cache_ref.get(&id) {
            Some(dyn_) => match dyn_.downcast_ref::<P>() {
                // Just dyn | Just p <- fromDynamic dyn -> return p
                // (Fact.hs:56-57): the port's `p.clone()` is the one
                // duplication the Haskell's sharing performed for free.
                Some(p) => Ok(p.clone()),
                // | otherwise -> liftIO $ throwIO $ ErrorCall
                //   "decodeRef: wrong type"  (Fact.hs:58) — the channel
                // build.rs pinned in advance for this exact site
                // (build.rs:357-359); Display is the raw message.
                None => Err(DecodeError::ErrorCall("decodeRef: wrong type".to_string())),
            },
            None => {
                // case IntMap.lookup id serialized of  (Fact.hs:59-60)
                match env.serialized.get(&id) {
                    // Nothing -> return (justId fid)  (Fact.hs:61) —
                    // the empty fact (mkFact fid Nothing Nothing,
                    // predicate.rs:336-338).
                    None => Ok(just_id(fid)),
                    Some(fact) => {
                        // f <- decodeFact serialized cacheRef fid fact
                        // (Fact.hs:63) — NOTE the source passes the
                        // IORef (the live cell), not the line-53
                        // snapshot, so decode_fact's nested
                        // decodeWithCache calls mutate the same cache
                        // this insert then extends.
                        let f = decode_fact(env.serialized, env.cache_ref, &fid, fact)?;
                        // liftIO $ modifyIORef' cacheRef $ \cache ->
                        //   IntMap.insert id (toDyn f) cache
                        // (Fact.hs:64-65) — an in-place plain overwrite
                        // into the CURRENT contents (post-nested-
                        // mutations), the new value forced at insert
                        // time by the strict application modifyIORef'
                        // performs; the boxed clone is the port's image
                        // of `toDyn` sharing `f` with the cache.
                        env.cache_ref.insert(id, Box::new(f.clone()));
                        // return f  (Fact.hs:66)
                        Ok(f)
                    }
                }
            }
        }
    })
}

// Coverage: every export of the Haskell module has a Rust counterpart —
// the export list is exactly `decodeFact, decodeRef` (Fact.hs:10-11):
// decodeFact -> `decode_fact` (:pub, #[inline]); decodeRef ->
// `decode_ref` (:pub). Everything else in the source is private and this
// file adds NO pub item beyond the two — there is no scaffolding at all;
// both bodies inline every helper against the shipped sibling surfaces.
//
// Marker breakdown (closed set — one open port todo, no source-defect
// reproductions, no performance deferrals, no inventory-bug-candidates):
//   * the one open port todo (at `decode_ref`): the port-added `P:
//     Clone` bound — the sharing-vs-owning gap between Haskell's
//     `Dynamic` cache and the shipped `Box<dyn Any>` cache pin;
//     generated predicate records must derive Clone, and the fallback
//     (an Arc-in-cache pin) is recorded at the site as a cross-unit
//     decision.
//   * no source-defect reproductions: the source's only partial
//     behaviors on this path — the wrong-type `ErrorCall` (:58, carried
//     through the pre-pinned channel) and thriftEnum's partial
//     `toThriftEnum` (which lives inside already-shipped decode units
//     and panics there with its own message) — are faithful behaviors
//     of this file's or siblings' channels, not defects to flag.
//   * no performance deferrals: the clone-per-cache-interaction is the
//     faithful image of the source's sharing (a faster Arc-based cache
//     would change the shipped DecoderEnv image and is rejected at the
//     todo, not deferred); both lookups are the maps' own `get`s; the
//     id "reconstruction" is a Copy field read.
//   * no inventory-bug-candidates: no row governs this unit (see the
//     INVENTORY block); the row-43 key-type flag that cites this
//     file's Fact.hs:54 belongs to build.rs and is followed as pinned.
//
// SAFETY count: 0 unsafe constructs — this unit performs no FFI of its
// own; DecoderEnv's raw pointers (begin/end) are read only by the
// shipped pop shims inside the consumed decoders, and this file touches
// just `serialized`/`cache_ref`. RULE §3's escape hatch is unused, per
// the glean-typed non-decoder precedent (predicate.rs/id.rs/
// build_fact.rs carry none either).
//
// PORT STATUS: confidence=high todos=1
