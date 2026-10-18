// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// Ported from: glean/db/Glean/Database/Catalog/Filter.hs (166 lines)
// Manifest unit: rust/glean-db/src/database/catalog/filter.rs
// (migration/manifest.tsv line 49, crate glean-db). RULE §4: one source
// file -> one target file; the `database/catalog/` subdirectory is
// generated from the module path `Glean.Database.Catalog.Filter`.
//
// WHAT THIS FILE IS: the Catalog's in-memory query language. A `Filter a`
// is a strict State [Item] a value (Filter.hs:110-114) built from field
// selectors (`Value a` = Item -> a, :87), predicates (:120-147), grouping
// (:150-154), sorting (:160-162) and limiting (:165-166). The combinators
// form a tiny Functor/Applicative/Monad DSL (:111); `runFilter` (:113)
// executes it. The source is explicitly "very much work in progress"
// (:9-13) and is used by the in-memory Catalog and the Backup/Janitor
// selection paths.
//
// ---------------------------------------------------------------------------
// INVENTORY (RULE §5; migration/inventory*.tsv)
// ---------------------------------------------------------------------------
//
// NO row's source file is this file. Grep command (run from the repo root):
//
//   grep -n -E 'Catalog/Filter|runFilter|ItemStatus|Locality' \
//       migration/inventory*.tsv
//
// Result: ZERO hits (no output). The Catalog-area rows that do exist name
// DIFFERENT files and therefore cannot govern this unit:
//   * inventory.sites.tsv:215-232 are the STM_GROUP rows for
//     db/Glean/Database/Catalog.hs (its TVars and atomically blocks), and
//     inventory.tsv:21 / inventory.stm.tsv:23 are the same Catalog.hs row;
//   * inventory.tsv:111 / inventory.protocol.tsv:14 are the Util/IO
//     HandleIO LAZY_IO row, whose consumer evidence names
//     db/Glean/Database/Catalog/Local/Files.hs;
//   * no other row's SOURCE FILE is Catalog/Filter.hs. The remaining
//     "Catalog" hits are consumer evidence in other files' rows (e.g.
//     Create.hs, Backup.hs, Open.hs, List.hs, CompletePredicates.hs,
//     Write/Batch.hs, Write/Queue.hs, Query/UserQuery.hs) and cannot
//     govern this unit.
// No INVENTORY-BUG-CANDIDATE applies.
//
// ---------------------------------------------------------------------------
// DEPENDENCY TRUTH (migration/depmap/edges.tsv)
// ---------------------------------------------------------------------------
//
// INBOUND consumers (edges.tsv:145/171/259/293/319/359/1609) — none is
// edited by this unit:
//   * Backup.hs:58 imports the module; uses statusV (:122/:135/:145),
//     repoV (:130/:134/:161/:531), backedUpV (:146), sortF (:162/:174),
//     limitF (:163/:175) and groupF (:173), with `head`-selection of the
//     result at :123/:132/:137;
//   * Catalog.hs:78 imports; runFilter at :410 and groupF/sortF/limitF at
//     :427-430 (listMostRecent);
//   * Delete.hs:33, Janitor.hs:51, Retention.hs:34 import the module;
//   * Open.hs:52 imports only `Locality(..)`;
//   * Server/Sharding.hs:37 imports it qualified as Catalog.
//
// OUTBOUND (edges.tsv:176 -> glean/db/Glean/Database/Meta.hs):
//   * manifest row 48, SHIPPED at rust/glean-db/src/database/meta.rs.
//     This file consumes exactly the SIBLING-UNIT CONTRACTS listed below.
//   * generated thrift types (glean_thrift), see ASSUMED-IMPORT.
//
// ---------------------------------------------------------------------------
// ASSUMED-IMPORT (RULE §4 B4): glean_thrift
// ---------------------------------------------------------------------------
//
// The source imports `Glean.Types (Repo(..))` and `qualified Glean.Types as
// Thrift` (Filter.hs:53-54). Per RULE §1 (A10) these are the build.rs
// images of glean/if/glean.thrift, never hand-translated. One spelling is
// used here, matching the shipped convention:
//
//   use glean_thrift as thrift;      // meta.rs:519, backend/types.rs:361
//
// Members this file depends on, with pinned evidence:
//
//   pub struct Repo { pub name: String, pub hash: String }
//     // glean.thrift:106-109; gen-hs2 data Repo{repo_name, repo_hash};
//     // the Haskell `repo_name` accessor maps to `.name`. Derives
//     // Clone/Debug/PartialEq/Eq/Hash per backend/types.rs:170-176.
//
//   pub type PosixEpochTime = i64;
//     // glean.thrift:49 `typedef i64 PosixEpochTime (hs.newtype)`;
//     // the generated Rust image is an alias (meta.rs:129-130), so
//     // createdV's value is a plain i64.
//
//   pub enum DatabaseStatus { ... }
//     // glean.thrift:270; generated as a fieldless Rust enum. Only
//     // statusV/queryableF pass it through, so no variant is named here.
//
// ---------------------------------------------------------------------------
// ASSUMED-IMPORT (RULE §4 B4): crate::internal::types::Meta
// ---------------------------------------------------------------------------
//
// `Glean.Database.Meta` exports `Meta(..)` and this unit needs two fields
// plus one function. Both are re-exported/defined by the shipped sibling
// meta.rs, so the import goes THROUGH that unit (one owner; no second
// sketch of the generated type):
//
//   use crate::database::meta::{completeness_status, Meta};
//
// The generated home `crate::internal::types` is the assumed-import pin
// recorded in meta.rs:195-277 (DECISIONS 2026-10-05); it is not on disk yet
// and only the import path changes when Step 4 creates it.
//
// ---------------------------------------------------------------------------
// SIBLING-UNIT CONTRACTS (read from rust/glean-db/src/database/meta.rs)
// ---------------------------------------------------------------------------
//
//   pub use crate::internal::types::Meta;            // meta.rs:529
//   pub fn completeness_status(meta: &Meta) -> thrift::DatabaseStatus;
//                                                    // meta.rs:628
//   Meta fields used here (meta.rs:250-262, IDL order):
//     pub meta_created: PosixEpochTime,              // :252 (i64 alias)
//     pub meta_backup: Option<String>,               // :254
//   Meta derives Clone/Debug/Default/PartialEq/Eq (meta.rs:249-250), which is
//   what lets `Item` derive Debug and lets the selectors return fields.
//
// ---------------------------------------------------------------------------
// EXPORT MAPPING (Filter.hs:16-38, source order, 23 names)
// ---------------------------------------------------------------------------
//
//   Locality(..)          -> pub enum Locality { Local, Restoring, Cloud }
//   ItemStatus(..)        -> pub enum ItemStatus { ItemComplete, ... }
//   Filter                -> pub struct Filter<A>   (field private)
//   Item(..)              -> pub struct Item        (fields public)
//   Value                 -> pub struct Value<A>    (field private)
//   Order(..)             -> pub enum Order { Ascending, Descending }
//   everythingF           -> everything_f()
//   queryableF            -> queryable_f()
//   incompleteQueryableF  -> incomplete_queryable_f()
//   (.==.)                -> eq_f                    (operator image)
//   inF                   -> in_f
//   notInF                -> not_in_f
//   filterF               -> filter_f
//   groupF                -> group_f
//   sortF                 -> sort_f
//   limitF                -> limit_f
//   repoV                 -> REPO_V
//   localityV             -> LOCALITY_V
//   repoNameV             -> REPO_NAME_V
//   createdV              -> CREATED_V
//   statusV               -> STATUS_V
//   backedUpV             -> BACKED_UP_V
//   runFilter             -> run_filter
//
// PRIVATE, as in the source (not exported, :104/:116):
//   entryStatusV          -> ENTRY_STATUS_V
//   modify                -> modify
//
// ABSTRACTION (RULE §4, export lists cut both ways): `Filter` and `Value`
// are exported WITHOUT `(..)` in the source, so their Rust fields and
// constructors are private; the only public ways to build them are the
// combinators and the public `Value` constants. `Item` and the three
// enums are exported WITH `(..)`, so their fields/variants are public.
//
// ---------------------------------------------------------------------------
// FIDELITY NOTES (representation decisions and their evidence)
// ---------------------------------------------------------------------------
//
// * Value: `newtype Value a = Value (Item -> a)` (:87). The source
//   constructor is hidden, and every in-tree `Value` is a field selector
//   (:89-108), so the faithful image is a plain fn pointer
//   `pub struct Value<A>(fn(&Item) -> A)`. Clone/Copy are hand-written
//   with NO `A: Clone/Copy` bounds: the field is a fn pointer, which is
//   Copy for every `A`, and this preserves Haskell-style reuse of
//   REPO_V/STATUS_V across many filters. `get` is private, mirroring the
//   hidden constructor.
//
// * Filter: `newtype Filter a = Filter (S.State [Item] a)` (:110). A
//   State value is a reusable pure function `[Item] -> (a, [Item])`, so
//   the image is `Arc<dyn Fn(Vec<Item>) -> (A, Vec<Item>)>` (the
//   reusable-handle shape of the Decoder precedent, typed/build.rs:208-300).
//   `map`/`and_then`/`pure` are the Functor/Monad/Applicative surface;
//   the free `ap` mirrors `(<*>)`. `pure` needs `A: Clone` because the
//   closure is `Fn` and must produce a fresh value on every run. `Clone`
//   is an Arc handle copy, so a Filter may be run repeatedly.
//
// * runFilter: `execState` (:113-114) discards the final value, hence
//   `run_filter(f: Filter<()>, items) -> Vec<Item>`. By value is the direct
//   image of function application; callers that reuse a filter clone it
//   (cheap, above).
//
// * modify: `S.modify'` (:116-117) is strict in the new state. Rust is
//   strict already, so `modify` is just the state transformation applied
//   when run; there is no separate forcing step and no divergence.
//
// * groupF (:150-154): `fromListWith (++)` PREPENDS each item, so the
//   intermediate per-group list is reverse input order, and the source's
//   `reverse` restores input order before running the inner filter. The
//   port pushes into a Vec in input order and does NOT reverse — the two
//   operations cancel, so per-group order matches exactly. Cross-group
//   order is `HashMap.elems` order in Haskell and HashMap iteration order
//   in Rust; both are hash-dependent and the exact Haskell order is not
//   reproducible without reimplementing Data.Hashable/HAMT. The single
//   canonical port todo at the iteration site records that divergence and
//   audits the order-sensitive consumers.
//
// * sortF (:160-162): `sortOn` is a STABLE decorate-sort-undecorate: it
//   computes each key ONCE per element, then sorts the decorated pairs.
//   `sort_by_cached_key` is the direct image (stable, key computed once
//   per element); `Down . f` is `Reverse(v.get(item))` (std::cmp::Reverse
//   is the Down image). Equal keys keep input order in both directions.
//
// * limitF (:165-166): Haskell `take n` returns [] for n <= 0 and the
//   first n elements otherwise; the port clears on n <= 0 and truncates
//   with a checked-positive `n as usize`.
//
// * (.==.) (:138-139) is an OPERATOR in the source; Rust has no infix
//   declaration, so it becomes the free function `eq_f` in the same
//   F-combinator family (filter_f/in_f/not_in_f/...). The argument order is
//   preserved: `Value a -> a -> Filter ()`.
//
// * inF/notInF (:142-147): `HashSet a` is taken BY VALUE
//   (`std::collections::HashSet<A>`), matching the source's by-value
//   argument; membership is `contains(&get(item))`.
//
// * CAFs: everythingF/queryableF/incompleteQueryableF (:123-135) are
//   top-level Haskell values; each Rust call returns a fresh cheap handle
//   (`pub fn ...() -> Filter<()>`). REPO_V, LOCALITY_V, REPO_NAME_V,
//   CREATED_V, STATUS_V and BACKED_UP_V are `pub const` values; the
//   private ENTRY_STATUS_V is likewise a const.
//
// * Locality (:58-59): declaration order Local, Restoring, Cloud is the
//   priority Ord the doc comment names (:56-57); deriving Ord preserves it.
//   The Enum and Bounded instances have no in-tree consumer and no Rust
//   trait image; they are omitted rather than faked.
//
// * ItemStatus (:63-76): declaration order is exactly the severity order
//   (ItemComplete < ... < ItemMissing), so derived Ord is the source Ord.
//   Semigroup `max` -> `combine` = Ord::max; Monoid `mempty =
//   ItemComplete` -> `Default`. The Generic instance has no Rust analogue
//   and no consumer; Hashable -> `Hash` (needed by the set combinators).
//
// * Order (:156-157): Enum, Bounded and Read are omitted — grep shows no
//   read/parse of Ascending/Descending anywhere in glean/ and no Rust trait
//   corresponds; Eq/Ord derive, Show -> Debug per the port-wide stand-in.
//
// * Item (:79-84): the source derives only Show; Rust derives only Debug
//   (the port-wide Show stand-in) and adds no Clone/Eq.
//
// * No Send/Sync bounds on Filter. The source value is used within one
//   transaction/thread; if a future unit needs to store a Filter across
//   threads, the missing bound surfaces as a compile error at Step 4
//   rather than being silently promised here.
//
// * Debug on the enums/Item is the canonical Show stand-in (RULE §2
//   deriving-Show row); no Display is added because no consumer displays
//   these types.

