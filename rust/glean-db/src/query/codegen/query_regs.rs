// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// Ported from: glean/db/Glean/Query/Codegen/QueryRegs.hs (142 lines)
// Manifest unit: rust/glean-db/src/query/codegen/query_regs.rs
// (migration/manifest.tsv line 86, crate glean-db). RULE §4: one source
// file -> one target file; the manifest path is authoritative (the
// directory-level path is anchored by src/query/codegen.rs — created as
// wiring scaffolding by this unit; that file is manifest row 110's target
// and its body is row 110's obligation, see its header).
//
// WHAT THIS FILE IS: a pure RECORD DECLARATION — the file constructs
// nothing (QueryRegs.hs:16-142 is one `data QueryRegs` of 22 fields: 19
// function-typed bytecode-emitter fields over registers, each returning
// `Code ()`, plus 3 direct `Register 'Word` fields, saveState/maxResults/
// maxBytes, which the source's own haddock marks "Unused, temporarily kept
// for backwards compatibility", QueryRegs.hs:134). It declares the SHAPE
// of the syscall table the codegen runs on: row 110 (Codegen.hs, NOT YET
// SHIPPED) constructs the record (the 19 wrapper closures +
// `f QueryRegs{..}`, Codegen.hs:1625-1700), row 89 (Database/Schema/
// Types.hs, NOT YET SHIPPED) takes it as the first argument of its
// optional `Maybe (QueryRegs -> …)` transformation fields (Types.hs:132/
// :138), row 95 (Query/Transform.hs, NOT YET SHIPPED) threads it as a
// plain argument (:346/:371/:628-642).
//
// INVENTORY (RULE §5): NO row in any of migration/inventory*.tsv governs
// glean/db/Glean/Query/Codegen/QueryRegs.hs — grep "QueryRegs" over
// inventory{,.ffi,.protocol,.sites,.stm,.codeflow}.tsv: zero hits (the
// module's path never occurs, not even in evidence columns). Nothing here
// is an inventory-owned class: no FFI, no STM, no laziness-DEPENDENT
// construct (the record's fields are functions; Haskell function values
// are not lazy in a way this port observes), no exceptions, no
// unsafePerformIO. No divergence from any row (there is no row).
//
// DEPENDENCY TRUTH (migration/depmap/edges.tsv — checked, not assumed;
// QueryRegs.hs is the source of 2 edges and the target of 3):
//   IN  :394 Database/Schema/Types.hs -> QueryRegs.hs  (row 89's unit)
//   IN  :534 Codegen.hs             -> QueryRegs.hs  (row 110's unit)
//   IN  :633 Query/Transform.hs     -> QueryRegs.hs  (row 95's unit)
//   OUT :548 QueryRegs.hs -> glean/bytecode/Glean/Bytecode/Types.hs
//   OUT :549 QueryRegs.hs -> glean/hs/Glean/RTS/Bytecode/Code.hs
// (the Haskell import list, QueryRegs.hs:13-14, is exactly those two —
// no third import, so this unit consumes nothing else; rts/bytecode/
// code.rs's header IN-block independently lists Codegen/QueryRegs.hs
// among its consumer units, edge 549.) Crate mapping:
//   * glean-db -> glean-bytecode (edge 548): LIVE cross-crate form
//     `glean_bytecode::bytecode::types::…`, the exact path shipped
//     consumers use (rust/glean-core/src/rts/bytecode/supply.rs:113).
//     NOTE for Step 4: this is a NEW anticipated Cargo edge for
//     glean-db — glean-db/Cargo.toml's anticipated-dependency list
//     (glean-thrift, glean-core, glean-typed, glean-util, glean-angle)
//     predates this unit and does not yet name glean-bytecode; the edge
//     is recorded here per the Cargo.toml's own convention ("recorded
//     here rather than as a real entry") — the scaffold bullet is owed.
//   * glean-db -> glean-core (edge 549): the edge is already anticipated
//     in glean-db/Cargo.toml (the row-52 bullet); the Code path below is
//     unresolvable until Step 4 lands the crate scaffolds — the expected
//     draft state for every cross-crate consumer (rust/glean-db/src/
//     database/repo.rs:60-65 records the identical disposition for its
//     `use glean_core::repo::text::show_repo_sep;`).
//
// DESIGN — THE FIRST-CLASS-ACTION IMAGE (this file's one decision; the
// image row 110 must fill):
//   The Haskell fields are first-class `args -> Code ()` actions stored in
//   a record and applied by record selector inside enclosing do-blocks
//   (Codegen.hs:225/:447/:994/:1425/:1700 bind `regs@QueryRegs{..}` once
//   and apply the fields as plain functions, repeatedly, all over one
//   codegen pass). The shipped Code image (code.rs:130-137, header DESIGN
//   (1)) collapsed the monad into `&mut Code`-threaded methods but
//   established NO first-class-action image (grep code.rs: the only
//   first-class functions are `&dyn Fn` mapping callbacks at :88-91,
//   a different shape). The image chosen, for all 19 function fields:
//
//       Box<dyn Fn(<source args, in source order>, &mut Code)>
//
//   * Arguments in EXACT source order. The monadic context is not a
//     source argument — appending it LAST reorders nothing (RULE §0);
//     the first N closure parameters are the source's N parameters,
//     register-width phantoms preserved (the A1 width-fidelity gate;
//     the source's per-argument width comments are carried as each
//     field's doc text below).
//   * `&mut Code` LAST is the established in-tree closure convention for
//     a Code-monad action with data args: code.rs:335/:368/:430
//     (`body: impl FnOnce(REGS, &mut Code) -> A`), :446/:458
//     (`FnOnce(Vec<A>, &mut Code) -> R`), :690 (`FnOnce(REGS, &mut Code)
//     -> ()`); the code-sole-arg form is :488 (`FnOnce(&mut Code) -> A`).
//   * `Fn`, not FnMut/FnOnce: the emitters run REPEATEDLY and the record
//     is consumed through a SHARED reference — the source threads
//     `QueryRegs` as a plain immutable argument (Transform.hs:371/:628,
//     Codegen.hs:399) and applies one field many times per pass (e.g.
//     `seek`/`result` in Codegen.hs's per-pattern loops); FnMut would
//     force &mut-record plumbing the source does not have, FnOnce would
//     break reuse. `Box`, not `fn`-pointers or Rc: row 110's wrappers
//     CAPTURE the syscall register handles that `generate`'s RegSupply
//     supplies at runtime (Codegen.hs:1625-1629 — `seek_ …` etc. are
//     RegSupply-supplied `Register ('Fun '[…])` values: the generator
//     emits `'Fun`-tagged syscall handles — Traverse.hs:28 `Register
//     ('Fun '[ 'Word, 'Word ])`, Generate/Haskell.hs:127 — NOT plain
//     `'Word`; only the 3 direct fields below are `'Word`), so the
//     field values are capturing closures; the record OWNS its
//     emitters exactly as the Haskell record owns its closures.
//   * `next`'s `Bool` (QueryRegs.hs:48, "do we need the value?") -> bool;
//     every field's `Code ()` -> the implicit `()` return.
//   * PROBE (rustc 1.98.1, scratch /tmp/qregs_probe/probe.rs): a struct
//     of `Box<dyn Fn(args…, &mut Code)>` fields + `Register<Word>`
//     fields; `move` closures capturing Copy `Register` handles and
//     calling `Code` methods (`constant`, `issue`) inside; invoked
//     REPEATEDLY from a free fn `(&QueryRegs, &mut Code)` and from an
//     `&self` method via `(self.regs.seek)(…)` — compiles clean and
//     runs. The invocation idiom is the parenthesized call-through-
//     reference form `(regs.seek)(…)`: a bare `regs.seek(…)` is E0599
//     ("field, not a method"; rustc's own help prescribes the parenthesized
//     form). Row 110's consumption sites use the parenthesized
//     form; this is the fill contract's call shape.
//
// SIBLING-UNIT CONTRACTS (RULE §4; shipped surfaces read, never
// re-declared; unshipped units recorded with their cites):
//   - SHIPPED rust/glean-bytecode/src/bytecode/types.rs owns, for the
//     whole port, `Register` (types.rs:374,
//     `Register<T: TyTag>(pub u64, pub PhantomData<T>)`), the TyTag
//     markers `Word` (:199), `DataPtr` (:211), `BinaryOutputPtr` (:215),
//     and `cast_register` (:392). Its header ALREADY pins this file as a
//     downstream register-type user ("Register<'Word>/<'DataPtr>/
//     <'BinaryOutputPtr> signatures in db/Glean/Query/Codegen/
//     QueryRegs.hs:20-77", types.rs:69-71; struct-doc restatement
//     types.rs:371). The Haskell `Register 'Word` maps to
//     `Register<Word>`, `'DataPtr` -> `DataPtr`, `'BinaryOutputPtr` ->
//     `BinaryOutputPtr`; no field here needs `cast_register` (the
//     retagging happens in row 110's WRAPPERS, Codegen.hs:1631/:1634
//     etc., which is where the source does it too).
//   - SHIPPED rust/glean-core/src/rts/bytecode/code.rs owns `Code`
//     (code.rs:256 — "ONE public struct `Code` whose private fields ARE
//     CodeS", with the exported combinators as `&mut self` methods).
//     This file references Code ONLY as the trailing context parameter
//     of the field closures; it calls no Code method.
//   - NOT YET SHIPPED, manifest row 110 (glean/db/Glean/Query/Codegen.hs
//     -> src/query/codegen.rs): CONSTRUCTS this record — the 19 wrappers
//     over the RegSupply-supplied syscall handles (Codegen.hs:1625-1696;
//     note the `lookupKeyValue` FIELD wraps the `lookupKey_` HANDLE,
//     :1657-1658 — field name and syscall name differ, as in the source)
//     plus the three direct registers, then hands the record to the
//     codegen lambda, `f QueryRegs{..}` (:1700); CONSUMES it via
//     `regs@QueryRegs{..}` at :225/:447/:994/:1425/:1598 and threads it
//     to helpers (:399 cmpOutputPat, :969 matchDef, :1065,
//     :1420 recursive, :1470 matchPat). This file's fields ARE what row
//     110 fills.
//   - NOT YET SHIPPED, manifest row 89 (glean/db/Glean/Database/Schema/
//     Types.hs -> src/database/schema/types.rs): the record appears as
//     the FIRST ARGUMENT of the optional pattern-transformation function
//     fields `transformKeyPattern` (Types.hs:132) and
//     `transformValuePattern` (:138) — `Maybe (QueryRegs -> (Type ->
//     Term … -> Code ()) -> … -> Code a)`, i.e. `Maybe` of a function
//     TAKING the record, not a bare `Maybe QueryRegs`.
//   - NOT YET SHIPPED, manifest row 95 (glean/db/Glean/Query/Transform.hs
//     -> src/query/transform.rs): consumes the record as a plain argument
//     (:346 transformPattern, :371 transformMatch, :628 transformBytes,
//     :637 transformBytes' with a `QueryRegs{..}` wildcard at :642).
//
// Deriving map: the source record has NO deriving clause
// (QueryRegs.hs:16-142) — no derives here. Field order below IS the
// source record order, field for field (reviewer checklist):
//   seek (:19) seekWithinSection (:26) currentSeek (:36) endSeek (:41)
//   next (:46) lookupKeyValue (:57) result (:65) resultWithPid (:73)
//   newDerivedFact (:82) firstFreeId (:89) newSet (:93) insertOutputSet
//   (:97) setToArray (:102) freeSet (:107) newWordSet (:111) insertWordSet
//   (:115) wordSetToArray (:120) byteSetToByteArray (:125) freeWordSet
//   (:130) saveState (:135) maxResults (:138) maxBytes (:141).

