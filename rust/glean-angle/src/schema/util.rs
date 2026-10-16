// Ported from: glean/angle/Glean/Schema/Util.hs
// Manifest unit: rust/glean-angle/src/schema/util.rs (crate glean-angle) —
// migration/manifest.tsv row 5. RULE §4 (naming): one source file → one
// target file; the manifest path is authoritative.
//
// INVENTORY (RULE §5): no row in any migration/inventory*.tsv names this
// unit (grep "Schema/Util"/"Util.hs" across inventory.tsv and the five
// per-area sweeps: zero rows). The module is pure construction / parsing /
// rendering over the sibling AST — no inventory-rowed gap construct (no
// FFI, no STM, no orphan Binary) — so there is no target_translation to follow
// and no INVENTORY-BUG-CANDIDATE. The one kit-external import
// (Util.Text.textToInt from hsthrift common/util, an fbcode-style shim
// outside the 383-unit manifest, like Compat.Prettyprinter in display.rs)
// has a fully-pinned source: transcribed below as `text_to_int`.
//
// DEPENDENCY TRUTH (the source's whole import list, Util.hs:27-33):
//   - Glean.Angle.Types — the SHIPPED sibling unit 3
//     (rust/glean-angle/src/angle/types.rs): Type_, FieldDef_, Name,
//     SourceRef, PredicateId, TypeId, TUPLE_FIELD, and the glean_thrift
//     re-exports PredicateRef/TypeRef/Version. Only `use`d here, never
//     re-declared.
//   - Util.Text (hsthrift common/util) — kit-external shim, owns no
//     manifest unit; its textToInt is transcribed below as the private
//     `text_to_int` (source pinned — see INVENTORY above).
//   - TextShow — external; its `showt` rendering is the crate's Display
//     stand-in (RULE §2 deriving-Show row): showt on the thrift Version
//     (i32) is signed decimal, i.e. i32's Display, used in show_ref.
//   - Data.Text (GHC boot lib) — breakOnEnd/init/split transcribe to
//     std str ops inline (rfind / pop / split); no separate unit.
//
// SIBLING CONTRACTS (RULE §4 assumed-imports; in-crate sibling per §0/§4 ownership): crate::angle::types owns Type_,
// FieldDef_, Name, SourceRef, PredicateId, TypeId, TUPLE_FIELD and the
// pub re-exports of PredicateRef/TypeRef/Version from glean_thrift; this
// file only `use`s them and never re-declares any of them. hash.rs's
// `fmt::Display for Fingerprint` (exactly 32 lowercase hex digits — GHC's
// `show :: Fingerprint -> String`) is what `show hash` renders inside
// `show_ref` for PredicateId/TypeId, reached through the sibling field
// (`predicate_id_hash.to_string()` / `{}`), with no direct import.
//
// Semantics pinned against the actual packages this build compiles
// against (RULE §0: the old code is the spec), by running the tree's own
// GHC 9.4.7 against text-2.0.2 and hsthrift's Util.Text:
//   - Data.Text.breakOnEnd: `breakOnEnd "." "abc.def.3"` = ("abc.def.","3")
//     (prefix up to AND INCLUDING the last separator), and with no '.' in
//     the input the pair is ("", whole) — `breakOnEnd "." "123"` =
//     ("","123"), which is what makes the source's `Text.init` partial
//     (see the defect marker at parse_ref).
//   - Data.Text.init on empty errors "Data.Text.init: empty input".
//   - Util.Text.textToInt (hsthrift common/util/Util/Text.hs:294-313):
//     empty fails; optional leading '-' then ASCII digits only ('+',
//     whitespace, trailing garbage all fail — "007" parses as 7, "-3" as
//     -3); the Int accumulator WRAPS ("99999999999999999999" parses as
//     7766279631452241919, GHC Int being 64-bit two's-complement).

