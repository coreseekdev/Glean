// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// Ported from: glean/db/Glean/Database/Meta.hs (194 lines)
// Manifest unit: rust/glean-db/src/database/meta.rs (migration/manifest.tsv
// line 48, crate glean-db). RULE §4: one source file -> one target file; the
// `database/` subdirectory is generated from the module path
// `Glean.Database.Meta`.
//
// WHAT THIS FILE IS: the DB-metadata module. It owns
//   * `DBTimestamp` (:52-55) — the creation/repo-hash time pair;
//   * `newMeta` (:57-77) — the record-literal builder for `Meta`;
//   * the status readers (:79-95): `showCompleteness`,
//     `showCompletenessFull`, `completenessStatus`;
//   * the age/time readers (:97-110): `dbAge`, `dbTime`,
//     `dbCompletedOrCreated`;
//   * the thrift-database bridge (:112-136): `metaToThriftDatabase`;
//   * the catalog-property codec (:138-145): `metaToProps`,
//     `metaFromProps` — `Meta` is persisted as ONE `"meta"` property whose
//     value is hsthrift's JSON serialization;
//   * the epoch conversions (:149-156) and
//   * ACL-mode detection from database properties (:158-194).
//
// PRAGMA NOTE: the source carries `{-# OPTIONS_GHC -Wno-orphans #-}`
// (Meta.hs:9) but declares no ORPHAN instances — the only instances are
// ACLMode's in-module `deriving (Eq, Show)` (:165), non-orphan and fully
// imaged by the Rust derive list below. The pragma is vestigial; recorded
// here so a later reader does not hunt for a missing orphan instance.
//
// MODULE WIRING (RULE §4): this unit also adds the crate's Database-area
// scaffolding — src/database.rs (Rust-only: Haskell has no
// `Glean.Database` root module; glean/db/Glean/ contains only Database/,
// Logger/, Logger.hs, Query/, Write/) and `pub mod database;` in
// src/lib.rs. This file is a DIRECT child of src/database.rs, so
// `pub mod meta;` is declared there immediately (the src/write.rs:29
// `pub mod send_batch;` shape). Row 49 (src/database/catalog/filter.rs,
// manifest.tsv line 49) appends `pub mod catalog;` inside src/database.rs
// and declares `pub mod filter;` in its own src/database/catalog.rs — that
// contract is recorded in src/database.rs's prose, which row 49's unit
// consumes.
//
// ---------------------------------------------------------------------------
// INVENTORY (RULE §5; migration/inventory*.tsv)
// ---------------------------------------------------------------------------
//
// NO row's source file is this file. Grep-verified "Database/Meta",
// "ACLMode", "Meta.hs" across migration/inventory*.tsv: zero hits. The
// `serializeJSON` grep hits exactly two rows — inventory.tsv:111 and
// inventory.protocol.tsv:14 — and both are the SAME row (the Util/IO
// `HandleIO` LAZY_IO row): its evidence text says "consumers
// db/Glean/Database/Catalog/Local/Files.hs:62,94 (meta deserializeJSON)".
// That names Files.hs's OWN `Thrift.deserializeJSON` call sites (the
// row's :62/:94 citation), not `metaFromProps` — which, like
// `metaToProps`, has zero callers in the OSS tree (repo-wide grep) — and
// the only match inside the row is the substring `serializeJSON` inside
// the word "deserializeJSON"; the row's ruling governs Util/IO's reader,
// not the Meta codec. So no row governs this unit and there is no in-file
// inventory flag. (DECISIONS.md 2026-10-05's "pinned in inventory" clause
// for Glean.Internal.Types is likewise unbacked — see the TODO below.)
//
// ---------------------------------------------------------------------------
// THE CENTRAL ARCHITECTURE CALL: the generated-module homes
// ---------------------------------------------------------------------------
//
// `Glean.Internal.Types` (:49) and `Glean.ServerConfig.Types` (:48) are
// GENERATED modules — gen-hs2 output of glean/if/internal.thrift and
// glean/config/server/server_config.thrift. Neither exists in the checkout
// and neither is a manifest row (stress-test/report.md:74-75), so both are
// ASSUMED imports (RULE §4 B4): import the pinned path, sketch the shape,
// never hand-translate.
//
//   * `Glean.Internal.Types` -> the assumed same-crate module
//     `crate::internal::types`. The HOME is pinned by DECISIONS.md
//     (2026-10-05: StorageName "owns its own module in glean-db
//     (generated-thrift adjacent), pinned in inventory"). Two caveats are
//     carried below in the module's TODO.
//
// TODO(port): the `crate::internal::types` home is provisional in two
// respects, both carried to the records. (1) The DECISIONS.md pin says
// "pinned in inventory", but grep "Internal.Types" / "StorageName" across
// migration/inventory*.tsv finds NO row — the clause has no inventory row
// to point at. (2) The generated module has a CROSS-CRATE consumer:
// glean/schema/gen/Glean/Schema/Gen/Main.hs imports it (:68 `qualified
// Glean.Internal.Types as Internal`) and uses the SchemaIndex/SchemaInstance
// members (:467-493), which are NOT part of this file's surface but live in
// the same generated module — so a glean-db-only home may need
// reconciliation with the shared generated crate at Step 4. Whichever home
// wins, ONLY this file's import PATH changes (the member shapes are
// identical; both are images of internal.thrift), so the port is stable
// under the ruling.
//
//   * `Glean.ServerConfig.Types (DBVersion(..))` -> the assumed
//     `thrift::DBVersion` in the generated `glean_thrift` crate. RULE §1's
//     thrift row pins `glean_thrift` as the A10 generated crate, and
//     server_config.thrift's `typedef i64 DBVersion (hs.newtype)`
//     (:28) collapses to `pub type DBVersion = i64` (gen-hs2
//     ServerConfig/Types.hs:107-108) — the SchemaId
//     `hs.newtype`-over-Text -> alias precedent (backend/types.rs:162-167).
//     NO ruling exists for the ServerConfig.Types home specifically (the
//     DECISIONS 2026-10-05 pin covers StorageName/Internal.Types only);
//     grep of migration/inventory*.tsv for "DBVersion" finds zero rows.
//     ASSUMED: one generated crate for all .thrift files (A10's default).
//     server_config.thrift does declare `namespace rust glean_server_config`
//     (:23), so a per-file generated crate would move this one import —
//     carried to records; the fallback is a compile error at Step 4, never
//     a silent divergence.
//
// ---------------------------------------------------------------------------
// ASSUMED-IMPORT (RULE §4 B4): `glean_thrift`, EXTENDING the first sketch
// ---------------------------------------------------------------------------
//
// rust/glean-db/src/backend/types.rs:87-226 is the FIRST SKETCH of this
// crate's assumed image (flat crate root, IDL field names in snake_case,
// `optional` -> Option, `list` -> Vec, `bytestring` -> Vec<u8>, thrift
// `map` -> BTreeMap, struct Default = hsthrift `def`; with code sketches
// for `Repo` :171-175, `Id` :159 and `DatabaseProperties` :169-170, and
// `Dependencies` :105/:134 acknowledged there as an OPAQUE payload
// (:220-226, no code sketch) — `PredicateRef` was first sketched at
// rust/glean-angle/src/angle/types.rs:52-75, reached through the shipped
// consumer rust/glean-typed/src/typed/predicate.rs:83-96 — with `Version`
// at internal.rs:107). This unit does NOT re-sketch those; it extends the
// surface with the members its OWN code mentions. IDL cites are
// glean/if/glean.thrift; gen-hs2 cites are
// ~/glean-build/Glean/glean/if/gen-hs2/Glean/Types.hs.
//
//     pub type PosixEpochTime = i64;        // :49 `typedef i64
//                                           // PosixEpochTime (hs.newtype)`;
//                                           // gen-hs2 :325-327 — newtype
//                                           // over Int64 -> alias, the
//                                           // SchemaId precedent
//
//     #[derive(Clone, Debug, Default, PartialEq, Eq)]
//     pub struct DatabaseBroken {            // :292-295, fields 1/2
//         pub task: String,                  // gen-hs2 :2383-2385
//         pub reason: String,
//     }                                      // Default = def
//                                           // (gen-hs2 :2447-2448)
//
//     #[derive(Clone, Debug, Default, PartialEq, Eq)]
//     pub struct DatabaseComplete {          // :297-301, fields 1/2
//         pub time: PosixEpochTime,
//         pub bytes: Option<i64>,            // `optional i64`
//     }                                      // gen-hs2 :2455-2458
//                                            // (Default :2528)
//
//     #[derive(Clone, Copy, Debug, PartialEq, Eq)]
//     pub enum DatabaseStatus {              // :270-290, `(hs.nounknown)`
//         Complete,                          // = 0  (gen-hs2 :2315-2323
//         Available,                         // = 7   arm names are
//         Incomplete,                        // = 1   `DatabaseStatus_*`;
//         Restoring,                         // = 2   the Rust arm
//         Broken,                            // = 3   spelling is the
//         Restorable,                        // = 4   fbthrift codegen's,
//         Finalizing,                        // = 5   ASSUMED — Step-4
//         Missing,                           // = 6   spelling check)
//     }   // Default = Complete (gen-hs2 :2331-2332; hsthrift `def`). This
//         // file maps/reads the four arms Complete/Incomplete/Broken/
//         // Finalizing and never names the other four — they are listed so
//         // the enum is complete at Step 4, per the IDL's
//         // `(hs.nounknown)` annotation.
//         // Discriminants from the IDL are documented because gen-hs2's
//         // enumeration order (:2366-2369) differs from IDL order.
//
//     #[derive(Clone, Debug, Default, PartialEq, Eq)]
//     pub struct Database {                  // :303-342, fields 1/4-13;
//                                            // gen-hs2 :2535-2545
//                                            // (Default :2895)
//         pub repo: Repo,                    // :304 (1)
//         pub status: DatabaseStatus,        // :310 (4)
//         pub location: Option<String>,      // :313 (5)
//         pub created_since_epoch: PosixEpochTime, // :317 (6)
//         pub expire_time: Option<PosixEpochTime>, // :321 (7)
//         pub properties: DatabaseProperties, // :325 (8)
//         pub completed: Option<PosixEpochTime>,   // :328 (9)
//         pub repo_hash_time: Option<PosixEpochTime>, // :332 (10)
//         pub dependencies: Option<Dependencies>,  // :335 (11)
//         pub broken: Option<DatabaseBroken>,      // :338 (12)
//         pub complete: Option<DatabaseComplete>,  // :341 (13)
//     }
//     // :306-307 mark slots 2/3 deprecated and they are absent from the
//     // struct; unlike GetDatabaseResult (:869-874) / ListDatabasesResult
//     // (:862-866), `Database` carries NO auth fields — all 11 fields are
//     // listed above, nothing is skipped. `metaToThriftDatabase` sets all
//     // of them (:118-136).
//
// If the generated crate spells an arm or field differently, only the use
// sites in THIS file change (RULE §4 B4: a compile error at Step 4, never a
// silent divergence). All shapes above are plain data (String/i64/bool/
// Option/Vec/BTreeMap) — `Send + Sync` structurally, no interior mutability.
//
// ---------------------------------------------------------------------------
// ASSUMED-IMPORT (RULE §4 B4): `crate::internal::types` (generated-adjacent)
// ---------------------------------------------------------------------------
//
// The module is the assumed glean-db home of Glean.Internal.Types (DECISIONS
// 2026-10-05; the TODO above carries its two caveats). IDL cites are
// glean/if/internal.thrift; gen-hs2 cites are
// ~/glean-build/Glean/glean/if/internal/gen-hs2/Glean/Internal/Types.hs.
// The full assumed member set this file depends on — the code below names
// `Completeness`, `Meta`, `StorageName`; `DatabaseIncomplete` and
// `DatabaseFinalizing` ride `Completeness`'s variants and are never named,
// so they are sketched here but not imported:
//
//     #[derive(Clone, Debug, Default, PartialEq, Eq)]
//     pub struct DatabaseIncomplete {}       // :39; gen-hs2 :190-191,
//                                            // Default :219-220
//
//     #[derive(Clone, Debug, Default, PartialEq, Eq)]
//     pub struct DatabaseFinalizing {}       // :41; gen-hs2 :225-226,
//                                            // Default :254-255
//
//     #[derive(Clone, Debug, PartialEq, Eq)]
//     pub enum Completeness {                // :44-49 (union, field ids
//         Empty,                             //        1/3/4/5, `hs.prefix
//         Incomplete(DatabaseIncomplete),    //        = ""`, `hs.nonempty`)
//         Complete(DatabaseComplete),        // glean.thrift :297-301
//         Broken(DatabaseBroken),            // glean.thrift :292-295
//         Finalizing(DatabaseFinalizing),
//     }
//     // gen-hs2 :260-264 has exactly FOUR arms (Incomplete/Complete/Broken/
//     // Finalizing): `hs.nonempty` drops hsthrift's unset value from the
//     // Haskell type. The Rust image adds the fbthrift union convention's
//     // `Empty` unset arm (the SelectSchema convention,
//     // backend/types.rs:176-186); the wire parser rejects an unset union
//     // (gen-hs2 :320 "union 'Completeness' is empty"), so `Empty` is
//     // unconstructible from the wire — the same assumption unit 46 recorded
//     // for SendResponse/FinishResponse (send_batch.rs:526-531), revisited
//     // against the generated crate if it becomes constructible.
//     // `Default` must equal hsthrift `def` = `Incomplete DatabaseIncomplete`
//     // (gen-hs2 :333-334) — it is `Meta::default()`'s
//     // `meta_completeness` (gen-hs2 :679) and is the generated crate's
//     // obligation at Step 4.
//
//     pub type StorageName = String;         // :51 `typedef string
//                                            // StorageName (hs.newtype)`;
//                                            // gen-hs2 :348-349 — alias,
//                                            // the SchemaId precedent; the
//                                            // `unStorageName` accessor
//                                            // and `StorageName` constructor
//                                            // sites (e.g.
//                                            // Database/Types.hs:55 `import
//                                            // ... (StorageName(..))`,
//                                            // :251 `envStorage`) collapse to
//                                            // the alias
//
//     #[derive(Clone, Debug, Default, PartialEq, Eq)]
//     pub struct Meta {                      // :54-88, IDL order, fields 1-10
//         pub meta_version: DBVersion,       // :56 (1) server_config.DBVersion
//         pub meta_created: PosixEpochTime,  // :59 (2)
//         pub meta_completeness: Completeness, // :62 (3)
//         pub meta_backup: Option<String>,   // :65 (4, optional)
//         pub meta_properties: DatabaseProperties, // :69 (5)
//         pub meta_dependencies: Option<Dependencies>, // :72 (6, optional)
//         pub meta_complete_predicates: Vec<PredicateRef>, // :75 (7)
//         pub meta_axiom_complete: bool,     // :80 (8)
//         pub meta_repo_hash_time: Option<PosixEpochTime>, // :84 (9,
//                                            // optional)
//         pub meta_storage: StorageName,     // :87 (10)
//     }   // gen-hs2 :358-368 (Eq/Show); Default = def, gen-hs2 :675-686
//         // (version 0, epoch 0, Incomplete, Nothing, empty map, Nothing,
//         // [], False, Nothing, StorageName "")
//
// The serde obligation (owned by this generated module, owed at Step 4):
// `metaToProps`/`metaFromProps` below serialize through `serde_json`
// (RULE §1 JSON row), so `Meta` must implement `serde::Serialize` +
// `serde::Deserialize` with the hsthrift JSON shape (gen-hs2's ToJSON
// :370-393 names the fields; the Thrift JSON writer emits them in IDL order
// with `Nothing` optionals OMITTED — gen-hs2 :418-427/:441-450/:464-476).
// Ground-truth bytes for sample `Meta`s, captured against the real
// generated packages, are in /tmp/opencode/db48-probe/. No ruling pins the
// attribute-level spelling; carried to records.
//
// ---------------------------------------------------------------------------
// ASSUMED-IMPORT: the `glean_util::time_sec` seam (FIRST SKETCH)
// ---------------------------------------------------------------------------
//
// `import Util.TimeSec` (Meta.hs:46) has NO manifest unit: Util.TimeSec is
// hsthrift's common utility
// (~/glean-build/Glean/hsthrift/common/util/Util/TimeSec.hs, export list
// :12-30) — the external-util seam class, sibling of the `glean_util::log`
// seam (RULEBOOK.md:38; first sketch internal.rs:190-213). This unit is the
// seam's FIRST consumer, so the pinned surface is written HERE and imported
// directly (`use glean_util::time_sec::{to_utc_time, Time};`) — the
// units-45/46 direct-final-call-site convention (DECISIONS.md 2026-10-18),
// no stand-in. The module does not exist on disk yet; the swappable part is
// import resolution only, never a call site.
//
// Members this file CONSUMES, cited to the pinned hsthrift source:
//
//     /// `newtype Time = Time { timeInSeconds :: Int }`
//     /// (TimeSec.hs:46-47; Int = i64 on the pinned 64-bit target).
//     pub struct Time(pub i64);
//
//     /// `toUTCTime (Time t) = posixSecondsToUTCTime $ fromIntegral t`
//     /// (TimeSec.hs:66-67) — EXACT whole seconds: the `fromIntegral`
//     /// (Int -> POSIXTime) is exact, and posixSecondsToUTCTime inverts
//     /// utcTimeToPOSIXSeconds exactly at integer points. It does NOT
//     /// truncate: `Time 1` is 1970-01-01T00:00:01Z.
//     pub fn to_utc_time(t: Time) -> SystemTime;
//
// `posixEpochTimeToUTCTime` (:152-153) is `toUTCTime . posixEpochTimeToTime`
// and the port goes through the same member, exactly as the source does.
//
// Members the seam OWES to later consumers — first-sketched here so the
// next consumer extends rather than re-derives, NOT called by this file:
//
//     /// `fromUTCTime = Time . truncate . utcTimeToPOSIXSeconds`
//     /// (TimeSec.hs:63-64) — TRUNCATES toward zero, in deliberate contrast
//     /// to `utcTimeToPosixEpochTime`'s round-half-even (:150).
//     /// CONSUMERS: Janitor.hs:343 (`fromUTCTime now \`timeDiff\`
//     /// posixEpochTimeToTime t0`) and :419 (`fromUTCTime t1 \`timeDiff\`
//     /// fromUTCTime t2`); manifest row 130.
//     pub fn from_utc_time(t: SystemTime) -> Time;
//
// The wider surface (incl. TimeSpan :55, TimeRange :58-61, Time's derived
// Num/Enum/JSON instances :49-52, now :84-85, timeAgo :87-88,
// addTime/subTime/timeDiff :69-76, addTimeSpan :81-82,
// minutes/hours/days/weeks :90-95, minute/hour/day/week :96-100,
// trLength/trLast/trBetween :102-115, toDays/toHours/toMinutes/toSeconds
// :132-136, ppUTCTime/ppDate/ppTime/ppTimeSpan/ppTimeSpanWithGranularity
// :118-176, PPTimeSpanGranularity :127, showNominalDiffTime :178-179) is
// owed to future consumers: the db/server areas' later users grep to
// `timeDiff`/`timeSpanInSeconds`/`fromUTCTime` (all Janitor.hs, above)
// plus Delete.hs:30's bare `import Util.TimeSec`; each arrives with its
// own unit's extension in its file's block (RULE §4 consumer-extension).
// The module HOME follows the external-util seam pattern (Util.Log ->
// `glean_util::log`, RULEBOOK.md:38); no ruling names the time_sec home
// yet — carried to records.
//
// ---------------------------------------------------------------------------
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, module-level)
// ---------------------------------------------------------------------------
//
// Outbound: NO rows. `awk -F'\t' '$1=="glean/db/Glean/Database/Meta.hs"'`
// is empty — every import of this file is stdlib, an external hsthrift
// module, or generated code, and the depmap carries no edge for those
// (generated modules are absent from the depmap; that absence is exactly
// why they are assumed imports). The direct source imports with manifest
// units: NONE.
//
// Inbound: 23 rows, all consumers (row-1 = importer, row-2 = this file):
//   Backup.hs, Backup/Backend.hs, Catalog.hs, Catalog/Filter.hs,
//   Catalog/Local/Files.hs, Catalog/Local/Memory.hs, Catalog/Store.hs,
//   CompletePredicates.hs, Create.hs, Delete.hs, Finish.hs, Janitor.hs,
//   List.hs, Open.hs, Restore.hs, Retention.hs, Types.hs, Write/Batch.hs,
//   tools/gleancli/GleanCLI/{Complete,Create,Finish,Restore,Write}.hs.
//   Consumers only — this file's export surface is the contract they were
//   written against, no impact.
//
// ---------------------------------------------------------------------------
// Import dispositions (Meta.hs:35-50 — every import, in source order)
// ---------------------------------------------------------------------------
//
//   * Data.ByteString.Char8 as B (:35; `B.unpack` :139, `B.pack` :144) ->
//     erased at the JSON seam: `serde_json::to_string`/`from_str` work on
//     `String` directly (RULE §1 strings row; the byte/UTF-8 accounting is
//     in `meta_to_props`'s doc).
//   * Data.Functor (:36; `<&>` :144) -> `Result::map` in expression
//     position; the port's `?`+construct-by-update is the same dataflow
//     (`deserializeJSON (B.pack str) <&> \meta -> meta{..}` = "on success,
//     rewrite").
//   * Data.HashMap.Strict as HashMap (:37; `HashMap.lookup` :177) ->
//     `DatabaseProperties` is a `BTreeMap` (FIRST SKETCH
//     backend/types.rs:169-170, pinned by glean-typed's predicate.rs); the
//     lookup is `BTreeMap::get`.
//   * Data.Map (Map) + qualified Data.Map as Map (:38-39; `Map.fromList`
//     :139, `Map.lookup` :142) -> `std::collections::BTreeMap` +
//     `insert`/`get`. NOTE: `metaToProps`' `Map String String` is Data.Map
//     (a different Haskell type from DatabaseProperties' HashMap) but the
//     same Rust type — one `BTreeMap<String, String>` image for both.
//   * Data.Maybe (:40; `fromMaybe` :102) -> `Option::unwrap_or`.
//   * Data.Text (Text) (:41; `loc :: Text` :141, returns :79-88) ->
//     `&str` for the borrowed argument, `String` (owned) for returns
//     (RULE §1 strings row).
//   * Data.Time (UTCTime, NominalDiffTime, diffUTCTime) (:42) ->
//     `std::time::SystemTime` (UTCTime), `std::time::Duration` (the
//     unsigned image; the signed `NominalDiffTime` -> saturation divergence
//     is documented at `db_age`), `SystemTime::duration_since` for
//     `diffUTCTime` (:98). RULE §1 Time row / B3.
//   * Data.Time.Clock.POSIX (:43; utcTimeToPOSIXSeconds :150,
//     posixSecondsToUTCTime via Util.TimeSec :153) -> the explicit epoch
//     arithmetic in `utc_time_to_posix_epoch_time` /
//     `posix_epoch_time_to_utc_time` below.
//   * Thrift.Protocol.JSON (:45; `serializeJSON` :139, `deserializeJSON`
//     :144) -> `serde_json` (RULE §1 JSON row); the byte-parity obligation
//     belongs to the generated `Meta` (see its block above).
//   * Util.TimeSec (:46) -> the `glean_util::time_sec` seam above.
//   * Glean.ServerConfig.Types (DBVersion(..)) (:48) -> assumed
//     `thrift::DBVersion` (see the architecture call).
//   * Glean.Internal.Types (:49) -> assumed `crate::internal::types` (see
//     the architecture call).
//   * Glean.Types (:50) -> `glean_thrift` as `thrift` (generated;
//     `Repo`/`Dependencies`/`PredicateRef` first sketch:
//     backend/types.rs:87-226, angle/types.rs:52-75). The source imports it
//     UNQUALIFIED, unlike backend/types.rs's double import; both collapse
//     to the crate's one `thrift::` spelling (backend/types.rs decision
//     (I): one image, no second spelling).
//
// ---------------------------------------------------------------------------
// EXPORT MAPPING (Meta.hs:10-33 — 21 names, source order)
// ---------------------------------------------------------------------------
//
//   Meta(..)                   -> pub struct Meta in crate::internal::types,
//                                 re-exported here (`pub use`) for the four
//                                 consumer modules that import Meta FROM
//                                 this module (Catalog/Local/Files.hs:30,
//                                 Memory.hs:19, Store.hs:15, Open.hs:61 —
//                                 the last with `Meta(..)`)
//   DBTimestamp(..)            -> pub struct DBTimestamp
//   newMeta                    -> new_meta
//   showCompleteness           -> show_completeness
//   showCompletenessFull       -> show_completeness_full
//   completenessStatus         -> completeness_status
//   dbAge                      -> db_age
//   dbTime                     -> db_time
//   dbCompletedOrCreated       -> db_completed_or_created
//   metaToThriftDatabase       -> meta_to_thrift_database
//   metaToProps                -> meta_to_props
//   metaFromProps              -> meta_from_props
//   utcTimeToPosixEpochTime    -> utc_time_to_posix_epoch_time
//   posixEpochTimeToUTCTime    -> posix_epoch_time_to_utc_time
//   posixEpochTimeToTime       -> posix_epoch_time_to_time
//   ACLMode(..)                -> pub enum AclMode (acronym re-cased per
//                                 the house Rust UpperCamel: the ACL
//                                 acronym is already lower-cased in file
//                                 naming — ACLConfig.hs -> acl_config.rs,
//                                 manifest.tsv:55; `ACLDisabled` ->
//                                 `AclDisabled`. Note the DbShard TYPE kept
//                                 its spelling only because the source type
//                                 is already `DbShard`, ThriftService.hs:36)
//   getACLMode                 -> get_acl_mode
//   showACLMode                -> show_acl_mode
//   isACLEnabled               -> is_acl_enabled
//   isACLEnforced              -> is_acl_enforced
//   isACLPermissive            -> is_acl_permissive
//
// Nothing else is defined in the source; nothing else is pub here.
//
// ---------------------------------------------------------------------------
// RUST-ONLY DECISIONS AND MAPPING NOTES
// ---------------------------------------------------------------------------
//
//   * `DBTimestamp` derives NOTHING — the source declares no deriving
//     clause (:52-55) and no instance; `SystemTime`/`Option<SystemTime>`
//     are `Copy`, so the one consumer below needs no `Clone`.
//   * Time types: UTCTime -> SystemTime, NominalDiffTime -> Duration (RULE
//     §1 Time row / B3).
//   * `utcTimeToPosixEpochTime = PosixEpochTime . round .
//     utcTimeToPOSIXSeconds` (:149-150): `round` is `RealFrac`'s DEFAULT
//     method — round-half-even on the exact nominal-diff value, whose
//     resolution is PICOSECONDS (Data.Fixed's `MkFixed Integer` at
//     10^12/s; GHC.Real's default `round` is the half-even branch with
//     floor-based `properFraction`). The port converts SystemTime ->
//     secs+nanos -> an i128 picosecond count and applies the same
//     floor/half-even decision (`round_ps_to_posix_epoch_time`), including
//     PRE-EPOCH instants. Differential battery vs GHC 9.4.7:
//     /tmp/opencode/db48-probe/ (exact .5/.25/.75-second fractions, both
//     signs, the pre-epoch integer-boundary cases) — byte-identical
//     results. `PosixEpochTime` is an alias now, so the source's newtype
//     wrapper disappears; `round`'s Int64 target is the `i128 as i64` cast,
//     which at the +999999999ns-past-i64::MAX boundary wraps to i64::MIN
//     exactly as the source's Int64 `round` does (probe-verified,
//     bug-for-bug parity).
//   * `posixEpochTimeToUTCTime` (:152-153) = `toUTCTime .
//     posixEpochTimeToTime`: exact whole seconds, NEGATIVE epochs allowed
//     (the TimeSec seam's `to_utc_time` is total; `posixSecondsToUTCTime`
//     is its image).
//   * Resolution: `UTCTime` carries PICOSECOND-resolution
//     `NominalDiffTime`; `SystemTime`'s resolution is the platform's
//     (nanosecond on Linux). Sub-nanosecond instants are not representable
//     as `SystemTime`, so none can reach the port; over every representable
//     input the round decision above is exact. The one visible GHC/Rust
//     battery difference — `now = metaCreated + 1 ps`, where GHC's `dbAge`
//     reports 1 ps and the port reports 0 — is exactly this grid collapse,
//     not an arithmetic divergence. (The source's own inputs —
//     `getCurrentTime`, persisted meta values — are nanosecond-or-coarser
//     in practice.)
//   * `posixEpochTimeToTime = Time . fromIntegral . unPosixEpochTime`
//     (:155-156): identity on i64 -> `Time(p)`, the TimeSec seam.
//   * `dbAge`'s sign loss: the source returns a SIGNED `NominalDiffTime`;
//     Rust's `Duration` is unsigned, and `duration_since` errors when
//     `now < created`. The port saturates that case to `Duration::ZERO`
//     (documented at the site); callers compare against positive
//     thresholds, so only an already-degenerate (clock-skew / future
//     `metaCreated`) value displays differently.
//   * `showCompleteness`'s literals are `Text` (OverloadedStrings is in
//     the project's default-extensions, glean.cabal.in:25-48, the
//     extension at :42); the Rust image is owned `String` (RULE §1
//     strings row).
//   * `hs.nonempty` unions: `Completeness` has no unset arm in Haskell
//     (gen-hs2 :260-264) but the Rust enum must carry one for
//     exhaustiveness. The two EXHAUSTIVE source matches
//     (`showCompleteness` :79-83, `completenessStatus` :90-95) get an
//     `Empty => unreachable!(...)` arm — the unit-46 convention
//     (send_batch.rs:526-531); the source's own `_` catch-alls
//     (:88, :110, :126-135) absorb `Empty` exactly as they absorb the
//     four real arms, so those matches need no special arm.
//   * ACLMode derives `Clone, Copy, Debug, PartialEq, Eq` (source:
//     `Eq, Show` → `PartialEq/Eq/Debug`; Copy/Clone per the fieldless-sum
//     convention, the StackedDbOpts precedent backend/types.rs:377-380)
//     plus a constructor-shaped `Display` (RULE §2 deriving-Show row)
//     that renders the SOURCE constructor names `ACLDisabled` … — the
//     exact text GHC's `deriving Show` would produce (the Rust variant
//     spelling drops the capitalisation, the parity text must not).
//   * `getACLMode`'s `HashMap.lookup` (:177) -> `BTreeMap::get` on the
//     pinned DatabaseProperties image; the five-case match is
//     byte-for-byte the source's (including the `_ -> ACLDisabled`
//     default for a missing key or an unrecognized value).
//   * Struct construction order mirrors the source's record literals /
//     record syntax field order so a line-by-line diff is possible.