use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::sync::Arc;

use glean_thrift as thrift;

use crate::database::meta::{completeness_status, Meta};

// ---------------------------------------------------------------------------
// Locality (Filter.hs:56-59)
// ---------------------------------------------------------------------------

/// `data Locality = Local | Restoring | Cloud` (:58) — "How a DB is
/// available, in priority order (used to resolve conflicts of two dbs with
/// the same name)" (:56-57). The declaration order IS the priority order,
/// so derived Ord is the source Ord.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Locality {
    Local,
    Restoring,
    Cloud,
}

// ---------------------------------------------------------------------------
// ItemStatus (Filter.hs:61-76)
// ---------------------------------------------------------------------------

/// `data ItemStatus` (:63-69) — "The status of a stacked database, ordered
/// by increasing severity. This is DatabaseStatus minus Restorable"
/// (:61-62). Variant order is severity order; derived Ord is the source Ord.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ItemStatus {
    ItemComplete,
    ItemFinalizing,
    ItemIncomplete,
    ItemBroken,
    ItemRestoring,
    ItemMissing,
}

impl ItemStatus {
    /// `instance Semigroup ItemStatus where (<>) = max` (:72-73).
    pub fn combine(self, other: Self) -> Self {
        std::cmp::Ord::max(self, other)
    }
}