// RULE §4 (imports): `use crate::…` within the crate (rulebook's literal `crate::glean::…` template predates the manifest's path layout — manifest is authoritative per §4; queued template note). `SourceRef` comes
// in ONLY via the `pub use` below (a second private import of the same
// name would be E0252).
use crate::angle::types::{FieldDef_, Name, PredicateId, TUPLE_FIELD, Type_, TypeId};
// Re-exported by the sibling types.rs from the generated glean_thrift.
use crate::angle::types::{PredicateRef, TypeRef, Version};

// The source's export list re-exports `SourceRef(..)` from Glean.Angle.Types
// (Util.hs:24) — mirrored as a named in-crate re-export (RULE §4 forbids
// glob re-exports ACROSS crates; this is neither), which also brings the
// name into scope for the code below.
pub use crate::angle::types::SourceRef;

/// `unit :: Type_ s a b` — `unit = RecordTy []` (Util.hs:35-36). Fully
/// polymorphic in the span/ref params, hence the three unused generics
/// (RULE §0: keep the shape; consumers instantiate `Type_` at many
/// ref types — see types.rs's Note [Types]).
pub fn unit<ST, P, T>() -> Type_<ST, P, T> {
    Type_::RecordTy(Vec::new())
}

/// A maybe type is equivalent to a sum type with {nothing,just} fields.
/// (Source doc.) `lowerMaybe :: Type_ s a b -> Type_ s a b` (Util.hs:39-40).
pub fn lower_maybe<ST, P, T>(param: Type_<ST, P, T>) -> Type_<ST, P, T> {
    Type_::SumTy(maybe_fields(param))
}

/// `maybeFields :: Type_ s a b -> [FieldDef_ s a b]` (Util.hs:42-45). The
/// param MOVES into the "just" field: the Haskell shares one immutable
/// value into exactly one field, so ownership transfers with no clone
/// (RULE §0 structure-preserving; sibling bimap/rm_loc_* take values by
/// value for the same reason).
pub fn maybe_fields<ST, P, T>(param: Type_<ST, P, T>) -> Vec<FieldDef_<ST, P, T>> {
    vec![
        FieldDef_ {
            field_def_name: "nothing".to_owned(),
            field_def_type: unit(),
        },
        FieldDef_ {
            field_def_name: "just".to_owned(),
            field_def_type: param,
        },
    ]
}

/// `lowerBool :: Type_ s a b` (Util.hs:47-48).
pub fn lower_bool<ST, P, T>() -> Type_<ST, P, T> {
    Type_::SumTy(bool_fields())
}

/// `boolFields :: [FieldDef_ s a b]` (Util.hs:50-51).
pub fn bool_fields<ST, P, T>() -> Vec<FieldDef_<ST, P, T>> {
    vec![
        FieldDef_ {
            field_def_name: "false".to_owned(),
            field_def_type: unit(),
        },
        FieldDef_ {
            field_def_name: "true".to_owned(),
            field_def_type: unit(),
        },
    ]
}

/// `lowerEnum :: [Name] -> Type_ s a b` (Util.hs:53-54).
pub fn lower_enum<ST, P, T>(ides: &[Name]) -> Type_<ST, P, T> {
    Type_::SumTy(enum_fields(ides))
}

/// `enumFields :: [Name] -> [FieldDef_ s a b]` (Util.hs:56-57). By-ref:
/// the Haskell callers (e.g. Unify.hs:72-75) pass the `[Name]` out of an
/// `EnumeratedTy` they still hold, mirroring the source's shared immutable
/// list; each field DEF owns a copy of its name regardless (RULE §0).
pub fn enum_fields<ST, P, T>(ides: &[Name]) -> Vec<FieldDef_<ST, P, T>> {
    ides.iter()
        .map(|ide| FieldDef_ {
            field_def_name: ide.clone(),
            field_def_type: unit(),
        })
        .collect()
}

