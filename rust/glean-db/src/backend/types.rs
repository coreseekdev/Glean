// Ported from: glean/backend-api/Glean/Backend/Types.hs (482 lines)
// Manifest unit: rust/glean-db/src/backend/types.rs (migration/manifest.tsv
// line 41, crate glean-db). RULE §4: one source file -> one target file.
//
// This is the FIRST glean-db unit (90 manifest rows); it owns the crate
// scaffolding (rust/glean-db/Cargo.toml, src/lib.rs, src/backend.rs) per
// the glean-util/glean-typed first-unit precedent (rust/glean-util/src/
// util/some.rs, commit 4bdf95a; rust/glean-typed/src/lib.rs, commit
// 82b4437).
//
// The source module is the Backend API surface: the `Backend` class
// (28 abstract methods + 1 default, :86-185), the `Some Backend`
// adapter instance (:192-224), ten free functions built on the class
// (:229-338), the `Query` wrapper (:346-357), the type-indexed Haxl
// request types `GleanGet`/`GleanQuery` with their Eq/Hashable/DataSource
// instances and per-datasource state (:362-465), the `AppendList`/
// `QueryResult` batching-result surface (:420-442), and the shard
// hashing `dbShard`/`dbShardWord` (:470-482).
//
// Export-surface mapping (Haskell -> Rust; source export list :11-44):
//
//   Backend(..)             -> `pub trait Backend` (all 29 methods;
//                              Rust trait methods are public with the
//                              trait, so `(..)` is the whole trait)
//   StackedDbOpts(..)       -> `pub enum StackedDbOpts` (2 variants)
//   LogDerivationResult     -> `pub type LogDerivationResult`
//   Query(..)               -> `pub struct Query<A>` + pub field
//                              `query_spec` + `pub fn new` (carries the
//                              constructor's `Type a =>` bound; see the
//                              type docs) + `pub fn into_spec` (extraction
//                              helper — the image of the exported record
//                              selector, needed because the private
//                              marker blocks cross-module functional
//                              update; probe evidence in
//                              /tmp/opencode/db41/probe_a/)
//   SchemaPredicates        -> `pub use glean_typed::typed::SchemaPredicates`
//                              (already shipped; NAMED re-export, no glob)
//   loadPredicates          -> `load_predicates`
//   loadPredicatesForSchema -> `load_predicates_for_schema`
//   databases               -> `databases`
//   localDatabases          -> `local_databases`
//   create                  -> `create`
//   finish                  -> `finish`
//   fillDatabase            -> `fill_database`
//   finalize                -> `finalize`
//   completePredicates      -> `complete_predicates`
//   untilDone               -> `until_done`
//   GleanGet(..)            -> `pub enum GleanGet<P>` (Get/GetKey)
//   GleanQuery(..)          -> `pub enum GleanQuery<Q>` (QueryReq)
//   GleanFetcher            -> `pub type GleanFetcher`
//   GleanQueryer            -> `pub type GleanQueryer`
//   Haxl.State(..)          -> `GleanGetState`/`GleanQueryState` (the two
//                              data instances this module defines) + the
//                              engine's state-key surface (manifest row
//                              146; see the Haxl seam block)
//   QueryResult(..)         -> `pub trait QueryResult<Q>`
//   AppendList(..)          -> `pub struct AppendList<Q>`
//   fromAppendList          -> `from_append_list`
//   dbShard                 -> `db_shard`
//   dbShardWord             -> `db_shard_word`
//
// Rust-only additions, each justified at its site and nothing else is
// exported:
//
//   * `SomeBackend<R>` — the Rust image of `instance Backend (Some
//     Backend)` (:192-224). Haskell's `Some c` is the existential
//     wrapper; RULE §2's GADT/existential row: "`enum` where the variant
//     set is closed" — the set IS closed (five inhabitants, decision (B)
//     below), so the open-existential representation of
//     `Glean.Util.Some` (row 38, util/some.rs) is NOT reused. `Some
//     Backend` itself is the type this instance is FOR, so the type
//     must exist for the instance to have an image.
//   * `BackendError` — the class methods' exception channel; RULE §2
//     dynamic-exceptions row: "typed error enum per crate + a
//     catch-taxonomy table". Open question flagged at its definition.
//   * `try_bracket` — stand-in for `Util.Control.Exception.tryBracket`
//     (hsthrift), first consumer where (`fill_database`, :276). The
//     hsthrift module has no manifest unit; one owner prevents forks
//     (further consumers: Catalog.hs:348/:528, Write.hs:113,
//     SendQueue.hs:224/:267 — later glean-db rows import THIS one).
//   * `Sum<T>` — Rust image of `Data.Monoid.Sum`, the aggregate of the
//     second `QueryResult` instance (:441-442). A base-library type with
//     no Rust base; the first consumer owns it (decision (D)); the
//     Logging.hs:272 `getSum` consumer (row 148) imports this one.
//
// ---------------------------------------------------------------------------
// ASSUMED-IMPORT: the `glean_thrift` crate (generated)
// ---------------------------------------------------------------------------
//
// RULE §1 thrift row + §4 B4: every `thrift::*` name below is generated
// by build.rs from glean/if/glean.thrift into the pinned crate
// `glean_thrift` — generated code is never a manifest unit and never
// hand-translated. The source imports it TWICE — `qualified Glean.Types
// as Thrift` (:71) and unqualified `Glean.Types` (:74) — and both
// collapse to this one alias (decision (I); RULE §4 B4 import duality).
// Unqualified source uses (`Repo`, `Dependencies`, `SchemaId`, `Retry`,
// :265-267, :294-303, :336) render as `thrift::…` here.
//
// Pinned IDL (glean/if/glean.thrift), the members this file uses:
//   :31   typedef i64 Id
//   :76   struct Empty {}
//   :91   typedef string SchemaId (hs.newtype)
//   :106  struct Repo { 1: string name; 2: string hash }
//   :116  typedef map<string, string> (hs.type = "HashMap") DatabaseProperties
//   :134  union Dependencies          (payload opaque here)
//   :239  exception Retry { 1: double seconds }
//   :303  struct Database             (opaque)
//   :344  struct PredicateStats       (opaque)
//   :351  typedef string Handle
//   :354/:369/:374/:383/:388 ComputedBatch / SendResponse / FinishResponse /
//         FinalizeResponse / FinishDatabaseResponse      (opaque)
//   :539  struct DerivePredicateQuery (opaque)
//   :635  union DerivationStatus      (opaque)
//   :640  struct UserQuery            (opaque)
//   :670  struct UserQueryBatch       (opaque)
//   :705  struct UserQueryStats       (opaque)
//   :768  struct UserQueryResults     (opaque)
//   :816  union UserQueryResultsOrException (opaque)
//   :829  struct UserQueryFacts       (opaque)
//   :854  struct ListDatabases { 1: bool includeBackups = false; 2: optional
//         ClientInfo client_info }
//   :862  struct ListDatabasesResult { 1: list<Database> databases; 2-3
//         optional auth fields }
//   :869  struct GetDatabaseResult    (opaque)
//   :876  struct DeleteDatabaseResult (opaque)
//   :890  struct SendJsonBatch        (opaque)
//   :902  struct SendJsonBatchResponse (opaque)
//   :920  union EnqueueBatch          (opaque)
//   :924  enum EnqueueBatchWaitPolicy (opaque)
//   :933  struct EnqueueBatchResponse (opaque)
//   :940  struct KickOff { 1: Repo repo; 3: DatabaseProperties properties;
//         4: optional Dependencies dependencies; 5: optional PosixEpochTime
//         repo_hash_time; 6: bool update_schema_for_stacked = false;
//         7: optional map<string, list<string>> (hs.type = "HashMap")
//         acl_config }
//   :974  struct KickOffResponse { 1: bool alreadyExists; 2-3 optional auth }
//   :981  struct UpdatePropertiesResponse (opaque)
//   :987  struct SchemaInfo { 1: bytestring schema; 2: map<Id, PredicateRef>
//         predicateIds; 3-6 other maps; 7-8 optional auth }
//   :1017 union SelectSchema { 1: Empty stored; 2: Empty current;
//         3: SchemaId schema_id }
//   :1028 struct GetSchemaInfo { 1: SelectSchema select; 2: bool omit_source }
//   :1037 struct FactIdRange { 1: Id start; 2: Id finish }
//   :1042 struct ValidateSchema { 1: bytestring schema }
//   :1058 union CompletePredicates   (opaque)
//   :1063 struct CompletePredicatesResponse (opaque)
//   :1069 struct WaitForWritesResponse (opaque)
//
// The assumed Rust image is the in-tree fbthrift-style codegen image
// (flat at the crate root; IDL field names in snake_case — pinned by
// glean-util/src/util/service.rs:56-72 "Field names follow fbthrift's
// Rust codegen"; thrift `map` -> `BTreeMap` is pinned by
// glean-typed/src/typed/predicate.rs:482-486 "Data.Map in Haskell,
// BTreeMap in the generated crate" for SchemaInfo.predicateIds,
// glean.thrift:995; `optional` -> `Option`; `list` -> `Vec`;
// `bytestring` -> `Vec<u8>`; struct Default = hsthrift `def`). Shapes
// this file's code depends on:
//
//     pub type Id = i64;                    // :31, no hs.newtype —
//                                           // typed/predicate.rs:497-500
//                                           // wraps i64 directly
//     pub type SchemaId = String;           // :91, gen-hs2 :599 is a
//                                           // newtype over Text; the
//                                           // Rust typedef image is an
//                                           // alias (service.rs
//                                           // "typedef i32 Version ->
//                                           // pub type Version = i32")
//     pub type Handle = String;             // :351
//     pub type DatabaseProperties =
//         std::collections::BTreeMap<String, String>;   // :116
//     #[derive(Clone, Debug, PartialEq, Eq, Hash)]
//     pub struct Repo { pub name: String, pub hash: String } // :106;
//         // gen-hs2 :629 data Repo{repo_name, repo_hash}
//         // deriving (Eq, Show, Ord); Hash is the hsthrift
//         // Hashable image (GleanGet's Hashable at :388-392 needs it)
//     #[derive(Clone, Debug, PartialEq, PartialOrd)]
//     pub struct Retry { pub seconds: f64 }  // :239; gen-hs2 :1857
//         // newtype Retry{retry_seconds :: Double}
//     #[derive(Clone, Debug, PartialEq, Eq)]
//     pub enum SelectSchema { Empty, Stored, Current, SchemaId(SchemaId) }
//         // :1017; `Empty` is the union's unset value == hsthrift
//         // `SelectSchema_EMPTY` (gen-hs2 :10288 def = SelectSchema_EMPTY)
//         // — fbthrift renders a union as a closed enum with an Empty
//         // arm; `stored`/`current` carry the empty `Empty` struct
//         // (:76) and are rendered unit-shaped (never constructed
//         // here; only `SchemaId` is)
//     #[derive(Clone, Debug, Default, PartialEq, Eq)]
//     pub struct GetSchemaInfo { pub select: SelectSchema,
//                                pub omit_source: bool } // :1028,
//         // Default = gen-hs2 :10370 (select = SelectSchema_EMPTY)
//     #[derive(Clone, Debug, Default, PartialEq, Eq)]
//     pub struct KickOff { pub repo: Repo,
//                          pub properties: DatabaseProperties,
//                          pub dependencies: Option<Dependencies>,
//                          /* repo_hash_time, update_schema_for_stacked,
//                             acl_config */ }  // :940, Default = gen-hs2
//         // :9584; the three named fields are the only ones `create`
//         // sets (:295-302)
//     #[derive(Clone, Debug, PartialEq, Eq)]
//     pub struct KickOffResponse { pub already_exists: bool, /* auth */ }
//         // :974; gen-hs2 :9657 alreadyExists
//     #[derive(Clone, Debug, Default, PartialEq, Eq)]
//     pub struct ListDatabases { pub include_backups: bool,
//                                /* client_info */ } // :854, Default =
//         // gen-hs2 :8310; `databases`/`local_databases` flip only
//         // include_backups (:251/:256)
//     #[derive(Clone, Debug, Default, PartialEq, Eq)]
//     pub struct ListDatabasesResult { pub databases: Vec<Database>,
//                                      /* auth fields */ } // :862,
//         // gen-hs2 :8231-8234
//     #[derive(Clone, Debug, Default, PartialEq, Eq)]
//     pub struct SchemaInfo { /* schema, */ pub predicate_ids:
//         std::collections::BTreeMap<Id, PredicateRef>, /* ... */ }
//         // :983; `load_predicates_for_schema` reads predicate_ids
//         // (:246 `Map.elems`)
//     #[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
//     pub struct UserQuery { /* opaque */ }  // :640; Hash needed by
//         // Query's Hashable image (:356-357)
//
// The remaining members (`Dependencies`, `Fact`, `PredicateStats`,
// `UserQueryFacts`, `UserQueryResults`, the batch/response types, …)
// are opaque payloads here: the aliases above fix only what signatures
// mention; their derive lists follow gen-hs2 deriving + value semantics
// plus hsthrift's Hashable where a source instance needs it. If the
// generated crate ships different names, only the use sites in this
// file change.
//
// ---------------------------------------------------------------------------
// ASSUMED-IMPORT: glean_typed / glean_util / glean_angle (shipped)
// ---------------------------------------------------------------------------
//
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, module-level): the
// rows naming this file are 18/19/20 (typed/util edges, cited at the
// Cargo.toml dep comments); generated modules carry no depmap rows.
// Decision (I) import map — all four shipped surfaces by exact path
// (each pin is verified against the shipped file, not memory):
//   * `glean_typed::typed::SchemaPredicates` (typed/predicate.rs:420,
//     a NAMED re-export in this module's export block below);
//   * `glean_typed::typed::{make_predicates, IdOf, Predicates, Type}`
//     (predicate.rs:482 `make_predicates(&[SchemaPredicates],
//     &SchemaInfo) -> Predicates`; id.rs:197 `IdOf<P>` — the
//     phantom-tagged `Fid` newtype with an unbounded `id_of()` accessor
//     and an unbounded manual Debug (id.rs:211-218); binary.rs:235
//     `Type<A: 'static>`);
//   * `glean_util::util::thrift_service::DbShard` (thrift_service.rs:433;
//     a `String` alias — `db_shard` returns it);
//   * `glean_angle::angle::hash::{fingerprint_data, Fingerprint}`
//     (hash.rs:230 `fingerprint_data(&[u8]) -> Fingerprint`;
//     hash.rs:74-76 pub fields `fingerprint1`/`fingerprint2`) — this
//     import discharges the SIBLING-UNIT CONTRACT pinned at hash.rs:21-30
//     ("The glean-db Backend/Types translation … must import … and must
//     NOT reimplement MD5").
//
// ---------------------------------------------------------------------------
// SIBLING-UNIT CONTRACTS (same crate, later manifest rows)
// ---------------------------------------------------------------------------
//
// These paths do not exist in the tree yet; they are forward same-crate
// contracts, written directly and verified against a stub harness (the
// hash.rs:59 `crate::angle::types::Binary` precedent):
//   * `crate::database::types::Env` (row 104, db/Glean/Database/Types.hs)
//     — the `SomeBackend::Local` payload; `Env: Backend` lands with
//     row 147 (Local.hs:73 `instance Backend Database.Env`);
//   * `crate::backend::retry::{RetryWritesBackend, RetryReadsBackend}`
//     (row 43, Retry.hs:20/:76) — `impl<R: Backend> Backend` for each
//     (Retry.hs:27/:83), each holding `SomeBackend<R>` by value;
//   * `crate::backend::logging::LoggingBackend` (row 148, Logging.hs:45
//     `newtype LoggingBackend = LoggingBackend Database.Env`, instance
//     :47);
//   * `crate::haxl::engine::{GleanGetFetcher, GleanQueryFetcher}` (row
//     146, haxl/Haxl/DataSource/Glean/Common.hs) — see the Haxl seam
//     block below.
// The `SomeBackend` <-> `Retry*Backend` recursion is broken by the
// `Box` in the enum's wrapper variants (the wrapper structs hold the
// enum by value; the enum boxes the wrappers) — scratch-verified,
// including the mutual trait-impl resolution (no overflow; probe
// /tmp/opencode/db41/probe_c/).
//
// ---------------------------------------------------------------------------
// RECORD DIVERGENCE: Glean.Util.Some decision record
// ---------------------------------------------------------------------------
//
// rust/glean-util/src/util/some.rs:57-66 lists the `Some Backend`
// concrete set as "Database.Env @ Local.hs:73, ThriftBackend @
// Remote.hs:184, RetryWrites/RetryReads @ Retry.hs:27,83, LoggingBackend
// @ backend-local/Logging.hs:47, S3Backend @ S3.hs:101, MockBackend @
// Backup/Mock.hs:44 — closed". The first five cites are CORRECT; the
// last two are NOT inhabitants of this class: S3Backend and MockBackend
// implement `Glean.Database.Backup.Backend` (Backup/Backend.hs:39), a
// DIFFERENT class — S3.hs:58 / Mock.hs:27 import that module, and
// neither file imports Glean.Backend.Types at all. Per the record-file
// rule (dispatch records are not edited by later units), the divergence
// is recorded here instead: the closed set Decision (B) encodes is
// exactly {Database.Env @ Local.hs:73, ThriftBackend @ Remote.hs:184,
// RetryWritesBackend @ Retry.hs:27, RetryReadsBackend @ Retry.hs:83,
// LoggingBackend @ Logging.hs:47} — plus the repo's sixth instance,
// `Backend (Some LocalOrRemote)` (LocalOrRemote.hs:211, production-used
// at Indexer.hs:37): its `Some` forwarding grounds every value in
// {Env, ThriftBackend, LoggingBackend}, all already in this set, so the
// closed variant set stays complete without a new variant. Value
// coverage is what matters; Decision (B)'s five-name list is corrected
// by this note, not re-edited. inventory.protocol.tsv:23 (:223
// pass-through) and inventory.tsv row 136 do NOT name any backend, so
// there is no governing-row divergence, only the record's.
//
// ---------------------------------------------------------------------------
// Port decisions (A)-(I), with cites
// ---------------------------------------------------------------------------
//
// (A) `Backend` -> a Rust trait, receivers `&self`, IO -> `Result<_,
//     BackendError>` (RULE §2 dynamic-exceptions row). The
//     `backendWithClientId` default (:184-185) is a by-value method
//     `fn backend_with_client_id(self, client_id: &str) -> Self { self }`
//     with `where Self: Sized` — REQUIRED: a by-value `self` return has
//     no image for unsized Self (scratch rustc 1.98.1: without the
//     bound the trait itself fails to compile, E0277 at the default
//     body, /tmp/opencode/db41/probe_b/no_sized.out; with it the trait
//     compiles and `dyn Backend` remains WRITABLE but is never used —
//     the backend set is closed, see (B); probe_b/with_sized.out). 29
//     methods total (28 abstract + the default).
// (B) `instance Backend (Some Backend)` (:192-224) -> the closed
//     `SomeBackend<R>` enum + a full 29-method forwarding impl. The
//     closed set and its cites are the RECORD DIVERGENCE block above.
//     `R` is the remote leg: glean-db cannot name `ThriftBackend` (it
//     lives in glean-client/Remote.hs:184 — the inverse of this
//     crate's dependency), so the enum is generic and glean-client
//     instantiates `SomeBackend<ThriftBackend>`.
// (C) The Haxl seam (:362-465) -> request types + state structs HERE,
//     engine owned by manifest row 146 (`crate::haxl::engine`); see the
//     Haxl seam block below.
// (D) `Query`/request images: `Query<A>` private-marker struct (:346-357);
//     `GleanGet<P>`/`GleanQuery<Q>` with hand-written Eq/Hash/Debug —
//     see their definitions for the type-indexation and Hash contrasts.
// (E) `dbShardWord` (:473-482) -> `db_shard_word` over the SHARED
//     glean-angle fingerprint module; INVENTORY-BUG-CANDIDATE row 10 at
//     the definition (hsthrift's MD5 is never reimplemented; golden
//     probe /tmp/opencode/db41/probe_d/).
// (F) `Util.Control.Exception.tryBracket` -> the module-owned
//     `try_bracket` stand-in; pinned semantics + one flagged divergence
//     at its definition (probe /tmp/opencode/db41/probe_c/).
// (G) Thrift struct construction: `def { field = … }` -> a struct
//     literal with `..Default::default()` (hsthrift `def` == Rust
//     `Default`; gen-hs2 default-instance cites in the assumed-import
//     block above). Field names are the IDL snake_case names.
// (H) `LogDerivationResult` (:188-190) -> `Box<dyn Fn(&Result<…>) +
//     Send + Sync>`: reference argument (Logging.hs:341 calls `log res`
//     and re-uses `res` at :347/:350), `Send` (the value moves into the
//     `spawn_ envWarden` action, Derive.hs:136/:139), `Fn` (the log is
//     called on the one result value); probe_c leg C5.
// (I) Import duality -> one `glean_thrift` image (see above).