use std::collections::BTreeMap;
use std::fmt;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use glean_thrift as thrift;
use glean_util::time_sec::{to_utc_time, Time};

use crate::internal::types::{Completeness, StorageName};

// The source's `Meta(..)` export (:11): re-exported from its owner so the
// in-tree consumers that import `Meta` FROM this module keep resolving
// (Catalog/Local/Files.hs:30, Catalog/Local/Memory.hs:19,
// Catalog/Store.hs:15 and Open.hs:61 — the last with `Meta(..)`);
// this file re-instantiates nothing.
pub use crate::internal::types::Meta;

// ---------------------------------------------------------------------------
// DBTimestamp (Meta.hs:52-55)
// ---------------------------------------------------------------------------

/// `data DBTimestamp = DBTimestamp { timestampCreated :: UTCTime,
/// timestampRepoHash :: Maybe UTCTime }` (:52-55).
///
/// No derive list, faithfully: the source declares no deriving clause and
/// defines no instance. `SystemTime` and `Option<SystemTime>` are `Copy`,
/// so by-value passing needs nothing more.
pub struct DBTimestamp {
    pub timestamp_created: SystemTime,
    pub timestamp_repo_hash: Option<SystemTime>,
}

// ---------------------------------------------------------------------------
// newMeta (Meta.hs:57-77)
// ---------------------------------------------------------------------------

