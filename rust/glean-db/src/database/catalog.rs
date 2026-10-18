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
//! children — row 49's filter.rs and row 50's store.rs — and it is the
//! port target of Catalog.hs (migration/manifest.tsv line 54). A file can
//! be both a port target and a directory module — the src/query/thrift.rs
//! precedent (row 47's port of Thrift.hs declares `pub mod internal;` at
//! query/thrift.rs:369). Reconciling the two roles at this path is row
//! 54's call; rows 49 and 50 record only the `pub mod` lines they owe.
//!
//! Created by the unit that owns the area's first shipped file
//! (migration/manifest.tsv line 49,
//! glean/db/Glean/Database/Catalog/Filter.hs ->
//! rust/glean-db/src/database/catalog/filter.rs), following the
//! write.rs/database.rs precedent (the earliest unit of an area owns the
//! scaffolding and declares its own direct child immediately); row 50
//! appended the second child, store.rs.

pub mod filter;

pub mod store;

// Coverage: two `pub mod` lines wiring this area's shipped files
// (database/catalog/filter.rs, row 49; database/catalog/store.rs, row 50)
// and nothing else — no re-exports (each submodule is its own facade).
//
// Marker breakdown (closed set — zero of every class): no port todos, no
// source-defect reproductions (rows 49/50's directory-module role has no
// Haskell source; Catalog.hs's own port is row 54's), no performance
// deferrals (no runtime code), no inventory divergence (no inventory row
// governs either scaffolding line — the "Database/Catalog" hits in
// migration/inventory*.tsv name Catalog.hs's own rows, and the Util/IO
// row (inventory.tsv:111 == inventory.protocol.tsv:14) cites
// Catalog/Local/Files.hs only as consumer evidence; none names this
// scaffolding or either child (see filter.rs's and store.rs's INVENTORY
// blocks for the enumerations and the Catalog/Store.hs zero-hit grep).
//
// PORT STATUS: confidence=high todos=0
