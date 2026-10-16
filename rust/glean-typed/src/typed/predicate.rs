// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// Predicate — the schema-predicate type class, Pid lookup, and the
// per-process predicate-index CAFs.
//
// Ported from `glean/typed/Glean/Typed/Predicate.hs` (156 lines).
//
// Manifest unit: rust/glean-typed/src/typed/predicate.rs
// (migration/manifest.tsv line 33, crate glean-typed). RULE §4: one
// source file -> one target file; the manifest path is authoritative.
//
// INVENTORY (RULE §5) — unlike its siblings 15/16/17, this file IS
// inventoried; grep "Typed/Predicate" across migration/inventory*.tsv:
//   * inventory.tsv row 108 (`:108`): predicateIndices — "predicateIndex
//     global MVar+IORef CAF", GLOBAL_STATE, target_translation
//     "once_cell Lazy<Mutex<HashMap<PredicateRef,i64>>> + AtomicI64
//     counter; order of allocation must match source (sequential per
//     PredicateRef first-use)". FOLLOWED EXACTLY (see THE TWO CAFs
//     below): Lazy<Mutex<HashMap<...>>> + a const-initialized AtomicI64
//     static, with the fetch_add inside the Mutex critical section so
//     allocation order == first-lookup order. No divergence, no flag.
//   * inventory.tsv row 135 (`:135`): KeyType/ValueType type families,
//     MECHANICAL, target "concrete types per predicate (set is closed):
//     enum-dispatch keyed on PredicateRef; NO generic trait fork — one
//     PredicateData module owns key/value codecs". NOT followed to the
//     letter: the associated-type trait below is not an enum-dispatch.
//     The row-135 divergence flag is raised at the trait itself (RULE
//     §5/A9 — silent divergence is a failed review); this unit's brief
//     rules RULE §2's type-families row governing here.
//   * inventory.sites.tsv rows 19-20 (`:19`, `:20`): unsafepure — the
//     two hidden-effect pure initializers (Predicate.hs:134/:138,
//     RULEBOOK §2.1): dissolved into Lazy initialization and const
//     initialization respectively (§2 unsafePerformIO row's spirit —
//     the impure-in-pure becomes explicit initialization).
//   * inventory.sites.tsv rows 22-23 (`:22`, `:23`): global-caf — the
//     two NOINLINE pragmas (Predicate.hs:132/:136, RULEBOOK §2.2):
//     both CAFs are statics below.
//   * inventory.stm.tsv line 4: the STM shard's header note that it
//     does NOT duplicate the predicateIndex CAF row (it lives in
//     inventory.tsv row 108) — consistent; nothing to follow there.
//
// DEPENDENCY TRUTH (the source's whole import list, Predicate.hs:26-44):
//   - Glean.RTS.Types (invalidPid) — SHIPPED unit 10 (manifest line 11,
//     commit 6abba13):
//     glean_core::rts::types::invalid_pid (rts/types.rs:136-138).
//   - qualified Glean.Schema.Util as Angle — SHIPPED unit 5:
//     glean_angle::schema::util::convert_ref (util.rs:262-269).
//   - Glean.Types (PredicateRef(..), SchemaInfo(..)) — GENERATED
//     glean_thrift (RULE §1 thrift row); ASSUMED-IMPORT blocks below.
//   - Glean.Typed.Binary — SHIPPED unit 17: crate::typed::binary::Type
//     (binary.rs:235) — the supertrait.
//   - Glean.Typed.Id — SHIPPED unit 16: crate::typed::id::{IdOf, PidOf}
//     (id.rs:141-216); `Pid` too arrives through this module's named
//     re-export (id.rs:124 `pub use glean_core::rts::types::{Fid,
//     Pid};`) exactly as the source imports it via Glean.Typed.Id.
//   - qualified Glean.Angle.Types as Angle — SHIPPED unit 3:
//     glean_angle::angle::types::Type_ (PredicateTy at types.rs:749)
//     and SourceTypeWithRefs (types.rs:1087).
//   - External packages: Control.Concurrent (MVar) -> std::sync::Mutex
//     (the CAF mapping); Control.Exception (throw, Exception) ->
//     panic! (§2 dynamic-exceptions + Show-tag rows: no catch site for
//     MissingPredicate exists in this module or its in-tree consumers'
//     use of these functions — it escapes to the top level, and a
//     panic is the port's top-level escape); Data.HashMap.Strict ->
//     std::collections::HashMap (values are i64, so Strict-vs-Lazy is
//     moot — the pinning is HashMap); Data.IORef -> AtomicI64 (row
//     108); Data.Map -> the generated BTreeMap on SchemaInfo (thrift
//     map, ordered; only iterated — Map.toList ascending ==
//     BTreeMap::iter ascending); Data.Proxy -> dissolved (the type
//     rides `Self` on associated fns — the source_type precedent,
//     binary.rs:263-267); Data.Text -> String (RULE §1 strings row);
//     Data.Vector.Primitive -> Vec (Primitive is an unboxing token,
//     dissolved — the id.rs deriving-map disposition, id.rs:102-104);
//     System.IO.Unsafe (unsafeDupablePerformIO, unsafePerformIO) ->
//     Lazy init / const init (see THE TWO CAFs); TextShow (showt) ->
//     signed decimal, i32's Display — the exact ruling util.rs:282-284
//     pins for showt on the thrift Version.
//
// ASSUMED-IMPORT (RULE §4 B4) — PredicateRef: NOT re-sketched here. The
// first sketch is owned by angle/types.rs:52-75:
//     #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
//     pub struct PredicateRef { pub name: String, pub version: Version }
//     pub type Version = i32;   // glean.thrift:39 typedef i32 Version
// (thrift cites glean.thrift:39 and :55-58). This file consumes that
// surface unchanged and adds NO extension: Eq + Hash are exactly what
// HashMap<PredicateRef, _> requires (the Haskell's Data.HashMap needed
// the generated Hashable/Eq instances — same deriving family), Clone
// feeds the map insert, Debug feeds the throw renderings.
//
// ASSUMED-IMPORT (RULE §4 B4, FIRST SKETCH — this file owns it):
// SchemaInfo from the generated crate, consumed only through its
// predicateIds field:
//     #[derive(Clone, Debug, /* ... */)]
//     pub struct SchemaInfo {
//         pub schema: Vec<u8>,   // glean.thrift:989 `1: bytestring schema`
//         pub predicate_ids: BTreeMap<Id, PredicateRef>,
//                                  // glean.thrift:995 `2: map<Id,
//                                  // PredicateRef> predicateIds;`
//         // + fields 3-6: schemaIds (:1000), dbSchemaIds (:1003),
//         // otherSchemaIds (:1007), derivationDependencies (:1010) —
//         // not read by this unit.
//     }
// `Id` is the plain thrift typedef i64 (glean.thrift:31, NO hs.newtype
// — a type synonym, unlike Nat/Byte at :97/:100; the pinned compiler
// maps plain typedefs to synonyms, never images — the binary.rs:60-64
// byte->Int8 lesson's converse) — the same Id glean_core::rts::types
// re-exports (rts/types.rs:101). The Haskell compiler maps thrift map
// to Data.Map (ordered); the Rust image is BTreeMap (the unit pinning:
// ordered map), so `Map.toList`'s ascending iteration ==
// BTreeMap::iter's, and `HashMap.fromList`'s last-wins on a duplicate
// ref is fed the same order on both sides. If the generated crate
// ships different field names, only the one use site in
// make_predicates changes (the angle/types.rs:73-75 convention).
//
// Show/Debug rendering at the throw sites (§2 Show-tag row, A4/B6):
// Haskell's derived Show renders `MissingPredicate PredicateRef
// {predicateRef_name = "x", predicateRef_version = 3}` (the generated
// record's selectors); the port renders `MissingPredicate(PredicateRef
// { name: "x", version: 3 })` via the sketch's #[derive(Debug)] — the
// constructor-shaped TAG is preserved and the deep record-layout drift
// is the documented crate-wide convention (angle/types.rs:77-90),
// cited rather than re-marked per type.
//
// SIBLING-UNIT CONTRACTS (real APIs read, never re-declared):
//   - binary.rs: `pub trait Type<A: 'static>` (:235) — methods
//     build_rts_value (:239), decode_rts_value (:242),
//     constant_rts_value default None (:245), decode_as_fact default
//     (:253), source_type -> SourceTypeWithRefs<()> (:267). THIS
//     file's supertrait; its parameter bound `A: 'static` (plus
//     Rust's implicit `A: Sized` on trait params) forces the explicit
//     `Sized + 'static` supertrait bounds on Predicate below.
//   - id.rs: IdOf<P> (struct :197-200, new :205, id_of :213), PidOf<P>
//     (struct :141-144, new :151, pid_of :159), the Pid re-export
//     (:124).
//   - glean_core::rts::types: Pid(pub i64) (:120-122 — the Haskell
//     wraps Int64 directly, Types.hs:51, NOT the thrift Id);
//     invalid_pid() (:136-138) = Pid(0) (Types.hs:60-61). The rts
//     submodules are NOT re-exported at the rts root (binary.rs:154-
//     160 precedent) — the path is glean_core::rts::types.
//   - glean_angle::schema::util: convert_ref(&PredicateRef) ->
//     SourceRef (util.rs:262-269, Util.hs:81-86 — "guaranteed to have
//     a Version").
//   - glean_angle::angle::types: Type_::PredicateTy(ST, P) (:749) —
//     the payload carries the converted ref (the source writes
//     `Angle.PredicateTy () (Angle.convertRef ...)`, Predicate.hs:83);
//     SourceTypeWithRefs<()> = Type_<(), SourceRef, SourceRef>
//     (:1087) — the source's `SourceType' ()` instantiated at ().
//
// STRUCTURE CHOICE (RULE §2 type-families row; inventory row 135): the
// class keeps ASSOCIATED TYPES (`type KeyType; type ValueType;`) — not
// generic parameters, not concrete per-predicate types. Reasons:
//   (a) the Haskell members are associated type families of the class
//       (a p -> KeyType functional dependency, one impl per predicate);
//       Rust associated types are the direct image — consumers name
//       `P::KeyType`, exactly as the source's consumers name
//       `KeyType p`.
//   (b) It type-checks against the REAL supertrait: `Predicate:
//       Type<Self>` together with the assoc types, the default
//       get_index body, and a method generic over the predicate
//       (get_pid<P>) — verified with a scratch rustc 1.98.1 check
//       written for this unit.
//   (c) The row's "prefer concrete" cannot apply HERE: this file only
//       DECLARES the class; every impl lives in generated per-schema
//       code (the closed set the row cites), so this unit has nothing
//       concrete to declare.
// The source's class-level superclass constraints `Type (KeyType p)`
// and `Type (ValueType p)` (Predicate.hs:64) have NO Rust supertrait
// image — a supertrait cannot mention Self's associated types. They
// become where clauses at the consumer sites that actually serialize
// keys/values — `P::KeyType: Type<P::KeyType>, P::ValueType:
// Type<P::ValueType>` (verified legal on methods/fns in the same
// scratch check) — which is where the source's own consumers
// (Typed/Fact.hs, Typed/BuildFact.hs, both inbound edges of this unit,
// edges.tsv:1841/:1845) use them. The first superclass `Type p` IS a
// supertrait (below).
//
// THE TWO CAFs (inventory row 108; RULE §2 NOINLINE-CAF row,
// Predicate.hs:122-138):
//   predicateIndices :: MVar (HashMap PredicateRef Int)
//     -> static PREDICATE_INDICES: Lazy<Mutex<HashMap<PredicateRef, i64>>>
//   predicateNextIndex :: IORef Int
//     -> static PREDICATE_NEXT_INDEX: AtomicI64 (const 0)
// Equivalence argument, including the unsafeDupablePerformIO at
// Predicate.hs:124: unsafeDupablePerformIO permits the action's
// DUPLICATED concurrent entry — in the Haskell, the MVar serializes
// the whole lookup-or-allocate action, so concurrent duplicate entries
// for one ref produce exactly one allocation (the loser re-looks-up
// under the lock and finds the winner's entry). In the Rust,
// once_cell's Lazy guarantees the map is initialized exactly once and
// Mutex::lock serializes the same critical section with the same
// one-allocation-per-ref outcome — Lazy + Mutex serializes
// identically. The IORef's atomicModifyIORef' (\x -> (x+1, x+1)) is an
// atomic RMW returning the NEW value; the atomic image is
// fetch_add(1) + 1 (SeqCst — GHC's atomicModifyIORef' is a full-barrier
// CAS), preferred over Mutex<i64> per the row's own wording ("+
// AtomicI64 counter"). The RMW runs INSIDE the MVar/Mutex section in
// BOTH sources, so counter increments and map insertions interleave
// identically: allocation order is first-lookup order (row 108's
// requirement). The second CAF's initializer (unsafePerformIO $
// newIORef 0) is a constant effect; Rust const-initializes the static,
// which subsumes lazily initializing a constant. Neither CAF is in the
// source's export list (Predicate.hs:12-24); neither is pub here.