use glean_bytecode::bytecode::types::{BinaryOutputPtr, DataPtr, Register, Word};
use glean_core::rts::bytecode::code::Code;

/// The syscall table the query codegen emits through —
/// `data QueryRegs` (QueryRegs.hs:16-142), field for field, in source
/// order. 19 emitter actions + 3 direct registers (see header DESIGN for
/// the field-image contract row 110 fills; per-field invocation is
/// `(regs.<field>)(<args…>, code)`).
pub struct QueryRegs {
    /// "Start a new traversal of facts beginning with a given prefix"
    /// (source haddock, QueryRegs.hs:18).
    ///
    /// `seek :: Register 'Word -> Register 'DataPtr -> Register 'DataPtr
    /// -> Register 'Word -> Code ()` (:19-24). Arguments, in source
    /// order (the width comments are the source's own, :20-23):
    ///   * `Register<Word>` — predicate id
    ///   * `Register<DataPtr>` — prefix
    ///   * `Register<DataPtr>` — prefix end
    ///   * `Register<Word>` — (output) token
    pub seek: Box<dyn Fn(Register<Word>, Register<DataPtr>, Register<DataPtr>, Register<Word>, &mut Code)>,

    /// `seekWithinSection :: Register 'Word -> Register 'DataPtr ->
    /// Register 'DataPtr -> Register 'Word -> Register 'Word ->
    /// Register 'Word -> Code ()` (QueryRegs.hs:26-33). Arguments, in
    /// source order (:27-32):
    ///   * `Register<Word>` — predicate id
    ///   * `Register<DataPtr>` — prefix
    ///   * `Register<DataPtr>` — prefix end
    ///   * `Register<Word>` — section start
    ///   * `Register<Word>` — section end
    ///   * `Register<Word>` — (output) token
    pub seek_within_section: Box<dyn Fn(Register<Word>, Register<DataPtr>, Register<DataPtr>, Register<Word>, Register<Word>, Register<Word>, &mut Code)>,

