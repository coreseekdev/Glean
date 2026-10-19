// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// RUST-ONLY SCAFFOLDING (no Haskell body yet). This file is the directory
// module for the Logger/ namespace: it declares 'pub mod database;' so that
// rust/glean-db/src/logger/database.rs (row 74's port of
// Logger/Database.hs) resolves at crate::logger::database.
//
// TWO ROLES AT THIS PATH (the catalog.rs, backup.rs and
// database/schema.rs precedent). migration/manifest.tsv line 106 maps
// glean/db/Glean/Logger.hs to THIS SAME path, rust/glean-db/src/logger.rs
// (crate glean-db), so a later unit will add the port body of Logger.hs
// here, beside the module declarations. This unit (row 74) creates only the
// scaffolding; it does NOT port Logger.hs and adds no body.
//
// INVENTORY: no Haskell body maps to this path yet. Logger.hs's own rows
// govern the future body at this path (row 106); the logger-sink carve-out
// (inventory.tsv:136, dyn allowed for plugin sinks) governs the traits in
// the child files. This scaffolding adds no governed surface.
//
// WIRING: lib.rs declares 'pub mod logger;' after 'pub mod database;'; this
// file in turn declares 'pub mod database;'.
//
// ROW 74'S WIRING CONTRACT — CONSUMED
// Row 74 (Logger/Database.hs, manifest line 74) appends
// 'pub mod database;' below, resolving .../logger/database.rs.
//
// ROW 75'S WIRING CONTRACT (forward)
// Row 75 (Logger/Server.hs, manifest line 75) will append 'pub mod server;'
// below, resolving .../logger/server.rs. This unit does not declare it: the
// file does not exist yet, and the declaration belongs to the unit that
// creates it.
//
// Coverage: one 'pub mod' line (database.rs, row 74) and nothing else; no
// re-exports. Marker breakdown: zero of every class — no port todos (the
// server line is a documented forward reference, not a deferral), no
// source-defect reproductions (no body yet), no performance deferrals
// (declarations only), no inventory divergence (no Haskell counterpart
// governs this scaffolding), no SAFETY notes (no unsafe code).

pub mod database;

// PORT STATUS: confidence=high todos=0