use std::any::TypeId;
use std::collections::BTreeMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::thread;
use std::time::Duration;

use glean_angle::angle::hash::{fingerprint_data, Fingerprint};
use glean_thrift as thrift;
use glean_typed::typed::{make_predicates, IdOf, Predicates, Type};
use glean_util::util::thrift_service::DbShard;

// `SchemaPredicates` (:19) is exported as itself; `Predicates` is
// imported above (with `make_predicates`/`IdOf`/`Type`) for the free
// functions (:229-246); `Predicate` itself is not needed here.
pub use glean_typed::typed::SchemaPredicates;

// ---------------------------------------------------------------------------
// StackedDbOpts (:77-80)
// ---------------------------------------------------------------------------

/// `data StackedDbOpts = IncludeBase | ExcludeBase deriving (Eq, Show)`
/// — selector for `Backend::predicate_stats` (:92).
//
// Derive note: `Eq`/`Show` -> `PartialEq`/`Eq`/`Debug`; `Clone`/`Copy`
// added per the fieldless-sum convention (angle/types.rs:396 precedent)
/// — the value is passed by value through `predicate_stats`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StackedDbOpts {
    IncludeBase,
    ExcludeBase,
}

// ---------------------------------------------------------------------------
// BackendError (RUST-ONLY; RULE §2 dynamic-exceptions row)
// ---------------------------------------------------------------------------

/// The error channel of every `Backend` method. The source has no such
/// type: the class methods run in `IO` and throw dynamically
/// (`throwIO`), `untilDone` catches `Retry` (:333-337), and
/// `fillDatabase` branches on success/exception (:281-284). RULE §2
/// dynamic-exceptions row: "typed error enum per crate + a catch-taxonomy
/// table". This is glean-db's enum; the per-crate taxonomy table lands
/// with the concrete backend impls.
//
/// Variants:
///   * `Thrift` — anything the thrift runtime raises (`glean_thrift::
///     ThriftError`, the shipped union sketch: channel/protocol/
///     application/service; rust/glean-util/src/util/thrift_service.rs:
///     264-292);
///   * `Retry` — the IDL-declared `exception Retry` (glean.thrift:239;
///     the source's `Left (Retry n)` arm, :336). `until_done` matches
///     exactly this variant.
///   * `Other` — the open leg, mirroring `SomeException` (:190).
// TODO(port): the taxonomy is provisional — which upstream exceptions
// (and which `ThriftError` variants, incl. declared-exception boxes
// under `ThriftError::Service`) map to which `BackendError` variant is
// a catch-taxonomy decision that the concrete backend impls (rows 147/
// 167) and the exception-taxonomy inventory rows (inventory.tsv:13
// EXC_TAXONOMY, :54 glass classify) must pin at Step 4; `Display`/
// `std::error::Error` impls land with that table (RULE §2 deriving-Show
// row). Until then the three-variant shape is the minimal faithful one.
#[derive(Debug)]
pub enum BackendError {
    Thrift(thrift::ThriftError),
    Retry(thrift::Retry),
    Other(Box<dyn std::error::Error + Send + Sync>),
}

// ---------------------------------------------------------------------------
// class Backend (:86-185)
// ---------------------------------------------------------------------------

