// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// RUST-ONLY SCAFFOLDING (no Haskell body). This file is the directory
// module for the Catalog/Local/ namespace: it declares 'pub mod files;' for
// rust/glean-db/src/database/catalog/local/files.rs (row 64's port of
// Catalog/Local/Files.hs) and 'pub mod memory;' for
// .../catalog/local/memory.rs (row 65's port of Catalog/Local/Memory.hs).
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
// file in turn declares 'pub mod files;' and 'pub mod memory;'.
//
// ROW 64'S WIRING CONTRACT — CONSUMED
// Row 64 (Catalog/Local/Files.hs, manifest line 64) appends
// 'pub mod files;' below, resolving .../catalog/local/files.rs. Files is
// the payload of store.rs's CatalogStore::Files variant, so this
// declaration closed that forward reference; the Memory half was closed by
// row 65 (next).
//
// ROW 65'S WIRING CONTRACT — CONSUMED
// Row 65 (Catalog/Local/Memory.hs, manifest line 65) appends
// 'pub mod memory;' below, resolving .../catalog/local/memory.rs and
// closing CatalogStore::Memory, the enum's last forward reference.
//
// Coverage: two 'pub mod' lines (files.rs, row 64; memory.rs, row 65) and
// nothing else; no re-exports. Marker breakdown: zero of every class — no
// port todos (both wiring contracts are consumed), no source-defect
// reproductions (no body), no performance deferrals (declarations only), no
// inventory divergence (no Haskell counterpart governs this scaffolding),
// no SAFETY notes (no unsafe code).

pub mod files;
pub mod memory;

// PORT STATUS: confidence=high todos=0