/// `newMeta :: StorageName -> DBVersion -> DBTimestamp -> Completeness ->
/// DatabaseProperties -> Maybe Dependencies -> Meta` (:57-65) — "Produce DB
/// metadata".
///
/// All 10 fields are built here; the literal's field order (:66-77) is
/// preserved. `metaCompletePredicates = mempty` (:74) is `[]` for the
/// `[PredicateRef]` field -> `Vec::new()`.
pub fn new_meta(
    storage: StorageName,
    version: thrift::DBVersion,
    timestamp: DBTimestamp,
    completeness: Completeness,
    properties: thrift::DatabaseProperties,
    deps: Option<thrift::Dependencies>,
) -> Meta {
    Meta {
        meta_version: version,
        meta_created: utc_time_to_posix_epoch_time(timestamp.timestamp_created),
        meta_repo_hash_time: timestamp
            .timestamp_repo_hash
            .map(utc_time_to_posix_epoch_time),
        meta_completeness: completeness,
        meta_properties: properties,
        meta_backup: None,
        meta_dependencies: deps,
        meta_complete_predicates: Vec::new(),
        meta_axiom_complete: false,
        meta_storage: storage,
    }
}

// ---------------------------------------------------------------------------
// showCompleteness (Meta.hs:79-83)
// ---------------------------------------------------------------------------

