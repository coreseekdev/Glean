// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// Ported from: glean/github/ServiceData/Types.hs (11 lines; module
// ServiceData.Types).
// Manifest unit: rust/glean-fb303/src/service_data/types.rs
// (migration/manifest.tsv line 96, crate glean-fb303). RULE §4: one
// source file -> one target file.
//
// DEPENDENCY TRUTH (migration/depmap/edges.tsv): Types.hs has ZERO
// imports (the whole source is lines 9-11: the module header and
// `data ExportType = Sum | Avg` — a leaf). Reverse edges (files that
// import ServiceData.Types): edges.tsv:266 Database/Delete.hs, :379
// Database/Schema.hs, :504 Database/Write/Queue.hs, :517
// Database/Writes.hs, :699 Query/UserQuery.hs, :817 glass
// Glean/Glass/Handler/Documents.hs, :1305 hs Glean/Write/Stats.hs
// (plus :743 GlobalStats.hs — this batch's sibling row 97).
//
// INVENTORY: grep "ServiceData" across migration/inventory*.tsv — zero
// rows. Nothing in this file's source is inventory-scoped (no
// unsafePerformIO, no CAF, no STM, no FFI): RULE §5 flags missing rows
// for inventory-worthy sites; a one-constructor-pair data declaration
// is not one.
//
// SIBLING-UNIT CONTRACTS (consumers — manifest rows; the survey covers
// every `import [qualified] ServiceData` site in the tree):
//   * Database/Write/Queue.hs (row 105 -> rust/glean-db/src/database/
//     write/queue.rs) — the ONLY explicit-list import,
//     `import ServiceData.Types (ExportType(..))` (Queue.hs:28);
//     constructors unqualified: `Sum` (:120, :152), `Avg` (:154, :176,
//     :178-179, :184).
//   * glass Glean/Glass/Handler/Documents.hs (row 267 -> rust/glean-glass/
//     src/glass/handler/documents.rs) — `import ServiceData.Types as
//     Stats` (Documents.hs:89); `Stats.Avg` (:244, :291, :304).
//   * Database/Schema.hs (row 98 -> glean-db/src/database/schema.rs) —
//     `import ServiceData.Types as ODS` (Schema.hs:69); `ODS.Sum`
//     (:328, :333, :338).
//   * Database/Delete.hs (row 122 -> glean-db/src/database/delete.rs) —
//     `import ServiceData.Types as Stats` (Delete.hs:24); `Stats.Sum`
//     (:86).
//   * Database/Writes.hs (row 116 -> glean-db/src/database/writes.rs) —
//     plain import (Writes.hs:65); `Sum` (:123, :125, :352, :369).
//   * Query/UserQuery.hs (row 144 -> glean-db/src/query/user_query.rs) —
//     `import qualified ServiceData.Types as Stats` (UserQuery.hs:49);
//     `Stats.Sum` (:881, :1196-1197).
//   * hs Glean/Write/Stats.hs (row 101 -> glean-core/src/write/stats.rs)
//     — plain import (Stats.hs:29); `Avg`/`Sum` as constructor args
//     (:143, :148).
//   * GlobalStats.hs (row 97, this batch's sibling) — plain import
//     (GlobalStats.hs:14).
//   * glean/test/tests/DatabaseJanitorTest.hs:45 imports GlobalStats
//     only, not Types — and tests are not translated (DECISIONS §4;
//     depmap excludes glean/test/**).
// EVERY use across the tree is a constructor value.
//
// EXPORT PARITY (both directions): the module header is the self-export
// form `module ServiceData.Types (module ServiceData.Types) where`
// (Types.hs:9) — bare module export = every top-level name defined in
// the module. Defined names: exactly `ExportType` (Types.hs:11, no
// instances). Exported names: exactly `ExportType`. The Rust image
// exports exactly `pub enum ExportType` and nothing else.

/// Source: `data ExportType = Sum | Avg` (Types.hs:11) — the module's
/// entire definition. The metric-aggregation flavor a stat site requests:
/// callers pass a literal constructor per call site (`Sum`/`Avg` —
/// Write/Queue.hs:120,154; Handler/Documents.hs:244,291,304).
///
/// NO derives, deliberately: the source has no deriving clause
/// (Types.hs:11 is a bare `data`), and the consumer survey (the
/// SIBLING-UNIT CONTRACTS block — all eight import sites checked) uses
/// the constructors as plain values only — no Show, Eq, Ord or Hash
/// need exists in tree. Derive policy
/// per the house rule that derives map from the source's own derived
/// classes (glean-db/src/database/acl_config.rs:105-108: "`Show, Eq`
/// (:48) -> `Debug, PartialEq, Eq`; NO `PartialOrd`/`Ord`: the source
/// does not derive them"). If a future unit needs a trait instance here,
/// that unit's header records the disposition — none is invented now.
///
/// There is no export list beyond the enum: RULE §4 — the manifest is
/// authoritative, and the source exports nothing else.
pub enum ExportType {
    /// Types.hs:11 — `Sum`.
    Sum,
    /// Types.hs:11 — `Avg`.
    Avg,
}

// Coverage: the 11-line source is fully accounted for — the module
// header's self-export and the single data declaration above; no imports
// to port (leaf), no instances, nothing left unported.
//
// Marker breakdown (closed set — zero of every class): no open port
// todos (nothing deferred — the enum is the complete source), no
// performance deferrals (no code), no source-defect reproductions (no
// behavior), no unsafe (so no safety contracts), no inventory
// divergence (no row governs this site — see INVENTORY
// above).
//
// PORT STATUS: confidence=high todos=0
