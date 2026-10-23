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
//!
//! ROW 86'S WIRING CONTRACT — CONSUMED (row 86 shipped): when
//! src/query/codegen/query_regs.rs landed (row 86,
//! Glean.Query.Codegen.QueryRegs), its author appended `pub mod
//! codegen;` below and created src/query/codegen.rs as the Rust-2018
//! directory-module anchor for the directory-level manifest path (the
//! src/typed.rs + src/typed/ layout). codegen.rs is manifest row 110's
//! target (Glean.Query.Codegen.hs — NOT YET SHIPPED); its `pub mod
//! query_regs;` anchor line and scaffolding header are row 86's
//! contribution, its body is row 110's obligation (ownership handoff in
//! codegen.rs's header). `crate::query::codegen::query_regs` is now
//! importable; the codegen AREA grows downward from this line.

pub mod thrift;

pub mod codegen;

// Coverage: two `pub mod` lines wiring the area's shipped files and
// nothing else — no re-exports (each submodule is its own facade):
// query/thrift.rs (row 47 — declares `pub mod internal;` and so also
// wires row 45's query/thrift/internal.rs) and query/codegen.rs (row
// 110's file, created early as row 86's directory anchor — declares `pub
// mod query_regs;` for row 86's query/codegen/query_regs.rs). The
// row-47 append promised in prose above has shipped; the row-86 append
// is the `pub mod codegen;` line above. Nothing remains deferred here.
//
// Marker breakdown (closed set — zero of every class): no port todos (the
// row-47 and row-86 appends are done — they are the two `pub mod` lines
// above; codegen.rs's own row-110 body todo lives in codegen.rs, not
// here), no source-defect reproductions (no Haskell source), no
// performance deferrals (the declarations are not runtime code), no
// inventory divergence (no inventory row can govern a file with no
// Haskell counterpart — grep "Query/Thrift" across migration/inventory*.tsv:
// every hit is the Internal.hs:98,106 evidence inside inventory.tsv:56
// and its inventory.stm.tsv:35 twin, which govern row 45's caches).
//
// PORT STATUS: confidence=high todos=0
