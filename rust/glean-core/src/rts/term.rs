// Ported from: glean/hs/Glean/RTS/Term.hs
// Manifest unit: rust/glean-core/src/rts/term.rs (migration/manifest.tsv
// row 12, crate glean-core). RULE §4: one source file → one target file;
// the manifest path is authoritative.
//
// Dependency truth (migration/depmap/edges.tsv, the module-level edge
// list — checked, not assumed):
//   glean/hs/Glean/RTS/Term.hs → glean/hs/Glean/RTS/Types.hs
//   glean/hs/Glean/RTS/Term.hs → glean/angle/Glean/Display.hs
// Types.hs does NOT import Term.hs, so the source's own direction is
// Term → Types and this file depends on rts/types.rs, never the reverse.
// The Display edge crosses into crate glean-angle (module-level edge in
// edges.tsv; depmap/crates.txt: 33 crates, 102 crate edges, 0 cycles, so
// the core→angle edge is legal) — hence `use glean_angle::display::…`
// (RULE §4 imports: `use glean_<crate>::…` across crates, no glob
// re-exports). rust/glean-angle/src/display.rs's own SIBLING-UNIT
// CONTRACT lists RTS/Term.hs among its verified consumers (encloseSep at
// Term.hs:55,58): this file codes against that shipped surface and never
// re-declares Doc/Display/DisplayOpts.
//
// RULE §5: this source file has NO inventory rows — grep "RTS/Term" (and
// "Term" case-insensitively, filtered to the RTS/ path, not the
// shell/lang Terminal ones) across migration/inventory*.tsv returns only
// manifest.tsv:12 and edges.tsv rows. No ownership/laziness/STM/JSON
// site exists in the source to look up; nothing here diverges from a row.
//
// What this module is (RULE §0: the old code is the spec): 69 lines of
// pure data — the runtime Term type, its deriving-clause surface, one
// Display instance, one type alias. No FFI, no IO, no unsafe; the
// crate::ffi and crate::rts::builder siblings are NOT imported because
// the Haskell imports neither.

// ASSUMED-IMPORT (RULE §4 draft stub policy / assumed-imports; owner:
// rust/glean-core/src/rts/types.rs, manifest.tsv row 11, translated IN
// PARALLEL as unit 10 — this file was written WITHOUT reading that
// target): the source imports exactly one name, `import Glean.RTS.Types
// (Fid)` (Term.hs:24), so the Rust mirror imports exactly one name:
//
//     use crate::rts::types::Fid;
//
// Assumed contract (from the Haskell spec Types.hs:70-71, unit 10's
// file): `newtype Fid = Fid { fromFid :: Id }` with Id = i64 — an owned
// Fid type at crate::rts::types (with Eq/Ord/Hash/Display per
// Types.hs:71/76-77). Only the NAME is load-bearing here, for `Value`
// below; if unit 10 ships Fid under a different shape this import (and
// nothing else in this file) needs updating.

//! Runtime terms — `Term ref`, the Haskell representation of a Glean
//! term (fact key/value), parameterised over the fact-reference type
//! (Term.hs:26-36 doc comment, carried below onto the enum).

use glean_angle::display::{align, enclose_sep, Doc, Display, DisplayOpts};

use crate::rts::types::Fid;

// -----------------------------------------------------------------------------
// Runtime terms (Term.hs:26-47)
// -----------------------------------------------------------------------------

