// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// Ported from: glean/db/Glean/Database/Catalog.hs (632 lines)
// Manifest unit: rust/glean-db/src/database/catalog.rs
// (migration/manifest.tsv line 54, crate glean-db). RULE §4: one source
// file -> one target file; the manifest path is authoritative.
//
// WHAT THIS FILE IS: the catalog of DB entries — the in-memory registry the
// database layer consults for "which databases exist, where, in what state,
// and what do their dependencies imply". The source's doc comment
// (Catalog.hs:10-12) states the two consumers: "A Catalog stores metadata
// about available databases. A thread running Glean.Database.Backup.backuper
// will continuously read the state of the catalog to decide which dbs to
// restore/backup/finalize".
//
// The state is:
//   * one Entry per known database, holding its status, metadata, dirty and
//     committing flags and expiry time (EntryF, :93-107);
//   * the Entries maps: live-here, live-elsewhere, restoring, ephemeral and
//     failed (:116-127);
//   * a dirty queue drained by a dedicated committer thread (:138, :252-290);
//   * a repo -> dependents index used to propagate status changes (:142,
//     :227-232).
//
// TWO ROLES AT THIS PATH (row 49's contract, CONSUMED HERE). Haskell's
// `Glean.Database.Catalog` is both a module (Catalog.hs) and a namespace
// (the Catalog/ directory), so the single Rust path
// src/database/catalog.rs serves both: it is the port target of Catalog.hs
// (this file) and the directory module for database/catalog/filter.rs
// (row 49, shipped), database/catalog/store.rs (row 50, shipped) and
// database/catalog/local.rs (rows 64/65's Catalog/Local/ namespace, which
// declares `pub mod files;` and `pub mod memory;`). Rows
// 49/50 declared only the `pub mod` lines they owed and left the
// reconciliation to row 54 (their headers say so; database.rs:39-49 records
// the same deferral). RESOLVED HERE: the directory-module declarations stay
// at the top of this file, immediately after this header, and the port body
// follows them. This is the src/query/thrift.rs precedent (row 47's port of
// Thrift.hs declares `pub mod internal;` and then carries its own body).
//
// TEST-ONLY GENERICITY IS COLLAPSED (RULE §4: tests are not translated).
// The source parameterizes EntryF over the state container
// (`data EntryF ref`, :93) and EntriesF over the entry type
// (`data EntriesF entry`, :116); the only non-Identity instantiation is
// the test mock. The source says so itself at :91-92 (EntryF) and
// :115-116 (EntriesF): "This extra generality comes handy in tests". The
// production aliases are
// `type Entry = EntryF TVar` (:89) and `type Entries = EntriesF Entry`
// (:113). The port has exactly one concrete `Entry` and one concrete
// `Entries`; there is no phantom parameter and no test hook. The five
// per-entry TVars (:95-105) collapse into one `EntryState` behind one
// mutex, per the governing inventory row.
//
// ---------------------------------------------------------------------------
// INVENTORY (RULE §5; migration/inventory*.tsv)
// ---------------------------------------------------------------------------
//
// GOVERNING ROW — migration/inventory.tsv:21 == migration/inventory.stm.tsv:23
// (identical rows; both confirmed). Quoted verbatim (TAB separators shown as
// \t so the row stays one readable line; the source file uses real tabs):
//
//   glean/db/Glean/Database/Catalog.hs\tCatalog\tCatalog of DB entries: per-Entry TVars (status/meta/dirty/committing/expiring) + catEntries map-swap + catDirtyQueue + catRepoDependents; committer thread drains dirty queue\tSTM_GROUP\tONE Mutex<CatalogState> { entries: Option<HashMap<Repo, Arc<Mutex<EntryState>>>>, dirty_queue: VecDeque<Arc<..>>, repo_dependents: HashMap<..> } — per-entry inner Mutex ONLY if source updates single entries without catEntries in the same txn (445-461 meta writes are entry-local). Global map swap on open/close (313-342, 349-358) = replace whole Option under the outer lock; `entries == None` is the closed/busy flag -> keep as Option. dirtyEntry dedup via entryDirty swapTVar (245-246) -> Mutex<bool> flag + queue push. Committer thread (258-290) -> dedicated thread + Condvar on dirty_queue. LOCK ORDER: outer CatalogState before EntryState; never take two entry locks in different order than sorted-by-Repo (source orders via HashMap traversal — pin sorted order). Memory catalog backend (Catalog/Local/Memory.hs:22,28-45 IORef HashMap) -> plain Mutex<HashMap<Repo,Meta>> behind the same CatalogBackend trait\tCatalog.hs:89 type Entry = EntryF TVar; :136 catEntries :: TVar (Maybe Entries); :138 catDirtyQueue :: TQueue Entry; :142 catRepoDependents :: TVar; :158-162 newEntry TVars (status/meta/dirty/committing/expiring); :168-231 recalculateStatus multi-read txn; :206-224 status transition; :229-230 dependents+entries read; :245-246 dirty dedup; :258-290 committer loop (265 open check, 269 reset committing); :313-322 open builds entries atomically; :339-342 close writeTVar catEntries Nothing; :349-358 load swap; :378-387 reopen preserving dirty queue; :427 listMostRecent; :445-461 meta read/modify; :487-492 expiring\tconfirmed
//
// The row's five target clauses, mapped one-for-one:
//   1. "ONE Mutex<CatalogState>" -> `CatalogInner.state`, the single outer
//      lock. Its `entries` field is `Option<Entries>`; `None` is the
//      closed flag (open/close replace the whole Option under the lock).
//   2. "per-entry inner Mutex ONLY if ... (445-461 meta writes are
//      entry-local)" -> `Entry.state: Mutex<EntryState>`. The map value is
//      `Arc<Entry>` rather than the row's illustrative
//      `Arc<Mutex<EntryState>>`: the Entry carries its `repo` alongside
//      the state so the dirty queue can log `inRepo entryRepo` (:280,
//      :285) without re-deriving the repo. The per-entry mutex and the
//      outer-map discipline are exactly the row's; the composition is a
//      representational refinement, not a decision change. It IS a
//      divergence from the row's literal target, so it is flagged:
//
//      // INVENTORY-BUG-CANDIDATE row 21: the target's map value is
//      // `Arc<Mutex<EntryState>>`; this file stores `Arc<Entry>` where
//      // `Entry { repo, state: Mutex<EntryState> }`. The mutex discipline
//      // (one inner mutex per entry, outer before inner) is unchanged; the
//      // immutable `repo` travels with the state because the dirty queue,
//      // delete's queue filter (:384) and the committer's in_repo logging
//      // (:280, :285) need it without a second map lookup.
//   3. "dirtyEntry dedup ... -> Mutex<bool> flag + queue push" ->
//      `EntryState.dirty` + `dirty_entry_locked` (:243-247).
//   4. "Committer thread (258-290) -> dedicated thread + Condvar on
//      dirty_queue" -> `commit` + `CatalogInner.changed`.
//   5. "Memory catalog backend ... behind the same CatalogBackend trait" ->
//      the Store trait shipped at store.rs (the row's "CatalogBackend" is
//      the source's `Store` class); the closed enum for the existential is
//      `CatalogStore`, owned by store.rs and consumed here (see
//      EXISTENTIAL below).
// No OTHER clause is diverged from (the `Arc<Entry>` refinement above is
// the single flagged divergence).
//
// SITES ROWS — migration/inventory.sites.tsv:215-232 are one `stm` row per
// cited source line, all sourced to this file:
//   :215 -> Catalog.hs:89   `type Entry = EntryF TVar`
//   :216 -> :136            `catEntries :: TVar (Maybe Entries)`
//   :217 -> :138            `catDirtyQueue :: TQueue Entry`
//   :218 -> :142            `catRepoDependents :: TVar (HashMap Repo [Repo])`
//   :219 -> :258            committer `atomically`
//   :220 -> :269            committing reset
//   :221 -> :272            meta clear/read transaction
//   :222 -> :287            re-dirty on commit failure
//   :223 -> :313            open builds the live map
//   :224 -> :333            open's per-entry recalculation
//   :225 -> :339            close's transaction
//   :226 -> :349            create's pre-transaction
//   :227 -> :358            create's post-transaction
//   :228 -> :378            delete's pre-transaction
//   :229 -> :393            delete's post-transaction
//   :230 -> :427            listMostRecent
//   :231 -> :529            finishRestoring's pre-transaction
//   :232 -> :540            finishRestoring's post-transaction
// Every one is imaged by the corresponding function below: :215-:218 are
// the state fields, :219-:222 the committer's transactions, :223-:225
// open/close, :226-:229 create/delete, :230 listMostRecent, and :231-:232
// finishRestoring. Each transaction is one acquisition of the outer mutex
// (the :219-:222 group's entry-local parts take only that entry's mutex),
// EXCEPT the two transactions that run before the Catalog exists
// (open's `traverseWithKey mkEntry`, :313, builds the live map with no lock
// because no other thread can see it yet) and the read transactions, which
// now collect their whole snapshot under one outer acquisition (see `list`,
// `get_local_databases`, `exists`).
//
// EXCEPTION-TAXONOMY SITES — migration/inventory.sites.tsv:602-605
// (`exc-classify`), all sourced to this file:
//   :602 -> Catalog.hs:53   `SomeException` import
//   :603 -> :126            `entriesFailed :: HashMap Repo SomeException`
//   :604 -> :282            `Left (exc :: SomeException) -> do`
//   :605 -> :466            `dbFailed :: Catalog -> Repo -> SomeException -> STM ()`
// The image is `CatalogError` below (RULE §2 dynamic-exceptions row), with
// `StoreError` for the store leg and the boxed `Error + Send + Sync` for
// the failed-map payload. The db-error leg funnels through row 53's
// `DbError` (exception.rs) rather than inventing a second funnel.
//
// ZERO-HIT VERIFICATION for the other inventory files. Command (repo root):
//
//   grep -c -E 'Database/Catalog\.hs' migration/inventory.tsv \
//       migration/inventory.sites.tsv migration/inventory.protocol.tsv \
//       migration/inventory.stm.tsv migration/inventory.codeflow.tsv \
//       migration/inventory.ffi.tsv
//
// Result: 1, 22, 0, 1, 0, 0 — i.e. ZERO rows in
// migration/inventory.protocol.tsv, migration/inventory.codeflow.tsv and
// migration/inventory.ffi.tsv (the three inventory files other than the
// three named above). One near-miss is NOT governing:
// migration/inventory.protocol.tsv:25 is the Create.hs EXC_TAXONOMY row; its
// evidence text names `Catalog.hs:126` and `Catalog.hs:276-286` as
// consumer evidence for Create.hs's mark-broken behaviour, but the row's
// source file is Create.hs, so it cannot govern this unit. No
// INVENTORY-BUG-CANDIDATE applies.
//
// ---------------------------------------------------------------------------
// DEPENDENCY TRUTH (migration/depmap/edges.tsv)
// ---------------------------------------------------------------------------
//
// OUTBOUND (5 rows, all shipped siblings; this unit calls them and
// re-implements none of them):
//   * edges.tsv:171 Catalog.hs -> Catalog/Filter.hs  -> manifest row 49,
//     SHIPPED rust/glean-db/src/database/catalog/filter.rs. Import :78;
//     uses Locality (:399-400, :427, :434-439), Item (:402-423), Filter
//     (:399-423), runFilter (:410), groupF/sortF/limitF (:427-430),
//     repoNameV (:428), createdV (:429), Descending (:429).
//   * edges.tsv:172 Catalog.hs -> Catalog/Store.hs   -> manifest row 50,
//     SHIPPED rust/glean-db/src/database/catalog/store.rs. Imports :79-80;
//     the class bound :133, :310; calls Store.put :276, Store.list :312,
//     Store.create :370/:550, Store.delete :396.
//   * edges.tsv:173 Catalog.hs -> Database/Exception.hs -> manifest row 53,
//     SHIPPED rust/glean-db/src/database/exception.rs. Import :81; throws
//     DBAlreadyExists :518; dbError :372/:539/:551/:558.
//   * edges.tsv:174 Catalog.hs -> Database/Meta.hs    -> manifest row 48,
//     SHIPPED rust/glean-db/src/database/meta.rs. Import :82; entryMeta
//     :98/:168/:206/:214/:274/:422/:445/:451/:459/:461/:607;
//     metaDependencies :180; metaProperties :208; metaCompleteness :219;
//     metaToThriftDatabase :611/:621.
//   * edges.tsv:175 Catalog.hs -> Database/Repo.hs    -> manifest row 52,
//     SHIPPED rust/glean-db/src/database/repo.rs. Import :83; inRepo :280,
//     :285.
//
// INBOUND (18 consumer rows; NONE is edited by this unit):
//   edges.tsv:30 backend-local/Local.hs; :144 Backup.hs; :195
//   CompletePredicates.hs; :217 Config.hs; :238 Create.hs; :258 Delete.hs;
//   :269 Env.hs; :288 Finish.hs; :292 Janitor.hs; :311 List.hs; :318
//   Open.hs; :355 Restore.hs; :448 Types.hs; :474 Write/Batch.hs; :498
//   Write/Queue.hs; :558 Query/Derive.hs; :682 Query/UserQuery.hs; :1608
//   server/Sharding.hs.
// This unit ships the surface those consumers were written against; it does
// not touch any of their files.
//
// ---------------------------------------------------------------------------
// TRANSLATION NOTES (every representation decision, with its evidence)
// ---------------------------------------------------------------------------
//
// * STM -> Mutex + Condvar. The row pins ONE outer mutex over the catalog
//   state and one inner mutex per entry. Every source transaction becomes
//   one acquisition of the outer mutex; the transactions that touch only
//   one entry's TVars (:269, :272, :287, :445-461) take only that entry's
//   mutex where they do not also touch `catEntries`/`catDirtyQueue`.
//
// * THE THREE `retry` SITES (the complete list; `grep -n retry` on the
//   source returns exactly these):
//     - :258-266 committer: dequeue or (catalog open -> retry; closed ->
//       stop). Image: the `commit` loop's `pop_front` / closed-check /
//       `Condvar::wait`.
//     - :339-341 close: retry while `entriesEphemeral` is non-empty.
//       Image: `close`'s wait loop on the same condvar.
//     - :380-381 delete: retry while `entryComitting` is true. Image:
//       `delete`'s wait loop; because `retry` re-runs the whole
//       transaction, the loop re-runs `get_entry_locked` too, so a repo
//       that disappears while we wait surfaces UnknownDatabase exactly as
//       the source's re-run does.
//   All three wait on `CatalogInner.changed`, because a Condvar::wait
//   consumes the MutexGuard and therefore cannot live inside the guarded
//   state. `notify_all` is issued on: a dirty-queue push
//   (`dirty_entry_locked`), a committing reset (`commit`), an
//   entries swap/close (`close`), and an ephemeral change (`create`'s
//   post, `delete`'s pre and post, and `finish_restoring`'s pre and
//   post). Expiring changes do not notify: no source retry observes them.
//
//   CONDVAR DISCIPLINE (Rust-only): every state change a waiter tests is
//   made UNDER the outer mutex, and the notify follows it. In particular
//   the committer's committing reset takes the outer lock even though the
//   source's :269 transaction is entry-local: a Condvar notification is not
//   buffered, so a notify issued outside the waiter's mutex can land
//   between the waiter's predicate check and its wait and be lost forever.
//   STM's retry re-runs the transaction automatically and needs no such
//   rule; this is the price of the Condvar image, not a semantic change.
//   Panic safety: `commit`'s committing reset and the ephemeral
//   reservation are `Drop` guards (FIX 3), so an unwind cannot leave a
//   waiter asleep forever; the guards are the `bracket`/`tryBracket`
//   release image on the unwind path.
//
// * LOCK ORDER (the row's clause, made trivial). Outer `CatalogState`
//   before any `EntryState`, everywhere. NO FUNCTION EVER HOLDS TWO ENTRY
//   LOCKS AT ONCE: `recalculate_status_locked` takes its own entry's lock
//   in a scope and drops it, then takes each dependency's lock in its own
//   scope, then re-takes its own; `collect_items` and `local_db` take one
//   entry lock per map element WHILE THE OUTER GUARD IS HELD (the correct
//   outer-before-inner order; the read snapshot is atomic, as in the
//   source's single transaction). The row's "never take two entry locks in
//   different order than sorted-by-Repo" is therefore satisfied vacuously,
//   not by pinning a sort. The committer takes an entry lock under the
//   outer lock at dequeue time, and takes the outer lock again (never
//   holding an entry lock across the `Store::put` call) for the reset.
//
// * POISONING. STM has no poisoning concept. The port funnels every lock
//   acquisition through ONE private helper, `lock`, which recovers with
//   `PoisonError::into_inner`. std::sync cannot roll back a panic, so the
//   RULEBOOK §2 STM divergence applies: a panic mid-mutation can leave a
//   partial state (the source's `atomically` aborts and retries). The port
//   limits the damage in two ways: (a) `recalculate_status_locked` performs
//   every fallible computation (including the `Completeness::Empty`
//   `unreachable!`) BEFORE its first mutation, and (b) the two
//   liveness-critical flags (committing, ephemeral) are released by Drop
//   guards, so an unwind cannot hang `close`/`delete`. Every `Store` call
//   is outside the locks and returns a `Result`. A panic elsewhere is a
//   Step-4 defect, not a catalog state the port promises to roll back.
//
// * EXISTENTIAL -> CLOSED ENUM. `data Catalog = forall local. Store local
//   => Catalog` (:133) is the row-50 existential. The variant set is
//   closed ({Files, Memory}; the test MockStore is tests, RULE §4), so the
//   image is store.rs's `CatalogStore` enum (the SomeBackend precedent,
//   backend/types.rs:745-778), NOT a second enum and NOT `Box<dyn Store>`.
//   store.rs owns it; this file only stores it (`CatalogInner.local`) and
//   accepts it in `open`. Rows 64/65's catalog/local/files.rs and
//   catalog/local/memory.rs are shipped, so the enum is fully resolved;
//   this unit creates neither file.
//
// * ERROR TAXONOMY (RULE §2). The source throws five distinct things:
//   CatalogClosedError (:297), EntryAlreadyExists (:355), DBAlreadyExists
//   (:518), DbError via dbError (:372/:539/:551/:558), UnknownDatabase
//   (:307), plus untyped SomeException through the Store methods. The port
//   has ONE typed enum, `CatalogError`, with one arm per source throw and
//   a `Store` arm for the Store channel. `DBAlreadyExists` does NOT get its
//   own arm: its Exception instance is `toException = toDBError`
//   (Exception.hs:50-52, :39-40), so the source's thrown `SomeException`
//   is a DBError wrapper; row 53 already provides `From<DBAlreadyExists>
//   for DbError` and `TryFrom<&DbError> for DBAlreadyExists`, and
//   `start_restoring` goes through the `Db` arm so a consumer's
//   `catch \DBAlreadyExists` image (match `Db`, downcast) keeps working.
//   The db-error arm carries row 53's `DbError` (never a
//   re-implementation); `UnknownDatabase` is the generated IDL exception
//   (glean.thrift:243-246). `CatalogClosedError` and `EntryAlreadyExists`
//   are the source's own types, derived Show in the source -> Debug here
//   (RULE §2 Show stand-in); their Display impls reproduce the
//   constructor-shaped text (the inherent Rust-vs-Haskell punctuation
//   difference is noted at `EntryAlreadyExists`).
//
// * `uninterruptibleMask_` (:253) has NO Rust image. Haskell async
//   exceptions can interrupt a thread at almost any point; Rust threads are
//   not cancelled asynchronously, so the committer cannot be interrupted
//   except by process exit. The port therefore runs the plain loop and does
//   not fake a mask. The source's comment (:248-251: the thread should never
//   be cancelled because it must flush outstanding commits) is satisfied
//   structurally: nothing in the port cancels the thread; `close` waits
//   for it.
//
// * `bracket`/`bracket_`/`tryBracket` -> explicit pre/body/post.
//   `create` (:347-373) and `finishRestoring` (:527-551) use
//   `tryBracket`, whose `after` runs on both success and a `Left`
//   result; `delete` (:376-396) uses `bracket_`, whose release always
//   runs. The port writes these as: pre-transaction (outer lock), body
//   (Store call, NO lock), post-transaction (outer lock, always run), then
//   return the body's result. The body's outcome is a `Result`, exactly
//   the shape row 41's `try_bracket` models (backend/types.rs:720-745).
//   A PANIC inside a Store call would skip the post (the source's bracket
//   would run its release); the port treats a panicking Store impl as a
//   Step-4 defect — Store failures are `Result`s by RULE §2 — but still
//   releases the ephemeral reservation and the committing flag through
//   `EphemeralGuard`/`CommittingGuard` `Drop` impls, so the unwind cannot
//   wedge `close` or `delete`. The guards are the release-on-exception
//   image; the rest of the post transaction (live insertion, recalculation)
//   is skipped on a panic and is NOT rolled back (the sanctioned std::sync
//   divergence).
//
// * `create`'s post-transaction does NOT throw when the catalog is closed:
//   the source uses `modifyTVar' catEntries (fmap f)` (:360), which is a
//   no-op on `Nothing`, and `recalculateStatus` returns () on `Nothing`
//   (:171-173). Only the PRE-transaction (`getEntries`) throws
//   CatalogClosedError. The port reproduces that asymmetry: pre uses
//   `entries_mut()?`, post uses `if let Some(entries) = ...`.
//
// * `on_success` (:347, :364) runs AFTER the post-transaction and OUTSIDE
//   the outer lock. RULEBOOK §2 B1 (line 59) is the pin: source atomicity
//   is unreproducible in std::sync, and an in-lock call deadlocks on
//   re-entry, so the sanctioned divergence is out-of-lock. THE WINDOW: the
//   source runs the `STM ()` INSIDE the release transaction (:368), so a
//   concurrent reader cannot observe the promoted entry before the
//   callback's TVar updates commit; here a concurrent `close` can observe
//   `ephemeral` empty (and a concurrent reader can see the live entry)
//   before `on_success` runs. That is the callback-run/credit-not-yet-
//   released window B1 names. It runs only when the body succeeded (the
//   source's `forM_ r` over the `Right`). The only production caller,
//   Create.hs:151-155, touches only Env TVars (`envActive`, `dbState`,
//   `acquireDB` = `modifyTVar' (dbUsers db) (+1)`, Open.hs:433-434).
//
// * `modify_meta`'s callback also runs OUTSIDE the outer lock (B1). Because
//   it is a read-modify-write, not a notification, the port uses an
//   optimistic retry loop: snapshot the meta under the lock, run `f`
//   outside, reacquire and re-check that the entry is the same `Arc` and
//   the meta is unchanged, retrying otherwise. That is STM's transaction
//   retry, which is why the callback is `Fn` (it may run more than once)
//   and must be side-effect-free — exactly the discipline STM imposes on
//   an `STM` action. The signature still cannot express a THROWING
//   callback (`Meta -> STM Meta`); see the TODO at the function.
//
// * `db_failed` ORDERING (fidelity note, NOT a source defect). The source
//   writes `entriesFailed` (:470-473) BEFORE `getEntry` (:474). If the
//   repo is not live, `getEntry` throws UnknownDatabase and STM ROLLS THE
//   WRITE BACK. The Rust post-state must match the aborted transaction, so
//   the port validates `get_entry_locked` FIRST and only then inserts into
//   `failed`. Doing it in the source's order would leave a failed entry
//   for a repo that has no live entry — a state the source can never
//   produce.
//
// * `open`'s `fixIO` knot (:323-330) is unnecessary here. The source
//   ties the knot because the committer thread needs the whole `Catalog`
//   (including `catCommitter` itself). The Rust committer needs only
//   `Arc<CatalogInner>` (local store + state + condvar), which exists
//   before the thread is spawned, so the thread is spawned with a plain Arc
//   clone and the `Catalog` handle is built afterwards.
//
// * `close` takes `&self`. The source consumes the `Catalog` value
//   (`close :: Catalog -> IO ()`), but Env keeps the field and calls
//   `close` on it, so the Rust image borrows. It returns
//   `Result<(), CatalogError>` because the source's `getEntries` can
//   throw CatalogClosedError (closing twice). No `Drop` impl is added:
//   dropping a Catalog without closing leaves the committer thread running,
//   exactly as the source's async thread would keep running.
//
// * `EntryAlreadyExists` is a newtype over Repo (:151) and derives Eq/Show
//   in the source; the Rust image derives Debug/PartialEq/Eq and implements
//   Display as `EntryAlreadyExists(<Debug of Repo>)` — constructor-shaped,
//   per RULE §2's Show rule. Haskell's derived Show would print
//   `EntryAlreadyExists (Repo {repo_name = ..., repo_hash = ...})`; Rust's
//   Debug punctuation (parentheses without a space, field names without
//   underscores) differs inherently — the source type is an exception
//   payload, never a wire/log format, so this is the sanctioned
//   Show->Debug image, not a behavioral divergence.
//
// * `UTCTime` -> `SystemTime` (RULE §1 Time row / B3). `Maybe UTCTime`
//   -> `Option<SystemTime>`; `SystemTime` is Copy, so reads return it by
//   value.
//
// * `ItemStatus`/`Item`/`Filter`/`Locality` are row 49's shipped
//   types; this file uses them as-is (no re-declaration, no re-export).
//
// ---------------------------------------------------------------------------
// ASSUMED-IMPORT (RULE §4 B4): `glean_thrift`, EXTENDING the first sketch
// ---------------------------------------------------------------------------
//
// rust/glean-db/src/backend/types.rs:87-226 is the FIRST SKETCH of this
// crate's assumed `glean_thrift` image (flat crate root, IDL field names
// in snake_case, `optional` -> Option, `list` -> Vec, `bytestring` ->
// Vec<u8>, thrift `map` -> BTreeMap, struct Default = hsthrift `def`).
// Row 48's meta.rs:116-190 already sketches `DatabaseBroken` (:292-295),
// `DatabaseComplete` (:297-301), `DatabaseStatus` (:270-290) and
// `Database` (:303-342); those are NOT re-sketched here (consumer-
// extension rule). This unit extends the surface with the members its own
// code mentions. IDL cites are glean/if/glean.thrift; gen-hs2 cites are
// ~/glean-build/Glean/glean/if/gen-hs2/Glean/Types.hs.
//
//     #[derive(Clone, Debug, Default, PartialEq, Eq)]
//     pub struct Stacked {                   // :119-123, fields 1-3
//         pub name: String,                  // :120
//         pub hash: String,                  // :121
//         pub guid: Option<String>,          // :122 optional string
//     }
//
//     #[derive(Clone, Debug, Default, PartialEq, Eq)]
//     pub struct Pruned {                    // :126-131, fields 1-4
//         pub base: Repo,                    // :127
//         pub units: Vec<Vec<u8>>,           // :128 list<binary>
//         pub exclude: bool,                 // :129
//         pub guid: Option<String>,          // :130 optional string
//     }
//
//     #[derive(Clone, Debug, PartialEq, Eq)]
//     pub enum Dependencies {                // :134-137 union, (hs.nonempty)
//         Stacked(Stacked),                  // field id 1
//         Pruned(Pruned),                    // field id 2
//     }
//     // The Haskell union constructors are Dependencies_stacked /
//     // Dependencies_pruned; the Rust union image is the enum above
//     // (ASSUMED arm spelling; Step-4 compile check). (hs.nonempty) means
//     // the Haskell type has no unset arm; if the generated Rust union adds
//     // an Empty arm the match in recalculate_status_locked becomes a
//     // compile error at Step 4 — never a silent divergence.
//
//     #[derive(Clone, Debug, Default, PartialEq, Eq)]
//     pub struct UnknownDatabase {           // :243-246, fields 1-2
//         pub repo: Repo,                    // :244
//         pub call_stack: String,            // :245
//     }
//     // ASSUMED: the generated exception implements std::error::Error,
//     // Display and Debug (the generated-exception convention CatalogError
//     // delegates to). If it does not, the fallback is a Step-4 compile
//     // decision at this file's Display/source impls.
//
//     #[derive(Clone, Debug, Default, PartialEq, Eq)]
//     pub struct GetDatabaseResult {         // :869-874, fields 1-3
//         pub database: Database,            // :870
//         pub auth_status: Option<thrift::AuthStatus>, // :872 optional
//         pub auth_message: Option<String>,  // :874 optional
//     }
//     // Clone is load-bearing: get_local_database returns a lookup clone.
//
// `DatabaseStatus` is named only as a VALUE (`DatabaseStatus::Complete`,
// `::Available`, `::Incomplete`, `::Restoring`, `::Broken`,
// `::Missing`, `::Finalizing`) and is already sketched at meta.rs:150-167
// (arms Complete=0, Available=7, Incomplete=1, Restoring=2, Broken=3,
// Restorable=4, Finalizing=5, Missing=6; no `DatabaseStatus_` prefix).
// `Database` is only constructed by meta_to_thrift_database (row 48) and
// its `broken` field is written in get_local_databases.
//
// ---------------------------------------------------------------------------
// ASSUMED-IMPORT (RULE §4 B4): `crate::internal::types::Completeness`
// ---------------------------------------------------------------------------
//
// `item_status_for` matches the four real arms of the completeness union.
// The type is the generated `crate::internal::types::Completeness` whose
// shape (four Haskell arms + the fbthrift unset `Empty` arm) is FIRST
// sketched at rust/glean-db/src/database/meta.rs:216-236, together with the
// proof that `Empty` is unconstructible from the wire ((hs.nonempty);
// gen-hs2 :260-264, :320). meta.rs does not re-export the type, so this file
// imports it from its owner and does NOT re-sketch it:
//
//     use crate::internal::types::Completeness;
//
// The `Empty` arm is `unreachable!` with the same message meta.rs uses
// (meta.rs:596-597, :634-635): a fifth arm has no Haskell image, so it must
// never be mapped to an invented status.
//
// ---------------------------------------------------------------------------
// ASSUMED-IMPORT (RULE §1 logging row (A10)): the `glean_util::log` seam,
// EXTENDING send_batch.rs:150-172
// ---------------------------------------------------------------------------
//
// `import Util.Log` (Catalog.hs:75) has NO manifest unit. Unit 45 first
// sketched the seam (rust/glean-db/src/query/thrift/internal.rs:190-213:
// `pub fn vlog(level: i64, msg: impl FnOnce() -> String);`) and unit 46
// extended it with `log_info` (rust/glean-db/src/write/send_batch.rs:
// 150-172). This unit adds the third member, used twice by `commit`:
//
//     pub fn log_error(msg: impl FnOnce() -> String);
//
// Util.Log re-exports Util.Log.String's `logError` (Util.Log.hs:12-14);
// `logError :: String -> m ()` (Util.Log.String.hs:61-63) feeds
// `logCommon`, which calls `withCString msg` (:73-75) — so, unlike
// `vlog` (:36-41), the message is ALWAYS forced. The lazy-thunk signature
// is still the seam's shape (it preserves the call-site form and costs
// nothing), and the closure is forced immediately inside the call. Direct
// import, no stand-in (DECISIONS.md 2026-10-18, the units-45/46 convention):
//
//     use glean_util::log::log_error;
//
// The module does not exist on disk yet; that is the same forward-reference
// form `glean_thrift` is consumed by, and the swap when the log unit lands
// is import resolution only, never a call-site edit.
//
// ---------------------------------------------------------------------------
// EXPORT MAPPING (source export list :15-47, 31 names)
// ---------------------------------------------------------------------------
//
// CALL SYNTAX: every source function whose first argument is `Catalog` is an
// ASSOCIATED FUNCTION on `Catalog` (`Catalog::open`, `cat.close()`, ...).
// The right-hand column below names the source-level function; the method
// form is the Rust image of the source's `Catalog -> ...` first argument.
// `mk_entry` and `list_prime` are free functions (their source signatures
// take no Catalog).
//   :16 Catalog                 -> pub struct Catalog (fields PRIVATE: the
//                                  source exports the type abstractly)
//   :17 Entry                   -> pub struct Entry (fields PUBLIC: the
//                                  source exports EntryF(..) at :46)
//   :18 CatalogClosedError(..)  -> pub struct CatalogClosedError
//   :19 EntryAlreadyExists(..)  -> pub struct EntryAlreadyExists(pub Repo)
//   :20 open                    -> pub fn open
//   :21 close                   -> pub fn close
//   :22 create                  -> pub fn create
//   :23 delete                  -> pub fn delete
//   :24 list                    -> pub fn list
//   :25 listMostRecent          -> pub fn list_most_recent
//   :26 exists                  -> pub fn exists
//   :27 readMeta                -> pub fn read_meta
//   :28 writeMeta               -> pub fn write_meta
//   :29 modifyMeta              -> pub fn modify_meta
//   :30 readExpiring            -> pub fn read_expiring
//   :31 setExpiring             -> pub fn set_expiring
//   :32 unsetExpiring           -> pub fn unset_expiring
//   :33 startRestoring          -> pub fn start_restoring
//   :34 finishRestoring         -> pub fn finish_restoring
//   :35 abortRestoring          -> pub fn abort_restoring
//   :36 resetElsewhere          -> pub fn reset_elsewhere
//   :37 getLocalDatabases       -> pub fn get_local_databases
//   :38 getLocalDatabase        -> pub fn get_local_database
//   :39 dbFailed                -> pub fn db_failed
//   :40 resetFailed             -> pub fn reset_failed
//   :42 getEntries              -> pub fn get_entries
//   :43 list'                   -> pub fn list_prime
//   :44 Entries                 -> pub struct Entries (fields PUBLIC: the
//                                  source exports EntriesF(..) at :45)
//   :45 EntriesF(..)            -> collapsed into Entries (tests only)
//   :46 EntryF(..)              -> collapsed into Entry (tests only)
//   :47 mkEntry                 -> pub fn mk_entry
//
// Rust-only items (documented, not source exports): `EntryState` (the five
// TVars), `CatalogState`, `CatalogInner`, `CatalogError`, `lock` and
// the private helpers below.
//
// PRIVATE, as in the source (no export):
//   :166 recalculateStatus     -> recalculate_status_locked
//   :227 recalculateDepsStatus -> recalculate_deps_status_locked
//   :235 itemDatabaseStatus    -> item_database_status
//   :243 dirtyEntry            -> dirty_entry_locked
//   :252 commit                -> commit
//   :299 lookupEntry           -> lookup_entry
//   :302 getEntry              -> get_entry_locked
//   (where-block helpers of getLocalDatabases)
//     local_db, updateFailed's inline branch, restoring_db -> local_db,
//     restoring_db (free fns) and the inline failed-map branch
//   (source-internal `itemStatusFor`, :195-199) -> item_status_for
//   (Rust-only) `lock`, `CatalogState::entries/entries_mut`
//
// ---------------------------------------------------------------------------
// Source order is preserved in the body below, except for the `_locked`
// helpers and `list_prime`, which sit next to their callers.

