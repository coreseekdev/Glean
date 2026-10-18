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
//! ROW 47'S WIRING CONTRACT — CONSUMED (row 47 shipped): when
//! src/query/thrift.rs landed (row 47), its author appended
//! `pub mod thrift;` at the top of THIS file (the line below) and declared
//! `pub mod internal;` in src/query/thrift.rs. Those two declarations —
//! both now on disk — are what bind `crate::query::thrift::internal`
//! (row 45's file) to the crate root; the path is no longer a forward
//! reference of the class lib.rs:28-37 names for
//! `crate::database::types::Env` and `crate::haxl::engine`, and siblings
//! may now import both.

pub mod thrift;

// Coverage: one `pub mod` line wiring the area's second shipped file
// (query/thrift.rs, row 47 — which declares `pub mod internal;` and so
// also wires row 45's query/thrift/internal.rs) and nothing else — no
// re-exports (each submodule is its own facade). The append promised in
// prose above has shipped; nothing remains deferred here.
//
// Marker breakdown (closed set — zero of every class): no port todos (the
// row-47 append is done — it is the `pub mod` line above), no
// source-defect reproductions (no Haskell source), no performance
// deferrals (the declarations are not runtime code), no inventory
// divergence (no inventory row can govern a file with no Haskell
// counterpart — grep "Query/Thrift" across migration/inventory*.tsv:
// every hit is the Internal.hs:98,106 evidence inside inventory.tsv:56
// and its inventory.stm.tsv:35 twin, which govern row 45's caches).
//
// PORT STATUS: confidence=high todos=0