/// `showCompleteness :: Completeness -> Text` (:79-83). The literals are
/// `Text` (OverloadedStrings); the Rust image is owned `String`.
pub fn show_completeness(completeness: &Completeness) -> String {
    match completeness {
        Completeness::Incomplete(_) => "incomplete".to_string(),
        Completeness::Complete(_) => "complete".to_string(),
        Completeness::Broken(_) => "broken".to_string(),
        Completeness::Finalizing(_) => "finalizing".to_string(),
        // (hs.nonempty) — no Haskell image (gen-hs2 :260-264); only the
        // wire-level deserializer could produce an unset union, and it
        // rejects one (gen-hs2 :320).
        Completeness::Empty => {
            unreachable!("Completeness::Empty has no Haskell image (hs.nonempty)")
        }
    }
}

// ---------------------------------------------------------------------------
// showCompletenessFull (Meta.hs:85-88)
// ---------------------------------------------------------------------------

/// `showCompletenessFull :: Completeness -> Text` (:85-88) —
/// `"broken at task \"" <> task <> "\": " <> reason` for `Broken`, and
/// `showCompleteness` for everything else.
///
/// `task`/`reason` are `Text` in the source (glean.thrift:292-295,
/// `DatabaseBroken`), `String` here; both render raw, with no escaping —
/// the quotes in the output are literal.
pub fn show_completeness_full(completeness: &Completeness) -> String {
    match completeness {
        Completeness::Broken(thrift::DatabaseBroken { task, reason }) => {
            format!("broken at task \"{}\": {}", task, reason)
        }
        x => show_completeness(x),
    }
}