pub mod filter;

pub mod store;

// The Catalog/Local/ namespace (rows 64/65): catalog/local.rs declares
// `pub mod files;` and `pub mod memory;`, resolving the two sibling
// ports.
pub mod local;

use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::JoinHandle;
use std::time::SystemTime;

use glean_thrift as thrift;
use glean_util::log::log_error;

use crate::database::catalog::filter::{
    group_f, limit_f, run_filter, sort_f, Filter, Item, ItemStatus, Locality, Order, CREATED_V,
    REPO_NAME_V,
};
use crate::database::catalog::store::{CatalogStore, Store, StoreError};
use crate::database::exception::{db_error, DBAlreadyExists, DbError};
use crate::database::meta::{meta_to_thrift_database, Meta};
use crate::database::repo::in_repo;
use crate::internal::types::Completeness;

// ---------------------------------------------------------------------------
// EntryF / Entry (Catalog.hs:89-107)
// ---------------------------------------------------------------------------

/// The five per-entry TVars (:95-105) collapsed into one guarded record, per
/// the inventory row. Field order follows the source:
///   entryStatus (:95)     -> status
///   entryMeta (:98)       -> meta
///   entryDirty (:99)      -> dirty
///   entryComitting (:102) -> committing (the source field name has one 'm')
///   entryExpiring (:105)  -> expiring
pub(crate) struct EntryState {
    pub(crate) status: ItemStatus,
    pub(crate) meta: Meta,
    pub(crate) dirty: bool,
    pub(crate) committing: bool,
    pub(crate) expiring: Option<SystemTime>,
}

