// Ported from: glean/bytecode/Glean/Bytecode/Decode.hs (64 lines)
//
// MANIFEST RESOLUTION — manifest.tsv line 108 (read back with sed -n
// '108p'): glean/bytecode/Glean/Bytecode/Decode.hs →
// rust/glean-bytecode/src/bytecode/decode.rs. This file sits at exactly
// that path (crate glean-bytecode, migration/depmap/crates.txt line 2).
//
// INVENTORY (RULE §5): NO row in any of the six migration/inventory*.tsv
// files governs Decode.hs (grep -i "Decode.hs" over
// inventory{,.ffi,.protocol,.sites,.stm,.codeflow}.tsv: zero hits — the
// module's path never occurs, not even in evidence columns; the brief's
// "inventory.tsv rows for this file" resolves to the empty set). Same
// shape as the shipped sibling findings (bytecode/types.rs and
// bytecode/sys_calls.rs headers, and disassemble.rs's INVENTORY block).
// The zero-row result is CORRECT, not a row to invent: the gap inventory
// records sites needing a decision ("ownership/lifetimes, effect order,
// exception taxonomy, laziness, STM transactions, JSON parity" — §5);
// this module has no such construct — no FFI (its imports are
// Control.Monad, Control.Monad.State.Strict, Control.Monad.Trans.Maybe,
// Data.Word and Glean.Bytecode.Types only, Decode.hs:17-22), no STM, no
// laziness-dependent construct (State.Strict; every word is demanded by
// the cursor as it is pulled), no exceptions (the only failure is
// MaybeT's Nothing), no unsafePerformIO, no partial functions of its own
// (the one `fromIntegral` is the RULE §2 numeric-row wrap, made explicit
// below). No INVENTORY-BUG-CANDIDATE: there is no row to diverge from —
// RULE §5/A9 fires only on diverging from a row (types.rs/sys_calls.rs
// precedent).
//
// BYTE-PARITY SURFACE (RULE §0, gate A1): this module IS the binary
// instruction format's reader — word-by-word decoding with
// failed-element-yields-rest-starting-at-the-failing-word semantics. The
// disassembler (manifest row 109, rust/glean-core/src/rts/bytecode/
// disassemble.rs) consumes it inside the old-vs-new `disassemble`
// identity gate, and every label-bearing emitter row closes with "emitted
// bytecode must disassemble identically" (inventory.tsv rows 37/39/42/
// 64/80/81, inventory.codeflow.tsv rows 9-12 — the gate chain recorded in
// disassemble.rs's header), so the rollback semantics below are
// load-bearing, not incidental. No byte-order decision lives here: the
// input is already a WORD stream (`[Word64]`; the disassembler feeds
// `V.toList subInsns`, Disassemble.hs:53) — word splitting/byte layout is
// upstream of this module.
//
// CONTRACT WITH THE DISASSEMBLER (sibling manifest row 109, SHIPPED —
// its ASSUMED-IMPORT 1 records, verbatim):
//     pub fn decode_all<T: Decodable>(words: &[u64]) -> (Vec<T>, Vec<u64>)
// with "a failed element decode yields NOTHING for that element and the
// rest STARTS AT THE FAILED WORD (runDecode discards the partial state,
// Decode.hs:28-31 — the MaybeT(State) run backtracks the whole element)"
// and "`Decodable` is that module's trait". decode_all below carries
// exactly that signature and that semantics — the assumed import is now
// supplied, no divergence.
//
// SIBLING CONTRACTS (RULE §4; DONE units' real APIs read, never
// re-declared):
//   * crate::bytecode::types (SHIPPED, read): `Label(pub i64)`,
//     `Literal(pub u64)`, `Register<T: TyTag>` with `Register::new(u64)`
//     and the pub `(u64, PhantomData<T>)` fields. types.rs's Label doc
//     already records this module's cast ("Decode.hs:46 narrows the
//     Word64 stream with `Label . fromIntegral`, wrapping — the consumer
//     owns that cast"): this file is that consumer; the cast is
//     two's-complement `as i64` (RULE §2 numeric row; the same pin
//     disassemble.rs's label fold carries for its own fromIntegral).
//   * crate::bytecode::sys_calls (SHIPPED): NOT consumed. Decode.hs's
//     import list (Decode.hs:17-22) contains no Glean.Bytecode.SysCalls —
//     the syscall name tables never flow through the decoder (they reach
//     the RENDERER as a `&[&str]` parameter, sys_calls.rs CONSUMER
//     CONTRACT point 1 and disassemble.rs's wiring note; instruction
//     DECODE reads words only). Zero use statements to it.
//   * The GENERATED instruction modules (build output under OUT_DIR per
//     RULE §4; owner the generator units, manifest rows 150-152)
//     implement `Decodable` for `Insn`: Generate/Haskell.hs:62 emits
//     `import qualified Glean.Bytecode.Decode as D` into every generated
//     module, and the pinned generated instance (the ~/glean-build
//     checkout, .../RTS/Bytecode/Gen/Instruction.hs:238-302 — diff-
//     verified byte-identical to this repo's def table per
//     disassemble.rs's header) is `op <- D.decode` (a Word64) followed by
//     per-opcode `X <$> D.decode <*> ...` over the instruction's fields —
//     every field type this class serves (Register t / Literal / Label /
//     [Label]) — with `fail $ "invalid opcode " ++ show op` (:302) on any
//     other opcode. The trait shape below serves that instance unchanged
//     (the same overloaded-`decode` shape as the source); the `fail` is
//     None, see MONAD MAPPING — which is what makes disassemble's `rest`
//     non-empty and drives its `*** ERROR ***` line (Disassemble.hs:48).
//
// MONAD MAPPING (RULE §2 — MaybeT(State) has no Rust construct; the
// closed usage here makes the port exact, not an approximation):
//   `type Decode = MaybeT (State [Word64])` (Decode.hs:24-25) is a strict
//   state thread over the remaining words whose only effects are "pull
//   one word" (the Word64 instance's `state` action) and "fail". The
//   state is NEVER written except forward — no Put anywhere, every
//   instance is decode-and-consume — and failure backtracks by DISCARDING
//   the state at the run boundary (Decode.hs:28-31's `_ -> Nothing`
//   drops the final state), not by internal undo. Therefore:
//     State [Word64] → the private cursor `&'a [u64]` inside `Decode<'a>`
//                      (advance = drop the head; zero-copy, the exact
//                      image of the persistent list's tail-sharing).
//     MaybeT         → Option (the failure value).
//     runDecode's discard-on-Nothing → run_decode returning None and
//                      dropping its cursor: the whole-element backtrack
//                      IS that discard.
//   MonadFail pin (decides the generated instance's `fail`): transformers
//   0.5.6.2 — the GHC 9.4 bundle; the sibling headers pin base-4.17.1.0 —
//   Control.Monad.Trans.Maybe:
//       instance (Monad m) => MonadFail (MaybeT m) where
//           fail _ = MaybeT (return Nothing)   {- INLINE fail -}
//   — `fail` is Nothing for ANY Monad m (only `Monad m` is required), NOT
//   State's MonadFail error. The invalid-opcode `fail` therefore stops
//   the decode and yields the rollback; it does not throw.
//
// DERIVING MAP: the source derives nothing in this module (the class and
// its five instances carry no deriving); no Rust derives added.