/// Haskell representation of a Glean term, i.e. the key or value of
/// a fact. These are stored binary-encoded in the DB (see
/// "Glean.Typed.Binary").
///
/// Parameterised over `R`, the representation of a fact
/// reference. This type is often extended by instantiating `R`. For
/// example we represent query patterns by instantiating `R` with a
/// pattern type in the query engine, see "Glean.Query.Typecheck.Types".
///
/// (Source doc comment, Term.hs:29-36, verbatim in substance.)
///
/// `#{-# UNPACK #-}` pragmas (Term.hs:39,42,44) are a GHC heap-layout
/// detail with no Rust counterpart (the payloads are already inline
/// here) — dropped, representation-only (RULE §0 translating, not
/// improving).
///
/// `Box` in [`Term::Alt`] is the mechanical Rust requirement for
/// recursion through a direct field (the Haskell field was a lazy
/// pointer anyway); `Array`/`Tuple` need no Box because `Vec` already
/// indirects.
///
/// Deriving parity (Term.hs:47 `deriving(Eq, Ord, Generic, Show,
/// Functor, Foldable, Traversable)`):
/// - `Eq`/`Ord` → derived `PartialEq`/`Eq`/`PartialOrd`/`Ord`: Rust
///   compares enum variants in DECLARATION order, fields left-to-right —
///   identical to Haskell's derived constructor order (Byte < Nat <
///   Array < ByteArray < Tuple < Alt < String < Ref). `Vec<u8>`
///   (ByteString) ordering is lexicographic bytes with the
///   shorter-prefix-first tiebreak on both sides.
/// - `Show` → derived `Debug`. RULE §2 (deriving-Show row, A4/B6):
///   Show-TAG parity — the constructor tags (`Byte`, `Nat`, `Array`,
///   `ByteArray`, `Tuple`, `Alt`, `String`, `Ref`) carry; the payload
///   punctuation differs (Rust `Byte(65)` vs Haskell `Byte 65`, and
///   `Vec<u8>` renders as numbers where Haskell's ByteString Show
///   renders a string literal). Same choice as the DONE sibling
///   rust/glean-angle/src/angle/types.rs `SourcePat_` (Haskell
///   `deriving (Eq, Show, …)` → plain `#[derive(Debug)]`, types.rs:477).
///   Show IS exercised in the tree — an earlier draft claimed a
///   closed-set grep found zero `show` call sites on Term values; in
///   fact Glean/Query/Transform.hs renders Terms into error messages
///   (`"buildTerm: " <> show other`, :263; `"expected Array/Tuple/Alt,
///   got " <> show term`, :499/:511/:549/:579 and the same shape again
///   in the SetTy arms :591/:603), and the Show output leaks into
///   composite derives that embed Terms (Codegen/Types.hs:90 CgStatement_
///   `deriving (Show, …)` over generators carrying `Pat`; :253 Match
///   `deriving (…, Show)` with `MatchArrayPrefix` storing `Term (Match
///   ext var)` fields). The justification for tag-parity is therefore
///   RULE §2 itself — the row makes the constructor TAGS load-bearing
///   exactly for these error/diagnostic surfaces, which is how Show is
///   exercised; the payload punctuation divergence appears only inside
///   those diagnostics. The conclusion (plain `#[derive(Debug)]`)
///   stands; the closed-set premise did not. The byte-exact
///   string-literal rendering the Display path needs lives in
///   [`show_byte_string`].
/// - `Hashable` (Term.hs:49, `instance Hashable ref => Hashable
///   (Term ref)`, Generic-derived) → derived `std::hash::Hash`. In-memory
///   HashMap salt only, never persisted, never golden-printed — the same
///   standing as the angle crate's Hashable ports
///   (angle/hash.rs:170-196 records that Data.Hashable's exact
///   hashWithSalt chain is not reproduced anywhere; Term adds no new
///   obligation because no port hashes Terms across a persisted
///   boundary).
/// - `Generic` is GHC-deriving machinery feeding the two derivations
///   above; it has no Rust counterpart and none is needed.
/// - `Functor`/`Foldable`/`Traversable` have no Rust derive — their
///   faithful surface is the `map`/`fold_map`/`try_map` inherent methods
///   below.
/// - `Clone` is added (Haskell values are freely copyable; every sibling
///   enum derives it).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Term<R> {
    Byte(u8),
    Nat(u64),
    Array(Vec<Term<R>>),
    ByteArray(Vec<u8>),
    Tuple(Vec<Term<R>>),
    Alt(u64, Box<Term<R>>),
    /// utf8-encoded (source comment, Term.hs:45)
    String(Vec<u8>),
    Ref(R),
}

// -----------------------------------------------------------------------------
// Functor / Foldable / Traversable (Term.hs:47 deriving clause)
// -----------------------------------------------------------------------------

