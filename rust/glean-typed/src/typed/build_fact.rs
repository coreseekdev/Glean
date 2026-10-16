// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// BuildFact — the typed fact-building effect (the NewFact class) and the
// `Facts` collection it builds into.
//
// Ported from `glean/typed/Glean/Typed/BuildFact.hs` (213 lines).
//
// Manifest unit: rust/glean-typed/src/typed/build_fact.rs
// (migration/manifest.tsv line 34, crate glean-typed). RULE §4: one source
// file -> one target file; the manifest path is authoritative.
//
// INVENTORY (RULE §5) — grep "BuildFact" across all six
// migration/inventory*.tsv returns NO row with this file as its col1
// subject; the path occurs only inside OTHER rows' construct/evidence
// columns, in one row duplicated across the two sweeps:
//   * inventory.tsv row 118 (line 118; == inventory.stm.tsv line 31), col1
//     glean/write/Glean/Write/Async.hs, whose target_translation rules on
//     THIS file's state representation: "Facts' two IORef maps
//     (BuildFact.hs:113-114) become plain fields of Facts under the writer
//     Mutex (single-threaded per writer, NOTE at Async.hs:111-112)".
//     FOLLOWED EXACTLY: `factsOwnership`/`factsDerivations`
//     (BuildFact.hs:98-99) are PLAIN fields of `Facts` — no RefCell, no
//     per-field interior-mutability cell — and every mutation flows
//     through `&mut` receivers (`FactsM<'_>` holds `&'a mut Facts`;
//     `extend_facts` takes `&mut Facts`; `build_facts` owns the `Facts`
//     for the bracket). The ReaderT-over-shared-Facts-with-IORefs shape
//     becomes exclusive-borrow plumbing, which is exactly what the row's
//     Mutex<Facts> future (the Write/Async.rs port, inbound edge
//     edges.tsv:1863) needs. No divergence, no INVENTORY-BUG-CANDIDATE.
//     One FORWARD tension the row exposes is flagged as a port-todo at
//     the `Facts` struct (the Send bound), not diverged from.
//   No other row governs this unit; the source is the spec (RULE §0).
//
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, module-level): outbound —
// rows 1834 (-> glean/hs/Glean/RTS/Builder.hs), 1835 (-> RTS/Foreign/
// Define.hs), 1836 (-> RTS/Foreign/FactSet.hs), 1837 (-> RTS/Foreign/
// Lookup.hs), 1838 (-> RTS/Types.hs), 1839 (-> Typed/Binary.hs), 1840
// (-> Typed/Id.hs), 1841 (-> Typed/Predicate.hs), plus the thrift import
// `qualified Glean.Types as Thrift` (BuildFact.hs:50) whose Rust home is
// the generated `glean_thrift` crate. Edge 1837 is the source's UNUSED
// import `Glean.RTS.Foreign.Lookup` (BuildFact.hs:45 — no Lookup-class
// binding is used anywhere in the 213 lines; `defineFact` needs only
// `CanDefine`, which Define.hs owns), so it dissolves with nothing
// consumed. Inbound — rows 1819 (Glean/Typed.hs re-exports this module's
// surface) and 1863 (write/Glean/Write/Async.hs, the row-118 consumer).
// External packages (BuildFact.hs:26-39), dispositions:
//   Data.Coerce -> dissolved (newtype reads are `.0`/accessors);
//   Data.HashMap.Strict -> std::collections::HashMap (binary.rs:146
//     precedent; both thrift maps this file touches carry
//     hs.type = "HashMap", glean.thrift:1345-1346 — see ASSUMED-IMPORT);
//   Data.Int (Int64) -> i64 (LP64);
//   Data.IORef -> plain fields (inventory row 118, above);
//   Data.Maybe (fromMaybe) -> Option::unwrap_or_else;
//   Data.Vector.Storable (Vector.fromList) -> collect into Vec (the
//     VectorStorable unboxing token dissolves the same way the adjacent
//     Prim one did — id.rs:102-104 pins the Data.Vector.Primitive
//     dissolution for `Vec<PidOf<P>>`; Rust's monomorphic Vec needs
//     neither class);
//   Control.Monad (void, when) -> .map(|_| ()), if;
//   Control.Monad.Fail / MonadIO / MonadTrans / ReaderT -> the error enum
//     + the `&mut`-carrier mapping below.
//
// ASSUMED-IMPORT (RULE §4 B4) — RESOLVED against the shipped sibling, then
// EXTENDED: the first sketch of `glean_thrift::Batch` is OWNED by
// rust/glean-core/src/rts/foreign/fact_set.rs:177-199 (define.rs:208-226
// has the same sketch; this file does NOT fork a second owner — it
// cross-references and extends, the typed/binary.rs -> typed/build.rs
// `from_thrift_enum` precedent). Shipped pin, reused: Batch fields
// `first_id`/`count`/`facts`/`ids` (fact_set.rs:192-199; the shipped
// draft's single touch of those four fields is the `Ok(Batch { … })`
// CONSTRUCTION at fact_set.rs:701-711 — field-init syntax, no dotted
// reads exist anywhere in that file) —
// none of those four are touched here beyond passing `serialize()`'s
// Batch through. THIS unit's extension — the two map fields and the
// dependency struct that serializeFacts fills (BuildFact.hs:123-127), all
// against glean/if/glean.thrift (the pinned schema):
//     // glean.thrift:74 `typedef bytestring UnitName`; :51 `typedef string
//     // (hs.type = "ByteString") bytestring` — the Haskell image is
//     // ByteString, the Rust image Vec<u8> (RULE §1 strings row). If the
//     // generated crate ships a `UnitName` alias, only the three use
//     // sites below rename.
//     // glean.thrift:32 `typedef list<Id> (hs.type = "VectorStorable")
//     // listOfIds` — Haskell Data.Vector.Storable.Vector Id, Rust Vec<i64>.
//     // glean.thrift:1346 `typedef map<UnitName, listOfIds> (hs.type =
//     // "HashMap") map_UnitName_listOfIds` -> field 5 `owned`
//     // (glean.thrift:198):
//     pub owned: HashMap<Vec<u8>, Vec<i64>>,
//     // glean.thrift:1345 `typedef map<Id, list<FactDependencies>>
//     // (hs.type = "HashMap") map_Id_list_FactDependencies_964` -> field 6
//     // `dependencies` (glean.thrift:208) — Id-KEYED (the source's
//     // `fromPid` feeds the key, BuildFact.hs:125):
//     pub dependencies: HashMap<i64, Vec<FactDependencies>>,
//     // glean.thrift:153-156 `struct FactDependencies { 1: listOfIds
//     // facts; 2: listOfIds dependencies; }`:
//     pub struct FactDependencies {
//         pub facts: Vec<i64>,
//         pub dependencies: Vec<i64>,
//     }
// Map kind: both typedefs pin the Haskell image to Data.HashMap.Strict
// via the `hs.type = "HashMap"` annotation (the default thrift map image
// is Data.Map — predicate.rs's SchemaInfo block documents that converse;
// these two fields are explicitly annotated), so the Rust image is
// std::collections::HashMap (binary.rs's Data.HashMap.Strict pin), NOT
// BTreeMap. `Id` is the plain thrift typedef `pub type Id = i64`
// (glean.thrift:31; the assumed shape pinned at rts/types.rs:50-51 and
// re-exported at :101) — a type synonym with NO newtype layer, so the
// Haskell's `coerce [Fid] -> [Id]` and `fromFid :: Fid -> Int64` both
// read through Fid's SINGLE Rust newtype layer: `fid.0` IS the i64
// (Fid(pub Id), rts/types.rs:153-155).
//
// SIBLING-UNIT CONTRACTS (RULE §4; DONE crates'/units' real APIs read,
// never re-declared):
//   * glean_core::rts::builder (DONE, read): `pub fn with_builder<R>(f:
//     impl FnOnce(&mut Builder) -> R) -> R` (builder.rs:229) and `pub fn
//     size_of_builder(builder: &Builder) -> usize` (builder.rs:318) —
//     the bracket + size snapshot of newFact (BuildFact.hs:151-153). The
//     rts submodules are NOT re-exported at the rts root (binary.rs:154
//     precedent): full paths.
//   * glean_core::rts::foreign::define (DONE, read): `pub fn define_fact(
//     facts: &impl CanDefine, pid: Pid, clause: &Builder, key_size: usize)
//     -> Result<Fid, InvalidRedefinition>` (define.rs:531-536) and
//     `InvalidRedefinition(pub String)` (define.rs:384). FactSet
//     implements CanDefine (fact_set.rs:497-508), so `&FactSet` is the
//     first argument. The sentinel exception of Define.hs:59-60 arrives
//     as this Err; the invoke-Err channel is aborted inside define.rs
//     (its INVENTORY-BUG-CANDIDATE row 65) and never reaches this file.
//   * glean_core::rts::foreign::fact_set (DONE, read): `FactSet` (the
//     owned handle, fact_set.rs:415-420) with `pub fn new(next_id: Fid)
//     -> Result<FactSet, FfiError>` — a FREE function in the module
//     (fact_set.rs:551; the unit brief's `FactSet::new` spelling — an
//     associated fn would need an impl-block home the shipped file does
//     not give it), consumed here as `fact_set::new`. The `&self`-
//     receiver surface — `first_free_id(&self) -> Fid` (:641),
//     `fact_memory(&self) -> usize` (:588), `serialize(&self) ->
//     Result<Batch, FfiError>` (:725) — is consumed in METHOD form,
//     matching those `&self` receivers exactly (`facts.facts_data.
//     first_free_id()` etc.). Its ASSUMED-IMPORT block (:177-199) owns
//     the Batch sketch extended above.
//   * glean_core::rts::types (DONE, read): `Fid(pub Id)` (:153-155),
//     `Pid(pub i64)` (:120-122), `lowest_fid() -> Fid` (:171-174 =
//     `Fid fIRST_FREE_ID`, Types.hs:82-83), both Copy + Eq + Ord (so
//     `last_id > first_id` is the derived comparison the Haskell's
//     newtype-derived Ord gave).
//   * crate::typed::predicate (DONE, read): `pub trait Predicate: Sized +
//     'static + Type<Self>` with `type KeyType; type ValueType;` and
//     `mk_fact(id: IdOf<Self>, key: Option<Self::KeyType>, value:
//     Option<Self::ValueType>) -> Self` (predicate.rs:268-299);
//     `pub trait HasPredicates { fn get_pid<P: Predicate>(&self) ->
//     PidOf<P>; }` (:380-382) implemented for `Predicates` (:389-413) —
//     the impl PANICS `MissingPredicate` when the pid is absent (:411,
//     the pinned shipped behavior; this file CALLS it and never
//     re-renders the panic). `Predicates(Vec<Pid>)` (:374) keeps its
//     field PRIVATE (the source exports the type abstractly) — consumed
//     here only via `get_pid`. The class-superclass constraints
//     `Type (KeyType p)` / `Type (ValueType p)` (Predicate.hs:64) are
//     inexpressible as Rust supertraits and land as WHERE CLAUSES at the
//     serializing consumer sites — predicate.rs's header NAMES this
//     unit as that consumer (edges.tsv:1841); `new_fact` below is that
//     landing.
//   * crate::typed::id (DONE, read): `PidOf<P>::new(pid)` /
//     `.pid_of() -> Pid` (id.rs:151-161), `IdOf<P>::new(id)` /
//     `.id_of() -> Fid` (id.rs:205-215); `Fid`/`Pid` re-exported at
//     id.rs:124 (this file imports them from their glean-core home —
//     same items).
//   * crate::typed::binary (DONE, read): `pub trait Type<A: 'static>`
//     with `fn build_rts_value(&self, builder: &mut Builder)` — "an Err
//     aborts" on that channel (binary.rs:235-239; `impl Type<()> for ()`
//     at :390 is what `make_fact`'s `ValueType = ()` bound resolves to).
//     build_rts_value returning `()` means new_fact's body has NO error
//     contribution from the two encodes — only define_fact's Err.
//   * glean_core::ffi: `FfiError` (ffi.rs:316-320) — the error channel of
//     fact_set::new / FactSet::serialize, joined into BuildFactsError.
//
// ---------------------------------------------------------------------------
// Mapping choices for the effect type (RULE §0: the old code is the spec)
// ---------------------------------------------------------------------------
//   * `class (MonadFail m, Monad m) => NewFact m` (BuildFact.hs:57) — the
//     class exists "to avoid needing 'error'" (:56): builder code fails
//     through MonadFail instead of `error`. Rust has no monads; the class
//     becomes a TRAIT over the effect carrier with `&mut self` receivers,
//     and `m a` becomes `Result<A, Self::Error>`:
//       - `type Error` is the monad's failure channel. For the one
//         in-tree instance it is `NewFactError` (below), whose two arms
//         enumerate exactly the channels a FactsM action can raise:
//         define_fact's `InvalidRedefinition` (BuildFact.hs:155 ->
//         Define.hs:59-60's throwIO) and the MonadFail `fail` (the class
//         constraint; a String message). Everything else on the path is
//         pinned NON-Result by the shipped siblings: get_pid PANICS
//         MissingPredicate (predicate.rs:411), buildRtsValue ABORTS on
//         its channel (binary.rs:237-239), first_free_id/fact_memory are
//         channel-free totals (fact_set.rs:641/:588).
//       - the methods keep their source arities and orders; `withUnit ::
//         Thrift.UnitName -> m a -> m a`'s continuation becomes the
//         `build: impl FnOnce(&mut Self) -> Result<A, Self::Error>`
//         parameter (the nested action over the SAME carrier — the
//         source's `a <- runFactsM build`).
//   * `FactsM` (BuildFact.hs:145, `ReaderT Facts IO`) — NOT exported by
//     the source, and kept abstract here: the port's carrier is
//     `FactsM<'a> { facts: &'a mut Facts }` with a PRIVATE field and no
//     pub constructor; the only places that open the bracket are
//     `build_facts`/`extend_facts` in this file. The source's own NOTE
//     (:143-144, "very specifically not an instance of MonadIO or
//     MonadTrans as we want to restrict side effects just to fact
//     creation") is preserved BY CONSTRUCTION: a `&mut FactsM` exposes
//     nothing but the NewFact methods — there is no IO surface, no
//     serialization, no read of the maps. (The pub NAME is a Rust
//     signature artifact — the FactBuilder alias and the taker bounds
//     must spell the type; the abstraction the source kept by not
//     exporting FactsM is kept by the private field + private
//     constructor.)
//   * `type FactBuilder = forall m. NewFact m => m ()` (BuildFact.hs:183)
//     — the rank-2 quantification's Rust image is threefold: (a) the
//     `NewFact` trait stays OPEN (a future instance implements it
//     independently); (b) every in-tree INSTANTIATION site — the source
//     instantiates only at FactsM (runFactsM, BuildFact.hs:196/:200) —
//     maps to the generic closure parameter `B: FnOnce(&mut FactsM<'_>)
//     -> Result<(), NewFactError>` on build_facts/extend_facts/
//     build_batch (elision makes it higher-ranked over both the &mut and
//     FactsM's inner borrow, `for<'x, 'm> FnOnce(&'x mut FactsM<'m>)`);
//     (c) a named alias is provided for consumers needing an owned or
//     erased builder value (the future Write/Async port's `writeFacts`,
//     the test helpers' `forall m. NewFact m => m ()` parameters —
//     TestDB.hs:114, Glean/Database/Test.hs:184). A polymorphic builder
//     VALUE (e.g. AngleIndexer/Builder.hs:51's `buildFacts ... ->
//     FactBuilder`) maps to a generic fn over the carrier, not a value.
//   * `Facts` (BuildFact.hs:95-100) — exported WITHOUT `(..)`, so the
//     port's four fields are PRIVATE; `new_facts`/`serialize_facts`/
//     `facts_memory`/`extend_facts`/`build_facts` are the only surface.
//   * Error renderings: `NewFactError::Fail`'s Display is
//     `user error ({msg})` — the pinned GHC 9.4.7 renders IO's `fail`
//     (ioError . userError, lifted through the GND MonadFail of
//     ReaderT) exactly so; verified with a scratch GHC probe
//     (`fail "boom" :: IO ()` -> `user error (boom)`).
//
// RULE §1 (third-party deps row): `thiserror` (error derive) is
// whitelisted; `HashMap` is std.
//
// MODULE-WIRING: `pub mod build_fact;` — deferred to the crate-root unit
// (cf. id.rs/build.rs's MODULE-WIRING notes).

