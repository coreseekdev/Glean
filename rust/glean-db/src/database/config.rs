// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// Ported from: glean/db/Glean/Database/Config.hs (590 lines).
// Manifest unit: rust/glean-db/src/database/config.rs
// (migration/manifest.tsv line 85). RULE §4: one source file -> one target
// file; the manifest path is authoritative.
//
// WHAT THIS FILE IS: the database layer's configuration surface.
//   * DataStore (:112-164) — where DBs live (file tree / temp dir / memory)
//     plus the scoped "open the storage backends" action.
//   * Config/DebugFlags (:166-258) — the record every DB consumer threads
//     through, its default, and its Semigroup/Monoid/Show instances.
//   * The schema pipeline (:260-480) — SchemaIndex/ProcessedSchema, the
//     parse+resolve+hash steps, the ThriftSource constructors that read a
//     schema from a directory/file/index/config, and the location logic.
//   * The CLI surface (:482-590) — schema-location parsing, --schema, and
//     the big options parser.
//
// ---------------------------------------------------------------------------
// INVENTORY (RULE §5; migration/inventory*.tsv)
// ---------------------------------------------------------------------------
//
// grep -n 'Glean/Database/Config.hs' migration/inventory*.tsv returns six
// rows: the three lazy-io site rows below, the DB-admin STM_GROUP row
// (inventory.tsv:125), and the strict-eager-reads group row, which names
// the same three sites under the abbreviated spelling
// 'db/Glean/Database/Config.hs:410,423,425' in both
// migration/inventory.tsv:19 and migration/inventory.protocol.tsv:15. All
// are followed:
//
//   * migration/inventory.sites.tsv:441 (lazy-io, Config.hs:410)
//     'str <- B.readFile (dir </> Text.unpack schemaInstance_file)' in
//     parseSchemaIndex. RULE §1's lazy-IO ban: the port reads eagerly with
//     std::fs::read in load_json/parse_schema_index (no lazy handle).
//   * migration/inventory.sites.tsv:442 (lazy-io, Config.hs:423)
//     'version <- mapM B.readFile ...' in catSchemaFiles. Eager read.
//   * migration/inventory.sites.tsv:443 (lazy-io, Config.hs:425)
//     'str <- B.readFile file' in catSchemaFiles. Eager read.
//   * migration/inventory.tsv:125 (STM_GROUP, DB admin operations;
//     "Config.hs schema-source defaults become plain consts (Config.hs:108,
//     454 #ifdef OSS deleted per RULEBOOK 2)"). Both CPP blocks are gone:
//     :108-110 (the Paths_glean import) and :454-458 (the datadir
//     substitution). Per RULEBOOK §1's CPP row ("delete — OSS-only build")
//     the OSS body is the live one and is kept: the #else arm
//     (:456-457 'datadir <- return id') is the FB arm and is deleted. No
//     STM state group is created here; the file only reads config.
//     "schema-source defaults become plain consts": schemaSourceDir IS a
//     const (SCHEMA_SOURCE_DIR, :430-431). rocksdbName/lmdbName and
//     schemaLocationFiles stay functions because StorageName/SchemaLocation
//     own a String and Rust consts cannot (DIVERGENCES 6) — a language
//     constraint, not a divergence. defaultSchemaLocation itself is row
//     82's.
//
// The group row's target is "direct std::fs::read — 1:1, no decision
// needed", which is exactly what load_json/parse_schema_index/
// cat_schema_files do; the abbreviated and the full spelling govern the
// same three sites. Every row is FOLLOWED — zero inventory divergences in
// this file.
//
// ---------------------------------------------------------------------------
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, checked not assumed)
// ---------------------------------------------------------------------------
//
// OUTBOUND (edges.tsv:211-236), with the Rust owner of each:
//   :211 Angle/Types        -> glean_angle::angle::types (SourceSchemas)
//   :212 Schema/Resolve     -> glean_angle::schema::resolve
//   :213 Schema/Types       -> glean_angle::schema::types (ResolvedSchemas)
//   :214 Backup/Backend     -> database/backup/backend.rs (Backends)
//   :215 Backup/Mock        -> database/backup/mock.rs (mock)
//   :216 BatchLocation      -> database/batch_location.rs (SomeParser)
//   :217 Catalog            -> database/catalog.rs (Catalog)
//   :218 Catalog/Local/Files-> database/catalog/local/files.rs (file_catalog)
//   :219 Catalog/Local/Memory->database/catalog/local/memory.rs (memory_catalog)
//   :220 Catalog/Store      -> database/catalog/store.rs (CatalogStore)
//   :221 Schema/ComputeIds  -> database/schema/compute_ids.rs
//   :222 Storage            -> database/storage.rs (take_directory,
//                              with_temp_directory)
//   :223 Storage/LMDB       -> database/storage/lmdb.rs (LMDB::new_storage)
//   :224 Storage/Memory     -> database/storage/memory.rs (new_storage)
//   :225 Storage/RocksDB    -> database/storage/rocks_db.rs (new_storage)
//   :226 Database/Trace     -> database/trace.rs (GleanTrace)
//   :227 Logger/Database    -> logger/database.rs
//   :228 Logger/Server      -> logger/server.rs
//   :229 github/Database/AclKnobs -> database/acl_knobs.rs
//   :230 github/DefaultConfigs    -> default_configs.rs
//   :231 Util/ConfigProvider     -> glean_util::util::config_provider
//   :232 Util/Observed           -> glean_util::util::observed
//   :233 Util/ShardManager       -> glean_util::util::shard_manager
//   :234 Util/Some               -> glean_util::util::some (exports nothing;
//                                   inventory row 136 maps the existential at
//                                   each use site — see the payload note below)
//   :235 Util/ThriftSource       -> glean_util::util::thrift_source
//   :236 Util/Trace              -> glean_util::util::trace (Listener)
//
// INBOUND (edges.tsv, none edited by this unit): :32 backend-local,
// :239 Create, :271 Env, :370 Schema, :449 Types, :653 Query/Typecheck/Monad,
// :732 Backup/S3, :1343 AngleIndexer/Builder, :1347 AngleIndexer/Main,
// :1555 Schema/Gen/Main, :1589 Server, :1626 Shell, :1658 Shell/Types,
// :1682 tools/diff, :1689 tools/disassemble, :1700 gleancli, :1758
// Merge, :1771 Query, :1804 plugin/Types.
//
// ---------------------------------------------------------------------------
// ASSUMED-IMPORT (RULE §4 B4)
// ---------------------------------------------------------------------------
//
//   * crate::internal::types::{SchemaIndex, SchemaInstance, StorageName} —
//     the gen-hs2 image of glean/if/internal.thrift (SchemaIndex :143-149,
//     SchemaInstance :93-140, StorageName typedef at :51). The module has no
//     manifest unit; it is the generated-thrift home DECISIONS.md pinned to
//     glean-db (meta.rs:231-248 records the pin and the alias image
//     'pub type StorageName = String'). ASSUMED SHAPE:
//         pub type StorageName = String;
//         #[derive(Clone)] pub struct SchemaInstance {
//             pub versions: BTreeMap<String, Version>, // :120
//             pub file: String,                        // :133
//         }
//         #[derive(Clone)] pub struct SchemaIndex {
//             pub current: SchemaInstance,             // :145
//             pub older: Vec<SchemaInstance>,          // :148
//         }
//     Clone/Send/Sync are required by ThriftSource's bounds (thrift_source.rs:
//     270-276); the generated crate owes them.
//   * glean_thrift::{Config, SchemaId, SchemaLocation, Version} — generated
//     from glean/if/glean.thrift and glean/config/server/server_config.thrift
//     (SchemaLocation union :170-189, Config.schema_location :322,
//     'typedef string SchemaId', 'typedef i32 Version'). One generated crate
//     for all .thrift files is the A10 default recorded at meta.rs:103-108 and
//     relied on by lmdb.rs:69-72 / rocks_db.rs:53. NOTE: default_configs.rs:92
//     imports SchemaLocation from 'glean_server_config' instead (that file's
//     own assumption, server_config.thrift:23's 'namespace rust'); the two
//     must name the same type at Step 4 — see SIBLING-UNIT CONTRACTS.
//     ASSUMED union spelling: Dir/File/Index/Config/IndexConfig plus the
//     fbthrift unset arm Empty (meta.rs:225-231's convention; the source's
//     constructors are SchemaLocation_dir/_file/_index/_config/
//     _indexconfig/_EMPTY).
//   * glean_util::log::log_info — the unit-45/46 convention (rocks_db.rs:62-63):
//     a lazy message thunk, always forced.
//   * serde::de::DeserializeOwned / serde_json — the image of
//     Thrift.Protocol.JSON.deserializeJSON (hsthrift, external; see
//     Thrift/Util.hs:30-36 for loadJSON's error text).
//   * Control.Trace (Tracer) — external hsthrift package, NO manifest unit.
//     See the local Tracer sketch below (one port todo).
//   * Util.IO (listDirectoryRecursive) and Paths_glean (getDataDir) —
//     external/generated, NO manifest unit. See the local sketches below
//     (one port todo each).
//
// ---------------------------------------------------------------------------
// SIBLING-UNIT CONTRACTS (what downstream units may rely on)
// ---------------------------------------------------------------------------
//
//   * row 104 (database/types.rs) MUST provide the closed enum that images
//     'Some Storage' (inventory row 137: 'enum Storage {Rocks, Lmdb,
//     Memory}'). This file is its first consumer and spells the variant
//     payloads it constructs: Storage::Rocks(RocksDB),
//     Storage::Lmdb(LMDB), Storage::Memory(Memory). The payload map is
//     HashMap<StorageName, Storage>, NOT a Box<dyn Storage>: the trait has a
//     generic method (with_scratch_root, storage.rs:548-551), so it is not
//     object-safe; the source's existential is closed (row 137). NOTE: row
//     137's literal text governs 'odbHandle :: Some DatabaseOps' (the
//     open-database existential), while Config's map holds BACKENDS; row 104
//     may need TWO closed enums, and row 137's literal text covers the
//     DatabaseOps one.
//   * THE !Send DECISION (row 71's TODO at storage/memory.rs:218-227). The
//     storage map is handed to Env and stored (Types.hs:251 envStorage,
//     passed at Env.hs:115 'initEnv ...'), and Env is shared across threads.
//     DataStore::with_storage takes the map BY VALUE (the caller moves it
//     into Env), so Env's Send + Sync requirement lands on
//     HashMap<StorageName, Storage>. That requirement is NOT enforced by
//     this file's signature — the '&mut dyn FnMut' parameter carries no
//     bound. Rows 71 (Memory) and 104 (Env) own the choice: a thread-local
//     Memory or a revised FactSet !Send posture (memory.rs:218-227). One
//     port todo at the payload doc.
//   * row 132 (database/env.rs) consumes: cfg_schema_hook, cfg_shard_manager
//     (via Config::with_shard_manager), cfg_server_logger,
//     cfg_database_logger, cfg_batch_location_parser, cfg_read_only,
//     cfg_mock_writes, cfg_listener, cfg_schema_id, cfg_backup_backends,
//     cfg_enable_recursion, cfg_filter_available_dbs,
//     cfg_acl_group_resolver, cfg_acl_calculate_enabled,
//     cfg_acl_check_enabled, cfg_tracer, cfg_debug (Env.hs:152-172).
//   * Config is GENERIC over the ConfigProvider (Config<C>): the source's
//     ThriftSource is rank-2 over cfg, while row 81's Rust image is
//     ThriftSource<A, C> (thrift_source.rs:158-161, DIVERGENCES 1 there).
//     cfg_schema_hook and cfg_server_config both use the SAME C.
//   * SchemaIndex is NOT Clone (HashedSchema, compute_ids.rs:264, has no
//     Clone — the source has no deriving clause), but row 81's constructors
//     require A: Clone + Send + Sync + 'static. The ThriftSource payload is
//     therefore Arc<SchemaIndex>: the Haskell value is shared, not copied, so
//     Arc is the faithful image and row 66 need not grow an invented Clone.
//     schema_for_schema_id and the SchemaIndex accessors return references.
//   * row 82 (default_configs.rs) supplies default_schema_location(),
//     SERVER_CONFIG_PATH and SCHEMA_CONFIG_PATH. OPEN QUESTION for Step 4:
//     row 82's SchemaLocation comes from 'glean_server_config', this file's
//     from 'glean_thrift' — they must be one type (one-crate assumption) or
//     one of the two imports must move.
//     TODO(port): schema_location's '.unwrap_or_else(default_schema_location)'
//     is ill-typed until that is settled: default_configs.rs:92 imports
//     SchemaLocation from 'glean_server_config', while this file's
//     re-export (:344) and the three units that adopt glean_thrift
//     (lmdb.rs:69, rocks_db.rs:53, meta.rs:103-108) name glean_thrift's.
//     Owner: Step-4 crate wiring (it picks the single crate); do not move
//     this file's import.
//   * rows 112/116/125 (write/batch.rs, writes.rs, backup.rs) call traceMsg;
//     this file's Tracer only carries the log half — see the port todo.
//
// ---------------------------------------------------------------------------
// DIVERGENCES (deliberate; no inventory row governs them)
// ---------------------------------------------------------------------------
//
//   1. DataStore(..)'s withStorage field is a rank-2 function
//      ('forall a. Config -> ((map, store) -> IO a) -> IO a', :116-119).
//      Rust has no rank-2 function field, and a boxed 'dyn Fn' cannot be
//      generic over the action's return type. The port keeps the field
//      (public, as DataStore(..) requires) as a boxed erased-callback
//      closure and adds a GENERIC METHOD of the same name (the
//      thrift_source.rs:158-215 with_value precedent: the generic method
//      wraps the caller's closure in a FnMut writing into an Option; a field
//      and a method may share a name). default_storage/data_store_tag are
//      plain public fields. Tests
//      construct DataStore literals (LifecycleTest.hs:58-62); tests are not
//      translated (RULE §4), so the only production constructors are
//      file_data_store/tmp_data_store/memory_data_store.
//   2. cfgShardManager is rank-2 the same way (:183-188) and gets the same
//      treatment: a public erased-callback field plus Config::with_shard_manager.
//   3. Config is generic over C (see SIBLING-UNIT CONTRACTS); the source's
//      Config is not parameterized.
//   4. ThriftSource<Arc<SchemaIndex>, C> instead of ThriftSource SchemaIndex
//      (row 81's A: Clone bound; see SIBLING-UNIT CONTRACTS).
//   5. processSchemaForTesting's 'toList :: forall k v. HashMap k v -> [(k,v)]'
//      parameter is collapsed, because row 66 already collapsed it
//      (compute_ids.rs:165-171: every result is order-independent, and
//      tests are not translated). The export keeps its name and loses the
//      parameter.
//   6. CAF values (tmpDataStore, memoryDataStore, rocksdbName, lmdbName,
//      schemaLocationFiles, schemaSourceDir) become functions/consts: a
//      StorageName is an owned String and a DataStore owns an Arc, neither of
//      which can be a Rust const. default_configs.rs:65-70 records the same
//      mapping for its CAFs.
//   7. options/schemaLocationOption: optparse-applicative has no image in
//      this tree (config_provider.rs:198-203 defers its Parser method for the
//      same reason) and clap is NOT on RULE §1's dependency whitelist. The
//      port is a hand-written std-only parser. Faithful: every flag, its
//      argument shape (--opt VALUE and --opt=VALUE), the <|> precedence of
//      dbRoot/dbTmp/dbMem and serverConfigThriftSource/serverConfigTier, the
//      ignored --db-schema-override, and the DebugFlags pair. Also
//      reproduced (the parser was more permissive before): --lmdb is legal
//      only with --db-root (:528-536); --db-tmp/--db-memory are illegal
//      with --db-root (:543); --tier is illegal with --server-config
//      (:547-550); repeated --db-root/--schema are errors (optparse consumes
//      each at most once); and a bare '--' terminates option parsing, after
//      which any argument is an error (no positionals). NOT reproduced: the
//      rendering of help/usage and of parse errors, exit codes,
//      --help/--version, the optparse 'internal' hiding of
//      --experimental-recursion, metavar/help text, and optparse's parser
//      internals (the port todo at options).
//   8. showSchemaLocation/schemaLocationToSource panic with the source's
//      'error' messages ("showSchemaLocation"/"schemaLocationToSource") on
//      the unset union arm (RULE §2 partial-function rule).
//   9. loadJSON (Thrift.Protocol.JSON) becomes serde_json on the generated
//      type. Thrift.Util.hs:30-36's error text is reproduced exactly
//      ("[<path>] <err>") but serde's inner message is not hsthrift's.
//  10. SchemaError is a Rust-only error enum (RULE §2 dynamic-exceptions
//      row): the source throws ErrorCall/ThriftException/IOException from
//      parseSchemaDir/parseSchemaIndex. Display follows RULE §2's Show row
//      for the constructors whose source Show is constructor-shaped:
//      'ThriftException {:?}' (hsthrift Thrift/Util.hs:24-25, a newtype
//      deriving Show) and 'Exception {{message = {:?}}}' (glean.thrift:225,
//      a one-field record with derived Show). ErrorCall displays its bare
//      message, as ErrorCall's Show does. Io keeps std::io::Error's text:
//      Haskell's IOException Show also carries the path and the failing
//      function ('openFile: ...'), which cannot be reproduced without
//      re-wrapping the error at every read site.
//
// ---------------------------------------------------------------------------
// EXPORT MAPPING (source export list :10-49)
// ---------------------------------------------------------------------------
//
//   DataStore(..)                -> pub struct DataStore (all three fields
//                                   public; with_storage's type is the erased
//                                   callback, plus a generic method)
//   StorageName                  -> pub use crate::internal::types::StorageName
//   fileDataStore                -> pub fn file_data_store
//   tmpDataStore                 -> pub fn tmp_data_store
//   memoryDataStore              -> pub fn memory_data_store
//   rocksdbName / lmdbName       -> pub fn rocksdb_name / lmdb_name
//   options                      -> pub fn options
//   Config(..)                   -> pub struct Config<C> (all fields pub)
//   DebugFlags(..)               -> pub struct DebugFlags (both fields pub)
//   SchemaLocation(..)           -> pub use glean_thrift::SchemaLocation
//   showSchemaLocation           -> pub fn show_schema_location
//   schemaLocation               -> pub fn schema_location
//   schemaLocationOption         -> pub fn schema_location_option
//   processSchema                -> pub fn process_schema
//   processSchemaCached          -> pub fn process_schema_cached
//   processOneSchema             -> pub fn process_one_schema
//   SchemaIndex(..)              -> pub struct SchemaIndex (fields pub)
//   schemaForSchemaId            -> pub fn schema_for_schema_id
//   ProcessedSchema(..)          -> pub struct ProcessedSchema (fields pub)
//   catSchemaFiles               -> pub fn cat_schema_files
//   schemaLocationToSource       -> pub fn schema_location_to_source
//   parseSchemaDir               -> pub fn parse_schema_dir
//   parseSchemaIndex             -> pub fn parse_schema_index
//   loadSchemaIndex              -> pub fn load_schema_index
//   processSchemaForTesting      -> pub fn process_schema_for_testing
//   schemaLocationFiles          -> pub fn schema_location_files
//   schemaSourceDir              -> pub const SCHEMA_SOURCE_DIR
//
// NOT exported in the source, hence private here: memoryName,
// schemaSourceConfig, schemaSourceFile, schemaSourceIndexFile,
// schemaSourceFilesFromDir, schemaSourceIndexConfig, checkVersions,
// schemaLocationParser, debugParser, serverConfigThriftSource,
// serverConfigTier.
//
// EXTRA PUBLIC SURFACE (Rust-only, justified):
//   * Tracer, Tracer::combine, Tracer::default — Control.Trace is external
//     hsthrift with no manifest unit; cfg_tracer needs the type.
//   * SchemaError — the Rust image of the exceptions this file throws
//     (DIVERGENCES 10); callers must be able to name the error.
//   * DataStore::with_storage — the callable form of the erased
//     'with_storage' field (DIVERGENCES 1).
//   * Config::with_shard_manager — the callable form of the erased
//     cfgShardManager field (DIVERGENCES 2).
//   * DebugFlags::combine — the Semigroup instance (RULE §2 mapping).
// The WithStorage type alias and the take-option helper are private.

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::path::Path;
use std::sync::Arc;