/// `data EntryF ref` (:93-107) at `ref = TVar` (`type Entry = EntryF
/// TVar`, :89). The `ref` parameter is the test-only genericity
/// (:91-92) and is collapsed (RULE §4); the record fields are public
/// because the source exports `EntryF(..)` (:46).
pub struct Entry {
    pub repo: thrift::Repo,
    pub(crate) state: Mutex<EntryState>,
}

/// `data EntriesF entry` (:116-127) at `entry = Entry` (`type Entries =
/// EntriesF Entry`, :113). Fields are public because the source exports
/// `EntriesF(..)` (:45). `entriesFailed` stores the boxed
/// `SomeException` image (:126); the Arc lets the map be cloned with the
/// rest of `Entries`.
#[derive(Clone)]
pub struct Entries {
    /// available databases
    pub live_here: HashMap<thrift::Repo, Arc<Entry>>,
    /// available databases in another shard; ignored for all purposes
    /// except by getLocalDatabases
    pub live_elsewhere: HashMap<thrift::Repo, Arc<Entry>>,
    /// databases that are being restored
    pub restoring: HashMap<thrift::Repo, Meta>,
    /// databases that are being created or deleted
    pub ephemeral: HashSet<thrift::Repo>,
    /// databases that failed to open, and their exceptions
    pub failed: HashMap<thrift::Repo, Arc<dyn std::error::Error + Send + Sync>>,
}