/// An abstraction over Glean's Thrift API, allowing client code to work
/// with either a local or remote backend, chosen at runtime (:82-85).
//
/// Method-for-method image of the class; 28 abstract methods plus the
/// `backend_with_client_id` default (29 total). Signature conventions
/// (decision (A) + RULE §1):
///   * receiver `&self` (Haskell's `a ->` first argument), except
///     `backend_with_client_id`, which consumes (its Haskell shape is
///     `Text -> a -> a` with a value-returning default);
///   * `Thrift.Repo` -> `&thrift::Repo`; `Maybe Thrift.Repo` ->
///     `Option<&thrift::Repo>` (:89, :156);
///   * other thrift arguments by value (the generated-call convention);
///   * `IO` returns -> `Result<_, BackendError>` (RULE §2).
//
/// Batch/Haxl seams: `init_global_state` is the row-9 contract — it
/// returns exactly the two per-datasource states (:179-180); the
/// explicit request-batching layer lives behind the fetchers stored in
/// those states (see the Haxl seam block).
pub trait Backend {
    /// `queryFact :: a -> Thrift.Repo -> Thrift.Id -> IO (Maybe
    /// Thrift.Fact)` (:87).
    fn query_fact(&self, repo: &thrift::Repo, id: thrift::Id)
        -> Result<Option<thrift::Fact>, BackendError>;

    /// `factIdRange :: a -> Thrift.Repo -> IO Thrift.FactIdRange` (:88).
    fn fact_id_range(&self, repo: &thrift::Repo)
        -> Result<thrift::FactIdRange, BackendError>;

    /// `getSchemaInfo :: a -> Maybe Thrift.Repo -> Thrift.GetSchemaInfo
    /// -> IO Thrift.SchemaInfo` (:89-90).
    fn get_schema_info(
        &self,
        repo: Option<&thrift::Repo>,
        request: thrift::GetSchemaInfo,
    ) -> Result<thrift::SchemaInfo, BackendError>;

    /// `validateSchema :: a -> Thrift.ValidateSchema -> IO ()` (:91).
    fn validate_schema(&self, request: thrift::ValidateSchema)
        -> Result<(), BackendError>;

    /// `predicateStats :: a -> Thrift.Repo -> StackedDbOpts -> IO (Map
    /// Thrift.Id Thrift.PredicateStats)` (:92-93). The `Data.Map` (an
    /// ordered map in the source, :54-55) renders as `BTreeMap`.
    fn predicate_stats(
        &self,
        repo: &thrift::Repo,
        opts: StackedDbOpts,
    ) -> Result<BTreeMap<thrift::Id, thrift::PredicateStats>, BackendError>;

    /// `listDatabases :: a -> Thrift.ListDatabases -> IO
    /// Thrift.ListDatabasesResult` (:94).
    fn list_databases(&self, request: thrift::ListDatabases)
        -> Result<thrift::ListDatabasesResult, BackendError>;

    /// `getDatabase :: a -> Thrift.Repo -> IO
    /// Thrift.GetDatabaseResult` (:95).
    fn get_database(&self, repo: &thrift::Repo)
        -> Result<thrift::GetDatabaseResult, BackendError>;

    /// `userQueryFacts :: a -> Thrift.Repo -> Thrift.UserQueryFacts ->
    /// IO Thrift.UserQueryResults` (:97-98).
    fn user_query_facts(
        &self,
        repo: &thrift::Repo,
        request: thrift::UserQueryFacts,
    ) -> Result<thrift::UserQueryResults, BackendError>;

    /// `userQuery :: a -> Thrift.Repo -> Thrift.UserQuery -> IO
    /// Thrift.UserQueryResults` (:99-100).
    fn user_query(
        &self,
        repo: &thrift::Repo,
        request: thrift::UserQuery,
    ) -> Result<thrift::UserQueryResults, BackendError>;

    /// `userQueryBatch :: a -> Thrift.Repo -> Thrift.UserQueryBatch ->
    /// IO [Thrift.UserQueryResultsOrException]` (:101-102).
    fn user_query_batch(
        &self,
        repo: &thrift::Repo,
        request: thrift::UserQueryBatch,
    ) -> Result<Vec<thrift::UserQueryResultsOrException>, BackendError>;

    /// `deriveStored :: a -> LogDerivationResult -> Thrift.Repo ->
    /// Thrift.DerivePredicateQuery -> IO Thrift.DerivationStatus`
    /// (:104-105). The log callback is consumed by value (`Box<dyn Fn>`
    /// is move-only where Haskell function values are shareable): a
    /// caller that keeps its callback wraps it in a cheaply-cloneable
    /// handle (e.g. `Arc`) — see the type's docs for the callback
    /// contract.
    fn derive_stored(
        &self,
        log: LogDerivationResult,
        repo: &thrift::Repo,
        query: thrift::DerivePredicateQuery,
    ) -> Result<thrift::DerivationStatus, BackendError>;

    /// `kickOffDatabase :: a -> Thrift.KickOff -> IO
    /// Thrift.KickOffResponse` (:107).
    fn kick_off_database(&self, request: thrift::KickOff)
        -> Result<thrift::KickOffResponse, BackendError>;

    /// `finishDatabase :: a -> Thrift.Repo -> IO
    /// Thrift.FinishDatabaseResponse` (:108).
    fn finish_database(&self, repo: &thrift::Repo)
        -> Result<thrift::FinishDatabaseResponse, BackendError>;

    /// `finalizeDatabase :: a -> Thrift.Repo -> IO
    /// Thrift.FinalizeResponse` (:109).
    fn finalize_database(&self, repo: &thrift::Repo)
        -> Result<thrift::FinalizeResponse, BackendError>;

    /// `updateProperties :: a -> Thrift.Repo ->
    /// Thrift.DatabaseProperties -> [Text] -> IO
    /// Thrift.UpdatePropertiesResponse` (:111-116).
    fn update_properties(
        &self,
        repo: &thrift::Repo,
        properties: thrift::DatabaseProperties,
        names: Vec<String>,
    ) -> Result<thrift::UpdatePropertiesResponse, BackendError>;

    /// `completePredicates_ :: a -> Thrift.Repo ->
    /// Thrift.CompletePredicates -> IO
    /// Thrift.CompletePredicatesResponse` (:118-122). Underscored: the
    /// public `complete_predicates` wrapper adds `until_done` (:325-327).
    fn complete_predicates_(
        &self,
        repo: &thrift::Repo,
        predicates: thrift::CompletePredicates,
    ) -> Result<thrift::CompletePredicatesResponse, BackendError>;

    /// `restoreDatabase :: a -> Text -> IO ()` (:130): issue the restore
    /// request for the backup locator; does not wait for completion.
    fn restore_database(&self, backup: &str) -> Result<(), BackendError>;

    /// `deleteDatabase :: a -> Thrift.Repo -> IO
    /// Thrift.DeleteDatabaseResult` (:133): deletes the specified repo
    /// for a local database.
    fn delete_database(&self, repo: &thrift::Repo)
        -> Result<thrift::DeleteDatabaseResult, BackendError>;

    /// `enqueueBatch :: a -> Thrift.ComputedBatch -> IO
    /// Thrift.SendResponse` (:136).
    fn enqueue_batch(&self, batch: thrift::ComputedBatch)
        -> Result<thrift::SendResponse, BackendError>;

    /// `enqueueJsonBatch :: a -> Thrift.Repo -> Thrift.SendJsonBatch ->
    /// IO Thrift.SendJsonBatchResponse` (:139-143).
    fn enqueue_json_batch(
        &self,
        repo: &thrift::Repo,
        batch: thrift::SendJsonBatch,
    ) -> Result<thrift::SendJsonBatchResponse, BackendError>;

    /// `enqueueBatchDescriptor :: a -> Thrift.Repo -> Thrift.EnqueueBatch
    /// -> Thrift.EnqueueBatchWaitPolicy -> IO
    /// Thrift.EnqueueBatchResponse` (:146-151).
    fn enqueue_batch_descriptor(
        &self,
        repo: &thrift::Repo,
        batch: thrift::EnqueueBatch,
        wait_policy: thrift::EnqueueBatchWaitPolicy,
    ) -> Result<thrift::EnqueueBatchResponse, BackendError>;

    /// `pollBatch :: a -> Maybe Thrift.Repo -> Thrift.Handle -> IO
    /// Thrift.FinishResponse` (:155-156): the repo is used for shard
    /// routing — pass `None` when no repo is available.
    fn poll_batch(
        &self,
        repo: Option<&thrift::Repo>,
        handle: thrift::Handle,
    ) -> Result<thrift::FinishResponse, BackendError>;

    /// `waitForWrites :: a -> Thrift.Repo -> IO
    /// Thrift.WaitForWritesResponse` (:159).
    fn wait_for_writes(&self, repo: &thrift::Repo)
        -> Result<thrift::WaitForWritesResponse, BackendError>;

    /// `displayBackend :: a -> String` (:162): render for debugging.
    fn display_backend(&self) -> String;

    /// `hasDatabase :: a -> Thrift.Repo -> IO Bool` (:167): for remote
    /// sharded backends, whether any server advertises the shard.
    fn has_database(&self, repo: &thrift::Repo) -> Result<bool, BackendError>;

    /// `schemaId :: a -> Maybe Thrift.SchemaId` (:171): the schema
    /// version the client wants; sent along with queries.
    fn schema_id(&self) -> Option<thrift::SchemaId>;

    /// `usingShards :: a -> Bool` (:176): True for a distributed backend
    /// where servers may have different DBs (then `has_database` is the
    /// availability check).
    fn using_shards(&self) -> bool;

    /// `initGlobalState :: a -> IO (Haxl.State GleanGet, Haxl.State
    /// GleanQuery)` (:179): initialise the Haxl state for this backend.
    /// Returns EXACTLY the two per-datasource states (inventory.tsv row
    /// 8: "initGlobalState returns exactly the pair"); wrappers pass
    /// through unchanged (row 7), they only decorate the batch executor.
    fn init_global_state(&self)
        -> Result<(GleanGetState, GleanQueryState), BackendError>;

    /// `backendWithClientId :: Text -> a -> a` (:184-185): tag outbound
    /// RPCs with a ServiceRouter client id; default is a no-op.
    /// `where Self: Sized` is forced by the by-value default body
    /// (decision (A)).
    fn backend_with_client_id(self, client_id: &str) -> Self
    where
        Self: Sized,
    {
        let _ = client_id;
        self
    }
}

// ---------------------------------------------------------------------------
// LogDerivationResult (:188-190)
// ---------------------------------------------------------------------------

/// `type LogDerivationResult = Either (DiffTimePoints, SomeException)
/// Thrift.UserQueryStats -> IO ()` (:188-190): the derive-status logger.
//
/// Image (decision (H)): `Box<dyn Fn(&Result<UserQueryStats, (Duration,
/// Box<dyn Error + Send + Sync>)>) + Send + Sync>`.
///   * `DiffTimePoints` -> `std::time::Duration` (RULE §1 time row);
///   * `SomeException` -> `Box<dyn Error + Send + Sync>` (the same open
///     leg as `BackendError::Other`);
///   * `Either e a` -> `Result<a, e>` (error-side in the `Err` arm — the
///     source's `Either` carries the error first, `Result` second;
///     consumers match `Ok`/`Err` in the source's `Right`/`Left`
///     positions);
///   * the argument is a REFERENCE: Logging.hs:341 calls `log res`, then
///     re-uses `res` at :347/:350 (so `Fn(&Result<…>)`, not
///     `Fn(Result<…>)`);
///   * `Send + Sync`: the callback is moved into the `spawn_ envWarden`
///     action (Derive.hs:136/:139);
///   * `Fn` (not `FnOnce`): stored and called through a shared handle;
///     the derivations log exactly one result, so call sites invoke it
///     once. `Box` because the callback is an open trait object (the
///     source type is a bare function).
pub type LogDerivationResult = Box<
    dyn Fn(
            &Result<
                thrift::UserQueryStats,
                (Duration, Box<dyn std::error::Error + Send + Sync>),
            >,
        ) + Send
        + Sync,
>;

// ---------------------------------------------------------------------------
// try_bracket — stand-in for Util.Control.Exception.tryBracket
// ---------------------------------------------------------------------------

