// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

//! `glean-typed` — the Rust image of the Haskell `Glean.Typed` package
//! area (glean/typed/Glean/*.hs): support for working with strongly-typed
//! Glean data.
//!
//! RUST-ONLY SCAFFOLDING: this file has NO Haskell counterpart (Haskell
//! needs no crate-root file). The module `Glean.Typed` itself is ported
//! at `crate::typed` (rust/glean-typed/src/typed.rs, migration/
//! manifest.tsv line 37, crate glean-typed — the unit that also owns this
//! lib.rs and the crate's Cargo.toml; cf. id.rs:113-115's and
//! build.rs:148-152's MODULE-WIRING notes). This file exists so the
//! shipped siblings' `use crate::typed::…` paths resolve. Nothing belongs
//! here beyond the module declaration: the public facade lives in
//! `crate::typed` — no lib-level `pub use`.

pub mod typed;

// Coverage: one `pub mod` and nothing else — no lib-level re-exports (a
// `pub use` here would fork the namespace the siblings' `crate::typed::…`
// paths already presume; the facade is `crate::typed`'s alone).
//
// Marker breakdown (closed set — zero of every class): no port todos (the
// crate-root wiring is complete with this file), no source-defect
// reproductions (no Haskell source to reproduce from), no performance
// deferrals (no code), no inventory divergence (no inventory row can
// govern a file with no Haskell counterpart — grep "Glean/Typed.hs"
// across migration/inventory*.tsv: zero rows).
//
// PORT STATUS: confidence=high todos=0