use serde::de::DeserializeOwned;

use glean_angle::angle::types::SourceSchemas;
use glean_angle::schema::resolve::{
    parse_and_resolve_schema, parse_and_resolve_schema_cached, SchemaParserCache,
};
use glean_angle::schema::types::ResolvedSchemas;
use glean_thrift::{Config as ServerConfig, Repo, SchemaId, Version};
// 'ServerConfig.SchemaLocation(..)' is re-exported by the source (:26), so
// this is a public re-export (the generated union's five payload variants
// plus the unset arm are its public surface).
pub use glean_thrift::SchemaLocation;
use glean_util::log::log_info;
use glean_util::util::config_provider::{ConfigError, ConfigProvider, Deserializer};
use glean_util::util::observed::Observed;
use glean_util::util::shard_manager::{no_sharding, SomeShardManager};
use glean_util::util::thrift_source::{
    config, config_with_deserializer, file_with_deserializer, generic_config, once,
    parse as parse_thrift_source, ThriftSource,
};
use glean_util::util::trace::Listener;

use crate::database::acl_knobs::{acl_calculate_enabled, acl_check_enabled};
use crate::database::backup::backend::Backends;
use crate::database::backup::mock::mock;
use crate::database::batch_location::{DefaultParser, SomeParser};
use crate::database::catalog::local::files::file_catalog;
use crate::database::catalog::local::memory::memory_catalog;
use crate::database::catalog::store::CatalogStore;
use crate::database::catalog::Catalog;
use crate::database::schema::compute_ids::{compute_ids, HashedSchema};
use crate::database::storage::lmdb::LMDB;
use crate::database::storage::memory::new_storage as new_memory_storage;
use crate::database::storage::rocks_db::RocksDB;
use crate::database::storage::{take_directory, with_temp_directory};
use crate::database::trace::GleanTrace;
use crate::default_configs::{default_schema_location, SCHEMA_CONFIG_PATH, SERVER_CONFIG_PATH};
/// 'Glean.Internal.Types (StorageName(..))' is re-exported by the source
/// (:13); the assumed image is 'pub type StorageName = String' (meta.rs:237).
pub use crate::internal::types::StorageName;
use crate::internal::types::{
    SchemaIndex as InternalSchemaIndex, SchemaInstance as InternalSchemaInstance,
};
use crate::logger::database::{GleanDatabaseLogger, NullGleanDatabaseLogger};
use crate::logger::server::{GleanServerLogger, NullGleanServerLogger};

