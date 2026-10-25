// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// Ported from: glean/github/ServiceData/GlobalStats.hs (35 lines; module
// ServiceData.GlobalStats).
// Manifest unit: rust/glean-fb303/src/service_data/global_stats.rs
// (migration/manifest.tsv line 97, crate glean-fb303). RULE §4: one
// source file -> one target file.
//
// STUB PROVENANCE — ADJUDICATED UNDER RULE §0 (old code is the spec):
// the whole 35-line source has been read end to end; every function body
// is `return ()` or `return HashMap.empty` (GlobalStats.hs:17,20,23,26,
// 29,32,35). This file is the OSS build's service-data SHIM: the real
// fb303 counters/histograms backend (GlobalData, thrift fb303) is
// Meta-internal and NOT in this tree — the `github/` tree exists exactly
// so the OSS build compiles and runs against stub interfaces
// (cf. depmap_haskell.py CRATE_PREFIXES: "shared thrift-type wrappers
// (fb303/ServiceData)"; the FB variant lives only in the FB-internal
// repo). The no-ops are therefore the SOURCE'S OWN SEMANTICS, not a port
// deferral: RULE §0 reproduces them no-op-for-no-op, and BUG(port) is
// wrong too (a deliberate shim is not a defect). Hence this file carries
// NO TODO(port) for the stubbedness — there is nothing the port still
// owes. Reviewers: do not "improve" these bodies into real
// implementations; parity here means empty-body-for-empty-body.
//
// MAPPING DECISIONS (each pinned):
//   * `ByteString` PARAMETER -> `&[u8]` (borrowed byte slice) — RULE §1
//     strings row ("`Vec<u8>` for `ByteString`"; the house form for a
//     by-value read-only ByteString parameter, per
//     glean-db/src/database/storage/memory.rs:446 `key: &[u8]`,
//     storage.rs:577 and glean-angle/src/angle/hash.rs:167
//     `hash_byte_string(bytes: &[u8])`; the caller-side name argument is
//     a constructed literal, Write/Queue.hs:120,153 — no mutation need).
//   * `ByteString` MAP KEY -> `Vec<u8>` (owned) — the same strings row;
//     owned form for stored/returned bytes per the
//     storage.rs:490-492 precedent
//     (`type AxiomOwnership = HashMap ByteString (VS.Vector Fid)` ->
//     `HashMap<Vec<u8>, Vec<Fid>>`).
//   * `Int` -> `i64` — Haskell Int is 64-bit on the LP64 target (RULE §2
//     numeric row; glean-angle/src/angle/types.rs:409 "Int (RULE §2
//     numeric row: Haskell Int = 64-bit)",
//     glean-core/src/rts/foreign/ownership.rs:911 "the Haskell Int (i64
//     here ...)").
//   * `IO ()` -> a plain Rust function — RULE §1 sync-core row (the
//     async runtime is banned outside the front-ends; the IO monad is
//     not imaged: Rust functions ARE the effectful form). House
//     precedent: glean-db/src/logger/database.rs:67/88 and
//     logger/server.rs:64 ("`IO ()` -> `()`").
//   * `HashMap.HashMap ByteString Int` (qualified Data.HashMap.Strict,
//     GlobalStats.hs:12) -> `std::collections::HashMap<Vec<u8>, i64>` —
//     the HashMap house image (glean-db/src/database/storage.rs:492,
//     acl_config.rs:113 `HashMap Path [ACL]` -> HashMap;
//     glean-typed/src/typed/build_fact.rs:457
//     `HashMap Thrift.UnitName [Int64]` -> `HashMap<Vec<u8>, Vec<i64>>`).
//
// DEPENDENCY TRUTH (migration/depmap/edges.tsv): GlobalStats.hs:11-14
// imports Data.ByteString, Data.HashMap.Strict (qualified) and
// ServiceData.Types. In-repo edge: :743 GlobalStats.hs -> Types.hs
// (sibling row 96, this batch — wired via `crate::service_data::types`).
// The other two are in-tree stdlib-equivalent packages (bytestring,
// unordered-containers) imaged by the MAPPING DECISIONS above. Reverse
// edges (importers of GlobalStats): edges.tsv:189 Database/Close.hs,
// :265 Database/Delete.hs, :277 Database/Env.hs, :304 Database/Janitor.hs,
// :378 Database/Schema.hs, :503 Database/Write/Queue.hs, :516
// Database/Writes.hs, :698 Query/UserQuery.hs, :816 glass
// Glean/Glass/Handler/Documents.hs, :1304 hs Glean/Write/Stats.hs.
//
// INVENTORY: grep "ServiceData" across migration/inventory*.tsv — zero
// rows. Nothing here is inventory-scoped (no unsafePerformIO, no CAF,
// no STM, no FFI — seven discard-everything no-ops).
//
// SIBLING-UNIT CONTRACTS (future consumers — manifest rows): 105
// (Write/Queue.hs -> glean-db/src/database/write/queue.rs — calls
// addStatValueType with Sum/Avg, Queue.hs:120,152-184), 121 (Close.hs ->
// glean-db/src/database/close.rs), 122 (Delete.hs -> delete.rs), 132
// (Env.hs -> env.rs), 130 (Janitor.hs -> janitor.rs), 98 (Schema.hs ->
// database/schema.rs — already shipped), 116 (Writes.hs -> writes.rs),
// 144 (UserQuery.hs -> glean-db/src/query/user_query.rs), 267
// (Documents.hs -> glean-glass/src/glass/handler/documents.rs —
// qualified `Stats.addStatValueType ... Stats.Avg`), 101 (Write/Stats.hs
// -> glean-core/src/write/stats.rs). These units' stats calls compile
// against exactly the seven no-op signatures below.
//
// CALL-SITE SURVEY (grep, whole tree; grounds for the `_name` naming
// and the dead-but-ported getCounters note below): setCounter —
//     Env.hs:250-255, Close.hs:133, Janitor.hs:89,
//     Writes.hs:398-404 (five sites: envwrites.count/
//     completed_unpolled/pending/…), Write/Stats.hs:153;
//     clearCounter — Janitor.hs:106 (the sole clearCounter site);
// addStatValueType — Queue.hs:120-184, Schema.hs:328-338, Delete.hs:86,
// Writes.hs:123-369, UserQuery.hs:881-1197, Documents.hs:244-304,
// Write/Stats.hs:143-148. addStatValue, addHistogram and
// exportHistogramPercentile have ZERO call sites in tree. getCounters's
// only consumer is glean/test/tests/DatabaseJanitorTest.hs:45,:1021 —
// tests are not translated (DECISIONS §4; depmap excludes
// glean/test/**), so it is dead in the translated tree but still part
// of the module's export surface: ported anyway, because export parity
// is the spec (RULE §4, the manifest is authoritative).
//
// EXPORT PARITY (both directions): the module header is the self-export
// form `module ServiceData.GlobalStats (module ServiceData.GlobalStats)
// where` (GlobalStats.hs:9) — bare module export = every top-level name
// defined in the module. Defined names: exactly the seven functions
// (no types, no instances). Exported names: exactly the seven functions.
// The Rust image exports exactly these seven and nothing else.
//
// PARAMETER NAMING: the source discards every argument unnamed
// (`setCounter _ _ = return ()`, GlobalStats.hs:17). The leading
// ByteString is named `_name` in the port — its role is fixed by the ten
// consumer call sites (metric-name strings, e.g. Queue.hs:120
// "glean.db.write.enqueued"). The trailing `Int`s have NO naming
// evidence in tree (grep addHistogram / exportHistogramPercentile across
// glean/ — zero call sites outside this module), so the port keeps them
// positional rather than inventing semantics (`_p1`, `_p2`, `_p3`;
// RULE §0: never guess from the name).