impl Default for ItemStatus {
    /// `instance Monoid ItemStatus where mempty = ItemComplete` (:75-76).
    fn default() -> Self {
        ItemStatus::ItemComplete
    }
}

// ---------------------------------------------------------------------------
// Item (Filter.hs:79-84)
// ---------------------------------------------------------------------------

/// `data Item = Item { itemRepo, itemLocality, itemMeta, itemStatus }`
/// (:79-84), deriving Show only. Debug is the port-wide Show stand-in.
#[derive(Debug)]
pub struct Item {
    pub item_repo: thrift::Repo,
    pub item_locality: Locality,
    pub item_meta: Meta,
    pub item_status: ItemStatus,
}

// ---------------------------------------------------------------------------
// Value (Filter.hs:86-108)
// ---------------------------------------------------------------------------

/// `newtype Value a = Value (Item -> a)` (:87) — "A field that can be used
/// in queries". The constructor is NOT exported, so the field is private.
///
/// The field is a plain fn pointer: every in-tree Value is a field
/// selector, and a fn pointer is Copy for every `A`, which lets the
/// constants below be reused freely (the Haskell value is likewise freely
/// reusable).
pub struct Value<A>(fn(&Item) -> A);

impl<A> Clone for Value<A> {
    fn clone(&self) -> Self {
        Value(self.0)
    }
}