impl<R> Term<R> {
    /// Functor — `deriving Functor` (Term.hs:47): `fmap` applies `f` at
    /// the `Ref` positions only; Byte/Nat/ByteArray/String carry no refs
    /// and pass through untouched. Consumer shape: Codegen.hs:1228
    /// `inlineVars vars term = fmap (bimap …) term`.
    ///
    /// Signature house style (matching the DONE sibling angle/types.rs
    /// `bimap`, types.rs:1258): `self` by value, closure as `&mut F`.
    pub fn map<R2, F>(self, f: &mut F) -> Term<R2>
    where
        F: FnMut(R) -> R2,
    {
        fn map_vec<R, R2, F>(xs: Vec<Term<R>>, f: &mut F) -> Vec<Term<R2>>
        where
            F: FnMut(R) -> R2,
        {
            xs.into_iter().map(|x| x.map(f)).collect()
        }
        match self {
            Term::Byte(x) => Term::Byte(x),
            Term::Nat(x) => Term::Nat(x),
            Term::Array(xs) => Term::Array(map_vec(xs, f)),
            Term::ByteArray(xs) => Term::ByteArray(xs),
            Term::Tuple(xs) => Term::Tuple(map_vec(xs, f)),
            Term::Alt(s, x) => Term::Alt(s, Box::new((*x).map(f))),
            Term::String(s) => Term::String(s),
            Term::Ref(r) => Term::Ref(f(r)),
        }
    }

    /// Traversable — `deriving Traversable` (Term.hs:47), as the
    /// canonical Rust shape of `traverse`/`mapM` specialised to
    /// Either/Result: a fallible `map` whose FIRST error in field order
    /// short-circuits (Result's Applicative keeps the first error, same
    /// as Either). Consumer evidence, corrected: the genuine
    /// Term-Traversable consumer is BindOrder.hs:114
    /// `fixBindOrder isPat term = mapM (fixBindOrder isPat) term`,
    /// which runs in `Fix = StateT (Scope, NoBind) (Except
    /// FixBindOrderError)` (BindOrder.hs:80) — the fallible dimension is
    /// Except, whose mapM short-circuits on the first error exactly like
    /// Result; this is the strongest justification for the Result shape
    /// (the State dimension is composed at that site, not here).
    /// Typecheck/Unify.hs:305-307 was cited here before but is NOT a
    /// Traversable consumer: `zonkTcPat` hand-recurses over the RTS
    /// constructors with a LIST mapM inside (`RTS.Array <$> mapM
    /// zonkTcPat ts`) and never uses the Term instance. Known narrowing
    /// boundary: Vars.hs:136 `freshWild pat = mapM freshWildMatch pat`
    /// over `Pat = Term (Match () var)` (Codegen/Types.hs:186) IS a
    /// Term-Traversable consumer but a STATEFUL one (Fresh — fresh
    /// wildcard numbering); `try_map` cannot serve it, and its unit
    /// ports its own stateful traversal — the narrowing to Result is a
    /// visible decision (RULE §0), not full Traversable coverage.
    pub fn try_map<R2, E, F>(self, f: &mut F) -> Result<Term<R2>, E>
    where
        F: FnMut(R) -> Result<R2, E>,
    {
        fn try_map_vec<R, R2, E, F>(xs: Vec<Term<R>>, f: &mut F) -> Result<Vec<Term<R2>>, E>
        where
            F: FnMut(R) -> Result<R2, E>,
        {
            xs.into_iter().map(|x| x.try_map(f)).collect()
        }
        match self {
            Term::Byte(x) => Ok(Term::Byte(x)),
            Term::Nat(x) => Ok(Term::Nat(x)),
            Term::Array(xs) => Ok(Term::Array(try_map_vec(xs, f)?)),
            Term::ByteArray(xs) => Ok(Term::ByteArray(xs)),
            Term::Tuple(xs) => Ok(Term::Tuple(try_map_vec(xs, f)?)),
            Term::Alt(s, x) => Ok(Term::Alt(s, Box::new((*x).try_map(f)?))),
            Term::String(s) => Ok(Term::String(s)),
            Term::Ref(r) => Ok(Term::Ref(f(r)?)),
        }
    }

