// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// Crate-root facade of the `glean-typed` crate — support for working with
// strongly-typed Glean data.
//
// Ported from: glean/typed/Glean/Typed.hs (28 lines)
// Manifest unit: rust/glean-typed/src/typed.rs
// (migration/manifest.tsv line 37, crate glean-typed). RULE §4: one source
// file -> one target file; the manifest path is authoritative.
//
// WHAT THIS FILE IS: Glean.Typed is a pure facade — 28 lines with no
// definitions (Typed.hs:12-28). Its Haskell body is the curated export
// list (seven `module Glean.Typed.X` items, Typed.hs:13-19) plus seven
// blanket imports (Typed.hs:22-28). The Rust image keeps that two-part
// shape: `pub mod` declarations for the seven submodules (the wiring every
// shipped sibling's MODULE-WIRING note deferred here), then one NAMED
// `pub use` per module mirroring that module's CURATED Haskell export
// list — never a glob (RULE §4's no-glob-re-exports discipline; house
// precedent rust/glean-core/src/rts.rs:190). The source's export-list
// order (Binary, Build, BuildFact, Fact, Id, Predicate, Prim — already
// alphabetical) governs the `pub mod` block; within each `pub use` block
// the names follow the source export list line by line, so the mapping is
// reviewable against the .hs text. Naming: `Glean.Typed` ->
// `crate::typed` (the module path every shipped sibling's
// `use crate::typed::…` presumes).
//
// INVENTORY (RULE §5): NO inventory row governs this file — grep
// "Glean/Typed.hs" across all six migration/inventory*.tsv files returns
// ZERO rows (the path appears elsewhere only in the depmap artifacts —
// edges.tsv, order.txt — and manifest.tsv:37). A re-export-only file has no §5-class constructs. No
// INVENTORY-BUG-CANDIDATE: there is no row to diverge from.
//
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, module-level): this file
// is the SOURCE of the seven re-export edges and the TARGET of the
// consumers':
//   * outbound, one per submodule (rows 1817-1823): Glean/Typed.hs ->
//     Binary.hs / Build.hs / BuildFact.hs / Fact.hs / Id.hs /
//     Predicate.hs / Prim.hs — each mirrored by a `pub mod` + `pub use`
//     pair below.
//   * inbound (rows 18, 27, 79, 91, 1219, 1222, 1816): Backend/Types.hs,
//     Query/Thrift/Internal.hs, Glean.hs, Haxl.hs,
//     Haxl/DataSource/Glean.hs, Haxl/DataSource/Glean/Common.hs and
//     Query/Angle.hs import Glean.Typed and see the union of the seven
//     export lists — exactly the union re-exported below.
//   * NO ASSUMED-IMPORT of its own (RULE §4 B4): this unit imports no
//     generated code — `pub mod` declarations bind module names, not
//     items, and every `pub use` re-exports a shipped item. The
//     generated-code surface (glean_thrift's Nat/Byte/Fact/…) lives one
//     module down, already sketched by the siblings' ASSUMED-IMPORT
//     blocks.
//
// SIBLING-UNIT CONTRACTS (RULE §4; all seven SHIPPED — each name below
// was grep-verified against the target module's real pub surface, and
// each export list was read from the Haskell source):
//   * crate::typed::binary (unit 17, ae1b1eb): Type(..) (Binary.hs:17 —
//     all five class members have Rust trait items at binary.rs:235-268:
//     buildRtsValue/decodeRtsValue/constantRtsValue/decodeAsFact/
//     sourceType -> build_rts_value/decode_rts_value/constant_rts_value/
//     decode_as_fact/source_type), decodeRts (:277), mapD (:283),
//     buildRtsSelector (:293), thriftEnum_buildRtsValue (:307).
//   * crate::typed::build (unit 15, e4626cc): Decoder(..) (:237 — the
//     Haskell record selector runDecoder, Build.hs:51-52, is the pub
//     method `run_decoder` at build.rs:254), DecoderEnv(..) (:188 — all
//     five record fields pub, mirroring Build.hs:41-47),
//     DecodingException(..) (:344, pub field decoding_exception :347,
//     Build.hs:72), decodeFail (:380), decode (:394), decodeWithCache
//     (:414), sumD (:487), enumD (:545), thriftEnumD (:584).
//   * crate::typed::build_fact (unit 34, 367f952): NewFact(newFact,
//     withUnit, derivedFrom) (:305 — the three curated methods :318,
//     :331, :339; the trait additionally carries `type Error` (:309),
//     the image of the MonadFail superclass, BuildFact.hs:57 — see the
//     asymmetry notes), makeFact (:398), makeFact_ (:410), makeFactV
//     (:358), makeFactV_ (:378), Facts (:448 — exported BARE in Haskell,
//     BuildFact.hs:16; its four fields are private here, matching),
//     newFacts (:477), serializeFacts (:532), factsMemory (:554),
//     FactBuilder (:752), buildFacts (:779), extendFacts (:800),
//     buildBatch (:814).
//   * crate::typed::fact (unit 35, e879d8a): decodeFact (:272),
//     decodeRef (:334) (Fact.hs:10-11).
//   * crate::typed::id (unit 16, 4382ebe): Pid(..), Fid(..) (Id.hs:14 —
//     re-exported by id.rs:124 from glean_core::rts::types; re-exporting
//     a pub re-export keeps the chain), PidOf(..) (:141 — constructor =
//     `PidOf::new` :151, field read = `pid_of()` :159, the encapsulation
//     id.rs:147-161 documents), IdOf(..) (:197 — `IdOf::new` :205,
//     `id_of()` :213).
//   * crate::typed::predicate (unit 33, d54cd8a): Predicate(..) (:268 —
//     both associated types and all six members — five abstract methods
//     plus getIndex with its default impl (Predicate.hs:74-77; get_index
//     at predicate.rs:314) — mirror Predicate.hs:64-77), justId (:336),
//     predicateRef (:557), predicateSourceType
//     (:330), HasPredicates(..) (:380), MissingPredicate(..) (:346 — pub
//     tuple field = the positional newtype constructor, Predicate.hs:88),
//     throwMissingPredicate (:362), Predicates (Predicate.hs:23 exports
//     it WITHOUT `(..)` — the unit-33 ruling: field private; the type
//     NAME only is re-exported below, and predicate.rs:374's private
//     tuple field already hides any constructor), SchemaPredicates
//     (:420), makePredicates (:482).
//   * crate::typed::prim (unit 36, ab55ecb): fromNat (:105), toNat
//     (:116) (Prim.hs:12).
//
// SELECTOR/ASYMMETRY NOTES (nothing invented to patch them):
//   * `NewFact` ships `type Error`: Haskell's export list curates three
//     methods out of the class, but a Rust trait is exported atomically,
//     so the MonadFail superclass's image (BuildFact.hs:57's
//     `MonadFail m` constraint -> `type Error`) rides along. Consumers
//     see no obligation the Haskell consumer did not already have (the
//     superclass constraint); recorded here, not patched.
//   * Sibling pub items OUTSIDE the Haskell export lists stay
//     module-scoped by the named `pub use` form: build.rs's ap (:312),
//     DecodeError (:365), BoundedEnum (:521); build_fact.rs's
//     NewFactError (:287), FactsM (:579), BuildFactsError (:760);
//     predicate.rs's `PredicateIndex` (:369, non-pub by design — the
//     source keeps it out of its export list too, Predicate.hs:12-24).
//     DecodeError appears in exported signatures (e.g. `decode`'s
//     Result) exactly as it is reachable-but-unexported in Haskell.
//   * NO exported Haskell record selector lacks a Rust pub counterpart:
//     runDecoder -> pub method run_decoder; DecoderEnv's and
//     DecodingException's fields are pub; PidOf/IdOf field reads are pub
//     methods (fields private — the documented id.rs encapsulation);
//     MissingPredicate's field is pub. Predicates and Facts are
//     bare-type exports with private fields on BOTH sides.
//
// NAME-COLLISION CHECK: the 45 re-exported names are pairwise distinct
// across the seven curated lists (Haskell's compiler would reject a
// duplicate export in Typed.hs's union; a grep of the seven lists
// confirms none — no `decode`-vs-`decode_rts` style clash either). No
// ambiguity to report.
//
// MODULE-WIRING: this unit OWNS the wiring the shipped siblings
// deferred — the `pub mod` declarations below (id.rs:113-115,
// build.rs:148-152, prim.rs:93-94 and the other siblings' MODULE-WIRING
// notes name unit 37 as owner) — plus the crate root lib.rs (`pub mod
// typed;`) and this crate's Cargo.toml. lib.rs and Cargo.toml are
// Rust-only scaffolding with no Haskell counterpart.