/// `tupleSchema :: [Type_ s a b] -> Type_ s a b` (Util.hs:59-62) —
/// `FieldDef (tupleField <> Text.pack (show n)) ty`, n from 0.
/// `tupleField` is the sibling types.rs `TUPLE_FIELD` = "tuplefield"
/// (Types.hs:536-537); `show n` for a non-negative Int is decimal, as is
/// usize's Display. The types move from the list into their fields
/// (same no-clone argument as `maybe_fields`).
pub fn tuple_schema<ST, P, T>(tys: Vec<Type_<ST, P, T>>) -> Type_<ST, P, T> {
    Type_::RecordTy(
        tys.into_iter()
            .enumerate()
            .map(|(n, ty)| FieldDef_ {
                field_def_name: format!("{TUPLE_FIELD}{n}"),
                field_def_type: ty,
            })
            .collect(),
    )
}

/// Transcription of `textToInt` (hsthrift common/util/Util/Text.hs:294-313,
/// read from the pinned build sources — RULE §0). The `Left
/// InvalidConversion` payload ("textToInt: string \"…\" is not an
/// integer") is discarded: `parseRef`, this unit's only consumer, uses
/// just the Right/Left discriminator, and the message is never surfaced
/// anywhere in the source tree. RULE §1 (strings row): `Text` → `&str`.
// TODO(port): this transcription is private to this unit because
// Util.Text (hsthrift common/util) is kit-external and owns no manifest
// unit — but e.g. glass (Glass/Search/LSIF.hs, Glass/Search/Haskell.hs),
// shell (Shell.hs) and the CLI (GleanCLI/Derive.hs) call textToInt too,
// all in OTHER crates; if those units land, hoist this to a shared
// module (crate-local `util` or a glean-util surface) instead of
// private copies — same shape as display.rs's prettyprinter-ownership
// flag; deciding the home is a Step-4 wiring question.
fn text_to_int(txt: &str) -> Result<i64, ()> {
    let (negate, digits) = match txt.chars().next() {
        // len == 0 -> err
        None => return Err(()),
        // leading '-' with len > 1: negate <$> go 0 (off+1). The guard
        // uses byte length; outcome-equivalent to the source's byte length
        // (text-2.0.2 stores UTF-8 bytes), because the only strings that
        // parse are pure ASCII
        // digits (any non-digit tail is Err either way).
        Some('-') if txt.len() > 1 => (true, &txt[1..]),
        // any other lead char parses from offset 0 and fails in the
        // loop unless it is an ASCII digit; a bare "-" (len == 1) also
        // lands here and fails the loop — textToInt "-" is Left.
        Some(_) => (false, txt),
    };
    let mut n: i64 = 0;
    for c in digits.chars() {
        // c = code unit - ord '0', accepted only in 0..=9: every
        // non-ASCII-digit char fails here exactly as in the original
        // (its code point minus '0' falls outside 0..=9 in both UTF-16
        // and Rust char arithmetic; Rust `str` never contains surrogate
        // halves, which the original also rejects).
        let Some(d) = c.to_digit(10) else { return Err(()) };
        // GHC Int arithmetic wraps on overflow (verified: the 20-digit
        // input above parses as 7766279631452241919) — RULE §2 numeric
        // row: keep the source's wrap semantics.
        n = n.wrapping_mul(10).wrapping_add(d as i64);
    }
    Ok(if negate { n.wrapping_neg() } else { n })
}