/// `tryBracket :: IO a -> (a -> Either SomeException b -> IO ()) -> (a
/// -> IO b) -> IO b` (hsthrift common/util/Util/Control/Exception.hs:
/// 87-94), first consumer `fill_database` (:276).
//
/// Pinned semantics of the source body:
///     tryBracket before after inner =
///       mask $ \restore -> do
///         a <- before
///         r <- restore (inner a) `catch` \ex -> do
///           after a (Left ex)
///           throwIO ex
///         _ <- after a (Right r)
///         return r
///   * `before` runs masked; `inner` runs under `restore` (the mask is
///     only for the bracket bookkeeping);
///   * on inner error: `after a (Left ex)` runs, then the exception is
///     RE-THROWN;
///   * on success: `after a (Right r)` runs, then `r` is returned;
///   * `after` does NOT run if `before` throws (it is outside the catch);
///   * an error from `after` itself propagates (it is not caught).
/// The Rust body below reproduces this order exactly (probe
/// /tmp/opencode/db41/probe_c/, leg C3, traced).
//
/// Consumer shapes this signature is built for (verified against the
/// consumer sources): Catalog.hs:348/:528 branch on `Right _`/`Left _`;
/// glean/db/Glean/Database/Writes.hs:113 `done (…) result = … isLeft
/// result`; SendQueue.hs:224/:267 `\r ex -> case (ex, r) of (Left exc,
/// Just wait) -> …`; the generic-E signature likewise accommodates the
/// remaining tryBracket consumers GleanCLI/Index.hs:51 and
/// GleanCLI/Write.hs:148. Hence
/// `After: FnOnce(&A, &Result<B, E>) -> Result<(), E>` — the inner
/// result is passed BY REFERENCE so the `after` action can branch
/// without consuming it, and `after` returns `Result<(), E>` (its own
/// error wins).
// TODO(port): the panic-unwind leg is a real divergence. Haskell's
// `restore (inner a) \`catch\` \ex -> …` catches EVERY synchronous
// exception including pure `error` calls (`SomeException`), and the
// masked/restore + async-exception distinction (isSyncException,
// Exception.hs:75-93) has no `std::panic` image without a catch_unwind
// boundary in EVERY action — deliberately not paid by default. If a
// consumer's action can panic, `after` will NOT run under unwinding
// (the equivalent of `Left ex` delivered to `after` is lost). Revisit if
// a panic-visible `after` is ever needed; scratch evidence:
// /tmp/opencode/db41/probe_c/ (leg C3 proves only the Result leg).
pub fn try_bracket<A, B, E, Before, After, Action>(
    before: Before,
    after: After,
    action: Action,
) -> Result<B, E>
where
    Before: FnOnce() -> Result<A, E>,
    After: FnOnce(&A, &Result<B, E>) -> Result<(), E>,
    Action: FnOnce(&A) -> Result<B, E>,
{
    let a = before()?;
    let r = action(&a);
    after(&a, &r)?;
    r
}

// ---------------------------------------------------------------------------
// instance Backend (Some Backend) (:192-224) -> SomeBackend<R>
// ---------------------------------------------------------------------------

/// The closed-set image of Haskell's open existential `Some Backend`.
/// RULE §2's GADT/existential row: "`enum` where the variant set is
/// closed; `Box<dyn Trait>` only where genuinely open" — the variant set
/// IS closed (the RECORD DIVERGENCE block at the top records the
/// corrected list, with cites):
//
///   * `Local(crate::database::types::Env)` — Local.hs:73;
///   * `Remote(R)` — `R` is the remote backend: glean-db cannot name
///     `ThriftBackend` (glean-client/Remote.hs:184 — glean-client
///     depends on glean-db, not the other way); glean-client
///     instantiates `SomeBackend<ThriftBackend>`;
///   * `RetryWrites(Box<RetryWritesBackend<R>>)` — Retry.hs:27;
///   * `RetryReads(Box<RetryReadsBackend<R>>)` — Retry.hs:83;
///   * `Logging(Box<LoggingBackend>)` — Logging.hs:47.
//
/// The `Box` on the wrapper variants is load-bearing: `Retry*Backend<R>`
/// holds `SomeBackend<R>` BY VALUE (Retry.hs:20/:76 field `Some
/// Backend`), so one of the two sides must be indirect. Boxes here keep
/// the wrapper structs (row 43) unsized-field-free and let their
/// `impl<R: Backend> Backend` bounds stay plain `R: Backend`;
/// scratch-verified including the mutual impl resolution (no overflow,
/// probe_c leg C1).
//
/// Forward contract: the impl compiles once `Env: Backend` exists (row
/// 147), the two wrapper impls exist (row 43) and `LoggingBackend:
/// Backend` exists (row 148) — all same-crate forward references
/// recorded in the SIBLING-UNIT CONTRACTS block at the top.
pub enum SomeBackend<R> {
    Local(crate::database::types::Env),
    Remote(R),
    RetryWrites(Box<crate::backend::retry::RetryWritesBackend<R>>),
    RetryReads(Box<crate::backend::retry::RetryReadsBackend<R>>),
    Logging(Box<crate::backend::logging::LoggingBackend>),
}

/// Method-for-method forwarding (:193-224): every class method delegates
/// to the wrapped backend. `backend_with_client_id` (:224) maps over the
/// variant — `Some (backendWithClientId cid b)` becomes
/// `SomeBackend::X(x.backend_with_client_id(client_id))`.
impl<R: Backend> Backend for SomeBackend<R> {
    fn query_fact(
        &self,
        repo: &thrift::Repo,
        id: thrift::Id,
    ) -> Result<Option<thrift::Fact>, BackendError> {
        match self {
            SomeBackend::Local(x) => x.query_fact(repo, id),
            SomeBackend::Remote(x) => x.query_fact(repo, id),
            SomeBackend::RetryWrites(x) => x.query_fact(repo, id),
            SomeBackend::RetryReads(x) => x.query_fact(repo, id),
            SomeBackend::Logging(x) => x.query_fact(repo, id),
        }
    }

    fn fact_id_range(
        &self,
        repo: &thrift::Repo,
    ) -> Result<thrift::FactIdRange, BackendError> {
        match self {
            SomeBackend::Local(x) => x.fact_id_range(repo),
            SomeBackend::Remote(x) => x.fact_id_range(repo),
            SomeBackend::RetryWrites(x) => x.fact_id_range(repo),
            SomeBackend::RetryReads(x) => x.fact_id_range(repo),
            SomeBackend::Logging(x) => x.fact_id_range(repo),
        }
    }

    fn get_schema_info(
        &self,
        repo: Option<&thrift::Repo>,
        request: thrift::GetSchemaInfo,
    ) -> Result<thrift::SchemaInfo, BackendError> {
        match self {
            SomeBackend::Local(x) => x.get_schema_info(repo, request),
            SomeBackend::Remote(x) => x.get_schema_info(repo, request),
            SomeBackend::RetryWrites(x) => x.get_schema_info(repo, request),
            SomeBackend::RetryReads(x) => x.get_schema_info(repo, request),
            SomeBackend::Logging(x) => x.get_schema_info(repo, request),
        }
    }

    fn validate_schema(
        &self,
        request: thrift::ValidateSchema,
    ) -> Result<(), BackendError> {
        match self {
            SomeBackend::Local(x) => x.validate_schema(request),
            SomeBackend::Remote(x) => x.validate_schema(request),
            SomeBackend::RetryWrites(x) => x.validate_schema(request),
            SomeBackend::RetryReads(x) => x.validate_schema(request),
            SomeBackend::Logging(x) => x.validate_schema(request),
        }
    }

    fn predicate_stats(
        &self,
        repo: &thrift::Repo,
        opts: StackedDbOpts,
    ) -> Result<BTreeMap<thrift::Id, thrift::PredicateStats>, BackendError> {
        match self {
            SomeBackend::Local(x) => x.predicate_stats(repo, opts),
            SomeBackend::Remote(x) => x.predicate_stats(repo, opts),
            SomeBackend::RetryWrites(x) => x.predicate_stats(repo, opts),
            SomeBackend::RetryReads(x) => x.predicate_stats(repo, opts),
            SomeBackend::Logging(x) => x.predicate_stats(repo, opts),
        }
    }

    fn list_databases(
        &self,
        request: thrift::ListDatabases,
    ) -> Result<thrift::ListDatabasesResult, BackendError> {
        match self {
            SomeBackend::Local(x) => x.list_databases(request),
            SomeBackend::Remote(x) => x.list_databases(request),
            SomeBackend::RetryWrites(x) => x.list_databases(request),
            SomeBackend::RetryReads(x) => x.list_databases(request),
            SomeBackend::Logging(x) => x.list_databases(request),
        }
    }

    fn get_database(
        &self,
        repo: &thrift::Repo,
    ) -> Result<thrift::GetDatabaseResult, BackendError> {
        match self {
            SomeBackend::Local(x) => x.get_database(repo),
            SomeBackend::Remote(x) => x.get_database(repo),
            SomeBackend::RetryWrites(x) => x.get_database(repo),
            SomeBackend::RetryReads(x) => x.get_database(repo),
            SomeBackend::Logging(x) => x.get_database(repo),
        }
    }

    fn user_query_facts(
        &self,
        repo: &thrift::Repo,
        request: thrift::UserQueryFacts,
    ) -> Result<thrift::UserQueryResults, BackendError> {
        match self {
            SomeBackend::Local(x) => x.user_query_facts(repo, request),
            SomeBackend::Remote(x) => x.user_query_facts(repo, request),
            SomeBackend::RetryWrites(x) => x.user_query_facts(repo, request),
            SomeBackend::RetryReads(x) => x.user_query_facts(repo, request),
            SomeBackend::Logging(x) => x.user_query_facts(repo, request),
        }
    }

    fn user_query(
        &self,
        repo: &thrift::Repo,
        request: thrift::UserQuery,
    ) -> Result<thrift::UserQueryResults, BackendError> {
        match self {
            SomeBackend::Local(x) => x.user_query(repo, request),
            SomeBackend::Remote(x) => x.user_query(repo, request),
            SomeBackend::RetryWrites(x) => x.user_query(repo, request),
            SomeBackend::RetryReads(x) => x.user_query(repo, request),
            SomeBackend::Logging(x) => x.user_query(repo, request),
        }
    }

    fn user_query_batch(
        &self,
        repo: &thrift::Repo,
        request: thrift::UserQueryBatch,
    ) -> Result<Vec<thrift::UserQueryResultsOrException>, BackendError> {
        match self {
            SomeBackend::Local(x) => x.user_query_batch(repo, request),
            SomeBackend::Remote(x) => x.user_query_batch(repo, request),
            SomeBackend::RetryWrites(x) => x.user_query_batch(repo, request),
            SomeBackend::RetryReads(x) => x.user_query_batch(repo, request),
            SomeBackend::Logging(x) => x.user_query_batch(repo, request),
        }
    }

    fn derive_stored(
        &self,
        log: LogDerivationResult,
        repo: &thrift::Repo,
        query: thrift::DerivePredicateQuery,
    ) -> Result<thrift::DerivationStatus, BackendError> {
        match self {
            SomeBackend::Local(x) => x.derive_stored(log, repo, query),
            SomeBackend::Remote(x) => x.derive_stored(log, repo, query),
            SomeBackend::RetryWrites(x) => x.derive_stored(log, repo, query),
            SomeBackend::RetryReads(x) => x.derive_stored(log, repo, query),
            SomeBackend::Logging(x) => x.derive_stored(log, repo, query),
        }
    }

    fn kick_off_database(
        &self,
        request: thrift::KickOff,
    ) -> Result<thrift::KickOffResponse, BackendError> {
        match self {
            SomeBackend::Local(x) => x.kick_off_database(request),
            SomeBackend::Remote(x) => x.kick_off_database(request),
            SomeBackend::RetryWrites(x) => x.kick_off_database(request),
            SomeBackend::RetryReads(x) => x.kick_off_database(request),
            SomeBackend::Logging(x) => x.kick_off_database(request),
        }
    }

    fn finish_database(
        &self,
        repo: &thrift::Repo,
    ) -> Result<thrift::FinishDatabaseResponse, BackendError> {
        match self {
            SomeBackend::Local(x) => x.finish_database(repo),
            SomeBackend::Remote(x) => x.finish_database(repo),
            SomeBackend::RetryWrites(x) => x.finish_database(repo),
            SomeBackend::RetryReads(x) => x.finish_database(repo),
            SomeBackend::Logging(x) => x.finish_database(repo),
        }
    }

    fn finalize_database(
        &self,
        repo: &thrift::Repo,
    ) -> Result<thrift::FinalizeResponse, BackendError> {
        match self {
            SomeBackend::Local(x) => x.finalize_database(repo),
            SomeBackend::Remote(x) => x.finalize_database(repo),
            SomeBackend::RetryWrites(x) => x.finalize_database(repo),
            SomeBackend::RetryReads(x) => x.finalize_database(repo),
            SomeBackend::Logging(x) => x.finalize_database(repo),
        }
    }