    /// Foldable — `deriving Foldable` (Term.hs:47): folds over the `Ref`
    /// positions only (elements without refs contribute nothing).
    /// Accumulator-threaded left-to-right visit, the sibling
    /// house shape (angle/types.rs `bifold_map`, types.rs:1351, `F:
    /// FnMut(&P, &mut A)`). ORDER NOTE: derived `foldr` visits elements
    /// LEFT-to-right — GHC.Tc.Deriv.Functor generates `g1 v1 (g2 v2
    /// ... z)` for `foldr f z (Ct x1 x2 ...)`, i.e. `f x1 (f x2 (...
    /// z))`: the first field is folded first, exactly the visit order
    /// of this accumulator thread. The port matches the source order
    /// EXACTLY, with no reversal to excuse. (An earlier draft claimed
    /// derived foldr was right-to-left; it is not.) Belt-and-braces,
    /// a non-excuse: every fold over Terms in the tree accumulates via
    /// IntSet.insert, which is commutative, so even a mismatched order
    /// would be unobservable — Codegen.hs:161 `findOutputsPat pat r =
    /// foldr findOutputsMatch r pat`, and Opt.hs:542 `queryScope`
    /// (`foldr termScope … maybeVal`), :570 `genScope` (`foldr
    /// termScope r args`), :576 `addToCurrentScope (Var _ v _) r =
    /// IntSet.insert v r` — but no divergence exists to hide.
    pub fn fold_map<A, F>(&self, acc: &mut A, f: &mut F)
    where
        F: FnMut(&R, &mut A),
    {
        match self {
            Term::Byte(..) | Term::Nat(..) | Term::ByteArray(..) | Term::String(..) => {}
            Term::Array(xs) | Term::Tuple(xs) => {
                for x in xs {
                    x.fold_map(acc, f);
                }
            }
            Term::Alt(_, x) => x.fold_map(acc, f),
            Term::Ref(r) => f(r, acc),
        }
    }
}

// -----------------------------------------------------------------------------
// Display (Term.hs:51-66)
// -----------------------------------------------------------------------------

/// Haskell `show :: ByteString -> String`, needed by the ByteArray
/// Display (Term.hs:56) — `pretty (show xs) <> "#"`. Byte-exact port of
/// bytestring-0.11.5.2's `instance Show ByteString where showsPrec p ps
/// = showsPrec p (unpackChars ps)` (Data/ByteString/Internal/Type.hs:
/// 321-322): the bytes Latin-1-decoded to a Haskell String (each byte <
/// 0x100 becomes one Char), then shown with base-4.17.1.0's
/// GHC.Show showLitString/showLitChar/asciiTab (GHC/Show.hs:375-398,
/// 428-434). No `fromString` prefix — that is the 0.11.5.2 form. All
/// byte-level facts here are pinned from the package source on disk
/// (/tmp/base-4.17.1.0/GHC/Show.hs): in particular protectEsc (:422-424)
/// terminates an ambiguous escape with the two-char `\&` sequence
/// (applied at :373 for `\DDD`, :384 for `\SO`) — never `;`. The DONE
/// sibling rust/glean-angle/src/angle/types.rs:2018-2082 ported the same
/// `show` for the Angle/Types.hs:906 ByteArray site and was initially
/// cited here as verification, but it carried the same wrong `;`
/// terminator itself (a sibling's output is not provenance — the pin is
/// base's own source); it is corrected in the same pass. The copy still
/// stands because the sibling's fn is `pub(crate)`-private and cannot be
/// imported across the crate edge — this mirrors the Haskell, where each
/// instance independently calls `show`.
fn show_byte_string(bytes: &[u8]) -> String {
    // asciiTab — GHC.Show.hs:428-434 (indices 0..32).
    const ASCII_TAB: [&str; 33] = [
        "NUL", "SOH", "STX", "ETX", "EOT", "ENQ", "ACK", "BEL", "BS", "HT", "LF", "VT", "FF", "CR",
        "SO", "SI", "DLE", "DC1", "DC2", "DC3", "DC4", "NAK", "SYN", "ETB", "CAN", "EM", "SUB",
        "ESC", "FS", "GS", "RS", "US", "SP",
    ];
    // protectEsc inspects the first char of the rendered REST, which is
    // the next input byte's first output char (a raw printable or a
    // backslash), or the closing quote at end-of-string — so the guard
    // reduces to a test on the next byte, as below.
    let mut out = String::with_capacity(bytes.len() + 2);
    out.push('"');
    for (i, &b) in bytes.iter().enumerate() {
        let next_is_digit = bytes.get(i + 1).is_some_and(u8::is_ascii_digit);
        let next_is_h = bytes.get(i + 1) == Some(&b'H');
        match b {
            0x5C => out.push_str("\\\\"), // '\\'
            0x22 => out.push_str("\\\""), // '"'
            0x7F => out.push_str("\\DEL"),
            b if (0x20..0x7F).contains(&b) => out.push(b as char),
            0x07 => out.push_str("\\a"),
            0x08 => out.push_str("\\b"),
            0x0C => out.push_str("\\f"),
            0x0A => out.push_str("\\n"),
            0x0D => out.push_str("\\r"),
            0x09 => out.push_str("\\t"),
            0x0B => out.push_str("\\v"),
            // '\SO': protectEsc (== 'H') (GHC.Show.hs:384). When the
            // guard fires, protectEsc prepends the TWO-CHAR sequence
            // `\&` before the rest (GHC.Show.hs:422-424,
            // cont s@(c:_) | p c = "\\&" ++ s) — never `;`.
            0x0E => {
                out.push_str("\\SO");
                if next_is_h {
                    out.push_str("\\&");
                }
            }
            // c > '\DEL': \DDD decimal + protectEsc isDec (GHC.Show.hs:373,
            // same two-char `\&` terminator, GHC.Show.hs:422-424)
            b if b > 0x7F => {
                out.push('\\');
                out.push_str(&(b as u32).to_string());
                if next_is_digit {
                    out.push_str("\\&");
                }
            }
            b => {
                out.push('\\');
                out.push_str(ASCII_TAB[b as usize]);
            }
        }
    }
    out.push('"');
    out
}