// ---------------------------------------------------------------------------
// Catalog (Catalog.hs:133-143) — the existential's state
// ---------------------------------------------------------------------------

/// The mutable half of the source's `Catalog` record: `catEntries`
/// (:136), `catDirtyQueue` (:138) and `catRepoDependents` (:142),
/// behind ONE mutex (the row's clause). `entries == None` is the
/// closed/busy flag; open/close replace the whole Option.
pub(crate) struct CatalogState {
    /// all known entries (None means catalog is closed)
    pub(crate) entries: Option<Entries>,
    /// queue of entries which need to have their metadata committed
    pub(crate) dirty_queue: VecDeque<Arc<Entry>>,
    /// keep track of repos that depend on a repo
    pub(crate) repo_dependents: HashMap<thrift::Repo, Vec<thrift::Repo>>,
}

/// The non-guarded half of `Catalog`: the local store (the existential's
/// `catLocal`, :134) and the condition variable that replaces STM's
/// blocking `retry`. The Condvar lives OUTSIDE the mutex because
/// `Condvar::wait` consumes the guard.
///
/// The source's `catCommitter` (:140) is not here: it is the
/// `Catalog.committer` handle, and the thread itself receives an
/// `Arc<CatalogInner>` (see TRANSLATION NOTES on `fixIO`).
pub(crate) struct CatalogInner {
    local: CatalogStore,
    state: Mutex<CatalogState>,
    changed: Condvar,
}

/// `data Catalog = forall local. Store local => Catalog` (:133-143).
/// Fields are PRIVATE: the source exports `Catalog` abstractly (:16), so
/// the only way to obtain one is `open`.
pub struct Catalog {
    inner: Arc<CatalogInner>,
    committer: Mutex<Option<JoinHandle<()>>>,
}

impl CatalogState {
    /// The private `entries` helper: `getEntries` without the clone.
    /// Throws the source's CatalogClosedError (:297) when closed.
    fn entries(&self) -> Result<&Entries, CatalogError> {
        self.entries
            .as_ref()
            .ok_or(CatalogError::Closed(CatalogClosedError))
    }

    /// The private `entries_mut` helper (same throw).
    fn entries_mut(&mut self) -> Result<&mut Entries, CatalogError> {
        self.entries
            .as_mut()
            .ok_or(CatalogError::Closed(CatalogClosedError))
    }
}

/// The one lock helper (see TRANSLATION NOTES: poisoning).
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

// ---------------------------------------------------------------------------
// Errors (Catalog.hs:146-153)
// ---------------------------------------------------------------------------

/// `data CatalogClosedError = CatalogClosedError deriving (Show, Typeable)`
/// (:146-147). Display reproduces the derived Show text exactly.
#[derive(Debug)]
pub struct CatalogClosedError;

impl fmt::Display for CatalogClosedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CatalogClosedError")
    }
}

impl std::error::Error for CatalogClosedError {}

/// `newtype EntryAlreadyExists = EntryAlreadyExists Repo deriving (Eq, Show,
/// Typeable)` (:151-153). The field is public: the source exports
/// `EntryAlreadyExists(..)` (:19).
#[derive(Debug, PartialEq, Eq)]
pub struct EntryAlreadyExists(pub thrift::Repo);

/// Derived Show -> constructor-shaped Display (RULE §2). See TRANSLATION
/// NOTES for the inherent Haskell/Rust punctuation difference.
impl fmt::Display for EntryAlreadyExists {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EntryAlreadyExists({:?})", self.0)
    }
}

impl std::error::Error for EntryAlreadyExists {}

/// The area's typed channel for every throw in Catalog.hs (RULE §2
/// dynamic-exceptions row; the RunQueryError precedent,
/// rust/glean-db/src/query/thrift.rs:409-454). One arm per source throw:
///
///   :297 CatalogClosedError      -> Closed
///   :355 EntryAlreadyExists      -> AlreadyExists
///   :518 DBAlreadyExists         -> Db (via DbError::from; see below)
///   :372/:539/:551/:558 dbError  -> Db (row 53's DbError, the area funnel)
///   Store methods (untyped)      -> Store (row 50's StoreError)
///   :307 UnknownDatabase         -> UnknownDatabase (generated exception)
#[derive(Debug)]
pub enum CatalogError {
    /// `throwSTM CatalogClosedError` (:297) — getEntries on a closed
    /// catalog.
    Closed(CatalogClosedError),
    /// `throwSTM $ EntryAlreadyExists repo` (:355) — create on a repo that
    /// is live, ephemeral or restoring.
    AlreadyExists(EntryAlreadyExists),
    /// The dbError funnel (:372, :539, :551, :558) AND `throwM
    /// DBAlreadyExists` (:518): the source's Exception instance wraps it in
    /// a DBError (`toException = toDBError`, Exception.hs:50-52), so
    /// `start_restoring` returns `DbError::from(DBAlreadyExists)` here and
    /// row 53's `TryFrom<&DbError>` downcast is the consumer's image of
    /// `catch \DBAlreadyExists`.
    Db(DbError),
    /// Any Store method's failure (Store::list/create/delete/put).
    Store(StoreError),
    /// `CallStack.throwSTM $ Thrift.UnknownDatabase repo` (:307).
    UnknownDatabase(thrift::UnknownDatabase),
}

impl From<CatalogClosedError> for CatalogError {
    fn from(e: CatalogClosedError) -> Self {
        CatalogError::Closed(e)
    }
}

impl From<EntryAlreadyExists> for CatalogError {
    fn from(e: EntryAlreadyExists) -> Self {
        CatalogError::AlreadyExists(e)
    }
}

impl From<DbError> for CatalogError {
    fn from(e: DbError) -> Self {
        CatalogError::Db(e)
    }
}

impl From<StoreError> for CatalogError {
    fn from(e: StoreError) -> Self {
        CatalogError::Store(e)
    }
}

impl From<thrift::UnknownDatabase> for CatalogError {
    fn from(e: thrift::UnknownDatabase) -> Self {
        CatalogError::UnknownDatabase(e)
    }
}

/// Constructor-shaped Display, delegating to each payload's own rendering
/// (RULE §2's Show rule: no invented prose).
impl fmt::Display for CatalogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CatalogError::Closed(e) => fmt::Display::fmt(e, f),
            CatalogError::AlreadyExists(e) => fmt::Display::fmt(e, f),
            CatalogError::Db(e) => fmt::Display::fmt(e, f),
            CatalogError::Store(e) => fmt::Display::fmt(e, f),
            CatalogError::UnknownDatabase(e) => fmt::Display::fmt(e, f),
        }
    }
}

/// `source()` is the CAUSE chain, not the payload: the source's fieldless
/// exceptions have no cause (CatalogClosedError :146-147; EntryAlreadyExists
/// :151-153), so they return None. The three error-carrying variants expose
/// their payload.
impl std::error::Error for CatalogError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CatalogError::Closed(_) | CatalogError::AlreadyExists(_) => None,
            CatalogError::Db(e) => Some(e),
            CatalogError::Store(e) => Some(e),
            CatalogError::UnknownDatabase(e) => Some(e),
        }
    }
}

