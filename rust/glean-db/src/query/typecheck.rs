// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// MANIFEST UNIT: this file IS manifest row 92's target
// (glean/db/Glean/Query/Typecheck.hs -> rust/glean-db/src/query/
// typecheck.rs, crate glean-db) — its Typecheck.hs port is NOT YET
// SHIPPED. The file was CREATED EARLY, by row 88's unit (the typecheck
// area's first file, query/typecheck/types.rs, manifest.tsv line 88), as
// the Rust-2018 directory-module anchor: rows 88/90/91 have
// DIRECTORY-LEVEL manifest paths (src/query/typecheck/types.rs — line 88,
// src/query/typecheck/monad.rs — line 90, src/query/typecheck/unify.rs —
// line 91), which Rust resolves only through src/query/typecheck.rs +
// src/query/typecheck/ — the same 2018 layout as src/query/codegen.rs +
// codegen/ (created early by row 86, the unit-87-anchored precedent this
// file mirrors). Without the anchor line below, the area's files are
// unwireable (src/query.rs cannot name a directory module that has no
// file).
//
// OWNERSHIP HANDOFF (row 92's implementer: read before appending):
//   * The `pub mod types;` line below (row 88's wiring) and this header
//     are rows 88's contribution — keep the line, replace nothing else.
//     Rows 90 (monad.rs) and 91 (unify.rs) append their own `pub mod`
//     lines beside it when they land: row 90's (`pub mod monad;`) is
//     already below; row 91's is still to come.
//   * The FILE BODY (the whole Typecheck.hs port: the `typecheck` /
//     `typecheckDeriving` entry points — Typecheck.hs:78-116/:119-174,
//     the infer* drivers, typecheckPattern, promote/demote, resolvePromote,
//     zonk — plus `tcQueryDeps`, consumed by Prune.hs:29/Derive.hs:50/
//     Schema/Gen/Main.hs:59) is row 92's obligation. Append below the
//     wiring lines.
//   * The TcPred span image row 92 builds against is DECIDED in
//     query/typecheck/types.rs (the TcPred archaeology block): the
//     concrete `SrcSpan` — both production entry-point callers
//     (UserQuery.hs:980 via the parser, Schema.hs:881 via
//     predicateSchema :: PredicateDef' SrcSpan ()) instantiate the
//     typechecker at SrcSpan; the reopen condition is recorded there.
//
// Area wiring: `pub mod typecheck;` hangs off src/query.rs (appended by
// row 88's unit, with the ownership note); `crate::query` hangs off
// lib.rs (lib.rs:64 — already present). The crate-root lib.rs carries no
// `pub mod typecheck;` line — that would resolve to crate::typecheck and
// need src/typecheck.rs, which is not this area's manifest path (RULE §4:
// the module header decides the path).

pub mod types;

pub mod monad;

// Coverage: two `pub mod` lines (types — row 88's wiring; monad — row
// 90's wiring) and the ownership notes; no re-exports. The Typecheck.hs
// port itself has NOT started here (row 92's obligation, unchanged by
// rows 88/90); row 91 adds `pub mod unify;` when it lands.
//
// Marker breakdown (closed set — one TODO(port), zero of every other
// class): the single port todo below is the row-92 body, named with its
// owner; no source-defect reproductions (no translated code yet), no
// performance deferrals, no inventory divergence (grep
// "Query/Typecheck" over migration/inventory*.tsv: the only Typecheck
// rows are inventory.tsv:77/:78/:81 — RTS/Foreign/Typecheck.hs and
// RTS/Typecheck.hs, different modules; db/Glean/Query/Typecheck.hs has
// no row). Row 88's types.rs and row 90's monad.rs carry their own marker
// breakdowns and trailers.
//
// TODO(port): row 92 — the Typecheck.hs port owns this file's body
//   (the typecheck/typecheckDeriving entry points, Typecheck.hs:78-116/
//   :119-174, the infer*/typecheckPattern drivers, promote/demote,
//   resolvePromote, zonk, tcQueryDeps); append below the wiring lines.
//   Until it lands, this file is wiring scaffolding only.
// PORT STATUS: confidence=high todos=1
