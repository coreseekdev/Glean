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
//! glean-util units add their own `pub mod` lines here, in src/util.rs
//! and in src/impl.rs as they land.
//!
//! The Haskell area has no `Glean.Util` root module (only Glean/Util/
//! *.hs and Glean/Impl/*.hs), so unlike glean-typed there is no
//! translated crate-root unit — src/util.rs and src/impl.rs are pure
//! wiring files.
//!
//! ROW 68'S WIRING CONTRACT — CONSUMED (row 68 shipped
//! src/impl/memory_reader.rs): that unit appended `pub mod r#impl;` HERE
//! and created src/impl.rs (the Impl/ subdirectory's directory module,
//! Rust-only scaffolding for that level) declaring
//! `pub mod memory_reader;`. Those two lines bind
//! `crate::r#impl` and `crate::r#impl::memory_reader`; both paths now
//! resolve. `impl` is a Rust keyword, so the crate-root line uses the
//! RAW IDENTIFIER — the module's name is `impl` and every path to it
//! (inside this crate and from consumers) must spell `r#impl`. This is
//! the migration's first keyword-named module. Rows 156
//! (Impl/ConfigProvider.hs), 166 (Impl/ThriftService.hs) and 384
//! (Impl/TestConfigProvider.hs) append their own `pub mod` lines in
//! src/impl.rs.

// glean/util/Glean/Impl/MemoryReader.hs -> src/impl/memory_reader.rs
// (manifest line 68). The raw identifier is required: `impl` is a keyword.
pub mod r#impl;

pub mod util;

// Coverage: two `pub mod` lines — r#impl (row 68's directory module,
// which declares `pub mod memory_reader;`) and util (the crate's first
// area) — and nothing else: no lib-level re-exports (a `pub use` here
// would fork the namespace; the facades are each submodule's own, RULE §4
// "no glob re-exports across crates"). Row 68's append is now CONSUMED
// (past tense above).
//
// Marker breakdown (closed set — zero of every class): no port todos
// (the crate-root wiring is complete for both areas), no source-defect
// reproductions (no Haskell source), no performance deferrals (no
// code), no inventory divergence (no inventory row can govern a file
// with no Haskell counterpart).
//
// PORT STATUS: confidence=medium todos=0
