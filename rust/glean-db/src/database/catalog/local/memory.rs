// Ported from: glean/db/Glean/Database/Catalog/Local/Memory.hs (45 lines).
// Manifest unit: rust/glean-db/src/database/catalog/local/memory.rs
// (migration/manifest.tsv line 65, crate glean-db). RULE §4: one source
// file -> one target file; the manifest path is authoritative.
//
// WHAT THIS FILE IS: the in-memory metadata store. Memory is one mutable
// map from Repo to Meta (:22); memoryCatalog starts it empty (:24-25) and
// the five Store methods are the map's member/insert/delete/lookup
// operations (:27-45). Nothing touches the filesystem and no method can
// fail.
//
// ---------------------------------------------------------------------------
// INVENTORY (RULE §5; migration/inventory*.tsv)
// ---------------------------------------------------------------------------
//
// GOVERNING ROW — migration/inventory.tsv:21 == migration/inventory.stm.tsv:23
// (the Catalog STM_GROUP row, confirmed). Its target clause, verbatim:
//
//   "Memory catalog backend (Catalog/Local/Memory.hs:22,28-45 IORef
//    HashMap) -> plain Mutex<HashMap<Repo,Meta>> behind the same
//    CatalogBackend trait"
//
// Implemented exactly: Memory wraps Mutex<HashMap<Repo, Meta>> and its
// Store impl is the same trait Files implements (the row's
// "CatalogBackend" is the source's Store class, store.rs).
//
// ZERO other rows name this file. Exact command at the repo root:
//
//   $ grep -n 'Catalog/Local/Memory' migration/inventory*.tsv
//
// returns only the two copies of the governing row. inventory.sites.tsv's
// 'Memory.hs' hits (:301-306) are all Storage/Memory.hs, a different file
// (manifest line 71). inventory.ffi.tsv, inventory.codeflow.tsv and
// inventory.protocol.tsv: zero hits.
//
// No INVENTORY-BUG-CANDIDATE applies: the row is followed verbatim.
//
// ---------------------------------------------------------------------------
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, checked not assumed)
// ---------------------------------------------------------------------------
//
//   * OUTBOUND (2), both shipped:
//       edges.tsv:182 Catalog/Store.hs -> row 50, catalog/store.rs
//       edges.tsv:183 Database/Meta.hs -> row 48, database/meta.rs
//   * INBOUND (1): edges.tsv:219 Config.hs -> Memory.hs (manifest line 85,
//     NOT shipped). Config.hs:159 binds 'cat <- Catalog.memoryCatalog' in
//     the in-memory configuration; see SIBLING-UNIT CONTRACTS.
//   * The remaining imports are base: Data.HashMap.Strict (:14-15),
//     Data.IORef (:16) and the generated Glean.Types (:20, an assumed
//     import per RULE §4 B4).
//
// ---------------------------------------------------------------------------
// ASSUMED IMPORTS (RULE §4 B4)
// ---------------------------------------------------------------------------
//
//   * glean_thrift::Repo. Pinned shape (glean/if/glean.thrift:106-109):
//         struct Repo { 1: string name; 2: string hash; }
//     The image recorded at rust/glean-db/src/backend/types.rs:171-175 is
//     Clone + Debug + PartialEq + Eq + Hash. The map key needs Eq + Hash
//     for its HashMap operations and Clone for the owned insert; the
//     pinned derive list provides all three.
//   * Meta: row 48's re-export at database/meta.rs:529
//     ('pub use crate::internal::types::Meta;'), with the assumed derives
//     Clone + Debug + Default + PartialEq + Eq (meta.rs:249-250). The map
//     value needs Clone (owned insert/lookup), which that derive list
//     provides.
//
// ---------------------------------------------------------------------------
// SIBLING-UNIT CONTRACTS
// ---------------------------------------------------------------------------
//
//   * Store (row 50, SHIPPED, catalog/store.rs): the five methods. Every
//     one returns Ok here: the source's IORef operations cannot throw, so
//     this impl never constructs StoreError::Io, ::Db or ::Other.
//   * CatalogStore (row 50, catalog/store.rs): its 'Memory' variant holds
//     this type and is the LAST forward reference the enum carried. This
//     unit closes it. Memory is Send + Sync (Mutex<HashMap<..>>), the
//     Step-4 obligation recorded at store.rs for the committer thread's
//     Arc<CatalogInner>.
//   * local.rs (row 65, THIS unit): appends 'pub mod memory;' below row
//     64's 'pub mod files;', resolving this file.
//   * Config (row 85, future): Config.hs:159 'cat <- Catalog.memoryCatalog'
//     becomes 'memory_catalog()' (the source's IO wrapper is dropped
//     because constructing the mutex cannot fail; the mock() precedent).
//
// ---------------------------------------------------------------------------
// EXPORT MAPPING (2/2 source exports, names at :10-11)
// ---------------------------------------------------------------------------
//
//   :10 Memory        -> pub struct Memory (field private)
//   :11 memoryCatalog -> pub fn memory_catalog
//   instance Store Memory (:27-45) -> impl Store for Memory
// Rust-only helper: 'lock' (poison-recovering Mutex acquisition).
//
// ---------------------------------------------------------------------------
// FIDELITY NOTES
// ---------------------------------------------------------------------------
//
//   * IORef -> Mutex, per the governing row. readIORef becomes one lock
//     plus a clone; atomicModifyIORef' becomes one lock acquisition with
//     the same check-then-update body. No other thread can observe an
//     intermediate state, exactly as the IORef's atomic modify guarantees.
//   * list returns an OWNED map, so it clones every entry. The trait
//     method's performance-deferral note (store.rs) covers this
//     signature-forced copy; Memory adds no second marker. get clones one
//     Meta for the same
//     reason. create/put clone the meta into the owned map; Haskell shared
//     the immutable value.
//   * Lock poisoning is recovered with 'PoisonError::into_inner', the
//     catalog.rs:707-709 convention. The source's IORef has no failure
//     mode, so a panic in another thread must not turn every Store call
//     into an error; the stored map is still structurally valid.
//   * delete is 'remove(repo).is_some()': the membership test and the
//     deletion are one critical section, like atomicModifyIORef'.
//   * No unsafe code, no source-defect reproductions, no port deferrals.
//
// PORT STATUS: confidence=high todos=0

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, PoisonError};

