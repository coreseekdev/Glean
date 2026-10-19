// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// Ported from: glean/db/Glean/Database/Catalog/Store.hs (37 lines)
// Manifest unit: rust/glean-db/src/database/catalog/store.rs
// (migration/manifest.tsv line 50, crate glean-db). RULE §4: one source
// file -> one target file; the `database/catalog/` subdirectory is
// generated from the module path `Glean.Database.Catalog.Store`.
//
// WHAT THIS FILE IS: the metadata-store interface — the source's
// `class Store a` (Store.hs:19-37) with exactly five methods: `list`
// (:21), `create` (:25), `delete` (:29), `put` (:33) and `get` (:37).
// It is the abstraction the Catalog is parameterized over: the
// existential `data Catalog = forall local. Store local => Catalog`
// (Catalog.hs:133) and Config's `Some Catalog.Store` (Config.hs:118).
// The production instances are Files
// (glean/db/Glean/Database/Catalog/Local/Files.hs:43-99) and Memory
// (glean/db/Glean/Database/Catalog/Local/Memory.hs:27-45); the
// test-only `instance Store MockStore`
// (glean/test/lib/Glean/Database/Catalog/Test.hs:46-51) is TESTS and is
// not translated (RULE §4).
//
// ---------------------------------------------------------------------------
// INVENTORY (RULE §5; migration/inventory*.tsv)
// ---------------------------------------------------------------------------
//
// NO row's source file is this file. Grep command (run from the repo
// root):
//
//   grep -c -E 'Catalog/Store\.hs' \
//       migration/inventory.tsv migration/inventory.sites.tsv \
//       migration/inventory.protocol.tsv migration/inventory.stm.tsv \
//       migration/inventory.codeflow.tsv migration/inventory.ffi.tsv
//
// Result: ZERO hits in every inventory file (exit status 1):
//
//   migration/inventory.tsv:0          migration/inventory.stm.tsv:0
//   migration/inventory.sites.tsv:0    migration/inventory.codeflow.tsv:0
//   migration/inventory.protocol.tsv:0 migration/inventory.ffi.tsv:0
//
// Two near-misses are NOT governing:
//   * migration/inventory.stm.tsv:22 is the STM_GROUP row whose source
//     file is glean/db/Glean/Database/Janitor.hs; its evidence text
//     mentions `Catalog.startRestoring`, but the row names Janitor.hs,
//     a different file.
//   * the `instance Store MockStore` at
//     glean/test/lib/Glean/Database/Catalog/Test.hs:46 is test code
//     (RULE §4: tests are not translated) and has no inventory row.
//
// No INVENTORY-BUG-CANDIDATE applies.
//
// ---------------------------------------------------------------------------
// DEPENDENCY TRUTH (migration/depmap/edges.tsv)
// ---------------------------------------------------------------------------
//
// INBOUND consumers (none edited by this unit):
//   * edges.tsv:172 Catalog.hs -> Store.hs. Imports at Catalog.hs:79
//     (`Store`) and :80 (qualified as `Store`); the existential at
//     :133; the `open` bound at :310; call sites `Store.put` :276,
//     `Store.list` :312, `Store.create` :370 and :550, `Store.delete`
//     :396.
//   * edges.tsv:177 Catalog/Local/Files.hs -> Store.hs. Import :28;
//     instance :43-99 (list :44-67, create :69-75, delete :77-81,
//     put :83-87, get :89-99).
//   * edges.tsv:182 Catalog/Local/Memory.hs -> Store.hs. Import :18;
//     instance :27-45.
//   * edges.tsv:220 Database/Config.hs -> Store.hs. Import :84
//     (qualified as `Catalog`); the `Some Catalog.Store` field at :118.
//
// OUTBOUND (edges.tsv:184 Store.hs -> Database/Meta.hs): manifest row 48,
// SHIPPED at rust/glean-db/src/database/meta.rs. This file consumes
// exactly the SIBLING-UNIT CONTRACTS listed below; it never names a
// field of `Meta`.
//
// ---------------------------------------------------------------------------
// ASSUMED-IMPORT (RULE §4 B4): glean_thrift
// ---------------------------------------------------------------------------
//
// The source imports `Data.HashMap.Strict` (:13), `Glean.Database.Meta
// (Meta)` (:15) and `Glean.Types (Repo)` (:16). Their Rust images, one
// spelling each:
//
//   use std::collections::HashMap;   // Data.HashMap.Strict
//   use glean_thrift as thrift;      // Glean.Types -> generated crate
//   use crate::database::meta::Meta; // row 48's shipped re-export
//
//   // glean_thrift::Repo — build.rs image of glean/if/glean.thrift:106-109
//   // (gen-hs2:629 data Repo{repo_name, repo_hash}); the same assumed
//   // shape recorded by backend/types.rs:171-175 and
//   // catalog/filter.rs:80-83:
//   #[derive(Clone, Debug, PartialEq, Eq, Hash)]
//   pub struct Repo { pub name: String, pub hash: String }
//
// `Hash` + `Eq` are load-bearing here: `std::collections::HashMap` needs
// them for the `list` return type. (backend/types.rs:174-175 records
// Hash as the hsthrift Hashable image; the Haskell `Ord` derivation is
// not needed by this unit and is not added.)
//
// Meta is NOT an assumed shape in this file: row 48 shipped
// `pub use crate::internal::types::Meta;` at
// rust/glean-db/src/database/meta.rs:529, and meta.rs:249-250 records
// the generated struct's assumed derive list
// `#[derive(Clone, Debug, Default, PartialEq, Eq)]`. This unit relies
// on `Clone` (the owned `list` map and the owned `Option<Meta>` that
// `get` returns) and on the type being nameable.
//
// ---------------------------------------------------------------------------
// SIBLING-UNIT CONTRACTS
// ---------------------------------------------------------------------------
//
//   * Meta (row 48, shipped): `crate::database::meta::Meta` (re-export at
//     meta.rs:529; derives at meta.rs:249-250). No field access here.
//   * Files (row 64, SHIPPED): `impl Store for Files` in
//     catalog/local/files.rs. Its uncaught std::fs failures land in
//     `StoreError::Io` (`list`'s root listing :47, `create` :73/:74,
//     `delete` :80, `put` :86, `get`'s read :94); the `catchAll` legs
//     (`list` :51-54 and :55-67) log and skip inside the impl and never
//     surface. `get`'s JSON `Left` arm at :97 now lands in the typed
//     `StoreError::Db` arm: row 64 CONSUMED the forward contract below.
//     The source's `.../meta/meta` defect (:90/:94) is reproduced
//     bug-for-bug there.
//   * Memory (row 65, future): `impl Store for Memory`. The source
//     methods (Memory.hs:27-45) are IORef operations that cannot throw,
//     so every method returns `Ok(...)`; the IORef ->
//     `Mutex<HashMap<..>>` image is that row's call.
//   * Config (row 85, future): `Some Catalog.Store` (Config.hs:118).
//   * Catalog (row 54, THIS unit, SHIPPED): `forall local. Store local =>
//     Catalog` (Catalog.hs:133) -> `crate::database::catalog::CatalogInner`
//     holds `local: CatalogStore`.
//
// ---------------------------------------------------------------------------
// EXPORT MAPPING (1/1 source exports; StoreError is Rust-only)
// ---------------------------------------------------------------------------
//
// The source export list (Store.hs:10-11) is exactly one name: `Store(..)`
// — the class and all five methods. Nothing else is exported and the
// class has no private members.
//
//   Store(..)   -> pub trait Store + its five required methods
//   class Store a (:19) -> pub trait Store
//   list   (:21) -> fn list
//   create (:25) -> fn create
//   delete (:29) -> fn delete
//   put    (:33) -> fn put
//   get    (:37) -> fn get
//
// Source order is preserved. The class declares no superclasses, no
// associated types, no default method bodies, no instances and no
// deriving clause (:19-37): there is nothing further to map and nothing
// omitted. The port adds no helper functions or private items.
// `StoreError` is a RUST-ONLY addition required by RULE §2, not a source
// export (see RUST-ONLY ADDITIONS).
//
// ---------------------------------------------------------------------------
// FIDELITY NOTES (every representation decision)
// ---------------------------------------------------------------------------
//
//   * Receiver. Each source method's leading `a ->` becomes `&self`, the
//     Backend precedent (backend/types.rs:430-437). No method consumes
//     self and none requires `Self: Sized`.
//   * Arguments. `Repo -> &thrift::Repo` and `Meta -> &Meta`: the Haskell
//     shares immutable values, no call site mutates or consumes the
//     argument, and the trait must not force a clone. This matches the
//     Backend convention for Repo (backend/types.rs:430-437) and adds the
//     same rule for Meta.
//   * `list`'s result. `HashMap Repo Meta -> HashMap<thrift::Repo, Meta>`
//     OWNED. A trait method cannot lend out storage it does not own; the
//     returned map is a deep clone where the source shared the entries.
//     This is the value-semantics consequence of the owned return; it is
//     recorded as the PERF(port) deferral at the method (the fast version
//     is a shared snapshot). Both sides are unordered maps
//     (Data.HashMap.Strict and std HashMap), so no iteration-order contract
//     is introduced or lost here.
//   * Effects. Every `IO` return becomes `Result<_, StoreError>` per RULE
//     §2's dynamic-exceptions row. `Bool` stays `bool`; `Maybe Meta`
//     stays `Option<Meta>`.
//   * No Send/Sync supertraits. The source class declares no such
//     constraints, and the existential image is a closed enum (below),
//     not `dyn Store`, so auto traits survive on the concrete types.
//   * Names. The trait and all five methods keep the source names
//     (`Store`, `list`, `create`, `delete`, `put`, `get`); there are no
//     constants to name.
//
// ---------------------------------------------------------------------------
// RUST-ONLY ADDITIONS: StoreError (RULE §2 dynamic-exceptions row)
// ---------------------------------------------------------------------------
//
// The source throws untyped `SomeException` through `IO`; the port gives
// the area one typed channel at every `?`, following the RunQueryError
// precedent (rust/glean-db/src/query/thrift.rs:409-454: area-local enum,
// constructor-shaped Display, std::error::Error impl).
//
//   * `Io(std::io::Error)` — the Files impl's uncaught std::fs failures
//     (Files.hs:47, :73/:74, :80, :86, :94; the doesDirectoryExist/
//     doesFileExist checks return Bool and never throw).
//   * `Db(DbError)` — the typed database-error leg, added by row 64 (see
//     the CONSUMED contract below). It carries `get`'s `dbError` at
//     Files.hs:97 with the area's own taxonomy instead of erasing it.
//   * `Other(Box<dyn std::error::Error + Send + Sync>)` — the open
//     `SomeException` leg. No in-tree Store impl uses it today; it stays
//     for callers that raise an unclassified error through a store. The
//     `catchAll` legs at :52-54 and :67 do NOT land here: they log and
//     return `[]`/`Nothing` inside the impl. JSON parse failures are
//     `Either String Meta`, not exceptions, so they are also handled
//     inside the impl (:63-66 skip, :95-97 dbError).
//
// FORWARD CONTRACT (row 53/64) — CONSUMED BY ROW 64: `database/`
// `exception.rs` shipped the DbError image, and catalog/local/files.rs
// added the typed `Db(DbError)` variant plus `From<DbError>`; `get` now
// uses `db_error(repo, ..)?` directly and nothing is erased into `Other`.
// The two concrete impls (rows 64/65) pin which io/dbError cases land
// where; the catch taxonomy itself is Step 4's, exactly as BackendError
// defers its Display/catch table and RunQueryError defers only its
// catch table (it already has Display). `BackendError` is
// NOT reused or renamed: it belongs to another area's surface and
// reusing it would invent a cross-area edge.
//
// ---------------------------------------------------------------------------
// EXISTENTIAL MAPPING (documented here; one shared enum, owned here)
// ---------------------------------------------------------------------------
//
// Both existential sites — `Some Catalog.Store` (Config.hs:118) and
// `forall local. Store local => Catalog` (Catalog.hs:133) — are RULE §2
// existential sites. Their production instance set is CLOSED:
// {Files (row 64), Memory (row 65)}. The test `MockStore` instance is
// tests, not translated (RULE §4), so it does not widen the set.
//
// The image is therefore ONE closed enum shared by both sites, following
// the `SomeBackend` precedent (backend/types.rs:745-778), NOT
// `Box<dyn Store>`. The five methods ARE object-safe (no generics, no
// `Self`-by-value receivers, no associated items), so `Box<dyn Store>`
// would compile — but the closed-set rule chooses the enum.
//
// OWNERSHIP (single owner, no forks): the same value flows from Config's
// constructor (`Some (Catalog.fileCatalog path)` Config.hs:141, `Some
// cat` :159) into `Catalog.open` (Env.hs:84-85 -> Catalog.hs:310) and is
// stored in `catLocal` (Catalog.hs:133), so two distinct enums would not
// type-check without a conversion. The enum belongs to THIS module (the
// trait's home, the SomeBackend placement) and was ADDED BY THE FIRST
// CONSUMER, row 54 (Catalog.hs), which precedes rows 64/65 in the
// manifest and therefore forward-references `Files`/`Memory` (the
// SomeBackend forward-reference precedent). Row 54 CONSUMED this
// contract: the `CatalogStore` enum and its forwarding `impl Store`
// are at the bottom of this file. Row 85 (Config.hs) imports the enum;
// it must not define a second one. Test re-expression (RULE §4;
// `MockStore`, Test.hs:46-51) may need a `#[cfg(test)]` variant or an
// equivalent hook, but tests do not widen the production set.
//
// The variant types: `crate::database::catalog::local::files::Files` is
// row 64, SHIPPED at catalog/local/files.rs, and `...::local::memory::
// Memory` is row 65, still a FORWARD REFERENCE — row 54 created neither
// `catalog/local.rs` nor the `pub mod local;` line, and row 64 has since
// created both plus files.rs. Only the Memory path is missing (the
// SomeBackend forward-reference form: this compiles once row 65 lands, and
// a missing path is a Step-4 compile error, never a silent divergence).