    /// "Fetch the current seek token" (source haddock, QueryRegs.hs:35).
    ///
    /// `currentSeek :: Register 'Word -> Code ()` (:36-38); argument:
    /// `Register<Word>`.
    pub current_seek: Box<dyn Fn(Register<Word>, &mut Code)>,

    /// "Release the state associated with an iterator token" (source
    /// haddock, QueryRegs.hs:40).
    ///
    /// `endSeek :: Register 'Word -> Code ()` (:41-43); argument:
    /// `Register<Word>`.
    pub end_seek: Box<dyn Fn(Register<Word>, &mut Code)>,

    /// "Grab the next fact in a traversal" (source haddock,
    /// QueryRegs.hs:45).
    ///
    /// `next :: Register 'Word -> Bool -> Register 'Word ->
    /// Register 'DataPtr -> Register 'DataPtr -> Register 'DataPtr ->
    /// Register 'Word -> Code ()` (:46-54). Arguments, in source order
    /// (:47-53):
    ///   * `Register<Word>` — token
    ///   * `bool` — do we need the value? (source `Bool`, :48)
    ///   * `Register<Word>` — result
    ///   * `Register<DataPtr>` — clause begin
    ///   * `Register<DataPtr>` — key end
    ///   * `Register<DataPtr>` — clause end
    ///   * `Register<Word>` — id
    pub next: Box<dyn Fn(Register<Word>, bool, Register<Word>, Register<DataPtr>, Register<DataPtr>, Register<DataPtr>, Register<Word>, &mut Code)>,