// ---------------------------------------------------------------------------
// completenessStatus (Meta.hs:90-95)
// ---------------------------------------------------------------------------

/// `completenessStatus :: Meta -> DatabaseStatus` (:90-95). Exhaustive in
/// the source; the unset-union arm is unreachable (see `show_completeness`).
pub fn completeness_status(meta: &Meta) -> thrift::DatabaseStatus {
    match &meta.meta_completeness {
        Completeness::Incomplete(_) => thrift::DatabaseStatus::Incomplete,
        Completeness::Complete(_) => thrift::DatabaseStatus::Complete,
        Completeness::Broken(_) => thrift::DatabaseStatus::Broken,
        Completeness::Finalizing(_) => thrift::DatabaseStatus::Finalizing,
        Completeness::Empty => {
            unreachable!("Completeness::Empty has no Haskell image (hs.nonempty)")
        }
    }
}

// ---------------------------------------------------------------------------
// dbAge (Meta.hs:97-98)
// ---------------------------------------------------------------------------

/// `dbAge :: UTCTime -> Meta -> NominalDiffTime` (:97-98) —
/// `now \`diffUTCTime\` posixEpochTimeToUTCTime (metaCreated meta)`.
///
/// DIVERGENCE, documented: the source's `NominalDiffTime` is SIGNED;
/// `SystemTime::duration_since` returns `Err` when `now` precedes the
/// created time, and the port saturates that case to `Duration::ZERO`
/// (Rust `Duration` cannot be negative). Reachable only through clock skew
/// or a `metaCreated` in the future — both already-degenerate inputs; real
/// callers (Janitor's retention windows) compare ages against positive
/// thresholds. GHC vs Rust behavior for the negative case is captured in
/// /tmp/opencode/db48-probe/ (GHC: negative NominalDiffTime; port: zero).
pub fn db_age(now: SystemTime, meta: &Meta) -> Duration {
    now.duration_since(posix_epoch_time_to_utc_time(meta.meta_created))
        .unwrap_or(Duration::ZERO)
}

