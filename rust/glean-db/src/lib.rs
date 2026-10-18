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
//!
//! A THIRD kind, added by row 48: `crate::internal::types`
//! (`{Completeness, Meta, StorageName, …}`, named by src/database/meta.rs)
//! is GENERATED code — the gen-hs2 image of glean/if/internal.thrift — so
//! it is neither a manifest unit nor a Rust-only module owed by a row: it
//! is the assumed-import home DECISIONS.md (2026-10-05) pinned to glean-db
//! (generated-thrift adjacent), owed to Step 4's build.rs. No inventory row
//! backs that pin (grep-verified), and the generated module has a
//! cross-crate consumer (glean-schema's SchemaIndex/SchemaInstance), so
//! src/database/meta.rs's one TODO carries both caveats; whichever home
//! wins, only the import path changes.

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

// The Database/ area wiring: the source modules Glean.Database.Meta.hs
// (migration/manifest.tsv line 48, src/database/meta.rs) and
// Glean.Database.Catalog/Filter.hs (line 49, src/database/catalog/
// filter.rs) are SIBLINGS of the Glean.Backend.* area in Haskell, so
// their `pub mod` hangs off the crate root here — after `pub mod
// write;` — not inside crate::backend (RULE §4: the module header
// decides the path; the siblings are not Backend children). The
// directory module file is src/database.rs (Rust-only scaffolding —
// Haskell has no `Glean.Database` root module; glean/db/Glean/ contains
// only Database/, Logger/, Logger.hs, Query/, Write/); this unit appends
// `pub mod database;` for row 48's file, which src/database.rs declares
// as `pub mod meta;` (a direct child, wired now), and row 49 appends
// `pub mod catalog;` inside src/database.rs and declares
// `pub mod filter;` in src/database/catalog.rs when it lands (that
// contract is recorded in src/database.rs's prose, which row 49's unit
// consumes).
pub mod database;

// Coverage: four `pub mod`s (backend, query, write, database) and nothing
// else — no lib-level re-exports (a `pub use` here would fork the
// namespace the siblings' `crate::backend::…` / `crate::query::…` /
// `crate::write::…` / `crate::database::…` paths already presume; each
// submodule is its own facade, RULE §4 "no glob
// re-exports").
//
// Marker breakdown (closed set — zero of every class): no port todos
// (the crate-root wiring is complete — row 47's `pub mod thrift;` inside
// src/query.rs has since SHIPPED; the two remaining CONTRACTED area lines
// so far are row 115's `pub mod json;` inside src/write.rs and row 49's
// `pub mod catalog;` inside src/database.rs, both named in prose here and
// in their files; the `haxl`/`logger`/github-straggler lines named in the
// doc header above remain for their own units), no source-defect
// reproductions (no Haskell
// source), no performance deferrals (no code), no inventory divergence (no
// inventory row can govern a file with no Haskell counterpart).
//
// PORT STATUS: confidence=medium todos=0
