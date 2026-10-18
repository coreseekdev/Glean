// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

//! RUST-ONLY SCAFFOLDING (for now): this file is the directory module for
//! the `Glean.Database.Catalog/` subdirectory. Haskell's
//! `Glean.Database.Catalog` is both a module (Catalog.hs) and a namespace
//! (the Catalog/ directory), so the single Rust path
//! src/database/catalog.rs serves two roles: it declares the directory's
//! children for row 49's filter.rs today, and it is the port target of
//! Catalog.hs (migration/manifest.tsv line 54). A file can be both a port
//! target and a directory module — the src/query/thrift.rs precedent (row
//! 47's port of Thrift.hs declares `pub mod internal;` at
//! query/thrift.rs:369). Reconciling the two roles at this path is row
//! 54's call, not row 49's; row 49 records only the `pub mod filter;`
//! line it owes.
//!
//! Created by the unit that owns the area's first shipped file
//! (migration/manifest.tsv line 49,
//! glean/db/Glean/Database/Catalog/Filter.hs ->
//! rust/glean-db/src/database/catalog/filter.rs), following the
//! write.rs/database.rs precedent (the earliest unit of an area owns the
//! scaffolding and declares its own direct child immediately).

pub mod filter;

// Coverage: one `pub mod` line wiring this area's first shipped file
// (database/catalog/filter.rs, row 49) and nothing else — no re-exports
// (the submodule is its own facade).
//
// Marker breakdown (closed set — zero of every class): no port todos, no
// source-defect reproductions (row 49's directory-module role has no
// Haskell source; Catalog.hs's own port is row 54's), no performance
// deferrals (no runtime code), no inventory divergence (no inventory row
// governs row 49's scaffolding — the "Catalog" hits in
// migration/inventory*.tsv name Catalog.hs and Catalog/Local/Files.hs and
// belong to those files' units; see filter.rs's INVENTORY block for the
// grep).
//
// PORT STATUS: confidence=high todos=0