impl<A> Copy for Value<A> {}

impl<A> Value<A> {
    /// Apply the selector. Private: the source's constructor is hidden and
    /// only the combinators below read a Value.
    fn get(&self, item: &Item) -> A {
        (self.0)(item)
    }
}

// Selectors. Haskell's selectors return the field directly; Rust returns
// an owned value, so non-Copy fields (Repo, String) are cloned.
fn item_repo(item: &Item) -> thrift::Repo {
    item.item_repo.clone()
}

fn item_repo_name(item: &Item) -> String {
    item.item_repo.name.clone()
}

fn item_locality(item: &Item) -> Locality {
    item.item_locality
}

fn item_created(item: &Item) -> thrift::PosixEpochTime {
    item.item_meta.meta_created
}

fn item_status_value(item: &Item) -> thrift::DatabaseStatus {
    completeness_status(&item.item_meta)
}

fn item_entry_status(item: &Item) -> ItemStatus {
    item.item_status
}

fn item_backed_up(item: &Item) -> bool {
    item.item_meta.meta_backup.is_some()
}

/// `repoV :: Value Repo` (:89-90).
pub const REPO_V: Value<thrift::Repo> = Value(item_repo);

/// `repoNameV :: Value Text` (:92-93) — `repo_name . itemRepo`, i.e. the
/// generated `Repo::name` field.
pub const REPO_NAME_V: Value<String> = Value(item_repo_name);

