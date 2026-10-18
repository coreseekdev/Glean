// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

//! RUST-ONLY SCAFFOLDING: this file has NO Haskell counterpart — the
//! Haskell area has no `Glean.Database` root module (glean/db/Glean/
//! contains only Database/, Logger/, Logger.hs, Query/, Write/; there is no
//! Database.hs). It exists so lib.rs's `pub mod database;` resolves to the
//! `src/database/` directory the manifest's RULE §4 naming generates
//! (glean/db/Glean/Database/Meta.hs -> src/database/meta.rs,
//! migration/manifest.tsv line 48; glean/db/Glean/Database/Catalog/
//! Filter.hs -> src/database/catalog/filter.rs, line 49; Catalog/Store.hs
//! -> src/database/catalog/store.rs, line 50; Repo.hs -> src/database/
//! repo.rs, line 52; Exception.hs -> src/database/exception.rs, line 53;
//! Catalog.hs -> src/database/catalog.rs, line 54; Catalog/Local/{Files,
//! Memory}.hs -> src/database/catalog/local/{files,memory}.rs, lines
//! 64-65; …). Owned by the same unit that owns the area's first write file
//! (row 48, src/database/meta.rs), following the query.rs precedent (row
//! 45, src/query.rs) and the write.rs precedent (row 46, src/write.rs):
//! the earliest unit of an area owns the area's scaffolding. Because row
//! 48's file is a DIRECT child of this directory module, its
//! `pub mod meta;` line is declared here immediately — the
//! write.rs:29 `pub mod send_batch;` shape.
//!
//! ROW 49'S WIRING CONTRACT (prose, not code — the `pub mod` lines belong
//! to row 49's unit; migration/HANDOFF.md §2.4 pins the same two edits):
//! when src/database/catalog/filter.rs ships, its author appends
//! `pub mod catalog;` at the top of THIS file AND creates
//! src/database/catalog.rs (the Catalog/ subdirectory's directory module —
//! Rust-only scaffolding for that level) declaring `pub mod filter;`
//! there. Those two lines are what bind `crate::database::catalog` and
//! `crate::database::catalog::filter` to the crate root; until then both
//! paths are forward references, and no sibling may import them. NOTE for
//! the reader of the manifest: manifest line 54 maps
//! glean/db/Glean/Database/Catalog.hs to the same src/database/catalog.rs
//! path — a file can be both a port target and a directory module, the
//! src/query/thrift.rs precedent (row 47's port of Thrift.hs declares
//! `pub mod internal;` at query/thrift.rs:369 for row 45's deeper file);
//! reconciling the two roles at that path is row 54's call, not this
//! unit's, and this contract records only the two lines row 49 owes.

pub mod meta;

// Coverage: one `pub mod` line wiring the area's first shipped file
// (database/meta.rs, row 48) and nothing else — no re-exports (the
// submodule is its own facade). Row 49 appends its own `pub mod catalog;`
// line per the contract above; this unit deliberately writes no such line
// here: the file it would name (src/database/catalog.rs) does not exist
// until row 49 lands.
//
// Marker breakdown (closed set — zero of every class): no port todos (the
// row-49 append is named above in prose, and its unit owns it), no
// source-defect reproductions (no Haskell source), no performance
// deferrals (the one declaration is not runtime code), no inventory
// divergence (no inventory row can govern a file with no Haskell
// counterpart: the "Database/Catalog" grep hits across
// migration/inventory*.tsv all name Catalog.hs — its STM_GROUP rows
// (inventory.tsv:21 == inventory.stm.tsv:23), its sites.tsv rows
// (:215-232, :602-605) — or the Util/IO HandleIO row's
// Catalog/Local/Files.hs consumer evidence (inventory.tsv:111 ==
// inventory.protocol.tsv:14); none names this scaffolding, and
// "Glean.Database" finds no row at all).
//
// PORT STATUS: confidence=high todos=0
