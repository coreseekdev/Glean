// Ported from: glean/angle/Glean/Angle/Types.hs (1216 lines)
// Manifest unit: rust/glean-angle/src/angle/types.rs (crate glean-angle) —
// migration/manifest.tsv row 4. RULE §4 (naming): one source file → one
// target file; the manifest path is authoritative.
//
// INVENTORY (RULE §5): this unit is governed by migration/inventory.tsv
// row 6 (glean/angle/Glean/Angle/Types.hs, "11 Binary instances") plus the
// eleven per-site binary-fmt rows in migration/inventory.sites.tsv:420-430
// (:150 SourceRef, :185 Ordered, :266 SeekSection, :363 PrimOp,
// :447 RecordOrSum, :577 PredicateRef, :581 PredicateId, :600 TypeRef,
// :604 TypeId, :613 DerivingInfo q, :629 DeriveWhen). The row's
// target_translation is followed exactly: "one hand-written
// Data.Binary-format encoder module (RULEBOOK 2 binary-fmt row): replicate
// GHC Data.Binary encoding EXACTLY per instance — generic-derived defaults
// (constructor tag + field order), hand-written pairs (:577 put a >> put b),
// and the bare-derived PredicateId/TypeId (which serialize TypeRef+2-word
// Fingerprint)". No divergence from the row; no INVENTORY-BUG-CANDIDATE.
//
// This file IS the Data.Binary encoder module for the port (RULE §2
// "Data.Binary serializations feeding hashes" row). Every byte-level fact
// below was pinned from the actual package sources, not from memory:
//   - binary-0.8.9.1 (the GHC 9.4.7 boot lib this build compiles against;
//     /home/nzinfo/glean-build/ghc-9.4.7-install/.../binary-0.8.9.1):
//     src/Data/Binary/Class.hs and src/Data/Binary/Generic.hs
//   - text-2.0.2 (GHC 9.4.7 boot lib): src/Data/Text.hs:385 —
//     `instance Binary Text where put t = put (encodeUtf8 t)` (the Text
//     Binary instance comes from the text package, verified present in the
//     built interface files; neither binary nor binary-orphans 1.0.5 has
//     one). Provenance was checked by grepping the built .hi files for the
//     `$fBinaryText` dictionary and by reading the sources.
//   - bytestring-0.11.5.2 (the version in the GHC install),
//     base-4.17.1.0 GHC.Show (showLitChar/showLitString/asciiTab) for the
//     `show :: ByteString -> String` Display path, aeson-2.2.3.0
//     Data/Aeson/Encoding/Builder.hs escapeAscii for the String-pattern
//     Display path.
//
// SIBLING CONTRACTS (RULE §4 B4, the assumed-import regime for drafts):
//   - rust/glean-angle/src/angle/hash.rs (DONE sibling) does
//     `use crate::angle::types::Binary;` and its contract comment pins the
//     trait shape to `pub trait Binary { fn encode(&self) -> Vec<u8>; }`
//     with raw-runPut-stream semantics (no top-level length header,
//     fixed-width words BIG-ENDIAN). The trait below matches that contract
//     verbatim. hash_binary<T: Binary> consumes it; this module never
//     re-declares anything hash.rs owns (Fingerprint/Hash/hash0/
//     fingerprint_data/hash_byte_string).
//   - rust/glean-angle/src/display.rs (DONE sibling) owns `Doc`, the
//     combinator surface (hsep/vsep/sep/cat/vcat/hcat/punctuate/nest/hang/
//     parens/brackets/braces/enclose_sep), `DisplayOpts`/`PredicateStyle`,
//     and the `Display` trait with the default `display_atom`. Every impl
//     below implements THAT trait and never re-declares it.
//
// ASSUMED-IMPORT (RULE §4 B4): `PredicateRef`, `TypeRef` and `Version`
// are thrift types (glean/if/glean.thrift:39 `typedef i32 Version`, :55
// `struct PredicateRef { 1: PredicateName name; 2: Version version; }`,
// :68 `struct TypeRef`). RULE §1 (thrift row): thrift types are generated
// by build.rs into the pinned crate `glean_thrift` — generated code is
// never a manifest unit and never hand-translated, so this import assumes:
//     #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
//     pub struct PredicateRef { pub name: String, pub version: Version }
//     #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
//     pub struct TypeRef      { pub name: String, pub version: Version }
//     pub type Version = i32;   // thrift typedef i32; gen-hs2 emits
//                               // `type Version = Int.Int32` (a synonym)
// (the derived-conventional bounds mirror the gen-hs2 deriving
// `(Prelude.Eq, Prelude.Show, Prelude.Ord)` plus value semantics;
// PredicateId's derived Ord below needs Ord on PredicateRef/TypeRef)
// The Haskell source imported these from the generated `Glean.Types`
// (Angle/Types.hs:131) and gave them ORPHAN instances here — `Binary
// PredicateRef` (:577), `Binary TypeRef` (:600), `Display PredicateRef`
// (:733), `Display TypeRef` (:736), hence the source's `-Wno-orphans`.
// In Rust these are plain local-trait impls for a foreign type (legal,
// no coherence problem) and they live in THIS unit, mirroring the source.
// The generated code's field names (`name`, `version`) follow fbthrift's
// Rust codegen; if the generated crate ships different field names, only
// the `use` sites in this file change.
//
// Debug-vs-Show convention (RULE §2 deriving-Show row): `#[derive(Debug)]`
// is the port-wide stand-in for `deriving Show`. Haskell's derived Show
// renders record selectors in camelCase (`predicateIdRef = ...`); Rust's
// Debug renders the snake_case field names (`predicate_id_ref: ...`). The
// constructor-shaped TAG (`PredicateId { .. }`, `SourcePat_::Nat(..)`)
// — the part the RULE §2 row makes load-bearing for error/sum surfaces —
// is preserved. Deep record-layout Show text differs systematically
// crate-wide; no persisted or golden surface in this unit renders via
// Show (the golden surfaces go through Display below), so this is a
// documented convention, not a per-type TODO. Types whose Haskell
// declaration derived only Eq (TypeDef_, PredicateDef_, DerivingDef_,
// SourceEvolves_, SourceSchema_, SourceSchemas_, SourceDecl_) still get
// derive(Debug) under the same row's "constructor-shaped naming is the
// default for types with no Show" clause.

use std::collections::BTreeMap;

// RULE §4 (imports): `use crate::…` within the crate. Sibling units:
use crate::angle::hash::{hash0, hash_fingerprint, Hash};
// (Fingerprint itself is only needed for the Binary impl below; `Hash` is
// hash.rs's alias for it.)
use crate::display::{
    append_space, braces, brackets, cat, enclose_sep, hang, hcat, hsep, nest, parens, punctuate,
    sep, vcat, vsep, Display, DisplayOpts, Doc, PredicateStyle,
};

// ASSUMED-IMPORT (RULE §4 B4): see the header block.
pub use glean_thrift::{PredicateRef, TypeRef, Version}; // RULE §0: Types.hs:21,131 re-exports these via this module

// ---------------------------------------------------------------------------
// The Data.Binary encoder (inventory.tsv row 6; RULE §2 Data.Binary row)
// ---------------------------------------------------------------------------
//
// Encode-only on purpose. The Haskell instances' `get` sides have no
// consumer in the hash chain — Hash.hs:47-48 is `hashByteString .
// LB.toStrict . Binary.encode` (put-only), and the only other serial
// boundary in the Glean tree is thrift, which does not go through
// Data.Binary. If a future unit needs decode, extend this trait with a
// `Get`-style decoder in this module (the one owner of the format).
//
// The encoding contract, pinned from binary-0.8.9.1 sources:
//
// Primitives (src/Data/Binary/Class.hs):
//   - `()`          : zero bytes (Class.hs:193-195)
//   - Bool          : 1 byte, fromEnum (Class.hs:198)
//   - Word8/16/32/64: fixed-width BIG-ENDIAN (Class.hs:220-253)
//   - Int8/16/32/64 : fixed-width BIG-ENDIAN two's complement
//                     (Class.hs:256-289; putIntNbe = builder BE)
//   - Int           : putInt64be . fromIntegral — 8 bytes BE (Class.hs:303)
//   - Word          : putWord64be . fromIntegral (Class.hs:294)
//   - Char          : UTF-8 (Class.hs:481) — [Char] encodes via the
//                     putList override `put (length str) <> putStringUtf8
//                     str` (Class.hs:483): Int64be CHAR-COUNT prefix +
//                     UTF-8 bytes. Rust's String maps the *Text* instance
//                     (below), so the [Char] shape is exposed separately
//                     as [`encode_char_string`] for genuine Haskell
//                     String values. Note that `showRef` is NOT one of
//                     them: it returns Text (Schema/Util.hs:88-89), so the
//                     ComputeIds schema hash uses `Binary for String`.
//   - Text          : text-2.0.2 Data/Text.hs:385-386
//                     `put t = put (encodeUtf8 t)`, i.e. the strict
//                     ByteString instance over the UTF-8 bytes.
//   - ByteString    : `put (B.length bs) <> putByteString bs`
//                     (Class.hs:625) — Int64be BYTE-COUNT prefix + bytes.
//   - [a]           : defaultPutList (Class.hs:592-593 via :588):
//                     Int64be length prefix + elements in order. (The
//                     Word/Int/Char putList overrides only matter for
//                     lists of those primitives; no such list occurs in
//                     the AST encoded here.)
//   - Maybe a       : Word8 tag 0 / 1 + payload (Class.hs:606-608)
//   - tuples        : fields left-to-right, NO tag (Class.hs:513-521)
//   - Map k v       : Int64be size + (k,v) pairs in ASCENDING key order
//                     (Class.hs:655, Map.toAscList) — BTreeMap below.
//   - NonEmpty a    : put . toList, i.e. the plain list encoding
//                     (Class.hs:844-850)
//   - Fingerprint   : put x1 <> put x2 — two Word64be words,
//                     fingerprint1 first (Class.hs:739-742)
//
// Generic derivation (src/Data/Binary/Generic.hs + GHC.Generics):
//   - single-constructor types emit NO tag (there is no (:+:) node in
//     the Rep; GBinaryPut (M1/C1/K1/:*:) just concatenates fields in
//     declaration order);
//   - sums with n constructors emit a constructor tag before the fields:
//     Word8 when n-1 <= 255, Word16be when n-1 <= 65535, Word32be when
//     n-1 <= 2^32-1 (Generic.hs:86-93 GUARD/PUTSUM). Every sum in this
//     file has <= 27 constructors, so the tag is always ONE byte — the
//     width rule is still stated here and asserted by debug_assert.
//   - tag VALUE: Generic.hs:129-141 builds the code by splitting size
//     `sizeL = size >> 1` at each (:+:) node. GHC.Generics deriving
//     emits exactly that balanced nesting (verified with the build's GHC
//     9.4.7: `:kind! Rep Foo` for a 5-constructor type gives
//     ((A:+:B):+:(C:+:(D:+:E)))), so the code is the constructor's
//     0-based LEFT-TO-RIGHT declaration index.
//
// The chain this serves (inventory.tsv row 6 evidence column):
// hash_binary (hash.rs) <- fingerprintDef/cycleHash/hash+cycleHash/
// schemaId (ComputeIds.hs:161,169,270,273,342) and dbSchemaKey
// (Schema.hs:270-280). Byte-exactness is judged by the old-binary
// getSchemaInfo case — see the TODO(port) at the bottom of this file.

// PERF(port): every `encode` allocates and concatenates fresh Vec<u8>s
// through the recursion (put a <> put b compositionally). Fast version =
// a `&mut Vec<u8>` sink or the Builder pattern over a single buffer;
// first pass is slow-but-faithful per RULE §0.
pub trait Binary {
    /// Reproduce `Data.Binary.encode` composed with `LB.toStrict`:
    /// the raw runPut byte stream, no framing.
    fn encode(&self) -> Vec<u8>;
}

/// Concatenation for the encoder below (NOT prettyprinter's `cat`,
/// which is imported from display.rs for the Struct Display arm).
fn enc_cat(mut a: Vec<u8>, b: Vec<u8>) -> Vec<u8> {
    a.extend_from_slice(&b);
    a
}

fn put_word8(w: u8) -> Vec<u8> {
    vec![w]
}

/// One-byte constructor tag for the generic-derived sums in this file
/// (binary-0.8.9.1 Generic.hs:86-93: n <= 256 constructors => Word8).
fn put_ctor_tag(index: u8, num_ctors: usize) -> Vec<u8> {
    debug_assert!(
        num_ctors <= 256,
        "sum wider than 256 constructors needs a Word16be tag (binary Generic.hs GUARD)"
    );
    debug_assert!(
        (index as usize) < num_ctors,
        "ctor tag out of range: N must equal the enum's constructor count (tripwire for future ctors)"
    );
    let _ = num_ctors;
    put_word8(index)
}

impl Binary for () {
    fn encode(&self) -> Vec<u8> {
        Vec::new() // Class.hs:193 put () = mempty
    }
}

impl Binary for bool {
    fn encode(&self) -> Vec<u8> {
        put_word8(*self as u8) // Class.hs:198 fromEnum
    }
}

macro_rules! impl_binary_be {
    ($t:ty, $n:expr, $doc:expr) => {
        #[doc = $doc]
        impl Binary for $t {
            fn encode(&self) -> Vec<u8> {
                self.to_be_bytes().to_vec()
            }
        }
    };
}

impl_binary_be!(u8, 1, "Word8: putWord8 (Class.hs:220).");
impl_binary_be!(u16, 2, "Word16: putWord16be (Class.hs:229).");
impl_binary_be!(u32, 4, "Word32: putWord32be (Class.hs:238).");
impl_binary_be!(u64, 8, "Word64: putWord64be (Class.hs:247).");
impl_binary_be!(i8, 1, "Int8: putInt8 — BE two's complement (Class.hs:256).");
impl_binary_be!(i16, 2, "Int16: putInt16be — BE two's complement (Class.hs:265).");
impl_binary_be!(i32, 4, "Int32: putInt32be — BE two's complement (Class.hs:274).");
impl_binary_be!(i64, 8, "Int: putInt64be . fromIntegral — 8 bytes BE (Class.hs:303).");

/// Text (RULE §1 strings row: Haskell `Text` → Rust `String`):
/// text-2.0.2 Data/Text.hs:385 `put t = put (encodeUtf8 t)` = the strict
/// ByteString instance: Int64be BYTE-count prefix + UTF-8 bytes.
impl Binary for String {
    fn encode(&self) -> Vec<u8> {
        enc_cat((self.len() as i64).encode(), self.as_bytes().to_vec())
    }
}

/// ByteString (RULE §1 strings row: `Vec<u8>`):
/// `put (B.length bs) <> putByteString bs` (Class.hs:625).
impl Binary for Vec<u8> {
    fn encode(&self) -> Vec<u8> {
        enc_cat((self.len() as i64).encode(), self.clone())
    }
}