// ---------------------------------------------------------------------------
// dbTime (Meta.hs:100-102)
// ---------------------------------------------------------------------------

/// "We sort DBs by metaRepoHashTime if available, or otherwise metaCreated"
/// (:100) — `fromMaybe (metaCreated meta) (metaRepoHashTime meta)` (:102).
pub fn db_time(meta: &Meta) -> thrift::PosixEpochTime {
    meta.meta_repo_hash_time.unwrap_or(meta.meta_created)
}

// ---------------------------------------------------------------------------
// dbCompletedOrCreated (Meta.hs:104-110)
// ---------------------------------------------------------------------------

/// "The time indexing completed, or the creation time for DBs that are not
/// complete. Used as a stable tiebreaker between DBs that share a 'dbTime',
/// e.g. multiple schema-id builds of the same repo revision." (:104-106).
/// The source's `_` catch-all (:110) absorbs every non-`Complete` arm —
/// including the unset union, exactly as the four real arms are absorbed.
pub fn db_completed_or_created(meta: &Meta) -> thrift::PosixEpochTime {
    match &meta.meta_completeness {
        Completeness::Complete(thrift::DatabaseComplete { time, .. }) => *time,
        _ => meta.meta_created,
    }
}

// ---------------------------------------------------------------------------
// metaToThriftDatabase (Meta.hs:112-136)
// ---------------------------------------------------------------------------

/// `metaToThriftDatabase :: DatabaseStatus -> Maybe UTCTime -> Repo -> Meta
/// -> Database` (:112-117). The source takes the `Meta` by record wildcard
/// and returns a per-call value; the port borrows `&Meta` and clones the
/// four owned payloads (`metaBackup`, `metaProperties`, `metaDependencies`,
/// the completeness payloads) into the returned `Database`, which the
/// thrift-call convention consumes by value (backend/types.rs:437-438).
pub fn meta_to_thrift_database(
    status: thrift::DatabaseStatus,
    expire: Option<SystemTime>,
    repo: thrift::Repo,
    meta: &Meta,
) -> thrift::Database {
    thrift::Database {
        repo,
        status,
        location: meta.meta_backup.clone(),
        created_since_epoch: meta.meta_created,
        expire_time: expire.map(utc_time_to_posix_epoch_time),
        properties: meta.meta_properties.clone(),
        completed: match &meta.meta_completeness {
            Completeness::Complete(thrift::DatabaseComplete { time, .. }) => Some(*time),
            _ => None,
        },
        repo_hash_time: meta.meta_repo_hash_time,
        dependencies: meta.meta_dependencies.clone(),
        broken: match &meta.meta_completeness {
            Completeness::Broken(broken) => Some(broken.clone()),
            _ => None,
        },
        complete: match &meta.meta_completeness {
            Completeness::Complete(complete) => Some(complete.clone()),
            _ => None,
        },
    }
}