    fn update_properties(
        &self,
        repo: &thrift::Repo,
        properties: thrift::DatabaseProperties,
        names: Vec<String>,
    ) -> Result<thrift::UpdatePropertiesResponse, BackendError> {
        match self {
            SomeBackend::Local(x) => x.update_properties(repo, properties, names),
            SomeBackend::Remote(x) => x.update_properties(repo, properties, names),
            SomeBackend::RetryWrites(x) => x.update_properties(repo, properties, names),
            SomeBackend::RetryReads(x) => x.update_properties(repo, properties, names),
            SomeBackend::Logging(x) => x.update_properties(repo, properties, names),
        }
    }

    fn complete_predicates_(
        &self,
        repo: &thrift::Repo,
        predicates: thrift::CompletePredicates,
    ) -> Result<thrift::CompletePredicatesResponse, BackendError> {
        match self {
            SomeBackend::Local(x) => x.complete_predicates_(repo, predicates),
            SomeBackend::Remote(x) => x.complete_predicates_(repo, predicates),
            SomeBackend::RetryWrites(x) => x.complete_predicates_(repo, predicates),
            SomeBackend::RetryReads(x) => x.complete_predicates_(repo, predicates),
            SomeBackend::Logging(x) => x.complete_predicates_(repo, predicates),
        }
    }

    fn restore_database(&self, backup: &str) -> Result<(), BackendError> {
        match self {
            SomeBackend::Local(x) => x.restore_database(backup),
            SomeBackend::Remote(x) => x.restore_database(backup),
            SomeBackend::RetryWrites(x) => x.restore_database(backup),
            SomeBackend::RetryReads(x) => x.restore_database(backup),
            SomeBackend::Logging(x) => x.restore_database(backup),
        }
    }

    fn delete_database(
        &self,
        repo: &thrift::Repo,
    ) -> Result<thrift::DeleteDatabaseResult, BackendError> {
        match self {
            SomeBackend::Local(x) => x.delete_database(repo),
            SomeBackend::Remote(x) => x.delete_database(repo),
            SomeBackend::RetryWrites(x) => x.delete_database(repo),
            SomeBackend::RetryReads(x) => x.delete_database(repo),
            SomeBackend::Logging(x) => x.delete_database(repo),
        }
    }

    fn enqueue_batch(
        &self,
        batch: thrift::ComputedBatch,
    ) -> Result<thrift::SendResponse, BackendError> {
        match self {
            SomeBackend::Local(x) => x.enqueue_batch(batch),
            SomeBackend::Remote(x) => x.enqueue_batch(batch),
            SomeBackend::RetryWrites(x) => x.enqueue_batch(batch),
            SomeBackend::RetryReads(x) => x.enqueue_batch(batch),
            SomeBackend::Logging(x) => x.enqueue_batch(batch),
        }
    }

    fn enqueue_json_batch(
        &self,
        repo: &thrift::Repo,
        batch: thrift::SendJsonBatch,
    ) -> Result<thrift::SendJsonBatchResponse, BackendError> {
        match self {
            SomeBackend::Local(x) => x.enqueue_json_batch(repo, batch),
            SomeBackend::Remote(x) => x.enqueue_json_batch(repo, batch),
            SomeBackend::RetryWrites(x) => x.enqueue_json_batch(repo, batch),
            SomeBackend::RetryReads(x) => x.enqueue_json_batch(repo, batch),
            SomeBackend::Logging(x) => x.enqueue_json_batch(repo, batch),
        }
    }

    fn enqueue_batch_descriptor(
        &self,
        repo: &thrift::Repo,
        batch: thrift::EnqueueBatch,
        wait_policy: thrift::EnqueueBatchWaitPolicy,
    ) -> Result<thrift::EnqueueBatchResponse, BackendError> {
        match self {
            SomeBackend::Local(x) => x.enqueue_batch_descriptor(repo, batch, wait_policy),
            SomeBackend::Remote(x) => x.enqueue_batch_descriptor(repo, batch, wait_policy),
            SomeBackend::RetryWrites(x) => x.enqueue_batch_descriptor(repo, batch, wait_policy),
            SomeBackend::RetryReads(x) => x.enqueue_batch_descriptor(repo, batch, wait_policy),
            SomeBackend::Logging(x) => x.enqueue_batch_descriptor(repo, batch, wait_policy),
        }
    }

    fn poll_batch(
        &self,
        repo: Option<&thrift::Repo>,
        handle: thrift::Handle,
    ) -> Result<thrift::FinishResponse, BackendError> {
        match self {
            SomeBackend::Local(x) => x.poll_batch(repo, handle),
            SomeBackend::Remote(x) => x.poll_batch(repo, handle),
            SomeBackend::RetryWrites(x) => x.poll_batch(repo, handle),
            SomeBackend::RetryReads(x) => x.poll_batch(repo, handle),
            SomeBackend::Logging(x) => x.poll_batch(repo, handle),
        }
    }

    fn wait_for_writes(
        &self,
        repo: &thrift::Repo,
    ) -> Result<thrift::WaitForWritesResponse, BackendError> {
        match self {
            SomeBackend::Local(x) => x.wait_for_writes(repo),
            SomeBackend::Remote(x) => x.wait_for_writes(repo),
            SomeBackend::RetryWrites(x) => x.wait_for_writes(repo),
            SomeBackend::RetryReads(x) => x.wait_for_writes(repo),
            SomeBackend::Logging(x) => x.wait_for_writes(repo),
        }
    }

    fn display_backend(&self) -> String {
        match self {
            SomeBackend::Local(x) => x.display_backend(),
            SomeBackend::Remote(x) => x.display_backend(),
            SomeBackend::RetryWrites(x) => x.display_backend(),
            SomeBackend::RetryReads(x) => x.display_backend(),
            SomeBackend::Logging(x) => x.display_backend(),
        }
    }

    fn has_database(&self, repo: &thrift::Repo) -> Result<bool, BackendError> {
        match self {
            SomeBackend::Local(x) => x.has_database(repo),
            SomeBackend::Remote(x) => x.has_database(repo),
            SomeBackend::RetryWrites(x) => x.has_database(repo),
            SomeBackend::RetryReads(x) => x.has_database(repo),
            SomeBackend::Logging(x) => x.has_database(repo),
        }
    }

    fn schema_id(&self) -> Option<thrift::SchemaId> {
        match self {
            SomeBackend::Local(x) => x.schema_id(),
            SomeBackend::Remote(x) => x.schema_id(),
            SomeBackend::RetryWrites(x) => x.schema_id(),
            SomeBackend::RetryReads(x) => x.schema_id(),
            SomeBackend::Logging(x) => x.schema_id(),
        }
    }

    fn using_shards(&self) -> bool {
        match self {
            SomeBackend::Local(x) => x.using_shards(),
            SomeBackend::Remote(x) => x.using_shards(),
            SomeBackend::RetryWrites(x) => x.using_shards(),
            SomeBackend::RetryReads(x) => x.using_shards(),
            SomeBackend::Logging(x) => x.using_shards(),
        }
    }

    fn init_global_state(
        &self,
    ) -> Result<(GleanGetState, GleanQueryState), BackendError> {
        match self {
            SomeBackend::Local(x) => x.init_global_state(),
            SomeBackend::Remote(x) => x.init_global_state(),
            SomeBackend::RetryWrites(x) => x.init_global_state(),
            SomeBackend::RetryReads(x) => x.init_global_state(),
            SomeBackend::Logging(x) => x.init_global_state(),
        }
    }

    /// (`backendWithClientId cid (Some b) = Some (backendWithClientId
    /// cid b)`, :224.)
    fn backend_with_client_id(self, client_id: &str) -> Self {
        match self {
            SomeBackend::Local(x) => {
                SomeBackend::Local(x.backend_with_client_id(client_id))
            }
            SomeBackend::Remote(x) => {
                SomeBackend::Remote(x.backend_with_client_id(client_id))
            }
            SomeBackend::RetryWrites(x) => SomeBackend::RetryWrites(Box::new(
                (*x).backend_with_client_id(client_id),
            )),
            SomeBackend::RetryReads(x) => SomeBackend::RetryReads(Box::new(
                (*x).backend_with_client_id(client_id),
            )),
            SomeBackend::Logging(x) => SomeBackend::Logging(Box::new(
                (*x).backend_with_client_id(client_id),
            )),
        }
    }
}

// ---------------------------------------------------------------------------
// Functionality built on Backend (:226-338)
// ---------------------------------------------------------------------------

/// `loadPredicates :: Backend a => a -> Thrift.Repo -> [SchemaPredicates]
/// -> IO Predicates` (:229-237): build the per-predicate Pid vector for
/// a repo's schema, with the schema source omitted.
pub fn load_predicates<B: Backend>(
    backend: &B,
    repo: &thrift::Repo,
    refs: &[SchemaPredicates],
) -> Result<Predicates, BackendError> {
    let info = backend.get_schema_info(
        Some(repo),
        thrift::GetSchemaInfo {
            omit_source: true,
            ..Default::default()
        },
    )?;
    Ok(make_predicates(refs, &info))
}

/// `loadPredicatesForSchema :: Backend a => a -> SchemaId -> IO
/// Predicates` (:239-246): select the named schema (no repo — the
/// schema is global), omit its source, and use the stored schema's
/// predicate map.
pub fn load_predicates_for_schema<B: Backend>(
    backend: &B,
    schema_id: thrift::SchemaId,
) -> Result<Predicates, BackendError> {
    let info = backend.get_schema_info(
        None,
        thrift::GetSchemaInfo {
            select: thrift::SelectSchema::SchemaId(schema_id),
            omit_source: true,
        },
    )?;
    // `makePredicates [Map.elems (Thrift.schemaInfo_predicateIds info)]
    // info` (:246): Map.elems is ascending key order; the assumed image
    // is a BTreeMap, so `.values()` is the same order (the ORDER feeds
    // the Pid vector — keep the ordered map).
    Ok(make_predicates(
        &[info.predicate_ids.values().cloned().collect::<Vec<_>>()],
        &info,
    ))
}

/// `databases :: Backend a => a -> IO [Thrift.Database]` (:248-251):
/// all databases including backups.
pub fn databases<B: Backend>(
    be: &B,
) -> Result<Vec<thrift::Database>, BackendError> {
    Ok(be
        .list_databases(thrift::ListDatabases {
            include_backups: true,
            ..Default::default()
        })?
        .databases)
}

/// `localDatabases :: Backend a => a -> IO [Thrift.Database]` (:253-256):
/// local databases only (no backup-server query).
pub fn local_databases<B: Backend>(
    be: &B,
) -> Result<Vec<thrift::Database>, BackendError> {
    Ok(be
        .list_databases(thrift::ListDatabases {
            include_backups: false,
            ..Default::default()
        })?
        .databases)
}

/// `fillDatabase :: Backend a => a -> Repo -> Maybe Dependencies -> IO ()
/// -> IO b -> IO b` (:261-285): create a database and run the supplied
/// action to write into it; when the action returns, the DB is marked
/// complete and cannot be modified further.
//
/// The bracket framing is the source's: `before` creates (running
/// `if_exists` when the DB already existed — it may `Err` to forbid),
/// `after` finishes when the inner action succeeded, and the action runs
/// between them (with `try_bracket`'s pinned semantics — see above).
pub fn fill_database<B, R, IfExists, Action>(
    env: &B,
    repo: &thrift::Repo,
    maybe_deps: Option<thrift::Dependencies>,
    if_exists: IfExists,
    action: Action,
) -> Result<R, BackendError>
where
    B: Backend,
    IfExists: FnOnce() -> Result<(), BackendError>,
    Action: FnOnce() -> Result<R, BackendError>,
{
    try_bracket(
        || {
            let exists = create(env, repo, maybe_deps)?;
            if exists {
                if_exists()?;
            }
            Ok(())
        },
        |_unit, result| {
            // `when (isRight ex) $ do finish env repo` (:281-284).
            if result.is_ok() {
                finish(env, repo)?;
            }
            Ok(())
        },
        |_unit| action(),
    )
}