use std::collections::HashMap;

use glean_core::ffi::FfiError;
use glean_core::rts::builder::{size_of_builder, with_builder};
use glean_core::rts::foreign::define::{define_fact, InvalidRedefinition};
use glean_core::rts::foreign::fact_set::{self, FactSet};
use glean_core::rts::types::{lowest_fid, Fid, Pid};
use glean_thrift::{Batch, FactDependencies};

use crate::typed::binary::Type;
use crate::typed::id::{IdOf, PidOf};
use crate::typed::predicate::{HasPredicates, Predicate, Predicates};

// ---------------------------------------------------------------------------
// The NewFact class (BuildFact.hs:54-65)
// ---------------------------------------------------------------------------

/// The failure channel of a [`NewFact`] carrier — the Rust join of the
/// exception sources a FactsM action can raise (the build.rs `DecodeError`
/// precedent: the source's open IO channel, enumerated):
///
/// * [`NewFactError::InvalidRedefinition`] — `defineFact` at
///   BuildFact.hs:155-159 throws `InvalidRedefinition "invalid fact
///   redefinition"` (Define.hs:59-60) when the fact set already holds the
///   same key with a different value; the shipped Rust sibling returns it
///   as `Err` (define.rs:531-536) and this channel propagates it as `Err`
///   up through `new_fact`/`build_facts` — the source's uncaught-exception
///   propagation made explicit.
/// * [`NewFactError::Fail`] — the class's `MonadFail` constraint
///   (BuildFact.hs:56-57, "Include 'MonadFail' constraint to avoid needing
///   'error'"): builder code's `fail msg`. The derived MonadFail (GND
///   through `ReaderT Facts IO`, BuildFact.hs:146) lands in IO's
///   `fail = ioError . userError`, whose rendering is `user error (msg)`
///   (verified against the pinned GHC 9.4.7) — kept byte-for-byte in the
///   Display.
///
/// NOT channels (pinned by the shipped siblings, never Err here):
/// `get_pid`'s MissingPredicate is a panic (predicate.rs:411),
/// `build_rts_value`'s failure is an abort (binary.rs:237-239), and
/// `first_free_id`/`fact_memory` have no error channel (fact_set.rs:641/
/// :588).
#[derive(Debug, thiserror::Error)]
pub enum NewFactError {
    /// `InvalidRedefinition` from `defineFact` (Define.hs:59-60), the
    /// same-key-different-value redefinition guard.
    #[error(transparent)]
    InvalidRedefinition(#[from] InvalidRedefinition),
    /// The MonadFail channel: builder-code `fail` with its message.
    #[error("user error ({0})")]
    Fail(String),
}

/// Class of monads which support creation of new typed facts (source doc,
/// BuildFact.hs:54) — "Include 'MonadFail' constraint to avoid needing
/// 'error'" (:56).
///
/// Rust shape (header, Mapping choices): the monad `m` becomes the
/// `&mut self` carrier; `m a` becomes `Result<A, Self::Error>`; the class
/// stays open for future instances. The three methods keep their source
/// orders and effects (see the impl below for the order-sensitive bodies).
pub trait NewFact {
    /// The carrier's failure channel — the MonadFail half of the source's
    /// superclass constraint, plus whatever exceptions the concrete
    /// instance's internals throw (FactsM adds `InvalidRedefinition`).
    type Error;

