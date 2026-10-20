// Ported from: glean/github/Glean/DefaultConfigs.hs (36 lines).
// Manifest unit: rust/glean-db/src/default_configs.rs
// (migration/manifest.tsv line 82). RULE §4: one source file -> one target
// file; the manifest path is authoritative.
//
// WHAT THIS FILE IS: four constants of the OSS default configuration — the
// default schema location (a directory under the Cabal datadir), the
// ThriftSource for ~/.config/glean/client, and the two config paths
// ("server", "schema-index"). Three modules import it; it imports
// ThriftSource (row 81) plus the two generated thrift type modules.
//
// ---------------------------------------------------------------------------
// INVENTORY (RULE §5; migration/inventory*.tsv)
// ---------------------------------------------------------------------------
//
// NONE. grep for "DefaultConfigs" over migration/inventory*.tsv returns
// zero rows: the file is four pure values with no IO, FFI, STM, protocol or
// exception surface. RULE §5's mechanical row applies (a 1:1 transcription
// of literals).
//
// ---------------------------------------------------------------------------
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, checked not assumed)
// ---------------------------------------------------------------------------
//
//   * OUTBOUND (1): edges.tsv:738 ThriftSource.hs — SHIPPED (row 81,
//     commit 829909d). Only `configDefault` (:28) is used.
//   * OUTBOUND, outside the depmap because generated:
//       Glean.ClientConfig.Types (:19) and Glean.ServerConfig.Types (:20) —
//       the gen-hs2 output for glean/config/client/client_config.thrift:35
//       and glean/config/server/server_config.thrift:170. Assumed Rust
//       images: `glean_client_config::ClientConfig` and
//       `glean_server_config::SchemaLocation` (see ASSUMED IMPORTS).
//   * INBOUND (3), none edited by this unit: client/Remote.hs:100,
//     client/local/LocalOrRemote.hs:134, db/Database/Config.hs:230.
//
// ---------------------------------------------------------------------------
// ASSUMED IMPORTS (RULE §4 B4)
// ---------------------------------------------------------------------------
//
//   * `glean_util::util::thrift_source` — the in-repo crate edge to row 81
//     (the glean-db Cargo.toml gains it at Step 4; no crate in rust/
//     compiles yet). The `util::` level is real: glean-util's crate root
//     declares `pub mod util;` and src/util.rs declares the modules
//     (backend/types.rs:363 imports DbShard the same way).
//   * `glean_client_config::ClientConfig` — generated from
//     client_config.thrift:35-50 (serv/use_shards/min_db_age/
//     host_timeout_ms/max_batch_size). The value type of
//     defaultClientConfigSource; it must satisfy row 81's bounds
//     (DeserializeOwned + Default + Clone + Send + Sync + 'static), which
//     the generated serde type does.
//   * `glean_server_config::SchemaLocation` — generated from
//     server_config.thrift:170-185, a union of four strings
//     (dir/file/index/config). ASSUMED VARIANT SPELLING:
//     `SchemaLocation::Dir(String)` (Rust's PascalCase enum convention;
//     the Haskell constructor is SchemaLocation_dir). Step 4's generator
//     owns the final name — if it differs, only this one expression
//     changes.
//
// ---------------------------------------------------------------------------
// DIVERGENCES (deliberate)
// ---------------------------------------------------------------------------
//
//   1. serverConfigPath/schemaConfigPath are `pub const &'static str`
//      rather than functions or `String` values: the source's top-level
//      bindings are CAFs with no allocation, and every consumer either
//      compares or converts (ConfigPath is String). A caller that needs an
//      owned value writes .to_string().
//   2. defaultSchemaLocation is a FUNCTION, not a const: SchemaLocation
//      owns a String (the generated union), which cannot be built in a
//      const. The source's CAF is evaluated once; the Rust function
//      allocates on each call. Callers should hoist it (all three in-tree
//      call sites build it once at startup).
//   3. defaultClientConfigSource is generic over the provider type C
//      (row 81's second parameter): the source's `ThriftSource ClientConfig`
//      is rank-2 over cfg, and the concrete provider (the client's
//      ConfigAPI in practice) is chosen at the use site.
//
// ---------------------------------------------------------------------------
// SIBLING-UNIT CONTRACTS
// ---------------------------------------------------------------------------
//
//   * row 81 (ThriftSource) owns configDefault's fallback semantics; this
//     file only supplies the path "client".
//   * rows 85 (db/Database/Config.hs), 156 (Impl/ConfigProvider.hs) and the
//     client's Remote/LocalOrRemote pick C. The client side's C is the
//     ConfigAPI of row 156; the db side's C is row 85's Config.
//
// MARKERS: zero port todos, zero BUG(port), zero PERF(port), zero
// INVENTORY-BUG-CANDIDATE, zero SAFETY notes (no unsafe in this file).

use glean_client_config::ClientConfig;
use glean_server_config::SchemaLocation;
use glean_util::util::config_provider::ConfigProvider;
use glean_util::util::thrift_source::{config_default, ThriftSource};

/// 'defaultSchemaLocation :: SchemaLocation' (:23-24) —
/// 'SchemaLocation_dir "$datadir/glean/schema/source"'.
pub fn default_schema_location() -> SchemaLocation {
    SchemaLocation::Dir("$datadir/glean/schema/source".to_string())
}

/// 'defaultClientConfigSource :: ThriftSource ClientConfig' (:27-28) —
/// 'configDefault "client"'.
pub fn default_client_config_source<C>() -> ThriftSource<ClientConfig, C>
where
    C: ConfigProvider + Clone + Send + Sync + 'static,
{
    config_default("client".to_string())
}

/// 'serverConfigPath :: String' (:31-32): the config path under
/// ~/.config/glean where the server config lives.
pub const SERVER_CONFIG_PATH: &str = "server";

/// 'schemaConfigPath :: Text' (:35-36): the config path under
/// ~/.config/glean used by '--schema indexconfig'.
pub const SCHEMA_CONFIG_PATH: &str = "schema-index";

// PORT STATUS: confidence=high todos=0
