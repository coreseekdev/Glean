// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

//! RUST-ONLY SCAFFOLDING: this file has NO Haskell counterpart — the
//! Haskell area has no `Glean.Query` root module (glean/typed/Glean/Query/
//! contains only Angle.hs). It exists so the crate root's `pub mod query;`
//! (lib.rs — the source module `Glean.Query.Angle` is a SIBLING of
//! Glean.Typed, so the wiring hangs off the crate root, not inside
//! crate::typed; RULE §4 module-header-decides) resolves to the
//! `src/query/` directory the manifest's RULE §4 naming generates
//! (glean/typed/Glean/Query/Angle.hs -> src/query/angle.rs, i.e. the
//! module path crate::query::angle; migration/manifest.tsv line 44).
//! Owned by the same unit that owns src/query/angle.rs (manifest line
//! 44), following the glean-util src/util.rs and glean-db src/backend.rs
//! precedents. Later units append their own `pub mod` lines here.

pub mod angle;

// Coverage: one `pub mod` line wiring the one shipped query file
// (query/angle.rs) and nothing else — no re-exports (the submodule is its
// own facade).
//
// Marker breakdown (closed set — zero of every class): no port todos (the
// wiring is complete for the shipped file), no source-defect
// reproductions (no Haskell source), no performance deferrals (no code),
// no inventory divergence (no inventory row can govern a file with no
// Haskell counterpart).
//
// PORT STATUS: confidence=high todos=0