use crate::bytecode::types::{Label, Literal, Register, TyTag};

/// A decoder which can be applied to a stream of words via `run_decode`
/// (source doc, Decode.hs:24) — the port of
/// `type Decode = MaybeT (State [Word64])` (Decode.hs:25): an opaque
/// cursor over the remaining words. See MONAD MAPPING in the header.
pub struct Decode<'a> {
    /// The `State [Word64]` cursor: the words not yet consumed.
    words: &'a [u64],
}

impl<'a> Decode<'a> {
    /// The Word64 instance's `state` action (Decode.hs:38-40):
    /// `state $ \xs -> case xs of y : ys -> (Just y, ys); _ -> (Nothing,
    /// xs)` — pop one word, or fail with the state unchanged.
    fn next_word(&mut self) -> Option<u64> {
        match self.words.split_first() {
            Some((y, ys)) => {
                self.words = ys;
                Some(*y)
            }
            None => None,
        }
    }
}

/// Things that can be decoded (source doc, Decode.hs:33) —
/// `class Decodable arg where decode :: Decode arg` (Decode.hs:34-35).
///
/// The monadic computation becomes the method over the cursor: `None` is
/// the MaybeT failure, and the whole-element rollback happens where the
/// decode is RUN (run_decode/decode_all discard the advanced state —
/// exactly Decode.hs:28-31's `_ -> Nothing`). External implementors (the
/// generated `impl Decodable for Insn`, SIBLING CONTRACTS above) drive it
/// with the same overloaded-`decode` shape as the source: `u64::decode(d)`
/// for the opcode, per-field `T::decode(d)` for Register t / Literal /
/// Label / [Label].
pub trait Decodable: Sized {
    fn decode(d: &mut Decode<'_>) -> Option<Self>;
}

/// `instance Decodable Word64` (Decode.hs:37-40) — the primitive every
/// other instance composes.
impl Decodable for u64 {
    fn decode(d: &mut Decode<'_>) -> Option<Self> {
        d.next_word()
    }
}

/// `instance Decodable Literal` (Decode.hs:42-43): `Literal <$> decode`.
impl Decodable for Literal {
    fn decode(d: &mut Decode<'_>) -> Option<Self> {
        u64::decode(d).map(Literal)
    }
}

/// `instance Decodable Label` (Decode.hs:45-46):
/// `Label . fromIntegral <$> (decode :: Decode Word64)` — the decoded
/// Word64 wrapped to the label's Haskell `Int`: two's-complement `as i64`
/// (RULE §2 numeric row; the wrap pinned in types.rs's Label doc and
/// disassemble.rs's label fold).
impl Decodable for Label {
    fn decode(d: &mut Decode<'_>) -> Option<Self> {
        u64::decode(d).map(|w| Label(w as i64))
    }
}

/// `instance Decodable [Label]` (Decode.hs:48-51) — count-prefixed:
/// `n <- decode; replicateM (fromIntegral (n :: Word64)) decode`.
///
/// The count's `fromIntegral :: Word64 -> Int` is the wrap cast `as i64`
/// (RULE §2 numeric row). `replicateM n x = sequence (replicate n x)`
/// (Control.Monad) and `replicate n x = take n (repeat x)` with
/// `take n _ | n <= 0 = []` (base-4.17.1.0, GHC/List.hs) — so a count
/// that wraps NEGATIVE (≥ 2^63 as Word64) decodes ZERO labels, consuming
/// only the count word: bug-for-bug, `0..n` with n ≤ 0 is the empty
/// range and does exactly that. A mid-list exhaustion returns None having
/// consumed words — the caller's discard (run_decode) backtracks them,
/// which is the MaybeT semantics, not an approximation.
impl Decodable for Vec<Label> {
    fn decode(d: &mut Decode<'_>) -> Option<Self> {
        let n = u64::decode(d)?;
        let n = n as i64;
        let mut labels = Vec::new();
        for _ in 0..n {
            labels.push(Label::decode(d)?);
        }
        Some(labels)
    }
}

/// `instance Decodable (Register t)` (Decode.hs:53-54):
/// `Register <$> decode` — the register word wrapped in its tag; the
/// phantom type parameter flows through (generic over every `TyTag t`),
/// `Register::new` being types.rs's sanctioned constructor.
impl<T: TyTag> Decodable for Register<T> {
    fn decode(d: &mut Decode<'_>) -> Option<Self> {
        u64::decode(d).map(|w| Register::new(w))
    }
}

/// `runDecode :: Decode a -> [Word64] -> Maybe (a, [Word64])`
/// (Decode.hs:27-31) — execute a decode and return the result and the
/// remaining words. A failure returns None and DISCARDS the advanced
/// state: the whole element backtracks (the `_ -> Nothing` arm — the
/// exact semantic decode_all's rollback rests on, and the disassembler
/// contract's "rest STARTS AT THE FAILED WORD").
///
/// The remaining words are borrowed (the Haskell list value's zero-copy
/// tail; decode_all clones it once into the contract's owned Vec).
pub fn run_decode<'a, T: Decodable>(words: &'a [u64]) -> Option<(T, &'a [u64])> {
    let mut d = Decode { words };
    let value = T::decode(&mut d)?;
    Some((value, d.words))
}