/// Parse a predicate reference of the form @<predicate>[.<version>]@
/// If the version is omitted, it defaults to the most recent version
/// of that predicate.
///
/// This syntax is used in:
///  * The JSON format for writing facts
///  * Referring to predicates in the shell
/// (Source docs, Util.hs:64-71.)
pub fn parse_ref(txt: &str) -> SourceRef {
    // (before,after) = Text.breakOnEnd "." txt — the prefix up to and
    // INCLUDING the last '.', or ("", txt) when there is none (behavior
    // verified against text-2.0.2; see the header).
    let (before, after) = match txt.rfind('.') {
        Some(i) => (&txt[..i + 1], &txt[i + 1..]),
        None => ("", txt),
    };
    match text_to_int(after) {
        Ok(ver) => {
            // BUG(port): `Text.init before` is partial — a dotless
            // all-digits argument reaches it with before == "" and
            // crashes (RULE §2 BUG rule: reproduced bug-for-bug).
            // Repro: parseRef "123" → before = "", after = "123" parses,
            // so `Text.init ""` throws. Panicked here with the source's
            // exact message per the RULE §2 partial-functions row.
            // DEVIATION (RULE §2 TIMING row / A6 — which this eager
            // check VIOLATES, not sanctions): the Haskell throws only
            // when the name field is forced (lazy), this eager String
            // has no lazy cell to defer into, so the panic fires at
            // parse time. Unimplementable per-access without reshaping
            // SourceRef; TODO(port): amortize via TrySourceRef or a
            // NameCell newtype if any consumer proves observably
            // sensitive (surveyed: ~20 call sites; Write.hs:76 and
            // Resolve.hs:783-784 force immediately; Shell.hs:831 maps
            // over user words without immediate force — the one open
            // sensitivity).
            // Fix ships only as a flagged post-parity change.
            if before.is_empty() {
                panic!("Data.Text.init: empty input");
            }
            // Text.init before: drop the trailing '.', which is the last
            // Char and one byte, so the slice lands on a char boundary.
            SourceRef {
                source_ref_name: before[..before.len() - 1].to_owned(),
                // Just (fromIntegral ver): Int -> Version (thrift i32) —
                // RULE §2 numeric row: truncating `as`, wrapping like
                // fromIntegral on the 64→32-bit narrowing.
                source_ref_version: Some(ver as Version),
            }
        }
        Err(()) => SourceRef {
            source_ref_name: txt.to_owned(),
            source_ref_version: None,
        },
    }
}

/// Convert from a `PredicateRef` (oftem from 'getName') to `SourceRef`
/// (which is guaranteed to have a 'Version') — source doc, Util.hs:81-86,
/// typo "oftem" included. By-ref: the Haskell reads the fields without
/// consuming the argument (its only caller keeps using `pred`,
/// CompletePredicates.hs:307), and the name is cloned into the fresh
/// SourceRef exactly once.
pub fn convert_ref(p: &PredicateRef) -> SourceRef {
    SourceRef {
        source_ref_name: p.name.clone(),
        // fromIntegral (predicateRef_version p): Version -> Version —
        // the thrift i32 on both sides, an identity.
        source_ref_version: Some(p.version),
    }
}

/// `class ShowRef t where showRef :: t -> Text` (Util.hs:88-89).
/// RULE §1 (strings row): `Text` → `String`.
pub trait ShowRef {
    fn show_ref(&self) -> String;
}

/// Render the SourceRef to @name@ or @name.ver@ (source doc, Util.hs:91-94).
impl ShowRef for SourceRef {
    fn show_ref(&self) -> String {
        match self.source_ref_version {
            None => self.source_ref_name.clone(),
            // name <> "." <> showt ver — TextShow's Int32 instance is
            // signed decimal, i.e. i32's Display.
            Some(ver) => format!("{}.{}", self.source_ref_name, ver),
        }
    }
}

/// `instance ShowRef TypeRef` (Util.hs:96-97): name <> "." <> showt ver.
impl ShowRef for TypeRef {
    fn show_ref(&self) -> String {
        format!("{}.{}", self.name, self.version)
    }
}

/// `instance ShowRef PredicateRef` (Util.hs:99-100):
/// showRef = showRef . convertRef.
impl ShowRef for PredicateRef {
    fn show_ref(&self) -> String {
        convert_ref(self).show_ref()
    }
}