/// text-2.0.2's lenient UTF-8 decode, byte-exact:
/// `Text.decodeUtf8With Text.lenientDecode` (Term.hs:61-62's String
/// Display path). `lenientDecode = OnDecodeError (\_ _ -> Just
/// '\xFFFD')` (Data/Text/Encoding/Error.hs:106-107, pinned from
/// /tmp/text-2.0.2), threaded through `decodeUtf8With1`
/// (Data/Text/Internal/Encoding.hs:473-492) and `decodeUtf8With2`
/// (:501-528). The error policy is PER-BYTE, not per-maximal-subpart:
///   - a byte that is invalid on its own (bare continuation 0x80..0xBF,
///     overlong leads 0xC0/0xC1, 0xF5..0xFF) emits ONE U+FFFD and drops
///     EXACTLY that one byte — `handleUtf8Error` on the single byte at
///     the failure index, then the loop restarts from `startUtf8State`
///     at the NEXT byte (Internal/Encoding.hs:486, :522-524);
///   - a byte that fails to complete an accumulated partial emits ONE
///     U+FFFD PER ACCUMULATED BYTE — `skipIncomplete` folds the error
///     handler over `partialUtf8CodePoint` (Internal/Encoding.hs:453-459,
///     :127-135), which stores every accepted byte of the incomplete
///     code point INCLUDING its lead — and then the offending byte is
///     re-examined from the fresh state (:517-521);
///   - a partial truncated by end of input likewise emits one U+FFFD per
///     byte including the lead (decodeUtf8With1's `Just` branch, :480-484).
/// Which prefixes may accumulate is decided by the well-formedness
/// ranges of Utf8.hs's validate2/3/4 (:190-218): leads C2..DF, E0..EF,
/// F0..F4 with the E0→A0..BF, ED→80..9F, F0→90..BF, F4→80..8F
/// constraints (E0 9F is rejected as a second byte, not a truncation).
///
/// This is NOT Rust's `String::from_utf8_lossy` (Unicode "maximal
/// subpart" / WHATWG replacement, one U+FFFD per maximal invalid
/// subpart). Reviewer divergence examples — `from_utf8_lossy` vs this
/// port:
/// ```text
/// [E0 A0 00] -> 1 vs 2 U+FFFD  (E0 A0 is a valid 3-byte prefix; 00
///                               cannot complete it, so both bytes flush
///                               per-byte and 00 survives as NUL)
/// [E2 82]    -> 1 vs 2 U+FFFD  (truncated 3-byte sequence: the lead
///                               flushes too, one FFFD per byte)
/// [E1 80 C0] -> 2 vs 3 U+FFFD  (E1 80 flushes per-byte; C0 is then a
///                               fresh invalid lead, one FFFD, dropped)
/// ```
/// Still not a RULE §1 lossy-boundary conversion smuggled in: the
/// SOURCE chose the lenient decode, and the bytes stay untouched in the
/// variant; only the rendered copy is lossy, exactly as in Haskell.
fn lenient_decode_utf8(bytes: &[u8]) -> String {
    const REPLACEMENT: char = '\u{FFFD}';
    let mut out = String::with_capacity(bytes.len());
    // Utf8State: the bytes accepted so far of an incomplete code point
    // (`partialUtf8CodePoint`, including the lead) and how many total
    // bytes the lead promised (0 = fresh / `startUtf8State`).
    let mut seq = [0u8; 4];
    let mut have = 0usize;
    let mut need = 0usize;
    for &b in bytes {
        // Is `b` acceptable from the current state? Fresh-state leads
        // and constrained continuations per Utf8.hs validate2/3/4.
        let accept = if have == 0 {
            matches!(b, 0x00..=0x7F | 0xC2..=0xDF | 0xE0..=0xEF | 0xF0..=0xF4)
        } else {
            match (seq[0], have) {
                (0xE0, 1) => (0xA0..=0xBF).contains(&b),
                (0xED, 1) => (0x80..=0x9F).contains(&b),
                (0xF0, 1) => (0x90..=0xBF).contains(&b),
                (0xF4, 1) => (0x80..=0x8F).contains(&b),
                _ => (0x80..=0xBF).contains(&b),
            }
        };
        if accept {
            if have == 0 && b < 0x80 {
                out.push(b as char);
            } else {
                if have == 0 {
                    need = if b < 0xE0 {
                        2
                    } else if b < 0xF0 {
                        3
                    } else {
                        4
                    };
                    seq = [0; 4];
                }
                seq[have] = b;
                have += 1;
                if have == need {
                    let cp = match have {
                        2 => ((seq[0] as u32 & 0x1F) << 6) | (seq[1] as u32 & 0x3F),
                        3 => {
                            ((seq[0] as u32 & 0x0F) << 12)
                                | ((seq[1] as u32 & 0x3F) << 6)
                                | (seq[2] as u32 & 0x3F)
                        }
                        _ => {
                            ((seq[0] as u32 & 0x07) << 18)
                                | ((seq[1] as u32 & 0x3F) << 12)
                                | ((seq[2] as u32 & 0x3F) << 6)
                                | (seq[3] as u32 & 0x3F)
                        }
                    };
                    out.push(char::from_u32(cp).expect("validate2/3/4 ranges rule out non-scalar code points"));
                    have = 0;
                    need = 0;
                }
            }
        } else if have > 0 {
            // `b` cannot complete the accumulated partial (the len < 0
            // arm of decodeUtf8With2's loop): one FFFD per accumulated
            // byte INCLUDING the lead (skipIncomplete), then re-examine
            // `b` itself from the fresh state.
            for _ in 0..have {
                out.push(REPLACEMENT);
            }
            have = 0;
            need = 0;
            if b < 0x80 {
                out.push(b as char);
            } else if (0xC2..=0xF4).contains(&b) {
                need = if b < 0xE0 {
                    2
                } else if b < 0xF0 {
                    3
                } else {
                    4
                };
                seq = [0; 4];
                seq[0] = b;
                have = 1;
            } else {
                out.push(REPLACEMENT);
            }
        } else {
            // Fresh invalid byte (bare continuation, C0/C1, F5..FF):
            // one FFFD, drop exactly this byte, restart at the next.
            out.push(REPLACEMENT);
        }
    }
    // End of input with an incomplete code point: one FFFD per pending
    // byte including the lead (decodeUtf8With1's `Just` branch).
    for _ in 0..have {
        out.push(REPLACEMENT);
    }
    out
}