// FORWARD REFERENCE (row 104, src/database/types.rs): the closed enum that
// images 'Some Storage' (inventory row 137). Row 104 has not shipped; this is
// the same deferral database/storage.rs:671-678 records for the
// 'Some DatabaseOps' forwarding instances. The variant payloads this file
// constructs are Storage::Rocks(RocksDB), Storage::Lmdb(LMDB) and
// Storage::Memory(Memory).
// TODO(port): row 104 must ship 'pub enum Storage { Rocks(RocksDB),
// Lmdb(LMDB), Memory(Memory) }' (owner: the row-104 unit) before this file
// resolves.
use crate::database::types::Storage;

/// 'Tracer msg' from Control.Trace.Core (:44-50) — ASSUMED IMAGE.
///
/// Control.Trace is an external hsthrift package with NO manifest unit
/// (hsthrift/common/util/Control/Trace/Core.hs:44-50). The source record has
/// two halves: 'logMsg_ :: msg -> IO ()' and
/// 'traceMsg_ :: msg -> IO (ExitCase a -> IO ())', with Monoid/Semigroup
/// (:63-75) and Contravariant. Only the log half is modelled: no translated
/// consumer calls traceMsg yet (rows 112/116/125 own those sites) and
/// ExitCase has no image in this tree.
///
/// CROSS-REFERENCE: database/trace.rs:100-105 records the same facts (an
/// earlier revision of that comment wrongly placed Tracer in a future
/// 'util/trace.rs', row 84): row 84 (Util/Trace.hs) shipped WITHOUT Tracer
/// — it is Glean.Util.Trace's Listener — and Control.Trace has no manifest
/// unit at all. This sketch is Tracer's only home until Step 4 decides one.
// TODO(port): add the traceMsg_ half (and its ExitCase image) before rows
// 112/116/125 land; owner: the first unit that consumes traceMsg, or the
// Step-4 wiring that gives Control.Trace a home. This sketch lives here
// because Config.hs:209 is the first consumer; database/types.rs (row 104)
// may import it from here until a real home exists.
pub struct Tracer<A: 'static>(Arc<dyn Fn(&A) + Send + Sync>);

impl<A: 'static> Tracer<A> {
    /// 'instance Semigroup (Tracer msg)' (Core.hs:66-75): log through both,
    /// left then right.
    pub fn combine(self, other: Self) -> Self {
        let first = self.0;
        let second = other.0;
        Tracer(Arc::new(move |msg: &A| {
            first(msg);
            second(msg);
        }))
    }
}

impl<A: 'static> Default for Tracer<A> {
    /// 'mempty = Tracer (\_ -> pure ()) ...' (Core.hs:63-64).
    fn default() -> Self {
        Tracer(Arc::new(|_: &A| {}))
    }
}

// ---------------------------------------------------------------------------
// DataStore (:112-164)
// ---------------------------------------------------------------------------

