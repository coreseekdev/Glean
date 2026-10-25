// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

//! `glean-fb303` — the Rust image of the OSS build's shared service-data
//! layer: the `glean/github/` tree's shims for Meta-internal interfaces
//! (glean/github/ServiceData/** and glean/github/Facebook/**, mapped to
//! this crate by depmap_haskell.py::CRATE_PREFIXES — "shared
//! thrift-type wrappers (fb303/ServiceData) used by core, indexers,
//! server alike — lowest shared layer"; migration/depmap/crates.txt line
//! 7, 5 units).
//!
//! The Haskell sources here are deliberately thin OSS shims: the real
//! fb303 service-data backend is FB-internal and not in this tree, so
//! `github/` ships stub interfaces that the rest of the OSS build links
//! against (the no-op metrics functions of ServiceData/GlobalStats.hs).
//! RULE §0 (old code is the spec) ports them no-op-for-no-op; see
//! crate::service_data::global_stats for the stub-provenance
//! adjudication.
//!
//! RUST-ONLY SCAFFOLDING: this file has NO Haskell counterpart (Haskell
//! needs no crate-root file). Created by the crate's first shipped units
//! — the ServiceData batch (migration/manifest.tsv lines 96-97) —
//! following the first-unit precedent (glean-typed, commit 82b4437;
//! glean-util, commit 4bdf95a): the earliest unit of a crate owns its
//! crate scaffolding. Later units append their own `pub mod` lines here.
//! Nothing belongs here beyond module declarations: no lib-level
//! `pub use` (a re-export here would fork the namespace; RULE §4 "no
//! glob re-exports across crates" — each submodule is its own facade).

// The ServiceData/ area wiring: src/service_data.rs is the directory
// module (Rust-only scaffolding — Haskell has no `ServiceData` root
// module, only ServiceData/Types.hs and ServiceData/GlobalStats.hs). It
// declares both of this batch's rows: types (line 96) and global_stats
// (line 97), wired to each other exactly as the sources are
// (GlobalStats.hs:14 `import ServiceData.Types`).
pub mod service_data;

// glean/github/Facebook/** wiring — future rows 178-180 (Fb303.hs ->
// src/facebook/fb303.rs, Service/HttpFb303.hs ->
// src/facebook/service/http_fb303.rs, Service.hs ->
// src/facebook/service.rs) append `pub mod facebook;` here (plus the
// src/facebook.rs directory module) when their units land.

// Coverage: one `pub mod` line — service_data (this batch's rows 96-97)
// — and nothing else: the facebook/ wiring above is a recorded contract,
// not a declaration (its files do not exist yet).
//
// Marker breakdown (closed set — zero of every class): no port todos
// (the crate-root wiring is complete for the shipped units; the
// facebook/ append is owned by rows 178-180, named in prose per the
// glean-util lib.rs precedent), no source-defect reproductions (no
// Haskell source), no performance deferrals (no code), no inventory
// divergence (no inventory row can govern a file with no Haskell
// counterpart).
//
// PORT STATUS: confidence=high todos=0
