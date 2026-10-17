// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

//! RUST-ONLY SCAFFOLDING: this file has NO Haskell counterpart — the
//! Haskell area has no `Glean.Util` root module (glean/util/Glean/Util/
//! contains only Some.hs, Vector.hs, …). It exists so lib.rs's
//! `pub mod util;` resolves to the `src/util/` directory the manifest's
//! RULE §4 naming generates (glean/util/Glean/Util/Some.hs ->
//! src/util/some.rs). Owned by the same unit that owns the crate
//! scaffolding (src/util/some.rs, manifest line 38). Later units append
//! their own `pub mod` lines here.

pub mod some;
pub mod vector;

// Coverage: two `pub mod` lines wiring every glean-util file shipped so
// far (util/some.rs, util/vector.rs) and nothing else — no re-exports
// (each submodule is its own facade).
//
// Marker breakdown (closed set — zero of every class): no port todos
// (wiring is complete for all shipped files), no source-defect
// reproductions (no Haskell source), no performance deferrals (no
// code), no inventory divergence (no inventory row can govern a file
// with no Haskell counterpart).
//
// PORT STATUS: confidence=medium todos=1