/// `create :: Backend a => a -> Repo -> Maybe Dependencies -> IO Bool`
/// (:288-303): returns `True` if the DB already existed. The properties
/// map carries the singleton `("glean.schema_id", id)` only when the
/// backend has a schema id (:297-300).
pub fn create<B: Backend>(
    backend: &B,
    repo: &thrift::Repo,
    maybe_deps: Option<thrift::Dependencies>,
) -> Result<bool, BackendError> {
    let properties: thrift::DatabaseProperties = match backend.schema_id() {
        Some(id) => BTreeMap::from([("glean.schema_id".to_string(), id)]),
        None => BTreeMap::new(),
    };
    let r = backend.kick_off_database(thrift::KickOff {
        repo: repo.clone(),
        properties,
        dependencies: maybe_deps,
        ..Default::default()
    })?;
    Ok(r.already_exists)
}

/// `finish :: Backend a => a -> Repo -> IO ()` (:306-313): finish the
/// write side of a DB created with `create`, then finalize it.
pub fn finish<B: Backend>(
    backend: &B,
    repo: &thrift::Repo,
) -> Result<(), BackendError> {
    backend.finish_database(repo)?;
    finalize(backend, repo)
}

/// `finalize :: Backend a => a -> Repo -> IO ()` (:319-321): wait for a
/// database to finish finalizing (retrying `Retry` while polling).
pub fn finalize<B: Backend>(
    env: &B,
    repo: &thrift::Repo,
) -> Result<(), BackendError> {
    until_done(|| env.finalize_database(repo))?;
    Ok(())
}

/// `completePredicates :: Backend a => a -> Repo -> CompletePredicates ->
/// IO ()` (:325-327): notify the server when non-derived predicates are
/// complete (must be called before derivedStored).
pub fn complete_predicates<B: Backend>(
    env: &B,
    repo: &thrift::Repo,
    preds: thrift::CompletePredicates,
) -> Result<(), BackendError> {
    // The retried action re-passes the same pure value, exactly like the
    // source (the generated-call convention takes thrift arguments by
    // value, so each retry clones); `predicates` is moved in once.
    let predicates = preds;
    until_done(move || env.complete_predicates_(repo, predicates.clone()))?;
    Ok(())
}

/// `untilDone :: IO a -> IO a` (:329-338): retry the action while it
/// reports `Retry`, sleeping the reported number of seconds;
/// `threadDelay (truncate (n * 1000000))` -> `Duration::from_micros`.
/// Every other error propagates unchanged (the source's `try` rethrows
/// non-`Retry` exceptions).
//
/// Note: `retry.seconds` is server-supplied and non-negative by
/// protocol; the `as u64` cast matches `truncate` for that domain.
pub fn until_done<A, F>(io: F) -> Result<A, BackendError>
where
    F: FnMut() -> Result<A, BackendError>,
{
    let mut io = io;
    loop {
        match io() {
            Ok(a) => return Ok(a),
            Err(BackendError::Retry(retry)) => {
                let micros = (retry.seconds * 1_000_000.0) as u64;
                thread::sleep(Duration::from_micros(micros));
            }
            Err(e) => return Err(e),
        }
    }
}

// ---------------------------------------------------------------------------
// Query (:346-357)
// ---------------------------------------------------------------------------

/// `data Query a = Type a => Query { querySpec :: UserQuery }` (:346-348):
/// a query that can be performed by `runQuery`; built with `query`
/// (the combinator at query/thrift/Internal.hs:81, row 45 — re-exported
/// through Glean.Query.Thrift/Glean.hs, rows 47/153).
///
/// The `Type a =>` constraint is on the Haskell CONSTRUCTOR, so the Rust
/// image carries it on `Query::<A>::new` and not on the type: `Query<A>`
/// is nameable at any `A`, but a value can only be built for
/// `A: Type<A>` (`glean_typed::typed::Type<A: 'static>`, binary.rs:235;
/// the source's `Typeable` leg). Rustc does NOT elaborate the shipped
/// trait's `A: 'static` into the impl header, so the constructor impl
/// spells `+ 'static` explicitly (probe /tmp/opencode/db41/probe_e/,
/// E0310 without it) — every `Type<A>` impl satisfies it anyway.
///
/// `_marker` is private and phantom; `PhantomData<fn() -> A>` keeps the
/// auto traits independent of `A` (the `IdOf` precedent, typed/id.rs:
/// 193-194). The record selector `querySpec` is exported (`Query(..)`,
/// :16), so `query_spec` is pub. One structural consequence: the private
/// marker blocks cross-module FUNCTIONAL UPDATE (`Query { query_spec:
/// _, ..q }` is E0451; probe /tmp/opencode/db41/probe_a/
/// functional_update_fails.rs) while a destructuring pattern with `..`
/// stays legal; `into_spec` is the exported extraction helper for the
/// consumers that would have updated/destructured the record (the
/// record-literal/functional-update sites of query/thrift (Internal.hs,
/// row 45: :133/:139/:164/:176/:188/:200/:208/:218/:227/:233/:239) and
/// the dataFetch sites (row 146)).
pub struct Query<A> {
    /// `querySpec :: UserQuery` (:347).
    pub query_spec: thrift::UserQuery,
    _marker: PhantomData<fn() -> A>,
}

impl<A: Type<A> + 'static> Query<A> {
    /// The constructor `Query :: Type a => UserQuery -> Query a` (:346;
    /// "Build using 'query'", :345). The source's `query` smart
    /// constructor (Glean.hs, row 153) is a bare alias for this.
    pub fn new(query_spec: thrift::UserQuery) -> Self {
        Query {
            query_spec,
            _marker: PhantomData,
        }
    }
}

impl<A> Query<A> {
    /// The `querySpec` selector image for downstream modules (see the
    /// type docs). Unbounded: reading the spec never needs the
    /// constructor context, exactly like the source's selector.
    pub fn into_spec(self) -> thrift::UserQuery {
        self.query_spec
    }
}

/// `instance Show (Query a) where show (Query spec) = show spec`
/// (:350-351) — pure forwarding to the spec (no `A` constraint in the
/// source and none here).
impl<A> fmt::Debug for Query<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.query_spec, f)
    }
}

/// `instance Eq (Query a)` (:353-354): compares the specs. Hand-written
/// so the derive's `A: PartialEq` bound does not appear.
impl<A> PartialEq for Query<A> {
    fn eq(&self, other: &Self) -> bool {
        self.query_spec == other.query_spec
    }
}

impl<A> Eq for Query<A> {}

/// `instance Hashable (Query a)` (:356-357): hashes the spec only.
impl<A> Hash for Query<A> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.query_spec.hash(state);
    }
}

// ---------------------------------------------------------------------------
// Haxl seam: engine contract (manifest row 146)
// ---------------------------------------------------------------------------
//
// This file defines the request types (:362-465) and the per-datasource
// state payloads; the Haxl ENGINE — the `Haxl.PerformFetch` image, the
// request-cache executor, and the `ShowP` / `DataSourceName` /
// `StateKey` / `DataSource` class images — is owned by manifest row 146
// (`haxl/data_source/glean/common.rs`, from
// haxl/Haxl/DataSource/Glean/Common.hs), which materializes
// `crate::haxl::engine::{GleanGetFetcher, GleanQueryFetcher}`.
//
// Contract the engine must honour (each item cites its source and the
// governing inventory row — inventory.tsv:7/:8, inventory.protocol.tsv:
// 17/:23):
//   1. TWO sub-states: `Backend::init_global_state` returns exactly the
//      pair (GleanGetState, GleanQueryState) (:179-180; inventory.tsv:8
//      "BatchCtx has two sub-states (facts-batcher, query-batcher) —
//      initGlobalState returns exactly the pair"). The facts batcher and
//      the query batcher are separate state machines; never merge them.
//   2. per-TypeId dedupe: request equality is type-indexed — `Get`/
//      `GetKey` for different predicate types are never equal (`eqT @a
//      @b` guards, :382-385) and the hash includes `typeOf` (:388-392).
//      In Rust the type parameter does the `eqT` work and the Hash impls
//      below carry `TypeId::of::<P>()`; the engine's in-flight request
//      map must keep requests with equal ids but different type
//      parameters apart (inventory.tsv:8 "dedupe must respect
//      type-indexed equality -> per-TypeId request maps in Rust").
//      FOR GleanQuery THIS IS NOT ENOUGH: the source's `QueryReq` is
//      indexed by the RESULT pair `(r, Bool)` (Types.hs:412-418), and
//      Haxl's cache keys on the request's FULL type including that
//      index (haxl-2.5.1.1 Haxl/Core/DataCache.hs:69-97, outer
//      `HashTable TypeRep (SubCache res)` keyed by `typeOf request`) —
//      so `count q` (aggregate `UniqueResults`, DataSource/Glean.hs:
//      127-133) and `search_ q` (aggregate `AppendList`, :105-110) are
//      SEPARATE fetches in the source even for the same (q, repo,
//      stream). This file's `GleanQuery<Q>` indexes by Q only, so
//      `TypeId::of::<GleanQuery<Q>>()` alone cannot distinguish them:
//      the engine's dedupe key MUST include the result-aggregate type
//      R (e.g. key by `(TypeId::of::<GleanQuery<Q>>(), TypeId::of::<R
//      as QueryResult<Q>>::AGGREGATE)` or carry R as a phantom/index
//      parameter of the request) — merging count's and searchWith's
//      fetches would serve one aggregate where the source computes
//      both. A row-146 engine built to item 2 alone would diverge.
//   3. continuation resume stays OUTSIDE the cache: streaming is handled
//      behind the datasource abstraction instead of exposing resumable
//      queries as a request, precisely so the continuation is never
//      hashed into the Haxl cache (:405-410 comment; inventory.tsv:8
//      "continuation resume stays OUTSIDE the dedupe cache"). The
//      continuation lives in the `GleanQueryer` state, never in
//      `GleanQuery`.
//   4. executor decoration: every `Backend` wrapper passes
//      `init_global_state` through UNCHANGED and decorates only the
//      batch executor (inventory.tsv:7 = inventory.protocol.tsv:23:
//      Retry.hs:72/:127, Logging.hs:148, Backend/Types.hs:223,
//      LocalOrRemote.hs:242). The two states come from the innermost
//      backend; wrapping (retry/logging) applies in the fetch pipeline.
//   5. row 9's explicit batch API (`get_facts(&mut BatchCtx, reqs)`,
//      inventory.tsv:9) is this engine's flush surface: `BatchCtx` IS
//      the two sub-states and the flush is called at await points; the
//      trait below carries the per-operation primitives the flush drives
//      — the same split the source has (`syncGetOne`/`syncQueryOne`,
//      defined at Backend/Local.hs:211/:213, call exactly those
//      methods — the inventory row's evidence line names Common.hs,
//      but Common.hs has neither; the bodies are Local's). The API's
//      concrete name/shape is row 146's to fix (see the TODO below).
//   6. Sync vs Background is a per-backend choice made at
//      `initGlobalState` time and encoded IN the fetcher:
//      Local.hs:191-205 picks `SyncFetch` when `getNumCapabilities == 1`
//      and `BackgroundFetch` (streamed over a capabilities-sized pool)
//      otherwise; Remote.hs:442 (`| maxBatchSize > 1`; definition
//      :468) decides from `maxBatchSize`. The two
//      fetcher types must therefore be able to represent both modes.
//   7. the `ShowP` (`showp = show`, :377/:445), `DataSourceName`
//      ("GleanGet"/"GleanQuery", :394-395/:456-457) and `DataSource`
//      (`fetch (GleanGetState fetcher) _ _ = fetcher` and the
//      GleanQuery mirror, :402-403/:464-465) instances are row-146
//      images — do NOT re-declare them here (inventory.sites.tsv:537-542
//      lists exactly these sites).
// TODO(port): the Rust shapes are pending row 146, the owner: the two
// fetcher types (Sync/Background strategies) behind the aliases below,
// the request-trait materialization carrying per-request result types,
// and the instance images of contract item 7. This unit fixes the
// type-level interface the engine must satisfy (the aliases and state
// structs below); drift from this contract is a row-146 review item.

// ---------------------------------------------------------------------------
// GleanGet (:362-392)
// ---------------------------------------------------------------------------