use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Mutex;

use once_cell::sync::Lazy;

use glean_angle::angle::types::{SourceTypeWithRefs, Type_};
use glean_angle::schema::util::convert_ref;
use glean_core::rts::types::invalid_pid;
use glean_thrift::{PredicateRef, SchemaInfo};

use crate::typed::binary::Type;
use crate::typed::id::{IdOf, Pid, PidOf};

// ---------------------------------------------------------------------------
// The Predicate class (Predicate.hs:64-78)
// ---------------------------------------------------------------------------

/// 'Predicate' provides information about the schema's data structures,
/// and convenience functions.
///
/// * Types
///     * `KeyType Foo` : the generated type (often `Foo_key*`) that
///       stores the predicate's fields.
///     * `ValueType Foo` : the generated values type, defaults to `()`.
/// * Methods
///     * `getName Foo` : the `glean.thrift` derived `PredicateRef`
///       holding the Text name and Int version of the predicate.
///     * `getIndex Foo` : internal use, index into all schema predicate
///       list. Used to efficiently lookup `PidOf p` for a particular
///       repo.
///
/// Superclass constraint of `Type` ensures this can be serialized into
/// the underlying binary format. (Source doc, Predicate.hs:48-62.)
///
/// Rust shape (see the header's STRUCTURE CHOICE):
///   - `getName`/`getIndex` took `proxy p`; the Rust associated fns
///     carry the type on `Self` (the binary.rs:263-267 source_type
///     precedent — no Proxy parameter).
///   - the `Sized + 'static` bounds are forced by the real supertrait
///     `Type<A: 'static>` (binary.rs:235), which Rust's implicit
///     parameter `Sized` bound also constrains — they are the direct
///     consequence of the source's `Type p` superclass, not additions.
///   - the source's OTHER two superclass constraints (`Type (KeyType
///     p)`, `Type (ValueType p)`, Predicate.hs:64) are inexpressible as
///     supertraits (they mention Self's associated types) and become
///     consumer where clauses — see the header.
// INVENTORY-BUG-CANDIDATE row 135 (inventory.tsv line 135): the row's
// target_translation prescribes "concrete types per predicate (set is
// closed): enum-dispatch keyed on PredicateRef; NO generic trait fork —
// one PredicateData module owns key/value codecs", which this
// associated-type trait is not. RULE §2's type-families row is the
// governing canonical mapping for this construct and this unit's brief
// rules the associated-type trait the structure-preserving choice when
// it type-checks against the real supertrait (it does — scratch-verified
// against binary.rs:235). The row's "one PredicateData module owns
// key/value codecs" concern belongs to the GENERATED per-schema impl
// units, not this file, which only declares the class exactly as
// Predicate.hs:64-72 does. Divergence flagged here per RULE §5/A9
// rather than applied silently.
pub trait Predicate: Sized + 'static + Type<Self> {
    /// `type KeyType p` (Predicate.hs:65) — the generated key type
    /// (often `Foo_key*`) that stores the predicate's fields.
    type KeyType;

    /// `type ValueType p` (Predicate.hs:66) — the generated values type.
    ///
    /// TODO(port): the source's default `type ValueType p = ()`
    /// (Predicate.hs:67) has no stable-Rust image — associated type
    /// defaults are E0658-unstable (rustc 1.98.1; verified with a
    /// scratch rustc check written for this unit). Every impl
    /// (generated per-schema code) must therefore state
    /// `type ValueType = ();` explicitly — the exact value the default
    /// would have supplied. The alternative shape that recovers a
    /// default (generic parameters, `trait Predicate<K, V = ()>`)
    /// trades impl-site brevity for named-parameter noise at every
    /// consumer bound and loses the "P determines KeyType" reading;
    /// Step-4 consolidation should confirm the associated-type choice
    /// stands.
    type ValueType;

    /// `getName :: proxy p -> PredicateRef` (Predicate.hs:68) — the
    /// `glean.thrift` derived PredicateRef holding the name and version.
    fn get_name() -> PredicateRef;

    /// `mkFact :: IdOf p -> Maybe (KeyType p) -> Maybe (ValueType p) ->
    /// p` (Predicate.hs:69).
    fn mk_fact(
        id: IdOf<Self>,
        key: Option<Self::KeyType>,
        value: Option<Self::ValueType>,
    ) -> Self;

    /// `getId :: p -> IdOf p` (Predicate.hs:70).
    fn get_id(&self) -> IdOf<Self>;

    /// `getFactKey :: p -> Maybe (KeyType p)` (Predicate.hs:71).
    fn get_fact_key(&self) -> Option<Self::KeyType>;

    /// `getFactValue :: p -> Maybe (ValueType p)` (Predicate.hs:72).
    fn get_fact_value(&self) -> Option<Self::ValueType>;

    /// A unique integer per predicate, used for fast lookup of Pids
    /// when writing facts. `getIndex :: proxy p -> PredicateIndex`
    /// with the default body `const (predicateIndex (getName (Proxy ::
    /// Proxy p)))` (Predicate.hs:74-77).
    fn get_index() -> PredicateIndex {
        predicate_index(&Self::get_name())
        // "this should be computed once and cached per predicate type"
        // (Predicate.hs:78) — the cache IS the predicateIndices CAF
        // below: every call of this default goes through the shared
        // map, so each ref's index is computed once and returned from
        // the map afterwards.
    }
}