/// `decodeAll :: Decodable a => [Word64] -> ([a], [Word64])`
/// (Decode.hs:56-64) — "Decode as many elements as possible and return
/// the decoded elements and the rest of the words, starting with the
/// first that can't be decoded" (source doc, Decode.hs:56-57).
///
/// go's arms (Decode.hs:60-64), one for one:
///   * `go xs [] = (reverse xs, [])` — the `rest.is_empty()` exit: empty
///     tail, no decode attempt;
///   * `Just (x, ws) -> go (x:xs) ws` — push (in order — the Vec appends
///     where the Haskell prepends and reverse pays for it at the end) and
///     continue from the tail;
///   * `Nothing -> (reverse xs, vs)` — stop with `rest` UNCHANGED: the
///     words from the failing element's start (run_decode discarded the
///     partial consumption).
///
/// The disassembler's pinned call (manifest row 109 ASSUMED-IMPORT 1):
/// `decode_all::<Insn>(&sub_insns)` — `rest` non-empty iff some element
/// failed, which drives the `*** ERROR ***` line (Disassemble.hs:48).
pub fn decode_all<T: Decodable>(words: &[u64]) -> (Vec<T>, Vec<u64>) {
    let mut decoded: Vec<T> = Vec::new();
    let mut rest: &[u64] = words;
    while !rest.is_empty() {
        match run_decode::<T>(rest) {
            Some((x, ws)) => {
                decoded.push(x);
                rest = ws;
            }
            None => break, // rest stays at the failing word
        }
    }
    (decoded, rest.to_vec())
}