use std::collections::HashMap;
use std::fmt;

use glean_thrift as thrift;

use crate::database::meta::Meta;

/// A metadata store (Store.hs:18-19).
///
/// The five methods are the class's five abstract methods, in source
/// order. `&self` is the Haskell `a ->` first argument; the IO returns
/// become `Result<_, StoreError>` (RULE §2).
pub trait Store {
    /// List all available entries (:20-21).
    ///
    /// The returned map is OWNED: a trait method cannot lend out the
    /// store's storage, so this is a deep clone where the source shared.
    /// Both maps are unordered, so no iteration-order contract is added.
    // PERF(port): the owned map deep-clones every Meta; the fast version
    // is a shared snapshot (Arc<HashMap<..>>) once the signature may
    // change. `get` clones one Meta for the same reason.
    fn list(&self) -> Result<HashMap<thrift::Repo, Meta>, StoreError>;

    /// Create an entry if it doesn't exist. Return `true` if the entry
    /// was created and `false` if it already existed (:23-25).
    fn create(&self, repo: &thrift::Repo, meta: &Meta) -> Result<bool, StoreError>;

    /// Delete an entry. Return `true` if the entry was deleted and
    /// `false` if it didn't exist (:27-29).
    fn delete(&self, repo: &thrift::Repo) -> Result<bool, StoreError>;