/// `data GleanGet p where Get | GetKey` (:362-374): the type-indexed
/// Haxl request for fact/key fetches.
///
/// INDEX CONVENTION (documented divergence from the GADT index): in the
/// source the enum's parameter is the REQUEST RESULT type — `Get`
/// produces `p`, `GetKey` produces `KeyType p` — so `GetKey`'s index is
/// the KEY type while the id it stores is `IdOf p` (:369-374). This Rust
/// image indexes BOTH variants by the type of the STORED id (`P`): for
/// `Get`, `P` is also the result type; for `GetKey`, the result is
/// `P`'s key type, `<P as Predicate>::KeyType` (the associated type at
/// typed/predicate.rs:271; `Predicate: Sized + 'static + Type<Self>`,
/// :268, predicate.rs:251-254 notes the two associated-type
/// superclasses become consumer where clauses). The alternative (naming
/// both the id's type and the key type in the enum) would thread a
/// second parameter through every request-cache use for no equality
/// benefit: the source's own Eq/Hashable instances compare and hash
/// `typeOf p` — the ID's type (:388-392) — and never the key type.
///
/// The constructor contexts `(Typeable p, Show p, Predicate p)`
/// (:364, :370) are not re-declared as enum bounds: construction is the
/// engine's job and its functions carry the bounds (`GetKey` is only
/// interpretable with `P: Predicate` in scope — `predicate.rs:268`
/// supplies `'static`, which the Hash impl below needs). Eq/Hash/Debug
/// are hand-written so no bound leaks into the cache surface.
pub enum GleanGet<P> {
    /// `Get !(IdOf p) Bool Repo` (:363-368): fetch fact `id` (or its
    /// key-only form) from `repo`; `rec` includes dependencies.
    Get {
    /// `!(IdOf p)` — the strict field is structural; `IdOf` itself
    /// deliberately carries NO Copy/Clone (typed/id.rs:195 derive list;
    /// the deliberate-omission note at id.rs:97-101) — copyability goes
    /// through `id_of() -> Fid`, and `Fid` IS Copy.
    id: IdOf<P>,
        /// `Bool` — record recursion.
        rec: bool,
        repo: thrift::Repo,
    },
    /// `GetKey !(IdOf p) Bool Repo` (:369-374): fetch the KEY of fact
    /// `id`; result `KeyType p` (see the index convention above).
    GetKey {
        id: IdOf<P>,
        rec: bool,
        repo: thrift::Repo,
    },
}

/// `instance Eq (GleanGet p)` (:379-386), hand-written: the source
/// compares the wrapped ids, `rec`, and `repo`; comparing the raw `Fid`
/// via `id_of()` (typed/id.rs:213) sidesteps the derive's `P: PartialEq`
/// bound (id.rs:195 — same reason the impl is manual). The `eqT @a @b`
/// reflection guard (:382-385) is discharged by the type system: both
/// sides of a comparison share `P`, and requests of different `P` are
/// different Rust types entirely. Cross-variant pairs are unequal,
/// exactly like the source's `_ == _ = False` (:386).
impl<P> PartialEq for GleanGet<P> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                GleanGet::Get {
                    id: i,
                    rec: r,
                    repo: p,
                },
                GleanGet::Get {
                    id: j,
                    rec: s,
                    repo: q,
                },
            )
            | (
                GleanGet::GetKey {
                    id: i,
                    rec: r,
                    repo: p,
                },
                GleanGet::GetKey {
                    id: j,
                    rec: s,
                    repo: q,
                },
            ) => i.id_of() == j.id_of() && r == s && p == q,
            _ => false,
        }
    }
}

impl<P> Eq for GleanGet<P> {}

/// `instance Hashable (GleanGet a)` (:388-392): the source hashes the
/// tuple `(tag, typeOf p, p, rec, repo)` with tag `0` for `Get` and `1`
/// for `GetKey`. Image: hash the tag, then `TypeId::of::<P>()` — the
/// source's `typeOf p` is the TypeRep of the phantom newtype `IdOf p`
/// (Typed/Id.hs:36-39), so this is the outcome-equivalent
/// discrimination, not the same rep — then the raw `Fid`, `rec`, and
/// `repo`, in
/// the source's order. `P: 'static` is what `TypeId::of` needs; every
/// construction path has it (`Predicate: 'static`).
impl<P: 'static> Hash for GleanGet<P> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            GleanGet::Get { id, rec, repo } => {
                0u8.hash(state);
                TypeId::of::<P>().hash(state);
                id.id_of().hash(state);
                rec.hash(state);
                repo.hash(state);
            }
            GleanGet::GetKey { id, rec, repo } => {
                1u8.hash(state);
                TypeId::of::<P>().hash(state);
                id.id_of().hash(state);
                rec.hash(state);
                repo.hash(state);
            }
        }
    }
}

/// `deriving instance Show (GleanGet a)` (:376). Hand-written: the
/// source's standalone deriving infers its constructor contexts, and
/// Rust's derive macro would demand `P: Debug` (which the constructor
/// contexts do not). Renders the variant name and the three fields;
/// `IdOf`'s manual Debug prints the raw id (typed/id.rs:219-223).
/// `ShowP`'s `showp = show` (:377) is a row-146 image (seam block).
impl<P> fmt::Debug for GleanGet<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GleanGet::Get { id, rec, repo } => f
                .debug_struct("Get")
                .field("id", id)
                .field("rec", rec)
                .field("repo", repo)
                .finish(),
            GleanGet::GetKey { id, rec, repo } => f
                .debug_struct("GetKey")
                .field("id", id)
                .field("rec", rec)
                .field("repo", repo)
                .finish(),
        }
    }
}

/// `type GleanFetcher = Haxl.PerformFetch GleanGet` (:397) — the batched
/// fetch function over `GleanGet`, owned by row 146's engine (seam
/// block).
pub type GleanFetcher = crate::haxl::engine::GleanGetFetcher;

/// `instance Haxl.StateKey GleanGet where data State GleanGet =
/// GleanGetState GleanFetcher` (:399-400): the per-datasource state
/// payload. The engine's `State`/`StateKey` images are row 146's; this
/// type is the associated state value the `DataSource` image extracts
/// (`fetch (GleanGetState fetcher) _ _ = fetcher`, :402-403).
pub struct GleanGetState(pub GleanFetcher);

// ---------------------------------------------------------------------------
// GleanQuery (:412-465)
// ---------------------------------------------------------------------------

/// `data GleanQuery a where QueryReq :: (Show q, Typeable q, QueryResult
/// q r) => Query q -> Repo -> Bool -> GleanQuery (r, Bool)` (:412-418):
/// the type-indexed Haxl request for user queries.
///
/// INDEX CONVENTION (documented divergence, the GleanGet mirror): the
/// source indexes by the RESULT pair `(r, Bool)` — the `QueryResult q r`
/// aggregate and the truncated flag; this Rust image indexes by the
/// QUERY type `Q`, and the engine (row 146) picks `R: QueryResult<Q>` at
/// the fetch site and pairs it with the truncation flag. The source's
/// `(Show q, Typeable q, QueryResult q r)` context is the engine's to
/// enforce at construction/interpretation; Eq/Hash/Debug below stay
/// unbounded.
pub enum GleanQuery<Q> {
    /// `QueryReq (Query q) Repo Bool` (:413-418): run `query` against
    /// `repo`; `stream` requests streaming all results.
    QueryReq {
        query: Query<Q>,
        repo: thrift::Repo,
        stream: bool,
    },
}

/// `instance Eq (GleanQuery r)` (:447-450): compares query, repo, and
/// stream under an `eqT @a @b` guard on the query type; the guard is
/// discharged by the type system here (same `Q`), and `Query`'s Eq is
/// the spec comparison.
impl<Q> PartialEq for GleanQuery<Q> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                GleanQuery::QueryReq {
                    query: q1,
                    repo: r1,
                    stream: s1,
                },
                GleanQuery::QueryReq {
                    query: q2,
                    repo: r2,
                    stream: s2,
                },
            ) => q1 == q2 && r1 == r2 && s1 == s2,
        }
    }
}

impl<Q> Eq for GleanQuery<Q> {}

/// `instance Hashable (GleanQuery q)` (:452-453): the source hashes the
/// tuple `(q, s, repo)` — query, then REPO, then stream (the source's
/// pattern-binder names mislead: `QueryReq q s repo` is positional, so
/// `s` binds the Repo field and `repo` the Bool). No `typeOf` here —
/// unlike `GleanGet` (:388-392) the query's own hash covers its content,
/// and the continuation never becomes a request (:405-410).
impl<Q> Hash for GleanQuery<Q> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            GleanQuery::QueryReq {
                query,
                repo,
                stream,
            } => {
                query.hash(state);
                repo.hash(state);
                stream.hash(state);
            }
        }
    }
}

/// `deriving instance Show (GleanQuery q)` (:444). Hand-written like
/// `GleanGet`'s Debug (no `Q` bound in the source's effective context).
impl<Q> fmt::Debug for GleanQuery<Q> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GleanQuery::QueryReq {
                query,
                repo,
                stream,
            } => f
                .debug_struct("QueryReq")
                .field("query", query)
                .field("repo", repo)
                .field("stream", stream)
                .finish(),
        }
    }
}

/// `type GleanQueryer = Haxl.PerformFetch GleanQuery` (:459), owned by
/// row 146's engine (seam block).
pub type GleanQueryer = crate::haxl::engine::GleanQueryFetcher;

/// `instance Haxl.StateKey GleanQuery where data State GleanQuery =
/// GleanQueryState GleanQueryer` (:461-462); the `DataSource` image
/// extracts it (`fetch (GleanQueryState queryer) _ _ = queryer`,
/// :464-465). The continuation the streaming flag starts lives in this
/// state, not in the request (contract item 3).
pub struct GleanQueryState(pub GleanQueryer);

// ---------------------------------------------------------------------------
// AppendList / QueryResult (:420-442)
// ---------------------------------------------------------------------------

/// `newtype AppendList a = AppendList ([a] -> [a])` (:420-421) — "List
/// with O(1) append and O(n) conversion to []", aka DList.
///
/// Rust image: a plain `Vec<Q>` behind a private field. The DList's two
/// uses are served directly: appending is `Vec::extend`, conversion to
/// `[]` is a move-out. The private field means construction is ONLY via
/// `from_results`, `Default`, or `append` — no external consumer uses
/// the raw Haskell constructor (grep over glean/**/*.hs: only
/// `fromAppendList` is used outside this module, at
/// haxl/Haxl/DataSource/Glean.hs:102/:110), so the `AppendList(..)`
/// constructor export (:38) has no faithful open image and the
/// aggregate constructors carry the surface.
// PERF(port): the source's O(1) append (function composition) becomes
// O(n) (elements move once per append); the engine appends once per
// flushed batch, so total cost stays linear in elements produced.
// Accepted, not deferred machinery: the field is private and every
// constructor is in this file, so a persistent structure can replace
// the representation without touching the API if a hot loop appears.
pub struct AppendList<Q>(Vec<Q>);

impl<Q> Default for AppendList<Q> {
    /// `mempty = AppendList id` (:427) — the empty difference list.
    fn default() -> Self {
        AppendList(Vec::new())
    }
}

/// `fromAppendList (AppendList f) = f []` (:429-430) — run the
/// difference list: the elements, in order.
impl<Q> AppendList<Q> {
    pub fn from_append_list(self) -> Vec<Q> {
        self.0
    }
}

/// `instance Show a => Show (AppendList a) where show = show .
/// fromAppendList` (:432-433) — the image prints the elements (the
/// constraint is the source's own `Show a`).
impl<Q: fmt::Debug> fmt::Debug for AppendList<Q> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

/// `class Monoid r => QueryResult q r where fromResults :: [q] -> r`
/// (:435-436): the batching-result class. Rust image: `Default` is the
/// `Monoid` superclass's `mempty`, and `append` is the `<>` the engine
/// merges partial batch results with (Rust has no built-in analogue of
/// `Monoid` covering both operations, so the second operation is a
/// trait item). `Self` is `r` (the aggregate); the method parameter is
/// `q` (`Q` here).
///
/// Call-site note: a type that implements this for several `Q` — as
/// `Sum` does for every `Q` — cannot have `append` disambiguated by
/// method syntax; consumers write the fully-qualified form, e.g.
/// `<Sum<i64> as QueryResult<Q>>::append` (scratch-verified, probe
/// /tmp/opencode/db41/probe_c/).
pub trait QueryResult<Q>: Default {
    /// `fromResults :: [q] -> r` (:436).
    fn from_results(results: Vec<Q>) -> Self;

    /// `<>` from the `Monoid r` superclass (:435), spelled out because
    /// the engine needs it and the `Default` supertrait only supplies
    /// `mempty`. `self.append(other)` corresponds to `self <> other`.
    fn append(self, other: Self) -> Self;
}

