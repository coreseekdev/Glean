// Ported from: glean/angle/Glean/Angle/Hash.hs
// Manifest unit: rust/glean-angle/src/angle/hash.rs (crate glean-angle)
//
// This file is the HASH LINCHPIN of the port. Hash = GHC.Fingerprint, which
// is the MD5 digest of the hashed bytes interpreted as two BIG-ENDIAN u64s:
//   fingerprint1 = digest[0..8]  (big-endian)
//   fingerprint2 = digest[8..16] (big-endian)
// The word layout above is not reconstructed from memory alone: it is pinned
// by a GHC-computed vector embedded in Glean's own test suite —
// glean/test/tests/TestShard.hs asserts dbShard (Repo "test" "1234") ==
// "6825422262439797754", and Backend/Types.hs:478 computes that from
// `Fingerprint w _ <- fingerprintData (castPtr ptr) len; return (w shiftR 1)`
// over the bytes "test/1234". MD5("test/1234") = bd718ac52588c7f5291e839f03ed457c,
// whose first 8 bytes big-endian are 0xbd718ac52588c7f5, and
// 0xbd718ac52588c7f5 >> 1 = 6825422262439797754 exactly (verified offline
// against the RFC 1321 reference suite; re-expressed in this crate's
// tests/ suite at rust/glean-angle/tests/hash.rs). RULE §2
// (GHC.Fingerprint row): dedicated fingerprint module,
// byte-exact, feeds schema IDs — compatibility linchpin.
//
// SIBLING-UNIT CONTRACT (inventory.tsv: Backend/Types.hs/dbShardWord row —
// "reuse the SAME dedicated MD5 fingerprint module as Hash.hs"): this module
// IS that shared module. The glean-db Backend/Types translation
// (rust/glean-db/src/backend/types.rs, manifest row 41) must
// import `use glean_angle::angle::hash::{fingerprint_data, Fingerprint}` and
// must NOT reimplement MD5. db_shard_word uses fingerprint1 (first word).
// INVENTORY-BUG-CANDIDATE row 10: row says "low word >> 1"; source truth is
// FIRST word (Fingerprint w _ at Backend/Types.hs:478, proven by TestShard
// golden bd718ac5...>>1=6825422262439797754) — row wording should say
// first/high word

//! Fingerprints for schema versioning.
//!
//! Doesn't need to be cryptographically secure, but does need to be
//! robust against accidental collisions.
//!
//! Port notes: everything here is pure — the Haskell's
//! `unsafeDupablePerformIO` around `GHC.fingerprintData` is a pure
//! function of the input bytes (no mutation, deterministic; inventory.tsv:
//! Hash.hs/hashByteString row mandates "pure fn, no IO in Rust").

use std::fmt;

// ASSUMED-IMPORT (RULE §4 B4): `Binary` is owned by the sibling manifest
// unit glean/angle/Glean/Angle/Types.hs -> rust/glean-angle/src/angle/types.rs,
// which per inventory.tsv (Types.hs / "11 Binary instances" row) provides the
// one hand-written Data.Binary-format encoder module (RULE §2 Data.Binary row).
// Contract:
//   pub trait Binary { fn encode(&self) -> Vec<u8>; }
// where `encode` must reproduce `Binary.encode` composed with
// `LB.toStrict` byte-exactly: the raw runPut stream with NO top-level
// length header, Data.Binary word encodings (fixed-width words are
// BIG-ENDIAN — `putWord64be` and friends) and the
// generic-derived constructor-tag/field order of each instance.
// This unit only consumes it (in hash_binary); it never re-declares the
// trait or any sibling type. If the sibling unit ships a free
// `encode(x: &T) -> Vec<u8>` instead of a trait method, adapting this one
// call site is mechanical and touches nothing else.
use crate::angle::types::Binary;

// ---------------------------------------------------------------------------
// Fingerprint (GHC.Fingerprint.Type)
// ---------------------------------------------------------------------------

/// GHC.Fingerprint.Type: `data Fingerprint = Fingerprint !Word64 !Word64
/// deriving (Eq, Ord)` (plus a Show instance — see Display below).
// RULE §1 (strings/bytes row, quoted verbatim): "`String`/`&str` (UTF-8)
// for `Text`; `Vec<u8>` for `ByteString`. No lossy conversions at
// boundaries; `Text.Encoding` errors in source become explicit `Result`".
// (The row covers Text/ByteString only.)
// Extrapolation beyond the quoted row: Haskell `String` (hashString's
// input) likewise maps to `String`/`&str`, and `Word64` stays `u64`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
/// In-memory std Hash (amendment 3, 2026-10-07): delegates to the two
/// words — stable, never persisted (schema-ID hashing uses fingerprint_data
/// above, not this impl). Owned here behind the hashable-chain TODO(port).
impl std::hash::Hash for Fingerprint {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u64(self.fingerprint1);
        state.write_u64(self.fingerprint2);
    }
}

