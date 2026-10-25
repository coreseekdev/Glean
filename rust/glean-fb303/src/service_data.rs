// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

//! RUST-ONLY SCAFFOLDING: this file has NO Haskell counterpart — the
//! Haskell area has no `ServiceData` root module (glean/github/ServiceData/
//! contains exactly Types.hs and GlobalStats.hs). It exists so the crate
//! root's `pub mod service_data;` (lib.rs — the source modules are named
//! `ServiceData.Types` / `ServiceData.GlobalStats`, so the RULE §4
//! module-header-decides path is crate::service_data::{types,global_stats})
//! resolves to the `src/service_data/` directory the manifest's naming
//! generates (glean/github/ServiceData/Types.hs ->
//! src/service_data/types.rs, line 96; GlobalStats.hs ->
//! src/service_data/global_stats.rs, line 97). Owned by the same batch as
//! the two units it wires (this is the crate's first shipped batch),
//! following the glean-typed src/query.rs and glean-util src/util.rs
//! directory-module precedents. Later units append their own `pub mod`
//! lines here.

pub mod global_stats;
pub mod types;

// Coverage: two `pub mod` lines — types (row 96, the leaf) and
// global_stats (row 97, which consumes types exactly as the source does,
// GlobalStats.hs:14 `import ServiceData.Types`) — and nothing else: no
// re-exports (each submodule is its own facade).
//
// Marker breakdown (closed set — zero of every class): no port todos (the
// wiring is complete for both shipped units), no source-defect
// reproductions (no Haskell source), no performance deferrals (no code),
// no inventory divergence (no inventory row can govern a file with no
// Haskell counterpart).
//
// PORT STATUS: confidence=high todos=0