/// `localityV :: Value Locality` (:95-96).
pub const LOCALITY_V: Value<Locality> = Value(item_locality);

/// `createdV :: Value Thrift.PosixEpochTime` (:98-99) — `metaCreated .
/// itemMeta`.
pub const CREATED_V: Value<thrift::PosixEpochTime> = Value(item_created);

/// `statusV :: Value Thrift.DatabaseStatus` (:101-102) — `completenessStatus
/// . itemMeta`.
pub const STATUS_V: Value<thrift::DatabaseStatus> = Value(item_status_value);

/// `entryStatusV :: Value ItemStatus` (:104-105) — private in the source.
const ENTRY_STATUS_V: Value<ItemStatus> = Value(item_entry_status);

/// `backedUpV :: Value Bool` (:107-108) — `isJust . metaBackup .
/// itemMeta`.
pub const BACKED_UP_V: Value<bool> = Value(item_backed_up);

// ---------------------------------------------------------------------------
// Filter (Filter.hs:110-117)
// ---------------------------------------------------------------------------

/// `newtype Filter a = Filter (S.State [Item] a)` (:110), deriving
/// Functor/Applicative/Monad as a newtype (:111).
///
/// A State value is a reusable pure function `[Item] -> (a, [Item])`; the
/// image is a shared closure handle. The field and `new` are private, so
/// the only ways to obtain a Filter are the combinators below (the source
/// exports the type abstractly).
pub struct Filter<A> {
    run: Arc<dyn Fn(Vec<Item>) -> (A, Vec<Item>)>,
}

impl<A: 'static> Filter<A> {
    /// Private constructor (the hidden `Filter` data constructor).
    fn new<F>(run: F) -> Filter<A>
    where
        F: Fn(Vec<Item>) -> (A, Vec<Item>) + 'static,
    {
        Filter { run: Arc::new(run) }
    }

    /// Private State runner (`runState`); `runFilter` discards the value.
    fn run(&self, items: Vec<Item>) -> (A, Vec<Item>) {
        (self.run)(items)
    }

    /// `fmap` over the State functor (:111, derived Functor).
    pub fn map<B: 'static>(&self, f: impl Fn(A) -> B + 'static) -> Filter<B> {
        let run = Arc::clone(&self.run);
        Filter::new(move |items| {
            let (a, items) = run(items);
            (f(a), items)
        })
    }

    /// `(>>=)` over the State monad (:111, derived Monad): the SAME state
    /// is threaded through both halves.
    pub fn and_then<B: 'static>(&self, k: impl Fn(A) -> Filter<B> + 'static) -> Filter<B> {
        let run = Arc::clone(&self.run);
        Filter::new(move |items| {
            let (a, items) = run(items);
            k(a).run(items)
        })
    }

    /// `pure` (:111, derived Applicative). `A: Clone` because the closure
    /// is `Fn` and each run must produce a fresh value.
    pub fn pure(x: A) -> Filter<A>
    where
        A: Clone,
    {
        Filter::new(move |items| (x.clone(), items))
    }
}

impl<A> Clone for Filter<A> {
    /// Cheap Arc handle copy: a Filter is reusable, exactly like the
    /// source's pure State value.
    fn clone(&self) -> Self {
        Filter {
            run: Arc::clone(&self.run),
        }
    }
}

