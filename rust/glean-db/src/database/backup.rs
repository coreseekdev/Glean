// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// RUST-ONLY SCAFFOLDING (no Haskell body yet). This file is the directory
// module for the Backup/ namespace: it declares `pub mod backend;` so that
// rust/glean-db/src/database/backup/backend.rs (row 56's port of
// Backup/Backend.hs) resolves at `crate::database::backup::backend`.
//
// TWO ROLES AT THIS PATH (the catalog.rs and query/thrift.rs precedent).
// migration/manifest.tsv line 125 maps glean/db/Glean/Database/Backup.hs to
// THIS SAME path, rust/glean-db/src/database/backup.rs (crate glean-db), so
// a later unit will add the port body of Backup.hs here, beside the module
// declarations — exactly as src/database/catalog.rs carries Catalog.hs's
// body after its `pub mod filter;`/`pub mod store;` lines. This unit
// (row 56) creates only the scaffolding; it does NOT port Backup.hs and
// adds no body.
//
// INVENTORY: Backup.hs is governed by migration/inventory.protocol.tsv:26
// (EXC_TAXONOMY for doBackup/restore-handler) and
// migration/inventory.sites.tsv rows 198-214 and 598-599. Those rows govern
// the FUTURE body at this path, not this scaffolding, which has no Haskell
// counterpart of its own. (Backup/Backend.hs, row 56's source, is zero-hit
// across all six inventory files — see backend.rs's INVENTORY block.)
//
// WIRING: database.rs declares `pub mod backup;` after
// `pub mod acl_config;`; this file in turn declares `pub mod backend;`.
//
// Coverage: one `pub mod` line (backend.rs, row 56) and nothing else; no
// re-exports. Marker breakdown: zero of every class — no port todos (the
// wiring is complete), no source-defect reproductions (no body yet), no
// performance deferrals (declarations only), no inventory divergence (no
// Haskell counterpart governs this scaffolding), no SAFETY notes (no
// unsafe code).

pub mod backend;

// PORT STATUS: confidence=high todos=0