use glean_thrift as thrift;

use crate::database::catalog::store::{Store, StoreError};
use crate::database::meta::Meta;

/// 'Memory' (:22): one mutable Repo -> Meta map. The source's newtype
/// hides its field (the module exports only the type name, :10-11), so the
/// field is private; 'memory_catalog' is its only constructor.
pub struct Memory {
    entries: Mutex<HashMap<thrift::Repo, Meta>>,
}

/// 'memoryCatalog' (:24-25): a new empty store. The source's 'IO' wrapper
/// is dropped: constructing the mutex cannot fail.
pub fn memory_catalog() -> Memory {
    Memory {
        entries: Mutex::new(HashMap::new()),
    }
}

impl Store for Memory {
    /// :28. 'readIORef v'.
    fn list(&self) -> Result<HashMap<thrift::Repo, Meta>, StoreError> {
        Ok(lock(&self.entries).clone())
    }

    /// :30-33. Insert only when the repo is absent.
    fn create(&self, repo: &thrift::Repo, meta: &Meta) -> Result<bool, StoreError> {
        let mut entries = lock(&self.entries);
        if entries.contains_key(repo) {
            Ok(false)
        } else {
            entries.insert(repo.clone(), meta.clone());
            Ok(true)
        }
    }

    /// :35-38. Delete only when the repo is present.
    fn delete(&self, repo: &thrift::Repo) -> Result<bool, StoreError> {
        Ok(lock(&self.entries).remove(repo).is_some())
    }

    /// :40-43. Replace only when the repo is present.
    fn put(&self, repo: &thrift::Repo, meta: &Meta) -> Result<bool, StoreError> {
        let mut entries = lock(&self.entries);
        if entries.contains_key(repo) {
            entries.insert(repo.clone(), meta.clone());
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// :45. 'HashMap.lookup repo <$> readIORef v'.
    fn get(&self, repo: &thrift::Repo) -> Result<Option<Meta>, StoreError> {
        Ok(lock(&self.entries).get(repo).cloned())
    }
}

/// The catalog.rs:707-709 convention: the source has no poisoning concept,
/// so recover the guard instead of surfacing an error.
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}