    /// "Fact lookup" (source haddock, QueryRegs.hs:56).
    ///
    /// `lookupKeyValue :: Register 'Word -> Register 'BinaryOutputPtr ->
    /// Register 'BinaryOutputPtr -> Register 'Word -> Code ()`
    /// (:57-62). Arguments, in source order; no per-argument comments in
    /// the source. NOTE for row 110: the FIELD wraps the `lookupKey_`
    /// syscall handle (Codegen.hs:1657-1658).
    pub lookup_key_value: Box<dyn Fn(Register<Word>, Register<BinaryOutputPtr>, Register<BinaryOutputPtr>, Register<Word>, &mut Code)>,

    /// "Record a result" (source haddock, QueryRegs.hs:64).
    ///
    /// `result :: Register 'Word -> Register 'BinaryOutputPtr ->
    /// Register 'BinaryOutputPtr -> Register 'Word -> Code ()` (:65-70).
    /// Arguments, in source order; no per-argument comments in the
    /// source.
    pub result: Box<dyn Fn(Register<Word>, Register<BinaryOutputPtr>, Register<BinaryOutputPtr>, Register<Word>, &mut Code)>,

    /// "Record a result, with a given pid and optional recursive
    /// expansion" (source haddock, QueryRegs.hs:72).
    ///
    /// `resultWithPid :: Register 'Word -> Register 'BinaryOutputPtr ->
    /// Register 'BinaryOutputPtr -> Register 'Word -> Register 'Word ->
    /// Code ()` (:73-79). Arguments, in source order; no per-argument
    /// comments in the source.
    pub result_with_pid: Box<dyn Fn(Register<Word>, Register<BinaryOutputPtr>, Register<BinaryOutputPtr>, Register<Word>, Register<Word>, &mut Code)>,

    /// "Record a new derived fact" (source haddock, QueryRegs.hs:81).
    ///
    /// `newDerivedFact :: Register 'Word -> Register 'BinaryOutputPtr ->
    /// Register 'Word -> Register 'Word -> Code ()` (:82-87). Arguments,
    /// in source order; no per-argument comments in the source.
    pub new_derived_fact: Box<dyn Fn(Register<Word>, Register<BinaryOutputPtr>, Register<Word>, Register<Word>, &mut Code)>,

    /// `firstFreeId :: Register 'Word -> Code ()` (QueryRegs.hs:89-91);
    /// argument: `Register<Word>` — first free id.
    pub first_free_id: Box<dyn Fn(Register<Word>, &mut Code)>,

    /// `newSet :: Register 'Word -> Code ()` (QueryRegs.hs:93-95);
    /// argument: `Register<Word>` — (output) set token.
    pub new_set: Box<dyn Fn(Register<Word>, &mut Code)>,