/// The erased-callback image of DataStore's rank-2 'withStorage' field
/// (:113-119). The Haskell field is
/// 'forall a. Config -> ((map, store) -> IO a) -> IO a'; a Rust closure
/// cannot be generic over the action's return type, so the stored closure
/// takes a '&mut dyn FnMut' and the public method below captures the result
/// (the thrift_source.rs:158-215 with_value shape). Private: see DIVERGENCES 1.
type WithStorage = Arc<
    dyn Fn(
            &ServerConfig,
            &mut dyn FnMut(HashMap<StorageName, Storage>, CatalogStore) -> Result<(), ConfigError>,
        ) -> Result<(), ConfigError>
        + Send
        + Sync,
>;

/// 'data DataStore' (:112-124). The storage backends are opened inside the
/// scoped action because doing so may create temporary resources.
///
/// The payload is 'HashMap<StorageName, Storage>' — the closed-enum image of
/// 'HashMap StorageName (Some Storage)' per inventory row 136 (glean-util's
/// some.rs exports nothing; the mapping is per use site).
// TODO(port): the payload's Send + Sync is NOT resolved here. The
// '+ Send + Sync' above bounds the erased CLOSURE, not the payload: the
// HashMap<StorageName, Storage> crosses '&mut dyn FnMut' with no bound, so
// this signature does not enforce anything. Env does store the map
// (Types.hs:251 envStorage, passed at Env.hs:115) and Env is shared across
// threads, while memory_data_store hands out Storage::Memory(Memory), which
// storage/memory.rs:218-227 documents as !Send/!Sync. Rows 71/104 own the
// decision (thread-local Memory or a revised FactSet posture).
pub struct DataStore {
    pub with_storage: WithStorage,
    /// Default storage, Bool => explicitly selected on CLI (otherwise can be
    /// overridden by ServerConfig.config_db_create_storage) (:120-122).
    pub default_storage: (StorageName, bool),
    pub data_store_tag: String,
}

impl DataStore {
    /// 'withStorage' (:113-119), generic over the action's result.
    pub fn with_storage<A, F>(&self, scfg: &ServerConfig, f: F) -> Result<A, ConfigError>
    where
        F: FnOnce(HashMap<StorageName, Storage>, CatalogStore) -> Result<A, ConfigError>,
    {
        let mut out: Option<A> = None;
        let mut f = Some(f);
        (self.with_storage)(scfg, &mut |storages, catalog| {
            let f = f
                .take()
                .expect("DataStore::with_storage: the action runs exactly once");
            out = Some(f(storages, catalog)?);
            Ok(())
        })?;
        Ok(out.expect("DataStore::with_storage: the action runs exactly once"))
    }
}

/// 'rocksdbName = StorageName "rocksdb"' (:127). A function, not a const:
/// StorageName is an owned String (DIVERGENCES 6).
pub fn rocksdb_name() -> StorageName {
    "rocksdb".to_string()
}

/// 'lmdbName = StorageName "lmdb"' (:128).
pub fn lmdb_name() -> StorageName {
    "lmdb".to_string()
}

/// 'memoryName = StorageName "memory"' (:129) — private in the source, so
/// private here.
fn memory_name() -> StorageName {
    "memory".to_string()
}

/// 'fileDataStore' (:131-145).
pub fn file_data_store(path: &str) -> DataStore {
    let path = path.to_string();
    // Built BEFORE the closure moves 'path' (:140 'dataStoreTag ...').
    let data_store_tag = format!("db:{}", path);
    DataStore {
        with_storage: Arc::new(move |scfg, f| {
            // 'rocksdb <- RocksDB.newStorage path scfg' (:134): the only
            // fallible constructor (Result<_, FfiError>).
            let rocksdb = RocksDB::new_storage(&path, scfg)?;
            let lmdb = LMDB::new_storage(&path, scfg);
            let mut storages: HashMap<StorageName, Storage> = HashMap::new();
            storages.insert(rocksdb_name(), Storage::Rocks(rocksdb));
            storages.insert(lmdb_name(), Storage::Lmdb(lmdb));
            f(storages, CatalogStore::Files(file_catalog(&path)))
        }),
        default_storage: (rocksdb_name(), false),
        data_store_tag,
    }
}

/// 'tmpDataStore' (:147-154): a fresh system temp directory named "glean".
pub fn tmp_data_store() -> DataStore {
    DataStore {
        with_storage: Arc::new(|scfg, f| {
            let parent = std::env::temp_dir().to_string_lossy().into_owned();
            with_temp_directory(&parent, "glean", |tmp| {
                log_info(|| format!("Storing temporary DBs in {}", tmp));
                file_data_store(tmp).with_storage(scfg, |storages, catalog| f(storages, catalog))
            })
        }),
        default_storage: (rocksdb_name(), false),
        // 'dataStoreTag (fileDataStore "<tmp>")' (:153) = "db:<tmp>".
        data_store_tag: "db:<tmp>".to_string(),
    }
}

/// 'memoryDataStore' (:156-164).
pub fn memory_data_store() -> DataStore {
    DataStore {
        with_storage: Arc::new(|_scfg, f| {
            let catalog = memory_catalog();
            let mem = new_memory_storage();
            let mut storages: HashMap<StorageName, Storage> = HashMap::new();
            storages.insert(memory_name(), Storage::Memory(mem));
            f(storages, CatalogStore::Memory(catalog))
        }),
        default_storage: (memory_name(), false),
        data_store_tag: "memory".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Config / DebugFlags (:166-258)
// ---------------------------------------------------------------------------

/// 'data Config' (:166-211). Generic over the ConfigProvider C because
/// cfgServerConfig/cfgSchemaHook hold ThriftSource<A, C> (DIVERGENCES 3).
pub struct Config<C> {
    pub cfg_data_store: DataStore,
    pub cfg_schema_location: Option<SchemaLocation>,
    /// 'ServerConfig.SchemaLocation -> (ThriftSource SchemaIndex, Bool)'
    /// (:169). The Bool is True if the schema for open DBs should be updated
    /// whenever the global schema changes.
    pub cfg_schema_hook:
        Arc<dyn Fn(&SchemaLocation) -> (ThriftSource<Arc<SchemaIndex>, C>, bool) + Send + Sync>,
    pub cfg_schema_id: Option<SchemaId>,
    pub cfg_server_config: ThriftSource<ServerConfig, C>,
    pub cfg_read_only: bool,
    pub cfg_mock_writes: bool,
    pub cfg_listener: Listener,
    /// 'forall a. Catalog -> Observed Config -> (SomeShardManager -> IO a) ->
    /// IO a' (:183-188): the erased-callback field (DIVERGENCES 2); call it
    /// through Config::with_shard_manager.
    pub cfg_shard_manager: Arc<
        dyn Fn(
                &Catalog,
                &Observed<ServerConfig>,
                &mut dyn FnMut(SomeShardManager) -> Result<(), ConfigError>,
            ) -> Result<(), ConfigError>
            + Send
            + Sync,
    >,
    /// 'Some GleanServerLogger' — inventory row 136's OPEN logger carve-out,
    /// so Box<dyn>. Send + Sync is the Rust thread-boundary constraint (the
    /// source class has none); Env shares the logger.
    pub cfg_server_logger: Box<dyn GleanServerLogger + Send + Sync>,
    pub cfg_database_logger: Box<dyn GleanDatabaseLogger + Send + Sync>,
    /// 'HashMap Text (Some Backup.Backend)' (:193) — the shipped closed-enum
    /// image is database/backup/backend.rs's Backends alias.
    pub cfg_backup_backends: Backends,
    pub cfg_batch_location_parser: SomeParser,
    pub cfg_enable_recursion: bool,
    /// '[Repo] -> IO [Repo]' (:199).
    pub cfg_filter_available_dbs:
        Arc<dyn Fn(&[Repo]) -> Result<Vec<Repo>, ConfigError> + Send + Sync>,
    /// '[Text] -> IO [Text]' (:201).
    pub cfg_acl_group_resolver:
        Arc<dyn Fn(&[String]) -> Result<Vec<String>, ConfigError> + Send + Sync>,
    /// 'IO Bool' (:205).
    pub cfg_acl_calculate_enabled: Arc<dyn Fn() -> Result<bool, ConfigError> + Send + Sync>,
    /// 'IO Bool' (:207).
    pub cfg_acl_check_enabled: Arc<dyn Fn() -> Result<bool, ConfigError> + Send + Sync>,
    pub cfg_tracer: Tracer<GleanTrace>,
    pub cfg_debug: DebugFlags,
}

impl<C> Config<C> {
    /// 'cfgShardManager cfg catalog serverConfig $ \shardManager -> ...'
    /// (:183-188), generic over the action's result. The action is fallible
    /// because the source's IO can throw; see DIVERGENCES 2.
    pub fn with_shard_manager<A, F>(
        &self,
        catalog: &Catalog,
        server_config: &Observed<ServerConfig>,
        f: F,
    ) -> Result<A, ConfigError>
    where
        F: FnOnce(SomeShardManager) -> Result<A, ConfigError>,
    {
        let mut out: Option<A> = None;
        let mut f = Some(f);
        (self.cfg_shard_manager)(catalog, server_config, &mut |shard_manager| {
            let f = f
                .take()
                .expect("Config::with_shard_manager: the action runs exactly once");
            out = Some(f(shard_manager)?);
            Ok(())
        })?;
        Ok(out.expect("Config::with_shard_manager: the action runs exactly once"))
    }
}

/// 'data DebugFlags' (:213-215).
pub struct DebugFlags {
    pub tc_debug: bool,
    pub query_debug: bool,
}

impl Default for DebugFlags {
    /// 'def = DebugFlags { tcDebug = False, queryDebug = False }' (:219).
    fn default() -> Self {
        DebugFlags {
            tc_debug: false,
            query_debug: false,
        }
    }
}

impl DebugFlags {
    /// 'a <> b' (:222-225): field-wise OR.
    pub fn combine(self, other: Self) -> Self {
        DebugFlags {
            tc_debug: self.tc_debug || other.tc_debug,
            query_debug: self.query_debug || other.query_debug,
        }
    }
}

// 'instance Monoid DebugFlags where mempty = def' (:227-228) is
// DebugFlags::default (RULE §2's Semigroup/Monoid mapping: combine + Default).

impl<C> fmt::Display for Config<C> {
    /// 'instance Show Config' (:230-234):
    ///   unwords ["Config {", "cfgDataStore: " <> tag,
    ///            "cfgServerConfig: " <> show src, "}"]
    /// i.e. "Config { cfgDataStore: <tag> cfgServerConfig: <src> }".
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Config {{ cfgDataStore: {} cfgServerConfig: {} }}",
            self.cfg_data_store.data_store_tag, self.cfg_server_config
        )
    }
}