/// `[a]`: defaultPutList (Class.hs:592 via :588) — Int64be length
/// prefix + elements in order. A free function, not an impl: Haskell
/// distinguishes `ByteString` from `[a]` at the type level, but both
/// map onto `Vec<_>` in Rust (RULE §1 strings row), and Rust coherence
/// forbids `Vec<u8>` (the ByteString shape below) coexisting with a
/// blanket `Vec<T>` list impl. The two encodings differ (raw bytes vs
/// per-element), so lists spell out [`encode_list`] at each encode
/// site, exactly the distinction instance selection made in Haskell.
pub fn encode_list<T: Binary>(xs: &[T]) -> Vec<u8> {
    let mut out = (xs.len() as i64).encode();
    for x in xs {
        out = enc_cat(out, x.encode());
    }
    out
}

/// `NonEmpty a`: `put = put . NE.toList` (Class.hs:849-850) — identical
/// bytes to the list encoding.
impl<T: Binary> Binary for NonEmpty<T> {
    fn encode(&self) -> Vec<u8> {
        encode_list(&self.to_vec())
    }
}

/// `Maybe a` (Class.hs:606-608).
impl<T: Binary> Binary for Option<T> {
    fn encode(&self) -> Vec<u8> {
        match self {
            None => put_word8(0),
            Some(x) => enc_cat(put_word8(1), x.encode()),
        }
    }
}

/// `(a,b)` (Class.hs:513-517): fields left-to-right, no tag.
impl<A: Binary, B: Binary> Binary for (A, B) {
    fn encode(&self) -> Vec<u8> {
        enc_cat(self.0.encode(), self.1.encode())
    }
}

/// `(a,b,c)` (Class.hs:519-521).
impl<A: Binary, B: Binary, C: Binary> Binary for (A, B, C) {
    fn encode(&self) -> Vec<u8> {
        enc_cat(enc_cat(self.0.encode(), self.1.encode()), self.2.encode())
    }
}

/// `(a,b,c,d)`: same shape as the 2/3-tuples (Class.hs "Instances for
/// the first few tuples" continues through 4 fields). ComputeIds.hs:273
/// hashes a 4-tuple.
impl<A: Binary, B: Binary, C: Binary, D: Binary> Binary for (A, B, C, D) {
    fn encode(&self) -> Vec<u8> {
        enc_cat(
            enc_cat(enc_cat(self.0.encode(), self.1.encode()), self.2.encode()),
            self.3.encode(),
        )
    }
}

/// `Map k v` (Class.hs:655): Int64be size + (k,v) pairs in ASCENDING key
/// order. RULE §1: Haskell's sorted `Data.Map` maps onto `BTreeMap`.
impl<K: Binary + Ord, V: Binary> Binary for BTreeMap<K, V> {
    fn encode(&self) -> Vec<u8> {
        let mut out = (self.len() as i64).encode();
        for (k, v) in self {
            out = enc_cat(out, enc_cat(k.encode(), v.encode()));
        }
        out
    }
}

/// `GHC.Fingerprint` — binary's own instance (Class.hs:739-742):
/// `put (Fingerprint x1 x2) = put x1 <> put x2`, two Word64be words,
/// fingerprint1 first. The type lives in the sibling hash.rs; the Binary
/// impl lives HERE because this module is the one owner of the
/// Data.Binary format (inventory.tsv row 6).
impl Binary for crate::angle::hash::Fingerprint {
    fn encode(&self) -> Vec<u8> {
        enc_cat(self.fingerprint1.encode(), self.fingerprint2.encode())
    }
}

/// Data.Binary's `[Char]`/String encoding — NOT the same as Text's.
/// Char's putList override (Class.hs:483-484) is
/// `put (length str) <> putStringUtf8 str` where `length` counts CHARS,
/// while the Text instance prefixes the UTF-8 BYTE count. Divergent for
/// any non-ASCII string.
///
/// NOT used by the ComputeIds unit: `showRef :: t -> Text`
/// (glean/angle/Glean/Schema/Util.hs:88-89), so
/// `Data.Binary.encode (showRef ref)` at ComputeIds.hs:270,273 takes the
/// Text instance — byte-count prefix + UTF-8 — which Rust's
/// `Binary for String` already provides. This function exists for
/// genuine `[Char]` values only, so the two encodings cannot be silently
/// mixed.
pub fn encode_char_string(s: &str) -> Vec<u8> {
    let mut out = (s.chars().count() as i64).encode();
    out.extend_from_slice(s.as_bytes()); // putStringUtf8
    out
}

// ---------------------------------------------------------------------------
// Names and references
// ---------------------------------------------------------------------------

/// `type Name = Text` (RULE §1 strings row).
pub type Name = String;

/// `type FieldName = Text`.
pub type FieldName = String;

/// A reference to a name or type in a source schema. Versions are
/// often omitted at this level, and will be later resolved by
/// resolveSchema. (Source docs.)
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceRef {
    pub source_ref_name: Name,
    pub source_ref_version: Option<Version>,
}

/// `instance Binary SourceRef` (Types.hs:150) — generic-derived default:
/// single constructor, so NO tag; fields in declaration order
/// (name :: Text, version :: Maybe Version).
impl Binary for SourceRef {
    fn encode(&self) -> Vec<u8> {
        enc_cat(self.source_ref_name.encode(), self.source_ref_version.encode())
    }
}

// ---------------------------------------------------------------------------
// Source locations
// ---------------------------------------------------------------------------

/// A 'SrcSpan' delimits a portion of a text file.
/// The end position is the column /after/ the end of the span.
/// That is, a span of (1,1)-(1,2) is one character long, and a span of
/// (1,1)-(1,1) is zero characters long. (Source docs.)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SrcSpan {
    pub span_start: SrcLoc, // inclusive ({-# UNPACK #-} in source)
    pub span_end: SrcLoc,   // exclusive
}

/// A point in a source text file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SrcLoc {
    pub loc_line: i64, // Int (RULE §2 numeric row: Haskell Int = 64-bit)
    pub loc_col: i64,
}

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceQuery_<S, ST, P, T> {
    pub src_query_head: Option<SourcePat_<S, ST, P, T>>,
    pub src_query_stmts: Vec<SourceStatement_<S, ST, P, T>>,
    pub src_query_ordered: Ordered,
}