    /// Update the metadata of an entry. Return `true` if the metadata was
    /// updated and `false` if the entry didn't exist (:31-33).
    fn put(&self, repo: &thrift::Repo, meta: &Meta) -> Result<bool, StoreError>;

    /// Read the metadata of an entry, returning `None` if the entry
    /// didn't exist (:35-37).
    fn get(&self, repo: &thrift::Repo) -> Result<Option<Meta>, StoreError>;
}

/// The area's `SomeException` image (RUST-ONLY; see the header's
/// RUST-ONLY ADDITIONS block). `Io` carries the Files impl's std::fs
/// failures, `Db` the database-error leg (row 64's consumption of the
/// forward contract), and `Other` the open leg for caller-raised errors.
#[derive(Debug)]
pub enum StoreError {
    /// `std::fs` / `std::io` failure from the Files impl on an uncaught
    /// path (Files.hs:47, :73/:74, :80, :86, :94; the existence checks
    /// return Bool and never throw).
    Io(std::io::Error),
    /// The typed database-error leg (row 64): `dbError` at Files.hs:97.
    /// The `catchAll` legs at :52-54/:67 do NOT reach here — they log and
    /// return `[]`/`Nothing` inside the impl.
    Db(crate::database::exception::DbError),
    /// The open leg for an unclassified caller-raised error.
    Other(Box<dyn std::error::Error + Send + Sync>),
}

