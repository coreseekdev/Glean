// Ported from: glean/hs/Glean/RTS/Bytecode/Disassemble.hs (92 lines)
// Manifest unit: rust/glean-core/src/rts/bytecode/disassemble.rs
// (migration/manifest.tsv line 109, crate glean-core). RULE §4 (naming):
// one source file -> one target file; the manifest path is authoritative.
//
// MANIFEST RESOLUTION — the Step-3 task brief for this unit named the
// target `rust/glean-bytecode/src/bytecode/disassemble.rs`; manifest.tsv
// line 109 (read back with sed -n '109p') pins
// `rust/glean-core/src/rts/bytecode/disassemble.rs`, and RULE §4 makes the
// manifest authoritative. The manifest path is also what the sibling set
// already plans around: rust/glean-bytecode/src/bytecode/sys_calls.rs's
// header ("Rust wiring for those units", point 6) pins "Row 109 (glean-core
// rts/bytecode/disassemble.rs) takes the table as a `&[&str]` parameter",
// and the crate edge is coherent — this file consumes the SHIPPED
// crate-internal crate::rts::foreign::bytecode (row 18) plus the cross-crate
// glean-bytecode decode/types (rows 108/20, the same core→bytecode edge
// rts/bytecode/supply.rs already carries). The brief's path is therefore a
// stale copy, not an inventory finding (there is no inventory row to
// diverge from — see INVENTORY below). This file sits at the manifest path.
//
// BYTE-PARITY SURFACE (RULE §0, gate A1) — the emitted text is the
// old-vs-new `disassemble` identity surface: inventory.tsv row 64 (RTS/
// Foreign/Bytecode.hs inspect) says "inspect is THE disassembly surface
// that verifies the two-pass label-alloc identity check", and the
// label-bearing rows (inventory.tsv rows 37/39/42/80/81, inventory.codeflow
// rows 9-12) all close with "emitted bytecode must disassemble
// identically (old vs new via disassemble tool)". Every render detail
// below — spacing, the `$N` label naming order, the `show` escaping of
// literals, the `*** ERROR ***` line — is load-bearing for that gate. This
// file only RENDERS; it allocates no labels and emits no bytecode (the A1
// bind-point obligations live in the emitters: code.rs's `label`, and the
// rows above' two-pass planners).
//
// DEPENDENCY TRUTH (migration/depmap/edges.tsv — checked, not assumed):
//   OUT (Disassemble.hs is the source of edges 1231-1233):
//     :1231 -> glean/bytecode/Glean/Bytecode/Decode.hs — cross-crate,
//             manifest row 108 (rust/glean-bytecode/src/bytecode/decode.rs),
//             NOT YET LANDED -> ASSUMED-IMPORT 1 below.
//     :1232 -> glean/bytecode/Glean/Bytecode/Types.hs — cross-crate,
//             manifest row 20, SHIPPED at
//             rust/glean-bytecode/src/bytecode/types.rs (Label).
//     :1233 -> glean/hs/Glean/RTS/Foreign/Bytecode.hs — crate-internal,
//             manifest row 18, SHIPPED at
//             rust/glean-core/src/rts/foreign/bytecode.rs (inspect,
//             SubroutineCode, Subroutine<S>).
//   There is NO edge to Glean.RTS.Bytecode.Gen.Instruction (generated
//   build output, RULE §4 "never in the manifest" — owner: the generator
//   units, manifest rows 150-152) — the same treatment code.rs's header
//   gives that module; ASSUMED-IMPORT 2 below extends code.rs's sketch.
//   The brief's "Glean.Types, Glean.RTS.Types?" guess is resolved by the
//   import list: Disassemble.hs:12-25 imports ByteString, Functor, IntMap,
//   IntSet, List, Maybe, Text, Vector.Storable and the three module imports
//   above — neither Glean.Types nor Glean.RTS.Types appears, and edges.tsv
//   pairs no such row.
//   IN (Disassemble.hs is the target of edges 541, 702, and the tools edge
//   1692): Query/Codegen.hs (row 110, not yet landed), Query/UserQuery.hs
//   (row 144, not yet landed), tools/disassemble/Disassemble.hs (row 368,
//   not yet landed). All three consume the SAME shape pinned below, which
//   is why the syscall table is a PARAMETER (next block).
//
// INVENTORY (RULE §5): NO row in any of the six migration/inventory*.tsv
// files governs glean/hs/Glean/RTS/Bytecode/Disassemble.hs — grep -i
// "disassemble" over inventory{,.ffi,.protocol,.sites,.stm,.codeflow}.tsv:
// the only hits are the CONSUMER rows quoted above (inventory.tsv rows
// 37/39/42/64/80/81; inventory.codeflow.tsv rows 9-12), whose file column
// is Codegen.hs/Transform.hs/Traverse.hs/Typecheck.hs/Foreign/Bytecode.hs —
// none is keyed to this module. Same shape as the shipped siblings'
// findings (bytecode/types.rs and bytecode/sys_calls.rs headers, and
// rts/bytecode/code.rs's INVENTORY block). The zero-row result is CORRECT,
// not a row to invent: the gap inventory records sites needing a decision
// ("ownership/lifetimes, effect order, exception taxonomy, laziness, STM
// transactions, JSON parity" — §5); this module is a pure text renderer —
// no FFI, no STM, no partial functions of its own (the one `!!` in its
// orbit is inside the GENERATED insnShow, pinned to that contract in
// ASSUMED-IMPORT 2), no exceptions, no unsafePerformIO. Nothing here is an
// inventory-owned class; no INVENTORY-BUG-CANDIDATE (no row exists to
// diverge from — RULE §5/A9 fires only on diverging from a row).
//
// SIBLING CONTRACTS (RULE §4; DONE units' real APIs read, never
// re-declared):
//   * crate::rts::foreign::bytecode (SHIPPED, read): `Subroutine<S>` with
//     the phantom kind marker (Bytecode.hs:55), `inspect<S>(&Subroutine<S>)
//     -> SubroutineCode` (inventory row 63: pure fn with owned copies) and
//     `SubroutineCode { sub_insns: Vec<u64>, sub_inputs: u64,
//     sub_outputs: u64, sub_locals: u64, sub_constants: Vec<u64>,
//     sub_literals: Vec<Vec<u8>> }` (field order = Bytecode.hs:32-50).
//     This file is the consumer that turns that surface into the
//     identity-gate text.
//   * glean_bytecode::bytecode::types (SHIPPED, read): `Label(pub i64)` —
//     `.0` is fromLabel (Types.hs:51; the payload is Haskell `Int`, i64 per
//     the numeric row, because Decode.hs:46 wraps the decoded Word64).
//   * glean_bytecode::bytecode::sys_calls (SHIPPED, read): the syscall
//     tables are passed BY VALUE-OF-REFERENCE as a `&[&str]` parameter —
//     that file's header (CONSUMER CONTRACT point 1 + wiring point 6) pins
//     exactly this: "Row 109 ... takes the table as a `&[&str]` parameter
//     and imports nothing from here". Honored: there is NO import of
//     sys_calls below; the source's import list (Disassemble.hs:12-25)
//     contains no Glean.Bytecode.SysCalls — the callers hand the table in
//     (Codegen.hs:295 and UserQuery.hs:844 pass userQuerySysCalls; tools/
//     disassemble/Disassemble.hs:83-89 passes typecheckSysCalls). sys_calls
//     also records there (point 4) that the `!!` partiality of
//     `(syscalls !! fromIntegral (fromRegister fun))` is the obligation of
//     the insnShow/disassemble ports — pinned in ASSUMED-IMPORT 2.
//
// ASSUMED-IMPORTS (RULE §4 draft-stub policy per stress-report-r2 B4:
// sibling-unit / generated items are assumed-imports with an in-file
// contract comment, never in-file type re-declarations):
//
//   1. glean_bytecode::bytecode::decode (manifest row 108, translated in
//     PARALLEL — this file was written WITHOUT reading it; owner:
//     rust/glean-bytecode/src/bytecode/decode.rs):
//         pub fn decode_all<T: Decodable>(words: &[u64]) -> (Vec<T>, Vec<u64>)
//     Source: `decodeAll :: Decodable a => [Word64] -> ([a], [Word64])`
//     (Decode.hs:56-64) — "Decode as many elements as possible and return
//     the decoded elements and the rest of the words, starting with the
//     first that can't be decoded"; a failed element decode yields NOTHING
//     for that element and the rest STARTS AT THE FAILED WORD (runDecode
//     discards the partial state, Decode.hs:28-31 — the MaybeT(State) run
//     backtracks the whole element). The slice parameter is the
//     V.toList subInsns borrow. `Decodable` is that module's trait; the
//     `Insn` instance lives in the GENERATED module, exactly as in Haskell
//     (`instance D.Decodable Insn`, Generate/Haskell.hs:269-282; generated
//     Instruction.hs:238-302: decode the opcode word, then N argument
//     words per the instruction's arity — 60 opcodes, `59 -> pure Ret` at
//     :301 — and `fail $ "invalid opcode " ++ show op` on any other opcode
//     at :302, which is what makes `rest` non-empty and drives the
//     `*** ERROR ***` line at Disassemble.hs:48).
//
//   2. crate::rts::bytecode::gen::instruction — the GENERATED instruction
//     module (Generate/Haskell.hs genModule "Instruction", :51-70; build
//     output under OUT_DIR per RULE §4; owner: the generator port, manifest
//     rows 150-152). This EXTENDS the surface code.rs's ASSUMED-IMPORTS 1
//     sketched (B4: a consumer extending an assumed surface records the
//     extension in its own block and cross-references the first sketch —
//     the typed/build.rs -> typed/binary.rs precedent). code.rs pinned:
//     `pub enum Insn { .. }` (every constructor of the def table, Insn:
//     Clone), `insn_size(&Insn) -> u64`, `insn_labels(&Insn) -> Vec<Label>`,
//     `map_labels`, `insn_words`. THE EXTENSION this unit consumes:
//         pub fn insn_show(
//             syscalls: &[&str],
//             show_label: &dyn Fn(Label) -> String,
//             show_reg: &dyn Fn(u64) -> String,
//             insn: &Insn,
//         ) -> String;
//     Source: `insnShow :: [String] -> (Label -> String) -> (forall t.
//     Register t -> String) -> Insn -> String` (Generate/Haskell.hs:290-295).
//     The Rank-2 register callback is erased to the register WORD exactly
//     per code.rs's insn_words precedent ("the phantom tag carries NO
//     runtime information ... the contract erases the polymorphism to the
//     register WORD (fromRegister, Types.hs:44)"): show_reg receives
//     fromRegister. The `[String]` syscall table arrives as `&[&str]` (the
//     same shape this unit's own parameter carries, per the sys_calls
//     contract above). RENDERING CONTRACT (pinned against the real
//     generated module at ~/glean-build/Glean/glean/hs/Glean/RTS/Bytecode/
//     Gen/Instruction.hs — the ~/glean-build checkout the sibling headers
//     declare as the source dep; its def/Generate/{Instruction,Haskell}.hs
//     and RTS/Bytecode/Disassemble.hs are diff-verified byte-identical to
//     this repo's — and against the generator, Generate/Haskell.hs:287-315):
//       * the line is mnemonic ++ elements joined by the literal string
//         "," — generated code shape `concat [MNEMONIC, ' ' : render1,
//         ",", ' ' : render2, ...]` (generated Instruction.hs:335:
//         `concat ["AddConst", ' ' : show imm, ",", ' ' : showReg dst]`),
//         i.e. "AddConst <imm>,<sp><dst>" — mnemonic, ONE space, then
//         args separated by comma-space;
//       * MNEMONIC is the instruction name in quotes, EXCEPT for the
//         CallFun_* family: `(syscalls !! fromIntegral (fromRegister fun))`
//         (Generate/Haskell.hs:312-315, generated :351) — the `fun`
//         register word is an INDEX into the syscall table and the first
//         argument is not rendered. Haskell `!!` partiality (RULE §2
//         partial-functions row: same message, bug-for-bug; sys_calls.rs
//         CONSUMER CONTRACT point 4 assigns it to this port): the index is
//         `fromIntegral (fromRegister fun) :: Int` — the Word64 wrapped to
//         i64 — so a negative index panics "Prelude.!!: negative index"
//         and an out-of-range one "Prelude.!!: index too large"
//         (base-4.17.1.0 GHC/List.hs:1355-1356, worker :1368);
//       * per-argument renders (Generate/Haskell.hs:304-311): ImmOffset ->
//         showLabel lbl (this unit's closure); ImmLit -> `show (fromLiteral
//         lit)` — unsigned decimal (Word64 show = showsPrec via toInteger,
//         base-4.17.1.0 GHC/Word.hs:841-842); ImmWord -> `show w` (same
//         decimal); Reg/Regs -> showReg word; Offsets (the Select
//         instruction, def/Instruction.hs:381-385) -> `showListWith
//         (showString . showLabel) tgts ""` (generated :364) =
//         showList__ = "[" ++ elems joined "," ++ "]" — brackets, comma, NO
//         space (Text/Show.hs:33-34 `showListWith = showList__`; GHC/Show.hs
//         :151-156).
//
// SHOW PINS — the text this unit formats ITSELF (everything above the
// instruction lines is rendered HERE, so its byte-parity evidence is
// carried in full):
//   * Decimal shows (`show (length ...)`, `show (V.length ...)`,
//     `show subInputs`, `show i`, `show c`, `show n`, the `show k` label
//     fallback): Haskell `show` on Int/Word64 is signed/unsigned decimal
//     (`showSignedInt`; Word64 via toInteger, GHC/Word.hs:841-842) —
//     `format!("{}", …)` on usize/u64/i64 is the same digit string,
//     including the minus sign for negative Int labels.
//   * `show lit` on a ByteString literal (Disassemble.hs:34 via
//     `sum $ map BS.length` is a plain Int; :40 via `show lit`) is the
//     non-trivial one — see show_byte_string below for the pinned
//     bytestring + GHC.Show chain.
//   * Label NAMES: `'$' : show i` (Disassemble.hs:57) — "$0", "$1", …
//     assigned in ASCENDING target-offset order: `IntMap.mapAccum` threads
//     its accumulator "through the map in ascending order of keys"
//     (containers v0.6.7 Data/IntMap/Internal.hs:2466-2471, mapAccum ->
//     mapAccumWithKey -> mapAccumL; the ascending contract is stable across
//     the 0.6/0.7 line GHC 9.2/9.4/9.6 bundle). `IntMap.fromSet (const ())`
//     (Disassemble.hs:59) preserves the IntSet's distinct sorted keys, so a
//     BTreeSet iterated ascending with an enumerate() counter reproduces
//     the naming exactly.
//
// DESIGN (RULE §0 — translating, not improving), per source line:
//   * `disassemble :: Text -> [String] -> Subroutine s -> [Text]`
//     (Disassemble.hs:27-28) -> `pub fn disassemble<S>(name: &str,
//     syscalls: &[&str], sub: &Subroutine<S>) -> Vec<String>`: Text -> &str
//     and Text -> String per RULE §1 (strings row); [String] -> &[&str] per
//     the sys_calls pinned contract; generic in S because the source is
//     (Subroutine s, Bytecode.hs:55) and all three call sites instantiate
//     it differently (CompiledQuery / CompiledTypecheck).
//   * `SubroutineCode{..} = inspect sub` (Disassemble.hs:50) -> one
//     destructuring let of all six fields (the record is an owned snapshot
//     per inventory row 63, so moving out of it is the same value).
//   * `(instructions, rest) = D.decodeAll $ V.toList subInsns`
//     (Disassemble.hs:52-53) -> decode_all::<Insn>(&sub_insns).
//   * The `labels` fold (Disassemble.hs:55-71): foldl' over the decoded
//     instructions with the accumulator = offset AFTER the current
//     instruction (`n = m + insnSize insn`), inserting every label of the
//     instruction at `fromIntegral n + fromLabel o` — i.e. each stored
//     label is relative to the START OF THE NEXT instruction (the `Offset`
//     doc, Types.hs:27). Word64 accumulation and Int arithmetic wrap in
//     GHC, so both adds are wrapping_add; `fromIntegral` is the
//     two's-complement `as i64` cast (RULE §2 numeric row: the source's
//     wrap/truncation semantics, explicit).
//   * `show_label o (Label k)` (Disassemble.hs:73-74): absolute target =
//     `fromIntegral o + k` (o = offset after the CURRENT instruction, fed
//     by the code loop); found -> the `$N` name, else `show k` — the
//     numeric fallback is reachable for labels pointing outside the
//     decoded stream (e.g. past its end) and is part of the gate surface.
//   * `show_reg (Register n) = '%' : show n` (Disassemble.hs:76).
//   * `code` (Disassemble.hs:78-92): mapAccumL with the offset
//     accumulator; for each instruction a `$N:` line is emitted when the
//     INSTRUCTION'S START offset (o — the pre-increment value, NOT o')
//     is a target (`IntMap.lookup (fromIntegral o) labels`), then the
//     instruction line `"  " <> insnShow syscalls (show_label o')
//     show_reg insn` — two-space indent, label resolution bound at o' = o
//     + size. catMaybes over the single Maybe -> if-let.
//   * The header block (Disassemble.hs:29-38), literal lines (:40-41),
//     constant lines (:43-44, indices continue at subInputs — `zip
//     [subInputs..]`, enumFrom on Word64 wraps at maxBound, hence
//     wrapping_add), the "" separator (:46), code (:47) and
//     `["*** ERROR ***" | not $ null rest]` (:48) assemble in exactly that
//     order into the returned Vec<String>.
//   * `sub_literals: Vec<Vec<u8>>` (row 63's owned copies; RULE §1 bytes
//     row) — `BS.length` sums to byte counts, `show lit` is replicated by
//     show_byte_string below.

