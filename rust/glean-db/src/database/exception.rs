// Ported from: glean/db/Glean/Database/Exception.hs (52 lines).
// Manifest unit: rust/glean-db/src/database/exception.rs
// (migration/manifest.tsv line 53, crate glean-db). RULE §4: one source
// file -> one target file; the manifest path is authoritative.
//
// INVENTORY (RULE §5) — the governing row is inventory.tsv:24 (identical to
// inventory.protocol.tsv:24), classification EXC_TAXONOMY, status confirmed.
//   $ sed -n '24p' migration/inventory.tsv
//   glean/db/Glean/Database/Exception.hs	DBError/SomeDBError/toDBError/fromDBError	open existential: DBError = DBError String | SomeDBError Exception e; DBAlreadyExists converts via fromDBError/cast	EXC_TAXONOMY	enum DbError { Msg(String), Other(Box<dyn Error+Send+Sync>) } + downcast helpers; DBAlreadyExists = TryFrom<&DbError>; all db throws funnel through DbError so the server/thrift mapping is single-point	Exception.hs:23-31 DBError GADT + Show; :33-37 dbException/dbError; :39-45 toDBError/fromDBError; :47-52 DBAlreadyExists	confirmed
//
// The five target_translation clauses, mapped one-for-one:
//   1. `enum DbError { Msg(String), Other(Box<dyn Error+Send+Sync>) }` ->
//      the `DbError` enum below. `Msg` is the `DBError :: String -> DBError`
//      constructor (:24); `Other` is the open `SomeDBError` leg (:25), the
//      `SomeException` image pinned by the sibling precedents in
//      ASSUMED-IMPORT.
//   2. `+ downcast helpers` -> the private `to_db_error` (image of the
//      private toDBError :39-40), `impl From<DBAlreadyExists> for DbError`
//      (the toException side, :52) and `impl TryFrom<&DbError> for
//      DBAlreadyExists` (the fromException/downcast side, :42-45/:51).
//   3. `DBAlreadyExists = TryFrom<&DbError>` -> exactly that impl, with
//      `type Error = ()` (FIDELITY NOTES).
//   4. `all db throws funnel through DbError` -> `db_exception` is the
//      intended constructor of the `Msg` leg (the enum variants stay public,
//      as the source exports `DBError(..)`), and `db_error` is the only
//      throwing helper; every later database unit calls `db_error(repo, msg)?`
//      rather than inventing an error type (SIBLING-UNIT CONTRACTS).
//   5. `so the server/thrift mapping is single-point` -> because all
//      database throws arrive as `DbError`, a future server/thrift boundary
//      matches this one enum instead of N per-unit error types. That mapping
//      itself is not part of this unit.
// The two exception-taxonomy SITES rows are also this file:
//   inventory.sites.tsv:610 -> toDBError (:39), exc-classify
//   inventory.sites.tsv:611 -> fromDBError (:42), exc-classify
// Both are private in the source and remain private here; they do not add
// an export and they do not require a separate inventory decision. No
// inventory divergence flag is needed: this port follows the row verbatim.
//
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, checked not assumed):
//   * Outbound edge, edges.tsv:287:
//       glean/db/Glean/Database/Exception.hs -> glean/db/Glean/Database/Repo.hs
//     That is manifest row 52, SHIPPED at 94787f9 (the current HEAD):
//     rust/glean-db/src/database/repo.rs:139 defines
//     `pub fn in_repo(repo: &thrift::Repo, s: &str) -> String`. This unit
//     calls it and does not re-implement the repo prefix.
//   * Inbound edges (11), by edges.tsv line, with the call sites verified by
//     `grep -n 'DBAlreadyExists\|dbError\|dbException\|DBError' glean --include=*.hs`:
//       :173  Catalog.hs              import :81;  dbError :372/:539/:551/:558;
//                                     throwM DBAlreadyExists :518
//       :178  Catalog/Local/Files.hs  import :29;  dbError :97
//       :241  Create.hs               import :43;  dbError :65/:66/:71/:76/:130
//       :255  Data.hs                 import :35;  dbError :73/:84/:101
//       :321  Open.hs                 import :58;  dbError :212/:320/:327/:565/:693;
//                                     the SomeException log at :757 (see FIDELITY)
//       :430  Storage/Memory.hs       import :26;  dbError :62/:67/:85/:134
//       :450  Types.hs                import :50;  dbError :316
//       :475  Write/Batch.hs          import :32;  dbError :92
//       :499  Write/Queue.hs          import :34;  dbError :96
//       :733  Backup/S3.hs            import :60;  dbError :263
//       :1779 gleancli/GleanCLI/Restore.hs
//                                     import :45;  catch \DBAlreadyExists :202
//     The import sites are the Haskell side of the contract; the Rust call
//     sites land as the corresponding rows are ported.
//
// ASSUMED-IMPORT (RULE §4 B4): the generated thrift crate and the
// `SomeException` image.
//   * The source's `import Glean.Types hiding (Exception)` (:20) supplies
//     `Repo`, which the generated Rust crate spells `glean_thrift::Repo`.
//     This file uses ONE spelling: `use glean_thrift as thrift;`, then
//     `thrift::Repo` everywhere (the row-51/row-52 convention).
//   * Pinned shape (glean/if/glean.thrift:106-109):
//         struct Repo { 1: string name; 2: string hash; }
//     Rust image (rust/glean-db/src/backend/types.rs:171-175):
//         #[derive(Clone, Debug, PartialEq, Eq, Hash)]
//         pub struct Repo { pub name: String, pub hash: String }
//     The Haskell type derives Eq, Show and Ord (recorded in row 52's
//     ASSUMED-IMPORT block; the generated Glean/Types.hs is build output,
//     not in this tree). This unit only passes `&thrift::Repo` through to
//     `in_repo`, so it imposes no additional derive demand.
//   * `SomeException` has no generated manifest unit. The pinned image in
//     this crate is `Box<dyn std::error::Error + Send + Sync>`, used by
//     `NotRetried` (rust/glean-db/src/backend/retry.rs:308-328: `#[derive(Debug)]`,
//     constructor-shaped `Display`, `impl std::error::Error`) and by
//     `RunQueryError::Other` (rust/glean-db/src/query/thrift.rs:418-422).
//     The same box is this unit's `Other` payload. `Box<dyn Error>`'s
//     `Display` forwards to the inner error's `Display`, which is the image
//     of the source's `show (SomeDBError e) = show e` (:29).
//   * glean-db's Cargo.toml lists glean-thrift only as an anticipated
//     Step-4 edge, so this path is not resolvable at the draft compile step;
//     that is the expected state for every generated-type consumer (RULE §4
//     B4: import from the generated crate, never fork a hand translation).
//
// SIBLING-UNIT CONTRACTS:
//   * Row 52 (SHIPPED at 94787f9) owns `in_repo` at
//     rust/glean-db/src/database/repo.rs:139. `db_exception` calls
//     `crate::database::repo::in_repo(repo, msg)`; no second owner and no
//     re-implementation of the `<subdir>: <msg>` prefix.
//   * DBAlreadyExists consumers: Catalog.hs:518 raises it via
//     `throwM DBAlreadyExists` inside `startRestoring`, and
//     gleancli/GleanCLI/Restore.hs:202 catches it with
//     `\DBAlreadyExists -> return ()`. The Rust image must therefore be
//     recoverable from the thrown `DbError`: `From<DBAlreadyExists> for
//     DbError` is the throw side, `TryFrom<&DbError> for DBAlreadyExists`
//     the catch side. A future `catch_unwind`-style boundary is not
//     involved; these are ordinary `Result` conversions.
//   * Single-funnel constraint: every later database unit's throw goes
//     through `db_error(repo, msg)` (or `db_exception` when it needs the
//     value), never a fresh per-unit error type. That is the inventory's
//     clause 4/5 and the reason this module is the only owner of the
//     database error taxonomy.
//   * StoreError's future Db arm: rust/glean-db/src/database/catalog/store.rs
//     :213-217 records the FORWARD CONTRACT (row 53/64): once this module
//     ships, the Files `get` arm (Files.hs:97) should extend `StoreError`
//     with a typed `Db(..)` variant (or `From<DbError>`) instead of erasing
//     the error into `StoreError::Other`. Row 64 (manifest line 64,
//     Catalog/Local/Files.hs) owns that change; this unit only supplies the
//     type it names.
//   * Wiring: this unit appends `pub mod exception;` to
//     rust/glean-db/src/database.rs after `pub mod repo;`, binding
//     `crate::database::exception` at the crate root. database.rs is
//     Rust-only scaffolding; no lib.rs change is needed (it already declares
//     `pub mod database;`).
//
// EXPORT MAPPING (4/4 source exports, :10-13):
//   :10  DBError(..)         -> `pub enum DbError { Msg, Other }`
//   :11  DBAlreadyExists(..) -> `pub struct DBAlreadyExists`
//   :12  dbException         -> `pub fn db_exception`
//   :13  dbError             -> `pub fn db_error`
// Trait impls (not source exports; the images of the source's instances):
//   * `impl fmt::Display for DbError` — custom Show, :27-29.
//   * `impl fmt::Debug for DbError` — delegates to Display (FIDELITY NOTES).
//   * `impl std::error::Error for DbError` — `instance Exception DBError`, :31.
//   * `impl fmt::Display for DBAlreadyExists` — derived Show, :47-48.
//   * `impl std::error::Error for DBAlreadyExists` — instance, :50-52.
//   * `impl From<DBAlreadyExists> for DbError` — toException, :52, via
//     toDBError, :39-40.
//   * `impl TryFrom<&DbError> for DBAlreadyExists` — fromException, :51,
//     via fromDBError, :42-45.
// Private helpers (not exports):
//   * `fn to_db_error` — the private toDBError (:39-40).
//   * fromDBError (:42-45) has no standalone function image: its body is
//     the `try_from` implementation, because it is only reachable through
//     the DBAlreadyExists Exception instance.
//
// FIDELITY NOTES:
//   * Existential -> enum. The source GADT is closed in practice (two
//     constructors, :23-25): `DBError String` becomes `Msg(String)`;
//     `SomeDBError (Exception e) => e` becomes the open
//     `Other(Box<dyn Error + Send + Sync>)` leg. RULE §2's GADT row says
//     `enum` where the variant set is closed; the open leg is carried by the
//     box, not by a trait object for the whole type.
//   * Custom Show -> Display + Debug. The source does NOT derive Show for
//     DBError: `show (DBError s) = s` and `show (SomeDBError e) = show e`
//     (:27-29). Display reproduces exactly that text (the bare message for
//     `Msg`, the inner error's Display for `Other`). Debug delegates to
//     Display, because a structural derive would print `Msg("...")` /
//     `Other(...)` and diverge from every log line that shows the exception
//     — notably Open.hs:757,
//     `logError $ inRepo dbRepo "couldn't open: " ++ show (exc :: SomeException)`,
//     where a thrown DBError is rendered by this Show instance. Debug is a
//     required supertrait of std::error::Error; delegating keeps both
//     renderings byte-identical.
//   * Exception/throwM -> Error + Result. `instance Exception DBError` (:31)
//     becomes `impl std::error::Error for DbError {}`; `MonadThrow m => Repo
//     -> String -> m a` (:36-37) becomes
//     `db_error<T>(...) -> Result<T, DbError>`. Callers use it as
//     `db_error(repo, msg)?`, which is the Rust image of `throwM` in an
//     `IO`/`STM` action. There is no `panic!` path: the source throws a
//     typed exception, not an `error` call.
//   * Typeable/cast -> downcast_ref. `fromDBError` (:42-45) unwraps
//     `SomeDBError e` and `cast`s to the requested type; the Rust image is
//     `downcast_ref::<DBAlreadyExists>()` on the boxed `Other` payload.
//     `TryFrom` is the inventory-pinned shape; `type Error = ()` is the
//     minimal image of `Maybe b`, which carries no failure payload.
//   * DBAlreadyExists is a fieldless marker. `#[derive(Debug)]` reproduces
//     the derived Show text exactly (`DBAlreadyExists`). `Clone + Copy` are
//     mechanical Rust requirements of the pinned
//     `.downcast_ref::<DBAlreadyExists>().copied()` expression and are
//     faithful to a nullary Haskell constructor, which is a shared value;
//     neither is a source export or observable behavior.
//   * No source chain. Neither type carries a cause; `std::error::Error`
//     keeps the default `source() -> None` for both. DBAlreadyExists DOES
//     override toException/fromException (:50-52), but those are the
//     From/TryFrom conversions above, not a cause chain.
//   * Repo is passed by `&thrift::Repo` (immutable sharing, the row-52
//     convention); the message is `&str` rather than an owned String because
//     `in_repo` returns a new String and no call site needs to retain the
//     input.
//   * No Send/Sync supertraits are added to DbError itself; the `Other`
//     variant's box already carries `Send + Sync`, so the enum is
//     automatically Send+Sync without widening the source's type.

