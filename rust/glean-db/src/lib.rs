// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

//! `glean-db` — the Rust image of the Haskell database/back-end area:
//! glean/backend-api (Glean.Backend.*), glean/backend-local
//! (Glean.Backend.Local / Logging / Dump), glean/db (Glean.Database.*,
//! Glean.Query.*, Glean.Logger.*, Glean.Write.*), the Glean-owned Haxl
//! datasource (glean/haxl/Haxl/DataSource/Glean*), and the github/
//! stragglers mapped into this crate — the 90 units the depmap assigns
//! to glean-db (migration/depmap/crates.txt:6; migration/manifest.tsv
//! rows with crate glean-db).
//!
//! RUST-ONLY SCAFFOLDING: this file has NO Haskell counterpart (Haskell
//! needs no crate-root file, and the area has no `Glean.Backend` /
//! `Glean.Database` root module either — glean/backend-api/Glean/
//! contains only Backend/, Query/, Write/). Created by this crate's
//! first shipped unit (rust/glean-db/src/backend/types.rs, migration/
//! manifest.tsv line 41) following the glean-util / glean-typed
//! precedent (rust/glean-util/src/lib.rs, rust/glean-typed/src/lib.rs,
//! commit 82b4437): the earliest unit of a crate owns its crate
//! scaffolding. Later units append their own `pub mod` lines here as
//! they land (`database`, `haxl`, `logger`, and the
//! github stragglers' paths).
//!
//! Forward-reference note: `backend/types.rs` names same-crate paths
//! whose owners are later units — `crate::database::types::Env` (row
//! 104) and `crate::backend::logging` (row 148) are still forward
//! references, while `crate::backend::retry` (row 43) has since shipped
//! (src/backend/retry.rs, wired in src/backend.rs) — plus
//! `crate::haxl::engine::{GleanGetFetcher, GleanQueryFetcher}` (row
//! 146), which is special: the Haxl framework core is an EXTERNAL
//! Haskell package with no manifest row, so its Rust image is a
//! Rust-only module owed by row 146
//! (src/haxl/data_source/glean/common.rs) — see the Haxl seam block in
//! backend/types.rs for the contract that module must satisfy.

pub mod backend;

// The Query/ area wiring: the source modules Glean.Query.Thrift.hs
// (migration/manifest.tsv line 47, src/query/thrift.rs) and
// Glean.Query.Thrift/Internal.hs (line 45, src/query/thrift/internal.rs)
// are SIBLINGS of the Glean.Backend.* area in Haskell, so their `pub mod`
// hangs off the crate root here — after `pub mod backend;` — not inside
// crate::backend (RULE §4: the module header decides the path; the
// siblings are not Backend children). The directory module file is
// src/query.rs (Rust-only scaffolding — Haskell has no `Glean.Query` root
// module); this unit appends `pub mod query;` for row 45's file, and row
// 47 has since appended `pub mod thrift;` inside src/query.rs (that
// contract was recorded there in prose and is now consumed).
pub mod query;

// The Write/ area wiring: the source modules Glean.Write.SendBatch.hs
// (migration/manifest.tsv line 46, src/write/send_batch.rs) and
// Glean.Write.JSON.hs (line 115, src/write/json.rs) are SIBLINGS of the
// Glean.Backend.* area in Haskell, so their `pub mod` hangs off the crate
// root here — after `pub mod query;` — not inside crate::backend (RULE §4:
// the module header decides the path; the siblings are not Backend
// children). The directory module file is src/write.rs (Rust-only
// scaffolding — Haskell has no `Glean.Write` root module); this unit
// appends `pub mod write;` for row 46's file, which src/write.rs declares
// as `pub mod send_batch;` (a direct child, wired now), and row 115
// appends `pub mod json;` inside src/write.rs when it lands (that
// contract is recorded there in prose).
pub mod write;

// Coverage: three `pub mod`s (backend, query, write) and nothing else — no
// lib-level re-exports (a `pub use` here would fork the namespace the
// siblings' `crate::backend::…` / `crate::query::…` / `crate::write::…`
// paths already presume; each submodule is its own facade, RULE §4 "no glob
// re-exports").
//
// Marker breakdown (closed set — zero of every class): no port todos
// (the crate-root wiring is complete — row 47's `pub mod thrift;` inside
// src/query.rs has since SHIPPED; the one remaining area line is row
// 115's `pub mod json;` inside src/write.rs, named in prose here and in
// that file), no source-defect reproductions (no Haskell
// source), no performance deferrals (no code), no inventory divergence (no
// inventory row can govern a file with no Haskell counterpart).
//
// PORT STATUS: confidence=medium todos=0