/// `instance (Binary p, Binary t) => Binary (SourceQuery_ () () p t)`
/// (Types.hs:187) — generic-derived: single constructor, fields in order
/// (Maybe head, [stmts], Ordered). Spans are `()` and contribute zero
/// bytes, which is exactly why the source constrains s = st = ().
impl<P: Binary, T: Binary> Binary for SourceQuery_<(), (), P, T> {
    fn encode(&self) -> Vec<u8> {
        enc_cat(
            enc_cat(self.src_query_head.encode(), encode_list(&self.src_query_stmts)),
            self.src_query_ordered.encode(),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ordered {
    Ordered,
    Unordered,
}

/// `instance Binary Ordered` (Types.hs:185) — generic-derived sum of 2:
/// Word8 tag = declaration index.
impl Binary for Ordered {
    fn encode(&self) -> Vec<u8> {
        match self {
            Ordered::Ordered => put_ctor_tag(0, 2),
            Ordered::Unordered => put_ctor_tag(1, 2),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceStatement_<S, ST, P, T>(pub SourcePat_<S, ST, P, T>, pub SourcePat_<S, ST, P, T>);

/// `instance (Binary p, Binary t) => Binary (SourceStatement_ () () p t)`
/// (Types.hs:201) — single constructor, both pats in order.
impl<P: Binary, T: Binary> Binary for SourceStatement_<(), (), P, T> {
    fn encode(&self) -> Vec<u8> {
        enc_cat(self.0.encode(), self.1.encode())
    }
}

/// The Angle source pattern AST. Phantom parameters (kept 1:1 from the
/// Haskell because consumers parametrize them — RULE §2 GADT row: closed
/// enum; the *parametrization* is preserved with Rust generics):
///   - `S`: the span type on patterns (`SrcSpan` parsed, `()` after
///     rmLoc for hashing)
///   - `ST`: the span type inside nested `Type_`s (same story)
///   - `P`: predicate reference (`SourceRef` parsed, `PredicateRef`
///     resolved, `PredicateId` hashed)
///   - `T`: type reference (`SourceRef`/`TypeRef`/`TypeId`)
/// Consumers: Query/Typecheck instantiates `SourcePat_ s st p t` with
/// `IsSrcSpan s` (Typecheck/Monad.hs:169,178); ComputeIds bimaps
/// `SourcePat_ SrcSpan SrcSpan PredicateRef TypeRef` to the Id versions
/// before hashing (ComputeIds.hs:76-89 refsToIds).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourcePat_<S, ST, P, T> {
    Nat(S, u64),
    String(S, String),
    StringPrefix(S, String),
    ByteArray(S, Vec<u8>),
    /// There's no concrete syntax for this (yet), but it can be used
    /// via the DSL. (Source comment.)
    Array(S, Vec<SourcePat_<S, ST, P, T>>),
    // Box on the NonEmpty: NonEmpty's `head: T` is unboxed, so the
    // unboxed form is E0072 (recursive without indirection — discovered
    // at unit 44, coordinator-repaired; the Haskell payload is
    // `NonEmpty (SourcePat_ v t p)` with free sharing, which the Box
    // preserves at one indirection's cost).
    ArrayPrefix(S, Box<NonEmpty<SourcePat_<S, ST, P, T>>>),
    Tuple(S, Vec<SourcePat_<S, ST, P, T>>),
    Struct(S, Vec<Field<S, ST, P, T>>),
    App(
        S,
        Box<SourcePat_<S, ST, P, T>>,
        Vec<SourcePat_<S, ST, P, T>>,
    ),
    KeyValue(S, Box<SourcePat_<S, ST, P, T>>, Box<SourcePat_<S, ST, P, T>>),
    Elements(S, Box<SourcePat_<S, ST, P, T>>),
    All(S, Box<SourcePat_<S, ST, P, T>>),
    Wildcard(S),
    Variable(S, Name),
    ElementsOfArray(S, Box<SourcePat_<S, ST, P, T>>),
    OrPattern(S, Box<SourcePat_<S, ST, P, T>>, Box<SourcePat_<S, ST, P, T>>),
    NestedQuery(S, Box<SourceQuery_<S, ST, P, T>>),
    Negation(S, Box<SourcePat_<S, ST, P, T>>),
    FactId(S, Option<String>, u64),
    TypeSignature(S, Box<SourcePat_<S, ST, P, T>>, Box<Type_<ST, P, T>>),
    Never(S),
    // IfPattern is the source's one record-syntax constructor; field
    // order in the Generic encoding is declaration order (span, cond,
    // then_, else_).
    IfPattern {
        span: S,
        cond: Box<SourcePat_<S, ST, P, T>>,
        then_: Box<SourcePat_<S, ST, P, T>>,
        else_: Box<SourcePat_<S, ST, P, T>>,
    },
    FieldSelect(S, Box<SourcePat_<S, ST, P, T>>, FieldName, RecordOrSum),
    Deref(S, Box<SourcePat_<S, ST, P, T>>),
    Enum(S, String),
    // The following forms are introduced by the resolver, and replace
    // the Variable and App forms produced by the parser. (Source
    // comment.) Note Clause carries TWO spans (pattern span, predicate
    // span) plus the resolved predicate `p`.
    Clause(S, S, P, Box<SourcePat_<S, ST, P, T>>, SeekSection),
    Prim(S, PrimOp, Vec<SourcePat_<S, ST, P, T>>),
}

/// `instance (Binary p, Binary t) => Binary (SourcePat_ () () p t)`
/// (Types.hs:268) — generic-derived over a 27-constructor sum: Word8 tag
/// = 0-based declaration index, then fields in order; `()` spans
/// contribute zero bytes; the nested `Type_ st p t` in TypeSignature
/// serializes with st = ().
impl<P: Binary, T: Binary> Binary for SourcePat_<(), (), P, T> {
    fn encode(&self) -> Vec<u8> {
        // 27 constructors in declaration order (Types.hs:210-246).
        const N: usize = 27;
        match self {
            SourcePat_::Nat(_, w) => enc_cat(put_ctor_tag(0, N), w.encode()),
            SourcePat_::String(_, s) => enc_cat(put_ctor_tag(1, N), s.encode()),
            SourcePat_::StringPrefix(_, s) => enc_cat(put_ctor_tag(2, N), s.encode()),
            SourcePat_::ByteArray(_, b) => enc_cat(put_ctor_tag(3, N), b.encode()),
            SourcePat_::Array(_, pats) => enc_cat(put_ctor_tag(4, N), encode_list(pats)),
            SourcePat_::ArrayPrefix(_, pats) => enc_cat(put_ctor_tag(5, N), pats.encode()),
            SourcePat_::Tuple(_, pats) => enc_cat(put_ctor_tag(6, N), encode_list(pats)),
            SourcePat_::Struct(_, fields) => enc_cat(put_ctor_tag(7, N), encode_list(fields)),
            SourcePat_::App(_, l, r) => {
                enc_cat(put_ctor_tag(8, N), enc_cat(l.encode(), r.encode()))
            }
            SourcePat_::KeyValue(_, k, v) => {
                enc_cat(put_ctor_tag(9, N), enc_cat(k.encode(), v.encode()))
            }
            SourcePat_::Elements(_, pat) => enc_cat(put_ctor_tag(10, N), pat.encode()),
            SourcePat_::All(_, query) => enc_cat(put_ctor_tag(11, N), query.encode()),
            SourcePat_::Wildcard(_) => put_ctor_tag(12, N),
            SourcePat_::Variable(_, n) => enc_cat(put_ctor_tag(13, N), n.encode()),
            SourcePat_::ElementsOfArray(_, pat) => enc_cat(put_ctor_tag(14, N), pat.encode()),
            SourcePat_::OrPattern(_, l, r) => {
                enc_cat(put_ctor_tag(15, N), enc_cat(l.encode(), r.encode()))
            }
            SourcePat_::NestedQuery(_, q) => enc_cat(put_ctor_tag(16, N), q.encode()),
            SourcePat_::Negation(_, pat) => enc_cat(put_ctor_tag(17, N), pat.encode()),
            SourcePat_::FactId(_, name, n) => {
                enc_cat(put_ctor_tag(18, N), enc_cat(name.encode(), n.encode()))
            }
            SourcePat_::TypeSignature(_, pat, ty) => {
                enc_cat(put_ctor_tag(19, N), enc_cat(pat.encode(), ty.encode()))
            }
            SourcePat_::Never(_) => put_ctor_tag(20, N),
            SourcePat_::IfPattern { span: _, cond, then_, else_ } => enc_cat(
                put_ctor_tag(21, N),
                enc_cat(enc_cat(cond.encode(), then_.encode()), else_.encode()),
            ),
            SourcePat_::FieldSelect(_, pat, field, q) => enc_cat(
                put_ctor_tag(22, N),
                enc_cat(enc_cat(pat.encode(), field.encode()), q.encode()),
            ),
            SourcePat_::Deref(_, pat) => enc_cat(put_ctor_tag(23, N), pat.encode()),
            SourcePat_::Enum(_, f) => enc_cat(put_ctor_tag(24, N), f.encode()),
            SourcePat_::Clause(_, _, p, pat, rng) => enc_cat(
                put_ctor_tag(25, N),
                enc_cat(enc_cat(p.encode(), pat.encode()), rng.encode()),
            ),
            SourcePat_::Prim(_, p, pats) => {
                enc_cat(put_ctor_tag(26, N), enc_cat(p.encode(), encode_list(pats)))
            }
        }
    }
}

/// Should a `seek` call be restricted to a section of the database?
///
/// When performing a query over a stacked database we have three levels
///  * base db
///  * stacked db
///  * writable FactSet
///
/// The writable FactSet is there to store facts from derived predicates
/// generated during the querying.
///
/// This type specifies which of these sections should be be covered in a
/// seek. (Source docs — typo "be be" included.)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeekSection {
    /// base + stacked + writable
    SeekOnAllFacts,
    /// base only
    SeekOnBase,
    /// stacked only
    SeekOnStacked,
}

/// `instance Binary SeekSection` (Types.hs:266) — generic-derived sum of
/// 3: Word8 tags 0/1/2.
impl Binary for SeekSection {
    fn encode(&self) -> Vec<u8> {
        match self {
            SeekSection::SeekOnAllFacts => put_ctor_tag(0, 3),
            SeekSection::SeekOnBase => put_ctor_tag(1, 3),
            SeekSection::SeekOnStacked => put_ctor_tag(2, 3),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field<S, ST, P, T>(pub FieldName, pub SourcePat_<S, ST, P, T>);

/// `instance (Binary p, Binary t) => Binary (Field () () p t)`
/// (Types.hs:336) — single constructor: name then pat.
impl<P: Binary, T: Binary> Binary for Field<(), (), P, T> {
    fn encode(&self) -> Vec<u8> {
        enc_cat(self.0.encode(), self.1.encode())
    }
}

/// `data.NonEmpty` stand-in: the non-emptiness is part of the DSL
/// contract (ArrayPrefix needs at least one element), so it stays a
/// distinct type instead of collapsing to Vec (RULE §0
/// structure-preserving).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NonEmpty<T> {
    pub head: T,
    pub tail: Vec<T>,
}

impl<T> NonEmpty<T> {
    pub fn to_vec(&self) -> Vec<T>
    where
        T: Clone,
    {
        let mut v = Vec::with_capacity(self.tail.len() + 1);
        v.push(self.head.clone());
        v.extend(self.tail.iter().cloned());
        v
    }

    pub fn iter(&self) -> std::iter::Chain<std::option::IntoIter<&T>, std::slice::Iter<'_, T>> {
        Some(&self.head).into_iter().chain(self.tail.iter())
    }
}

/// Primitive operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrimOp {
    PrimOpToLower,
    PrimOpLength,
    PrimOpZip,
    PrimOpConcat,
    PrimOpReverse,
    PrimOpSize,
    PrimOpRelToAbsByteSpans,
    PrimOpUnpackByteSpans,
    PrimOpGtNat,
    PrimOpGeNat,
    PrimOpLtNat,
    PrimOpLeNat,
    PrimOpNeNat,
    PrimOpAddNat,
    PrimOpNeExpr,
}

/// `instance Binary PrimOp` (Types.hs:363) — generic-derived sum of 15:
/// Word8 tags 0..14 in declaration order.
impl Binary for PrimOp {
    fn encode(&self) -> Vec<u8> {
        const N: usize = 15;
        match self {
            PrimOp::PrimOpToLower => put_ctor_tag(0, N),
            PrimOp::PrimOpLength => put_ctor_tag(1, N),
            PrimOp::PrimOpZip => put_ctor_tag(2, N),
            PrimOp::PrimOpConcat => put_ctor_tag(3, N),
            PrimOp::PrimOpReverse => put_ctor_tag(4, N),
            PrimOp::PrimOpSize => put_ctor_tag(5, N),
            PrimOp::PrimOpRelToAbsByteSpans => put_ctor_tag(6, N),
            PrimOp::PrimOpUnpackByteSpans => put_ctor_tag(7, N),
            PrimOp::PrimOpGtNat => put_ctor_tag(8, N),
            PrimOp::PrimOpGeNat => put_ctor_tag(9, N),
            PrimOp::PrimOpLtNat => put_ctor_tag(10, N),
            PrimOp::PrimOpLeNat => put_ctor_tag(11, N),
            PrimOp::PrimOpNeNat => put_ctor_tag(12, N),
            PrimOp::PrimOpAddNat => put_ctor_tag(13, N),
            PrimOp::PrimOpNeExpr => put_ctor_tag(14, N),
        }
    }
}

/// `sourcePatSpan :: SourcePat_ s st p t -> s` (Types.hs:365-393).
pub fn source_pat_span<S, ST, P, T>(pat: &SourcePat_<S, ST, P, T>) -> &S {
    match pat {
        SourcePat_::Nat(s, _)
        | SourcePat_::String(s, _)
        | SourcePat_::StringPrefix(s, _)
        | SourcePat_::ByteArray(s, _)
        | SourcePat_::Array(s, _)
        | SourcePat_::ArrayPrefix(s, _)
        | SourcePat_::Tuple(s, _)
        | SourcePat_::Struct(s, _)
        | SourcePat_::App(s, _, _)
        | SourcePat_::KeyValue(s, _, _)
        | SourcePat_::Elements(s, _)
        | SourcePat_::All(s, _)
        | SourcePat_::Wildcard(s)
        | SourcePat_::Variable(s, _)
        | SourcePat_::ElementsOfArray(s, _)
        | SourcePat_::OrPattern(s, _, _)
        | SourcePat_::IfPattern { span: s, .. }
        | SourcePat_::NestedQuery(s, _)
        | SourcePat_::Negation(s, _)
        | SourcePat_::FactId(s, _, _)
        | SourcePat_::TypeSignature(s, _, _)
        | SourcePat_::Never(s)
        | SourcePat_::FieldSelect(s, _, _, _)
        | SourcePat_::Deref(s, _)
        | SourcePat_::Enum(s, _)
        | SourcePat_::Clause(s, _, _, _, _)
        | SourcePat_::Prim(s, _, _) => s,
    }
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A Glean Type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Type_<ST, P, T> {
    // Native types
    ByteTy,
    NatTy,
    StringTy,
    ArrayTy(Box<Type_<ST, P, T>>),
    RecordTy(Vec<FieldDef_<ST, P, T>>),
    SumTy(Vec<FieldDef_<ST, P, T>>),
    SetTy(Box<Type_<ST, P, T>>),
    PredicateTy(ST, P),
    NamedTy(ST, T),

    // Derived types. These appear in the source language, but are
    // translated into the native types above internally. They also
    // act as hints to the code generators, e.g. maybe translates to
    // Thrift's optional and enum translates to Thrift's enum.
    /// maybe T  => { nothing | just : T }
    MaybeTy(Box<Type_<ST, P, T>>),
    /// enum { a | b } => { a : {}, b : {} }
    EnumeratedTy(Vec<Name>),
    /// bool => { false : {} | true : {} }
    BooleanTy,

    // These are used during typechecking only
    TyVar(i64),
    /// HasTy { field:type .. } R X
    ///   Constrains X to be a record or sum type containing at least
    ///   the given fields/types. X can only be instantiated
    ///   with a type containing a superset of those fields: either
    ///   a bigger HasTy or a RecordTy/SumTy.
    ///   R is
    ///     Just Record -> type must be a record
    ///     Just Sum -> type must be a sum type
    ///     Nothing -> type can be either a RecordTy or a SumTy
    HasTy(BTreeMap<Name, Type_<ST, P, T>>, Option<RecordOrSum>, i64),
    /// HasKey K X
    ///   A type variable X that is constrained to be a predicate
    ///   type with key K
    HasKey(Box<Type_<ST, P, T>>, i64),
    /// ElementsOf T X
    ///   A type variable that is either set T or [T]
    ElementsOf(Box<Type_<ST, P, T>>, i64),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordOrSum {
    Record,
    Sum,
}

/// `instance Binary RecordOrSum` (Types.hs:447) — generic-derived sum of
/// 2: Word8 tags 0/1.
impl Binary for RecordOrSum {
    fn encode(&self) -> Vec<u8> {
        match self {
            RecordOrSum::Record => put_ctor_tag(0, 2),
            RecordOrSum::Sum => put_ctor_tag(1, 2),
        }
    }
}

/// `instance (Binary pref, Binary tref) => Binary (Type_ () pref tref)`
/// (Types.hs:449) — generic-derived over a 16-constructor sum: Word8 tag
/// = 0-based declaration index, then fields in order. `TyVar`/`HasTy`/
/// `HasKey`/`ElementsOf` carry `Int` (putInt64be); `HasTy`'s Map
/// serializes ascending (BTreeMap).
impl<P: Binary, T: Binary> Binary for Type_<(), P, T> {
    fn encode(&self) -> Vec<u8> {
        const N: usize = 16;
        match self {
            Type_::ByteTy => put_ctor_tag(0, N),
            Type_::NatTy => put_ctor_tag(1, N),
            Type_::StringTy => put_ctor_tag(2, N),
            Type_::ArrayTy(ty) => enc_cat(put_ctor_tag(3, N), ty.encode()),
            Type_::RecordTy(xs) => enc_cat(put_ctor_tag(4, N), encode_list(xs)),
            Type_::SumTy(xs) => enc_cat(put_ctor_tag(5, N), encode_list(xs)),
            Type_::SetTy(ty) => enc_cat(put_ctor_tag(6, N), ty.encode()),
            Type_::PredicateTy(_, pref) => enc_cat(put_ctor_tag(7, N), pref.encode()),
            Type_::NamedTy(_, tref) => enc_cat(put_ctor_tag(8, N), tref.encode()),
            Type_::MaybeTy(ty) => enc_cat(put_ctor_tag(9, N), ty.encode()),
            Type_::EnumeratedTy(xs) => enc_cat(put_ctor_tag(10, N), encode_list(xs)),
            Type_::BooleanTy => put_ctor_tag(11, N),
            Type_::TyVar(x) => enc_cat(put_ctor_tag(12, N), x.encode()),
            Type_::HasTy(m, r, x) => enc_cat(
                put_ctor_tag(13, N),
                enc_cat(enc_cat(m.encode(), r.encode()), x.encode()),
            ),
            Type_::HasKey(ty, x) => enc_cat(put_ctor_tag(14, N), enc_cat(ty.encode(), x.encode())),
            Type_::ElementsOf(ty, x) => enc_cat(put_ctor_tag(15, N), enc_cat(ty.encode(), x.encode())),
        }
    }
}

// Note [Types] — from the source, verbatim in substance:
//
// There are three main instantiations of `Type_` used throughout
// Glean:
//
// > type SourceType = Type_ SourceRef SourceRef
//
// These are the types from the parser. Predicates and types may be
// unqualified and don't necessarily have versions.
//
// > type Type = Type_ PredicateId TypeId
//
// These are the types after name resolution
// (Glean.Schema.Resolve). Predicates and Types are fully qualified and
// refer to specific hashes.
//
// > type RTS.Type = Type_ PidRef ExpandedType
//
// (see 'Glean.RTS.Type')
//
// These are the types that relate to a particular DB, and are
// constructed when a schema is loaded in `Glean.Database.Schema`.
//
// * Predicates have a particular 'Pid'
// * Named types are expanded (but we can still see them in the AST)
// * A 'RTS.Type' can be converted into a pure representation ('Rep')
//   using 'repType'

// ---------------------------------------------------------------------------
// Schemas and definitions
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldDef_<ST, P, T> {
    pub field_def_name: Name,
    pub field_def_type: Type_<ST, P, T>,
}

/// `instance (Binary pref, Binary tref) => Binary (FieldDef_ () pref
/// tref)` (Types.hs:527-528) — single constructor: name then type.
impl<P: Binary, T: Binary> Binary for FieldDef_<(), P, T> {
    fn encode(&self) -> Vec<u8> {
        enc_cat(self.field_def_name.encode(), self.field_def_type.encode())
    }
}

/// `tupleField = "tuplefield"` (Types.hs:536-537).
pub const TUPLE_FIELD: &str = "tuplefield";

/// A definition of a named type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeDef_<ST, P, T> {
    pub type_def_ref: T,
    pub type_def_type: Type_<ST, P, T>,
    pub type_def_src_span: ST,
}

/// A definition of a predicate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PredicateDef_<S, ST, P, T> {
    pub predicate_def_ref: P,
    pub predicate_def_key_type: Type_<ST, P, T>,
    pub predicate_def_value_type: Type_<ST, P, T>,
    pub predicate_def_deriving: DerivingInfo<SourceQuery_<S, ST, P, T>>,
    pub predicate_def_src_span: S,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DerivingDef_<S, ST, P, T> {
    pub deriving_def_ref: P,
    pub deriving_def_derive_info: DerivingInfo<SourceQuery_<S, ST, P, T>>,
    pub deriving_def_src_span: S,
}

/// Globally unique identifier for a predicate. This is not the same
/// as a Pid, which is unique only within a particular DB.
///
// RULE §2 partial-functions/bug discipline note: the source has
// Eq ≠ Ord here — `Eq PredicateId` compares ONLY the hash
// (Types.hs:574-575) while `deriving Ord` compares
// (predicateIdRef, predicateIdHash) lexicographically (Types.hs:569).
// BUG(port): Eq ≠ Ord reproduced bug-for-bug below (RULE §2 BUG rule) — repro:
// PredicateId{ref:"a",hash:H} == PredicateId{ref:"b",hash:H} is true while
// Ord says Less; fix only as a flagged post-parity change. Equal values can
// order differently. No live repro in the tree depends on it; the
// HashMap-based environments key by PredicateRef or sort explicit
// lists (ComputeIds.hs:342 sorts before hashing, which is why hash
// order noise cannot reach the schema ID).
#[derive(Clone, Debug, PartialOrd, Ord)]
pub struct PredicateId {
    /// e.g. python.Name.1
    pub predicate_id_ref: PredicateRef,
    pub predicate_id_hash: Hash, // {-# UNPACK #-} !Hash
}

/// `instance Hashable PredicateId where hashWithSalt s (PredicateId _
/// h) = hashFingerprint s h` (Types.hs:571-572) — salts over the hash
/// field ONLY, consistent with the Eq above.
// TODO(port): hash_fingerprint is the sibling hash.rs's FNV stand-in
// for hashable-1.4.7's xxh3-style chain (its open TODO #1); this impl
// inherits that placeholder status. In-memory HashMap salt only —
// never persisted, never golden-printed (ComputeIds sorts before
// hashing); hashable-1.4.7.0: `hash = defaultHash = hashWithSalt defaultSalt`, defaultSalt = -3750763034362895579 (LowLevel.hs:41-51, Class.hs:254,329-330); the 0 here is the placeholder pending the real chain.
impl std::hash::Hash for PredicateId {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_i64(hash_fingerprint(0, &self.predicate_id_hash));
    }
}

/// `instance Eq PredicateId where PredicateId _ a == PredicateId _ b =
/// a == b` (Types.hs:574-575) — ref ignored.
impl PartialEq for PredicateId {
    fn eq(&self, other: &Self) -> bool {
        self.predicate_id_hash == other.predicate_id_hash
    }
}

impl Eq for PredicateId {}

/// `instance Binary PredicateRef where put (PredicateRef a b) = put a
/// >> put b` (Types.hs:577-579) — the source's HAND-WRITTEN pair over
/// the thrift type (name :: Text via the text-2.0.2 instance, then
/// version :: Version = Int32 via putInt32be). Byte-identical to what a
/// generic single-constructor derivation would emit; kept as its own
/// impl because the inventory row calls this pair out specifically.
impl Binary for PredicateRef {
    fn encode(&self) -> Vec<u8> {
        enc_cat(self.name.encode(), self.version.encode())
    }
}

/// `instance Binary PredicateId` (Types.hs:581-582, bare — generic
/// default; the source's "TODO maybe just serialize the hash?" was
/// never taken, so this serializes TypeRef+2-word Fingerprint):
/// single constructor => no tag; PredicateRef bytes then the
/// Fingerprint (two Word64be, fingerprint1 first — Class.hs:739).
impl Binary for PredicateId {
    fn encode(&self) -> Vec<u8> {
        enc_cat(self.predicate_id_ref.encode(), self.predicate_id_hash.encode())
    }
}

/// Globally unique identifier for a type.
#[derive(Clone, Debug, PartialOrd, Ord)]
pub struct TypeId {
    /// e.g. python.Declaration.1
    pub type_id_ref: TypeRef,
    pub type_id_hash: Hash,
}

/// `instance Hashable TypeId` (Types.hs:590-591) — hash field only.
// TODO(port): same placeholder status as PredicateId's Hash above.
impl std::hash::Hash for TypeId {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_i64(hash_fingerprint(0, &self.type_id_hash));
    }
}

/// `instance Eq TypeId` (Types.hs:593-594) — hash only.
impl PartialEq for TypeId {
    fn eq(&self, other: &Self) -> bool {
        self.type_id_hash == other.type_id_hash
    }
}

impl Eq for TypeId {}

/// `tempPredicateId` (Types.hs:597-598) — used by query compilation.
pub fn temp_predicate_id() -> PredicateId {
    PredicateId {
        predicate_id_ref: PredicateRef {
            name: "_tmp_".to_owned(),
            version: 0,
        },
        predicate_id_hash: hash0(),
    }
}

/// `instance Binary TypeRef where put (TypeRef a b) = put a >> put b`
/// (Types.hs:600-602) — hand-written pair, same shape as PredicateRef's.
impl Binary for TypeRef {
    fn encode(&self) -> Vec<u8> {
        enc_cat(self.name.encode(), self.version.encode())
    }
}

/// `instance Binary TypeId` (Types.hs:604-605, bare generic default;
/// the source's second "TODO maybe just serialize the hash?" also not
/// taken): TypeRef bytes + Fingerprint.
impl Binary for TypeId {
    fn encode(&self) -> Vec<u8> {
        enc_cat(self.type_id_ref.encode(), self.type_id_hash.encode())
    }
}

/// How to derive a predicate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DerivingInfo<Q> {
    NoDeriving,
    Derive(DeriveWhen, Q),
}

/// `instance Binary q => Binary (DerivingInfo q)` (Types.hs:613) —
/// generic-derived sum of 2: NoDeriving = tag 0 (no fields); Derive =
/// tag 1 then DeriveWhen then q.
impl<Q: Binary> Binary for DerivingInfo<Q> {
    fn encode(&self) -> Vec<u8> {
        match self {
            DerivingInfo::NoDeriving => put_ctor_tag(0, 2),
            DerivingInfo::Derive(when, q) => {
                enc_cat(put_ctor_tag(1, 2), enc_cat(when.encode(), q.encode()))
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeriveWhen {
    /// derived whenever this predicate is queried. Facts of this
    /// predicate are not expected to be stored in the DB.
    DeriveOnDemand,
    /// derived eagerly and stored in the DB. By default, queries for
    /// this predicate will search the DB, just like underived
    /// predicates.
    DerivedAndStored,
    /// derived only when the DB has no facts of this predicate.
    /// This is used to add derivations for backwards compatibility
    /// during a schema migration.
    DeriveIfEmpty,
}

/// `instance Binary DeriveWhen` (Types.hs:629) — generic-derived sum of
/// 3: Word8 tags 0/1/2.
impl Binary for DeriveWhen {
    fn encode(&self) -> Vec<u8> {
        match self {
            DeriveWhen::DeriveOnDemand => put_ctor_tag(0, 3),
            DeriveWhen::DerivedAndStored => put_ctor_tag(1, 3),
            DeriveWhen::DeriveIfEmpty => put_ctor_tag(2, 3),
        }
    }
}

// Source (parsed) abstract syntax — the primed alias family
// (`SourcePat'` etc. in Haskell; Rust identifiers cannot contain `'`,
// so the primed aliases carry a `WithRefs`/`WithIds` suffix naming the
// ref instantiation they fix):
pub type SourcePatWithRefs<S, ST> = SourcePat_<S, ST, SourceRef, SourceRef>;
pub type SourceStatementWithRefs<S, ST> = SourceStatement_<S, ST, SourceRef, SourceRef>;
pub type SourceQueryWithRefs<S, ST> = SourceQuery_<S, ST, SourceRef, SourceRef>;
pub type SourceDerivingInfoWithRefs<S> = DerivingInfo<SourceQueryWithRefs<S, S>>;
pub type SourcePredicateDefWithRefs<S> = PredicateDef_<S, S, SourceRef, SourceRef>;
pub type SourceTypeDefWithRefs<S> = TypeDef_<S, SourceRef, SourceRef>;
pub type DerivingDefWithRefs<S> = DerivingDef_<S, S, SourceRef, SourceRef>;
pub type SourceTypeWithRefs<ST> = Type_<ST, SourceRef, SourceRef>;
pub type TypeWithIds<ST> = Type_<ST, PredicateId, TypeId>;

pub type SourcePat = SourcePatWithRefs<SrcSpan, SrcSpan>;
pub type SourceStatement = SourceStatementWithRefs<SrcSpan, SrcSpan>;
pub type SourceQuery = SourceQueryWithRefs<SrcSpan, SrcSpan>;
pub type SourceType = Type_<SrcSpan, SourceRef, SourceRef>;
pub type SourceFieldDef = FieldDef_<SrcSpan, SourceRef, SourceRef>;
pub type SourceTypeDef = TypeDef_<SrcSpan, SourceRef, SourceRef>;
pub type SourcePredicateDef = SourcePredicateDefWithRefs<SrcSpan>;
pub type SourceDerivingInfo = SourceDerivingInfoWithRefs<SrcSpan>;
pub type SourceSchemas = SourceSchemas_<SrcSpan>;
pub type SourceSchema = SourceSchema_<SrcSpan>;
pub type SourceEvolves = SourceEvolves_<SrcSpan>;
pub type SourceDecl = SourceDecl_<SrcSpan>;

pub type Statement_<P, T> = SourceStatement_<SrcSpan, SrcSpan, P, T>;
pub type Query_<P, T> = SourceQuery_<SrcSpan, SrcSpan, P, T>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceEvolves_<S> {
    pub evolves_span: S,
    pub evolves_new: SourceRef,
    pub evolves_old: SourceRef,
}

/// A 'schema' declaration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceSchema_<S> {
    pub schema_name: SourceRef,
    pub schema_inherits: Vec<SourceRef>,
    pub schema_decls: Vec<SourceDecl_<S>>,
    pub schema_src_span: S,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceSchemas_<S> {
    pub src_angle_version: AngleVersion,
    pub src_schemas: Vec<SourceSchema_<S>>,
    pub src_evolves: Vec<SourceEvolves_<S>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceDecl_<S> {
    SourceImport(SourceRef, S),
    SourcePredicate(SourcePredicateDefWithRefs<S>),
    SourceType(SourceTypeDefWithRefs<S>),
    SourceDeriving(DerivingDefWithRefs<S>),
}

// Abstract syntax with global Ids:
pub type Type = Type_<SrcSpan, PredicateId, TypeId>;
pub type FieldDef = FieldDef_<SrcSpan, PredicateId, TypeId>;
pub type TypeDef = TypeDef_<SrcSpan, PredicateId, TypeId>;
pub type PredicateDef = PredicateDef_<SrcSpan, SrcSpan, PredicateId, TypeId>;

pub type PredicateDefWithIds<S, ST> = PredicateDef_<S, ST, PredicateId, TypeId>;
pub type FieldDefWithIds<ST> = FieldDef_<ST, PredicateId, TypeId>;

// ---------------------------------------------------------------------------
// Versions of the Angle syntax
// ---------------------------------------------------------------------------

/// Version of the syntax. This is required so that we can change the
/// syntax while still allowing DBs that contain a schema with the old
/// syntax to be understood. (Source docs.)
///
/// `newtype AngleVersion = AngleVersion Int deriving (Eq, Ord);
/// deriving newtype (Pretty)` — Pretty via the Int instance renders
/// decimal (prettyprinter's numeric instances are decimal), folded into
/// this crate's Display trait below (see the Display impl).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct AngleVersion(pub i64);

// version 8:
//   - added set constructs
// version 9:
//   - added dot syntax
// version 10:
//   - optional versions in `import`, `evolves`, inheritance
// version 11:
//   - size primop
// version 12:
//   - remove support for deriving default

/// `latestSupportedAngleVersion = AngleVersion 5` (Types.hs:719-720).
pub const fn latest_supported_angle_version() -> AngleVersion {
    AngleVersion(5)
}

/// `latestAngleVersion = AngleVersion 12` (Types.hs:722-723).
pub const fn latest_angle_version() -> AngleVersion {
    AngleVersion(12)
}

// ---------------------------------------------------------------------------
// Bifunctor / Bifoldable — the full-tree walks
// ---------------------------------------------------------------------------
//
// RULE §2 (bimap/bifoldMap row, B6): walk the FULL tree — predicates
// nested in Maybe/Array/Set/Record/Sum evolve too; a shallow copy is an
// infidelity (round-2 D2 lesson). These walks are the port of the
// source's Bifunctor/Bifoldable instances (Types.hs:189-207, 270-342,
// 451-534), case for case. The Haskell ALSO derives Functor (fmap over
// the last param `t`); in Rust that is `bimap(&mut |p| p, &mut g)` —
// one closure surface instead of two classes.
//
// Consumed by the sibling units as the refsToIds engine
// (ComputeIds.hs:76-89: `bimap (lookupPredicateId pids)
// (lookupTypeId tids)` over any `f PredicateRef TypeRef`) and by
// Query/Typecheck for SourceRef-vs-resolved evolution.

impl<S, ST, P, T> SourceQuery_<S, ST, P, T> {
    /// Bifunctor (SourceQuery_ s st) — Types.hs:189-191.
    pub fn bimap<P2, T2, F, G>(self, f: &mut F, g: &mut G) -> SourceQuery_<S, ST, P2, T2>
    where
        F: FnMut(P) -> P2,
        G: FnMut(T) -> T2,
    {
        SourceQuery_ {
            src_query_head: self.src_query_head.map(|h| h.bimap(f, g)),
            src_query_stmts: self
                .src_query_stmts
                .into_iter()
                .map(|s| s.bimap(f, g))
                .collect(),
            src_query_ordered: self.src_query_ordered,
        }
    }

    /// Bifoldable (SourceQuery_ s st) — Types.hs:193-195.
    pub fn bifold_map<A, F, G>(&self, acc: &mut A, f: &mut F, g: &mut G)
    where
        F: FnMut(&P, &mut A),
        G: FnMut(&T, &mut A),
    {
        if let Some(h) = &self.src_query_head {
            h.bifold_map(acc, f, g);
        }
        for s in &self.src_query_stmts {
            s.bifold_map(acc, f, g);
        }
    }
}

impl<S, ST, P, T> SourceStatement_<S, ST, P, T> {
    /// Bifunctor — Types.hs:203-204.
    pub fn bimap<P2, T2, F, G>(self, f: &mut F, g: &mut G) -> SourceStatement_<S, ST, P2, T2>
    where
        F: FnMut(P) -> P2,
        G: FnMut(T) -> T2,
    {
        SourceStatement_(self.0.bimap(f, g), self.1.bimap(f, g))
    }

    /// Bifoldable — Types.hs:206-207.
    pub fn bifold_map<A, F, G>(&self, acc: &mut A, f: &mut F, g: &mut G)
    where
        F: FnMut(&P, &mut A),
        G: FnMut(&T, &mut A),
    {
        self.0.bifold_map(acc, f, g);
        self.1.bifold_map(acc, f, g);
    }
}

impl<S, ST, P, T> SourcePat_<S, ST, P, T> {
    /// Bifunctor (SourcePat_ s st) — Types.hs:270-300, full tree: the
    /// walk reaches predicates nested in Array/ArrayPrefix/Tuple/
    /// Struct/App/KeyValue/All/OrPattern/NestedQuery/TypeSignature/
    /// IfPattern/FieldSelect/Deref/Clause/Prim (B6).
    pub fn bimap<P2, T2, F, G>(self, f: &mut F, g: &mut G) -> SourcePat_<S, ST, P2, T2>
    where
        F: FnMut(P) -> P2,
        G: FnMut(T) -> T2,
    {
        fn vec<S, ST, P, T, P2, T2, F, G>(
            xs: Vec<SourcePat_<S, ST, P, T>>,
            f: &mut F,
            g: &mut G,
        ) -> Vec<SourcePat_<S, ST, P2, T2>>
        where
            F: FnMut(P) -> P2,
            G: FnMut(T) -> T2,
        {
            xs.into_iter().map(|x| x.bimap(f, g)).collect()
        }
        fn fields<S, ST, P, T, P2, T2, F, G>(
            xs: Vec<Field<S, ST, P, T>>,
            f: &mut F,
            g: &mut G,
        ) -> Vec<Field<S, ST, P2, T2>>
        where
            F: FnMut(P) -> P2,
            G: FnMut(T) -> T2,
        {
            xs.into_iter().map(|x| x.bimap(f, g)).collect()
        }
        match self {
            SourcePat_::Nat(s, w) => SourcePat_::Nat(s, w),
            SourcePat_::String(s, t) => SourcePat_::String(s, t),
            SourcePat_::StringPrefix(s, t) => SourcePat_::StringPrefix(s, t),
            SourcePat_::ByteArray(s, t) => SourcePat_::ByteArray(s, t),
            SourcePat_::Array(s, pats) => SourcePat_::Array(s, vec(pats, f, g)),
            SourcePat_::ArrayPrefix(s, pats) => SourcePat_::ArrayPrefix(
                s,
                NonEmpty {
                    head: pats.head.bimap(f, g),
                    tail: vec(pats.tail, f, g),
                },
            ),
            SourcePat_::Tuple(s, pats) => SourcePat_::Tuple(s, vec(pats, f, g)),
            SourcePat_::Struct(s, fs) => SourcePat_::Struct(s, fields(fs, f, g)),
            SourcePat_::App(s, l, r) => SourcePat_::App(s, Box::new(l.bimap(f, g)), vec(r, f, g)),
            SourcePat_::KeyValue(s, k, v) => {
                SourcePat_::KeyValue(s, Box::new(k.bimap(f, g)), Box::new(v.bimap(f, g)))
            }
            SourcePat_::Elements(s, pat) => SourcePat_::Elements(s, Box::new(pat.bimap(f, g))),
            SourcePat_::All(s, query) => SourcePat_::All(s, Box::new(query.bimap(f, g))),
            SourcePat_::Wildcard(s) => SourcePat_::Wildcard(s),
            SourcePat_::Variable(s, n) => SourcePat_::Variable(s, n),
            SourcePat_::ElementsOfArray(s, pat) => {
                SourcePat_::ElementsOfArray(s, Box::new(pat.bimap(f, g)))
            }
            SourcePat_::OrPattern(s, l, r) => SourcePat_::OrPattern(
                s,
                Box::new(l.bimap(f, g)),
                Box::new(r.bimap(f, g)),
            ),
            SourcePat_::NestedQuery(s, q) => SourcePat_::NestedQuery(s, Box::new(q.bimap(f, g))),
            SourcePat_::Negation(s, pat) => SourcePat_::Negation(s, Box::new(pat.bimap(f, g))),
            SourcePat_::FactId(s, n, w) => SourcePat_::FactId(s, n, w),
            SourcePat_::TypeSignature(s, pat, ty) => {
                SourcePat_::TypeSignature(s, Box::new(pat.bimap(f, g)), Box::new(ty.bimap(f, g)))
            }
            SourcePat_::Never(s) => SourcePat_::Never(s),
            SourcePat_::IfPattern { span, cond, then_, else_ } => SourcePat_::IfPattern {
                span,
                cond: Box::new(cond.bimap(f, g)),
                then_: Box::new(then_.bimap(f, g)),
                else_: Box::new(else_.bimap(f, g)),
            },
            SourcePat_::FieldSelect(s, pat, field, q) => SourcePat_::FieldSelect(
                s,
                Box::new(pat.bimap(f, g)),
                field,
                q,
            ),
            SourcePat_::Deref(s, pat) => SourcePat_::Deref(s, Box::new(pat.bimap(f, g))),
            SourcePat_::Enum(s, fl) => SourcePat_::Enum(s, fl),
            SourcePat_::Clause(s_pat, s_pred, p, pat, rng) => SourcePat_::Clause(
                s_pat,
                s_pred,
                f(p),
                Box::new(pat.bimap(f, g)),
                rng,
            ),
            SourcePat_::Prim(s, p, pats) => SourcePat_::Prim(s, p, vec(pats, f, g)),
        }
    }

    /// Bifoldable (SourcePat_ s st) — Types.hs:302-331, left-to-right
    /// visit order matching the source's `<>` chain. Leaves without
    /// refs contribute nothing.
    pub fn bifold_map<A, F, G>(&self, acc: &mut A, f: &mut F, g: &mut G)
    where
        F: FnMut(&P, &mut A),
        G: FnMut(&T, &mut A),
    {
        match self {
            SourcePat_::Nat(..)
            | SourcePat_::String(..)
            | SourcePat_::StringPrefix(..)
            | SourcePat_::ByteArray(..)
            | SourcePat_::Wildcard(..)
            | SourcePat_::Variable(..)
            | SourcePat_::FactId(..)
            | SourcePat_::Never(..)
            | SourcePat_::Enum(..) => {}
            SourcePat_::Array(_, pats) | SourcePat_::ArrayPrefix(_, pats) => {
                for p in pats.iter() {
                    p.bifold_map(acc, f, g);
                }
            }
            SourcePat_::Tuple(_, pats) => {
                for p in pats {
                    p.bifold_map(acc, f, g);
                }
            }
            SourcePat_::Struct(_, fields) => {
                for fd in fields {
                    fd.bifold_map(acc, f, g);
                }
            }
            SourcePat_::App(_, l, r) => {
                l.bifold_map(acc, f, g);
                for p in r {
                    p.bifold_map(acc, f, g);
                }
            }
            SourcePat_::KeyValue(_, k, v) => {
                k.bifold_map(acc, f, g);
                v.bifold_map(acc, f, g);
            }
            SourcePat_::Elements(_, pat) | SourcePat_::ElementsOfArray(_, pat) => {
                pat.bifold_map(acc, f, g)
            }
            SourcePat_::All(_, query) => query.bifold_map(acc, f, g),
            SourcePat_::OrPattern(_, l, r) => {
                l.bifold_map(acc, f, g);
                r.bifold_map(acc, f, g);
            }
            SourcePat_::NestedQuery(_, q) => q.bifold_map(acc, f, g),
            SourcePat_::Negation(_, pat) => pat.bifold_map(acc, f, g),
            SourcePat_::TypeSignature(_, pat, ty) => {
                pat.bifold_map(acc, f, g);
                ty.bifold_map(acc, f, g);
            }
            SourcePat_::IfPattern { cond, then_, else_, .. } => {
                cond.bifold_map(acc, f, g);
                then_.bifold_map(acc, f, g);
                else_.bifold_map(acc, f, g);
            }
            SourcePat_::FieldSelect(_, pat, _, _) => pat.bifold_map(acc, f, g),
            SourcePat_::Deref(_, pat) => pat.bifold_map(acc, f, g),
            SourcePat_::Clause(_, _, p, pat, _) => {
                f(p, acc);
                pat.bifold_map(acc, f, g);
            }
            SourcePat_::Prim(_, _, pats) => {
                for p in pats {
                    p.bifold_map(acc, f, g);
                }
            }
        }
    }
}

impl<S, ST, P, T> Field<S, ST, P, T> {
    /// Bifunctor (Field s st) — Types.hs:338-339.
    pub fn bimap<P2, T2, F, G>(self, f: &mut F, g: &mut G) -> Field<S, ST, P2, T2>
    where
        F: FnMut(P) -> P2,
        G: FnMut(T) -> T2,
    {
        Field(self.0, self.1.bimap(f, g))
    }

    /// Bifoldable (Field s st) — Types.hs:341-342.
    pub fn bifold_map<A, F, G>(&self, acc: &mut A, f: &mut F, g: &mut G)
    where
        F: FnMut(&P, &mut A),
        G: FnMut(&T, &mut A),
    {
        self.1.bifold_map(acc, f, g);
    }
}

impl<ST, P, T> Type_<ST, P, T> {
    /// Bifunctor (Type_ s) — Types.hs:451-468, full tree (predicates in
    /// nested Array/Record/Sum/Set/Maybe/HasTy/HasKey/ElementsOf all
    /// evolve — B6).
    pub fn bimap<P2, T2, F, G>(self, f: &mut F, g: &mut G) -> Type_<ST, P2, T2>
    where
        F: FnMut(P) -> P2,
        G: FnMut(T) -> T2,
    {
        match self {
            Type_::ByteTy => Type_::ByteTy,
            Type_::NatTy => Type_::NatTy,
            Type_::StringTy => Type_::StringTy,
            Type_::ArrayTy(ty) => Type_::ArrayTy(Box::new(ty.bimap(f, g))),
            Type_::RecordTy(xs) => {
                Type_::RecordTy(xs.into_iter().map(|x| x.bimap(f, g)).collect())
            }
            Type_::SumTy(xs) => Type_::SumTy(xs.into_iter().map(|x| x.bimap(f, g)).collect()),
            Type_::SetTy(ty) => Type_::SetTy(Box::new(ty.bimap(f, g))),
            Type_::PredicateTy(s, pref) => Type_::PredicateTy(s, f(pref)),
            Type_::NamedTy(s, tref) => Type_::NamedTy(s, g(tref)),
            Type_::MaybeTy(ty) => Type_::MaybeTy(Box::new(ty.bimap(f, g))),
            Type_::EnumeratedTy(xs) => Type_::EnumeratedTy(xs),
            Type_::BooleanTy => Type_::BooleanTy,
            Type_::TyVar(x) => Type_::TyVar(x),
            Type_::HasTy(m, r, x) => Type_::HasTy(
                m.into_iter().map(|(k, v)| (k, v.bimap(f, g))).collect(),
                r,
                x,
            ),
            Type_::HasKey(ty, x) => Type_::HasKey(Box::new(ty.bimap(f, g)), x),
            Type_::ElementsOf(ty, x) => Type_::ElementsOf(Box::new(ty.bimap(f, g)), x),
        }
    }

    /// Bifoldable (Type_ s) — Types.hs:470-487. The Haskell instance is
    /// `bifoldr`; `bifoldMap` over it still yields elements in
    /// left-to-right order (foldr preserves element order in the
    /// Monoid chain), which is what this walk does.
    pub fn bifold_map<A, F, G>(&self, acc: &mut A, f: &mut F, g: &mut G)
    where
        F: FnMut(&P, &mut A),
        G: FnMut(&T, &mut A),
    {
        match self {
            Type_::ByteTy | Type_::NatTy | Type_::StringTy | Type_::BooleanTy | Type_::TyVar(_) => {}
            Type_::EnumeratedTy(_) => {}
            Type_::ArrayTy(ty) | Type_::SetTy(ty) | Type_::MaybeTy(ty) => ty.bifold_map(acc, f, g),
            Type_::RecordTy(xs) | Type_::SumTy(xs) => {
                for x in xs {
                    x.bifold_map(acc, f, g);
                }
            }
            Type_::PredicateTy(_, pref) => f(pref, acc),
            Type_::NamedTy(_, tref) => g(tref, acc),
            Type_::HasTy(m, _, _) => {
                for v in m.values() {
                    v.bifold_map(acc, f, g);
                }
            }
            Type_::HasKey(ty, _) | Type_::ElementsOf(ty, _) => ty.bifold_map(acc, f, g),
        }
    }
}

impl<ST, P, T> FieldDef_<ST, P, T> {
    /// Bifunctor (FieldDef_ st) — Types.hs:530-531.
    pub fn bimap<P2, T2, F, G>(self, f: &mut F, g: &mut G) -> FieldDef_<ST, P2, T2>
    where
        F: FnMut(P) -> P2,
        G: FnMut(T) -> T2,
    {
        FieldDef_ {
            field_def_name: self.field_def_name,
            field_def_type: self.field_def_type.bimap(f, g),
        }
    }

    /// Bifoldable (FieldDef_ st) — Types.hs:533-534.
    pub fn bifold_map<A, F, G>(&self, acc: &mut A, f: &mut F, g: &mut G)
    where
        F: FnMut(&P, &mut A),
        G: FnMut(&T, &mut A),
    {
        self.field_def_type.bifold_map(acc, f, g);
    }
}

impl<Q> DerivingInfo<Q> {
    /// Functor (DerivingInfo) — `deriving Functor` (Types.hs:611).
    pub fn map<Q2, F>(self, f: &mut F) -> DerivingInfo<Q2>
    where
        F: FnMut(Q) -> Q2,
    {
        match self {
            DerivingInfo::NoDeriving => DerivingInfo::NoDeriving,
            DerivingInfo::Derive(when, q) => DerivingInfo::Derive(when, f(q)),
        }
    }

    /// Foldable (DerivingInfo) — the traversal part of `deriving
    /// Foldable` (used by consumers as toList/foldMap over the query).
    pub fn fold_map<A, F>(&self, acc: &mut A, f: &mut F)
    where
        F: FnMut(&Q, &mut A),
    {
        if let DerivingInfo::Derive(_, q) = self {
            f(q, acc);
        }
    }

    pub fn as_ref(&self) -> Option<&Q> {
        match self {
            DerivingInfo::NoDeriving => None,
            DerivingInfo::Derive(_, q) => Some(q),
        }
    }
}

// ---------------------------------------------------------------------------
// Pretty-printing
// ---------------------------------------------------------------------------

/// `instance Display SourceRef` (Types.hs:728-731).
impl Display for SourceRef {
    fn display(&self, _opts: &DisplayOpts) -> Doc {
        let mut d = Doc::text(self.source_ref_name.clone());
        if let Some(ver) = self.source_ref_version {
            d = d
                .append(Doc::text("."))
                .append(Doc::text(ver.to_string()));
        }
        d
    }
}

/// `instance Display PredicateRef where display _ = pretty`
/// (Types.hs:733-734). The `pretty` is the thrift-side instance from
/// glean/if/glean_include.hs:13-14 — `pretty (PredicateRef n v) =
/// pretty n <> "." <> pretty v` — which this impl renders directly (in
/// the Rust port the whole orphan chain lands in this one impl).
impl Display for PredicateRef {
    fn display(&self, _opts: &DisplayOpts) -> Doc {
        Doc::text(self.name.clone())
            .append(Doc::text("."))
            .append(Doc::text(self.version.to_string()))
    }
}

/// `instance Display TypeRef` (Types.hs:736-737) via
/// glean_include.hs:16-17.
impl Display for TypeRef {
    fn display(&self, _opts: &DisplayOpts) -> Doc {
        Doc::text(self.name.clone())
            .append(Doc::text("."))
            .append(Doc::text(self.version.to_string()))
    }
}

/// `instance Display PredicateId` (Types.hs:739-743).
impl Display for PredicateId {
    fn display(&self, opts: &DisplayOpts) -> Doc {
        match opts.predicate_style {
            PredicateStyle::PredicateWithHash => self
                .predicate_id_ref
                .display(opts)
                .append(Doc::text("."))
                // `pretty (show hash)`: Show of GHC.Fingerprint is the
                // 32-lowercase-hex rendering — exactly the sibling
                // hash.rs's std fmt::Display for Fingerprint.
                .append(Doc::text(self.predicate_id_hash.to_string())),
            PredicateStyle::PredicateWithoutHash => self.predicate_id_ref.display(opts),
        }
    }
}

/// `instance Display TypeId` (Types.hs:745-748).
impl Display for TypeId {
    fn display(&self, opts: &DisplayOpts) -> Doc {
        match opts.predicate_style {
            PredicateStyle::PredicateWithHash => self
                .type_id_ref
                .display(opts)
                .append(Doc::text("."))
                .append(Doc::text(self.type_id_hash.to_string())),
            PredicateStyle::PredicateWithoutHash => self.type_id_ref.display(opts),
        }
    }
}

/// `instance Display (SourceEvolves_ s)` (Types.hs:751-753).
impl<S> Display for SourceEvolves_<S> {
    fn display(&self, opts: &DisplayOpts) -> Doc {
        Doc::text("schema ")
            .append(self.evolves_new.display(opts))
            .append(Doc::text(" evolves "))
            .append(self.evolves_old.display(opts))
    }
}

/// `Data.List.intersperse` — used by the HasTy Sum branch below
/// (Types.hs:781); distinct from prettyprinter's `punctuate`.
fn intersperse(sep: &str, docs: Vec<Doc>) -> Vec<Doc> {
    let mut out = Vec::with_capacity(docs.len());
    let mut iter = docs.into_iter();
    if let Some(first) = iter.next() {
        out.push(first);
        for d in iter {
            out.push(Doc::text(sep));
            out.push(d);
        }
    }
    out
}

/// `instance (Display pref, Display tref) => Display (Type_ st pref
/// tref)` (Types.hs:755-797) — the span param is never rendered, hence
/// no Display bound on ST.
impl<ST, P: Display, T: Display> Display for Type_<ST, P, T> {
    fn display(&self, opts: &DisplayOpts) -> Doc {
        match self {
            Type_::ByteTy => Doc::text("byte"),
            Type_::NatTy => Doc::text("nat"),
            Type_::StringTy => Doc::text("string"),
            Type_::ArrayTy(ty) => brackets(ty.display(opts)),
            Type_::RecordTy(fields) => sep(vec![
                nest(
                    2,
                    vsep(
                        Some(Doc::text("{"))
                            .into_iter()
                            .chain(punctuate(Doc::text(","), fields.iter().map(|f| f.display(opts)).collect())),
                    ),
                ),
                Doc::text("}"),
            ]),
            Type_::SumTy(fields) => sep(vec![
                nest(
                    2,
                    vsep(
                        Some(Doc::text("{"))
                            .into_iter()
                            .chain(fields.iter().map(|f| f.display(opts).append(Doc::text(" |")))),
                    ),
                ),
                Doc::text("}"),
            ]),
            Type_::SetTy(ty) => Doc::text("set ").append(ty.display_atom(opts)),
            Type_::PredicateTy(_, p) => p.display(opts),
            Type_::NamedTy(_, t) => t.display(opts),
            Type_::MaybeTy(t) => Doc::text("maybe").append_space(t.display_atom(opts)),
            Type_::EnumeratedTy(names) => sep(vec![
                nest(
                    2,
                    vsep(Some(Doc::text("enum {")).into_iter().chain(
                        names
                            .iter()
                            .map(|n| Doc::text(n.clone()).append(Doc::text(" |"))),
                    )),
                ),
                Doc::text("}"),
            ]),
            Type_::BooleanTy => Doc::text("bool"),
            Type_::TyVar(n) => Doc::text("T").append(Doc::text(n.to_string())),
            Type_::HasTy(m, rec, x) => {
                // doField (n, ty) = pretty n <> " : " <> display opts ty
                let do_field = |(n, ty): (&Name, &Type_<ST, P, T>)| {
                    Doc::text(n.clone())
                        .append(Doc::text(" : "))
                        .append(ty.display(opts))
                };
                // Map.toList — ascending; BTreeMap iter matches.
                let items: Vec<Doc> = m.iter().map(do_field).collect();
                if matches!(rec, Some(RecordOrSum::Sum)) {
                    sep(vec![
                        nest(
                            2,
                            vsep(
                                Some(Doc::text("{"))
                                    .into_iter()
                                    .chain(intersperse("|", items)),
                            ),
                        ),
                        Doc::text("|"),
                        Doc::text("T").append(Doc::text(x.to_string())),
                        Doc::text("}"),
                    ])
                } else {
                    sep(vec![
                        nest(
                            2,
                            vsep(Some(Doc::text("{")).into_iter().chain(
                                items.into_iter().map(|d| d.append(Doc::text(","))),
                            )),
                        ),
                        Doc::text("T").append(Doc::text(x.to_string())),
                        Doc::text("}"),
                    ])
                }
            }
            Type_::HasKey(ty, x) => Doc::text("T")
                .append(Doc::text(x.to_string()))
                .append(parens(ty.display(opts))),
            Type_::ElementsOf(ty, x) => Doc::text("T")
                .append(Doc::text(x.to_string()))
                .append(brackets(ty.display(opts))),
        }
    }

    /// `displayAtom` override (Types.hs:793-797).
    fn display_atom(&self, opts: &DisplayOpts) -> Doc {
        match self {
            Type_::MaybeTy(_) | Type_::EnumeratedTy(_) | Type_::SetTy(_) => {
                parens(self.display(opts))
            }
            _other => self.display(opts),
        }
    }
}

/// `instance (Display pref, Display tref) => Display (FieldDef_ st pref
/// tref)` (Types.hs:799-800).
impl<ST, P: Display, T: Display> Display for FieldDef_<ST, P, T> {
    fn display(&self, opts: &DisplayOpts) -> Doc {
        Doc::text(self.field_def_name.clone())
            .append(Doc::text(" : "))
            .append(self.field_def_type.display_atom(opts))
    }
}

/// `instance (Display pref, Display tref) => Display (PredicateDef_ s
/// st pref tref)` (Types.hs:802-815).
impl<S, ST, P: Display, T: Display> Display for PredicateDef_<S, ST, P, T> {
    fn display(&self, opts: &DisplayOpts) -> Doc {
        let mut parts = vec![
            Doc::text("predicate")
                .append_space(self.predicate_def_ref.display(opts))
                .append_space(Doc::text(":")),
            self.predicate_def_key_type.display(opts),
        ];
        // case predicateDefValueType of RecordTy [] -> [] ; _ -> ...
        match &self.predicate_def_value_type {
            Type_::RecordTy(fields) if fields.is_empty() => {}
            _other => parts.push(
                Doc::text("->").append_space(self.predicate_def_value_type.display(opts)),
            ),
        }
        match &self.predicate_def_deriving {
            DerivingInfo::Derive(DeriveWhen::DerivedAndStored, query) => {
                parts.push(Doc::text("stored"));
                parts.push(query.display(opts));
            }
            DerivingInfo::Derive(_, query) => parts.push(query.display(opts)),
            DerivingInfo::NoDeriving => {}
        }
        hang(2, sep(parts))
    }
}

/// `instance (Display pref, Display tref) => Display (TypeDef_ s pref
/// tref)` (Types.hs:817-822).
impl<ST, P: Display, T: Display> Display for TypeDef_<ST, P, T> {
    fn display(&self, opts: &DisplayOpts) -> Doc {
        hang(
            2,
            sep(vec![
                Doc::text("type")
                    .append_space(self.type_def_ref.display(opts))
                    .append_space(Doc::text("=")),
                self.type_def_type.display(opts),
            ]),
        )
    }
}

/// `instance Display SourceSchemas` (Types.hs:824-827).
impl<S> Display for SourceSchemas_<S>
where
    SourceSchema_<S>: Display,
    SourceEvolves_<S>: Display,
{
    fn display(&self, opts: &DisplayOpts) -> Doc {
        vcat(
            Some(Doc::text("version:").append_space(
                Doc::text(self.src_angle_version.0.to_string()),
            ))
            .into_iter()
            .chain(self.src_schemas.iter().map(|s| s.display(opts)))
            .chain(self.src_evolves.iter().map(|e| e.display(opts))),
        )
    }
}

/// `instance Display SourceSchema` (Types.hs:829-838).
impl<S> Display for SourceSchema_<S>
where
    SourceDecl_<S>: Display,
{
    fn display(&self, opts: &DisplayOpts) -> Doc {
        let inherits = if self.schema_inherits.is_empty() {
            Doc::empty()
        } else {
            Doc::text(" : ").append(hcat(punctuate(
                Doc::text(","),
                self.schema_inherits.iter().map(|i| i.display(opts)).collect(),
            )))
        };
        vcat(vec![
            Doc::text("schema")
                .append_space(self.schema_name.display(opts))
                .append(inherits)
                .append(Doc::text(" {")),
            vcat(self.schema_decls.iter().map(|d| d.display(opts))),
            Doc::text("}"),
        ])
    }
}

/// `instance Display SourceDecl` (Types.hs:840-846).
impl<S> Display for SourceDecl_<S>
where
    SourcePredicateDefWithRefs<S>: Display,
    SourceTypeDefWithRefs<S>: Display,
{
    fn display(&self, opts: &DisplayOpts) -> Doc {
        match self {
            SourceDecl_::SourceImport(name, _) => {
                Doc::text("import ").append(name.display(opts))
            }
            SourceDecl_::SourcePredicate(def) => def.display(opts),
            SourceDecl_::SourceType(def) => def.display(opts),
            SourceDecl_::SourceDeriving(def) => hang(
                2,
                sep(vec![
                    Doc::text("derive ").append(def.deriving_def_ref.display(opts)),
                    def.deriving_def_derive_info.display(opts),
                ]),
            ),
        }
    }
}

/// `instance Display q => Display (DerivingInfo q)` (Types.hs:848-852).
impl<Q: Display> Display for DerivingInfo<Q> {
    fn display(&self, opts: &DisplayOpts) -> Doc {
        match self {
            DerivingInfo::NoDeriving => Doc::empty(),
            DerivingInfo::Derive(DeriveWhen::DeriveOnDemand, q) => q.display(opts),
            DerivingInfo::Derive(DeriveWhen::DerivedAndStored, q) => {
                Doc::text("stored").append_space(q.display(opts))
            }
            DerivingInfo::Derive(DeriveWhen::DeriveIfEmpty, q) => {
                Doc::text("default").append_space(q.display(opts))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Display printing queries
// ---------------------------------------------------------------------------

/// `class IsWild pat where isWild :: pat -> Bool` (Types.hs:857-859).
pub trait IsWild {
    fn is_wild(&self) -> bool;
}

/// `instance IsWild (SourcePat_ s st p t)` (Types.hs:860-862).
impl<S, ST, P, T> IsWild for SourcePat_<S, ST, P, T> {
    fn is_wild(&self) -> bool {
        matches!(self, SourcePat_::Wildcard(_))
    }
}

/// `class (Pretty a, Pretty (Loc a)) => IsSrcSpan a` (Types.hs:865-869).
/// The Pretty superclass maps onto this crate's Display trait (the
/// port's single pretty-printing class — see display.rs's trait note);
/// `Loc` is the associated type.
pub trait IsSrcSpan: Display {
    type Loc: Display;

    fn start_loc(&self) -> Self::Loc;
    fn end_loc(&self) -> Self::Loc;
    fn mk_span(start: Self::Loc, end: Self::Loc) -> Self;
}

/// space encompassing from start of first span to end of second.
/// `spanBetween` (Types.hs:872-873).
pub fn span_between<T: IsSrcSpan>(x: &T, y: &T) -> T {
    T::mk_span(x.start_loc(), y.end_loc())
}

/// `instance IsSrcSpan SrcSpan` (Types.hs:875-879).
impl IsSrcSpan for SrcSpan {
    type Loc = SrcLoc;

    fn start_loc(&self) -> SrcLoc {
        self.span_start
    }

    fn end_loc(&self) -> SrcLoc {
        self.span_end
    }

    fn mk_span(start: SrcLoc, end: SrcLoc) -> SrcSpan {
        SrcSpan {
            span_start: start,
            span_end: end,
        }
    }
}

/// `instance Display SrcSpan` (Types.hs:881-885).
impl Display for SrcSpan {
    fn display(&self, opts: &DisplayOpts) -> Doc {
        self.span_start
            .display(opts)
            .append(Doc::text(" - "))
            .append(self.span_end.display(opts))
    }
}

/// `instance Display SrcLoc` (Types.hs:890-892).
impl Display for SrcLoc {
    fn display(&self, _opts: &DisplayOpts) -> Doc {
        Doc::text("line ")
            .append(Doc::text(self.loc_line.to_string()))
            .append(Doc::text(", column "))
            .append(Doc::text(self.loc_col.to_string()))
    }
}

/// `deriving newtype (Pretty)` on AngleVersion (Types.hs:704-706):
/// prettyprinter's Pretty Int is decimal, so `pretty (v :: AngleVersion)`
/// renders the wrapped Int — folded into the Display trait here (the
/// source has no separate Display instance; `pretty` on AngleVersion is
/// display-default-shaped).
impl Display for AngleVersion {
    fn display(&self, _opts: &DisplayOpts) -> Doc {
        Doc::text(self.0.to_string())
    }
}

/// aeson-2.2.3.0's Text-string JSON escaping, byte-exact — used by the
/// `String` pattern Display below (`Aeson.encode (Aeson.String str)`,
/// Types.hs:903). Pinned from Data/Aeson/Encoding/Builder.hs:108-137
/// (`escapeAscii`): `\\` and `\"` escapes, raw for >= 0x20, `\n`/`\r`/
/// `\t` shorthands, everything else < 0x20 as `\u00xx` with LOWERCASE
/// hex (bytestring's word16HexFixed lowerTable), quotes around, no
/// whitespace. Note aeson has NO `\b`/`\f` shorthands (0x08/0x0C render
/// as \u0008/\u000c) — serde_json's default escaping differs there,
/// which is why this is hand-written rather than delegated (RULE §1
/// JSON row serves serde consumers; this site's byte-truth is aeson).
pub(crate) fn aeson_escape_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Haskell `show :: ByteString -> String` for the ByteArray pattern
/// Display (Types.hs:906). bytestring-0.11.5.2's
/// `instance Show ByteString where showsPrec p ps = showsPrec p
/// (unpackChars ps)` (Data/ByteString/Internal/Type.hs:321-322) — the
/// bytes Latin-1-decoded to a Haskell String, then shown with base
/// 4.17.1.0's GHC.Show showLitString/showLitChar/asciiTab (GHC/Show.hs:
/// 375-398, 428-434): each byte < 0x100 becomes one Char.
fn show_byte_string(bytes: &[u8]) -> String {
    // asciiTab — GHC.Show.hs:428-434 (indices 0..32).
    const ASCII_TAB: [&str; 33] = [
        "NUL", "SOH", "STX", "ETX", "EOT", "ENQ", "ACK", "BEL", "BS", "HT", "LF", "VT", "FF", "CR",
        "SO", "SI", "DLE", "DC1", "DC2", "DC3", "DC4", "NAK", "SYN", "ETB", "CAN", "EM", "SUB",
        "ESC", "FS", "GS", "RS", "US", "SP",
    ];
    // protectEsc checks the first char of the rendered REST. The rendered
    // rest starts with the next byte's first output char: a raw printable
    // (0x20..=0x7E) or a backslash — so the guard reduces to "next input
    // byte is ...". At end-of-string the rest starts with '"' (the
    // closing quote) — neither guard fires.
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
            // c > '\DEL': \DDD decimal + protectEsc isDec
            // (GHC.Show.hs:373, same two-char `\&` terminator,
            // GHC.Show.hs:422-424)
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

/// `instance (Display p, Display t) => Display (SourcePat_ s st p t)`
/// (Types.hs:900-957) with the `displayAtom` override at :959-986.
impl<S, ST, P: Display, T: Display> Display for SourcePat_<S, ST, P, T> {
    fn display(&self, opts: &DisplayOpts) -> Doc {
        match self {
            SourcePat_::Nat(_, w) => Doc::text(w.to_string()),
            SourcePat_::String(_, s) => Doc::text(aeson_escape_string(s)),
            SourcePat_::StringPrefix(_, s) => {
                Doc::text(aeson_escape_string(s)).append(Doc::text(".."))
            }
            SourcePat_::ByteArray(_, b) => Doc::text(show_byte_string(b)),
            SourcePat_::Array(_, pats) => brackets(hsep(punctuate(
                Doc::text(","),
                pats.iter().map(|p| p.display(opts)).collect(),
            ))),
            SourcePat_::ArrayPrefix(_, pats) => enclose_sep(
                Doc::text("["),
                Doc::text(", ..]"),
                Doc::text(","),
                pats.iter().map(|p| p.display(opts)),
            ),
            SourcePat_::Tuple(_, pats) => braces(hsep(punctuate(
                Doc::text(","),
                pats.iter().map(|p| p.display(opts)).collect(),
            ))),
            SourcePat_::Struct(_, fs) => {
                // cat [ nest 2 $ cat [ "{", fields fs], "}"]
                //   fields = sep . punctuate "," . map field
                //   field (Field name pat) = pretty name <+> "=" <+> display opts pat
                let field = |(name, pat): (&FieldName, &SourcePat_<S, ST, P, T>)| {
                    Doc::text(name.clone())
                        .append_space(Doc::text("="))
                        .append_space(pat.display(opts))
                };
                let fields = sep(punctuate(
                    Doc::text(","),
                    fs.iter().map(|Field(name, pat)| field((name, pat))).collect(),
                ));
                cat(vec![
                    nest(2, cat(vec![Doc::text("{"), fields])),
                    Doc::text("}"),
                ])
            }
            SourcePat_::App(_, l, pats) => l.display(opts).append_space(hsep(punctuate(
                Doc::text(" "),
                pats.iter().map(|p| p.display_atom(opts)).collect(),
            ))),
            SourcePat_::KeyValue(_, k, v) => k
                .display_atom(opts)
                .append_space(Doc::text("->"))
                .append_space(v.display_atom(opts)),
            SourcePat_::Elements(_, pat) => {
                Doc::text("elements").append_space(parens(pat.display(opts)))
            }
            SourcePat_::All(_, query) => {
                Doc::text("all").append_space(parens(query.display(opts)))
            }
            SourcePat_::Wildcard(_) => Doc::text("_"),
            SourcePat_::Variable(_, name) => Doc::text(name.clone()),
            SourcePat_::ElementsOfArray(_, pat) => {
                pat.display_atom(opts).append(Doc::text("[..]"))
            }
            SourcePat_::OrPattern(_, lhs, rhs) => sep(vec![
                lhs.display_atom(opts).append_space(Doc::text("|")),
                rhs.display_atom(opts),
            ]),
            SourcePat_::IfPattern { cond, then_, else_, .. } => sep(vec![
                nest(2, sep(vec![
                    Doc::text("if"),
                    cond.display_atom(opts),
                ])),
                nest(2, sep(vec![
                    Doc::text("then"),
                    then_.display_atom(opts),
                ])),
                nest(2, sep(vec![
                    Doc::text("else"),
                    else_.display_atom(opts),
                ])),
            ]),
            SourcePat_::NestedQuery(_, q) => parens(q.display(opts)),
            SourcePat_::Negation(_, q) => Doc::text("!").append(parens(q.display(opts))),
            SourcePat_::FactId(_, None, n) => Doc::text("$").append(Doc::text(n.to_string())),
            SourcePat_::FactId(_, Some(p), n) => Doc::text("$")
                .append(Doc::text(p.clone()))
                .append_space(Doc::text(n.to_string())),
            SourcePat_::TypeSignature(_, p, t) => p
                .display_atom(opts)
                .append_space(Doc::text(":"))
                .append_space(t.display(opts)),
            SourcePat_::Never(_) => Doc::text("never"),
            SourcePat_::Clause(_, _, p, pat, rng) => {
                let prng = match rng {
                    SeekSection::SeekOnBase => Doc::text("#old"),
                    SeekSection::SeekOnStacked => Doc::text("#new"),
                    _ => Doc::empty(),
                };
                p.display(opts).append(prng).append_space(pat.display_atom(opts))
            }
            SourcePat_::Prim(_, p, pats) => p.display(opts).append_space(hsep(punctuate(
                Doc::text(" "),
                pats.iter().map(|q| q.display_atom(opts)).collect(),
            ))),
            SourcePat_::FieldSelect(_, pat, field, q) => pat.display_atom(opts)
                .append(Doc::text("."))
                .append(Doc::text(field.clone()))
                .append(match q {
                    RecordOrSum::Sum => Doc::text("?"),
                    RecordOrSum::Record => Doc::empty(),
                }),
            SourcePat_::Deref(_, pat) => pat.display_atom(opts).append(Doc::text(".*")),
            SourcePat_::Enum(_, f) => Doc::text(f.clone()),
        }
    }

    /// `displayAtom` override (Types.hs:959-986) — parens exactly for
    /// App, KeyValue, OrPattern, IfPattern, TypeSignature,
    /// ElementsOfArray, All, Clause, Prim; everything else renders bare.
    fn display_atom(&self, opts: &DisplayOpts) -> Doc {
        match self {
            SourcePat_::App(..)
            | SourcePat_::KeyValue(..)
            | SourcePat_::OrPattern(..)
            | SourcePat_::IfPattern { .. }
            | SourcePat_::TypeSignature(..)
            | SourcePat_::ElementsOfArray(..)
            | SourcePat_::All(..)
            | SourcePat_::Clause(..)
            | SourcePat_::Prim(..) => parens(self.display(opts)),
            _other => self.display(opts),
        }
    }
}

/// `instance (Display p, Display t) => Display (SourceQuery_ s st p t)`
/// (Types.hs:988-995).
impl<S, ST, P: Display, T: Display> Display for SourceQuery_<S, ST, P, T> {
    fn display(&self, opts: &DisplayOpts) -> Doc {
        let pstmts = punctuate(
            Doc::text(";"),
            self.src_query_stmts.iter().map(|s| s.display(opts)).collect(),
        );
        if self.src_query_stmts.is_empty() {
            match &self.src_query_head {
                None => Doc::empty(), // maybe mempty (display opts) maybeHead
                Some(head) => head.display(opts),
            }
        } else {
            match &self.src_query_head {
                Some(head) => hang(
                    2,
                    sep(
                        Some(head.display(opts).append_space(Doc::text("where")))
                            .into_iter()
                            .chain(pstmts),
                    ),
                ),
                None => sep(pstmts),
            }
        }
    }
}

/// `instance (Display p, Display t) => Display (SourceStatement_ s st
/// p t)` (Types.hs:997-998).
impl<S, ST, P: Display, T: Display> Display for SourceStatement_<S, ST, P, T> {
    fn display(&self, opts: &DisplayOpts) -> Doc {
        display_statement(opts, &self.0, &self.1)
    }
}

/// `displayStatement` (Types.hs:1000-1008) — exported.
pub fn display_statement<Pat>(opts: &DisplayOpts, lhs: &Pat, rhs: &Pat) -> Doc
where
    Pat: IsWild + Display,
{
    if lhs.is_wild() {
        rhs.display(opts)
    } else {
        hang(
            2,
            sep(vec![
                lhs.display(opts).append_space(Doc::text("=")),
                rhs.display(opts),
            ]),
        )
    }
}

/// `instance Display PrimOp` (Types.hs:1010-1025).
impl Display for PrimOp {
    fn display(&self, _opts: &DisplayOpts) -> Doc {
        Doc::text(match self {
            PrimOp::PrimOpToLower => "prim.toLower",
            PrimOp::PrimOpLength => "prim.length",
            PrimOp::PrimOpZip => "prim.zip",
            PrimOp::PrimOpConcat => "prim.concat",
            PrimOp::PrimOpReverse => "prim.reverse",
            PrimOp::PrimOpSize => "prim.size",
            PrimOp::PrimOpRelToAbsByteSpans => "prim.relToAbsByteSpans",
            PrimOp::PrimOpUnpackByteSpans => "prim.unpackByteSpans",
            PrimOp::PrimOpGtNat => "prim.gtNat",
            PrimOp::PrimOpGeNat => "prim.geNat",
            PrimOp::PrimOpLtNat => "prim.ltNat",
            PrimOp::PrimOpLeNat => "prim.leNat",
            PrimOp::PrimOpNeNat => "prim.neNat",
            PrimOp::PrimOpAddNat => "prim.addNat",
            PrimOp::PrimOpNeExpr => "prim.neExpr",
        })
    }
}

// ---------------------------------------------------------------------------
// Removing source locations from the AST
// ---------------------------------------------------------------------------
//
// The whole rmLoc/rmTypeLoc family takes values by value — the Haskell
// originals are `a -> a'` (Types.hs:1030-1174), so the Rust ports
// consume and rebuild rather than borrowing-and-cloning; no Clone
// bounds needed and the move structure mirrors the rebuild.

/// `rmLocSchemas` (Types.hs:1030-1032).
pub fn rm_loc_schemas<S>(schemas: SourceSchemas_<S>) -> SourceSchemas_<()> {
    SourceSchemas_ {
        src_angle_version: schemas.src_angle_version,
        src_schemas: schemas.src_schemas.into_iter().map(rm_loc_schema).collect(),
        src_evolves: schemas.src_evolves.into_iter().map(rm_loc_evolves).collect(),
    }
}

/// `rmLocSchema` (Types.hs:1034-1036).
pub fn rm_loc_schema<S>(schema: SourceSchema_<S>) -> SourceSchema_<()> {
    SourceSchema_ {
        schema_name: schema.schema_name,
        schema_inherits: schema.schema_inherits,
        schema_decls: schema.schema_decls.into_iter().map(rm_loc_decl).collect(),
        schema_src_span: (),
    }
}

/// `rmLocEvolves` (Types.hs:1038-1039).
pub fn rm_loc_evolves<S>(evolves: SourceEvolves_<S>) -> SourceEvolves_<()> {
    SourceEvolves_ {
        evolves_span: (),
        evolves_new: evolves.evolves_new,
        evolves_old: evolves.evolves_old,
    }
}

/// `rmLocDecl` (Types.hs:1041-1047).
pub fn rm_loc_decl<S>(decl: SourceDecl_<S>) -> SourceDecl_<()> {
    match decl {
        SourceDecl_::SourceImport(name, _) => SourceDecl_::SourceImport(name, ()),
        SourceDecl_::SourcePredicate(pred) => {
            SourceDecl_::SourcePredicate(rm_loc_pred_def(pred))
        }
        SourceDecl_::SourceType(type_def) => SourceDecl_::SourceType(rm_loc_type_def(type_def)),
        SourceDecl_::SourceDeriving(DerivingDef_ {
            deriving_def_ref,
            deriving_def_derive_info,
            ..
        }) => SourceDecl_::SourceDeriving(DerivingDef_ {
            deriving_def_ref,
            deriving_def_derive_info: deriving_def_derive_info.map(&mut rm_loc_query),
            deriving_def_src_span: (),
        }),
    }
}

/// `rmLocPredDef` (Types.hs:1049-1056).
pub fn rm_loc_pred_def<S, ST, P, T>(
    def: PredicateDef_<S, ST, P, T>,
) -> PredicateDef_<(), (), P, T> {
    PredicateDef_ {
        predicate_def_ref: def.predicate_def_ref,
        predicate_def_key_type: rm_loc_type(def.predicate_def_key_type),
        predicate_def_value_type: rm_loc_type(def.predicate_def_value_type),
        predicate_def_deriving: def.predicate_def_deriving.map(&mut rm_loc_query),
        predicate_def_src_span: (),
    }
}

/// `rmLocQuery` (Types.hs:1058-1060).
pub fn rm_loc_query<S, ST, P, T>(
    query: SourceQuery_<S, ST, P, T>,
) -> SourceQuery_<(), (), P, T> {
    SourceQuery_ {
        src_query_head: query.src_query_head.map(rm_loc_pat),
        src_query_stmts: query
            .src_query_stmts
            .into_iter()
            .map(rm_loc_statement)
            .collect(),
        src_query_ordered: query.src_query_ordered,
    }
}

/// `rmLocStatement` (Types.hs:1062-1064).
pub fn rm_loc_statement<S, ST, P, T>(
    stmt: SourceStatement_<S, ST, P, T>,
) -> SourceStatement_<(), (), P, T> {
    let SourceStatement_(x, y) = stmt;
    SourceStatement_(rm_loc_pat(x), rm_loc_pat(y))
}

/// `rmLocTypeDef` (Types.hs:1066-1067).
pub fn rm_loc_type_def<S, P, T>(def: TypeDef_<S, P, T>) -> TypeDef_<(), P, T> {
    let TypeDef_ {
        type_def_ref,
        type_def_type,
        ..
    } = def;
    TypeDef_ {
        type_def_ref,
        type_def_type: rm_loc_type(type_def_type),
        type_def_src_span: (),
    }
}

/// `rmLocType` (Types.hs:1069-1086).
pub fn rm_loc_type<ST, P, T>(ty: Type_<ST, P, T>) -> Type_<(), P, T> {
    match ty {
        Type_::ByteTy => Type_::ByteTy,
        Type_::NatTy => Type_::NatTy,
        Type_::StringTy => Type_::StringTy,
        Type_::ArrayTy(t) => Type_::ArrayTy(Box::new(rm_loc_type(*t))),
        Type_::RecordTy(xs) => Type_::RecordTy(xs.into_iter().map(rm_loc_field_def).collect()),
        Type_::SumTy(xs) => Type_::SumTy(xs.into_iter().map(rm_loc_field_def).collect()),
        Type_::SetTy(t) => Type_::SetTy(Box::new(rm_loc_type(*t))),
        Type_::PredicateTy(_, pref) => Type_::PredicateTy((), pref),
        Type_::NamedTy(_, tref) => Type_::NamedTy((), tref),
        Type_::MaybeTy(t) => Type_::MaybeTy(Box::new(rm_loc_type(*t))),
        Type_::EnumeratedTy(xs) => Type_::EnumeratedTy(xs),
        Type_::BooleanTy => Type_::BooleanTy,
        Type_::TyVar(x) => Type_::TyVar(x),
        Type_::HasTy(m, r, x) => Type_::HasTy(
            m.into_iter().map(|(k, v)| (k, rm_loc_type(v))).collect(),
            r,
            x,
        ),
        Type_::HasKey(t, x) => Type_::HasKey(Box::new(rm_loc_type(*t)), x),
        Type_::ElementsOf(t, x) => Type_::ElementsOf(Box::new(rm_loc_type(*t)), x),
    }
}

/// `rmLocFieldDef` (Types.hs:1088-1089).
pub fn rm_loc_field_def<ST, P, T>(def: FieldDef_<ST, P, T>) -> FieldDef_<(), P, T> {
    FieldDef_ {
        field_def_name: def.field_def_name,
        field_def_type: rm_loc_type(def.field_def_type),
    }
}

/// `rmLocPat` (Types.hs:1091-1119).
pub fn rm_loc_pat<S, ST, P, T>(pat: SourcePat_<S, ST, P, T>) -> SourcePat_<(), (), P, T> {
    match pat {
        SourcePat_::Nat(_, x) => SourcePat_::Nat((), x),
        SourcePat_::String(_, x) => SourcePat_::String((), x),
        SourcePat_::StringPrefix(_, x) => SourcePat_::StringPrefix((), x),
        SourcePat_::ByteArray(_, x) => SourcePat_::ByteArray((), x),
        SourcePat_::Array(_, xs) => {
            SourcePat_::Array((), xs.into_iter().map(rm_loc_pat).collect())
        }
        SourcePat_::ArrayPrefix(_, xs) => SourcePat_::ArrayPrefix(
            (),
            NonEmpty {
                head: rm_loc_pat(xs.head),
                tail: xs.tail.into_iter().map(rm_loc_pat).collect(),
            },
        ),
        SourcePat_::Tuple(_, xs) => {
            SourcePat_::Tuple((), xs.into_iter().map(rm_loc_pat).collect())
        }
        SourcePat_::Struct(_, xs) => SourcePat_::Struct(
            (),
            xs.into_iter().map(rm_loc_field).collect(),
        ),
        SourcePat_::App(_, x, xs) => SourcePat_::App(
            (),
            Box::new(rm_loc_pat(*x)),
            xs.into_iter().map(rm_loc_pat).collect(),
        ),
        SourcePat_::KeyValue(_, x, y) => SourcePat_::KeyValue(
            (),
            Box::new(rm_loc_pat(*x)),
            Box::new(rm_loc_pat(*y)),
        ),
        SourcePat_::Elements(_, pat) => SourcePat_::Elements((), Box::new(rm_loc_pat(*pat))),
        SourcePat_::All(_, query) => SourcePat_::All((), Box::new(rm_loc_pat(*query))),
        SourcePat_::Wildcard(_) => SourcePat_::Wildcard(()),
        SourcePat_::Never(_) => SourcePat_::Never(()),
        SourcePat_::Variable(_, v) => SourcePat_::Variable((), v),
        SourcePat_::ElementsOfArray(_, x) => {
            SourcePat_::ElementsOfArray((), Box::new(rm_loc_pat(*x)))
        }
        SourcePat_::OrPattern(_, x, y) => SourcePat_::OrPattern(
            (),
            Box::new(rm_loc_pat(*x)),
            Box::new(rm_loc_pat(*y)),
        ),
        SourcePat_::IfPattern { span: _, cond, then_, else_ } => SourcePat_::IfPattern {
            span: (),
            cond: Box::new(rm_loc_pat(*cond)),
            then_: Box::new(rm_loc_pat(*then_)),
            else_: Box::new(rm_loc_pat(*else_)),
        },
        SourcePat_::Negation(_, x) => SourcePat_::Negation((), Box::new(rm_loc_pat(*x))),
        SourcePat_::NestedQuery(_, query) => {
            SourcePat_::NestedQuery((), Box::new(rm_loc_query(query)))
        }
        SourcePat_::FactId(_, x, y) => SourcePat_::FactId((), x, y),
        SourcePat_::TypeSignature(_, x, t) => SourcePat_::TypeSignature(
            (),
            Box::new(rm_loc_pat(*x)),
            Box::new(rm_loc_type(*t)),
        ),
        SourcePat_::Clause(_, _, x, y, rng) => {
            SourcePat_::Clause((), (), x, Box::new(rm_loc_pat(*y)), rng)
        }
        SourcePat_::Prim(_, p, ps) => SourcePat_::Prim((), p, ps.into_iter().map(rm_loc_pat).collect()),
        SourcePat_::FieldSelect(_, pat, field, q) => {
            SourcePat_::FieldSelect((), Box::new(rm_loc_pat(*pat)), field, q)
        }
        SourcePat_::Deref(_, pat) => SourcePat_::Deref((), Box::new(rm_loc_pat(*pat))),
        SourcePat_::Enum(_, f) => SourcePat_::Enum((), f),
    }
}

/// `rmLocField` (Types.hs:1121-1123).
pub fn rm_loc_field<S, ST, P, T>(field: Field<S, ST, P, T>) -> Field<(), (), P, T> {
    Field(field.0, rm_loc_pat(field.1))
}

// Remove spans from type refs only. Useful when
// the two span types are different (s != st), e.g., s = SrcSpan, st = ()
// (Types.hs:1125-1126.)

/// `rmTypeLocPredDef` (Types.hs:1127-1131).
pub fn rm_type_loc_pred_def<S, ST, P, T>(
    def: PredicateDef_<S, ST, P, T>,
) -> PredicateDef_<S, (), P, T> {
    PredicateDef_ {
        predicate_def_ref: def.predicate_def_ref,
        predicate_def_key_type: rm_loc_type(def.predicate_def_key_type),
        predicate_def_value_type: rm_loc_type(def.predicate_def_value_type),
        predicate_def_deriving: def.predicate_def_deriving.map(&mut rm_type_loc_query),
        predicate_def_src_span: def.predicate_def_src_span,
    }
}

/// `rmTypeLocQuery` (Types.hs:1133-1135).
pub fn rm_type_loc_query<S, ST, P, T>(
    query: SourceQuery_<S, ST, P, T>,
) -> SourceQuery_<S, (), P, T> {
    SourceQuery_ {
        src_query_head: query.src_query_head.map(rm_type_loc_pat),
        src_query_stmts: query
            .src_query_stmts
            .into_iter()
            .map(rm_type_loc_statement)
            .collect(),
        src_query_ordered: query.src_query_ordered,
    }
}

/// `rmTypeLocStatement` (Types.hs:1137-1139).
pub fn rm_type_loc_statement<S, ST, P, T>(
    stmt: SourceStatement_<S, ST, P, T>,
) -> SourceStatement_<S, (), P, T> {
    let SourceStatement_(x, y) = stmt;
    SourceStatement_(rm_type_loc_pat(x), rm_type_loc_pat(y))
}

/// `rmTypeLocPat` (Types.hs:1141-1170).
pub fn rm_type_loc_pat<S, ST, P, T>(pat: SourcePat_<S, ST, P, T>) -> SourcePat_<S, (), P, T> {
    match pat {
        SourcePat_::Nat(s, x) => SourcePat_::Nat(s, x),
        SourcePat_::String(s, x) => SourcePat_::String(s, x),
        SourcePat_::StringPrefix(s, x) => SourcePat_::StringPrefix(s, x),
        SourcePat_::ByteArray(s, x) => SourcePat_::ByteArray(s, x),
        SourcePat_::TypeSignature(s, x, t) => SourcePat_::TypeSignature(
            s,
            Box::new(rm_type_loc_pat(*x)),
            Box::new(rm_loc_type(*t)),
        ),
        SourcePat_::Array(s, xs) => {
            SourcePat_::Array(s, xs.into_iter().map(rm_type_loc_pat).collect())
        }
        SourcePat_::ArrayPrefix(s, xs) => SourcePat_::ArrayPrefix(
            s,
            NonEmpty {
                head: rm_type_loc_pat(xs.head),
                tail: xs.tail.into_iter().map(rm_type_loc_pat).collect(),
            },
        ),
        SourcePat_::Tuple(s, xs) => {
            SourcePat_::Tuple(s, xs.into_iter().map(rm_type_loc_pat).collect())
        }
        SourcePat_::Struct(s, xs) => SourcePat_::Struct(
            s,
            xs.into_iter().map(rm_type_loc_field).collect(),
        ),
        SourcePat_::App(s, x, xs) => SourcePat_::App(
            s,
            Box::new(rm_type_loc_pat(*x)),
            xs.into_iter().map(rm_type_loc_pat).collect(),
        ),
        SourcePat_::KeyValue(s, x, y) => SourcePat_::KeyValue(
            s,
            Box::new(rm_type_loc_pat(*x)),
            Box::new(rm_type_loc_pat(*y)),
        ),
        SourcePat_::Elements(s, pat) => {
            SourcePat_::Elements(s, Box::new(rm_type_loc_pat(*pat)))
        }
        SourcePat_::All(s, query) => SourcePat_::All(s, Box::new(rm_type_loc_pat(*query))),
        SourcePat_::Wildcard(s) => SourcePat_::Wildcard(s),
        SourcePat_::Never(s) => SourcePat_::Never(s),
        SourcePat_::Variable(s, v) => SourcePat_::Variable(s, v),
        SourcePat_::ElementsOfArray(s, x) => {
            SourcePat_::ElementsOfArray(s, Box::new(rm_type_loc_pat(*x)))
        }
        SourcePat_::OrPattern(s, x, y) => SourcePat_::OrPattern(
            s,
            Box::new(rm_type_loc_pat(*x)),
            Box::new(rm_type_loc_pat(*y)),
        ),
        SourcePat_::IfPattern { span, cond, then_, else_ } => SourcePat_::IfPattern {
            span,
            cond: Box::new(rm_type_loc_pat(*cond)),
            then_: Box::new(rm_type_loc_pat(*then_)),
            else_: Box::new(rm_type_loc_pat(*else_)),
        },
        SourcePat_::Negation(s, x) => SourcePat_::Negation(s, Box::new(rm_type_loc_pat(*x))),
        SourcePat_::NestedQuery(s, query) => {
            SourcePat_::NestedQuery(s, Box::new(rm_type_loc_query(query)))
        }
        SourcePat_::FactId(s, x, y) => SourcePat_::FactId(s, x, y),
        SourcePat_::Clause(s, ss, x, y, rng) => {
            SourcePat_::Clause(s, ss, x, Box::new(rm_type_loc_pat(*y)), rng)
        }
        SourcePat_::Prim(s, p, ps) => {
            SourcePat_::Prim(s, p, ps.into_iter().map(rm_type_loc_pat).collect())
        }
        SourcePat_::FieldSelect(s, pat, field, q) => {
            SourcePat_::FieldSelect(s, Box::new(rm_type_loc_pat(*pat)), field, q)
        }
        SourcePat_::Deref(s, pat) => SourcePat_::Deref(s, Box::new(rm_type_loc_pat(*pat))),
        SourcePat_::Enum(s, f) => SourcePat_::Enum(s, f),
    }
}

/// `rmTypeLocField` (Types.hs:1172-1174).
pub fn rm_type_loc_field<S, ST, P, T>(field: Field<S, ST, P, T>) -> Field<S, (), P, T> {
    Field(field.0, rm_type_loc_pat(field.1))
}

// ---------------------------------------------------------------------------
// Describing the kind of pattern
// ---------------------------------------------------------------------------

/// `class Describe a where describe :: a -> Doc ann` (Types.hs:1179-1180).
pub trait Describe {
    fn describe(&self) -> Doc;
}

/// `instance Describe (SourcePat_ s st p t)` (Types.hs:1182-1210).
impl<S, ST, P, T> Describe for SourcePat_<S, ST, P, T> {
    fn describe(&self) -> Doc {
        Doc::text(match self {
            SourcePat_::Nat(..) => "a nat",
            SourcePat_::String(..) => "a string",
            SourcePat_::StringPrefix(..) => "a string prefix",
            SourcePat_::ByteArray(..) => "a byte array",
            SourcePat_::Array(..) => "an array",
            SourcePat_::ArrayPrefix(..) => "an array prefix",
            SourcePat_::Tuple(..) => "a tuple",
            SourcePat_::Struct(..) => "a struct",
            SourcePat_::App(..) => "an application",
            SourcePat_::KeyValue(..) => "a key value pattern",
            SourcePat_::All(..) => "a set with all the results of its argument",
            SourcePat_::Wildcard(..) => "a wildcard pattern",
            SourcePat_::Never(..) => "a never pattern",
            SourcePat_::Variable(..) => "a variable",
            SourcePat_::ElementsOfArray(..) => "an element of an array",
            SourcePat_::Elements(..) => "an element of a set",
            SourcePat_::OrPattern(..) => "an or pattern",
            SourcePat_::IfPattern { .. } => "an if pattern",
            SourcePat_::Negation(..) => "a negation",
            SourcePat_::NestedQuery(..) => "a nested query",
            SourcePat_::FactId(..) => "a fact id",
            SourcePat_::TypeSignature(..) => "a type signature",
            SourcePat_::Clause(..) => "a clause",
            SourcePat_::Prim(..) => "a primitive function",
            SourcePat_::FieldSelect(..) => "a record field",
            SourcePat_::Deref(..) => "a dereference",
            SourcePat_::Enum(..) => "an enum value",
        })
    }
}

/// `instance Describe SrcSpan` (Types.hs:1212-1213).
impl Describe for SrcSpan {
    fn describe(&self) -> Doc {
        Doc::text("a source span")
    }
}

/// `instance Describe SrcLoc` (Types.hs:1215-1216).
impl Describe for SrcLoc {
    fn describe(&self) -> Doc {
        Doc::text("a source location")
    }
}

// ---------------------------------------------------------------------------
// Marker breakdown (RULE §6: reviewers check the trailer against this)
// ---------------------------------------------------------------------------
//
// Open TODO(port) markers in this file: 3 (plus 1 PERF(port), perf-only,
// not a todo):
//   1. PredicateId std::hash::Hash (line ~925): delegates to
//      hash_fingerprint, whose hashable-1.4.7 chain is the sibling
//      hash.rs's FNV stand-in (its open TODO #1). In-memory HashMap
//      salt only; ComputeIds sorts before hashing so schema IDs are
//      unaffected.
//   2. TypeId std::hash::Hash (line ~978): same placeholder status.
//   3. Byte-exactness validation obligation for the whole encoder
//      surface (immediately below): the old-binary getSchemaInfo judge
//      case must pass over the 96 .angle schema sources before this
//      linchpin is proven (inventory.tsv row 6: "byte-exactness
//      validated by the same old-binary getSchemaInfo judge case as
//      ComputeIds"). This unit states the format from the pinned
//      package sources; the proof obligation is the judge's, tracked
//      here and in hash.rs.
// TODO(port): validation obligation — the Data.Binary encodings in this
// module (trait + all impls above) feed hash_binary and thereby the
// schema-ID / dbSchemaKey chain; they are byte-exact claims pinned from
// the binary-0.8.9.1 / text-2.0.2 sources until the old-binary
// getSchemaInfo judge case passes over the 96 .angle schema sources.
//
// The source's own "TODO maybe just serialize the hash?" comments on
// PredicateId/TypeId (Types.hs:582/:606) are NOT open: the source never
// took them, so the encodings serialize TypeRef + 2-word Fingerprint
// (inventory row: "the bare-derived PredicateId/TypeId (which serialize
// TypeRef+2-word Fingerprint)").
//
// Coverage: every declaration and instance in Angle/Types.hs has a Rust
// counterpart — all 25 data/newtype/type-synonym declarations, the 11
// Binary instances (+ the parametric 6: SourceQuery_/SourceStatement_/
// SourcePat_/Field/Type_/FieldDef_ with () spans), all 30+ Display
// instances, IsWild/IsSrcSpan/Describe, sourcePatSpan/spanBetween/
// displayStatement/tupleField/tempPredicateId/latest*Version, the
// Bifunctor/Bifoldable family as bimap/bifold_map (full-tree, B6), the
// Functor/Foldable derivings as DerivingInfo::map/fold_map (the
// AST-level Functor derivings are subsumed by bimap with an identity
// predicate closure), the complete rmLoc*/rmTypeLoc* families, and the
// module list of type aliases (primed Haskell aliases carry a
// WithRefs/WithIds suffix — Rust identifiers cannot contain ').
//
// MODULE-WIRING: lib.rs / angle/mod.rs scaffolding is deliberately NOT a manifest
// unit; owned by the Step-4 survey build (do not create ad-hoc here).
// PORT STATUS: confidence=medium todos=3