impl From<std::io::Error> for StoreError {
    fn from(e: std::io::Error) -> Self {
        StoreError::Io(e)
    }
}

/// Row 64's consumption of the forward contract: the Files `get` arm
/// converts the single throw funnel's `DbError` into this store error.
impl From<crate::database::exception::DbError> for StoreError {
    fn from(e: crate::database::exception::DbError) -> Self {
        StoreError::Db(e)
    }
}

/// Constructor-shaped, per RULE §2's deriving-Show default (the type has
/// no source counterpart, so there is no Show text to replicate).
impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::Io(e) => write!(f, "Io({})", e),
            StoreError::Db(e) => write!(f, "Db({})", e),
            StoreError::Other(e) => write!(f, "Other({})", e),
        }
    }
}

/// `source()` stays the default: the catch taxonomy is Step 4's, as with
/// BackendError and RunQueryError.
impl std::error::Error for StoreError {}

// ---------------------------------------------------------------------------
// CatalogStore (row 54's consumption of the existential contract)
// ---------------------------------------------------------------------------

/// The closed-set image of `Some Catalog.Store` (Config.hs:118) and
/// `forall local. Store local => Catalog` (Catalog.hs:133), per RULE §2's
/// GADT/existential row. The production variant set is closed:
///
///   * `Files(crate::database::catalog::local::files::Files)` — row 64,
///     SHIPPED;
///   * `Memory(crate::database::catalog::local::memory::Memory)` — row 65,
///     a FORWARD REFERENCE.
///
/// Row 64 created catalog/local.rs, its `pub mod files;` line and
/// catalog/local/files.rs. Row 65 still owes memory.rs and
/// `pub mod memory;` (the SomeBackend precedent, backend/types.rs:745-778).
/// The test `MockStore` instance is tests and does not widen the production
/// set (RULE §4).
/// No derive list is attached: the enum is moved into
/// `CatalogInner.local` and never cloned, so it must not impose Clone or
/// Debug on stores that do not have them. The variant types MUST be
/// Send + Sync: the committer thread holds an `Arc<CatalogInner>` that
/// owns this enum. That is a Step-4 obligation on rows 64/65 (the source
/// class has no such constraint; the constraint comes from the Rust thread
/// boundary, not from Store.hs).
pub enum CatalogStore {
    Files(crate::database::catalog::local::files::Files),
    Memory(crate::database::catalog::local::memory::Memory),
}