impl<C> Default for Config<C>
where
    C: ConfigProvider + Clone + Send + Sync + 'static,
{
    /// 'instance Default Config' (:236-258).
    fn default() -> Self {
        Config {
            cfg_data_store: file_data_store("."),
            cfg_schema_location: None,
            // '\l -> (schemaLocationToSource l, True)' (:240).
            cfg_schema_hook: Arc::new(|loc: &SchemaLocation| {
                (schema_location_to_source(loc), true)
            }),
            cfg_schema_id: None,
            cfg_server_config: ThriftSource::default(),
            cfg_read_only: false,
            cfg_mock_writes: false,
            cfg_listener: Listener::default(),
            // '\_ _ k -> k $ SomeShardManager noSharding' (:246).
            cfg_shard_manager: Arc::new(|_catalog, _observed, k| {
                k(SomeShardManager::NoSharding(no_sharding()))
            }),
            cfg_server_logger: Box::new(NullGleanServerLogger),
            cfg_database_logger: Box::new(NullGleanDatabaseLogger),
            // 'HashMap.fromList [("mock", Backup.Mock.mock)]' (:249).
            cfg_backup_backends: HashMap::from([("mock".to_string(), mock())]),
            cfg_batch_location_parser: SomeParser::Default(DefaultParser),
            cfg_enable_recursion: false,
            // 'const $ return []' (:252).
            cfg_filter_available_dbs: Arc::new(|_repos| Ok(Vec::new())),
            // 'const (pure [])' (:253).
            cfg_acl_group_resolver: Arc::new(|_groups| Ok(Vec::new())),
            cfg_acl_calculate_enabled: Arc::new(|| Ok(acl_calculate_enabled())),
            cfg_acl_check_enabled: Arc::new(|| Ok(acl_check_enabled())),
            cfg_tracer: Tracer::default(),
            cfg_debug: DebugFlags::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// SchemaIndex / ProcessedSchema (:260-330)
// ---------------------------------------------------------------------------

/// 'data SchemaIndex' (:260-263). Both fields are exported by the source.
pub struct SchemaIndex {
    pub schema_index_current: ProcessedSchema,
    pub schema_index_older: Vec<ProcessedSchema>,
}

/// 'schemaForSchemaId' (:265-272): the current schema first, then the older
/// ones, matching on the hashed id. Returns a reference (the source shares
/// the value lazily; ProcessedSchema is not Clone).
pub fn schema_for_schema_id<'a>(
    index: &'a SchemaIndex,
    id: &SchemaId,
) -> Option<&'a ProcessedSchema> {
    std::iter::once(&index.schema_index_current)
        .chain(index.schema_index_older.iter())
        .find(|p| &p.proc_schema_hashed.hashed_schema_id == id)
}

/// 'data ProcessedSchema' (:282-286) — the schema in its three forms:
/// parsed source (stored back into DBs we create), resolved (needed for
/// "evolves"), and hashed (ComputeIds).
pub struct ProcessedSchema {
    pub proc_schema_source: SourceSchemas,
    pub proc_schema_resolved: ResolvedSchemas,
    pub proc_schema_hashed: HashedSchema,
}

/// 'processOneSchema' (:288-295).
pub fn process_one_schema(
    versions: Option<(SchemaId, Version)>,
    str_: &[u8],
) -> Result<SchemaIndex, String> {
    match process_schema(versions, str_) {
        Err(err) => Err(err),
        Ok(schema) => Ok(SchemaIndex {
            schema_index_current: schema,
            schema_index_older: Vec::new(),
        }),
    }
}

/// 'processSchema = processSchemaForTesting HashMap.toList' (:297-303).
pub fn process_schema(
    versions: Option<(SchemaId, Version)>,
    str_: &[u8],
) -> Result<ProcessedSchema, String> {
    process_schema_for_testing(versions, str_)
}

/// 'processSchemaForTesting' (:305-314). The 'toList' parameter is collapsed
/// (DIVERGENCES 5; compute_ids.rs:165-171).
pub fn process_schema_for_testing(
    versions: Option<(SchemaId, Version)>,
    str_: &[u8],
) -> Result<ProcessedSchema, String> {
    match parse_and_resolve_schema(str_) {
        Err(err) => Err(err),
        Ok((ss, r)) => {
            let hashed = compute_ids(&r.schemas_resolved, versions);
            Ok(ProcessedSchema {
                proc_schema_source: ss,
                proc_schema_resolved: r,
                proc_schema_hashed: hashed,
            })
        }
    }
}

/// 'processSchemaCached' (:316-330): returns the grown cache and the result.
pub fn process_schema_cached(
    versions: Option<(SchemaId, Version)>,
    cache: SchemaParserCache,
    str_: &[u8],
) -> Result<(SchemaParserCache, ProcessedSchema), String> {
    match parse_and_resolve_schema_cached(cache, str_) {
        Err(err) => Err(err),
        Ok((ss, r, new_cache)) => {
            let hashed = compute_ids(&r.schemas_resolved, versions);
            Ok((
                new_cache,
                ProcessedSchema {
                    proc_schema_source: ss,
                    proc_schema_resolved: r,
                    proc_schema_hashed: hashed,
                },
            ))
        }
    }
}

/// 'schemaLocationFiles = SchemaLocation_dir (Text.pack schemaSourceDir)'
/// (:332-333).
pub fn schema_location_files() -> SchemaLocation {
    SchemaLocation::Dir(SCHEMA_SOURCE_DIR.to_string())
}

// ---------------------------------------------------------------------------
// Schema sources (:336-388) — all private in the source
// ---------------------------------------------------------------------------

/// 'schemaSourceConfig' (:336-339). The payload is Arc<SchemaIndex>
/// (DIVERGENCES 4): row 81's constructors require A: Clone.
fn schema_source_config<C>(loc: String) -> ThriftSource<Arc<SchemaIndex>, C>
where
    C: ConfigProvider + Clone + Send + Sync + 'static,
{
    config_with_deserializer(
        loc,
        Arc::new(|bytes: &[u8]| process_one_schema(None, bytes).map(Arc::new)),
    )
}

/// 'schemaSourceFile' (:342-344).
fn schema_source_file<C>(f: String) -> ThriftSource<Arc<SchemaIndex>, C>
where
    C: ConfigProvider + Clone + Send + Sync + 'static,
{
    file_with_deserializer(
        f,
        Arc::new(|bytes: &[u8]| process_one_schema(None, bytes).map(Arc::new)),
    )
}

/// 'schemaSourceIndexFile = ThriftSource.once . parseSchemaIndex' (:347-348).
fn schema_source_index_file<C>(f: String) -> ThriftSource<Arc<SchemaIndex>, C>
where
    C: ConfigProvider + Clone + Send + Sync + 'static,
{
    once(Arc::new(move || {
        parse_schema_index(&f)
            .map(Arc::new)
            .map_err(|e| -> ConfigError { Box::new(e) })
    }))
}

/// 'schemaSourceFilesFromDir = ThriftSource.once . parseSchemaDir' (:351-352).
fn schema_source_files_from_dir<C>(dir: String) -> ThriftSource<Arc<SchemaIndex>, C>
where
    C: ConfigProvider + Clone + Send + Sync + 'static,
{
    once(Arc::new(move || {
        parse_schema_dir(&dir)
            .map(Arc::new)
            .map_err(|e| -> ConfigError { Box::new(e) })
    }))
}

/// 'schemaSourceIndexConfig' (:357-388): a schema index read from the
/// ConfigProvider. The index file is watched for changes; the instance files
/// it references are immutable and are read once per load.
fn schema_source_index_config<C>(key: String) -> ThriftSource<Arc<SchemaIndex>, C>
where
    C: ConfigProvider + Clone + Send + Sync + 'static,
{
    let instance_key_base = key.clone();
    generic_config(
        key,
        // deserializeJSON :: ThriftSerializable a => ByteString -> Either
        // String a (Thrift/Protocol/JSON.hs:49-50) -> serde on the generated
        // type (ASSUMED-IMPORT).
        Arc::new(|bytes: &[u8]| {
            serde_json::from_slice::<InternalSchemaIndex>(bytes).map_err(|e| e.to_string())
        }),
        Arc::new(move |cfg: &C, index: InternalSchemaIndex| {
            load_instances(cfg, &instance_key_base, index).map(Arc::new)
        }),
        None,
    )
}

/// The 'loadInstances' helper of schemaSourceIndexConfig (:369-388), with the
/// StateT SchemaParserCache threaded explicitly (RULE §2: State -> &mut).
fn load_instances<C>(
    cfg: &C,
    key: &str,
    index: InternalSchemaIndex,
) -> Result<SchemaIndex, ConfigError>
where
    C: ConfigProvider + Clone + Send + Sync + 'static,
{
    let mut cache: Option<SchemaParserCache> = Some(SchemaParserCache::new());
    // 'Config.get cfg instanceKey Right' (:377): Right is the identity
    // deserializer — the provider value is already the raw ByteString.
    let identity: Deserializer<Vec<u8>> = Arc::new(|bytes: &[u8]| Ok(bytes.to_vec()));

    let mut proc = |inst: &InternalSchemaInstance| -> Result<ProcessedSchema, ConfigError> {
        // 'takeDirectory (Text.unpack key) </> Text.unpack schemaInstance_file'
        // (:372-375).
        let instance_key = Path::new(&take_directory(key))
            .join(&inst.file)
            .to_string_lossy()
            .into_owned();
        let maybe_version = check_versions(&inst.versions)?;
        let str_ = cfg.get::<Vec<u8>>(&instance_key, &identity)?;
        let (new_cache, result) = process_schema_cached(
            maybe_version,
            cache
                .take()
                .expect("load_instances: the schema cache is threaded"),
            &str_,
        )
        .map_err(|err| -> ConfigError {
            Box::new(SchemaError::Exception(format!(
                "error in schema: {}\nFile: {}",
                err, instance_key
            )))
        })?;
        cache = Some(new_cache);
        Ok(result)
    };

    let current = proc(&index.current)?;
    let mut older = index
        .older
        .iter()
        .map(&mut proc)
        .collect::<Result<Vec<_>, ConfigError>>()?;
    // 'return (SchemaIndex current (reverse older))' (:387).
    older.reverse();
    Ok(SchemaIndex {
        schema_index_current: current,
        schema_index_older: older,
    })
}

/// 'checkVersions' (:390-396): the map must hold exactly one entry.
fn check_versions(
    versions: &BTreeMap<String, Version>,
) -> Result<Option<(SchemaId, Version)>, SchemaError> {
    match versions.len() {
        0 => Ok(None),
        1 => {
            let (txt_id, ver) = versions.iter().next().expect("check_versions: one entry");
            Ok(Some((txt_id.clone(), *ver)))
        }
        _ => Err(SchemaError::Exception(
            "versions must contain a single entry".to_string(),
        )),
    }
}

// ---------------------------------------------------------------------------
// Reading schema files (:398-431)
// ---------------------------------------------------------------------------

/// 'parseSchemaDir' (:398-403).
pub fn parse_schema_dir(dir: &str) -> Result<SchemaIndex, SchemaError> {
    let files = list_directory_recursive(dir)?;
    let str_ = cat_schema_files(&files)?;
    match process_one_schema(None, &str_) {
        // 'Left err -> throwIO $ ErrorCall err' (:401).
        Err(err) => Err(SchemaError::ErrorCall(err)),
        Ok(schema) => Ok(schema),
    }
}

/// 'parseSchemaIndex' (:405-418).
pub fn parse_schema_index(file: &str) -> Result<SchemaIndex, SchemaError> {
    // 'Internal.SchemaIndex{..} <- loadJSON file' (:407).
    let index: InternalSchemaIndex = load_json(file)?;
    let dir = take_directory(file);

    let proc = |inst: &InternalSchemaInstance| -> Result<ProcessedSchema, SchemaError> {
        // 'B.readFile (dir </> Text.unpack schemaInstance_file)' (:410) —
        // eager read (inventory.sites.tsv:441).
        let path = Path::new(&dir).join(&inst.file);
        let str_ = std::fs::read(&path)?;
        let maybe_version = check_versions(&inst.versions)?;
        // 'either (throwIO . ErrorCall) return $ processSchema maybeVersion str'
        // (:413).
        process_schema(maybe_version, &str_).map_err(SchemaError::ErrorCall)
    };

    let current = proc(&index.current)?;
    let older = index
        .older
        .iter()
        .map(proc)
        .collect::<Result<Vec<_>, SchemaError>>()?;
    // NOTE: parseSchemaIndex does NOT reverse older (:415-416), unlike
    // schemaSourceIndexConfig (:387).
    Ok(SchemaIndex {
        schema_index_current: current,
        schema_index_older: older,
    })
}

/// 'catSchemaFiles' (:420-428): sort, prepend every VERSION file, then every
/// .angle file with a "#FILE" annotation, and a "# @generated" marker.
pub fn cat_schema_files(files: &[String]) -> Result<Vec<u8>, std::io::Error> {
    let mut sorted: Vec<&String> = files.iter().collect();
    sorted.sort();

    let mut out: Vec<u8> = Vec::new();
    for file in &sorted {
        // '(=="VERSION") . takeFileName' (:423).
        if Path::new(file.as_str())
            .file_name()
            .map(|name| name == std::ffi::OsStr::new("VERSION"))
            .unwrap_or(false)
        {
            // Eager read (inventory.sites.tsv:442).
            out.extend_from_slice(&std::fs::read(file.as_str())?);
        }
    }

    out.extend_from_slice(b"# @generated\n");
    for file in &sorted {
        // '(== ".angle") . takeExtension' (:424): takeExtension includes the
        // dot; Rust's extension() does not.
        if Path::new(file.as_str())
            .extension()
            .map(|ext| ext == std::ffi::OsStr::new("angle"))
            .unwrap_or(false)
        {
            // Eager read (inventory.sites.tsv:443).
            let str_ = std::fs::read(file.as_str())?;
            out.extend_from_slice(b"#FILE ");
            // 'UTF8.fromString file' (:426): the UTF-8 bytes of the path.
            out.extend_from_slice(file.as_bytes());
            out.push(b'\n');
            out.extend_from_slice(&str_);
        }
    }
    Ok(out)
}

/// 'schemaSourceDir = "glean/schema/source"' (:430-431). A const rather than
/// a function because the value is a static string (DIVERGENCES 6).
pub const SCHEMA_SOURCE_DIR: &str = "glean/schema/source";

// ---------------------------------------------------------------------------
// Locations (:433-523)
// ---------------------------------------------------------------------------

/// 'schemaLocationToSource' (:433-450).
pub fn schema_location_to_source<C>(loc: &SchemaLocation) -> ThriftSource<Arc<SchemaIndex>, C>
where
    C: ConfigProvider + Clone + Send + Sync + 'static,
{
    match loc {
        SchemaLocation::Dir(d) => schema_source_files_from_dir(d.clone()),
        SchemaLocation::File(f) => schema_source_file(f.clone()),
        SchemaLocation::Index(i) => schema_source_index_file(i.clone()),
        SchemaLocation::Config(c) => schema_source_config(c.clone()),
        SchemaLocation::IndexConfig(c) => schema_source_index_config(c.clone()),
        // 'SchemaLocation_EMPTY{} -> error "schemaLocationToSource"' (:449).
        SchemaLocation::Empty => panic!("schemaLocationToSource"),
    }
}

/// 'schemaLocation' (:452-473): CLI flag, then ServerConfig, then the
/// default; "$datadir" is substituted in dir/file/index locations.
pub fn schema_location<C>(cfg: &Config<C>, server_cfg: &ServerConfig) -> SchemaLocation {
    // OSS branch (:454-455): 'getDataDir' from Paths_glean.
    let datadir = get_data_dir();
    let loc = cfg
        .cfg_schema_location
        .clone()
        .or_else(|| server_cfg.config_schema_location.clone())
        .unwrap_or_else(default_schema_location);
    match loc {
        SchemaLocation::Dir(d) => SchemaLocation::Dir(d.replace("$datadir", &datadir)),
        SchemaLocation::File(f) => SchemaLocation::File(f.replace("$datadir", &datadir)),
        SchemaLocation::Index(i) => SchemaLocation::Index(i.replace("$datadir", &datadir)),
        // 'other -> return other' (:472).
        other => other,
    }
}

/// 'loadSchemaIndex' (:475-480): load the ServerConfig, resolve the location,
/// then load through cfgSchemaHook's source.
pub fn load_schema_index<C>(cfg: &Config<C>, cfg_api: &C) -> Result<Arc<SchemaIndex>, ConfigError>
where
    C: ConfigProvider + Clone + Send + Sync + 'static,
{
    let server_config = cfg.cfg_server_config.load(cfg_api)?;
    let loc = schema_location(cfg, &server_config);
    let (source, _update) = (cfg.cfg_schema_hook)(&loc);
    source.load(cfg_api)
}

/// 'showSchemaLocation' (:482-492).
pub fn show_schema_location(loc: &SchemaLocation) -> String {
    match loc {
        SchemaLocation::Dir(d) => format!("dir:{}", d),
        SchemaLocation::File(f) => format!("file:{}", f),
        SchemaLocation::Index(i) => format!("index:{}", i),
        SchemaLocation::Config(c) => format!("config:{}", c),
        SchemaLocation::IndexConfig(c) => format!("indexconfig:{}", c),
        // '_ -> error "showSchemaLocation"' (:491).
        SchemaLocation::Empty => panic!("showSchemaLocation"),
    }
}

/// 'schemaLocationParser' (:494-516) — private in the source.
fn schema_location_parser(s: &str) -> Result<SchemaLocation, String> {
    // The two shorthand spellings (:497-500).
    if s == "indexconfig" {
        // SchemaLocation_index schemaConfigPath — Index, not IndexConfig.
        return Ok(SchemaLocation::Index(SCHEMA_CONFIG_PATH.to_string()));
    }
    if s == "dir" {
        return Ok(SchemaLocation::Dir(SCHEMA_SOURCE_DIR.to_string()));
    }
    // 'case break (==':') s of' (:501).
    match s.split_once(':') {
        Some(("dir", path)) => Ok(SchemaLocation::Dir(path.to_string())),
        Some(("index", path)) => Ok(SchemaLocation::Index(path.to_string())),
        Some(("indexconfig", path)) => Ok(SchemaLocation::IndexConfig(path.to_string())),
        Some(("file", path)) => Ok(SchemaLocation::File(path.to_string())),
        Some(("config", path)) => Ok(SchemaLocation::Config(path.to_string())),
        // '(_, "") -> Right (SchemaLocation_dir (Text.pack s))' (:514).
        None => Ok(SchemaLocation::Dir(s.to_string())),
        // '_otherwise -> Left "invalid schema location"' (:516).
        Some(_) => Err("invalid schema location".to_string()),
    }
}

/// 'schemaLocationOption' (:518-523): 'option (eitherReader
/// schemaLocationParser) (long "schema" <> metavar ...)'. Without an
/// optparse image the option collapses to its reader (DIVERGENCES 7); the
/// flag itself is handled by options().
pub fn schema_location_option(s: &str) -> Result<SchemaLocation, String> {
    schema_location_parser(s)
}

// ---------------------------------------------------------------------------
// options (:525-590)
// ---------------------------------------------------------------------------

/// 'options :: Parser Config' (:525-590), as a hand-written std-only parser
/// (DIVERGENCES 7). Accepts '--opt VALUE' and '--opt=VALUE'. Each option is
/// consumed at most once, as in optparse; repeats are errors, and a bare
/// '--' ends option parsing (there are no positional arguments).
// TODO(port): the optparse surface is not reproduced: --help/--version,
// usage/error rendering, exit codes, the 'internal' hiding of
// --experimental-recursion, and the metavar/help strings. Owner: the CLI
// front-end unit (glean-cli / the tools), which owns argument parsing for
// the binaries. clap is not on RULE §1's whitelist.
pub fn options<C>(args: &[String]) -> Result<Config<C>, String>
where
    C: ConfigProvider + Clone + Send + Sync + 'static,
{
    let mut db_root: Option<String> = None;
    let mut use_lmdb = false;
    let mut db_tmp = false;
    let mut db_memory = false;
    let mut schema_location: Option<SchemaLocation> = None;
    let mut server_config: Option<ThriftSource<ServerConfig, C>> = None;
    let mut tier: Option<String> = None;
    let mut read_only = false;
    let mut mock_writes = false;
    let mut enable_recursion = false;
    let mut tc_debug = false;
    let mut query_debug = false;

    let mut i = 0;
    while i < args.len() {
        let arg = args[i].clone();
        // optparse's '--' terminator: nothing after it is an option, and
        // 'options' accepts no positional arguments, so anything there is
        // an error.
        if arg == "--" {
            if i + 1 < args.len() {
                return Err(format!("Invalid argument '{}'", args[i + 1]));
            }
            break;
        }
        let (name, inline) = match arg.split_once('=') {
            Some((name, value)) => (name.to_string(), Some(value.to_string())),
            None => (arg.clone(), None),
        };
        let inline = inline.as_deref();
        match name.as_str() {
            // dbRoot (:528-536): --db-root DIR plus the --lmdb modifier.
            "--db-root" => {
                if db_root.is_some() {
                    return Err(format!("Invalid argument '{}' (repeated)", name));
                }
                db_root = Some(take_option_value(args, &mut i, inline, &name)?);
            }
            "--lmdb" => {
                reject_inline(inline, &name)?;
                use_lmdb = true;
            }
            // dbTmp / dbMem (:537-542).
            "--db-tmp" => {
                reject_inline(inline, &name)?;
                db_tmp = true;
            }
            "--db-memory" => {
                reject_inline(inline, &name)?;
                db_memory = true;
            }
            // optional schemaLocationOption (:544).
            "--schema" => {
                if schema_location.is_some() {
                    return Err(format!("Invalid argument '{}' (repeated)", name));
                }
                let value = take_option_value(args, &mut i, inline, &name)?;
                schema_location = Some(schema_location_parser(&value)?);
            }
            // _ignored_for_backwards_compat (:545): accepted, ignored.
            "--db-schema-override" => {
                reject_inline(inline, &name)?;
            }
            // serverConfigThriftSource (:581-583).
            "--server-config" => {
                let value = take_option_value(args, &mut i, inline, &name)?;
                server_config = Some(parse_thrift_source::<ServerConfig, C>(&value)?);
            }
            // serverConfigTier (:585-590): serverConfigPath </> TIER.
            "--tier" => {
                let value = take_option_value(args, &mut i, inline, &name)?;
                tier = Some(value);
            }
            // cfgReadOnly / cfgMockWrites (:551-552).
            "--db-read-only" => {
                reject_inline(inline, &name)?;
                read_only = true;
            }
            "--db-mock-writes" => {
                reject_inline(inline, &name)?;
                mock_writes = true;
            }
            // cfgEnableRecursion (:553-557).
            "--experimental-recursion" => {
                reject_inline(inline, &name)?;
                enable_recursion = true;
            }
            // debugParser (:575-579).
            "--debug-tc" => {
                reject_inline(inline, &name)?;
                tc_debug = true;
            }
            "--debug-query" => {
                reject_inline(inline, &name)?;
                query_debug = true;
            }
            _ => return Err(format!("Invalid argument '{}'", arg)),
        }
        i += 1;
    }

    // optparse consumes each option at most once and rejects whatever the
    // winning alternative left over. --lmdb lives INSIDE the dbRoot parser
    // (:528-536), so without --db-root it is unconsumed; --db-tmp/--db-memory
    // are alternatives to dbRoot (:543) and are unconsumed when it wins;
    // --tier is an alternative to --server-config (:547-550).
    if use_lmdb && db_root.is_none() {
        return Err("Invalid argument '--lmdb' (requires --db-root)".to_string());
    }
    if db_root.is_some() && (db_tmp || db_memory) {
        let flag = if db_tmp { "--db-tmp" } else { "--db-memory" };
        return Err(format!("Invalid argument '{}' (not with --db-root)", flag));
    }
    if server_config.is_some() && tier.is_some() {
        return Err("Invalid argument '--tier' (not with --server-config)".to_string());
    }

    // 'cfgDataStore <- dbRoot <|> dbTmp <|> dbMem <|> pure tmpDataStore'
    // (:543): the first present alternative wins.
    let data_store = match db_root {
        Some(path) => {
            let mut data_store = file_data_store(&path);
            if use_lmdb {
                // '\s -> s { defaultStorage = (lmdbName, True) }' (:535).
                data_store.default_storage = (lmdb_name(), true);
            }
            data_store
        }
        None if db_tmp => tmp_data_store(),
        None if db_memory => memory_data_store(),
        None => tmp_data_store(),
    };

    // 'serverConfigThriftSource <|> serverConfigTier <|> pure def' (:547-550).
    let cfg_server_config = match (server_config, tier) {
        (Some(source), _) => source,
        (None, Some(tier)) => config(format!("{}/{}", SERVER_CONFIG_PATH, tier)),
        (None, None) => ThriftSource::default(),
    };

    let defaults = Config::<C>::default();
    Ok(Config {
        cfg_data_store: data_store,
        cfg_schema_location: schema_location,
        // cfgSchemaHook = cfgSchemaHook def (:561).
        cfg_schema_hook: defaults.cfg_schema_hook,
        cfg_schema_id: None,
        cfg_server_config,
        cfg_read_only: read_only,
        cfg_mock_writes: mock_writes,
        // cfgListener = mempty (:560).
        cfg_listener: Listener::default(),
        // cfgShardManager = cfgShardManager def (:562).
        cfg_shard_manager: defaults.cfg_shard_manager,
        cfg_server_logger: defaults.cfg_server_logger,
        cfg_database_logger: defaults.cfg_database_logger,
        cfg_backup_backends: defaults.cfg_backup_backends,
        cfg_batch_location_parser: defaults.cfg_batch_location_parser,
        cfg_enable_recursion: enable_recursion,
        // cfgFilterAvailableDBs = const $ return [] (:567).
        cfg_filter_available_dbs: Arc::new(|_repos| Ok(Vec::new())),
        cfg_acl_group_resolver: defaults.cfg_acl_group_resolver,
        cfg_acl_calculate_enabled: defaults.cfg_acl_calculate_enabled,
        cfg_acl_check_enabled: defaults.cfg_acl_check_enabled,
        // cfgTracer = mempty (:571).
        cfg_tracer: Tracer::default(),
        cfg_debug: DebugFlags {
            tc_debug,
            query_debug,
        },
    })
}

/// 'strOption'/'option' value consumption for the hand-written parser: the
/// inline '=VALUE' form, else the next argv element.
fn take_option_value(
    args: &[String],
    i: &mut usize,
    inline: Option<&str>,
    flag: &str,
) -> Result<String, String> {
    match inline {
        Some(value) => Ok(value.to_string()),
        None => {
            *i += 1;
            args.get(*i)
                .cloned()
                .ok_or_else(|| format!("Missing argument for option '{}'", flag))
        }
    }
}

/// 'switch'/'flag'' reject an attached value, as optparse does.
fn reject_inline(inline: Option<&str>, flag: &str) -> Result<(), String> {
    match inline {
        Some(_) => Err(format!("Option '{}' does not take an argument", flag)),
        None => Ok(()),
    }
}

// ---------------------------------------------------------------------------
// Rust-only error and external-package images
// ---------------------------------------------------------------------------

/// The image of the exceptions parseSchemaDir/parseSchemaIndex throw (RULE §2
/// dynamic-exceptions row; DIVERGENCES 10). Display is constructor-shaped
/// where the source's Show is ('ThriftException'/'Exception') and bare for
/// 'ErrorCall'.
#[derive(Debug)]
pub enum SchemaError {
    /// 'throwIO $ ErrorCall err' (parseSchemaDir :401, parseSchemaIndex :413).
    ErrorCall(String),
    /// 'throwIO $ Exception ...' (schemaSourceIndexConfig :378) and
    /// checkVersions' "versions must contain a single entry" (:395).
    Exception(String),
    /// Thrift.Util.loadJSON's ThriftException: "[path] err" (:30-36).
    Thrift(String),
    /// B.readFile / loadJSON's IOException.
    Io(std::io::Error),
}

impl fmt::Display for SchemaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SchemaError::ErrorCall(msg) => f.write_str(msg),
            SchemaError::Exception(msg) => {
                write!(f, "Exception {{message = {:?}}}", msg)
            }
            SchemaError::Thrift(msg) => write!(f, "ThriftException {:?}", msg),
            SchemaError::Io(err) => fmt::Display::fmt(err, f),
        }
    }
}

