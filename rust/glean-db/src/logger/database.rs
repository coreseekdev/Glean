// Ported from: glean/db/Glean/Logger/Database.hs (45 lines).
// Manifest unit: rust/glean-db/src/logger/database.rs
// (migration/manifest.tsv line 74). RULE §4: one source file -> one target
// file; the manifest path is authoritative.
//
// WHAT THIS FILE IS: the DB-side logging vocabulary — a 14-constructor log
// ADT (:20-34) that is a right-nested cons list (`<>` appends, :36-37), the
// sink class that renders it (:17-18), and the null sink (:42-45). The OSS
// tree's only call site is Database/Logger.hs:98-99 (`preamble <> log` then
// `runLog`), and the only instance in-tree is the null one: real sinks are
// plugins outside the manifest, which is exactly why inventory.tsv:136
// carves the logger sinks out of the closed-enum rule (see INVENTORY).
//
// ---------------------------------------------------------------------------
// INVENTORY (RULE §5; migration/inventory*.tsv)
// ---------------------------------------------------------------------------
//
// ZERO rows name this source file by path:
//   $ grep -c 'Logger/Database' migration/inventory*.tsv
//   inventory.codeflow.tsv:0  inventory.ffi.tsv:0  inventory.protocol.tsv:0
//   inventory.sites.tsv:0     inventory.stm.tsv:0  inventory.tsv:0
// ONE construct-level row governs the class's existential AT ITS USE SITES
// (rows 85/104), not this file:
//   * inventory.tsv:136 ('Some c', MECHANICAL, confirmed): "closed enum at
//     each use site; Box<dyn Trait> only for truly-open sites — CARVE-OUT
//     granted 2026-10-05: logger sinks (GleanServerLogger/
//     GleanDatabaseLogger) are genuinely open (plugin sinks), dyn allowed"
//     (quoted through the carve-out sentence; the row continues with its
//     pilot-bug parenthetical).
//     Followed: the class becomes an object-safe trait with no supertraits,
//     so a `Box<dyn GleanDatabaseLogger>` is expressible; the object type
//     itself is spelled where the value is stored (SIBLING-UNIT CONTRACTS).
//
// ---------------------------------------------------------------------------
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, checked not assumed)
// ---------------------------------------------------------------------------
//
//   * OUTBOUND: none. `awk -F'\t' '$1=="glean/db/Glean/Logger/Database.hs"'
//     migration/depmap/edges.tsv` is empty. The only import is
//     `Data.Text (Text)` (:15) — the `text` package, not a manifest unit.
//   * INBOUND (3), none edited by this unit:
//       edges.tsv:227 Config.hs (manifest line 85, NOT shipped):
//         cfgDatabaseLogger :: Some GleanDatabaseLogger (Config.hs:191),
//         default Some NullGleanDatabaseLogger (Config.hs:248).
//       edges.tsv:315 Database/Logger.hs (manifest line 124, NOT shipped):
//         `Logger.runLog logger $ preamble <> log` (Logger.hs:98-99).
//       edges.tsv:455 Types.hs (manifest line 104, NOT shipped):
//         envDatabaseLogger :: Some GleanDatabaseLogger (Types.hs:247).
//     Three further grep-only consumers assign the field without naming the
//     type and are absent from the edge set: Env.hs:153, server/Glean/
//     Server.hs:96 (constructs the FB sink), tools/gleancli/plugin/
//     GleanCLI/Types.hs:77.
//
// ---------------------------------------------------------------------------
// ASSUMED IMPORTS (RULE §4 B4)
// ---------------------------------------------------------------------------
//
//   None. The port needs nothing beyond `std`.
//
// ---------------------------------------------------------------------------
// TRANSLATION NOTES
// ---------------------------------------------------------------------------
//
//   * `class GleanDatabaseLogger l where runLog :: l -> GleanDatabaseLog ->
//     IO ()` (:17-18) -> `pub trait GleanDatabaseLogger { fn run_log(&self,
//     log: GleanDatabaseLog); }`. Receiver `&self` (the class's `l ->`), the
//     log by value (the caller builds it and passes it once). `IO ()` -> `()`
//     (DIVERGENCES 1); the shape matches rows 45/46's infallible
//     `log_info(msg: impl FnOnce() -> String)`.
//   * `data GleanDatabaseLog` (:20-34) -> `pub enum GleanDatabaseLog` with
//     the constructor names VERBATIM (RULE §2's constructor-shaped naming).
//     `Text` -> String, `Int` -> i64, `Bool` -> bool (RULE §1).
//   * `GleanDatabaseLogAnd` (:22) is recursive; the Haskell cons cell is a
//     heap pointer, so the Rust variant boxes both halves:
//     `And(Box<GleanDatabaseLog>, Box<GleanDatabaseLog>)`. This is what makes
//     the log a finite tree of unbounded depth without an unsized enum.
//   * `instance Semigroup` (:36-37) -> `pub fn combine(self, other) -> Self`
//     building `And(Box::new(self), Box::new(other))` — the `ItemStatus`
//     precedent (filter.rs:294-298). `instance Monoid` (:39-40) ->
//     `impl Default` returning `GleanDatabaseLogEmpty`.
//   * `NullGleanDatabaseLogger` (:42-45) -> a unit struct whose `run_log`
//     drops the log. The source's `return ()` has no effect to keep.
//
// ---------------------------------------------------------------------------
// DIVERGENCES (deliberate)
// ---------------------------------------------------------------------------
//
//   1. `IO ()` -> `()`. A sink that fails PANICS here, where the Haskell
//      raises an IOException that propagates to logStats's caller. The
//      source declares no error channel and its only in-tree instance (the
//      Null sink) cannot fail; the log seam rows 45/46 established the same
//      infallible shape for `log_info`. A plugin sink that needs to report
//      a failure must choose its own channel (as a Haskell instance would
//      choose to throw).
//   2. Box indirection in `And` is forced by Rust's sized enums, not a
//      behaviour change: the Haskell cons cell is already a heap pointer.
//
// ---------------------------------------------------------------------------
// SIBLING-UNIT CONTRACTS
// ---------------------------------------------------------------------------
//
//   * database/config.rs (row 85) and database/types.rs (row 104): the
//     `cfg_database_logger`/`env_database_logger` fields hold the open sink.
//     Per inventory.tsv:136 they are `Box<dyn GleanDatabaseLogger>`; if the
//     owning struct must be Send/Sync, spell the object type
//     `Box<dyn GleanDatabaseLogger + Send + Sync>` THERE — this trait
//     deliberately declares no supertrait, so a plugin sink is not forced to
//     be thread-safe by the vocabulary.
//   * database/logger.rs (row 124): builds `preamble.combine(log)` and calls
//     `run_log`; `Default` supplies the mempty preamble.
//   * logger/server.rs (row 75): the server-side twin, same shape.
//   * util/log.rs (rows 45/46): the infallible logging signature this trait
//     follows.
//
// MARKERS: zero of every class — 0 port todos, 0 source-defect
// reproductions, 0 performance deferrals, 0 inventory divergences, 0 SAFETY
// notes (no unsafe code).

