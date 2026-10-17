// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

//! RUST-ONLY SCAFFOLDING: this file has NO Haskell counterpart — the
//! Haskell area has no `Glean.Backend` root module
//! (glean/backend-api/Glean/ contains only Backend/, Query/, Write/;
//! glean/backend-local/Glean/Backend/ only Local.hs and Logging.hs). It
//! exists so lib.rs's `pub mod backend;` resolves to the `src/backend/`
//! directory the manifest's RULE §4 naming generates
//! (glean/backend-api/Glean/Backend/Types.hs -> src/backend/types.rs).
//! Owned by the same unit that owns the crate scaffolding
//! (src/backend/types.rs, migration/manifest.tsv line 41). Later units
//! append their own `pub mod` lines here as they land
//! (glean/backend-api/Glean/Backend/Retry.hs -> `retry`, row 43;
//! glean/backend-local/Glean/Backend/Local.hs -> `local`, row 147;
//! glean/backend-local/Glean/Backend/Logging.hs -> `logging`, row 148).

pub mod retry;
pub mod types;

// Coverage: two `pub mod` lines wiring the backend files shipped so far
// (backend/retry.rs, row 43, before types.rs alphabetically) and nothing
// else — no re-exports (each submodule is its own facade).
//
// Marker breakdown (closed set — zero of every class): no port todos
// (the wiring is complete for the shipped file; the future lines above
// name their owning units in prose), no source-defect reproductions (no
// Haskell source), no performance deferrals (no code), no inventory
// divergence (no inventory row can govern a file with no Haskell
// counterpart).
//
// PORT STATUS: confidence=medium todos=0