/// `instance QueryResult q (AppendList q) where fromResults qs =
/// AppendList (qs++)` (:438-439): the element-collecting aggregate.
/// `append` is `AppendList x <> AppendList y = AppendList (x . y)`
/// (:423-424).
impl<Q> QueryResult<Q> for AppendList<Q> {
    fn from_results(results: Vec<Q>) -> Self {
        AppendList(results)
    }

    fn append(mut self, other: Self) -> Self {
        self.0.extend(other.0);
        self
    }
}

/// `instance QueryResult q (Sum Int) where fromResults = Sum . length`
/// (:441-442): ported for instance parity — NOT the count aggregate:
/// `count` (DataSource/Glean.hs:127-133) consumes `UniqueResults` (the
/// HashSet-dedup Monoid, :119-121); `Sum Int` has zero OSS consumers
/// (repo grep: its declaration only). A row-153 engine must wire count
/// to `UniqueResults`, never `Sum`.
/// `Sum<i64>`: Haskell `Int` -> `i64` (RULE §1 numeric row);
/// `append` adds the counts — the `Monoid (Sum Int)` this instance
/// rides on.
impl<Q> QueryResult<Q> for Sum<i64> {
    fn from_results(results: Vec<Q>) -> Self {
        Sum(results.len() as i64)
    }

    fn append(self, other: Self) -> Self {
        Sum(self.0 + other.0)
    }
}

/// Rust image of `Data.Monoid`'s `Sum` (imported at :56, used at
/// :441-442): a newtype whose monoid adds the wrapped value. A
/// base-library type with no Rust base — the first consumer owns it
/// (this file); the `getSum` consumer at Logging.hs:272 (row 148)
/// imports this one. `count` (row 153) consumes `UniqueResults`, not
/// this type.
///
/// `default` is `mempty` (`Sum 0` for the numeric `T`s in use) — the
/// `Default` supertrait of `QueryResult` forces it to exist (probe
/// evidence: without it `<Sum<i64> as QueryResult<Q>>::from_results`
/// cannot be instantiated, /tmp/opencode/db41/probe_c/). The other
/// derives mirror the source newtype's list trimmed to what consumers
/// use.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Sum<T>(pub T);

impl<T> Sum<T> {
    /// `getSum` — the accessor of `newtype Sum a = Sum { getSum :: a }`,
    /// consumed at Logging.hs:272.
    pub fn get_sum(self) -> T {
        self.0
    }
}

// ---------------------------------------------------------------------------
// Shards (:467-482)
// ---------------------------------------------------------------------------

/// `dbShard :: Thrift.Repo -> DbShard` (:470-471): the shard name for a
/// repo — `Text.pack . show . dbShardWord`, i.e. the DECIMAL string of
/// the shard word (feeds client query routing; consumers Remote.hs:277/
/// :319/:405-407, ShellPrint.hs:300/:352 per inventory.tsv:10).
/// `DbShard` is the shipped `String` alias
/// (glean_util::util::thrift_service::DbShard, thrift_service.rs:433).
pub fn db_shard(repo: &thrift::Repo) -> DbShard {
    db_shard_word(repo).to_string()
}

/// `dbShardWord :: Thrift.Repo -> Word64` (:473-482): MD5 of the repo's
/// `"name/hash"` bytes, first word, shifted right one bit.
///
/// Source mechanics and their images:
///   * the hashed bytes are `Text.encodeUtf8 repo_name <> "/" <>
///     Text.encodeUtf8 repo_hash` (:482) — here
///     `format!("{}/{}", repo.name, repo.hash)` — UTF-8, exact;
///   * `fingerprintData (castPtr ptr) len` -> the SHARED glean-angle
///     fingerprint module (`fingerprint_data`, hash.rs:230; mandated by
///     inventory.tsv:10 "reuse the SAME dedicated MD5 fingerprint
///     module as Hash.hs (one shared module, byte-exact)"). The
///     `unsafeDupablePerformIO` / `B.unsafeUseAsCStringLen` machinery
///     evaporates: `fingerprint_data` is pure in Rust (hash.rs:37-40);
///   * `Fingerprint w _ <- ...; return (w \`shiftR\` 1)` — the FIRST
///     word (`fingerprint1`, the first 8 digest bytes big-endian); the
///     `shiftR 1` keeps shards below 2^63 ("SR doesn't like shards >=
///     0x8000000000000000", :480-481; inventory.tsv:139's numeric-
///     conversions group row names this shift as its exemplar).
//
// INVENTORY-BUG-CANDIDATE row 10: the row's target_translation says
// `low word >> 1`; the source says FIRST word: `Fingerprint w _` binds
// the first field (:478), and glean/test/tests/TestShard.hs asserts
// dbShard (Repo "test" "1234") == "6825422262439797754" — MD5
// bd718ac52588c7f5291e839f03ed457c, first 8 bytes big-endian
// 0xbd718ac52588c7f5, >> 1 == 6825422262439797754 — which only the
// first word produces (a low-word/little-endian reading does not).
// Re-verified in this port against the shipped real `fingerprint_data`
// (golden match, probe /tmp/opencode/db41/probe_d/run.out); the same
// candidate is already recorded at hash.rs:27-30.
pub fn db_shard_word(repo: &thrift::Repo) -> u64 {
    let bytes = format!("{}/{}", repo.name, repo.hash).into_bytes();
    let Fingerprint {
        fingerprint1: w, ..
    } = fingerprint_data(&bytes);
    w >> 1
}

// Coverage: every export of the Haskell module has a Rust counterpart —
//   * types (:12-16): Backend(..) -> `pub trait Backend` (29 methods);
//     StackedDbOpts(..) -> the enum; LogDerivationResult -> the boxed-fn
//     alias; Query(..) -> `Query<A>` + `Query::new` (constructor) +
//     `Query::into_spec` (selector image).
//   * operations (:18-29): loadPredicates -> load_predicates;
//     loadPredicatesForSchema -> load_predicates_for_schema; databases ->
//     databases; localDatabases -> local_databases; create -> create;
//     finish -> finish; fillDatabase -> fill_database; finalize ->
//     finalize; completePredicates -> complete_predicates; untilDone ->
//     until_done. One-to-one, ten functions.
//   * Haxl (:31-39): GleanGet(..) -> the enum; GleanQuery(..) -> the
//     enum; GleanFetcher/GleanQueryer -> the two aliases; Haxl.State(..)
//     -> GleanGetState/GleanQueryState plus the row-146 engine's
//     state-key surface (seam block); QueryResult(..) -> the trait;
//     AppendList(..) -> the struct (see its docs for the constructor
//     disposition); fromAppendList -> `from_append_list`.
//   * Shards (:41-43): dbShard -> db_shard; dbShardWord -> db_shard_word.
// Every instance of the module is ported: Backend (Some Backend) ->
// `SomeBackend`'s impl; Show/Eq/Hashable (Query a), (GleanGet a),
// (GleanQuery a) -> Debug / PartialEq+Eq / Hash (hand-written, with the
// bound differences documented at each); Semigroup/Monoid (AppendList a)
// -> `append` + `Default`; Show (AppendList a) -> Debug;
// QueryResult q (AppendList q) and QueryResult q (Sum Int) -> the two
// impls. The Haxl.ShowP / DataSourceName / StateKey / DataSource
// instances are row-146 images and are not re-declared (seam block).
// Rust-only additions are exactly the four named in the header
// (SomeBackend, BackendError, try_bracket, Sum); `Sum::get_sum` is the
// one accessor added there so row 148's `getSum` consumer imports the
// family type rather than its field.
//
// INVENTORY (RULE §5) — the rows naming this file, and how each is
// discharged (cites use tsv LINE numbers, the repo-wide convention):
//   * inventory.tsv:7 (= inventory.protocol.tsv:23, wrapper
//     initGlobalState pass-through group row): wrappers delegate
//     init_global_state unchanged and decorate the executor — seam
//     contract item 4; the wrappers' own impls land with rows 43/148,
//     this unit fixes `SomeBackend`'s forwarding (:223).
//   * inventory.tsv:8 (= inventory.protocol.tsv:17): two sub-states /
//     per-TypeId dedupe / continuation outside the cache — discharged by
//     `Backend::init_global_state`, the Eq/Hash impls, and seam contract
//     items 1-3.
//   * inventory.tsv:9 (class Backend -> "Rust trait with explicit batch
//     API: get_facts(&mut BatchCtx, reqs) instead of per-key haxl;
//     dedupe+memoize in BatchCtx"): the explicit batch API is the row-146
//     engine's flush surface over `BatchCtx` = the two sub-states (seam
//     contract item 5; its concrete name/shape is row 146's, which the
//     seam TODO defers). The trait below carries the per-operation
//     primitives that flush drives — the same split the source has,
//     whose evidence line for this row names Common.hs, but the
//     `syncGetOne`/`syncQueryOne` bodies live in Backend/Local.hs:211/
//     :213 (Common.hs has neither — grep zero; folded into the flag).
//     INVENTORY-BUG-CANDIDATE row 9: this unit ships method-for-method
//     batching primitives and defers the row's `get_facts(&mut
//     BatchCtx, reqs)` shape to row 146's engine flush — the
//     reinterpretation the seam block documents in full; flagged here
//     per §5/A9 rather than left implicit.
//   * inventory.tsv:10 (= inventory.codeflow.tsv:22): dbShardWord via
//     the shared fingerprint module — followed; the row's "low word"
//     wording is the INVENTORY-BUG-CANDIDATE at `db_shard_word` above.
//   * inventory.tsv:11 (Local.hs initGlobalState Sync/Background choice):
//     governs the row-147 impl; the fetcher requirement it imposes is
//     seam contract item 6.
//   * inventory.tsv:40 (= inventory.protocol.tsv:28, Derive.hs LogResult,
//     EXC_TAXONOMY) + inventory.sites.tsv:591 (exc-classify at :190):
//     fix the LogDerivationResult image (decision (H)); the "DeriveOutcome
//     = Result<UserQueryStats, (Duration, Box<Error>)>" payload is what
//     the alias carries, and the `Send + Sync` on the box is this port's
//     addition (Derive.hs:136/:139 spawn boundary, documented at the
//     type) whose catch taxonomy is the open port todo at `BackendError`.
//   * inventory.tsv:139 (numeric conversions group row; its exemplar is
//     "Backend/Types.hs:474-480"): `>> 1` kept; the `as u64` retry cast
//     in `until_done` carries the source's truncation.
//   * inventory.sites.tsv:435/:436 (GHC.Fingerprint import; `Fingerprint
//     w _`): the shared-module import and the first-word destructure,
//     both above.
//   * inventory.sites.tsv:537-542 (haxl batching surface at :179/:223/
//     :394/:402/:456/:464): `init_global_state` and `SomeBackend`'s
//     forwarding are here; the four DataSourceName/DataSource sites are
//     row-146 images (seam contract item 7).
// No row names `Query`, the ten free operations, or the `QueryResult`/
// `AppendList` surface — and none can govern `try_bracket` (the
// hsthrift module has no manifest unit; header note).
//
// Marker breakdown (closed set — exactly 3 open port todos, 1
// performance deferral, 2 inventory divergences, no source-defect
// reproductions):
//   * port todos: (1) the Haxl seam — engine shapes/instance images
//     await row 146 (canonical flag line in the seam block); (2) the
//     BackendError taxonomy — the catch table awaits the concrete
//     backends (rows 147/148/166) and the exception-taxonomy rows
//     (inventory.tsv:13, :54; sites.tsv:591), flag at the enum;
//     (3) try_bracket's panic-unwind leg — the flagged real divergence
//     at the stand-in.
//   * one performance deferral: AppendList is Vec-backed (append O(n)
//     where the source's DList composes in O(1)); total cost stays
//     linear in elements produced — flag at the struct.
//   * two inventory divergences: the row-10 wording at `db_shard_word`
//     (first word, not "low word" — golden-verified; flag at the shard
//     function, mirrored from hash.rs:27-30) and the row-9 batch-API
//     reinterpretation (flag at the inventory bullet — the row's
//     get_facts shape is row 146's flush surface).
//   * no source-defect reproductions: the source is well-formed. The
//     only divergence found is in a decision RECORD (Glean.Util.Some's
//     backend list, some.rs:57-66) — it concerns a different module's
//     record, is documented in this file's RECORD DIVERGENCE block, and
//     is not a defect of this unit's Haskell source.
//
// PORT STATUS: confidence=medium todos=3