// ---------------------------------------------------------------------------
// mkEntry (Catalog.hs:156-163)
// ---------------------------------------------------------------------------

/// `mkEntry :: Repo -> Meta -> STM Entry` (:156-157): ItemMissing, the
/// given Meta, False, False, Nothing.
pub fn mk_entry(repo: thrift::Repo, meta: Meta) -> Arc<Entry> {
    Arc::new(Entry {
        repo,
        state: Mutex::new(EntryState {
            status: ItemStatus::ItemMissing,
            meta,
            dirty: false,
            committing: false,
            expiring: None,
        }),
    })
}

// ---------------------------------------------------------------------------
// recalculateStatus (Catalog.hs:166-225)
// ---------------------------------------------------------------------------

/// `itemStatusFor` (:195-199). The fifth `Completeness` arm is the
/// fbthrift unset arm with no Haskell image ((hs.nonempty)); see the
/// ASSUMED-IMPORT block.
fn item_status_for(completeness: &Completeness) -> ItemStatus {
    match completeness {
        Completeness::Complete(_) => ItemStatus::ItemComplete,
        Completeness::Incomplete(_) => ItemStatus::ItemIncomplete,
        Completeness::Broken(_) => ItemStatus::ItemBroken,
        Completeness::Finalizing(_) => ItemStatus::ItemFinalizing,
        Completeness::Empty => {
            unreachable!("Completeness::Empty has no Haskell image (hs.nonempty)")
        }
    }
}

/// `recalculateStatus :: Catalog -> Entry -> STM ()` (:166-167), the
/// `_locked` form: the caller holds the outer `CatalogState` guard. This
/// split is mandatory — `write_meta`/`modify_meta` call this while
/// already holding the guard, and a non-reentrant std Mutex would deadlock.
///
/// Reads the entry's meta first (:168), returns () if the catalog is closed
/// (:171-173), computes the dependency statuses (:190-201) and this
/// entry's status (:203-212), THEN maintains `repo_dependents` (:182-188),
/// and on a change writes the status and recurses into the dependents
/// (:218-225). The computation-before-mutation order is deliberate: the
/// `Completeness::Empty` `unreachable!` must fire before the first write,
/// so a panic cannot leave a half-updated dependents map (see POISONING).
fn recalculate_status_locked(inner: &CatalogInner, state: &mut CatalogState, entry: &Arc<Entry>) {
    let (repo, meta) = {
        let entry_state = lock(&entry.state);
        (entry.repo.clone(), entry_state.meta.clone())
    };

    // :181-188 — the dependencies are derived from the entry's meta.
    let dependencies: Vec<(thrift::Repo, Option<String>)> = match &meta.meta_dependencies {
        Some(thrift::Dependencies::Stacked(stacked)) => vec![(
            thrift::Repo {
                name: stacked.name.clone(),
                hash: stacked.hash.clone(),
            },
            stacked.guid.clone(),
        )],
        Some(thrift::Dependencies::Pruned(pruned)) => {
            vec![(pruned.base.clone(), pruned.guid.clone())]
        }
        None => Vec::new(),
    };

    let new_status = {
        let CatalogState {
            entries,
            repo_dependents,
            ..
        } = state;
        // :171-173 — a closed catalog is a no-op, not a throw.
        let Some(entries) = entries.as_ref() else {
            return;
        };

        // :175-180 — missingStatus.
        let missing_status = |r: &thrift::Repo| {
            if entries.restoring.contains_key(r) {
                ItemStatus::ItemRestoring
            } else {
                ItemStatus::ItemMissing
            }
        };

        // :190-201 — dependencyStatuses. Each dependency's lock is taken and
        // dropped in its own scope: never two entry locks at once.
        let mut dependency_statuses = Vec::with_capacity(dependencies.len());
        for (dependency, guid) in &dependencies {
            let missing = missing_status(dependency);
            match entries.live_here.get(dependency) {
                None => dependency_statuses.push(missing),
                Some(dependency_entry) => {
                    let (dependency_meta, dependency_status) = {
                        let dependency_state = lock(&dependency_entry.state);
                        (dependency_state.meta.clone(), dependency_state.status)
                    };
                    // :198 — HashMap.lookup "glean.guid" properties == guid.
                    // Both sides are Maybe: Nothing == Nothing means "no
                    // guid recorded on either side" and is a match.
                    if dependency_meta.meta_properties.get("glean.guid") == guid.as_ref() {
                        dependency_statuses.push(dependency_status);
                    } else {
                        dependency_statuses.push(missing);
                    }
                }
            }
        }

        // :203-212 — `live` is whether THIS entry is live-here.
        let live = entries.live_here.contains_key(&repo);
        let status = if entries.failed.contains_key(&repo) {
            ItemStatus::ItemBroken
        } else if live {
            item_status_for(&meta.meta_completeness)
        } else {
            missing_status(&repo)
        };
        // mconcat = fold with Semigroup max, mempty = ItemComplete.
        let new_status = dependency_statuses
            .into_iter()
            .fold(status, ItemStatus::combine);

        // :182-188 — repo_dependents. This is the FIRST mutation in this
        // function: every fallible computation above has completed.
        for (dependency, _guid) in &dependencies {
            if live {
                // HashMap.insertWith (<>) dep [repo] == [repo] <> old:
                // PREPEND. Duplicates are possible in the source and are
                // reproduced.
                repo_dependents
                    .entry(dependency.clone())
                    .or_default()
                    .insert(0, repo.clone());
            } else if let Some(dependents) = repo_dependents.get_mut(dependency) {
                // HashMap.adjust (filter (repo /=)): only if the key exists,
                // and an emptied list stays present.
                dependents.retain(|r| *r != repo);
            }
        }

        new_status
    };

    let old_status = { lock(&entry.state).status };
    if new_status != old_status {
        {
            let mut entry_state = lock(&entry.state);
            entry_state.status = new_status;
        }
        recalculate_deps_status_locked(inner, state, &repo);
    }
}

// ---------------------------------------------------------------------------
// recalculateDepsStatus (Catalog.hs:227-232)
// ---------------------------------------------------------------------------

/// `recalculateDepsStatus :: Catalog -> Repo -> STM ()` (:227-228), the
/// `_locked` form. `HashMap.lookupDefault [] repo repoDependents` then
/// `HashMap.lookup dep . entriesLiveHere`; duplicates in the dependents
/// list are preserved (the same entry is recalculated once per duplicate).
fn recalculate_deps_status_locked(
    inner: &CatalogInner,
    state: &mut CatalogState,
    repo: &thrift::Repo,
) {
    // The Vec is cloned because recalculate_status_locked needs `&mut state`;
    // the clone also preserves the source's duplicate entries.
    let dependents = state.repo_dependents.get(repo).cloned().unwrap_or_default();
    let dependent_entries: Vec<Arc<Entry>> = {
        // :229-230 — `entries >>= ...`: Nothing means no iteration, no throw.
        let Some(entries) = state.entries.as_ref() else {
            return;
        };
        dependents
            .iter()
            .filter_map(|dependent| entries.live_here.get(dependent).cloned())
            .collect()
    };
    for dependent in dependent_entries {
        recalculate_status_locked(inner, state, &dependent);
    }
}

// ---------------------------------------------------------------------------
// itemDatabaseStatus (Catalog.hs:235-241)
// ---------------------------------------------------------------------------

/// `itemDatabaseStatus :: ItemStatus -> Thrift.DatabaseStatus` (:235-241).
/// The generated Rust arms carry no `DatabaseStatus_` prefix (meta.rs:150).
fn item_database_status(status: ItemStatus) -> thrift::DatabaseStatus {
    match status {
        ItemStatus::ItemComplete => thrift::DatabaseStatus::Complete,
        ItemStatus::ItemIncomplete => thrift::DatabaseStatus::Incomplete,
        ItemStatus::ItemRestoring => thrift::DatabaseStatus::Restoring,
        ItemStatus::ItemBroken => thrift::DatabaseStatus::Broken,
        ItemStatus::ItemMissing => thrift::DatabaseStatus::Missing,
        ItemStatus::ItemFinalizing => thrift::DatabaseStatus::Finalizing,
    }
}

// ---------------------------------------------------------------------------
// dirtyEntry (Catalog.hs:243-247)
// ---------------------------------------------------------------------------

/// `dirtyEntry :: Catalog -> Entry -> STM ()` (:243-244), the `_locked`
/// form: `swapTVar (entryDirty entry) True` then, only if it was False,
/// `writeTQueue (catDirtyQueue cat) entry`. The swap-before-push order is
/// preserved (two concurrent dirtying calls push once).
fn dirty_entry_locked(inner: &CatalogInner, state: &mut CatalogState, entry: &Arc<Entry>) {
    let was_dirty = {
        let mut entry_state = lock(&entry.state);
        let was_dirty = entry_state.dirty;
        entry_state.dirty = true;
        was_dirty
    };
    if !was_dirty {
        state.dirty_queue.push_back(Arc::clone(entry));
        inner.changed.notify_all();
    }
}

// ---------------------------------------------------------------------------
// bracket-release guards (Catalog.hs:256-269, :348-373, :377-396, :528-552)
// ---------------------------------------------------------------------------

/// The `bracket` release at :269 (`writeTVar (entryComitting entry) False`)
/// on the UNWIND path. The normal path calls `drop(guard)` at the point the
/// source's bracket runs it (after the result handling and any re-dirty).
/// The release takes the OUTER lock: delete tests `committing` under that
/// lock and waits on `changed`, so the reset must be made and notified under
/// the same mutex or the wakeup can be lost between the check and the wait.
struct CommittingGuard<'a> {
    inner: &'a CatalogInner,
    entry: Arc<Entry>,
}

impl Drop for CommittingGuard<'_> {
    fn drop(&mut self) {
        {
            let _state = lock(&self.inner.state);
            lock(&self.entry.state).committing = false;
        }
        self.inner.changed.notify_all();
    }
}

/// The `tryBracket`/`bracket_` release of the ephemeral reservation
/// (:358-368, :393-394, :540-548) on the UNWIND path. The normal path
/// removes the repo inside the post transaction (atomically with the live
/// insertion), which makes the Drop removal a no-op. Without the guard a
/// panic in a Store call would leave `ephemeral` non-empty forever and
/// `close`'s retry (:339-341) would wait forever.
struct EphemeralGuard<'a> {
    inner: &'a CatalogInner,
    repo: &'a thrift::Repo,
}

impl Drop for EphemeralGuard<'_> {
    fn drop(&mut self) {
        {
            let mut state = lock(&self.inner.state);
            if let Some(entries) = state.entries.as_mut() {
                entries.ephemeral.remove(self.repo);
            }
        }
        self.inner.changed.notify_all();
    }
}

// ---------------------------------------------------------------------------
// commit (Catalog.hs:252-290) — the committer thread
// ---------------------------------------------------------------------------

