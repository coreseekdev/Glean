// Ported from: glean/util/Glean/Util/ShardManager.hs (95 lines).
// Manifest unit: rust/glean-util/src/util/shard_manager.rs
// (migration/manifest.tsv line 83). RULE §4: one source file -> one target
// file; the manifest path is authoritative.
//
// WHAT THIS FILE IS: how Glean DBs are distributed over servers. A
// `ShardManager shard` answers three questions — which shards this server
// owns (`getAssignedShards`), which shard a DB stack belongs to
// (`computeShardMapping`, keyed on the BASE of the stack so every db in a
// stack lands on one shard — Note [DB to Shard], :56-69), and how shard
// sizes are bucketed for stats (`countersForShardSizes`). Three strategies
// ship: no sharding, by repo name, by repo hash (of the repo or of the
// stack base). Six modules import it; it imports only Glean.Types.
//
// ---------------------------------------------------------------------------
// INVENTORY (RULE §5; migration/inventory*.tsv)
// ---------------------------------------------------------------------------
//
// NONE. grep for "ShardManager" over migration/inventory*.tsv returns zero
// rows: no IO-heavy surface, no FFI, no STM group. The one non-mechanical
// decision is `SomeShardManager` (see DIVERGENCES 1), recorded here rather
// than in the inventory.
//
// ---------------------------------------------------------------------------
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, checked not assumed)
// ---------------------------------------------------------------------------
//
//   * OUTBOUND: none in the depmap. The one non-base import, `Glean.Types`
//     (:22, used as `Glean.Repo`), is the generated thrift module — an
//     assumed import (ASSUMED IMPORTS).
//   * INBOUND (6), none edited by this unit: db/Database/{Config,Env,
//     Janitor,Retention,Types}.hs and server/Server/Sharding.hs.
//
// ---------------------------------------------------------------------------
// ASSUMED IMPORTS (RULE §4 B4)
// ---------------------------------------------------------------------------
//
//   * `glean_thrift::Repo` — the generated Glean.Repo (glean/if/
//     glean.thrift), already assumed by rows 47/48/60/73. Only two field
//     reads are used: `repo_name` (:77) and `repo_hash` (:86, :94), both
//     Text -> String.
//
// ---------------------------------------------------------------------------
// TRANSLATION NOTES
// ---------------------------------------------------------------------------
//
//   * `getAssignedShards :: IO (Maybe [shard])` (:32) stays an ACTION: every
//     constructor takes the caller's action verbatim (:76, :85, :93). It
//     becomes a fallible closure (`Result`, since the IO can throw).
//   * `computeShardMapping :: IO (BaseOfStack -> Repo -> shard)` (:37) LOSES
//     its IO: all three constructors write `pure $ \… -> …` (:48, :77, :86,
//     :94), so the action never fails and never runs IO. The closure is
//     stored directly (DIVERGENCES 2).
//   * `countersForShardSizes` (:38) is a PURE function in the source;
//     `const mempty` (:49), `const []` (:78) and `pure []` (:87, :95) are
//     all the constant function (in the function applicative `pure []` is
//     `\_ -> []`). Stored as a plain closure (DIVERGENCES 3).
//   * Note [DB to Shard] (:56-69) is carried verbatim as a doc comment on
//     `compute_shard_mapping`: every db in a stack must map to one shard,
//     which is why the function takes the stack base.
//
// ---------------------------------------------------------------------------
// DIVERGENCES (deliberate)
// ---------------------------------------------------------------------------
//
//   1. `SomeShardManager` is a GADT existential —
//        SomeShardManager :: (Ord shard, Show shard) => ShardManager shard
//                          -> SomeShardManager
//      (:52-54). RULE §2's existential row takes a CLOSED enum, and the
//      row-136 `Box<dyn>` carve-out is logger-only, so the image is
//        enum SomeShardManager { NoSharding(ShardManager<NoSharding>),
//                                Text(ShardManager<String>) }
//      — the two shard types every in-tree producer uses: Sharding.hs:91
//      and Config.hs:246 wrap `noSharding`, Sharding.hs:95/:115/:133 wrap a
//      `ShardManager Text`. The set is closed for the TREE but not for the
//      language: a new shard type adds a variant here (and the consumer
//      match gains an arm), which is the price of the rulebook's shape.
//      Consumers unpack it exactly as Janitor.hs:122 does, then run generic
//      code under `ShardKey` (DIVERGENCES 6).
//   2. `computeShardMapping`'s IO wrapper is dropped (TRANSLATION NOTES).
//      A future constructor that needs a failing action would reintroduce
//      it here rather than at the call sites.
//   3. `countersForShardSizes`'s `pure []` is the constant function, not an
//      IO action — the field is pure in the source and stays pure.
//   4. `Int` -> `i64` (the count in the returned pairs), `Text` -> `String`
//      (RULE §1 strings row), `Map` -> `BTreeMap` (ordered iteration; the
//      source's Data.Map is ordered too, so bucketing order is preserved).
//   5. `NoSharding`'s `deriving (Eq, Ord, Show)` (:42) becomes
//      `PartialEq/Eq/PartialOrd/Ord/Display` with the constructor name as
//      the Display text. The type is `pub` where the source omits it from
//      the export list, because it appears in `no_sharding`'s return type
//      (a private type in a public signature is a lint).
//   6. `ShardKey` is the Rust image of the GADT's `(Ord shard, Show shard)`
//      constraint: `Clone + Ord + Display + Send + Sync + 'static`. It
//      exists so a consumer's generic helper (Janitor's `runWithShards`,
//      :135) can state the same bounds once. Only `NoSharding` and `String`
//      implement it here; a new shard type implements it at the variant.
//   7. `noSharding` (:45) is a FUNCTION, not a const: the record owns Arc
//      closures, which cannot be built in a const.
//
// ---------------------------------------------------------------------------
// SIBLING-UNIT CONTRACTS
// ---------------------------------------------------------------------------
//
//   * row 85 (db/Database/Config.hs) stores `cfgShardManager :: (… ->
//     (SomeShardManager -> IO a) -> IO a)` (:187) and wraps `noSharding`
//     (:246) — it owns the bracket that decides WHICH manager to build.
//   * row 104 (db/Database/Types.hs:289) stores `envShardManager :
//     SomeShardManager`; the Env type must keep it by value/Arc, not by
//     borrow (the Janitor thread holds it).
//   * Janitor.hs:122-143 unpacks the existential and needs `ShardKey`'s
//     three bounds; Retention.hs:70/:91 calls the mapping with
//     `BaseOfStack (last stack)` — the stack-base invariant.
//   * server/Server/Sharding.hs is the only module that builds a
//     non-trivial manager in-tree (:95 shardByRepo, :115/:133 the
//     Impl.ShardManager strategies) — row 148/Server's business.
//
// MARKERS: zero port todos, zero BUG(port), zero PERF(port), zero
// INVENTORY-BUG-CANDIDATE, zero SAFETY notes (no unsafe in this file).

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use glean_thrift::Repo;