use glean_thrift as thrift;

use crate::database::repo::in_repo;

/// Source: DBError (:23-25), the `Msg`/open-existential pair.
///
/// `Msg` is `DBError :: String -> DBError`; `Other` is
/// `SomeDBError :: Exception e => e -> DBError`, carrying the
/// `SomeException` image.
pub enum DbError {
    Msg(String),
    Other(Box<dyn std::error::Error + Send + Sync>),
}

/// Source: custom Show (:27-29) — the bare message for `DBError`, the inner
/// error's text for `SomeDBError`.
impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DbError::Msg(s) => f.write_str(s),
            DbError::Other(e) => std::fmt::Display::fmt(e, f),
        }
    }
}

/// Source: custom Show (:27-29), also used by every `show (exc ::
/// SomeException)` log site (e.g. Open.hs:757). Delegating to Display keeps
/// the log text identical to the Haskell output; a structural derive would
/// add the Rust constructor wrapper.
impl std::fmt::Debug for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

/// Source: `instance Exception DBError` (:31). The source has no cause
/// chain, so the default `source()` is correct.
impl std::error::Error for DbError {}

/// Source: dbException (:33-34) — `DBError . inRepo repo`.
pub fn db_exception(repo: &thrift::Repo, msg: &str) -> DbError {
    DbError::Msg(in_repo(repo, msg))
}