impl From<std::io::Error> for SchemaError {
    fn from(err: std::io::Error) -> Self {
        SchemaError::Io(err)
    }
}

impl std::error::Error for SchemaError {}

/// 'loadJSON :: ThriftSerializable a => FilePath -> IO a'
/// (Thrift/Util.hs:30-36): strict read, then deserializeJSON; a decode
/// failure throws ThriftException "[path] err".
fn load_json<T: DeserializeOwned>(path: &str) -> Result<T, SchemaError> {
    let bytes = std::fs::read(path)?;
    serde_json::from_slice(&bytes).map_err(|err| SchemaError::Thrift(format!("[{}] {}", path, err)))
}

/// 'listDirectoryRecursive :: FilePath -> IO [FilePath]' — external
/// Util.IO (hsthrift/common/util/Util/IO.hs:368-382), NO manifest unit.
/// Directory entries are returned in filesystem order (the source does not
/// sort; catSchemaFiles does).
// TODO(port): no manifest unit owns Util.IO; this local image should move to
// a shared util module when Step 4 gives the hsthrift helpers a home. Owner:
// the Step-4 wiring survey (or the first unit that needs another Util.IO
// helper).
fn list_directory_recursive(dir: &str) -> std::io::Result<Vec<String>> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        let path = path.to_str().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "non-UTF-8 path")
        })?;
        // doesFileExist/doesDirectoryExist never throw: a dangling symlink
        // or an unreadable entry is SKIPPED, and doesFileExist means "not a
        // directory", so a FIFO counts as a file (Util/IO.hs:375-382).
        match std::fs::metadata(path) {
            Ok(metadata) if metadata.is_dir() => {
                out.extend(list_directory_recursive(path)?);
            }
            Ok(_) => out.push(path.to_string()),
            Err(_) => continue,
        }
    }
    Ok(out)
}