    /// `newFact :: Predicate p => KeyType p -> ValueType p -> m (IdOf p)`
    /// (BuildFact.hs:59) — create a new fact with the given key and value.
    ///
    /// The key/value are borrowed: the Haskell passed them by value but
    /// consumed them exactly once (two `buildRtsValue`s); [`make_fact_v`]
    /// is the one caller that needs them again afterwards, and it keeps
    /// ownership while lending the borrow here.
    fn new_fact<P>(
        &mut self,
        key: &P::KeyType,
        value: &P::ValueType,
    ) -> Result<IdOf<P>, Self::Error>
    where
        P: Predicate,
        P::KeyType: Type<P::KeyType>,
        P::ValueType: Type<P::ValueType>;

    /// `withUnit :: Thrift.UnitName -> m a -> m a` (BuildFact.hs:62) —
    /// create some facts owned by the given UnitName. `UnitName` is the
    /// thrift bytestring (glean.thrift:74), image `Vec<u8>`.
    fn with_unit<A>(
        &mut self,
        unit: Vec<u8>,
        build: impl FnOnce(&mut Self) -> Result<A, Self::Error>,
    ) -> Result<A, Self::Error>;

    /// `derivedFrom :: Predicate p => [Fid] -> [p] -> m ()`
    /// (BuildFact.hs:65) — set dependencies of an externally derived fact.
    fn derived_from<P>(&mut self, deps: &[Fid], facts: &[P]) -> Result<(), Self::Error>
    where
        P: Predicate;
}

// ---------------------------------------------------------------------------
// makeFact / makeFact_ / makeFactV / makeFactV_ (BuildFact.hs:67-92)
// ---------------------------------------------------------------------------

/// `makeFactV :: forall p m. (Predicate p, NewFact m) => KeyType p ->
/// ValueType p -> m p` (BuildFact.hs:69-73) — create a new fact and return
/// the corresponding structure which will have 'Just' the passed key and
/// value (source doc, :67-68).
///
/// `(\i -> mkFact i (Just key) (Just value)) <$> newFact key value`: the
/// newFact effect runs FIRST (the `?`), then the pure `mk_fact` — the same
/// order the fmap sequenced. Key/value are OWNED here (the source shares
/// the immutable values between newFact and mkFact; the port moves them
/// into the returned fact after lending them to `new_fact` as borrows).
pub fn make_fact_v<P, NF>(
    nf: &mut NF,
    key: P::KeyType,
    value: P::ValueType,
) -> Result<P, NF::Error>
where
    P: Predicate,
    P::KeyType: Type<P::KeyType>,
    P::ValueType: Type<P::ValueType>,
    NF: NewFact,
{
    let id = nf.new_fact::<P>(&key, &value)?;
    Ok(P::mk_fact(id, Some(key), Some(value)))
}

/// `makeFactV_ :: forall p m. (Predicate p, NewFact m) => KeyType p ->
/// ValueType p -> m ()` (BuildFact.hs:76-79) — create a new fact.
/// `void $ newFact @m @p key value` — the type applications only select
/// the instance; the port's turbofish does the same. Borrowed key/value
/// (consumed once).
pub fn make_fact_v_<P, NF>(
    nf: &mut NF,
    key: &P::KeyType,
    value: &P::ValueType,
) -> Result<(), NF::Error>
where
    P: Predicate,
    P::KeyType: Type<P::KeyType>,
    P::ValueType: Type<P::ValueType>,
    NF: NewFact,
{
    nf.new_fact::<P>(key, value).map(|_| ())
}

/// `makeFact :: forall p m. (Predicate p, ValueType p ~ (), NewFact m) =>
/// KeyType p -> m p` (BuildFact.hs:83-86) — create a new key-only fact and
/// return the corresponding structure which will have 'Just' the passed
/// key (source doc, :81-82). The equality constraint `ValueType p ~ ()`
/// is the associated-type binding below; `impl Type<()> for ()`
/// (binary.rs:390) satisfies the encode bound the helper carries.
pub fn make_fact<P, NF>(nf: &mut NF, key: P::KeyType) -> Result<P, NF::Error>
where
    P: Predicate<ValueType = ()>,
    P::KeyType: Type<P::KeyType>,
    P::ValueType: Type<P::ValueType>,
    NF: NewFact,
{
    make_fact_v::<P, NF>(nf, key, ())
}

/// `makeFact_ :: forall p m. (Predicate p, ValueType p ~ (), NewFact m) =>
/// KeyType p -> m ()` (BuildFact.hs:89-92) — create a new key-only fact.
pub fn make_fact_<P, NF>(nf: &mut NF, key: &P::KeyType) -> Result<(), NF::Error>
where
    P: Predicate<ValueType = ()>,
    P::KeyType: Type<P::KeyType>,
    P::ValueType: Type<P::ValueType>,
    NF: NewFact,
{
    make_fact_v_::<P, NF>(nf, key, &())
}

// ---------------------------------------------------------------------------
// Facts (BuildFact.hs:94-114)
// ---------------------------------------------------------------------------

/// A collection of facts which can be written to and then serialized
/// (source doc, BuildFact.hs:94).
///
/// `data Facts = Facts { factsPredicates :: Predicates, factsData ::
/// FactSet, factsOwnership :: IORef (HashMap Thrift.UnitName [Int64]),
/// factsDerivations :: IORef (HashMap Pid [([Fid],[Fid])]) }`
/// (BuildFact.hs:95-100). Exported by the source WITHOUT `(..)`
/// (BuildFact.hs:16) — the port keeps every field PRIVATE.
///
/// The two IORef fields are PLAIN fields per inventory.tsv row 118
/// ("Facts' two IORef maps ... become plain fields of Facts under the
/// writer Mutex"); all mutation flows through `&mut` (the `FactsM`
/// carrier, `extend_facts`'s `&mut Facts`, `build_facts`'s ownership) —
/// the future Write/Async port puts the whole `Facts` under its writer
/// Mutex, which is why no per-field interior-mutability cell exists.
// TODO(port): row 118's target (`struct Writer { facts: Mutex<Facts>, ...}`,
// inventory.tsv:118) needs `Facts: Send` once the Async unit lands — but
// `Facts` embeds `FactSet`, and the shipped fact_set.rs deliberately
// WITHHOLDS Send/Sync from that handle (fact_set.rs:402-424, the
// conservative C++-mutable-state posture). Resolving this is not this
// unit's to do (this file follows row 118 exactly; the gap is between the
// row's target and the sibling's posture): either fact_set.rs revisits
// its Send withholding for the Mutex-shared case or the Async unit
// reviews its target. Flagged for the Async unit's implementer/reviewers.
pub struct Facts {
    /// `factsPredicates :: Predicates` — the pid map (abstract type,
    /// predicate.rs:374).
    predicates: Predicates,
    /// `factsData :: FactSet` — the owned C++ fact set.
    facts_data: FactSet,
    /// `factsOwnership :: IORef (HashMap Thrift.UnitName [Int64])` —
    /// per-unit inclusive id intervals `[x1,x2, y1,y2, ...]` (the thrift
    /// contract, glean.thrift:189-190). Plain field (row 118).
    facts_ownership: HashMap<Vec<u8>, Vec<i64>>,
    /// `factsDerivations :: IORef (HashMap Pid [([Fid],[Fid])])` — per
    /// pid, the (deps, derived-fact-ids) pairs. Plain field (row 118).
    /// The tuple order is LOAD-BEARING: the FIRST component is the
    /// dependency list, the SECOND the derived facts' ids — see
    /// `to_fact_dependencies` for how they land in the thrift struct.
    facts_derivations: HashMap<Pid, Vec<(Vec<Fid>, Vec<Fid>)>>,
}

/// `newFacts :: Predicates -> Maybe Fid -> IO Facts` (BuildFact.hs:106-114)
/// — create a new empty collection of facts. New facts will be assigned
/// ids >= start id if a start id is supplied; otherwise, they will be
/// assigned ids which are guaranteed not to clash with any pids in the
/// 'Predicates' map (source doc, :102-105).
///
/// `Facts ps <$> FactSet.new (fromMaybe lowestFid start) <*> newIORef
/// HashMap.empty <*> newIORef HashMap.empty` — the single FFI effect is
/// `FactSet::new` (fact_set.rs:551, allocation-guard, returned as `Err`);
/// the two empty-map initializers are field moves (row 118's plain
/// fields). `fromMaybe lowestFid start` is `unwrap_or_else(lowest_fid)`.
pub fn new_facts(ps: Predicates, start: Option<Fid>) -> Result<Facts, FfiError> {
    Ok(Facts {
        predicates: ps,
        facts_data: fact_set::new(start.unwrap_or_else(lowest_fid))?,
        facts_ownership: HashMap::new(),
        facts_derivations: HashMap::new(),
    })
}

// ---------------------------------------------------------------------------
// serializeFacts / factsMemory (BuildFact.hs:116-137)
// ---------------------------------------------------------------------------

/// `toFactDependencies :: ([Fid],[Fid]) -> Thrift.FactDependencies`
/// (BuildFact.hs:129-133, the where-clause helper — private here as
/// there):
///
/// * toFactDependencies (deps, facts) =
/// *   Thrift.FactDependencies
/// *     (Vector.fromList (coerce facts))
/// *     (Vector.fromList (coerce deps))
///
/// SOURCE SUBTLETY, reproduced exactly: the thrift constructor receives
/// the tuple elements SWAPPED relative to their names — field 1 `facts`
/// (glean.thrift:154) is filled from the tuple's SECOND element (the
/// derived facts' ids) and field 2 `dependencies` (:155) from the FIRST
/// (the deps). `coerce [Fid] -> [Id]` is the newtype read: the Rust `Id`
/// is a plain i64 synonym (see ASSUMED-IMPORT), so `f.0` is the whole
/// coercion; `Vector.fromList` is the collecting copy into `Vec<i64>`.
fn to_fact_dependencies(entry: &(Vec<Fid>, Vec<Fid>)) -> FactDependencies {
    let (deps, fact_ids) = entry;
    FactDependencies {
        facts: fact_ids.iter().map(|f| f.0).collect(),
        dependencies: deps.iter().map(|f| f.0).collect(),
    }
}

/// `serializeFacts :: Facts -> IO Thrift.Batch` (BuildFact.hs:117-127) —
/// serialize the facts into a batch which can be sent via Thrift (source
/// doc, :116).
///
/// Effect order preserved: the FactSet serialization (the FFI call, its
/// only fallible step — allocation-guard, `Err(FfiError)`) runs FIRST,
/// then the two map reads (plain field borrows; the source's `readIORef`s
/// land after the serialize in the same do-block), then the record
/// update. `Thrift.batch_owned = fmap Vector.fromList ownership` — each
/// `[Int64]` becomes its `Vec<i64>` image (a copying conversion on BOTH
/// sides; `Vector.fromList` copies too). `Thrift.batch_dependencies =
/// HashMap.fromList [ (fromPid pid, map toFactDependencies deps) | ... ]`
/// — `fromPid` reads the Pid's i64 straight out (Pid(pub i64),
/// rts/types.rs:120-122) for the Id-keyed map (glean.thrift:208/:1345);
/// `HashMap.fromList` and Rust's collecting insert are both last-wins on
/// a duplicate key (the predicate.rs:489-491 note), and iteration order
/// of the source map is arbitrary on both sides (HashMap in both — see
/// ASSUMED-IMPORT), so the collect is faithful.
pub fn serialize_facts(facts: &Facts) -> Result<Batch, FfiError> {
    let mut batch = facts.facts_data.serialize()?;
    batch.owned = facts
        .facts_ownership
        .iter()
        .map(|(unit, ids)| (unit.clone(), ids.clone()))
        .collect();
    batch.dependencies = facts
        .facts_derivations
        .iter()
        .map(|(pid, deps)| (pid.0, deps.iter().map(to_fact_dependencies).collect()))
        .collect();
    Ok(batch)
}

/// `factsMemory :: Facts -> IO Int` (BuildFact.hs:136-137) — return a
/// rough estimate of how much memory is used by the facts (source doc,
/// :135). `FactSet.factMemory . factsData`; the shipped sibling pinned
/// the return at `usize` (fact_set.rs:588, the A5 size_t row — the
/// Haskell's `IO Int` from `fromIntegral CSize` was a signed
/// reinterpretation the sibling documents as intentionally not
/// reproduced), consumed as pinned.
pub fn facts_memory(facts: &Facts) -> usize {
    facts.facts_data.fact_memory()
}

// ---------------------------------------------------------------------------
// FactsM, the NewFact instance (BuildFact.hs:139-180)
// ---------------------------------------------------------------------------

/// `newtype FactsM a = FactsM { runFactsM :: ReaderT Facts IO a }`
/// (BuildFact.hs:145), `deriving(Functor,Applicative,Monad,MonadFail)`
/// (:146) — a monad for creating fact batches (source doc, :139).
///
/// "We expose 'MonadFail' to avoid needing 'error'. NOTE: This is very
/// specifically not an instance of MonadIO or MonadTrans as we want to
/// restrict side effects just to fact creation." (source comment,
/// BuildFact.hs:141-144 — quoted verbatim.)
///
/// Rust image (header, Mapping choices): the ReaderT's environment is the
/// `&'a mut Facts` field (row 118's plain-fields ruling turns the
/// shared-IORef mutation into exclusive-borrow mutation; the source's
/// `ask` is the field read). NOT in the source's export list: the field
/// is private and there is no pub constructor — the bracket is opened
/// only by `build_facts`/`extend_facts` in this file, and a `&mut
/// FactsM` exposes exactly the three NewFact methods — the MonadIO/
/// MonadTrans restriction, preserved by construction.
pub struct FactsM<'a> {
    /// `ask` — the Facts this monad reads/mutates. Private (the source
    /// does not export FactsM; see the struct doc).
    facts: &'a mut Facts,
}