/// `commit :: Catalog -> IO ()` (:252-290). Runs on its own thread until
/// the catalog closes and the queue drains.
///
/// Source shape: `uninterruptibleMask_ loop`; each iteration is a
/// `bracket` that (a) dequeues one entry and marks it committing (or stops
/// when the catalog is closed), (b) clears `dirty`, reads the meta, calls
/// `Store.put` OUTSIDE the locks, then handles the result, and (c) resets
/// `committing`. The explicit arms below are that bracket; see
/// TRANSLATION NOTES for `uninterruptibleMask_` and the panic note.
fn commit(inner: Arc<CatalogInner>) {
    loop {
        // (a) Dequeue + mark committing. This is ONE critical section in the
        // source (readTQueue `orElse` close check, plus writeTVar
        // committing), so the outer lock is held across the entry lock:
        // outer before inner, always.
        let entry = {
            let mut state = lock(&inner.state);
            loop {
                if let Some(entry) = state.dirty_queue.pop_front() {
                    {
                        let mut entry_state = lock(&entry.state);
                        entry_state.committing = true;
                    }
                    break entry;
                }
                // :265 — the catalog is open and the queue is empty: retry.
                if state.entries.is_none() {
                    // :266-267 — closed and drained: stop the thread.
                    return;
                }
                state = inner
                    .changed
                    .wait(state)
                    .unwrap_or_else(PoisonError::into_inner);
            }
        };

        // The source's bracket release (:269) is panic-safe via the guard;
        // the normal path drops it at (c).
        let committing_guard = CommittingGuard {
            inner: &inner,
            entry: Arc::clone(&entry),
        };

        // (b) Clear dirty and take the meta (:272-274). Entry-local in the
        // source; no outer lock.
        let (repo, meta) = {
            let mut entry_state = lock(&entry.state);
            entry_state.dirty = false;
            (entry.repo.clone(), entry_state.meta.clone())
        };

        // The store call happens OUTSIDE every catalog lock (:276), so a slow
        // store cannot block the catalog and a failing store cannot poison a
        // half-applied transaction.
        let result = inner.local.put(&repo, &meta);

        // (b') The source handles the result INSIDE the bracket body, i.e.
        // BEFORE the committing reset (:278-289).
        match result {
            Ok(true) => {}
            Ok(false) => {
                // "couldn't commit meta because entry no longer exists"
                log_error(|| in_repo(&repo, "couldn't commit meta because entry no longer exists"));
            }
            Err(e) => {
                // "couldn't commit meta: " ++ show exc. The source's `exc`
                // is a SomeException over a Store-impl IO exception; the
                // Rust type is the port-only StoreError, whose
                // constructor-shaped Display (store.rs) is the chosen Show
                // image. Diagnostic only (not a wire field).
                log_error(|| in_repo(&repo, &format!("couldn't commit meta: {}", e)));
                let mut state = lock(&inner.state);
                dirty_entry_locked(&inner, &mut state, &entry);
            }
        }

        // (c) Reset committing (:269) and wake the delete waiters. This is
        // the source's bracket release, placed after the result handling
        // (the guard's Drop covers the unwind path with the same body).
        drop(committing_guard);
    }
}

// ---------------------------------------------------------------------------
// getEntries / lookupEntry / getEntry (Catalog.hs:292-307)
// ---------------------------------------------------------------------------

/// `lookupEntry :: Catalog -> Repo -> STM (Maybe Entry)` (:299-300).
/// Private in the source. Takes the guard: callers that already hold it use
/// this directly, so the outer mutex is never locked twice.
fn lookup_entry(
    state: &CatalogState,
    repo: &thrift::Repo,
) -> Result<Option<Arc<Entry>>, CatalogError> {
    Ok(state.entries()?.live_here.get(repo).cloned())
}

/// `getEntry :: HasCallStack => Catalog -> Repo -> STM Entry` (:302-307):
/// UnknownDatabase when the repo is not live-here. The Haskell CallStack is
/// not captured (Rust has no call-stack capture); the field is diagnostic
/// only, so it is left empty.
fn get_entry_locked(state: &CatalogState, repo: &thrift::Repo) -> Result<Arc<Entry>, CatalogError> {
    match lookup_entry(state, repo)? {
        Some(entry) => Ok(entry),
        None => Err(CatalogError::UnknownDatabase(thrift::UnknownDatabase {
            repo: repo.clone(),
            call_stack: String::new(),
        })),
    }
}

// ---------------------------------------------------------------------------
// The Catalog API
// ---------------------------------------------------------------------------

impl Catalog {
    /// `getEntries :: Catalog -> STM Entries` (:292-297). Public because
    /// the source exports it "for testing" (:42). The value is OWNED: the
    /// source shares the transaction's snapshot, the port clones it (Arc
    /// handles for the entries, owned Metas for the restoring map).
    // PERF(port): ownership forces this snapshot copy (the source shares the
    // transaction's value). The fast version is a shared snapshot
    // (Arc<Entries>) once the signature may change. This is the only PERF
    // marker in the file: read_meta's Meta clone and list_prime's per-Item
    // Meta clones are forced by their owned return types, not deferrals.
    pub fn get_entries(&self) -> Result<Entries, CatalogError> {
        let state = lock(&self.inner.state);
        state.entries().cloned()
    }

    /// `open :: Store local => local -> IO Catalog` (:310-335). Lists the
    /// store, builds one Entry per meta, starts the committer thread, then
    /// recalculates every live entry's status (:331-333).
    pub fn open(local: CatalogStore) -> Result<Catalog, CatalogError> {
        let metas = local.list()?;
        let live: HashMap<thrift::Repo, Arc<Entry>> = metas
            .into_iter()
            .map(|(repo, meta)| {
                let entry = mk_entry(repo.clone(), meta);
                (repo, entry)
            })
            .collect();
        let state = CatalogState {
            entries: Some(Entries {
                live_here: live,
                live_elsewhere: HashMap::new(),
                restoring: HashMap::new(),
                ephemeral: HashSet::new(),
                failed: HashMap::new(),
            }),
            dirty_queue: VecDeque::new(),
            repo_dependents: HashMap::new(),
        };
        let inner = Arc::new(CatalogInner {
            local,
            state: Mutex::new(state),
            changed: Condvar::new(),
        });

        // :323-330 — the source's fixIO knot is unnecessary: the thread only
        // needs the inner state, which already exists. `mask_ $ async` is a
        // plain spawn (the thread has no async-exception mask to reproduce).
        let committer = {
            let inner = Arc::clone(&inner);
            std::thread::spawn(move || commit(inner))
        };
        let catalog = Catalog {
            inner,
            committer: Mutex::new(Some(committer)),
        };

        // :331-333 — one transaction per entry, exactly as the source's
        // forM_ over HashMap.elems.
        let entries: Vec<Arc<Entry>> = {
            let state = lock(&catalog.inner.state);
            state
                .entries
                .as_ref()
                .map(|entries| entries.live_here.values().cloned().collect())
                .unwrap_or_default()
        };
        for entry in entries {
            let mut state = lock(&catalog.inner.state);
            recalculate_status_locked(&catalog.inner, &mut state, &entry);
        }
        Ok(catalog)
    }

    /// `close :: Catalog -> IO ()` (:337-343): wait until no ephemeral
    /// entry remains, mark the catalog closed, then wait for the committer.
    /// Takes `&self` (the source consumes the value; Env keeps the field).
    pub fn close(&self) -> Result<(), CatalogError> {
        {
            let mut state = lock(&self.inner.state);
            loop {
                if state.entries.is_none() {
                    return Err(CatalogError::Closed(CatalogClosedError));
                }
                let ephemeral_empty = state
                    .entries
                    .as_ref()
                    .map(|entries| entries.ephemeral.is_empty())
                    .unwrap_or(false);
                if ephemeral_empty {
                    break;
                }
                state = self
                    .inner
                    .changed
                    .wait(state)
                    .unwrap_or_else(PoisonError::into_inner);
            }
            // :342 — writeTVar catEntries Nothing.
            state.entries = None;
        }
        self.inner.changed.notify_all();
        // :343 — void $ waitCatch $ catCommitter cat: the join result
        // (including a panicked committer) is discarded.
        let committer = {
            let mut handle = lock(&self.committer);
            handle.take()
        };
        if let Some(committer) = committer {
            let _ = committer.join();
        }
        Ok(())
    }

    /// `create :: Catalog -> Repo -> Meta -> STM () -> IO ()` (:347-373),
    /// the `tryBracket` image. The pre-transaction reserves the repo as
    /// ephemeral; the body creates it in the store; the post-transaction
    /// always removes the reservation and promotes the entry only on
    /// success.
    ///
    /// `on_success` runs AFTER the post-transaction and OUTSIDE the lock
    /// (RULEBOOK §2 B1: source atomicity is unreproducible in std::sync and
    /// in-lock calls deadlock on re-entry). THE WINDOW: the source runs the
    /// `STM ()` inside the release transaction (:368), so a concurrent
    /// `close` here can observe `ephemeral` empty — and a reader can see the
    /// promoted entry — before `on_success` runs. `meta` is borrowed and
    /// cloned once into the Entry (the source shares the immutable value;
    /// the Store trait also takes `&Meta`).
    pub fn create<F>(
        &self,
        repo: &thrift::Repo,
        meta: &Meta,
        on_success: F,
    ) -> Result<(), CatalogError>
    where
        F: FnOnce(),
    {
        // :349-357 — pre-transaction.
        {
            let mut state = lock(&self.inner.state);
            let entries = state.entries_mut()?;
            if entries.live_here.contains_key(repo)
                || entries.ephemeral.contains(repo)
                || entries.restoring.contains_key(repo)
            {
                return Err(CatalogError::AlreadyExists(EntryAlreadyExists(
                    repo.clone(),
                )));
            }
            entries.ephemeral.insert(repo.clone());
        }
        // The bracket release on the unwind path; the normal path removes the
        // reservation in the post block below (the Drop is then a no-op).
        let ephemeral_guard = EphemeralGuard {
            inner: &self.inner,
            repo,
        };

        // :367-373 — body. A false return is NOT EntryAlreadyExists: it is
        // the dbError the source raises.
        let result: Result<(), CatalogError> = match self.inner.local.create(repo, meta) {
            Ok(true) => Ok(()),
            Ok(false) => db_error(repo, "database already exists in local metadata store")
                .map_err(CatalogError::from),
            Err(e) => Err(CatalogError::from(e)),
        };

        // :358-368 — post-transaction, ALWAYS. `modifyTVar' catEntries (fmap
        // f)` is a no-op on a closed catalog, so this block does not throw.
        let entry = mk_entry(repo.clone(), meta.clone());
        {
            let mut state = lock(&self.inner.state);
            if let Some(entries) = state.entries.as_mut() {
                entries.ephemeral.remove(repo);
                if result.is_ok() {
                    entries.live_here.insert(repo.clone(), Arc::clone(&entry));
                }
            }
            recalculate_status_locked(&self.inner, &mut state, &entry);
        }
        // Ephemeral changed: wake close's wait for an empty ephemeral set.
        self.inner.changed.notify_all();
        // :364 — forM_ r $ const on_success: only on the Right, and OUTSIDE
        // the lock per RULEBOOK §2 B1. See the B1 window note in the header.
        if result.is_ok() {
            on_success();
        }
        drop(ephemeral_guard);
        result
    }