/// `instance Display ref => Display (Term ref)` (Term.hs:51-66).
///
/// `pretty x` on Word8/Word64 is prettyprinter's viaShow — plain decimal
/// digits (`Doc::text(x.to_string())`; the newline-splitting inside
/// `Doc::text` can never fire on numerals).
impl<R: Display> Display for Term<R> {
    fn display(&self, opts: &DisplayOpts) -> Doc {
        match self {
            // display _ (Byte x) = "#" <> pretty x   (Term.hs:52)
            Term::Byte(x) => Doc::text("#").append(Doc::text(x.to_string())),
            // display _ (Nat x) = pretty x   (Term.hs:53)
            Term::Nat(x) => Doc::text(x.to_string()),
            // display opts (Array xs) =
            //   align $ encloseSep "[" "]" "," $ map (display opts) xs
            // (Term.hs:54-55). Note the elements use `display`, NOT
            // `displayAtom` — faithful to the source. Empty list:
            // encloseSep gives "[]" exactly as prettyprinter does.
            Term::Array(xs) => align(enclose_sep(
                Doc::text("["),
                Doc::text("]"),
                Doc::text(","),
                xs.iter().map(|x| x.display(opts)),
            )),
            // display _ (ByteArray xs) = pretty (show xs) <> "#"
            // (Term.hs:56) — the Haskell-Show string literal (quotes and
            // escapes included, via show_byte_string) followed by '#'.
            Term::ByteArray(xs) => Doc::text(show_byte_string(xs)).append(Doc::text("#")),
            // display opts (Tuple xs) =
            //   align $ encloseSep "{" "}" "," $ map (display opts) xs
            // (Term.hs:57-58)
            Term::Tuple(xs) => align(enclose_sep(
                Doc::text("{"),
                Doc::text("}"),
                Doc::text(","),
                xs.iter().map(|x| x.display(opts)),
            )),
            // display opts (Alt s x) =
            //   "(" <> pretty s <> "|" <> display opts x <> ")"
            // (Term.hs:59) — literal quote-bar concatenation, not the
            // `parens` combinator (the source writes raw <>).
            Term::Alt(s, x) => Doc::text("(")
                .append(Doc::text(s.to_string()))
                .append(Doc::text("|"))
                .append(x.display(opts))
                .append(Doc::text(")")),
            // Source's own commented-out earlier line, preserved
            // (Term.hs:60):
            //   -- display opts (String s) = "\"" <> display opts s <> "\""
            //
            // display _ (String s) =
            //   "\"" <> pretty (Text.decodeUtf8With Text.lenientDecode s)
            //   <> "\""   (Term.hs:61-62)
            //
            // lenientDecode = OnDecodeError (\_ _ -> Just '\xFFFD') under
            // the PINNED text-2.0.2 (GHC 9.4.7 boot lib) — a PER-BYTE
            // replacement policy, NOT maximal-subpart/WHATWG: one U+FFFD
            // per invalid byte, dropping exactly one byte per error and
            // restarting at the next, with truncated partials emitting
            // one U+FFFD per byte INCLUDING the lead. Rust's
            // `String::from_utf8_lossy` diverges from this (one U+FFFD
            // per maximal subpart) and must not be used here — see
            // [`lenient_decode_utf8`] for the pinned policy, sources, and
            // divergence examples. This is NOT a RULE §1 lossy-boundary
            // conversion smuggled in: the SOURCE itself chose the lenient
            // decode, and the bytes stay untouched in the variant; only
            // the rendered copy is lossy, exactly as in Haskell. `pretty`
            // on the decoded Text then splits real '\n' into soft lines —
            // `Doc::text` does the same.
            Term::String(s) => Doc::text("\"")
                .append(Doc::text(lenient_decode_utf8(s)))
                .append(Doc::text("\"")),
            // display opts (Ref ref) = display opts ref   (Term.hs:63)
            Term::Ref(r) => r.display(opts),
        }
    }