pub struct Fingerprint {
    pub fingerprint1: u64,
    pub fingerprint2: u64,
}

// `type Hash = GHC.Fingerprint` (Hash.hs:34) — RULE §2 GHC.Fingerprint row.
pub type Hash = Fingerprint;

impl Fingerprint {
    pub const fn new(fingerprint1: u64, fingerprint2: u64) -> Self {
        Fingerprint {
            fingerprint1,
            fingerprint2,
        }
    }
}

// RULE §2 (deriving Show row, A4/B6) mandates Show parity; here the
// source's Show is GHC's own hand-written instance, not a derived one —
// GHC.Fingerprint.Type (base):
//   instance Show Fingerprint where
//     show (Fingerprint w1 w2) = hex16 w1 ++ hex16 w2
//       where hex16 i = let hex = showHex i ""
//                       in replicate (16 - length hex) '0' ++ hex
// i.e. exactly 32 LOWERCASE hex digits: each word zero-padded to 16
// digits, no constructor tag, no 0x prefix, no separating space.
// Display replicates that string; Debug mirrors Display so neither can
// drift from the other. This IS a persisted surface: ComputeIds.hs:307
// builds `SchemaId $ Text.pack $ show $ hashNameEnv env`, so schema IDs
// stored in existing DBs embed this exact text (display test vector in
// this crate's tests/ suite pins it; the hash bytes are unaffected).
impl fmt::Display for Fingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:016x}{:016x}", self.fingerprint1, self.fingerprint2)
    }
}

impl fmt::Debug for Fingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

// ---------------------------------------------------------------------------
// The exported API, in Hash.hs export order
// ---------------------------------------------------------------------------

/// `hash0 = GHC.fingerprint0` (Hash.hs:36-37).
pub const fn hash0() -> Hash {
    Fingerprint::new(0, 0)
}

/// `hashString = GHC.fingerprintString` (Hash.hs:39-40): fingerprint over
/// the string's marshalled bytes. GHC marshals each Char as its 32-bit
/// code point in BIG-ENDIAN order — 4 bytes per character
/// (w32>>24, w32>>16, w32>>8, w32) — with NO locale involved
/// (GHC.Fingerprint.hs:57-63: `fingerprintString str = ...
/// withArrayLen word8s $ \len p -> fingerprintData p len
/// where word8s = concatMap f str; f c = ... [w32 shiftR 24,
/// w32 shiftR 16, w32 shiftR 8, w32]`). It is NOT a UTF-8 encoding:
/// for "abc" the hashed bytes are 00 00 00 61 00 00 00 62 00 00 00 63.
/// A Rust `&str` yields exactly the Haskell's `ord c` per char (both are
/// Unicode scalar values; no surrogate halves in either). No call sites
/// exist in the tree (grep-verified; export-only).
pub fn hash_string(s: &str) -> Hash {
    // byte-exact restatement of GHC's concatMap: big-endian Word32 per Char
    let mut bytes = Vec::with_capacity(s.chars().count() * 4);
    for c in s.chars() {
        let w = c as u32;
        bytes.push((w >> 24) as u8);
        bytes.push((w >> 16) as u8);
        bytes.push((w >> 8) as u8);
        bytes.push(w as u8);
    }
    fingerprint_data(&bytes)
}

/// `hashByteString` (Hash.hs:42-45): `unsafeDupablePerformIO $
/// unsafeUseAsCStringLen bs $ \(ptr, len) -> GHC.fingerprintData (castPtr
/// ptr) (fromIntegral len)`. The IO was only the FFI hop over bytes GHC
/// never mutated; inventory.tsv (Hash.hs/hashByteString row) mandates the
/// pure Rust form. No shortcut hash may replace this (spec
/// schema-hash.allium, rule HashIsByteExact).
pub fn hash_byte_string(bytes: &[u8]) -> Hash {
    fingerprint_data(bytes)
}

/// `hashBinary = hashByteString . LB.toStrict . Binary.encode`
/// (Hash.hs:47-48). `LB.toStrict` is just lazy-chunk concatenation, so the
/// encoder's `Vec<u8>` output is byte-identical by contract (see the
/// ASSUMED-IMPORT above). Consumers: ComputeIds.hs fingerprintDef/cycleHash/
/// schemaId, Schema.hs dbSchemaKey.
pub fn hash_binary<T: Binary + ?Sized>(value: &T) -> Hash {
    hash_byte_string(&value.encode())
}