    /// `delete :: Catalog -> Repo -> IO ()` (:376-396), the
    /// `bracket_` image. The pre-transaction waits for the committer to
    /// finish with the entry, drops it from the dirty queue if queued,
    /// removes it from live-here and marks it ephemeral; the body deletes
    /// from the store; the post always clears the ephemeral mark.
    pub fn delete(&self, repo: &thrift::Repo) -> Result<(), CatalogError> {
        {
            let mut state = lock(&self.inner.state);
            loop {
                let entry = get_entry_locked(&state, repo)?;
                let committing = { lock(&entry.state).committing };
                if committing {
                    // :380-381 — retry; the retry re-runs getEntry too.
                    state = self
                        .inner
                        .changed
                        .wait(state)
                        .unwrap_or_else(PoisonError::into_inner);
                    continue;
                }
                let dirty = { lock(&entry.state).dirty };
                if dirty {
                    // :382-386 — flushTQueue, filter, re-push: VecDeque::
                    // retain preserves the queue's order and removes every
                    // occurrence of this repo. The dirty flag is NOT cleared
                    // (the source does not clear it either).
                    state.dirty_queue.retain(|queued| &queued.repo != repo);
                }
                {
                    let entries = state.entries_mut()?;
                    entries.live_here.remove(repo);
                    entries.ephemeral.insert(repo.clone());
                }
                recalculate_status_locked(&self.inner, &mut state, &entry);
                break;
            }
        }
        // The bracket release on the unwind path; the post below removes the
        // reservation on the normal path (the Drop is then a no-op).
        let ephemeral_guard = EphemeralGuard {
            inner: &self.inner,
            repo,
        };
        // Ephemeral changed (the repo is now being deleted).
        self.inner.changed.notify_all();

        // :396 — void $ Store.delete catLocal repo: the Bool is ignored, an
        // error propagates.
        let result = self.inner.local.delete(repo);

        // :393-394 — post, ALWAYS (bracket_ release).
        {
            let mut state = lock(&self.inner.state);
            if let Some(entries) = state.entries.as_mut() {
                entries.ephemeral.remove(repo);
            }
        }
        self.inner.changed.notify_all();
        drop(ephemeral_guard);
        result.map(|_| ()).map_err(CatalogError::from)
    }

    /// `list :: Catalog -> [Locality] -> Filter () -> STM [Item]`
    /// (:399-400). `getEntries` then `list' readTVar`; the read function
    /// collapses because EntryState is concrete.
    ///
    /// ATOMICITY: the whole item snapshot is collected under ONE outer-lock
    /// acquisition, exactly the source's single transaction (map membership
    /// and every entry's status/meta are read together). The filter then
    /// runs outside the lock (B1: it is user code and must not run in-lock).
    pub fn list(&self, locs: &[Locality], f: Filter<()>) -> Result<Vec<Item>, CatalogError> {
        let items = {
            let state = lock(&self.inner.state);
            collect_items(state.entries()?, locs)
        };
        Ok(run_filter(f, items))
    }

    /// `listMostRecent :: Catalog -> IO [Item]` (:426-430) — group by repo
    /// name, newest first, one per group.
    pub fn list_most_recent(&self) -> Result<Vec<Item>, CatalogError> {
        let f = group_f(
            REPO_NAME_V,
            sort_f(CREATED_V, Order::Descending).and_then(|()| limit_f(1)),
        );
        self.list(&[Locality::Local, Locality::Restoring, Locality::Cloud], f)
    }

    /// `exists :: Catalog -> [Locality] -> Repo -> STM Bool` (:433-439).
    /// Cloud is always False (:437) — the elsewhere map is not consulted.
    pub fn exists(&self, locs: &[Locality], repo: &thrift::Repo) -> Result<bool, CatalogError> {
        let state = lock(&self.inner.state);
        let entries = state.entries()?;
        Ok(locs.iter().any(|loc| match loc {
            Locality::Local => entries.live_here.contains_key(repo),
            Locality::Restoring => entries.restoring.contains_key(repo),
            Locality::Cloud => false,
        }))
    }

    /// `readMeta :: Catalog -> Repo -> STM Meta` (:442-445).
    pub fn read_meta(&self, repo: &thrift::Repo) -> Result<Meta, CatalogError> {
        let state = lock(&self.inner.state);
        let entry = get_entry_locked(&state, repo)?;
        let meta = { lock(&entry.state).meta.clone() };
        Ok(meta)
    }

    /// `writeMeta :: Catalog -> Repo -> Meta -> STM ()` (:448-453):
    /// write, recalculate, dirty — in that order.
    pub fn write_meta(&self, repo: &thrift::Repo, meta: Meta) -> Result<(), CatalogError> {
        let mut state = lock(&self.inner.state);
        let entry = get_entry_locked(&state, repo)?;
        {
            let mut entry_state = lock(&entry.state);
            entry_state.meta = meta;
        }
        recalculate_status_locked(&self.inner, &mut state, &entry);
        dirty_entry_locked(&self.inner, &mut state, &entry);
        Ok(())
    }

    /// `modifyMeta :: Catalog -> Repo -> (Meta -> STM Meta) -> STM Meta`
    /// (:456-464).
    ///
    /// The callback runs OUTSIDE the outer lock (RULEBOOK §2 B1). Because it
    /// is a read-modify-write rather than a notification, the atomicity is
    /// recovered with an optimistic retry loop: snapshot the meta under the
    /// lock, run `f` outside, reacquire and re-check that the entry is the
    /// same `Arc` and its meta is unchanged, retrying otherwise. That is
    /// STM's transaction retry (a transaction may be re-run on conflict),
    /// which is why the callback is `Fn` and must be side-effect-free.
    // TODO(port): the source's callback type is `Meta -> STM Meta`, so it
    // can throw; three in-tree callbacks do — Finish.hs:44 (`throwM $
    // DatabaseNotIncomplete`), Finish.hs:100 (`throwM $ Thrift.Exception`)
    // and Create.hs:328 (`throwSTM $ Thrift.Exception`). The pinned
    // `Fn(Meta) -> Meta` signature cannot express them, so those consumer
    // units must hoist the check outside the callback — which loses the
    // source's check-and-write atomicity (a concurrent modify can land
    // between the check and the update). Revisit the signature at those
    // units: a `Result`-returning callback would need a CatalogError arm
    // for the generated exceptions. Catalog-reading callbacks are safe here
    // (they run outside the lock), but the retry may run them more than
    // once.
    pub fn modify_meta<F>(&self, repo: &thrift::Repo, f: F) -> Result<Meta, CatalogError>
    where
        F: Fn(Meta) -> Meta,
    {
        loop {
            // (1) Snapshot under the lock, then release it.
            let (entry, old_meta) = {
                let state = lock(&self.inner.state);
                let entry = get_entry_locked(&state, repo)?;
                let old_meta = { lock(&entry.state).meta.clone() };
                (entry, old_meta)
            };
            // (2) User code outside every lock (B1).
            let new_meta = f(old_meta.clone());
            // (3) Reacquire and re-check; retry on conflict (STM re-runs).
            let mut state = lock(&self.inner.state);
            let current_entry = get_entry_locked(&state, repo)?;
            if !Arc::ptr_eq(&entry, &current_entry) || lock(&current_entry.state).meta != old_meta {
                continue;
            }
            // (4) Commit the update.
            {
                let mut entry_state = lock(&current_entry.state);
                entry_state.meta = new_meta.clone();
            }
            recalculate_status_locked(&self.inner, &mut state, &current_entry);
            dirty_entry_locked(&self.inner, &mut state, &current_entry);
            return Ok(new_meta);
        }
    }

    /// `dbFailed :: Catalog -> Repo -> SomeException -> STM ()` (:466-474).
    /// FIDELITY: validate getEntry BEFORE writing the failed map; the
    /// source's write is rolled back by STM when getEntry throws (see
    /// TRANSLATION NOTES).
    pub fn db_failed(
        &self,
        repo: &thrift::Repo,
        exception: impl Into<Box<dyn std::error::Error + Send + Sync>>,
    ) -> Result<(), CatalogError> {
        let mut state = lock(&self.inner.state);
        let entry = get_entry_locked(&state, repo)?;
        {
            let entries = state.entries_mut()?;
            let boxed: Box<dyn std::error::Error + Send + Sync> = exception.into();
            entries.failed.insert(repo.clone(), Arc::from(boxed));
        }
        recalculate_status_locked(&self.inner, &mut state, &entry);
        Ok(())
    }

    /// `resetFailed :: Catalog -> STM ()` (:476-480) — clear the map; no
    /// recalculation in the source.
    pub fn reset_failed(&self) -> Result<(), CatalogError> {
        let mut state = lock(&self.inner.state);
        state.entries_mut()?.failed.clear();
        Ok(())
    }

    /// `readExpiring :: Catalog -> Repo -> STM (Maybe UTCTime)` (:484-485).
    pub fn read_expiring(&self, repo: &thrift::Repo) -> Result<Option<SystemTime>, CatalogError> {
        let state = lock(&self.inner.state);
        let entry = get_entry_locked(&state, repo)?;
        let expiring = { lock(&entry.state).expiring };
        Ok(expiring)
    }

    /// `setExpiring :: Catalog -> Repo -> UTCTime -> STM ()` (:489-490).
    pub fn set_expiring(&self, repo: &thrift::Repo, time: SystemTime) -> Result<(), CatalogError> {
        let state = lock(&self.inner.state);
        let entry = get_entry_locked(&state, repo)?;
        {
            let mut entry_state = lock(&entry.state);
            entry_state.expiring = Some(time);
        }
        Ok(())
    }

    /// `unsetExpiring :: Catalog -> Repo -> STM ()` (:495-496).
    pub fn unset_expiring(&self, repo: &thrift::Repo) -> Result<(), CatalogError> {
        let state = lock(&self.inner.state);
        let entry = get_entry_locked(&state, repo)?;
        {
            let mut entry_state = lock(&entry.state);
            entry_state.expiring = None;
        }
        Ok(())
    }

    /// `startRestoring :: Catalog -> Repo -> Meta -> STM ()` (:511-523):
    /// record the intent, storing the Meta VERBATIM, then recalculate the
    /// dependents (not this repo — it is not live). Throws DBAlreadyExists
    /// when live/restoring/ephemeral.
    pub fn start_restoring(&self, repo: &thrift::Repo, meta: Meta) -> Result<(), CatalogError> {
        let mut state = lock(&self.inner.state);
        {
            let entries = state.entries_mut()?;
            if entries.live_here.contains_key(repo)
                || entries.restoring.contains_key(repo)
                || entries.ephemeral.contains(repo)
            {
                // Source `throwM DBAlreadyExists` (:518): its Exception
                // instance is `toException = toDBError` (Exception.hs:50-52),
                // so the thrown value is a DBError wrapper. Row 53 owns the
                // conversion and the downcast.
                return Err(CatalogError::Db(DbError::from(DBAlreadyExists)));
            }
            entries.restoring.insert(repo.clone(), meta);
        }
        recalculate_deps_status_locked(&self.inner, &mut state, repo);
        Ok(())
    }