// ---------------------------------------------------------------------------
// metaToProps (Meta.hs:138-139)
// ---------------------------------------------------------------------------

/// `metaToProps :: Meta -> Map String String` (:138-139) —
/// `Map.fromList [("meta", B.unpack (serializeJSON meta))]`.
///
/// `serializeJSON` (hsthrift, Thrift/Protocol/JSON.hs:46-47) is the
/// TSimpleJSONProtocol serialization of the `ThriftSerializable Meta`
/// instance; its Rust image is `serde_json::to_string(meta)` (RULE §1 JSON
/// row). `B.unpack` (:139) is Char8 (one `Char` per byte); the value type
/// of the source map is Haskell `String`, the port's is `String` — for the
/// UTF-8 JSON the serializer emits, the round-trip through
/// `meta_from_props`' `B.pack` is byte-identical, so no byte-boundary is
/// lost (RULE §1 strings row).
///
/// Totality: hsthrift's `serializeJSON` is total and pure; serde's
/// `to_string` returns `Result`. For a generated plain-data shape the error
/// is unreachable — serde_json's `to_string` only fails on a non-string map
/// key (or a failing custom `Serialize`), and every map in the assumed
/// `Meta` has `String` keys (RULE §1 strings row). Pinned handling:
/// `.expect(...)` — a real failure would be a Step-4 generator defect and
/// must surface as a panic, never as an invented fallback. (Alternative
/// considered: `unwrap_or_default()` — rejected: it would silently persist
/// an empty `"meta"` property and corrupt the DB.)
///
/// Step-4 parity oracle: the aeson/hsthrift byte shape (camelCase names,
/// `Nothing` optionals omitted, the union's one-key object, integer epochs)
/// is the GENERATED `Meta`'s serde obligation, owed at Step 4; sample
/// ground-truth bytes and the `metaFromProps` behaviors are captured in
/// /tmp/opencode/db48-probe/. One ordering caveat, probed and carried: the
/// hsthrift writer emits `metaProperties` through `HashMap.toList`
/// (unspecified order — the two-entry probe sample renders
/// `{"c":"d","a":"b"}`), while a serde `BTreeMap` emits sorted keys; which
/// one the generated crate must reproduce is its call at Step 4. NO
/// inventory row pins this JSON site: the only `serializeJSON`-substring
/// hits in migration/inventory*.tsv are the Util/IO HandleIO row's "meta
/// deserializeJSON" evidence (inventory.tsv:111 ==
/// inventory.protocol.tsv:14) naming Catalog/Local/Files.hs's CONSUMER
/// call.
pub fn meta_to_props(meta: &Meta) -> BTreeMap<String, String> {
    let mut props = BTreeMap::new();
    props.insert(
        "meta".to_string(),
        serde_json::to_string(meta)
            .expect("Meta JSON serialization is infallible for the generated shape"),
    );
    props
}

// ---------------------------------------------------------------------------
// metaFromProps (Meta.hs:141-145)
// ---------------------------------------------------------------------------

/// `metaFromProps :: Text -> Map String String -> Either String Meta`
/// (:141-145). Missing key -> `Left "missing property 'meta'"` (:145),
/// reproduced byte-exactly; present key -> `deserializeJSON (B.pack str)`
/// with `metaBackup` rewritten to `Just loc` (:144).
///
/// The `metaBackup` rewrite is a Rust struct update, the image of the
/// source's record update: `Meta { meta_backup: Some(loc.to_string()),
/// ..meta }` (legal because every generated field is `pub` — the assumed
/// image's contract; rustc-verified in the probe).
///
/// Error text: only the MISSING-KEY message is byte-compatible. On
/// malformed JSON, hsthrift's `deserializeJSON` returns its parser's
/// `String`, serde_json returns its own `Display` text; the two parsers'
/// messages differ and the port maps serde's through `to_string()`
/// (documented, not flagged — no inventory row governs this site; the
/// ground-truth GHC messages are in the probe output for Step 4's parity
/// work: `Left "user error (incomplete input)"` for the input `{`).
///
/// One further parser-shape difference, probed and recorded for Step 4:
/// hsthrift's Thrift-JSON parser SKIPS a field whose wire type mismatches
/// the schema — `{"metaVersion":"x"}` deserializes successfully with the
/// field left at its default (probe `J wrongtype`) — while serde rejects a
/// value of the wrong type. Which shape the generated `Meta`'s
/// `Deserialize` obligation reproduces is a Step-4 decision (the generated
/// module owns it); the port's call shape is unaffected.
pub fn meta_from_props(loc: &str, ps: &BTreeMap<String, String>) -> Result<Meta, String> {
    match ps.get("meta") {
        Some(str) => {
            let meta: Meta = serde_json::from_str(str).map_err(|e| e.to_string())?;
            Ok(Meta {
                meta_backup: Some(loc.to_string()),
                ..meta
            })
        }
        None => Err("missing property 'meta'".to_string()),
    }
}

// ---------------------------------------------------------------------------
// Time functions (Meta.hs:147-156)
// ---------------------------------------------------------------------------

/// `utcTimeToPosixEpochTime = PosixEpochTime . round .
/// utcTimeToPOSIXSeconds` (:149-150).
///
/// `round` is `RealFrac`'s default method on a value whose resolution is
/// picoseconds (`NominalDiffTime` = `MkFixed Integer`, 10^12 units/s):
/// floor-based `properFraction`, then the half-even decision (toward the
/// even integer exactly at .5). `round_ps_to_posix_epoch_time` below is
/// that arithmetic on an i128 picosecond count; this function converts the
/// SystemTime to picoseconds exactly (nanosecond resolution * 1000), both
/// for pre- and post-epoch instants (the pre-epoch branch negates the whole
/// magnitude). The differential battery (23 picosecond values + 16
/// (secs, nanos) pairs, both signs) against GHC 9.4.7 is byte-identical —
/// /tmp/opencode/db48-probe/.
pub fn utc_time_to_posix_epoch_time(t: SystemTime) -> thrift::PosixEpochTime {
    // Both branches compute the magnitude `secs * 10^12 + nanos * 10^3`
    // exactly; a pre-epoch instant negates the WHOLE magnitude (nanos is
    // unsigned inside the error's Duration), it does not keep its fraction
    // positive.
    let ps: i128 = match t.duration_since(UNIX_EPOCH) {
        Ok(d) => (d.as_secs() as i128) * 1_000_000_000_000 + (d.subsec_nanos() as i128) * 1000,
        Err(e) => {
            let d = e.duration();
            -((d.as_secs() as i128) * 1_000_000_000_000 + (d.subsec_nanos() as i128) * 1000)
        }
    };
    round_ps_to_posix_epoch_time(ps)
}