/// `(<*>) = ap` — the Applicative surface of the derived instance
/// (:111). Kept a FREE function so the argument order matches
/// `mf <*> mx`; the in-tree consumers use only the Monad bind, so this is
/// provided for completeness of the source's instance.
pub fn ap<A, B, F>(mf: &Filter<F>, mx: &Filter<A>) -> Filter<B>
where
    F: Fn(A) -> B + 'static,
    A: 'static,
    B: 'static,
{
    let mf_run = Arc::clone(&mf.run);
    let mx_run = Arc::clone(&mx.run);
    Filter::new(move |items| {
        let (f, items) = mf_run(items);
        let (a, items) = mx_run(items);
        (f(a), items)
    })
}

// ---------------------------------------------------------------------------
// runFilter / modify (Filter.hs:113-117)
// ---------------------------------------------------------------------------

/// `runFilter :: Filter () -> [Item] -> [Item]` (:113-114) —
/// `execState`, i.e. keep only the final state. By-value matches function
/// application; reuse a filter by cloning its cheap handle.
pub fn run_filter(f: Filter<()>, items: Vec<Item>) -> Vec<Item> {
    f.run(items).1
}

/// `modify :: ([Item] -> [Item]) -> Filter ()` (:116-117) —
/// `Filter . S.modify'`. Rust is strict, so the function is applied when
/// the filter runs; there is no separate forcing step to reproduce.
fn modify<F>(f: F) -> Filter<()>
where
    F: Fn(Vec<Item>) -> Vec<Item> + 'static,
{
    Filter::new(move |items| ((), f(items)))
}

// ---------------------------------------------------------------------------
// filterF / everythingF (Filter.hs:119-124)
// ---------------------------------------------------------------------------

/// `filterF :: (Item -> Bool) -> Filter ()` (:120-121) —
/// `modify . filter`. The predicate sees `&Item`, exactly the item the
/// source predicate inspects.
pub fn filter_f(f: impl Fn(&Item) -> bool + 'static) -> Filter<()> {
    modify(move |items| items.into_iter().filter(|q| f(q)).collect())
}

/// `everythingF :: Filter ()` (:123-124) — `return ()`.
pub fn everything_f() -> Filter<()> {
    Filter::pure(())
}

// ---------------------------------------------------------------------------
// queryableF / incompleteQueryableF (Filter.hs:126-135)
// ---------------------------------------------------------------------------

/// `queryableF` (:126-130): entry status in
/// {ItemComplete, ItemIncomplete, ItemFinalizing}. "broken DBs may or may
/// not be queryable" (:130).
pub fn queryable_f() -> Filter<()> {
    in_f(
        ENTRY_STATUS_V,
        HashSet::from([
            ItemStatus::ItemComplete,
            ItemStatus::ItemIncomplete,
            ItemStatus::ItemFinalizing,
        ]),
    )
}

/// `incompleteQueryableF` (:132-135): entry status in
/// {ItemIncomplete, ItemFinalizing}.
pub fn incomplete_queryable_f() -> Filter<()> {
    in_f(
        ENTRY_STATUS_V,
        HashSet::from([ItemStatus::ItemIncomplete, ItemStatus::ItemFinalizing]),
    )
}

// ---------------------------------------------------------------------------
// eqF / inF / notInF (Filter.hs:137-147)
// ---------------------------------------------------------------------------

/// `(.==.) :: Eq a => Value a -> a -> Filter ()` (:138-139) — "Require
/// that a field has a specific value". Rust has no infix operator
/// declaration, so the operator is the free function `eq_f`; argument
/// order is unchanged.
pub fn eq_f<A>(v: Value<A>, x: A) -> Filter<()>
where
    A: Eq + 'static,
{
    filter_f(move |q| v.get(q) == x)
}

/// `inF :: (Eq a, Hashable a) => Value a -> HashSet a -> Filter ()`
/// (:142-143) — "Require that a field's value is in a set". The set is
/// taken by value, as in the source.
pub fn in_f<A>(v: Value<A>, xs: HashSet<A>) -> Filter<()>
where
    A: Eq + Hash + 'static,
{
    filter_f(move |q| xs.contains(&v.get(q)))
}

/// `notInF :: (Eq a, Hashable a) => Value a -> HashSet a -> Filter ()`
/// (:146-147) — the negation of `in_f`.
pub fn not_in_f<A>(v: Value<A>, xs: HashSet<A>) -> Filter<()>
where
    A: Eq + Hash + 'static,
{
    filter_f(move |q| !xs.contains(&v.get(q)))
}

