// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// RUST-ONLY SCAFFOLDING (no Haskell body yet). This file is the directory
// module for the Schema/ namespace: it declares 'pub mod compute_ids;' so
// that rust/glean-db/src/database/schema/compute_ids.rs (row 66's port of
// Schema/ComputeIds.hs) resolves at crate::database::schema::compute_ids.
//
// TWO ROLES AT THIS PATH (the catalog.rs and backup.rs precedent).
// migration/manifest.tsv line 98 maps glean/db/Glean/Database/Schema.hs to
// THIS SAME path, rust/glean-db/src/database/schema.rs (crate glean-db), so
// a later unit will add the port body of Schema.hs here, beside the module
// declarations. This unit (row 66) creates only the scaffolding; it does
// NOT port Schema.hs and adds no body.
//
// INVENTORY: no Haskell file maps to this path. The ComputeIds.hs row
// (inventory.tsv:27) governs compute_ids.rs; Schema.hs's own rows govern the
// future body at this path (row 98). This scaffolding adds no governed
// surface.
//
// WIRING: database.rs declares 'pub mod schema;' after
// 'pub mod batch_location;'; this file in turn declares
// 'pub mod compute_ids;'.
//
// ROW 66'S WIRING CONTRACT — CONSUMED
// Row 66 (Schema/ComputeIds.hs, manifest line 66) appends
// 'pub mod compute_ids;' below, resolving .../schema/compute_ids.rs.
//
// ROW 89'S WIRING CONTRACT (forward)
// Row 89 (Schema/Types.hs, manifest line 89) will append 'pub mod types;'
// below, resolving .../schema/types.rs. This unit does not declare it: the
// file does not exist yet, and the declaration belongs to the unit that
// creates it.
//
// Coverage: one 'pub mod' line (compute_ids.rs, row 66) and nothing else;
// no re-exports. Marker breakdown: zero of every class — no port todos (the
// types line is a documented forward reference, not a deferral), no
// source-defect reproductions (no body yet), no performance deferrals
// (declarations only), no inventory divergence (no Haskell counterpart
// governs this scaffolding), no SAFETY notes (no unsafe code).

pub mod compute_ids;

// PORT STATUS: confidence=high todos=0
