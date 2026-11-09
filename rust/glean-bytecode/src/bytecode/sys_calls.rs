// Ported from: glean/bytecode/Glean/Bytecode/SysCalls.hs (32 content lines)
//
// MANIFEST RESOLUTION — manifest.tsv line 107:
//   glean/bytecode/Glean/Bytecode/SysCalls.hs
//     → rust/glean-bytecode/src/bytecode/sys_calls.rs
// (read back with sed -n '107p' migration/manifest.tsv; this file sits at
// exactly that path; crate glean-bytecode, migration/depmap/crates.txt.)
//
// INVENTORY (RULE §5): NO row in any of the six migration/inventory*.tsv
// files governs SysCalls.hs (grep "SysCalls" over inventory{,.ffi,.protocol,
// .sites,.stm,.codeflow}.tsv on 2026-11-09: zero hits — the module's path
// never occurs even in evidence columns). Same shape as the sibling
// finding in bytecode/types.rs. This zero-row result is CORRECT, not a
// missing row to invent: the gap inventory records sites needing a decision
// ("ownership/lifetimes, effect order, exception taxonomy, laziness, STM
// transactions, JSON parity" — §5); this module contains no such construct
// (see SIBLING CONTRACTS below — it has no imports at all). No
// NOTE (not a §5 flag — no row exists to diverge from; see the INVENTORY block): the §5/A9 protocol fires on diverging from a
// row's target_translation, and there is no row to diverge from — the
// zero-row finding is recorded here instead (types.rs header precedent,
// which shipped with the identical treatment). The pre-survey claim
// ("SysCalls.hs is two [String] lists") is VERIFIED against the source: the
// module declares exactly two top-level bindings, both `[String]`
// (SysCalls.hs:16 and :27), and nothing else.
//
// SIBLING CONTRACTS (RULE §4; DONE crates' real APIs read, never
// re-declared):
//   - bytecode/types.rs (the crate's shipped module) is NOT consumed here:
//     the Haskell module imports NOTHING — there is no `import` declaration
//     anywhere in SysCalls.hs (module header :9-12, then the first
//     definition at :16). This file therefore has zero `use` statements and
//     declares no types of its own beyond the two tables. Reviewer B's
//     "no other consumers needing Types.hs" is verified: no consumer needs
//     Types.hs *for these lists* (Codegen.hs/UserQuery.hs/Disassemble.hs
//     import Types for their own bytecode plumbing, not to pass the list).
//   - glean-core's rts/bytecode modules: NO edge between them and this
//     module. edges.tsv rows 530 (Codegen.hs), 681 (UserQuery.hs),
//     1688 (Disassemble.hs) are the ONLY edges touching SysCalls.hs — all
//     inbound from db/db/tools, zero outbound (the source has no imports).
//     In particular manifest row 109 (rust/glean-core/src/rts/bytecode/
//     disassemble.rs) must NOT import this module: the source `disassemble
//     :: Text -> [String] -> Subroutine s -> [Text]` RECEIVES the table as
//     a parameter (RTS/Bytecode/Disassemble.hs:27-28; its import list
//     :11-17 contains no Glean.Bytecode.SysCalls) — the db/cli callers pass
//     it in. Crate edges carried by the consumers: glean-db and glean-cli
//     → glean-bytecode (rows 110/144/368, RULE §4 cross-crate import row);
//     the core→bytecode edge types.rs documents (via supply.rs) is
//     unrelated to this unit.
//
// CONSUMER CONTRACT (RULE §4 — who consumes the name lists and why; the
// known trap for this unit was exactly here: the lists feed disassembly
// rendering, and their ORDER is the syscall numbering):
//   1. Consumption chain. The list goes to `disassemble`
//      (RTS/Bytecode/Disassemble.hs:32), which passes it to the GENERATED
//      `insnShow :: [String] -> (Label -> String)
//      -> (forall t. Register t -> String) -> Insn -> String`
//      (module Glean.RTS.Bytecode.Gen.Instruction — build output, owned by
//      Generate/Haskell.hs:289-314, never a manifest unit per RULE §4).
//      For every CallFun_N_M instruction the printed mnemonic is
//      `(syscalls !! fromIntegral (fromRegister fun))` (Generate/
//      Haskell.hs:314): the instruction's `fun` register (Arg "fun" $
//      reg (Fun […]), def/Instruction.hs:326-379) is an INDEX into this
//      table, not a pointer.
//   2. Where the index comes from. The emitter lambda binds the syscall
//      handles as the subroutine's first N input registers, positionally:
//      queries — generateQueryCode binds seek_ … freeWordSet_ (19 handles,
//      Query/Codegen.hs:1624-1627 (freeWordSet_ at :1627)); typecheck — checkSignature binds
//      rename_ … freeWordSet_ (10 handles, RTS/Typecheck.hs:209-212).
//      At runtime the C++ RTS pre-loads those input slots with the syscall
//      function pointers, in the SAME positional order: query.cpp:601-621
//      `syscalls<&QueryExecutor::seek, …, &QueryExecutor::freeWordSet>`
//      then std::copy(handlers_begin, handlers_end, args) (:633);
//      inventory.h:100-110 `syscalls<&Rename<F>::rename, …,
//      &Rename<F>::freeWordSet>` for the typechecker. The encoded `fun`
//      word is the raw input-slot index (Code.hs get_reg, `Input -> n`).
//      THREE-WAY sync therefore holds: Haskell emitter lambda ↔ C++
//      `syscalls<…>` method list ↔ these name tables. Desync = "the wrong
//      name will be printed" (source comments, SysCalls.hs:14-15 and
//      :25-26) — text only: execution reads the function POINTERS in the
//      slots and never consults these lists, so this module is NOT on the
//      A1 bytecode-identity path (it emits no bytecode, allocates no
//      labels).
//   3. The 19 is exact — the emitter lambdas bind 3 non-syscall args after
//      the syscalls (saveState/maxResults/maxBytes; clause_begin/key_end/
//      clause_end for typecheck). saveState is NOT a syscall: Suspend's
//      first argument is `Arg "unused" $ reg Word Load` — "a temporary,
//      unused left-over for backwards compatibility" (def/
//      Instruction.hs:400-402); query.cpp:634 fills it with `*args++ = 0;
//      // unused`.
//   4. `!!` partiality (pinned obligation of the insnShow/disassemble
//      ports, NOT of this module — RULE §2 partial-functions row governs
//      that unit): an out-of-range `fun` register errors at PRINT time
//      with Haskell's "Prelude.!!: index too large" (Generate/
//      Haskell.hs:314; pass-through at RTS/Bytecode/Disassemble.hs:90).
//   5. Call sites — exhaustive (grep "userQuerySysCalls|typecheckSysCalls"
//      over the tree, 2026-11-09: 6 references outside the defining
//      module = 3 imports + 3 uses):
//        - Query/Codegen.hs:295 — vlog 3 dump of the compiled query.
//        - Query/UserQuery.hs:840-845 — bytecodeDiag, emitted under the
//          userQueryOptions_debug/queryDebugOptions_bytecode flag.
//        - tools/disassemble/Disassemble.hs:83-89 (typecheckSysCalls arg at :88) — the `ptc_` predicate-
//          typechecker dump; the ONLY consumer of typecheckSysCalls.
//   6. Rust wiring for those units: manifest rows 110
//      (glean-db/src/query/codegen.rs) and 144 (glean-db/src/query/
//      user_query.rs) will consume USER_QUERY_SYS_CALLS (not yet written); row 368
//      (glean-cli/src/disassemble.rs) consumes TYPECHECK_SYS_CALLS —
//      each via `use glean_bytecode::bytecode::sys_calls::…` (RULE §4).
//      Row 109 (glean-core rts/bytecode/disassemble.rs) takes the table
//      as a `&[&str]` parameter and imports nothing from here (point 1
//      above). As of 2026-11-09 disassemble.rs is NOT yet written
//      (rust/glean-core/src/rts/bytecode/ holds only code.rs and
//      supply.rs) — the slice-parameter shape above is its pinned
//      contract when it lands.
//
// DESIGN (RULE §0 — translating, not improving):
//   - Each Haskell `[String]` binding is a total top-level constant (CAF),
//     ported as a `pub const` array of `&'static str` with an explicit
//     length. RULE §1 strings row: Haskell `String` → `&str` (UTF-8).
//     RULE §2 CAF row (`{-# NOINLINE #-}` MVar/IORef → once_cell) does NOT
//     apply: nothing here is stateful, and the source's list-laziness is
//     unobservable — every consumer demands the spine (`!!` walks it,
//     disassemble iterates it) on a pure, total table, so eager storage is
//     semantically identical. No once_cell, no Lazy. In-repo precedent for
//     a closed constant table with pinned length:
//     rust/glean-core/src/rts/foreign/lookup_cache.rs:607
//     (`pub const ALL_STATS: [Stat; STAT_COUNT]`).
//   - The explicit length is contract, not decoration: a dropped or
//     duplicated entry is precisely the "wrong name" defect the source
//     comments warn about; the initializer below must carry exactly N
//     elements in exactly the source order.
//   - ORDER IS LOAD-BEARING (it IS the syscall numbering — CONSUMER
//     CONTRACT point 2). The element grouping below mirrors the source's
//     line breaks (SysCalls.hs:17-23 and :28-32) so the two sides can be
//     eyeballed against each other during review.
//   - Element text is VERBATIM, quirks included (RULE §0 — the old code is
//     the spec; do not normalize): the query table prints "lookupKeyValue"
//     where the emitter binds `lookupKey_` (C++
//     QueryExecutor::lookupKeyValue) and "byteSetToArray" where the
//     emitter binds `byteSetToByteArray_` (C++ byteSetToByteArray); the
//     typecheck table's first five entries lack the trailing underscore
//     its emitter bindings all carry ("rename" vs `rename_`, …) while the
//     last five keep it ("newWordSet_" …). These are the PRINTED
//     mnemonics; the source pins them exactly as written.
//   - Zero `use` statements, zero new types: the source imports nothing
//     (see SIBLING CONTRACTS).