/// `instance ShowRef PredicateId` (Util.hs:102-103):
/// showRef name <> "." <> Text.pack (show hash). `show` on
/// GHC.Fingerprint is the 32-lowercase-hex rendering — provided by the
/// sibling hash.rs's `fmt::Display for Fingerprint`, used here through
/// the field with no re-declaration (RULE §4 assumed-imports; in-crate sibling per §0/§4 ownership).
impl ShowRef for PredicateId {
    fn show_ref(&self) -> String {
        format!("{}.{}", self.predicate_id_ref.show_ref(), self.predicate_id_hash)
    }
}

/// `instance ShowRef TypeId` (Util.hs:105-106): same shape over TypeRef +
/// hash.
impl ShowRef for TypeId {
    fn show_ref(&self) -> String {
        format!("{}.{}", self.type_id_ref.show_ref(), self.type_id_hash)
    }
}

/// `type NameSpaces = [Text]` (Util.hs:108).
pub type NameSpaces = Vec<Name>;

/// `splitDot :: Name -> (NameSpaces, Text)` (Util.hs:110-115): split on
/// '.', return the leading segments and the last segment. RULE §1
/// (strings row): `&str` in, owned pieces out.
pub fn split_dot(x: &str) -> (NameSpaces, Name) {
    // Text.split ('.' ==) x always yields at least one piece (even ""
    // yields [""]), so the source's `[] -> ([], "")` arm is unreachable —
    // not a §2 partial function; the `pop` default below is that dead
    // branch's value.
    let mut pieces: Vec<&str> = x.split('.').collect();
    let last = pieces.pop().unwrap_or("");
    // case reverse pieces of x : ys -> (reverse ys, x)
    (
        pieces.into_iter().map(str::to_owned).collect(),
        last.to_owned(),
    )
}

// ---------------------------------------------------------------------------
// Marker breakdown (closed set — RULE §6: reviewers check the trailer
// against this; prose only, so a raw grep for the marker names counts
// exactly the real sites)
// ---------------------------------------------------------------------------
//
// Open port-question markers (todo kind): 2
//   1. parse_ref: the empty-name input panics eagerly here while the
//      Haskell throws only when the name field is forced (lazy);
//      amortizing the check per-access would require reshaping the
//      sibling-owned SourceRef, so the divergence is flagged at the
//      site instead.
//   2. text_to_int ownership: the transcription is private to this unit
//      because Util.Text (hsthrift common/util) is kit-external and owns
//      no manifest unit — but glass (Glass/Search/LSIF.hs,
//      Glass/Search/Haskell.hs), shell (Shell.hs) and the CLI
//      (GleanCLI/Derive.hs) call textToInt too, all in OTHER crates; if
//      those units land, hoisting to a shared module (crate-local
//      `util` or a glean-util surface) instead of private copies is a
//      Step-4 wiring question, same shape as display.rs's
//      prettyprinter-ownership flag.
// Defect markers (bug kind): 1 — parse_ref's Text.init-on-empty,
// reproduced bug-for-bug with the source's exact panic message (a
// defect marker, not a todo).
// No perf or unsafe markers: nothing perf-critical beyond the obvious
// Vec/String allocations the whole first pass shares, and no unsafe
// code in this pure unit.
//
// Coverage: every export of the Haskell module has a Rust counterpart —
// unit, lowerBool, lowerMaybe, lowerEnum, boolFields, maybeFields,
// enumFields, tupleSchema, parseRef, convertRef, ShowRef (trait + all five
// instances: SourceRef, TypeRef, PredicateRef, PredicateId, TypeId),
// NameSpaces, splitDot, and the SourceRef(..) re-export. The only
// additional code is the private text_to_int standing in for the external
// Util.Text import. Tests are not translated (RULE §4; the judge is the
// referee).
//
// MODULE-WIRING: lib.rs / schema/mod.rs scaffolding is deliberately NOT a
// manifest unit; owned by the Step-4 survey build (do not create ad-hoc
// here) — same note as the sibling types.rs.
//
// SAFETY count: 0 unsafe constructs (pure safe Rust — no FFI, no unsafe
// blocks in this unit).
// PORT STATUS: confidence=high todos=2