use std::collections::HashMap;

use crate::service_data::types::ExportType;

/// Source: `setCounter :: ByteString -> Int -> IO ()` (GlobalStats.hs:16),
/// body `setCounter _ _ = return ()` (:17) — discards both arguments.
pub fn set_counter(_name: &[u8], _value: i64) {}

/// Source: `clearCounter :: ByteString -> IO ()` (GlobalStats.hs:19),
/// body `clearCounter _ = return ()` (:20).
pub fn clear_counter(_name: &[u8]) {}

/// Source: `getCounters :: IO (HashMap.HashMap ByteString Int)`
/// (GlobalStats.hs:22), body `getCounters = return HashMap.empty` (:23) —
/// the empty map, exactly. The value type is the house Int image (i64).
pub fn get_counters() -> HashMap<Vec<u8>, i64> {
    HashMap::new()
}

/// Source: `addStatValue :: ByteString -> Int -> IO ()` (GlobalStats.hs:25),
/// body `addStatValue _ _ = return ()` (:26).
pub fn add_stat_value(_name: &[u8], _value: i64) {}

/// Source: `addStatValueType :: ByteString -> Int -> ExportType -> IO ()`
/// (GlobalStats.hs:28), body `addStatValueType _ _ _ = return ()` (:29).
/// Consumes the sibling row 96 exactly as the source does
/// (GlobalStats.hs:14 `import ServiceData.Types`).
pub fn add_stat_value_type(_name: &[u8], _value: i64, _ty: ExportType) {}

/// Source: `addHistogram :: ByteString -> Int -> Int -> Int -> IO ()`
/// (GlobalStats.hs:31), body `addHistogram _ _ _ _ = return ()` (:32).
/// The three `Int`s stay positional (no naming evidence in tree — see
/// PARAMETER NAMING above; zero call sites in the OSS build).
pub fn add_histogram(_name: &[u8], _p1: i64, _p2: i64, _p3: i64) {}

/// Source: `exportHistogramPercentile :: ByteString -> Int -> IO ()`
/// (GlobalStats.hs:34), body `exportHistogramPercentile _ _ = return ()`
/// (:35). The `Int` stays positional (no naming evidence in tree — see
/// PARAMETER NAMING above; zero call sites in the OSS build).
pub fn export_histogram_percentile(_name: &[u8], _p1: i64) {}

// Coverage: the 35-line source is fully accounted for — the module
// header (self-export), the three imports (mapped above) and the seven
// no-op functions, each with its exact signature and empty body; no
// types, instances or other definitions exist to port.
//
// Marker breakdown (closed set — zero of every class): no open port
// todos
// (the stubbedness is the source's own semantics, adjudicated in the
// STUB PROVENANCE block under RULE §0 — nothing was deferred by the
// port), no performance deferrals, no source-defect reproductions (a deliberate shim is not a
// defect), no unsafe (no unsafe), no inventory divergence (no row
// governs this site — see INVENTORY above).
//
// PORT STATUS: confidence=high todos=0