    /// `finishRestoring :: Catalog -> Repo -> IO ()` (:527-551), the
    /// `tryBracket` image: promote restoring -> ephemeral and take the
    /// stored Meta; create in the store; always clear ephemeral and promote
    /// to live on success.
    pub fn finish_restoring(&self, repo: &thrift::Repo) -> Result<(), CatalogError> {
        // :529-539 — pre-transaction.
        let meta = {
            let mut state = lock(&self.inner.state);
            let entries = state.entries_mut()?;
            match entries.restoring.remove(repo) {
                Some(meta) => {
                    entries.ephemeral.insert(repo.clone());
                    meta
                }
                None => {
                    return db_error(repo, "finishRestoring: unknown database")
                        .map_err(CatalogError::from)
                }
            }
        };
        // The bracket release on the unwind path; the post below removes the
        // reservation on the normal path (the Drop is then a no-op).
        let ephemeral_guard = EphemeralGuard {
            inner: &self.inner,
            repo,
        };
        // Ephemeral changed (the restore is now being promoted).
        self.inner.changed.notify_all();

        // :547-551 — body.
        let result: Result<(), CatalogError> = match self.inner.local.create(repo, &meta) {
            Ok(true) => Ok(()),
            Ok(false) => db_error(
                repo,
                "finishRestoring: entry already exists in local metadata store",
            )
            .map_err(CatalogError::from),
            Err(e) => Err(CatalogError::from(e)),
        };

        // :540-548 — post, ALWAYS (a closed catalog is a no-op, as in
        // create's post).
        let entry = mk_entry(repo.clone(), meta);
        {
            let mut state = lock(&self.inner.state);
            if let Some(entries) = state.entries.as_mut() {
                entries.ephemeral.remove(repo);
                if result.is_ok() {
                    entries.live_here.insert(repo.clone(), Arc::clone(&entry));
                }
            }
            recalculate_status_locked(&self.inner, &mut state, &entry);
        }
        // Ephemeral changed: wake close's wait for an empty ephemeral set.
        self.inner.changed.notify_all();
        drop(ephemeral_guard);
        result
    }

    /// `abortRestoring :: Catalog -> Repo -> STM ()` (:555-563): unknown
    /// database if not restoring, else drop it and recalculate dependents.
    pub fn abort_restoring(&self, repo: &thrift::Repo) -> Result<(), CatalogError> {
        let mut state = lock(&self.inner.state);
        {
            let entries = state.entries_mut()?;
            if !entries.restoring.contains_key(repo) {
                return db_error(repo, "abortRestoring: unknown database")
                    .map_err(CatalogError::from);
            }
            entries.restoring.remove(repo);
        }
        recalculate_deps_status_locked(&self.inner, &mut state, repo);
        Ok(())
    }

    /// `resetElsewhere :: Catalog -> [Item] -> STM ()` (:566-576): rebuild
    /// live-elsewhere from the items; HashMap.fromList keeps the LAST
    /// duplicate.
    pub fn reset_elsewhere(&self, items: &[Item]) -> Result<(), CatalogError> {
        let mut elsewhere: HashMap<thrift::Repo, Arc<Entry>> = HashMap::new();
        for item in items {
            let entry = mk_entry(item.item_repo.clone(), item.item_meta.clone());
            elsewhere.insert(item.item_repo.clone(), entry);
        }
        let mut state = lock(&self.inner.state);
        state.entries_mut()?.live_elsewhere = elsewhere;
        Ok(())
    }

    /// `getLocalDatabases :: Catalog -> STM (HashMap Repo
    /// Thrift.GetDatabaseResult)` (:578-628).
    ///
    /// Merge precedence is the source's Data.HashMap Monoid (left-biased
    /// union): elsewhere > local > restoring. The failed-map override is
    /// applied to the LOCAL map only (:594-604).
    ///
    /// ATOMICITY: the entries map, every entry's meta/status/expiring and the
    /// failed map are read under ONE outer-lock acquisition, the source's
    /// single transaction (:578-589). No user code runs here, so the guard is
    /// held throughout.
    pub fn get_local_databases(
        &self,
    ) -> Result<HashMap<thrift::Repo, thrift::GetDatabaseResult>, CatalogError> {
        let state = lock(&self.inner.state);
        let entries = state.entries()?;
        let mut result: HashMap<thrift::Repo, thrift::GetDatabaseResult> = HashMap::new();

        // mconcat [elsewhere, local', restoring]: build the weakest first.
        for (repo, meta) in &entries.restoring {
            result.insert(repo.clone(), restoring_db(repo, meta));
        }
        for (repo, entry) in &entries.live_here {
            let mut db_result = local_db(repo, entry, None);
            if let Some(exception) = entries.failed.get(repo) {
                // :598-602 — only the broken field is replaced.
                db_result.database.broken = Some(thrift::DatabaseBroken {
                    task: "open".to_string(),
                    // Source `Text.pack (show exception)` (:599): the Show
                    // image is Debug (RULEBOOK §2), which for DbError is
                    // byte-identical to its Display (exception.rs).
                    reason: format!("{:?}", exception),
                });
            }
            result.insert(repo.clone(), db_result);
        }
        for (repo, entry) in &entries.live_elsewhere {
            result.insert(
                repo.clone(),
                local_db(repo, entry, Some(thrift::DatabaseStatus::Available)),
            );
        }
        Ok(result)
    }

    /// `getLocalDatabase :: Catalog -> Repo -> STM (Maybe
    /// Thrift.GetDatabaseResult)` (:630-632). The source TODO "This isn't
    /// very efficient" (:631) is reproduced: it builds the whole map.
    pub fn get_local_database(
        &self,
        repo: &thrift::Repo,
    ) -> Result<Option<thrift::GetDatabaseResult>, CatalogError> {
        Ok(self.get_local_databases()?.get(repo).cloned())
    }
}

// ---------------------------------------------------------------------------
// list' (Catalog.hs:402-423)
// ---------------------------------------------------------------------------

/// `list' :: Monad m => (forall a. f a -> m a) -> [Locality] -> Filter ()
/// -> EntriesF (EntryF f) -> m [Item]` (:402-409). The `read` and `m`
/// parameters collapse because Entry's state is concrete; the body is the
/// source's: per locality, build (repo, (status, meta)) pairs, then one
/// `runFilter f . concat` over the locality-ordered list.
///
/// Public because the source exports `list'` "for testing" (:43).
///
/// Cross-group HashMap iteration order is NOT re-flagged here: row 49's
/// filter.rs already carries the single TODO for it (group_f, whose text
/// names Catalog.listMostRecent at Catalog.hs:427-430). Adding a second
/// marker here would double-count the same divergence.
pub fn list_prime(entries: &Entries, locs: &[Locality], f: Filter<()>) -> Vec<Item> {
    run_filter(f, collect_items(entries, locs))
}

/// The snapshot half of `list'`: per locality, build the `(status, meta)`
/// pairs and concatenate them in locality order. Split out so `list` can
/// run it under the outer guard (one atomic read) and apply the filter
/// afterwards, outside the lock. Callers must hold the outer lock when the
/// `Entries` came from a live catalog; `list_prime` passes a snapshot.
fn collect_items(entries: &Entries, locs: &[Locality]) -> Vec<Item> {
    let mut items = Vec::new();
    for loc in locs {
        let mut xs: Vec<(thrift::Repo, (ItemStatus, Meta))> = Vec::new();
        match loc {
            Locality::Local => {
                for (repo, entry) in &entries.live_here {
                    let (status, meta) = {
                        let entry_state = lock(&entry.state);
                        (entry_state.status, entry_state.meta.clone())
                    };
                    xs.push((repo.clone(), (status, meta)));
                }
            }
            Locality::Restoring => {
                for (repo, meta) in &entries.restoring {
                    xs.push((repo.clone(), (ItemStatus::ItemRestoring, meta.clone())));
                }
            }
            Locality::Cloud => {
                for (repo, entry) in &entries.live_elsewhere {
                    let (status, meta) = {
                        let entry_state = lock(&entry.state);
                        (entry_state.status, entry_state.meta.clone())
                    };
                    xs.push((repo.clone(), (status, meta)));
                }
            }
        }
        items.extend(xs.into_iter().map(|(repo, (status, meta))| Item {
            item_repo: repo,
            item_locality: *loc,
            item_meta: meta,
            item_status: status,
        }));
    }
    items
}

// ---------------------------------------------------------------------------
// getLocalDatabases' where-block helpers (Catalog.hs:606-628)
// ---------------------------------------------------------------------------

/// `local_db overrideStatus repo entry` (:606-618): read meta, expiring and
/// status under the entry lock; `fromMaybe (itemDatabaseStatus status)
/// overrideStatus`. The Meta is borrowed, so no clone is needed here.
fn local_db(
    repo: &thrift::Repo,
    entry: &Arc<Entry>,
    override_status: Option<thrift::DatabaseStatus>,
) -> thrift::GetDatabaseResult {
    let entry_state = lock(&entry.state);
    let status = override_status.unwrap_or_else(|| item_database_status(entry_state.status));
    thrift::GetDatabaseResult {
        database: meta_to_thrift_database(
            status,
            entry_state.expiring,
            repo.clone(),
            &entry_state.meta,
        ),
        auth_status: None,
        auth_message: None,
    }
}

/// `restoring_db repo meta` (:620-628): DatabaseStatus_Restoring, no
/// expiry.
fn restoring_db(repo: &thrift::Repo, meta: &Meta) -> thrift::GetDatabaseResult {
    thrift::GetDatabaseResult {
        database: meta_to_thrift_database(
            thrift::DatabaseStatus::Restoring,
            None,
            repo.clone(),
            meta,
        ),
        auth_status: None,
        auth_message: None,
    }
}

// ---------------------------------------------------------------------------
// Coverage / marker tally (closed set)
// ---------------------------------------------------------------------------
//
// Source symbols: all 31 export-list names (:15-47) have a Rust
// counterpart (EXPORT MAPPING above), and every private function
// (:166-307) plus the getLocalDatabases where-block helpers are imaged.
// Nothing is dropped except the test-only `ref`/`entry` type parameters
// (:93, :116) and the test instances, per RULE §4. `EntriesF`'s derived
// `Show` (:116) is NOT a production loss either: it requires `Show entry`,
// and `Entry = EntryF TVar` has no Show in the source (only the test-only
// `deriving instance Show (EntryF Identity)`, :108).
//
// ROWS 64/65'S WIRING CONTRACTS — CONSUMED: catalog/local.rs is the
// directory module for the Catalog/Local/ namespace and declares
// `pub mod files;` and `pub mod memory;`, resolving rows 64/65's
// catalog/local/{files,memory}.rs. It adds one `pub mod` line here and no
// marker of any class.
//
// Markers actually written:
//   * one performance deferral — the ownership-forced `Entries` snapshot
//     clone in `get_entries` (the public test accessor). `read_meta`'s Meta
//     clone and `list_prime`'s per-Item Meta clones are forced by their
//     owned return types, not deferrals.
//   * one port todo — modify_meta's callback cannot express the three
//     in-tree throwing callbacks (Finish.hs:44/:100, Create.hs:328).
// No source-defect reproductions (the dbFailed ordering is a fidelity note,
// not a source bug), no unsafe code (zero unsafe blocks).
//
// INVENTORY: the governing row is followed with ONE flagged divergence —
// INVENTORY-BUG-CANDIDATE row 21 for `Arc<Entry>` in place of the row's
// illustrative `Arc<Mutex<EntryState>>` (see the INVENTORY block). The
// marker is not part of the TODO/PERF/BUG/SAFETY tally.
//
// PORT STATUS: confidence=medium todos=1