/// `predicateSourceType :: Predicate p => Proxy p -> Angle.SourceType'
/// ()` (Predicate.hs:81-83) — retrieve the Angle representation of the
/// type: `Angle.PredicateTy () (Angle.convertRef (getName proxy))`.
/// `SourceType' ()` = `SourceTypeWithRefs<()>` (angle/Types.hs:640 at
/// st := (); types.rs:1087), and PredicateTy's payload is the converted
/// ref (Type_::PredicateTy(ST, P), types.rs:749).
pub fn predicate_source_type<P: Predicate>() -> SourceTypeWithRefs<()> {
    Type_::PredicateTy((), convert_ref(&P::get_name()))
}

/// `justId :: Predicate p => IdOf p -> p` (Predicate.hs:85-86) —
/// `justId x = mkFact x Nothing Nothing`.
pub fn just_id<P: Predicate>(x: IdOf<P>) -> P {
    P::mk_fact(x, None, None)
}

/// `newtype MissingPredicate = MissingPredicate PredicateRef
/// deriving(Show)` (Predicate.hs:88) with `instance Exception
/// MissingPredicate` (Predicate.hs:89). Show -> Debug (RULE §2
/// deriving-Show row); the Exception instance's throw sites become the
/// panic renderings (see `throw_missing_predicate`).
#[derive(Debug)]
pub struct MissingPredicate(pub PredicateRef);