/// Method-for-method forwarding, the `impl Backend for SomeBackend<R>`
/// shape (backend/types.rs:786-900): every class method delegates to the
/// wrapped store. This is the image of the source's dictionary passing
/// under `forall local. Store local =>`.
impl Store for CatalogStore {
    fn list(&self) -> Result<HashMap<thrift::Repo, Meta>, StoreError> {
        match self {
            CatalogStore::Files(store) => store.list(),
            CatalogStore::Memory(store) => store.list(),
        }
    }

    fn create(&self, repo: &thrift::Repo, meta: &Meta) -> Result<bool, StoreError> {
        match self {
            CatalogStore::Files(store) => store.create(repo, meta),
            CatalogStore::Memory(store) => store.create(repo, meta),
        }
    }

    fn delete(&self, repo: &thrift::Repo) -> Result<bool, StoreError> {
        match self {
            CatalogStore::Files(store) => store.delete(repo),
            CatalogStore::Memory(store) => store.delete(repo),
        }
    }

    fn put(&self, repo: &thrift::Repo, meta: &Meta) -> Result<bool, StoreError> {
        match self {
            CatalogStore::Files(store) => store.put(repo, meta),
            CatalogStore::Memory(store) => store.put(repo, meta),
        }
    }

    fn get(&self, repo: &thrift::Repo) -> Result<Option<Meta>, StoreError> {
        match self {
            CatalogStore::Files(store) => store.get(repo),
            CatalogStore::Memory(store) => store.get(repo),
        }
    }
}

// ---------------------------------------------------------------------------
// Coverage / marker tally (closed set)
// ---------------------------------------------------------------------------
//
// Source symbols: one export, `Store(..)`, imaged as the trait plus its
// five methods (EXPORT MAPPING above); nothing is omitted (no
// superclasses, associated types, defaults, instances or deriving in
// :19-37). Two Rust-only additions: `StoreError` (required by RULE §2,
// documented above; its typed `Db` arm is row 64's consumption of the
// forward contract) and `CatalogStore` (row 54's consumption of the
// existential contract, documented above).
//
// Markers: no port todos, no source-defect reproductions here (row 64's
// `get` double-join is reproduced at its own site in files.rs, recorded
// above), ONE performance
// deferral (the owned `list` deep clone, marked at the method; the
// forwarding impl adds no clone of its own), no unsafe code (no unsafe
// blocks), no inventory divergence (zero governing rows for Store.hs
// itself; the CatalogStore addition follows Catalog.hs's row 21 exactly
// as its EXISTENTIAL MAPPING contract required).
//
// PORT STATUS: confidence=high todos=0
