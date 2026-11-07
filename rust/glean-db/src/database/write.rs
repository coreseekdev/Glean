// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// RUST-ONLY SCAFFOLDING (no Haskell counterpart). This file is the
// directory module for the Write/ namespace: it declares `pub mod queue;`
// so that rust/glean-db/src/database/write/queue.rs (row 105's port of
// glean/db/Glean/Database/Write/Queue.hs, manifest line 105) resolves at
// `crate::database::write::queue`. The Haskell namespace has no root module
// (glean/db/Glean/Database/Write/ holds Queue.hs and Batch.hs only), so this
// path carries no port body of its own.
//
// OWNERSHIP: row 105 (the area's first shipped file) owns this scaffolding,
// following the backup.rs/catalog.rs directory-module precedent. The wiring
// rule is HANDOFF.md §2.4's direct-child wiring clause: the two lines that
// bind this level are database.rs's `pub mod write;` (row 105's append,
// recorded CONSUMED there) and this file's `pub mod queue;`.
//
// PENDING CHILD: migration/manifest.tsv line 112 maps
// glean/db/Glean/Database/Write/Batch.hs to
// rust/glean-db/src/database/write/batch.rs — row 112's unit will append
// `pub mod batch;` here when it ships (the row-61 mock.rs shape in
// backup.rs). Writes.hs maps to src/database/writes.rs (manifest line 116),
// a DIRECT child of database/, not of this directory module.
//
// WIRING: database.rs declares `pub mod write;` (row 105's append); this
// file declares `pub mod queue;`.
//
// Coverage: one `pub mod` line (queue.rs, row 105) and nothing else; no
// re-exports. Marker breakdown (closed set — zero of every class): no port
// todos (the wiring is complete; batch.rs is a documented forward
// reference, not a deferral), no source-defect reproductions (no body
// here), no performance deferrals (declarations only), no inventory
// divergence (no Haskell counterpart governs this scaffolding), no SAFETY
// notes (no unsafe code).

pub mod queue;

// PORT STATUS: confidence=high todos=0