impl<'a> NewFact for FactsM<'a> {
    type Error = NewFactError;

    /// `newFact key value = FactsM $ do { facts <- ask; lift $
    /// withBuilder $ \builder -> do { buildRtsValue builder key;
    /// key_size <- sizeOfBuilder builder; buildRtsValue builder value;
    /// mk facts $ \pid -> IdOf <$> defineFact (factsData facts) (pidOf
    /// pid) builder key_size } }` (BuildFact.hs:149-162).
    ///
    /// Effect order (each step observable before the next):
    ///   1. `buildRtsValue builder key` — the key encoded (abort-class,
    ///      binary.rs:237-239);
    ///   2. `key_size <- sizeOfBuilder builder` — the boundary snapshot
    ///      taken BEFORE the value is appended (so define_fact splits
    ///      key from value at the right offset — the whole point of the
    ///      interleaving);
    ///   3. `buildRtsValue builder value`;
    ///   4. `getPid` — the pid lookup, INSIDE the withBuilder bracket and
    ///      AFTER both encodes (the source's `mk` applies it as the 4th
    ///      statement). It PANICS `MissingPredicate` when the predicate
    ///      is absent (predicate.rs:411, the pinned shipped panic —
    ///      called, not duplicated); with_builder's Drop releases the
    ///      builder on that unwind, exactly as the source's bracket did;
    ///   5. `defineFact ... key_size` — the C++ define (CanDefine over
    ///      the FactSet), the only Err-producing step
    ///      (`InvalidRedefinition`).
    ///
    /// The source's `where mk facts f = f $ getPid (factsPredicates
    /// facts)` helper (:160-162) exists only to ascribe the type at the
    /// application; it inlines to the direct `get_pid` call here.
    fn new_fact<P>(
        &mut self,
        key: &P::KeyType,
        value: &P::ValueType,
    ) -> Result<IdOf<P>, NewFactError>
    where
        P: Predicate,
        P::KeyType: Type<P::KeyType>,
        P::ValueType: Type<P::ValueType>,
    {
        with_builder(|builder| {
            key.build_rts_value(builder);
            let key_size = size_of_builder(builder);
            value.build_rts_value(builder);
            let pid: PidOf<P> = self.facts.predicates.get_pid::<P>();
            let id = define_fact(&self.facts.facts_data, pid.pid_of(), builder, key_size)?;
            Ok(IdOf::new(id))
        })
    }

