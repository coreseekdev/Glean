// Ported from: glean/db/Glean/Logger/Server.hs (72 lines).
// Manifest unit: rust/glean-db/src/logger/server.rs
// (migration/manifest.tsv line 75). RULE §4: one source file -> one target
// file; the manifest path is authoritative.
//
// WHAT THIS FILE IS: the server-side logging vocabulary — the row 74 twin.
// A 41-constructor log ADT (:20-61) that is a right-nested cons list, the
// sink class (:17-18), and the null sink (:69-72). Unlike the DB-side log,
// this one has real OSS producers: Backend/Logging.hs:154-322 builds the
// records for every backend call (row 148), Database/Logger.hs:37-50
// appends SetMethod/SetWeight/SetRepoName/SetRepoHash (row 124), and
// Logger.hs:28-34's `ActionLog GleanServerLog` instance maps the
// success/failure/time/alloc hooks onto SetSuccess/SetError/SetTimeElapsed/
// SetAllocatedBytes (row 106). The sinks themselves are plugins, so the
// same inventory.tsv:136 carve-out applies.
//
// ---------------------------------------------------------------------------
// INVENTORY (RULE §5; migration/inventory*.tsv)
// ---------------------------------------------------------------------------
//
// ZERO rows name this source file by path:
//   $ grep -c 'Logger/Server' migration/inventory*.tsv
//   inventory.codeflow.tsv:0  inventory.ffi.tsv:0  inventory.protocol.tsv:0
//   inventory.sites.tsv:0     inventory.stm.tsv:0  inventory.tsv:0
// The same construct-level row as row 74 governs the class's existential AT
// ITS USE SITES (rows 85/104):
//   * inventory.tsv:136 ('Some c', MECHANICAL, confirmed): "closed enum at
//     each use site; Box<dyn Trait> only for truly-open sites — CARVE-OUT
//     granted 2026-10-05: logger sinks (GleanServerLogger/
//     GleanDatabaseLogger) are genuinely open (plugin sinks), dyn allowed".
//     Followed exactly as in logger/database.rs.
//
// ---------------------------------------------------------------------------
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, checked not assumed)
// ---------------------------------------------------------------------------
//
//   * OUTBOUND: none. `awk -F'\t' '$1=="glean/db/Glean/Logger/Server.hs"'
//     migration/depmap/edges.tsv` is empty. The only import is
//     `Data.Text (Text)` (:15).
//   * INBOUND (4), none edited by this unit:
//       edges.tsv:58 Backend/Logging.hs (manifest line 148, NOT shipped)
//         builds every record (Logging.hs:154-322).
//       edges.tsv:228 Config.hs (manifest line 85, NOT shipped):
//         cfgServerLogger :: Some GleanServerLogger (Config.hs:189),
//         default Some NullGleanServerLogger (Config.hs:247).
//       edges.tsv:456 Types.hs (manifest line 104, NOT shipped):
//         envServerLogger :: Some GleanServerLogger (Types.hs:246).
//       edges.tsv:523 Logger.hs (manifest line 106, NOT shipped): the
//         ActionLog instance (Logger.hs:28-34) and runLogCmd/runLogRepo
//         (Logger.hs:37-50).
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
//   * `class GleanServerLogger l where runLog :: l -> GleanServerLog ->
//     IO ()` (:17-18) -> an object-safe trait, receiver `&self`, the log by
//     value; `IO ()` -> `()` (DIVERGENCES 1, the same call as row 74).
//   * `data GleanServerLog` (:20-61) -> `pub enum GleanServerLog`, the
//     constructor names VERBATIM (RULE §2). `Text` -> String, `Int` -> i64,
//     `Bool` -> bool, `Double` -> f64 (RULE §1), `[Text]` -> Vec<String>
//     (SetFullScans, :44 — the only list-valued field).
//   * `GleanServerLogAnd` (:22) boxes both halves, as in row 74: the
//     Haskell cons cell is a heap pointer and the Rust enum must be sized.
//   * Semigroup (:63-64) -> `combine`; Monoid (:66-67) -> `impl Default`.
//   * `NullGleanServerLogger` (:69-72) -> a unit struct that drops the log.
//   * `Double` has no special encoding here: the value is only carried
//     (SetTimeElapsed, :27), never rendered, so f64 is exact for every
//     Double the source can hold.
//
// ---------------------------------------------------------------------------
// DIVERGENCES (deliberate)
// ---------------------------------------------------------------------------
//
//   1. `IO ()` -> `()`. A sink that fails PANICS here, where the Haskell
//      raises an IOException that propagates to runLogCmd's caller. The
//      source declares no error channel and its only in-tree instance (the
//      Null sink) cannot fail; rows 45/46's `log_info` set the same
//      infallible precedent (identical to logger/database.rs's divergence 1).
//   2. Box indirection in `And` is forced by Rust's sized enums, not a
//      behaviour change.
//
// ---------------------------------------------------------------------------
// SIBLING-UNIT CONTRACTS
// ---------------------------------------------------------------------------
//
//   * database/config.rs (row 85) and database/types.rs (row 104): the
//     `cfg_server_logger`/`env_server_logger` fields hold the open sink as
//     `Box<dyn GleanServerLogger>` (inventory.tsv:136); add `+ Send + Sync`
//     at the field if the owner must be thread-safe — this trait declares no
//     supertrait, exactly like row 74's.
//   * logger.rs (row 106): `ActionLog for GleanServerLog` (Logger.hs:28-34)
//     maps success/failure/time/alloc onto SetSuccess/SetError/
//     SetTimeElapsed/SetAllocatedBytes; runLogCmd/runLogRepo (Logger.hs:37-50)
//     append SetMethod/SetWeight/SetRepoName/SetRepoHash and call run_log.
//   * backend/logging.rs (row 148): the producer of every other constructor.
//   * logger/database.rs (row 74): the DB-side twin; same shape, same
//     carve-out, same divergences.
//
// MARKERS: zero of every class — 0 port todos, 0 source-defect
// reproductions, 0 performance deferrals, 0 inventory divergences, 0 SAFETY
// notes (no unsafe code).