/// Names of the bytecode syscalls issued by compiled user queries, indexed
/// by the syscall's input-register slot (0-based).
///
/// Source (SysCalls.hs:14-15): "This list has to sync up with the list in
/// CodeGen.hs or the wrong name will be printed" — i.e. position-for-
/// position with the `generateQueryCode` lambda bindings (Query/Codegen.hs
/// :1624-1626) and with query.cpp's `syscalls<&QueryExecutor::…>` handler
/// list (:601-621). Consumers: the disassembly/vlog text dumps only
/// (CONSUMER CONTRACT point 5); execution never reads this table.
pub const USER_QUERY_SYS_CALLS: [&'static str; 19] = [
    "seek", "seekWithinSection", "currentSeek", "endSeek",
    "next", "lookupKeyValue", "result", "resultWithPid",
    "newDerivedFact", "firstFreeId",
    "newSet", "insertOutputSet", "setToArray", "freeSet",
    "newWordSet", "insertWordSet", "wordSetToArray", "byteSetToArray",
    "freeWordSet",
];

/// Names of the bytecode syscalls issued by compiled predicate
/// typecheckers, indexed by the syscall's input-register slot (0-based).
///
/// Source (SysCalls.hs:25-26): "This list has to sync up with the list in
/// Glean.RTS.Typecheck or the wrong name will be printed" — position-for-
/// position with the `checkSignature` lambda bindings (RTS/Typecheck.hs
/// :209-212) and with inventory.h's `syscalls<&Rename<F>::…>` handler list
/// (:104-114). Sole consumer: the disassemble tool's `ptc_` dump
/// (tools/disassemble/Disassemble.hs:83-89 (typecheckSysCalls arg at :88)).
pub const TYPECHECK_SYS_CALLS: [&'static str; 10] = [
    "rename", "newSet", "insertOutputSet", "setToArray", "freeSet",
    "newWordSet_", "insertBytesWordSet_", "wordSetToArray_",
    "byteSetToByteArray_", "freeWordSet_",
];

// Marker inventory (reviewer counts the trailer against this): zero
// TODO(port), zero PERF(port), zero BUG(port), zero unsafe/SAFETY blocks,
// zero INVENTORY-BUG-CANDIDATE flags (no inventory row exists to diverge
// from — the zero-row finding is the INVENTORY block at the top). The one
// partiality in the lists' orbit (`!!`, CONSUMER CONTRACT point 4) is
// pinned to its owning unit (the generated-insnShow/disassemble ports),
// so it raises no marker here.
//
// Coverage: every export of the Haskell module has a Rust counterpart —
// userQuerySysCalls [USER_QUERY_SYS_CALLS, 19 elements, order/content
// verbatim], typecheckSysCalls [TYPECHECK_SYS_CALLS, 10 elements,
// order/content verbatim]. The module header's export list
// (SysCalls.hs:10-11) is exactly those two names; nothing else exists in
// the 33-line source.
//
// MODULE-WIRING: lib.rs / bytecode/mod.rs scaffolding is deliberately NOT
// a manifest unit; owned by the Step-4 survey build (do not create ad-hoc
// here) — same ruling as bytecode/types.rs.
//
// PORT STATUS: confidence=medium todos=0