use std::collections::{BTreeMap, BTreeSet};

use glean_bytecode::bytecode::decode::decode_all;
use glean_bytecode::bytecode::types::Label;

use crate::rts::bytecode::gen::instruction::{insn_labels, insn_show, insn_size, Insn};
use crate::rts::foreign::bytecode::{inspect, Subroutine, SubroutineCode};

/// `disassemble :: Text -> [String] -> Subroutine s -> [Text]`
/// (Disassemble.hs:27-28) — render a subroutine's bytecode as the
/// disassembly lines the RULE §0 (A1) old-vs-new identity gate compares.
///
/// `syscalls` is the caller's name table (glean_bytecode's
/// USER_QUERY_SYS_CALLS for the query consumers, TYPECHECK_SYS_CALLS for
/// the tools dump) — passed through verbatim to the generated `insn_show`,
/// which indexes it by the `fun` register word of CallFun instructions.
/// This unit deliberately does NOT import that module (see SIBLING
/// CONTRACTS).
pub fn disassemble<S>(name: &str, syscalls: &[&str], sub: &Subroutine<S>) -> Vec<String> {
    // `SubroutineCode{..} = inspect sub` (Disassemble.hs:50) — one pattern
    // binding of every field.
    let SubroutineCode {
        sub_insns,
        sub_inputs,
        sub_outputs,
        sub_locals,
        sub_constants,
        sub_literals,
    } = inspect(sub);

    // `(instructions, rest) = D.decodeAll $ V.toList subInsns`
    // (Disassemble.hs:52-53) — decode until the first element that cannot
    // be decoded; `rest` is the undecoded tail INCLUDING the failing word.
    let (instructions, rest) = decode_all::<Insn>(&sub_insns);

    // `labels` (Disassemble.hs:55-71) — first the fold: the accumulator is
    // the offset after the current instruction; every label of the
    // instruction is inserted at `fromIntegral n + fromLabel o`.
    let mut offset: u64 = 0;
    let mut targets: BTreeSet<i64> = BTreeSet::new();
    for insn in &instructions {
        // `let !n = m + insnSize insn` — Word64 addition, wraps in GHC
        // (RULE §2 numeric row: the source's wrap semantics, explicit).
        offset = offset.wrapping_add(insn_size(insn));
        for lbl in insn_labels(insn) {
            // `fromIntegral n + fromLabel o` — Word64->Int is the
            // two's-complement cast `as i64`; Int addition wraps.
            targets.insert((offset as i64).wrapping_add(lbl.0));
        }
    }
    // `IntMap.fromSet (const ())` then `IntMap.mapAccum (\i _ -> (i+1,
    // '$' : show i)) 0` (Disassemble.hs:56-59): distinct targets in
    // ASCENDING key order get the names "$0", "$1", … (mapAccum's
    // ascending-order contract — SHOW PINS above). BTreeSet/BTreeMap
    // iteration is the same order.
    let mut labels: BTreeMap<i64, String> = BTreeMap::new();
    for (i, target) in targets.iter().enumerate() {
        labels.insert(*target, format!("${}", i));
    }

    // `show_label o (Label k) = fromMaybe (show k) $ IntMap.lookup
    // (fromIntegral o + k) labels` (Disassemble.hs:73-74).
    let show_label = |o: u64, k: Label| -> String {
        match labels.get(&((o as i64).wrapping_add(k.0))) {
            Some(name) => name.clone(),
            None => format!("{}", k.0),
        }
    };

    // `show_reg (Register n) = '%' : show n` (Disassemble.hs:76) — the
    // Rank-2 callback erased to the register word (ASSUMED-IMPORT 2).
    let show_reg = |n: u64| format!("%{}", n);

    // `code` (Disassemble.hs:78-92) — mapAccumL with the offset
    // accumulator: a `$N:` line when the instruction's START offset (the
    // pre-increment `o`) is a target, then the two-space-indented
    // instruction line with label resolution bound at `o'`.
    let mut code: Vec<String> = Vec::new();
    let mut o: u64 = 0;
    for insn in &instructions {
        let o_prime = o.wrapping_add(insn_size(insn));
        if let Some(label) = labels.get(&(o as i64)) {
            code.push(format!("{}:", label));
        }
        let sl = |k: Label| show_label(o_prime, k);
        code.push(format!("  {}", insn_show(syscalls, &sl, &show_reg, insn)));
        o = o_prime;
    }

    // The output list, in the source's literal order (Disassemble.hs:28-48).
    let mut out: Vec<String> = Vec::new();
    // "Subroutine " <> name (:29)
    out.push(format!("Subroutine {}", name));
    // "  // Code: " <> show (length instructions) <> " insns, "
    //   <> show (V.length subInsns) <> " bytes" (:30-32)
    //
    // BUG(port): the second count is V.length subInsns — a count of Word64
    // WORDS (subInsns :: V.Vector Word64, Foreign/Bytecode.hs:34) — printed
    // under the label "bytes". Repro: any subroutine whose instruction
    // stream holds 4 words prints "4 bytes". Reproduced verbatim (the old
    // code is the spec; the identity gate compares this text).
    out.push(format!(
        "  // Code: {} insns, {} bytes",
        instructions.len(),
        sub_insns.len()
    ));
    // "  // Literals: " <> show (sum $ map BS.length subLiterals) <> " bytes"
    // (:33-35) — this one IS a byte count.
    out.push(format!(
        "  // Literals: {} bytes",
        sub_literals.iter().map(|lit| lit.len()).sum::<usize>()
    ));
    // "  #inputs = " <> show subInputs (:36); outputs/locals likewise.
    out.push(format!("  #inputs = {}", sub_inputs));
    out.push(format!("  #outputs = {}", sub_outputs));
    out.push(format!("  #locals = {}", sub_locals));
    // "  #" <> show i <> " = " <> show lit  (Disassemble.hs:40-41)
    for (i, lit) in sub_literals.iter().enumerate() {
        out.push(format!("  #{} = {}", i, show_byte_string(lit)));
    }
    // "  %" <> show i <> " = " <> show c  (Disassemble.hs:43-44) — indices
    // continue at subInputs (`zip [subInputs..]`); enumFrom on Word64
    // wraps at maxBound.
    let mut i = sub_inputs;
    for c in &sub_constants {
        out.push(format!("  %{} = {}", i, c));
        i = i.wrapping_add(1);
    }
    // "" (:46)
    out.push(String::new());
    // code (:47)
    out.extend(code);
    // "*** ERROR ***" when the decode stopped early (:48)
    if !rest.is_empty() {
        out.push("*** ERROR ***".to_string());
    }
    out
}