/// For building Hashable instances. Avoiding an orphan instance by
/// making this a function instead. (Hash.hs:50-53)
///
/// `hashFingerprint salt (GHC.Fingerprint _ w) = hashWithSalt salt w` —
/// salts over the SECOND word (fingerprint2) only.
// RULE §2 (UNKNOWN rule): conservative placeholder + open TODO.
// Data.Hashable (1.4.x line per cabal.project index-state 2025-04-14;
// chain verified against the hashable-1.4.7.0 sources) does NOT hash
// word-sized primitives by identity and does NOT combine with FNV-1a.
// The actual chain on 64-bit is xxh3-style, not FNV:
//   hashWithSalt @Word64 = hashWord64 (Class.hs:390-392);
//   hashWord64 s x = hashInt s (fromIntegral x) (LowLevel.hs:71);
//   hashInt s x = fromIntegral (mixHash (fromIntegral s)
//                                 (fromIntegral x)) (LowLevel.hs:64);
//   mixHash hi lo = avalanche (byteSwap lo + hi + mulFold hi lo)
//     (Mix.hs:48) — mulFold XORs the hi/lo halves of the 128-bit
//     product, avalanche is the MurmurHash3 64-bit finalizer ("inspired
//     by how xxh3 works on small (<=16byte) inputs", Mix.hs:46).
// The body below is a FNV-style multiply-xor stand-in, NOT that
// algorithm.
// TODO(port): implement the confirmed chain above (mulFold + avalanche
// over the salt and fingerprint2) and drop the FNV stand-in;
// open question only — the result is in-memory HashMap salt (Types.hs:572/591
// Hashable PredicateId/TypeId), never persisted, never golden-printed, and
// ComputeIds.hs:342 sorts before hashing, so HashMap iteration order is
// already neutralized on the schema-ID path.
pub fn hash_fingerprint(salt: i64, hash: &Hash) -> i64 {
    // FNV stand-in only; hashable's real mix is the xxh3-style chain
    // described above (still unimplemented).
    let w = hash.fingerprint2 as i64;
    salt.wrapping_mul(16777619) ^ w
}

// ---------------------------------------------------------------------------
// The fingerprint module proper (RULE §2 GHC.Fingerprint row + Data.Binary
// row: hand-written, no third-party hash crate — RULE §1 deps row: whitelist
// default NONE)
// ---------------------------------------------------------------------------

/// `GHC.fingerprintData ptr len`: MD5 over `bytes`, digest split as two
/// big-endian u64s. Pure and total — deterministic on the bytes.
///
/// Byte-exactness evidence lives in this crate's tests/ suite
/// (rust/glean-angle/tests/hash.rs): RFC 1321 vectors plus the
/// GHC-computed TestShard vector, expressed through this public API.
// TODO(port): validation obligation — spec/schema-hash.allium rule
// HashIsByteExact: the old-binary getSchemaInfo judge case must pass over
// all 96 schema sources before this linchpin is considered proven
// end-to-end (it also covers the Data.Binary encodings that feed
// hash_binary, which this unit consumes by contract, not by proof).
pub fn fingerprint_data(bytes: &[u8]) -> Fingerprint {
    let digest = md5_digest(bytes);
    Fingerprint::new(
        u64::from_be_bytes([
            digest[0], digest[1], digest[2], digest[3], digest[4], digest[5], digest[6], digest[7],
        ]),
        u64::from_be_bytes([
            digest[8], digest[9], digest[10], digest[11], digest[12], digest[13], digest[14],
            digest[15],
        ]),
    )
}

/// RFC 1321 MD5, matching GHC's own cbits copy (same reference code):
/// little-endian length suffix, little-endian digest-word serialization.
fn md5_digest(bytes: &[u8]) -> [u8; 16] {
    let mut md5 = Md5::new();
    md5.update(bytes);
    md5.finish()
}

/// Incremental MD5 state (one block buffered at a time — first pass is
/// faithful, not fast; RULE §0: slow but faithful is fine).
// PERF(port): fully-buffered one-block-at-a-time structure with a plain
// byte-wise transform; fast version = little-endian u64 message loads and
// a tight unrolled round loop over the input slice.
struct Md5 {
    a: u32,
    b: u32,
    c: u32,
    d: u32,
    /// total input bytes absorbed so far
    len: u64,
    buf: [u8; 64],
    fill: usize,
}

// K[i] = floor(|sin(i+1)| * 2^32), the RFC 1321 table (transcribed from the
// verified table used in this port's offline check — equals the RFC's
// published constants).
const K: [u32; 64] = [
    0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613, 0xfd469501,
    0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821,
    0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d, 0x02441453, 0xd8a1e681, 0xe7d3fbc8,
    0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed, 0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a,
    0xfffa3942, 0x8771f681, 0x6d9d6122, 0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70,
    0x289b7ec6, 0xeaa127fa, 0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665,
    0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
    0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391,
];