    /// `withUnit unit build = FactsM $ do { Facts{..} <- ask; firstId <-
    /// liftIO $ firstFreeId factsData; a <- runFactsM build; lastId <-
    /// liftIO $ firstFreeId factsData; when (lastId > firstId) $ liftIO $
    /// modifyIORef' factsOwnership $ HashMap.insertWith (++) unit
    /// [fromFid firstId, fromFid lastId - 1]; return a }`
    /// (BuildFact.hs:164-173).
    ///
    /// Effect order preserved: firstFreeId read, the nested builder run
    /// (over the SAME Facts — the `&mut *self` reborrow is the source's
    /// `runFactsM build` against the same `ask`), firstFreeId read again,
    /// then the conditional insert, then the value returned. On a failed
    /// nested builder (`?`), NO ownership entry is recorded — facts
    /// created before the failure remain in the FactSet but unowned,
    /// exactly the source's exception path (the insert never runs).
    ///
    /// `HashMap.insertWith (++) unit [a, b]`: the combining function
    /// receives (NEW, OLD) — `new ++ old` — so the fresh 2-element
    /// interval is PREPENDED to any existing entry (probe-verified with
    /// the pinned GHC 9.4.7 against unordered-containers-0.2.20 from the
    /// baseline store: `insertWith (++) "k" [3,4] (old [1,2])` yields
    /// `[3,4,1,2]`); `entry().and_modify(v => v.splice(0..0, ..
    /// )).or_insert_with(..)` reproduces that front-insert.
    /// `[fromFid firstId, fromFid lastId - 1]` — the ids of the facts the
    /// nested builder created (firstId inclusive .. lastId-1 inclusive;
    /// `fromFid` is the single-newtype `.0` read, `fromFid lastId - 1`
    /// parses as `(fromFid lastId) - 1` — function application binds
    /// tighter).
    fn with_unit<A>(
        &mut self,
        unit: Vec<u8>,
        build: impl FnOnce(&mut Self) -> Result<A, NewFactError>,
    ) -> Result<A, NewFactError> {
        let first_id = self.facts.facts_data.first_free_id();
        let a = build(&mut *self)?;
        let last_id = self.facts.facts_data.first_free_id();
        if last_id > first_id {
            let interval = [first_id.0, last_id.0 - 1];
            self.facts
                .facts_ownership
                .entry(unit)
                .and_modify(|v| {
                    v.splice(0..0, interval);
                })
                .or_insert_with(|| interval.to_vec());
        }
        Ok(a)
    }