/// `show lit` for a literal ByteString (Disassemble.hs:34/:40).
///
/// bytestring's Show instance:
/// ```haskell
/// instance Show ByteString where
///     showsPrec p ps r = showsPrec p (unpackChars ps) r
/// ```
/// (bytestring 0.11.4.0 Data/ByteString/Internal/Type.hs:276-277; the
/// identical text at 0.12.2.0 Type.hs:330-331 — the instance is stable
/// across the line the CI GHCs bundle, .github/workflows/ci.yml:13).
/// `unpackChars` maps every byte through `w2c = unsafeChr . fromIntegral`
/// (Type.hs:839-840) — the char with the SAME code point as the byte, so
/// no UTF-8 decoding happens — via unpackAppendCharsLazy (Type.hs:483-484);
/// that lazy list is fully demanded by show, so the eager loop below
/// computes the same value (RULE §1 bans lazy IO, not lazy lists; nothing
/// here observes the laziness). `showsPrec` on the resulting String is the
/// string-literal showList (base-4.17.1.0 GHC/Show.hs:188-192):
/// `"` ++ showLitString cs ++ `"`, where showLitString (:389-398) escapes
/// the double quote itself and hands every other char to showLitChar
/// (:372-385), in clause order:
///   * c > '\DEL'  -> '\' then `show (ord c)` (decimal, 128..255), with a
///     "\&" gap inserted when the NEXT character of the string is a decimal
///     digit — protectEsc isDec (:373, isDec :419-420, protectEsc
///     :422-425: `cont s@(c:_) | p c = "\\&" ++ s` peeks the continuation);
///   * '\DEL' -> "\DEL" (:374); '\\' -> "\\\\" (:375);
///   * c >= ' ' -> the character itself (:376) — note this makes NO
///     special case for '"', which is exactly why showLitString must
///     intercept it first (:397);
///   * '\a' '\b' '\f' '\n' '\r' '\t' '\v' -> the named escapes
///     (:377-383);
///   * '\SO' -> "\SO", with a "\&" gap when the next char is 'H' (:384)
///     (else "\SOH" would re-lex as the single char '\SOH');
///   * anything else (0x00..0x1F) -> '\' then asciiTab!!ord c (:385, tab
///     at :428-433).
fn show_byte_string(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() + 2);
    s.push('"');
    for (i, &b) in bytes.iter().enumerate() {
        // The gap rules peek at the NEXT character of the string
        // (protectEsc's `cont s@(c:_)`, GHC/Show.hs:423-424); w2c is 1:1
        // byte -> char, so the next string char is simply the next byte.
        // No next char -> `cont s = s`, no gap.
        let next = bytes.get(i + 1).copied();
        match b {
            // showLitString's '"' clause (GHC/Show.hs:397)
            b'"' => s.push_str("\\\""),
            // showLitChar '\\' (:375)
            b'\\' => s.push_str("\\\\"),
            // '\DEL' (:374)
            0x7F => s.push_str("\\DEL"),
            // the named escapes, in the source's clause order (:377-383)
            0x07 => s.push_str("\\a"),
            0x08 => s.push_str("\\b"),
            0x0C => s.push_str("\\f"),
            0x0A => s.push_str("\\n"),
            0x0D => s.push_str("\\r"),
            0x09 => s.push_str("\\t"),
            0x0B => s.push_str("\\v"),
            // '\SO' + protectEsc (== 'H') (:384)
            0x0E => {
                s.push_str("\\SO");
                if next == Some(b'H') {
                    s.push_str("\\&");
                }
            }
            // asciiTab!!ord c (:385) — the remaining 0x00..0x1F; the tab's
            // 33rd entry ("SP", index 0x20) is dead in Haskell too (the
            // `c >= ' '` clause takes it first).
            0x00..=0x1F => {
                s.push('\\');
                s.push_str(ASCII_TAB[b as usize]);
            }
            // c >= ' ' (:376) — printable ASCII, w2c is the identity there
            0x20..=0x7E => s.push(b as char),
            // c > '\DEL' (:373) — '\' ++ show (ord c); ord (w2c b) == b, so
            // the escape is the decimal byte value, with the "\&" gap when
            // the next char is a decimal digit (protectEsc isDec,
            // :373/:419-420/:422-425)
            0x80..=0xFF => {
                s.push('\\');
                s.push_str(&b.to_string());
                if matches!(next, Some(n) if n.is_ascii_digit()) {
                    s.push_str("\\&");
                }
            }
        }
    }
    s.push('"');
    s
}

