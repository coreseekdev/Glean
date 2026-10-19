// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

//! RUST-ONLY SCAFFOLDING: this file has NO Haskell counterpart — the
//! Haskell area has no `Glean.Impl` root module (glean/util/Glean/Impl/
//! holds four sources; only MemoryReader.hs has shipped so far —
//! ConfigProvider.hs, ThriftService.hs and TestConfigProvider.hs follow). It
//! exists so lib.rs's
//! `pub mod r#impl;` resolves to the `src/impl/` directory the manifest's
//! RULE §4 naming generates (glean/util/Glean/Impl/MemoryReader.hs ->
//! src/impl/memory_reader.rs, migration/manifest.tsv line 68).
//!
//! THE RAW IDENTIFIER: `impl` is a Rust keyword, so the crate-root line is
//! `pub mod r#impl;` and consumers spell the path the same way —
//! `glean_util::r#impl::memory_reader::total_mem_capacity_kb()`. The module's
//! NAME is still `impl`; the `r#` prefix is only the escape hatch required
//! to write it. This is the migration's first keyword-named module
//! (`grep -rn 'pub mod r#' rust/` was empty before this unit), so the
//! spelling is recorded here for the three later Impl/ units.
//!
//! ROW 68'S WIRING CONTRACT — CONSUMED (row 68 shipped
//! src/impl/memory_reader.rs): that unit appended `pub mod r#impl;` in
//! lib.rs and this file's `pub mod memory_reader;`. Rows 156
//! (Impl/ConfigProvider.hs), 166 (Impl/ThriftService.hs) and 384
//! (Impl/TestConfigProvider.hs) append their own `pub mod` lines here.

// glean/util/Glean/Impl/MemoryReader.hs -> src/impl/memory_reader.rs
// (manifest line 68)
pub mod memory_reader;

// Coverage: one `pub mod` line wiring the only shipped Impl/ file
// (impl/memory_reader.rs) and nothing else — no re-exports (each submodule
// is its own facade).
//
// Marker breakdown (closed set — zero of every class): no port todos
// (wiring is complete for every shipped file), no source-defect
// reproductions (no Haskell source), no performance deferrals (no code),
// no inventory divergence (no inventory row can govern a file with no
// Haskell counterpart).
//
// PORT STATUS: confidence=high todos=0
