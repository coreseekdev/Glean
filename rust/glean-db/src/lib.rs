// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

//! `glean-db` — the Rust image of the Haskell database/back-end area:
//! glean/backend-api (Glean.Backend.*), glean/backend-local
//! (Glean.Backend.Local / Logging / Dump), glean/db (Glean.Database.*,
//! Glean.Query.*, Glean.Logger.*, Glean.Write.*), the Glean-owned Haxl
//! datasource (glean/haxl/Haxl/DataSource/Glean*), and the github/
//! stragglers mapped into this crate — the 90 units the depmap assigns
//! to glean-db (migration/depmap/crates.txt:6; migration/manifest.tsv
//! rows with crate glean-db).
//!
//! RUST-ONLY SCAFFOLDING: this file has NO Haskell counterpart (Haskell
//! needs no crate-root file, and the area has no `Glean.Backend` /
//! `Glean.Database` root module either — glean/backend-api/Glean/
//! contains only Backend/, Query/, Write/). Created by this crate's
//! first shipped unit (rust/glean-db/src/backend/types.rs, migration/
//! manifest.tsv line 41) following the glean-util / glean-typed
//! precedent (rust/glean-util/src/lib.rs, rust/glean-typed/src/lib.rs,
//! commit 82b4437): the earliest unit of a crate owns its crate
//! scaffolding. Later units append their own `pub mod` lines here as
//! they land (`database`, `haxl`, `query`, `write`, `logger`, and the
//! github stragglers' paths).
//!
//! Forward-reference note: `backend/types.rs` names three same-crate
//! paths that no unit has shipped yet — `crate::database::types::Env`
//! (row 104), `crate::backend::retry` / `crate::backend::logging`
//! (rows 43 / 148), and `crate::haxl::engine::{GleanGetFetcher,
//! GleanQueryFetcher}` (row 146). The first three are ordinary
//! future-unit paths; `crate::haxl::engine` is special: the Haxl
//! framework core is an EXTERNAL Haskell package with no manifest row,
//! so its Rust image is a Rust-only module owed by row 146
//! (src/haxl/data_source/glean/common.rs) — see the Haxl seam block in
//! backend/types.rs for the contract that module must satisfy.

pub mod backend;

// Coverage: one `pub mod` and nothing else — no lib-level re-exports (a
// `pub use` here would fork the namespace the siblings'
// `crate::backend::…` paths already presume; each submodule is its own
// facade, RULE §4 "no glob re-exports").
//
// Marker breakdown (closed set — zero of every class): no port todos
// (the crate-root wiring is complete for the one shipped file; the
// forward module lines above name their owning units in prose), no
// source-defect reproductions (no Haskell source), no performance
// deferrals (no code), no inventory divergence (no inventory row can
// govern a file with no Haskell counterpart).
//
// PORT STATUS: confidence=medium todos=0