use super::config_provider::ConfigError;

/// 'newtype BaseOfStack = BaseOfStack Glean.Repo' (:25) — the base db of a
/// stack (or a single db).
#[derive(Clone)]
pub struct BaseOfStack(pub Repo);

/// The image of the GADT's '(Ord shard, Show shard)' constraint
/// (DIVERGENCES 6).
pub trait ShardKey: Clone + Ord + fmt::Display + Send + Sync + 'static {}

/// 'data NoSharding = NoSharding deriving (Eq, Ord, Show)' (:41-42).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct NoSharding;

impl fmt::Display for NoSharding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("NoSharding")
    }
}

impl ShardKey for NoSharding {}
impl ShardKey for String {}

/// 'data ShardManager shard' (:28-39) — an abstraction for sharding Glean
/// DBs across multiple servers.
pub struct ShardManager<S> {
    /// 'getAssignedShards :: IO (Maybe [shard])' (:32): the shards for the
    /// current server; Nothing means fall back to no sharding.
    pub get_assigned_shards: Arc<dyn Fn() -> Result<Option<Vec<S>>, ConfigError> + Send + Sync>,
    /// 'computeShardMapping :: IO (BaseOfStack -> Glean.Repo -> shard)'
    /// (:37). Note [DB to Shard]: the mapping takes the BASE of the stack so
    /// every db in a stack lands on one shard.
    pub compute_shard_mapping: Arc<dyn Fn(&BaseOfStack, &Repo) -> S + Send + Sync>,
    /// 'countersForShardSizes :: Map shard Int64 -> [(Text, Int)]' (:38).
    pub counters_for_shard_sizes:
        Arc<dyn Fn(&BTreeMap<S, i64>) -> Vec<(String, i64)> + Send + Sync>,
}

