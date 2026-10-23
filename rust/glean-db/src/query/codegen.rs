// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// MANIFEST UNIT: this file IS manifest row 110's target
// (glean/db/Glean/Query/Codegen.hs -> rust/glean-db/src/query/codegen.rs,
// crate glean-db) — its Codegen.hs port is NOT YET SHIPPED. The file was
// CREATED EARLY, by row 86's unit (QueryRegs, query/codegen/
// query_regs.rs), as the Rust-2018 directory-module anchor: row 86's
// manifest path is the DIRECTORY-LEVEL src/query/codegen/
// query_regs.rs, which Rust resolves only through src/query/codegen.rs +
// src/query/codegen/ — the same 2018 layout as rust/glean-typed/src/
// typed.rs + typed/. Without the anchor line below, row 86's file is
// unwireable (query.rs cannot name a directory module that has no file).
//
// OWNERSHIP HANDOFF (row 110's implementer: read before appending):
//   * The `pub mod query_regs;` line below and this header are row 86's
//     wiring contribution — keep the line, replace nothing else.
//   * The FILE BODY (the whole Codegen.hs port: the codegen driver,
//     generateQueryCode constructing QueryRegs at Codegen.hs:1625-1700,
//     compileStatements/compileFactGenerator/matchPat/recursive, the
//     mdo/label knots under RULE §0's A1 gate) is row 110's obligation.
//     Append below the wiring line.
//   * The field-image fill contract for the QueryRegs construction is
//     documented in query/codegen/query_regs.rs's header (DESIGN — THE
//     FIRST-CLASS-ACTION IMAGE): each field is
//     `Box<dyn Fn(<source args…>, &mut Code)>`, invoked as
//     `(regs.<field>)(<args…>, code)`.
//   * Sibling consumers already pinned there: row 89 stores
//     `Maybe (QueryRegs -> … -> Code a)` (Schema/Types.hs:132/:138 —
//     the record is the first ARGUMENT of the optional transformations,
//     not a bare Maybe payload), row 95 threads the
//     record (Transform.hs:346/:371/:628-642), row 110 both constructs
//     (:1625-1700) and consumes (:225/:399/:969 (matchDef)/:1065/:1420/:1470) it.
// TODO(port): row 110 — the Codegen.hs port owns this file's body
//   (generateQueryCode constructing QueryRegs, Codegen.hs:1625-1700, the
//   compileStatements/compileFactGenerator/matchPat/recursive drivers,
//   the mdo/label knots under RULE §0's A1 gate); append below the
//   wiring line. Until it lands, this file is wiring scaffolding only.
//
// Area wiring: `pub mod codegen;` hangs off src/query.rs (appended
// by row 86's unit, with the ownership note); `crate::query` hangs
// off lib.rs (lib.rs:64 — already present).

pub mod query_regs;

// Coverage: one `pub mod` line (the wiring this file exists for) and
// nothing else — no re-exports; query_regs is its own facade. The
// Codegen.hs port itself has NOT started here.
//
// Marker breakdown (closed set — one TODO(port), zero of every other
// class): the single port todo below is the row-110 body, named with its
// owner; no source-defect reproductions (no translated code yet), no
// performance deferrals, no inventory divergence (no row can govern a
// file whose Haskell counterpart is Codegen.hs's own — its bytecode-
// adjacent rows, inventory.tsv:37, constrain row 110's mdo/label OUTPUT,
// not this scaffolding).
//
// PORT STATUS: confidence=high todos=1
