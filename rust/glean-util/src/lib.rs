// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

//! `glean-util` — the Rust image of the Haskell `Glean.Util` package
//! area (glean/util/Glean/**, plus the two github/ stragglers
//! TestRunner.hs and Glean/Init.hsc): the leaf utility crate.
//!
//! RUST-ONLY SCAFFOLDING: this file has NO Haskell counterpart (Haskell
//! needs no crate-root file). Created by this crate's first shipped
//! decision-record unit (rust/glean-util/src/util/some.rs,
//! migration/manifest.tsv line 38) following the glean-typed precedent
//! (rust/glean-typed/src/lib.rs + Cargo.toml, commit 82b4437): the
//! earliest unit of a crate owns its crate scaffolding. The sibling
//! that shipped before this convention was recorded
//! (src/util/vector.rs, manifest line 27) owns no scaffolding — this
//! file retroactively wires it (`pub mod vector` below). Later
//! glean-util units add their own `pub mod` lines here and in
//! src/util.rs as they land.
//!
//! The Haskell area has no `Glean.Util` root module (only Glean/Util/
//! *.hs and Glean/Impl/*.hs), so unlike glean-typed there is no
//! translated crate-root unit — src/util.rs is a pure wiring file.

pub mod util;

// Coverage: one `pub mod` and nothing else — no lib-level re-exports (a
// `pub use` here would fork the namespace; the facades are each
// submodule's own, RULE §4 "no glob re-exports across crates").
//
// Marker breakdown (closed set — zero of every class): no port todos
// (the crate-root wiring is complete with this file), no source-defect
// reproductions (no Haskell source), no performance deferrals (no
// code), no inventory divergence (no inventory row can govern a file
// with no Haskell counterpart).
//
// PORT STATUS: confidence=medium todos=0