pub mod binary;
pub mod build;
pub mod build_fact;
pub mod fact;
pub mod id;
pub mod predicate;
pub mod prim;

// --- Glean.Typed.Binary (Typed.hs:13; export list Binary.hs:15-22) ---
pub use crate::typed::binary::{
    Type, decode_rts, map_d, build_rts_selector, thrift_enum_build_rts_value,
};

// --- Glean.Typed.Build (Typed.hs:14; export list Build.hs:11-18) ---
pub use crate::typed::build::{
    Decoder, DecoderEnv, DecodingException, decode_fail, decode, decode_with_cache, sum_d, enum_d,
    thrift_enum_d,
};

// --- Glean.Typed.BuildFact (Typed.hs:15; export list BuildFact.hs:10-24) ---
pub use crate::typed::build_fact::{
    NewFact, make_fact, make_fact_, make_fact_v, make_fact_v_, Facts, new_facts, serialize_facts,
    facts_memory, FactBuilder, build_facts, extend_facts, build_batch,
};

// --- Glean.Typed.Fact (Typed.hs:16; export list Fact.hs:9-12) ---
pub use crate::typed::fact::{decode_fact, decode_ref};

// --- Glean.Typed.Id (Typed.hs:17; export list Id.hs:12-16) ---
pub use crate::typed::id::{Pid, Fid, PidOf, IdOf};

// --- Glean.Typed.Predicate (Typed.hs:18; export list Predicate.hs:12-24) ---
pub use crate::typed::predicate::{
    Predicate, just_id, predicate_ref, predicate_source_type, HasPredicates, MissingPredicate,
    throw_missing_predicate, Predicates, SchemaPredicates, make_predicates,
};

// --- Glean.Typed.Prim (Typed.hs:19; export list Prim.hs:11-13) ---
pub use crate::typed::prim::{from_nat, to_nat};

// Coverage: the union of the seven `pub use` blocks is exactly the union
// of the seven Haskell export lists — 45 names (5 Binary + 9 Build + 13
// BuildFact + 2 Fact + 4 Id + 10 Predicate + 2 Prim), nothing more,
// nothing less; the `pub mod` block is the wiring the source's seven
// blanket imports imply (Typed.hs:22-28).
//
// Marker breakdown (closed set — zero of every class):
//   * no open port todos: every re-export names a grep-verified pub item;
//     the wiring deferrals the siblings recorded name THIS unit as owner
//     and are discharged here — no deferral remains.
//   * no source-defect reproductions: a re-export-only file has no
//     behavior.
//   * no performance deferrals: no code.
//   * no inventory divergence: no row governs this unit (INVENTORY
//     block).
//
// PORT STATUS: confidence=high todos=0