/// Useful error for 'getPid' in 'HasPredicates' (source doc,
/// Predicate.hs:91). `throwMissingPredicate :: forall p. Predicate p =>
/// PidOf p` (Predicate.hs:92-93): `throw $ MissingPredicate $ getName
/// (Proxy :: Proxy p)`.
///
/// §2 exceptions/Show-tag rows (A4/B6): the typed throw becomes a
/// panic carrying the constructor-shaped rendering — Haskell's derived
/// Show is `MissingPredicate PredicateRef {predicateRef_name = "...",
/// predicateRef_version = N}`; the port's Debug renders
/// `MissingPredicate(PredicateRef { name: "...", version: N })`
/// (constructor tag preserved; the deep-layout
/// drift is the crate-wide convention, angle/types.rs:77-90, and the
/// assumed Debug shape is the angle/types.rs:52-75 sketch's derive —
/// see the header's ASSUMED-IMPORT blocks).
pub fn throw_missing_predicate<P: Predicate>() -> PidOf<P> {
    panic!("MissingPredicate {:?}", P::get_name())
}

/// `type PredicateIndex = Int` (Predicate.hs:96) — an index into the
/// Predicates vector. Used when writing facts only. NOT in the source's
/// export list (Predicate.hs:12-24), so not pub. Int = i64 (LP64).
type PredicateIndex = i64;

