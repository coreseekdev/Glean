// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// Prim — support for base types (Prim.hs:10): the `Nat` <-> `Word64`
// conversion pair `fromNat`/`toNat`.
//
// Ported from `glean/typed/Glean/Typed/Prim.hs` (24 lines).
//
// Manifest unit: rust/glean-typed/src/typed/prim.rs
// (migration/manifest.tsv line 36, crate glean-typed). RULE §4: one source
// file -> one target file; the manifest path is authoritative.
//
// INVENTORY (RULE §5): NO inventory row governs this file — grep
// "Typed/Prim" across all six migration/inventory*.tsv files returns ZERO
// rows. The source is the spec (RULE §0). No inventory-bug-candidate:
// there is no row to diverge from, and this unit's neighborhood carries
// no CONTRADICTING claims elsewhere — the same wrap class is recorded
// (corroborating) at its own call sites in binary.rs and at
// glean-core's set.rs:467-470 (the unNat/unByte word-set sites) — see
// the sibling block below.
//
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, module-level): the ONLY
// edge involving this file is row 1823, `glean/typed/Glean/Typed.hs ->
// glean/typed/Glean/Typed/Prim.hs` — the crate root's inbound re-export
// (manifest unit 37, NOT YET SHIPPED). Outbound module edges: NONE. Every
// source import has a disposition:
//   * Data.Word (Prim.hs:15) -> base library; `Word64` is the language
//     builtin `u64` (RULE §2 numeric-conversions row) — no import.
//   * Glean.Types (Prim.hs:17) rides the generated-crate edge the way
//     BuildFact/Fact's thrift imports do (module-level edges do not list
//     Glean.Types; the fact.rs:33-35 wording precedent) — consumed as
//     `glean_thrift::Nat` (ASSUMED-IMPORT below).
//
// ASSUMED-IMPORT (RULE §4 B4) — cross-reference form, NO extension of the
// surface: `Nat`'s first sketch is OWNED by typed/binary.rs's
// ASSUMED-IMPORT block (binary.rs:54-69):
//     #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
//     pub struct Nat(pub i64);   // glean.thrift:97 typedef i64 Nat (hs.newtype)
// with `unNat` the newtype record selector — the `.0` field
// (binary.rs:65). Thrift truth: glean/if/glean.thrift:97 `typedef i64 Nat
// (hs.newtype)`. This unit CONSUMES the sketch unchanged and adds
// nothing: the consumer cross-reference precedent is binary.rs:71's
// extension block over build.rs's sketch, except that here there is not
// even an extension. Import form: the single-line thrift import
// (binary.rs:168 `use glean_thrift::{Byte, Nat};`, fact.rs:245 `use
// glean_thrift::Fact;`).
//
// SIBLING-UNIT CONTRACTS (RULE §4; shipped units' real APIs read, never
// re-declared):
//   * crate::typed (the crate root, manifest unit 37 — NOT YET SHIPPED;
//     no src/typed.rs exists in this draft tree): the source's only
//     consumer. Glean/Typed.hs's export list carries
//     `module Glean.Typed.Prim` (Typed.hs:19, imported at :28), so unit
//     37 will re-export exactly `from_nat`/`to_nat` from
//     `crate::typed::prim`; `pub mod prim;` is deferred to that unit
//     (MODULE-WIRING below). No SHIPPED unit consumes this module today.
//   * crate::typed::binary (SHIPPED, read) — OVERLAP NOTE, no dedup:
//     binary.rs's own `impl Type<Nat> for Nat` (binary.rs:420-436)
//     performs the same two wraps at ITS call sites — `self.0 as u64`
//     (binary.rs:429) and `Nat(n as i64)` (binary.rs:436) — each
//     commented there as the `fromIntegral` wrap class
//     (binary.rs:422-427, :432-433, citing Binary.hs:154-158). Those are
//     Binary.hs's OWN push/pop sites, not Prim's functions: the two
//     source modules independently spell the same conversion, so both
//     targets keep it (one file only; the no-second-owner rule governs
//     assumed SURFACES, not this shared idiom). Neither side dedupes.
//
// ---------------------------------------------------------------------------
// Mapping choices (RULE §0: the old code is the spec)
// ---------------------------------------------------------------------------
//   * `fromNat :: Nat -> Word64` = `fromIntegral . unNat`
//     (Prim.hs:20-21) -> `n.0 as u64`. `unNat` is the `.0` field
//     (Int64), and `fromIntegral`'s route via Integer from Int64 to
//     Word64 is the bit-preserving two's-complement wrap — the wrap
//     class binary.rs:425-427 records at its own push site. Rust `as`
//     between integer types is that same wrap (isolated rustc probe of
//     this exact pair: Nat(-1) -> u64::MAX, u64::MAX -> Nat(-1), and a
//     round-trip identity across the sign boundary and both extremes).
//   * `toNat :: Word64 -> Nat` = `Nat . fromIntegral` (Prim.hs:23-24) ->
//     `Nat(w as i64)`, the mirror wrap (binary.rs:432-433's class).
//   * Both functions are TOTAL, pure and effect-free: no Option/Result
//     channel, no error text to preserve, no laziness to stage. Each
//     takes its argument by value (Nat is Copy per the sketch's derive
//     list, binary.rs:57; u64 is Copy) — the source's one-argument
//     signatures, Prim.hs:20/:23.
//   * The source carries NO pragmas (no INLINE either), so the target
//     carries no attributes. Names: `fromNat`/`toNat` ->
//     `from_nat`/`to_nat` (RULE §4 snake_case).
//
// MODULE-WIRING: `pub mod prim;` — deferred to the crate-root unit (cf.
// id.rs/build.rs/build_fact.rs/fact.rs's MODULE-WIRING notes).

