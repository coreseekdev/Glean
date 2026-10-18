// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

//! RUST-ONLY SCAFFOLDING: this file has NO Haskell counterpart — the
//! Haskell area has no `Glean.Write` root module
//! (glean/backend-api/Glean/Write/ contains only SendBatch.hs). It exists so
//! lib.rs's `pub mod write;` resolves to the `src/write/` directory the
//! manifest's RULE §4 naming generates (glean/backend-api/Glean/Write/
//! SendBatch.hs -> src/write/send_batch.rs, migration/manifest.tsv line 46;
//! glean/db/Glean/Write/JSON.hs -> src/write/json.rs, line 115). Owned by
//! the same unit that owns the area's first write file (row 46, src/write/
//! send_batch.rs), following the query.rs precedent (row 45, src/query.rs)
//! and the glean-typed src/query.rs precedent (unit 44): the earliest unit
//! of an area owns the area's scaffolding. Because row 46's file is a
//! DIRECT child of this directory module (unlike row 45's
//! query/thrift/internal.rs, whose parent src/query/thrift.rs waits for
//! row 47), its `pub mod send_batch;` line is declared here immediately —
//! the glean-typed src/query.rs:20 `pub mod angle;` shape.
//!
//! ROW 115'S WIRING CONTRACT (prose, not code — the `pub mod` line belongs
//! to row 115's unit): when src/write/json.rs ships, its author appends
//! `pub mod json;` at the top of THIS file. That declaration is what binds
//! `crate::write::json` (row 115's file) to the crate root; until then the
//! path is a forward reference, and no sibling may import it.

pub mod send_batch;

// Coverage: one `pub mod` line wiring the area's first shipped file
// (write/send_batch.rs, row 46) and nothing else — no re-exports (the
// submodule is its own facade). Row 115 appends its own line per the
// contract above; this unit deliberately writes no `pub mod json;` here:
// the file it would name does not exist until row 115 lands.
//
// Marker breakdown (closed set — zero of every class): no port todos (the
// row-115 append is named above in prose, and its unit owns it), no
// source-defect reproductions (no Haskell source), no performance
// deferrals (the one declaration is not runtime code), no inventory
// divergence (no inventory row can govern a file with no Haskell
// counterpart — grep "SendBatch" across migration/inventory*.tsv finds
// nothing, and grep "Write/JSON" finds only inventory.tsv:43 ==
// inventory.stm.tsv:30, the JSON FactBuilder row, which governs row 115's
// future send path, not this file).
//
// PORT STATUS: confidence=high todos=0