/// `newtype Predicates = Predicates (Vector Pid)` (Predicate.hs:98) —
/// invalidPid for predicates missing from the schema (source comment,
/// Predicate.hs:99). Vector.Primitive.Vector -> Vec.
pub struct Predicates(Vec<Pid>);

/// Class of values `a` that have definitions for any predicate `p`
/// (source doc, Predicate.hs:101). `getPid :: Predicate p => a -> PidOf
/// p` (Predicate.hs:103) — the method is generic over the predicate,
/// exactly as the Rust method is.
pub trait HasPredicates {
    fn get_pid<P: Predicate>(&self) -> PidOf<P>;
}

/// `instance HasPredicates Predicates where getPid = get` with the
/// local `get` (Predicate.hs:105-116) — the exact decision tree:
/// index = getIndex p; pid = if index < length pids then pids ! index
/// else invalidPid; `pid /= invalidPid` -> PidOf pid, else throw
/// MissingPredicate (getName p).
impl HasPredicates for Predicates {
    fn get_pid<P: Predicate>(&self) -> PidOf<P> {
        // where index = getIndex (Proxy :: Proxy p) (Predicate.hs:113)
        let index = P::get_index();
        // pid = if index < Vector.length pids then pids Vector.! index
        //         else invalidPid  (Predicate.hs:114-116) — Int vs Int
        // comparison; the port widens the Vec length to i64 so the
        // comparison stays the source's signed one (index is always
        // >= 1, so the guard is a pure bounds check and `Vector.!`'s
        // partial index is always in range — no bug to reproduce).
        let pid = if index < self.0.len() as i64 {
            self.0[index as usize]
        } else {
            invalid_pid()
        };
        // | pid /= invalidPid = PidOf pid
        // | otherwise = throw $ MissingPredicate $ getName (...)  (:110-111)
        if pid != invalid_pid() {
            PidOf::new(pid)
        } else {
            // same rendering as throw_missing_predicate (the source
            // throws the identical value here, Predicate.hs:111 vs :93)
            panic!("MissingPredicate {:?}", P::get_name())
        }
    }
}