use glean_thrift::Nat;

// ---------------------------------------------------------------------------
// fromNat (Prim.hs:20-21)
// ---------------------------------------------------------------------------

/// `fromNat :: Nat -> Word64` (Prim.hs:20-21): `fromIntegral . unNat` —
/// unwrap the thrift newtype and wrap Int64 -> Word64
/// (two's-complement; the header's Mapping choices).
pub fn from_nat(n: Nat) -> u64 {
    // fromIntegral . unNat  (Prim.hs:21)
    n.0 as u64
}

// ---------------------------------------------------------------------------
// toNat (Prim.hs:23-24)
// ---------------------------------------------------------------------------

/// `toNat :: Word64 -> Nat` (Prim.hs:23-24): `Nat . fromIntegral` — the
/// mirror wrap, Word64 -> Int64.
pub fn to_nat(w: u64) -> Nat {
    // Nat . fromIntegral  (Prim.hs:24)
    Nat(w as i64)
}

// Coverage: the export list is exactly `fromNat, toNat` (Prim.hs:12) and
// the target exports exactly `pub fn from_nat` / `pub fn to_nat` —
// nothing more, nothing less; there is no scaffolding, no helper, and no
// other pub item.
//
// Marker breakdown (closed set — zero of every class):
//   * no open port todos: every construct has a disposition — the two
//     signatures, the two wrap conversions (RULE §2's explicit-`as`
//     image, probe-verified), the import surface (cross-referenced, not
//     re-sketched), and the re-export wiring (deferred with a named
//     owner, unit 37).
//   * no source-defect reproductions: the conversions are total; the
//     source's wrap semantics are the SPEC here (RULE §2 numeric row),
//     not a defect.
//   * no performance deferrals: two primitive wraps; nothing to defer.
//   * no inventory divergence: no row governs this unit (see the
//     INVENTORY block).
//
// Unsafe count: 0 — no FFI, no raw pointers; RULE §3's escape hatch is
// unused (the crate-wide non-decoder precedent, fact.rs's closing note).
//
// PORT STATUS: confidence=high todos=0