/// STUB — there is NO source counterpart to this function's body. The source
/// calls cabal's generated 'getDataDir' (Paths_glean, imported only under
/// 'ifdef OSS' at Config.hs:108-110 and used only at :455); that value is a
/// compiled-in constant, not an environment lookup. The GLEAN_DATADIR
/// override and the "." fallback below are invented so the port resolves
/// before Step 4; neither is a source surface.
// TODO(port): the compiled-in cabal datadir has no Rust image. Step 4's
// build wiring must supply it (a build.rs-generated env! is the expected
// shape) and delete this stub's env lookup. Owner: Step-4 build wiring.
fn get_data_dir() -> String {
    std::env::var("GLEAN_DATADIR").unwrap_or_else(|_| ".".to_string())
}

// ---------------------------------------------------------------------------
// Coverage / marker tally (closed set)
// ---------------------------------------------------------------------------
//
// Every export of Config.hs (:10-49) has a counterpart (EXPORT MAPPING above);
// the private helpers are private here. Nothing in the source is unported.
//
// Marker breakdown:
//   * SEVEN port todos:
//       1. the Storage enum forward reference (row 104 owns it);
//       2. the Send + Sync question on the withStorage payload — NOT enforced
//          by this file's signature; rows 71/104 own Memory's !Send posture;
//       3. the Tracer traceMsg_/ExitCase half and its eventual home;
//       4. getDataDir (cabal-generated, no unit; the current body is a stub);
//       5. listDirectoryRecursive (external Util.IO, no unit);
//       6. the optparse surface of options (help/usage/exit codes/internal);
//       7. the SchemaLocation owner conflict with default_configs.rs:92
//          (Step-4 crate wiring).
//   * Zero source-defect reproductions: the two 'error' calls keep their
//     exact messages as panics (RULE §2), which is a mapping, not a defect.
//   * Zero performance deferrals.
//   * Zero inventory divergences: rows 441-443 (eager reads), the two
//     strict-eager-reads group rows that repeat those sites, and row 125
//     (CPP deletion) are followed.
//   * Zero unsafe blocks (no FFI in this file).
//
// PORT STATUS: confidence=medium todos=7