/// `type SchemaPredicates = [PredicateRef]` (Predicate.hs:120) — the
/// type of `allPredicates` generated for each schema. To get this,
/// import the generated module for the schema, e.g. `Glean.Schema.Src`
/// (source doc, Predicate.hs:118-119).
pub type SchemaPredicates = Vec<PredicateRef>;

/// `predicateIndex :: PredicateRef -> Int` (Predicate.hs:122-130) — the
/// unsafeDupablePerformIO body (Predicate.hs:124): look the ref up in
/// the shared map under the lock; on a miss, allocate the next index
/// (atomic RMW, returning the NEW value) and insert. Impure-in-pure ->
/// a plain explicit fn (RULE §2 unsafePerformIO row's spirit). NOT in
/// the source's export list; not pub.
fn predicate_index(ref_: &PredicateRef) -> PredicateIndex {
    // modifyMVar predicateIndices $ \hm -> ...  (Predicate.hs:125) —
    // the lock is held for the whole lookup-or-allocate decision, so
    // the MVar and the Mutex serialize the triple (lookup, increment,
    // insert) identically; allocation order == first-lookup order
    // (inventory row 108).
    // An MVar cannot poison — modifyMVar's cleanup restores it on
    // exception (Predicate.hs:124-130 has no failure path of its own).
    // The Rust image: recover the guard through the poisoning the same
    // way; the map is still consistent because nothing inside the
    // critical section can panic.
    let mut indices = PREDICATE_INDICES
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    match indices.get(ref_) {
        // Just n -> return (hm, n)  (Predicate.hs:130)
        Some(&n) => n,
        // Nothing -> do n <- atomicModifyIORef' predicateNextIndex
        //   (\x -> (x+1, x+1)); return (HashMap.insert ref n hm, n)
        //   (Predicate.hs:126-129)
        None => {
            // atomicModifyIORef' is an atomic RMW — the atomic image.
            // (\x -> (x+1, x+1)) stores AND returns the INCREMENTED
            // value (not the old one), hence fetch_add + 1; SeqCst
            // matches GHC's full-barrier CAS. Runs inside the critical
            // section in both sources (see the header's CAF argument).
            let n = PREDICATE_NEXT_INDEX.fetch_add(1, Ordering::SeqCst) + 1;
            indices.insert(ref_.clone(), n);
            n
        }
    }
}