// Marker inventory (reviewer counts the trailer against this): zero
// TODO(port), zero PERF(port), zero BUG(port), zero unsafe/SAFETY blocks,
// zero INVENTORY-BUG-CANDIDATE flags (no inventory row exists to diverge
// from — the zero-row finding is the INVENTORY block at the top;
// types.rs/sys_calls.rs precedent).
//
// Coverage: every export of the Haskell module (module header
// Decode.hs:9-15) has a Rust counterpart — Decode [the type synonym →
// the opaque cursor struct Decode<'a>', MONAD MAPPING above],
// Decodable(..) [trait + decode method; external impls supported: the
// generated Insn instance, SIBLING CONTRACTS], runDecode [run_decode],
// decodeAll [decode_all, the disassembler's pinned signature verbatim],
// and all five instances — Word64 [u64, the private next_word primitive],
// Literal, Label [wrap cast], [Label] [Vec<Label>, count-prefixed with
// the negative-count replicate disposition], Register t [generic over
// TyTag]. Nothing else exists in the 64-line source.
//
// MODULE-WIRING: lib.rs / bytecode/mod.rs scaffolding is deliberately NOT
// a manifest unit; owned by the Step-4 survey build (do not create ad-hoc
// here) — same ruling as bytecode/types.rs and bytecode/sys_calls.rs.
//
// (medium: the crate is uncompiled until the Step-3 gate lifts the build ban.)
// (medium, not high: nothing in this crate has been compiled — the
// PORT STATUS: confidence=medium todos=0