/// 'class GleanDatabaseLogger l where runLog :: l -> GleanDatabaseLog ->
/// IO ()' (:17-18). Object-safe on purpose: inventory.tsv:136's carve-out
/// makes the logger sinks a genuinely open set (plugin sinks), so a
/// `Box<dyn GleanDatabaseLogger>` must be expressible.
pub trait GleanDatabaseLogger {
    /// 'runLog' (:18): render one (possibly combined) log record.
    fn run_log(&self, log: GleanDatabaseLog);
}

/// 'data GleanDatabaseLog' (:20-34) — the log record. `And` is the cons
/// cell built by `combine`; the rest are the individual fields a sink
/// renders.
pub enum GleanDatabaseLog {
    /// 'GleanDatabaseLogEmpty' (:21) — 'mempty' (:40).
    GleanDatabaseLogEmpty,
    /// 'GleanDatabaseLogAnd GleanDatabaseLog GleanDatabaseLog' (:22):
    /// 'a <> b' (:37). Both halves are boxed because the enum is sized.
    GleanDatabaseLogAnd(Box<GleanDatabaseLog>, Box<GleanDatabaseLog>),
    /// 'SetRepoName Text' (:23).
    SetRepoName(String),
    /// 'SetRepoHash Text' (:24).
    SetRepoHash(String),
    /// 'SetPredicateCount Int' (:25).
    SetPredicateCount(i64),
    /// 'SetUploadDestination Text' (:26).
    SetUploadDestination(String),
    /// 'SetPredicateName Text' (:27).
    SetPredicateName(String),
    /// 'SetPredicateVersion Int' (:28).
    SetPredicateVersion(i64),
    /// 'SetPredicateSize Int' (:29).
    SetPredicateSize(i64),
    /// 'SetMetric Text' (:30).
    SetMetric(String),
    /// 'SetCount Int' (:31).
    SetCount(i64),
    /// 'SetSize Int' (:32).
    SetSize(i64),
    /// 'SetHasExcludeProperty Bool' (:33).
    SetHasExcludeProperty(bool),
    /// 'SetDatabaseProperties Text' (:34).
    SetDatabaseProperties(String),
}

impl GleanDatabaseLog {
    /// 'instance Semigroup GleanDatabaseLog where a <> b =
    /// GleanDatabaseLogAnd a b' (:36-37).
    pub fn combine(self, other: Self) -> Self {
        GleanDatabaseLog::GleanDatabaseLogAnd(Box::new(self), Box::new(other))
    }
}

impl Default for GleanDatabaseLog {
    /// 'instance Monoid GleanDatabaseLog where mempty =
    /// GleanDatabaseLogEmpty' (:39-40).
    fn default() -> Self {
        GleanDatabaseLog::GleanDatabaseLogEmpty
    }
}

/// 'data NullGleanDatabaseLogger = NullGleanDatabaseLogger' (:42) — the
/// sink the OSS defaults install (Config.hs:248).
pub struct NullGleanDatabaseLogger;

impl GleanDatabaseLogger for NullGleanDatabaseLogger {
    /// 'runLog _ _ = return ()' (:45): drop the record.
    fn run_log(&self, _log: GleanDatabaseLog) {}
}

// PORT STATUS: confidence=high todos=0