/// 'class GleanServerLogger l where runLog :: l -> GleanServerLog ->
/// IO ()' (:17-18). Object-safe on purpose (inventory.tsv:136's carve-out).
pub trait GleanServerLogger {
    /// 'runLog' (:18): render one (possibly combined) log record.
    fn run_log(&self, log: GleanServerLog);
}

/// 'data GleanServerLog' (:20-61) — the server request record.
pub enum GleanServerLog {
    /// 'GleanServerLogEmpty' (:21) — 'mempty' (:67).
    GleanServerLogEmpty,
    /// 'GleanServerLogAnd GleanServerLog GleanServerLog' (:22): 'a <> b'
    /// (:64). Both halves are boxed because the enum is sized.
    GleanServerLogAnd(Box<GleanServerLog>, Box<GleanServerLog>),
    /// 'SetSuccess Bool' (:23).
    SetSuccess(bool),
    /// 'SetError Text' (:24).
    SetError(String),
    /// 'SetAuthStatus Text' (:25).
    SetAuthStatus(String),
    /// 'SetAuthMessage Text' (:26).
    SetAuthMessage(String),
    /// 'SetTimeElapsed Double' (:27).
    SetTimeElapsed(f64),
    /// 'SetAllocatedBytes Int' (:28).
    SetAllocatedBytes(i64),
    /// 'SetMethod Text' (:29).
    SetMethod(String),
    /// 'SetWeight Int' (:30).
    SetWeight(i64),
    /// 'SetRepoName Text' (:31).
    SetRepoName(String),
    /// 'SetRepoHash Text' (:32).
    SetRepoHash(String),
    /// 'SetQuery Text' (:33).
    SetQuery(String),
    /// 'SetPredicate Text' (:34).
    SetPredicate(String),
    /// 'SetPredicateVersion Int' (:35).
    SetPredicateVersion(i64),
    /// 'SetSchemaVersion Int' (:36).
    SetSchemaVersion(i64),
    /// 'SetNoBase64Binary Bool' (:37).
    SetNoBase64Binary(bool),
    /// 'SetExpandResults Bool' (:38).
    SetExpandResults(bool),
    /// 'SetRecursive Bool' (:39).
    SetRecursive(bool),
    /// 'SetMaxResults Int' (:40).
    SetMaxResults(i64),
    /// 'SetTruncated Bool' (:41).
    SetTruncated(bool),
    /// 'SetResults Int' (:42).
    SetResults(i64),
    /// 'SetFacts Int' (:43).
    SetFacts(i64),
    /// 'SetFullScans [Text]' (:44).
    SetFullScans(Vec<String>),
    /// 'SetSyntax Text' (:45).
    SetSyntax(String),
    /// 'SetType Text' (:46).
    SetType(String),
    /// 'SetBytecodeSize Int' (:47).
    SetBytecodeSize(i64),
    /// 'SetCompileTimeUs Int' (:48).
    SetCompileTimeUs(i64),
    /// 'SetExecuteTimeUs Int' (:49).
    SetExecuteTimeUs(i64),
    /// 'SetQueryResultBytes Int' (:50).
    SetQueryResultBytes(i64),
    /// 'SetClientUnixname Text' (:51).
    SetClientUnixname(String),
    /// 'SetClientApplication Text' (:52).
    SetClientApplication(String),
    /// 'SetClientName Text' (:53).
    SetClientName(String),
    /// 'SetSrClientId Text' (:54).
    SetSrClientId(String),
    /// 'SetRequestContinuationSize Int' (:55).
    SetRequestContinuationSize(i64),
    /// 'SetResponseContinuationSize Int' (:56).
    SetResponseContinuationSize(i64),
    /// 'SetSchemaId Text' (:57).
    SetSchemaId(String),
    /// 'SetBatchFactsSize Int' (:58).
    SetBatchFactsSize(i64),
    /// 'SetBatchFactsCount Int' (:59).
    SetBatchFactsCount(i64),
    /// 'SetBatchOwnedSize Int' (:60).
    SetBatchOwnedSize(i64),
    /// 'SetBatchDependenciesSize Int' (:61).
    SetBatchDependenciesSize(i64),
}

impl GleanServerLog {
    /// 'instance Semigroup GleanServerLog where a <> b =
    /// GleanServerLogAnd a b' (:63-64).
    pub fn combine(self, other: Self) -> Self {
        GleanServerLog::GleanServerLogAnd(Box::new(self), Box::new(other))
    }
}

impl Default for GleanServerLog {
    /// 'instance Monoid GleanServerLog where mempty =
    /// GleanServerLogEmpty' (:66-67).
    fn default() -> Self {
        GleanServerLog::GleanServerLogEmpty
    }
}

/// 'data NullGleanServerLogger = NullGleanServerLogger' (:69) — the sink
/// the OSS defaults install (Config.hs:247).
pub struct NullGleanServerLogger;

impl GleanServerLogger for NullGleanServerLogger {
    /// 'runLog _ _ = return ()' (:72): drop the record.
    fn run_log(&self, _log: GleanServerLog) {}
}

// PORT STATUS: confidence=high todos=0