// Per-round rotate amounts (RFC 1321).
const S: [u32; 64] = [
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9,
    14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15,
    21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
];

impl Md5 {
    fn new() -> Self {
        Md5 {
            a: 0x6745_2301,
            b: 0xefcd_ab89,
            c: 0x98ba_dcfe,
            d: 0x1032_5476,
            len: 0,
            buf: [0; 64],
            fill: 0,
        }
    }

    fn update(&mut self, data: &[u8]) {
        let mut data = data;
        self.len = self.len.wrapping_add(data.len() as u64);
        // top up a partial block first
        if self.fill > 0 {
            let want = 64 - self.fill;
            let take = want.min(data.len());
            self.buf[self.fill..self.fill + take].copy_from_slice(&data[..take]);
            self.fill += take;
            data = &data[take..];
            if self.fill == 64 {
                let block = self.buf;
                self.transform(&block);
                self.fill = 0;
            }
        }
        // whole blocks straight from the input
        // PERF(port): copies every 64-byte block into a temp array before
        // transforming; fast version transforms a borrowed view of the
        // input slice directly (no per-block copy).
        while data.len() >= 64 {
            let mut block = [0u8; 64];
            block.copy_from_slice(&data[..64]);
            self.transform(&block);
            data = &data[64..];
        }
        // stash the tail
        if !data.is_empty() {
            self.buf[..data.len()].copy_from_slice(data);
            self.fill = data.len();
        }
    }

    fn finish(mut self) -> [u8; 16] {
        let bit_len = self.len.wrapping_mul(8);
        // pad: 0x80, zeros to 56 mod 64, then the 64-bit little-endian bit
        // length (RFC 1321 §3.1). The length bytes are folded through the
        // same buffering so the final block is processed exactly once.
        // PERF(port): padding is fed one byte per update() call; fast
        // version writes 0x80 + the zero run + the length words into the
        // buffer with bulk copies and transforms at most twice.
        self.update(&[0x80]);
        while self.len % 64 != 56 {
            self.update(&[0]);
        }
        self.update(&bit_len.to_le_bytes());
        debug_assert_eq!(self.fill, 0);
        let mut digest = [0u8; 16];
        digest[0..4].copy_from_slice(&self.a.to_le_bytes());
        digest[4..8].copy_from_slice(&self.b.to_le_bytes());
        digest[8..12].copy_from_slice(&self.c.to_le_bytes());
        digest[12..16].copy_from_slice(&self.d.to_le_bytes());
        digest
    }

    fn transform(&mut self, block: &[u8; 64]) {
        let mut m = [0u32; 16];
        for i in 0..16 {
            m[i] = u32::from_le_bytes([
                block[4 * i],
                block[4 * i + 1],
                block[4 * i + 2],
                block[4 * i + 3],
            ]);
        }
        let (mut a, mut b, mut c, mut d) = (self.a, self.b, self.c, self.d);
        for i in 0..64usize {
            let (f, g) = match i / 16 {
                0 => ((b & c) | (!b & d), i),
                1 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                2 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | !d), (7 * i) % 16),
            };
            let temp = d;
            d = c;
            c = b;
            let sum = a.wrapping_add(f).wrapping_add(K[i]).wrapping_add(m[g]);
            b = b.wrapping_add(sum.rotate_left(S[i]));
            a = temp;
        }
        self.a = self.a.wrapping_add(a);
        self.b = self.b.wrapping_add(b);
        self.c = self.c.wrapping_add(c);
        self.d = self.d.wrapping_add(d);
    }
}

// The re-expressed test suite for this unit lives in this crate's tests/
// directory (rust/glean-angle/tests/hash.rs), per RULE §4 — including the
// MD5 core vectors, which test through this module's public API only.

// Marker breakdown (2 open TODO(port) markers, all of the sanctioned
// port-question form):
//   1. hash_fingerprint: implement the confirmed hashable-1.4.x
//      hashWithSalt @Word64 chain (xxh3-style mixHash/avalanche over the
//      salt and fingerprint2); current FNV body is a placeholder
//      (memory-only surface).
//   2. fingerprint_data: schema-hash.allium HashIsByteExact judge
//      obligation (96 schema sources) over the full hash chain.
// (The Display/Debug Show question is CLOSED: GHC.Fingerprint.Type's
// hex16 rendering is implemented and pinned by a test vector; the three
// PERF(port) markers above are perf-only, not todos.)
// PORT STATUS: confidence=medium todos=2