    // displayAtom opts (Ref ref) = displayAtom opts ref   (Term.hs:65)
    // displayAtom opts other = display opts other         (Term.hs:66)
    // — an explicit override of the trait's default `displayAtom =
    // display` (glean_angle::display::Display): everything but Ref
    // falls back to plain display, which IS the default's behaviour, so
    // only the Ref delegation is observable — kept as a full override to
    // mirror the source's shape line for line.
    fn display_atom(&self, opts: &DisplayOpts) -> Doc {
        match self {
            Term::Ref(r) => r.display_atom(opts),
            other => other.display(opts),
        }
    }
}

// -----------------------------------------------------------------------------
// Value (Term.hs:68)
// -----------------------------------------------------------------------------

/// `type Value = Term Fid` (Term.hs:68) — a term whose refs are fact
/// ids. `Fid` is the assumed-import from crate::rts::types (see the
// banner at the top of this file).
pub type Value = Term<Fid>;

// Marker breakdown: ZERO TODO(port)/PERF(port)/BUG(port)/SAFETY markers —
// no unsafe code (RULE §3 vacuously satisfied), no source defects to
// reproduce, no performance choices beyond the source's own, and every
// open-looking question was closed against a PINNED package source on
// disk, not against a sibling's output (bytestring-0.11.5.2 Show form +
// base-4.17.1.0 GHC/Show.hs protectEsc — the `\&` two-char terminator,
// :373/:384/:422-424, where an earlier pass had wrongly copied `;` from
// the sibling; text-2.0.2 lenient decode ported per-byte from
// decodeUtf8With1/2 + skipIncomplete, deliberately NOT Rust's
// from_utf8_lossy; angle Display surface). The one cross-unit
// dependency is the single-name Fid assumed-import, recorded in the
// banner.
// PORT STATUS: confidence=high todos=0