impl<S> Clone for ShardManager<S> {
    fn clone(&self) -> Self {
        ShardManager {
            get_assigned_shards: self.get_assigned_shards.clone(),
            compute_shard_mapping: self.compute_shard_mapping.clone(),
            counters_for_shard_sizes: self.counters_for_shard_sizes.clone(),
        }
    }
}

/// 'noSharding :: ShardManager NoSharding' (:45-49): a single shard and a
/// trivial assignment (DIVERGENCES 7).
pub fn no_sharding() -> ShardManager<NoSharding> {
    ShardManager {
        get_assigned_shards: Arc::new(|| Ok(Some(vec![NoSharding]))),
        compute_shard_mapping: Arc::new(|_base: &BaseOfStack, _repo: &Repo| NoSharding),
        counters_for_shard_sizes: Arc::new(|_sizes: &BTreeMap<NoSharding, i64>| Vec::new()),
    }
}

/// 'data SomeShardManager' (:52-54) — an existential wrapper around a
/// ShardManager. See DIVERGENCES 1 for the closed-enum image.
#[derive(Clone)]
pub enum SomeShardManager {
    /// The 'noSharding' strategy.
    NoSharding(ShardManager<NoSharding>),
    /// Any 'shardBy*' strategy (the source's shard type is Text).
    Text(ShardManager<String>),
}

/// 'shardByRepo :: IO (Maybe [Text]) -> ShardManager Text' (:73-79): shard
/// by repo name using the provided dynamic shard assignment.
pub fn shard_by_repo(
    get_assigned_shards: Arc<dyn Fn() -> Result<Option<Vec<String>>, ConfigError> + Send + Sync>,
) -> ShardManager<String> {
    ShardManager {
        get_assigned_shards,
        compute_shard_mapping: Arc::new(|base: &BaseOfStack, _repo: &Repo| {
            base.0.repo_name.clone()
        }),
        counters_for_shard_sizes: Arc::new(|_sizes: &BTreeMap<String, i64>| Vec::new()),
    }
}

/// 'shardByRepoHash :: IO (Maybe [Text]) -> ShardManager Text' (:83-87):
/// shards are repo hashes.
pub fn shard_by_repo_hash(
    get_assigned_shards: Arc<dyn Fn() -> Result<Option<Vec<String>>, ConfigError> + Send + Sync>,
) -> ShardManager<String> {
    ShardManager {
        get_assigned_shards,
        compute_shard_mapping: Arc::new(|_base: &BaseOfStack, repo: &Repo| repo.repo_hash.clone()),
        counters_for_shard_sizes: Arc::new(|_sizes: &BTreeMap<String, i64>| Vec::new()),
    }
}

/// 'shardByBaseOfStackRepoHash :: IO (Maybe [Text]) -> ShardManager Text'
/// (:91-95): shards are the BASE repo's hash.
pub fn shard_by_base_of_stack_repo_hash(
    get_assigned_shards: Arc<dyn Fn() -> Result<Option<Vec<String>>, ConfigError> + Send + Sync>,
) -> ShardManager<String> {
    ShardManager {
        get_assigned_shards,
        compute_shard_mapping: Arc::new(|base: &BaseOfStack, _repo: &Repo| {
            base.0.repo_hash.clone()
        }),
        counters_for_shard_sizes: Arc::new(|_sizes: &BTreeMap<String, i64>| Vec::new()),
    }
}

// PORT STATUS: confidence=high todos=0