// ---------------------------------------------------------------------------
// groupF (Filter.hs:149-154)
// ---------------------------------------------------------------------------

/// `groupF :: (Eq a, Hashable a) => Value a -> Filter () -> Filter ()`
/// (:150-154) — "Group the data by a field and apply the clauses to each
/// group separately".
///
/// Per-group order: `fromListWith (++)` prepends, then `reverse` restores
/// input order; pushing in input order here performs the same two
/// cancellations, so the inner filter sees exactly the source's list.
///
/// Cross-group order: the source iterates `HashMap.elems`; this iterates
/// `std::collections::HashMap`, whose order is hash-dependent too. The
/// exact Haskell order is not reproducible without reimplementing
/// Data.Hashable/HAMT (see the single port todo below).
pub fn group_f<A>(v: Value<A>, p: Filter<()>) -> Filter<()>
where
    A: Eq + Hash + 'static,
{
    modify(move |qs| {
        let mut groups: HashMap<A, Vec<Item>> = HashMap::new();
        for q in qs {
            let key = v.get(&q);
            groups.entry(key).or_default().push(q);
        }
        let mut out = Vec::new();
        // TODO(port): cross-group order is HashMap iteration order, not the
        // source's Data.HashMap/HAMT order (not reproducible without
        // reimplementing Data.Hashable/HAMT). Per-group order matches. Audit
        // of order-sensitive consumers: Backup.newestByRepo's head selection
        // (Backup.hs:123/:132/:137) and Catalog.listMostRecent
        // (Catalog.hs:427-430); revisit at Step 6.
        for (_key, group) in groups {
            let (_, group) = p.run(group);
            out.extend(group);
        }
        out
    })
}

// ---------------------------------------------------------------------------
// Order / sortF / limitF (Filter.hs:156-166)
// ---------------------------------------------------------------------------

/// `data Order = Ascending | Descending` (:156-157). Enum, Bounded and
/// Read have no in-tree consumer and no Rust trait image (grep finds no
/// reads/parses of the constructors anywhere in glean/), so they are
/// omitted; Eq/Ord derive and Show maps to Debug.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Order {
    Ascending,
    Descending,
}

/// `sortF :: Ord a => Value a -> Order -> Filter ()` (:160-162) — "Sort
/// the data by a field". `sortOn` is a stable decorate-sort-undecorate
/// that computes each key once; `sort_by_cached_key` reproduces that
/// (stable, one key per element). Descending is `sortOn (Down . f)`;
/// `std::cmp::Reverse` is the `Down` image, so ties keep input order.
pub fn sort_f<A>(v: Value<A>, order: Order) -> Filter<()>
where
    A: Ord + 'static,
{
    modify(move |mut items| {
        match order {
            Order::Ascending => items.sort_by_cached_key(|item| v.get(item)),
            Order::Descending => items.sort_by_cached_key(|item| std::cmp::Reverse(v.get(item))),
        }
        items
    })
}

/// `limitF :: Int -> Filter ()` (:165-166) — "Only keep the first n data
/// items". Haskell `take n` is [] for n <= 0; the checked-positive cast
/// preserves that.
pub fn limit_f(n: i64) -> Filter<()> {
    modify(move |mut items| {
        if n <= 0 {
            items.clear();
        } else {
            items.truncate(n as usize);
        }
        items
    })
}

// ---------------------------------------------------------------------------
// Coverage / marker tally (closed set)
// ---------------------------------------------------------------------------
//
// Source symbols: 23 exported + 2 private, all imaged above (see EXPORT
// MAPPING); nothing in the source is dropped except the instances listed in
// FIDELITY NOTES (Locality Enum/Bounded, ItemStatus Generic, Order
// Enum/Bounded/Read).
//
// Markers: one port todo (the cross-group iteration-order divergence and
// its consumer audit, at the group loop). No source-defect reproductions
// (no BUG marker), no performance deferrals (no PERF marker), no unsafe
// code (no SAFETY marker), no inventory divergence (zero governing rows,
// above).
//
// PORT STATUS: confidence=high todos=1
