// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// RUST-ONLY SCAFFOLDING (no Haskell body). This file is the directory
// module for the Catalog/Local/ namespace: it declares 'pub mod files;' so
// that rust/glean-db/src/database/catalog/local/files.rs (row 64's port of
// Catalog/Local/Files.hs) resolves at
// crate::database::catalog::local::files.
//
// INVENTORY: no Haskell file maps to this path — the Catalog/Local/
// directory has no Local.hs. A grep for 'Catalog/Local' across
// migration/inventory*.tsv hits four lines: the Util/IO HandleIO row
// (inventory.tsv:111 == inventory.protocol.tsv:14), whose evidence names
// Files.hs:62/:94 as readFileContents consumers, and the Catalog
// STM_GROUP row (inventory.tsv:21 == inventory.stm.tsv:23), whose evidence
// names Catalog/Local/Memory.hs:22,28-45 as the memory-backend clause.
// Neither row's source file is in this directory: the first governs row 63
// (util/io.rs), the second governs row 54 (catalog.rs). No row governs this
// scaffolding.
//
// WIRING: catalog.rs declares 'pub mod local;' after 'pub mod store;'; this
// file in turn declares 'pub mod files;'.
//
// ROW 64'S WIRING CONTRACT — CONSUMED
// Row 64 (Catalog/Local/Files.hs, manifest line 64) appends
// 'pub mod files;' below, resolving .../catalog/local/files.rs. Files is
// the payload of store.rs's CatalogStore::Files variant, so this
// declaration also closes that forward reference; the Memory half stays
// open until row 65.
//
// ROW 65'S WIRING CONTRACT (forward)
// Row 65 (Catalog/Local/Memory.hs, manifest line 65) will append
// 'pub mod memory;' below, resolving .../catalog/local/memory.rs and
// closing CatalogStore::Memory. This unit does not declare it: the file
// does not exist yet, and the declaration belongs to the unit that creates
// it.
//
// Coverage: one 'pub mod' line (files.rs, row 64) and nothing else; no
// re-exports. Marker breakdown: zero of every class — no port todos (the
// Memory line is a documented forward reference, not a deferral), no
// source-defect reproductions (no body yet), no performance deferrals
// (declarations only), no inventory divergence (no Haskell counterpart
// governs this scaffolding), no SAFETY notes (no unsafe code).

pub mod files;

// PORT STATUS: confidence=high todos=0