/// The `round . utcTimeToPOSIXSeconds` core: GHC.Real's default `round`
/// over a picosecond-resolution `Fixed` value — `properFraction` is
/// floor/floor-mod, the fractional part decides `n`, `n+1`, or (exactly
/// .5) the even neighbour. Every step is exact integer arithmetic, so the
/// GHC/Rust results agree on the full battery in
/// /tmp/opencode/db48-probe/. The `as i64` cast is the source's
/// `round :: NominalDiffTime -> Int64`; at the +999999999ns-past-i64::MAX
/// boundary it wraps to i64::MIN exactly as the source's Int64 `round`
/// does (probe-verified; bug-for-bug parity).
fn round_ps_to_posix_epoch_time(ps: i128) -> thrift::PosixEpochTime {
    const PICOSECONDS_PER_SECOND: i128 = 1_000_000_000_000;
    let secs = ps.div_euclid(PICOSECONDS_PER_SECOND);
    let rem = ps.rem_euclid(PICOSECONDS_PER_SECOND);
    let rounded = if rem * 2 < PICOSECONDS_PER_SECOND {
        secs
    } else if rem * 2 > PICOSECONDS_PER_SECOND {
        secs + 1
    } else if secs % 2 == 0 {
        secs
    } else {
        secs + 1
    };
    rounded as thrift::PosixEpochTime
}

/// `posixEpochTimeToUTCTime = toUTCTime . posixEpochTimeToTime` (:152-153)
/// — the port composes the same two steps: `posix_epoch_time_to_time`
/// (identity on i64) then the TimeSec seam's `to_utc_time` (exact whole
/// seconds, NEGATIVE epochs included — `posixSecondsToUTCTime` is total).
pub fn posix_epoch_time_to_utc_time(p: thrift::PosixEpochTime) -> SystemTime {
    to_utc_time(posix_epoch_time_to_time(p))
}

/// `posixEpochTimeToTime = Time . fromIntegral . unPosixEpochTime`
/// (:155-156): both sides are i64, so the conversion is the identity —
/// including negative epochs.
pub fn posix_epoch_time_to_time(p: thrift::PosixEpochTime) -> Time {
    Time(p)
}

// ---------------------------------------------------------------------------
// ACLMode (Meta.hs:158-165)
// ---------------------------------------------------------------------------

/// ACL mode for database operations (:158-164).
///
/// Derive note: source `deriving (Eq, Show)` -> `PartialEq`/`Eq`/`Debug`;
/// `Clone`/`Copy` added per the fieldless-sum convention
/// (StackedDbOpts, backend/types.rs:377-380) — the value is passed by
/// value through the readers below. The `Display` impl (RULE §2
/// deriving-Show row) renders the SOURCE constructor names, the exact text
/// GHC's `deriving Show` would produce.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AclMode {
    /// ACLs are disabled (no glean.acl property)
    AclDisabled,
    /// Basic ACL support (glean.acl = "enabled")
    AclActive,
    /// Strict mode (error on facts without units)
    AclEnforced,
    /// Permissive mode (no directory inheritance)
    AclPermissive,
    /// ACL property set but column family is missing/broken
    AclBroken,
}

impl fmt::Display for AclMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            AclMode::AclDisabled => "ACLDisabled",
            AclMode::AclActive => "ACLActive",
            AclMode::AclEnforced => "ACLEnforced",
            AclMode::AclPermissive => "ACLPermissive",
            AclMode::AclBroken => "ACLBroken",
        };
        f.write_str(name)
    }
}

// ---------------------------------------------------------------------------
// showACLMode (Meta.hs:167-173)
// ---------------------------------------------------------------------------

/// "Convert ACL mode to a human-readable description for logging" (:167) —
/// five byte-exact strings (:169-173).
pub fn show_acl_mode(mode: AclMode) -> String {
    match mode {
        AclMode::AclDisabled => "ACL: disabled".to_string(),
        AclMode::AclActive => "ACL: active".to_string(),
        AclMode::AclEnforced => "ACL: enforced (strict mode)".to_string(),
        AclMode::AclPermissive => "ACL: permissive (no directory inheritance)".to_string(),
        AclMode::AclBroken => "ACL: missing (column family not yet written)".to_string(),
    }
}

// ---------------------------------------------------------------------------
// getACLMode (Meta.hs:175-182)
// ---------------------------------------------------------------------------

/// "Get the ACL mode from database properties" (:175). The source's
/// `HashMap.lookup "glean.acl"` (:177) becomes `BTreeMap::get` on the
/// pinned DatabaseProperties image; the five cases (four values, the `_`
/// default) are the source's (:178-182).
pub fn get_acl_mode(props: &thrift::DatabaseProperties) -> AclMode {
    match props.get("glean.acl").map(String::as_str) {
        Some("enabled") => AclMode::AclActive,
        Some("enforced") => AclMode::AclEnforced,
        Some("permissive") => AclMode::AclPermissive,
        Some("broken") => AclMode::AclBroken,
        _ => AclMode::AclDisabled,
    }
}

// ---------------------------------------------------------------------------
// isACLEnabled / isACLEnforced / isACLPermissive (Meta.hs:184-194)
// ---------------------------------------------------------------------------

/// "Check if ACLs are enabled (any mode except disabled)" (:184-186).
pub fn is_acl_enabled(props: &thrift::DatabaseProperties) -> bool {
    get_acl_mode(props) != AclMode::AclDisabled
}

/// "Check if ACLs are in enforced mode" (:188-190).
pub fn is_acl_enforced(props: &thrift::DatabaseProperties) -> bool {
    get_acl_mode(props) == AclMode::AclEnforced
}

/// "Check if ACLs are in permissive mode" (:192-194).
pub fn is_acl_permissive(props: &thrift::DatabaseProperties) -> bool {
    get_acl_mode(props) == AclMode::AclPermissive
}

// Marker breakdown (closed set — one class present): one port todo, the
// Internal.Types home question above (a single canonical flag line at the
// architecture-call block); no source-defect reproductions (the port is
// faithful, incl. the `dbAge` saturation which is a documented
// representation effect, not a source defect); no performance deferrals
// (nothing here is hot-path); no inventory
// divergence (no governing row — see the INVENTORY block).
//
// PORT STATUS: confidence=high todos=1
