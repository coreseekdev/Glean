// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

//! RUST-ONLY SCAFFOLDING: this file has NO Haskell counterpart — the
//! Haskell area has no `Glean.Query` root module
//! (glean/backend-api/Glean/Query/ contains only Thrift.hs and
//! Thrift/Internal.hs). It exists so lib.rs's `pub mod query;` resolves to
//! the `src/query/` directory the manifest's RULE §4 naming generates
//! (glean/backend-api/Glean/Query/Thrift.hs -> src/query/thrift.rs,
//! migration/manifest.tsv line 47; glean/backend-api/Glean/Query/Thrift/
//! Internal.hs -> src/query/thrift/internal.rs, line 45). Owned by the
//! same unit that owns the area's first query file (row 45, src/query/
//! thrift/internal.rs), following the glean-typed src/query.rs precedent
//! (unit 44): the earliest unit of an area owns the area's scaffolding.
//!
//! ROW 47'S WIRING CONTRACT (prose, not code — the `pub mod` line belongs
//! to row 47's unit): when src/query/thrift.rs ships, its author appends
//! `pub mod thrift;` at the top of THIS file and declares `pub mod
//! internal;` in src/query/thrift.rs. Those two declarations are what bind
//! `crate::query::thrift::internal` (row 45's file, already on disk) to
//! the crate root; until then the path is a forward reference of the class
//! lib.rs:28-37 names for `crate::database::types::Env` and
//! `crate::haxl::engine`, and no sibling may import it.

// Coverage: zero code lines — no `pub mod` yet (row 47 appends its line
// per the contract above; this unit deliberately writes no `pub mod
// thrift;` here: the file it would name does not exist until row 47
// lands), no items, no re-exports. The file's whole job is to give
// lib.rs's `pub mod query;` a module to resolve to.
//
// Marker breakdown (closed set — zero of every class): no port todos (the
// row-47 append is named above in prose, and its unit owns it), no
// source-defect reproductions (no Haskell source), no performance
// deferrals (no code), no inventory divergence (no inventory row can
// govern a file with no Haskell counterpart — grep "Query/Thrift" across
// migration/inventory*.tsv: every hit is the Internal.hs:98,106 evidence
// inside inventory.tsv:56 and its inventory.stm.tsv:35 twin, which govern
// row 45's caches, not this file).
//
// PORT STATUS: confidence=high todos=0