    /// `derivedFrom deps facts = FactsM $ do { Facts{..} <- ask; let pid
    /// = pidOf (getPid factsPredicates :: PidOf p); liftIO $
    /// modifyIORef' factsDerivations $ HashMap.insertWith (<>) pid
    /// [(deps, map (idOf . getId) facts)] }` (BuildFact.hs:175-180).
    ///
    /// `getPid` (panic-parity as above) then the single map mutation;
    /// `map (idOf . getId) facts` is the pure `get_id().id_of()` map
    /// (evaluated before the insert here, lazily-before-forced in the
    /// source — unobservable, `getId` is total). `insertWith (<>)`
    /// combines as `new <> old` — `[new] ++ old` (same probe as
    /// with_unit's): the fresh `(deps, ids)` pair is PREPENDED to the
    /// pid's existing list.
    fn derived_from<P>(&mut self, deps: &[Fid], facts: &[P]) -> Result<(), NewFactError>
    where
        P: Predicate,
    {
        let pid: PidOf<P> = self.facts.predicates.get_pid::<P>();
        // PERF(port): `deps.to_vec()` copies the caller's list where the
        // source stored the SHARED immutable list by pointer (lazily).
        // The map must own its keys' data under row 118's plain-fields
        // ruling, and borrowing the caller's slice into the map would
        // give the Facts a lifetime parameter; the copy is the price.
        // The fast version (taking `Vec<Fid>` by value and moving it in)
        // changes the shared-use ergonomics of the source's callers
        // (CxxTargetUses.hs:99 passes `coerce deps` and keeps using
        // `deps`) and was rejected for this draft.
        let entry: (Vec<Fid>, Vec<Fid>) = (
            deps.to_vec(),
            facts.iter().map(|f| f.get_id().id_of()).collect(),
        );
        self.facts
            .facts_derivations
            .entry(pid.pid_of())
            .and_modify(|v| v.insert(0, entry.clone()))
            .or_insert_with(|| vec![entry]);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// FactBuilder / buildFacts / extendFacts / buildBatch (BuildFact.hs:182-213)
// ---------------------------------------------------------------------------

/// `type FactBuilder = forall m. NewFact m => m ()` (BuildFact.hs:183) —
/// a fact builder.
///
/// Rust image (header, Mapping choices): the rank-2 forall becomes (a)
/// the open `NewFact` trait, (b) the generic closure parameter
/// `B: FnOnce(&mut FactsM<'_>) -> Result<(), NewFactError>` on
/// [`build_facts`]/[`extend_facts`]/[`build_batch`] (every in-tree
/// instantiation of the source's FactBuilder is at FactsM — `runFactsM`
/// at BuildFact.hs:196/:200; elision makes the bound higher-ranked over
/// both lifetimes), and (c) THIS named alias for consumers that need an
/// owned/erased builder value — the `writeFacts`-style takers in the
/// future Write/Async port and the test helpers' `forall m. NewFact m =>
/// m ()` parameters (TestDB.hs:114, Glean/Database/Test.hs:184,
/// SendAndRebaseQueueTest.hs:47). The restriction the forall bought —
/// "fact ids created while running 'FactBuilder' are local to one call
/// and cannot be saved or reused later" (Async.hs:213-216) — holds: the
/// erased value carries no Facts, no ids, and no IO surface.
// TODO(port): the rank-2 TYPE is only partially recovered: a polymorphic
// builder VALUE (a `FactBuilder` stored and later instantiated at an
// as-yet-unwritten second NewFact instance — e.g. AngleIndexer/Builder.
// hs:51 returns `FactBuilder` as a rank-2 value) must be written as a
// generic fn over the carrier (or erased against this alias, pinning
// FactsM) in the port. No such value exists in-tree today (the one
// instance is FactsM); Step-4 consolidation should confirm the lang-
// indexer units can express their builders as generic fns over
// `NF: NewFact` before this image is frozen.
pub type FactBuilder<'a> = dyn for<'m> FnOnce(&'a mut FactsM<'m>) -> Result<(), NewFactError> + 'a;

/// Errors of [`build_facts`] and [`build_batch`] — the join of the two
/// exception sources the source functions had in IO: the `FactSet::new`
/// (and, for build_batch, `serialize`) FFI channel (`FfiError`) and the
/// builder's FactsM channel (`NewFactError`). Same merge-enum precedent
/// as the shipped `RenameFactsError` (fact_set.rs:900-910).
#[derive(Debug, thiserror::Error)]
pub enum BuildFactsError {
    /// The `fact_set::new` / `FactSet::serialize` channel
    /// (allocation-guard, returned; fact_set.rs:551/:725).
    #[error(transparent)]
    Ffi(#[from] FfiError),
    /// The builder's channel: `InvalidRedefinition` from newFact, or the
    /// builder code's own `fail`.
    #[error(transparent)]
    NewFact(#[from] NewFactError),
}

/// `buildFacts :: Predicates -> Maybe Fid -> FactBuilder -> IO Facts`
/// (BuildFact.hs:189-197) — run a fact builder to produce facts based on
/// the supplied 'Predicates' map (source doc, :185-188; the id-assignment
/// contract is `new_facts`'s).
///
/// `facts <- newFacts ps start; runReaderT (runFactsM builder) facts;
/// return facts` — create first, run the builder against it, then return
/// the Facts (owned, post-bracket — the ids live in it).
pub fn build_facts<B>(
    ps: Predicates,
    start: Option<Fid>,
    builder: B,
) -> Result<Facts, BuildFactsError>
where
    B: FnOnce(&mut FactsM<'_>) -> Result<(), NewFactError>,
{
    let mut facts = new_facts(ps, start)?;
    {
        let mut nf = FactsM { facts: &mut facts };
        builder(&mut nf)?;
    }
    Ok(facts)
}

/// `extendFacts :: Facts -> FactBuilder -> IO ()` (BuildFact.hs:199-200)
/// — `runReaderT (runFactsM builder) facts`. The source's shared `Facts`
/// becomes the `&mut Facts` receiver: row 118's plain-fields ruling means
/// the builder mutates the maps through the exclusive borrow (the future
/// Async port holds the writer's Mutex across this call).
pub fn extend_facts<B>(facts: &mut Facts, builder: B) -> Result<(), NewFactError>
where
    B: FnOnce(&mut FactsM<'_>) -> Result<(), NewFactError>,
{
    let mut nf = FactsM { facts: &mut *facts };
    builder(&mut nf)
}

/// `buildBatch :: Predicates -> Maybe Fid -> FactBuilder -> IO
/// Thrift.Batch` (BuildFact.hs:207-213) — run a fact builder and produce
/// a batch of facts based on the supplied 'Predicates' map (source doc,
/// :202-206). `serializeFacts =<< buildFacts ps start builder` —
/// buildFacts' effects first, then the serialization, both channels
/// joined in [`BuildFactsError`].
pub fn build_batch<B>(
    ps: Predicates,
    start: Option<Fid>,
    builder: B,
) -> Result<Batch, BuildFactsError>
where
    B: FnOnce(&mut FactsM<'_>) -> Result<(), NewFactError>,
{
    Ok(serialize_facts(&build_facts(ps, start, builder)?)?)
}

// Coverage: every export of the Haskell module (BuildFact.hs:10-24) has a
// Rust counterpart, and nothing is pub beyond it plus the two documented
// port-added channels —
//   * NewFact(newFact,withUnit,derivedFrom) (:11, all three methods) ->
//     the `NewFact` trait: new_fact (:59), with_unit (:62), derived_from
//     (:65), each with the source's constraints as where-clauses (the
//     Predicate.hs superclass constraints land here — predicate.rs's
//     header names this unit as that consumer).
//   * makeFact (:12) -> `make_fact` (the `ValueType p ~ ()` equality is
//     `Predicate<ValueType = ()>`); makeFact_ (:13) -> `make_fact_`;
//     makeFactV (:14) -> `make_fact_v`; makeFactV_ (:15) ->
//     `make_fact_v_`.
//   * Facts (:16, exported ABSTRACT — no `(..)`) -> `Facts` with four
//     PRIVATE fields.
//   * newFacts (:17) -> `new_facts`; serializeFacts (:18) ->
//     `serialize_facts` (+ the private `to_fact_dependencies`, the
//     source's where-clause helper :129-133); factsMemory (:19) ->
//     `facts_memory`.
//   * FactBuilder (:20) -> the `FactBuilder` alias + the generic builder
//     parameters on the three takers (header, Mapping choices).
//   * buildFacts (:21) -> `build_facts`; extendFacts (:22) ->
//     `extend_facts`; buildBatch (:23) -> `build_batch`.
// Port-added scaffolding with no source export: `NewFactError` /
// `BuildFactsError` (the typed exception channels, RULE §2
// dynamic-exceptions row — the build.rs `DecodeError` precedent), and
// `FactsM` itself (pub NAME for the alias/taker signatures, private
// field, no pub constructor — the source keeps the type internal by not
// exporting it; see its doc). The unused Lookup import
// (BuildFact.hs:45) is not mirrored.
//
// Marker breakdown (closed set — two open port todos, no source-defect
// reproductions, one performance deferral, no inventory-bug-candidates):
//   * the two open port todos: (1) the Send gap row 118's Mutex<Facts>
//     target opens against fact_set.rs's withheld-Send FactSet (at the
//     Facts struct — this file follows the row; the resolution belongs
//     to the sibling/Async units); (2) the rank-2 FactBuilder image's
//     partial recovery for polymorphic builder VALUES (at the alias).
//   * no source-defect reproductions: the one candidate —
//     toFactDependencies' swapped-looking tuple naming — is deliberate
//     field routing, not a defect, and is reproduced exactly (field 1
//     from the tuple's second element) with the mechanism documented at
//     the helper.
//   * one performance deferral: derived_from's `deps.to_vec()` (and the
//     entry clone on the append path) where the source shared the
//     caller's immutable list — the fast by-value shape and why it was
//     rejected are recorded at the site.
//   * no inventory-bug-candidates: row 118 is followed exactly; no other
//     row names this file.
//
// SAFETY count: 0 unsafe constructs — this unit performs no FFI of its
// own (every C++ crossing rides the shipped glean-core wrappers
// with_builder/define_fact/FactSet, each owning its own SAFETY story);
// RULE §3's escape hatch is unused, per the glean-typed non-decoder
// precedent (predicate.rs/id.rs carry none either).
//
// PORT STATUS: confidence=medium todos=2