/// `predicateIndices :: MVar (HashMap PredicateRef Int)`
/// (Predicate.hs:133) — `{-# NOINLINE #-}` CAF initialized by
/// `unsafePerformIO $ newMVar HashMap.empty` (Predicate.hs:132-134).
/// Inventory row 108's target_translation, followed exactly:
/// `once_cell::sync::Lazy<Mutex<HashMap<PredicateRef, i64>>>` (once_cell
/// is whitelisted, RULE §1).
static PREDICATE_INDICES: Lazy<Mutex<HashMap<PredicateRef, PredicateIndex>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// `predicateNextIndex :: IORef Int` (Predicate.hs:137) — the second
/// `{-# NOINLINE #-}` CAF, `unsafePerformIO $ newIORef 0`
/// (Predicate.hs:136-138). Row 108's "+ AtomicI64 counter": the
/// initializer is the constant effect `newIORef 0`, which Rust
/// const-evaluates directly — no Lazy needed, the static IS the cell
/// (the atomic is required because atomicModifyIORef' is an atomic RMW
/// the source performs on this cell; see the header's CAF argument).
static PREDICATE_NEXT_INDEX: AtomicI64 = AtomicI64::new(0);

/// `makePredicates :: [SchemaPredicates] -> SchemaInfo -> Predicates`
/// (Predicate.hs:140-150) — build the per-predicate Pid vector for a
/// repo's schema.
pub fn make_predicates(schemas: &[SchemaPredicates], info: &SchemaInfo) -> Predicates {
    // ref_ids = HashMap.fromList [ (ref, Pid id) | (id,ref) <- Map.toList
    //   (schemaInfo_predicateIds info) ]  (Predicate.hs:149-150).
    //
    // The thrift field is `2: map<Id, PredicateRef> predicateIds`
    // (glean.thrift:995) — Data.Map in Haskell, BTreeMap in the
    // generated crate; `Map.toList` ascending == `BTreeMap::iter`
    // ascending, so a ref stored under two Ids resolves to the same
    // LAST entry on both sides (HashMap.fromList is right-biased;
    // Rust's collect-insert overwrites, also last-wins). `Pid id`
    // (Predicate.hs:150) wraps the MAP KEY (the thrift Id, plain i64 —
    // glean.thrift:31, no hs.newtype) in the rts newtype:
    // `Pid(pub i64)` (rts/types.rs:120-122) — the Haskell Pid wraps
    // Int64 directly (Types.hs:51), so the marshal is the identity
    // `Pid(*id)`.
    let ref_ids: HashMap<&PredicateRef, Pid> = info
        .predicate_ids
        .iter()
        .map(|(id, r)| (r, Pid(*id)))
        .collect();
    // assocs = [ (predicateIndex ref, HashMap.lookupDefault invalidPid
    //   ref ref_ids) | ref <- concat schemas ]  (Predicate.hs:145-148)
    // — NOTE the SIDE EFFECT inside the list comprehension:
    // predicate_index runs per ref, in concat order, before any Pid is
    // placed; keeping that call order preserves the CAF allocation
    // order (row 108). lookupDefault invalidPid -> get().copied()
    // .unwrap_or_else(invalid_pid).
    let assocs: Vec<(PredicateIndex, Pid)> = schemas
        .iter()
        .flatten()
        .map(|r| {
            (
                predicate_index(r),
                ref_ids.get(r).copied().unwrap_or_else(invalid_pid),
            )
        })
        .collect();
    // maxIx = maximum $ map fst assocs  (Predicate.hs:144).
    //
    // BUG(port): `maximum` bottoms on the empty list — and this one is
    // REACHABLE: `schemas` comes from callers (`loadPredicates` passes
    // the caller's list through, Backend/Types.hs:236; a caller with no
    // schemas gives []), and `loadPredicatesForSchema` passes the
    // singleton `[Map.elems (schemaInfo_predicateIds info)]`
    // (Backend/Types.hs:246), whose concat is empty for a DB schema
    // with zero predicates. Reproduced bug-for-bug (RULE §2 BUG rule +
    // partial-functions row): panic with the source's exact message —
    // the same ruling code.rs:489-500 records for GHC.List's maximum.
    // Repro: make_predicates(&[[]], <any SchemaInfo>) — or any schema
    // whose predicateIds map is empty — => assocs == [] => panic
    // "Prelude.maximum: empty list".
    let max_ix = assocs
        .iter()
        .map(|(i, _)| *i)
        .reduce(i64::max)
        .unwrap_or_else(|| panic!("Prelude.maximum: empty list"));
    // (Vector.//) (Vector.replicate (maxIx+1) invalidPid) assocs
    //   (Predicate.hs:141-142): a (maxIx+1)-long invalidPid vector
    // bulk-updated at the assoc indices in list order — LAST write wins
    // on a duplicate index (`concat schemas` can repeat a ref; Vector.//
    // applies left-to-right). The for-loop is the same order semantics.
    // max_ix >= 1 whenever assocs is non-empty (indices are allocated
    // starting at 1 — see predicate_index), so max_ix+1 never overflows
    // and `as usize` is exact on LP64.
    let mut pids: Vec<Pid> = vec![invalid_pid(); (max_ix + 1) as usize];
    for (i, pid) in assocs {
        pids[i as usize] = pid;
    }
    Predicates(pids)
}