/// GHC/Show.hs:428-434 (base-4.17.1.0) — verbatim, 33 entries for
/// 0x00..0x20; index 0x20 ("SP") is unreachable (see show_byte_string).
const ASCII_TAB: [&str; 33] = [
    "NUL", "SOH", "STX", "ETX", "EOT", "ENQ", "ACK", "BEL", //
    "BS", "HT", "LF", "VT", "FF", "CR", "SO", "SI", //
    "DLE", "DC1", "DC2", "DC3", "DC4", "NAK", "SYN", "ETB", //
    "CAN", "EM", "SUB", "ESC", "FS", "GS", "RS", "US", //
    "SP",
];

// Marker breakdown (closed set — reviewer counts the trailer against this):
//   * 1 BUG(port): the "// Code: ... bytes" line prints a WORD count under
//     a byte label (Disassemble.hs:30-32 vs Foreign/Bytecode.hs:34) —
//     reproduced bug-for-bug with a repro at the site (RULE BUG rule).
//   * 0 TODO(port): every symbol this file consumes is either SHIPPED
//     (foreign/bytecode.rs, bytecode/types.rs, bytecode/sys_calls.rs — the
//     last deliberately unimported) or pinned as an ASSUMED-IMPORT with a
//     complete contract (decode_all / the generated insn_show); the
//     UNKNOWN rule has nothing left open here.
//   * 0 PERF(port): the label pass is two O(insns) loops plus an O(targets
//     log targets) map build against the source's IntSet/IntMap — same
//     complexity class; show_byte_string is the source's own show.
//   * 0 INVENTORY-BUG-CANDIDATE: no inventory row governs this file (the
//     zero-row finding is the INVENTORY block at the top; RULE §5/A9 fires
//     only on diverging from a row).
//   * 0 unsafe / SAFETY: this unit performs no FFI and no raw pointers —
//     RULE §3's escape hatch never opens here.
//
// Coverage: every binding of the Haskell module has a Rust counterpart —
// disassemble [disassemble<S>]; the where-block: labels [the two-stage
// fold + naming loop], show_label [the closure], show_reg [the closure],
// code [the mapAccumL loop]; the imported library calls: inspect [SHIPPED],
// D.decodeAll [ASSUMED-IMPORT 1], insnSize/insnLabels/insnShow [generated
// module, ASSUMED-IMPORT 2 + code.rs's sketch], BS.length [Vec::len],
// V.length [Vec::len], IntMap/IntSet machinery [BTreeSet/BTreeMap with the
// ascending-order pin], show on Int/Word64/ByteString [format! +
// show_byte_string], Text.pack/<> [format!], catMaybes/<&> [if-let],
// mapAccumL/foldl' [loops], fromMaybe [match]. The Haskell list
// comprehension for constants continues indices at subInputs — kept.
// Nothing in the source is dropped.
//
// MODULE-WIRING: lib.rs / rts/mod.rs / rts/bytecode/mod.rs scaffolding is
// deliberately NOT a manifest unit; owned by the Step-4 survey build (do
// not create ad-hoc here — same ruling as code.rs, supply.rs and
// rts/types.rs).
//
// PORT STATUS: confidence=medium todos=0