/// Source: dbError (:36-37) — `throwM $ dbException repo msg`.
///
/// The image of `MonadThrow m => Repo -> String -> m a`; callers write
/// `db_error(repo, msg)?`. This is the single funnel the inventory mandates:
/// all database throws become `DbError`.
pub fn db_error<T>(repo: &thrift::Repo, msg: &str) -> Result<T, DbError> {
    Err(db_exception(repo, msg))
}

/// Source: DBAlreadyExists (:47-48), a nullary type with derived Show.
///
/// `Debug` reproduces the derived Show text. `Clone + Copy` support the
/// pinned `copied()` downcast below (see FIDELITY NOTES).
#[derive(Clone, Copy, Debug)]
pub struct DBAlreadyExists;

/// Source: derived Show for DBAlreadyExists (:47-48).
impl std::fmt::Display for DBAlreadyExists {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DBAlreadyExists")
    }
}

/// Source: `instance Exception DBAlreadyExists` (:50-52).
impl std::error::Error for DBAlreadyExists {}

/// Source: toDBError (:39-40) — `toException . SomeDBError`. Private in the
/// source, private here.
fn to_db_error<E: std::error::Error + Send + Sync + 'static>(e: E) -> DbError {
    DbError::Other(Box::new(e))
}

/// Source: the toException side of DBAlreadyExists's Exception instance
/// (:52), via toDBError (:39-40).
impl From<DBAlreadyExists> for DbError {
    fn from(e: DBAlreadyExists) -> Self {
        to_db_error(e)
    }
}

/// Source: fromDBError (:42-45) and the fromException override (:51).
///
/// `fromException` returns `Maybe b`; `type Error = ()` is its minimal
/// image because the failure carries no payload. `Msg` is not a
/// `SomeDBError` wrapper, so it cannot downcast and returns `Err(())`.
impl TryFrom<&DbError> for DBAlreadyExists {
    type Error = ();

    fn try_from(e: &DbError) -> Result<Self, Self::Error> {
        match e {
            DbError::Other(e) => e.downcast_ref::<DBAlreadyExists>().copied().ok_or(()),
            DbError::Msg(_) => Err(()),
        }
    }
}

//
// PORT STATUS: confidence=high todos=0