/// `predicateRef :: forall p proxy . Predicate p => proxy p -> Text`
/// (Predicate.hs:154-156) — convert a predicate type to a versioned
/// predicate name, useful for building Angle queries that refer to
/// specific versions (source doc, Predicate.hs:152-153).
pub fn predicate_ref<P: Predicate>() -> String {
    // !(PredicateRef name version) = getName (Proxy @p)
    // (Predicate.hs:156) — the bang pattern's strictness is inherent in
    // the by-value destructuring.
    let PredicateRef { name, version } = P::get_name();
    // name <> "." <> showt version  (Predicate.hs:155): showt on the
    // thrift Version (plain i32) is signed decimal — i32's Display,
    // the TextShow ruling util.rs:282-284 pins.
    format!("{}.{}", name, version)
}

// Coverage: every export of the Haskell module (Predicate.hs:12-24) has
// a Rust counterpart, all pub, nothing extra pub —
//   * Predicate(..) (:14) -> the `Predicate` trait: assoc types
//     KeyType/ValueType (:65-66) + methods get_name (:68), mk_fact
//     (:69), get_id (:70), get_fact_key (:71), get_fact_value (:72),
//     get_index (:76, default body :77 kept as the Rust default).
//   * justId (:16) -> `just_id`; predicateRef (:17) ->
//     `predicate_ref`; predicateSourceType (:18) ->
//     `predicate_source_type`.
//   * HasPredicates(..) (:20) -> the `HasPredicates` trait (get_pid,
//     :103) + the instance for Predicates (:105-116) — the source's
//     instance is not an export, but it is ported (`impl
//     HasPredicates for Predicates`).
//   * MissingPredicate(..) (:21) -> `MissingPredicate` (Debug = the
//     Show derivation); throwMissingPredicate (:21) ->
//     `throw_missing_predicate`.
//   * Predicates (:23, exported ABSTRACT — no `(..)`, so the source
//     hides the constructor) -> `Predicates` with a PRIVATE field:
//     constructible only via make_predicates, readable only via
//     getPid — exactly the surface the source module exposes.
//     SchemaPredicates
//     (:23) -> the alias; makePredicates (:23) -> `make_predicates`.
// Port-added private scaffolding with no source export:
// `PredicateIndex` (private alias, mirroring the source's own
// non-exported alias, :96), `predicate_index` (non-exported, :122),
// and the two statics (non-exported, :132-138).
//
// Marker breakdown (closed set — one open port todo, one reproduced
// source defect, zero performance deferrals, one inventory-divergence
// flag):
//   * the open port todo (at the ValueType assoc type): the source
//     default's E0658 instability — generated impls must write
//     `type ValueType = ();` explicitly; the alternative
//     generic-param shape is recorded for Step-4 to confirm or reject.
//   * the reproduced source defect (at make_predicates): the reachable
//     `maximum []` bottom (repro + caller evidence at the site).
//   * no performance deferrals: the CAF holds one Mutex for the whole
//     lookup-or-allocate critical section exactly as the source's
//     modifyMVar did; nothing here is a deferred optimization.
//   * the inventory-divergence flag (row 135, raised at the trait):
//     the associated-type trait diverges from the row's enum-dispatch
//     wording (RULE §2 row + unit brief govern; flagged per §5/A9).
//
// SAFETY count: 0 unsafe constructs — this unit is pure safe Rust; the
// only impurity in the source (unsafeDupablePerformIO/unsafePerformIO)
// mapped to Lazy/const initialization, and there is no FFI here. RULE
// §3's escape hatch is unused.
//
// PORT STATUS: confidence=high todos=1