    /// `insertOutputSet :: Register 'Word -> Register 'BinaryOutputPtr
    /// -> Code ()` (QueryRegs.hs:97-100). Arguments, in source order
    /// (:98-99):
    ///   * `Register<Word>` — set token
    ///   * `Register<BinaryOutputPtr>` (no comment in the source)
    pub insert_output_set: Box<dyn Fn(Register<Word>, Register<BinaryOutputPtr>, &mut Code)>,

    /// `setToArray :: Register 'Word -> Register 'BinaryOutputPtr ->
    /// Code ()` (QueryRegs.hs:102-105). Arguments, in source order
    /// (:103-104):
    ///   * `Register<Word>` — set token
    ///   * `Register<BinaryOutputPtr>` — (output) array
    pub set_to_array: Box<dyn Fn(Register<Word>, Register<BinaryOutputPtr>, &mut Code)>,

    /// `freeSet :: Register 'Word -> Code ()` (QueryRegs.hs:107-109);
    /// argument: `Register<Word>` — set token (invalid after this call).
    pub free_set: Box<dyn Fn(Register<Word>, &mut Code)>,

    /// `newWordSet :: Register 'Word -> Code ()` (QueryRegs.hs:111-113);
    /// argument: `Register<Word>` — (output) set token.
    pub new_word_set: Box<dyn Fn(Register<Word>, &mut Code)>,

    /// `insertWordSet :: Register 'Word -> Register 'Word -> Code ()`
    /// (QueryRegs.hs:115-118). Arguments, in source order (:116-117):
    ///   * `Register<Word>` — set token
    ///   * `Register<Word>` (no comment in the source)
    pub insert_word_set: Box<dyn Fn(Register<Word>, Register<Word>, &mut Code)>,

    /// `wordSetToArray :: Register 'Word -> Register 'BinaryOutputPtr
    /// -> Code ()` (QueryRegs.hs:120-123). Arguments, in source order
    /// (:121-122):
    ///   * `Register<Word>` — set token
    ///   * `Register<BinaryOutputPtr>` — (output) array
    pub word_set_to_array: Box<dyn Fn(Register<Word>, Register<BinaryOutputPtr>, &mut Code)>,

    /// `byteSetToByteArray :: Register 'Word -> Register 'BinaryOutputPtr
    /// -> Code ()` (QueryRegs.hs:125-128). Arguments, in source order
    /// (:126-127):
    ///   * `Register<Word>` — set token
    ///   * `Register<BinaryOutputPtr>` — (output) array
    pub byte_set_to_byte_array: Box<dyn Fn(Register<Word>, Register<BinaryOutputPtr>, &mut Code)>,

    /// `freeWordSet :: Register 'Word -> Code ()` (QueryRegs.hs:130-132);
    /// argument: `Register<Word>` — set token (invalid after this call).
    pub free_word_set: Box<dyn Fn(Register<Word>, &mut Code)>,

    /// "Unused, temporarily kept for backwards compatibility" (source
    /// haddock, QueryRegs.hs:134). Direct field, not an action:
    /// `saveState :: Register 'Word` (:135) — the value `generate`'s
    /// RegSupply hands the record constructor (Codegen.hs:1624-1629,
    /// lambda's 20th parameter).
    pub save_state: Register<Word>,

    /// "Maximum number of results to return" (source haddock,
    /// QueryRegs.hs:137). Direct field: `maxResults :: Register 'Word`
    /// (:138).
    pub max_results: Register<Word>,

    /// "Maximum number of bytes to return" (source haddock,
    /// QueryRegs.hs:140). Direct field: `maxBytes :: Register 'Word`
    /// (:141).
    pub max_bytes: Register<Word>,
}

// Marker inventory (reviewer counts the trailer against this): zero of
// every class — no open port todos, no perf deferrals, no source-defect
// reproductions, no unsafe blocks (so no safety contracts to carry).
// The cross-crate Code import is unresolvable until Step 4 lands the
// crate scaffolds — the expected draft state, recorded in DEPENDENCY
// TRUTH with the repo.rs:60-65 precedent, not carried as a marker.
//
// Coverage: every export of the Haskell module has a Rust counterpart —
// the module's sole export is `QueryRegs(..)` (QueryRegs.hs:10), whose
// field list IS the record; all 22 fields are `pub` (the `..` exports
// every selector; cross-crate construction by row 110 and storage by
// row 89 need them pub — the Register-field pub precedent is types.rs's
// own note). No constructor helper: row 110 builds the record with a
// struct literal, mirroring the source's record syntax. No trait impls,
// no derives (source has none). Row 110 is the fill site for every
// function field; row 89 stores the whole record; row 95 threads it.
//
// PORT STATUS: confidence=high todos=0
