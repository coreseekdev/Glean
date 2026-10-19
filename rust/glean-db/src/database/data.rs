// Ported from: glean/db/Glean/Database/Data.hs (180 lines)
// Manifest unit: rust/glean-db/src/database/data.rs
// (migration/manifest.tsv line 60, crate glean-db). RULE §4: one source
// file -> one target file; the manifest path is authoritative.
//
// TYPE-MAPPING TRUTH:
//   * StoredSchema -> crate::internal::types::StoredSchema (generated,
//     inventory.tsv row 142; the same assumed import as storage.rs).
//   * Slice -> glean_core::rts::foreign::ownership::Slice (unit 57);
//     UsetId -> UsetId(pub u32).
//   * ByteString -> Vec<u8>; Text -> String; Word32 -> u32 (RULE §1).
//   * HashMap Text v -> std HashMap<String, v>.
//
// FORMAT TRUTH (these blobs are read back by this module and, for the
// schema, by older binaries; the byte formats are load-bearing):
//   * "schema" (Data.hs:42-43, :65-74): hsthrift Thrift Compact, via
//     serializeCompact/deserializeCompact (Thrift.Protocol.Compact:38-42,
//     a generated ThriftStruct codec — never hand-written, RULE §1 thrift
//     row). ASSUMED IMPORT below.
//   * "units" (:46-47, :76-85): Data.Binary encode of [ByteString].
//     binary-0.8.9.1: list = Int64be count + elements (Class.hs:588-591,
//     defaultPutList); Int = putInt64be (Class.hs:303); ByteString =
//     Int64be byte-count + bytes (Class.hs:625). So the blob is:
//         i64be count, then count * (i64be len, len bytes).
//   * "slices" (:50-51, :90-102): the same [ByteString] framing, where each
//     ByteString is a C++ Slice::serialize() payload produced by
//     serializeSlice (Ownership.hsc:149-153; Rust wrapper ownership.rs:573)
//     and consumed by deserializeSlice (Ownership.hsc:155-159;
//     ownership.rs:604).
//   * "acl_group_mapping" (:54-55, :106-124): raw bytes, written verbatim;
//     read as a JSON object mapping group name -> u32 (:113-124).
//   * "first_acl_id" (:62-63, :128-145): Data.Binary encode of Word32 =
//     4 bytes big-endian (Class.hs:220-253).
//   * "path_acl_config" (:58-59, :149-172): Aeson JSON object mapping path
//     -> [group name] (HashMap Text [Text]). Both JSON readers use
//     Aeson.decodeStrict' (aeson 2.2.3.0: an alias for decodeStrict since
//     2.2.0.0, Data/Aeson.hs:219-223), which requires end-of-input —
//     trailing non-whitespace makes it return Nothing (Decoding.hs:40-45).
//     serde_json::from_slice has the same EOF rule.
//
// INVENTORY (RULE §5): ZERO rows name this source file. Exact command, run
// at the repo root:
//   $ grep -n 'Database/Data\.hs' migration/inventory*.tsv
//   (no output; exit status 1)
// The construct rows that touch the codecs are followed by the owners they
// name, not here:
//   * RULE §2 "Data.Binary serializations feeding hashes" — these blobs do
//     not feed a hash, but the same hand-written-exact rule is applied
//     because an old DB must remain readable (FORMAT TRUTH above).
//   * RULE §1 JSON row — serde_json, with the key-order caveat in
//     DIVERGENCES 2.
//   * RULE §1 thrift row — the Compact codec is generated (ASSUMED IMPORT).
//   * inventory.tsv row 142 (generated internal.thrift module) — the
//     StoredSchema home.
//
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, checked not assumed):
//   * Outbound, edges.tsv:255-257 — Exception.hs, Storage.hs,
//     RTS/Foreign/Ownership.hsc. All shipped: exception.rs (row 53),
//     storage.rs (row 58), ownership.rs (row 57).
//   * Inbound, edges.tsv:196 (CompletePredicates.hs, row 117), :240
//     (Create.hs), :320 (Open.hs). None of the three is shipped yet; they
//     are the callers this surface must satisfy.
//   * Glean.Types (Repo) is the generated glean_thrift crate; Util.Log is
//     an external package (no edges.tsv row) — both ASSUMED IMPORTS.
//
// ASSUMED IMPORTS (RULE §4 B4):
//   * glean_thrift::compact — the generated Compact codec, pinned to
//     hsthrift's module (Thrift/Protocol/Compact.hs:38-42):
//         pub fn serialize_compact<T: ThriftStruct>(value: &T) -> Vec<u8>;
//         pub fn deserialize_compact<T: ThriftStruct>(bytes: &[u8])
//             -> Result<T, String>;
//     and impl ThriftStruct for StoredSchema emitted by the generator.
//     A different generated spelling is a Step-4 compile error, never a
//     silent divergence.
//   * glean_util::log::{log_warning, log_error} — the pinned log seam
//     (DECISIONS.md 2026-10-18; first sketch query/thrift/internal.rs:
//     190-213, log_error at catalog.rs:489-499):
//         pub fn log_warning(msg: impl FnOnce() -> String);
//         pub fn log_error(msg: impl FnOnce() -> String);
//     Util.Log.String.hs:56-63 always forces the message, so each closure
//     is forced immediately inside the call.
//
// DIVERGENCES (deliberate):
//   1. Data.Binary decode failures. The source's "msg" is binary-0.8.9.1's
//      Get failure string; the only failure this decoder can produce is
//      "not enough bytes" (Get.Internal's getByteString/readN), which is
//      the text used. A negative list count makes binary's getMany consume until the
//      input is exhausted and then fail with that same message; this
//      decoder fails immediately. A negative ByteString length is matched
//      exactly: getByteString returns empty for n <= 0 (:382-385).
//   2. Aeson JSON key order. Aeson's HashMap order and Rust's HashMap
//      iteration order differ, so the bytes of path_acl_config are not
//      byte-identical; both sides accept any object order and no consumer
//      depends on it (the value is parsed back into a map).
//   3. serde_json::to_vec returns Result though it cannot fail for a
//      string-keyed map; the source's Aeson.encode is total. The expect is
//      an impossible-path assertion, not a partial function of the input.
//   4. The Haskell db argument and Repo are taken by reference (&D,
//      &thrift::Repo); the source passes them by value but only reads them.
//   5. storeACLGroupMapping borrows its bytes instead of consuming a
//      ByteString (all callers pass a slice).
//   6. Aeson's FromJSON Word32 accepts any integral-valued JSON number
//      (parseIntegral/toBoundedInteger), including 3.0; serde_json's u32
//      rejects non-integer literals. Stored mappings are written as
//      integers, so only a hand-edited blob observes the difference (the
//      source accepts it; this port logs corruption and returns None).
//
// SIBLING-UNIT CONTRACTS (RULE §4; DONE siblings' real APIs read):
//   * crate::database::storage (row 58) — DatabaseOps::store/retrieve,
//     both Result<_, DbError>.
//   * crate::database::exception (row 53) — DbError, db_error,
//     db_exception; the FfiError -> DbError seam added by row 59.
//   * glean_core::rts::foreign::ownership (row 57) — Slice, UsetId,
//     serialize_slice -> Result<Vec<u8>, FfiError>, deserialize_slice ->
//     Result<Slice, FfiError>.
//   * crate::internal::types (row 142) — StoredSchema.
//
// PARTIAL FUNCTIONS: none. Every fallible step is a Result/Option; the one
// expect (DIVERGENCES 3) is unreachable for a string-keyed map.

//! The database's small key/value blobs: the stored schema (Thrift
//! Compact), the excluded units and stacked slices (Data.Binary), the ACL
//! group mapping and path ACL config (JSON), and the first-ACL-id boundary
//! (Data.Binary Word32).

use std::collections::HashMap;

use glean_core::rts::foreign::ownership::{deserialize_slice, serialize_slice, Slice, UsetId};
use glean_thrift as thrift;
use glean_thrift::compact::{deserialize_compact, serialize_compact};
use glean_util::log::{log_error, log_warning};

use crate::database::exception::{db_error, DbError};
use crate::database::storage::DatabaseOps;
use crate::internal::types::StoredSchema;

const SCHEMA_KEY: &[u8] = b"schema";
const UNITS_KEY: &[u8] = b"units";
const SLICES_KEY: &[u8] = b"slices";
const ACL_GROUP_MAPPING_KEY: &[u8] = b"acl_group_mapping";
const PATH_ACL_CONFIG_KEY: &[u8] = b"path_acl_config";
const FIRST_ACL_ID_KEY: &[u8] = b"first_acl_id";

/// The one failure text this Data.Binary decoder emits (see DIVERGENCES 1).
const NOT_ENOUGH_BYTES: &str = "not enough bytes";

/// 'storeSchema' (Data.hs:65-66): serialize the schema with Thrift Compact
/// under the "schema" key.
pub fn store_schema<D: DatabaseOps>(db: &D, schema: &StoredSchema) -> Result<(), DbError> {
    db.store(SCHEMA_KEY, &serialize_compact(schema))
}

/// 'retrieveSchema' (Data.hs:68-74): read the schema; a Compact failure is
/// a DbError prefixed "invalid schema: ".
pub fn retrieve_schema<D: DatabaseOps>(
    repo: &thrift::Repo,
    db: &D,
) -> Result<Option<StoredSchema>, DbError> {
    match db.retrieve(SCHEMA_KEY)? {
        Some(bytes) => match deserialize_compact::<StoredSchema>(&bytes) {
            Ok(schema) => Ok(Some(schema)),
            Err(msg) => db_error(repo, &format!("invalid schema: {}", msg)),
        },
        None => Ok(None),
    }
}

/// 'storeUnits' (Data.hs:76-77): Data.Binary [ByteString].
pub fn store_units<D: DatabaseOps>(db: &D, units: &[Vec<u8>]) -> Result<(), DbError> {
    db.store(UNITS_KEY, &encode_binary_bytestring_list(units))
}

/// 'retrieveUnits' (Data.hs:79-85): a Binary failure is a DbError prefixed
/// "invalid units: ".
pub fn retrieve_units<D: DatabaseOps>(
    repo: &thrift::Repo,
    db: &D,
) -> Result<Option<Vec<Vec<u8>>>, DbError> {
    match db.retrieve(UNITS_KEY)? {
        Some(bytes) => match decode_binary_bytestring_list(&bytes) {
            Ok(units) => Ok(Some(units)),
            Err(msg) => db_error(repo, &format!("invalid units: {}", msg)),
        },
        None => Ok(None),
    }
}

/// 'storeSlices' (Data.hs:90-93): serialize each slice with the C++
/// Slice::serialize, then Data.Binary [ByteString] the list.
pub fn store_slices<D: DatabaseOps>(db: &D, slices: &[Slice]) -> Result<(), DbError> {
    let mut bytestrings = Vec::with_capacity(slices.len());
    for slice in slices {
        bytestrings.push(serialize_slice(slice)?);
    }
    db.store(SLICES_KEY, &encode_binary_bytestring_list(&bytestrings))
}

/// 'retrieveSlices' (Data.hs:95-102): decode the list, then each slice.
/// A deserializeSlice failure propagates as the FFI exception (DbError's
/// SomeDBError leg), exactly as the source lets it escape.
pub fn retrieve_slices<D: DatabaseOps>(
    repo: &thrift::Repo,
    db: &D,
) -> Result<Option<Vec<Slice>>, DbError> {
    match db.retrieve(SLICES_KEY)? {
        Some(bytes) => match decode_binary_bytestring_list(&bytes) {
            Ok(bytestrings) => {
                let mut slices = Vec::with_capacity(bytestrings.len());
                for bs in &bytestrings {
                    slices.push(deserialize_slice(bs)?);
                }
                Ok(Some(slices))
            }
            Err(msg) => db_error(repo, &format!("invalid slices: {}", msg)),
        },
        None => Ok(None),
    }
}

/// 'storeACLGroupMapping' (Data.hs:106-107): store the JSON blob verbatim.
pub fn store_acl_group_mapping<D: DatabaseOps>(db: &D, bytes: &[u8]) -> Result<(), DbError> {
    db.store(ACL_GROUP_MAPPING_KEY, bytes)
}

/// 'retrieveACLGroupMapping' (Data.hs:113-124): parse {"group": uset_id}.
/// A stored-but-unparseable value is logged and reported as None, so
/// corruption is not silently mistaken for an empty mapping.
pub fn retrieve_acl_group_mapping<D: DatabaseOps>(
    db: &D,
) -> Result<Option<HashMap<String, u32>>, DbError> {
    match db.retrieve(ACL_GROUP_MAPPING_KEY)? {
        None => Ok(None),
        Some(bytes) => match parse_acl_group_mapping_json(&bytes) {
            Some(mapping) => Ok(Some(mapping)),
            None => {
                log_warning(|| {
                    "corrupted acl_group_mapping, ignoring stored ACL group mapping".to_string()
                });
                Ok(None)
            }
        },
    }
}

/// 'storeFirstACLID' (Data.hs:128-130): Data.Binary Word32 (4 bytes BE).
pub fn store_first_acl_id<D: DatabaseOps>(db: &D, id: UsetId) -> Result<(), DbError> {
    let UsetId(uid) = id;
    db.store(FIRST_ACL_ID_KEY, &uid.to_be_bytes())
}

/// 'retrieveFirstACLID' (Data.hs:136-145): a decode failure is logged as an
/// error (the boundary is security-relevant) and reported as None.
pub fn retrieve_first_acl_id<D: DatabaseOps>(db: &D) -> Result<Option<UsetId>, DbError> {
    match db.retrieve(FIRST_ACL_ID_KEY)? {
        None => Ok(None),
        Some(bytes) => match decode_binary_word32(&bytes) {
            Ok(uid) => Ok(Some(UsetId(uid))),
            Err(err_msg) => {
                log_error(|| format!("corrupted first_acl_id, ignoring boundary: {}", err_msg));
                Ok(None)
            }
        },
    }
}

/// 'storePathACLConfig' (Data.hs:149-153): Aeson JSON object path -> [group].
pub fn store_path_acl_config<D: DatabaseOps>(
    db: &D,
    config: &HashMap<String, Vec<String>>,
) -> Result<(), DbError> {
    // Aeson.encode is total; serde_json cannot fail for a string-keyed map
    // (DIVERGENCES 3).
    let bytes = serde_json::to_vec(config).expect("serializing a string-keyed map cannot fail");
    db.store(PATH_ACL_CONFIG_KEY, &bytes)
}

/// 'retrievePathACLConfig' (Data.hs:159-172): a stored-but-unparseable
/// value is logged and reported as None.
pub fn retrieve_path_acl_config<D: DatabaseOps>(
    db: &D,
) -> Result<Option<HashMap<String, Vec<String>>>, DbError> {
    match db.retrieve(PATH_ACL_CONFIG_KEY)? {
        None => Ok(None),
        Some(bytes) => match serde_json::from_slice::<HashMap<String, Vec<String>>>(&bytes).ok() {
            Some(config) => Ok(Some(config)),
            None => {
                log_warning(|| {
                    "corrupted path_acl_config, ignoring stored path ACL config".to_string()
                });
                Ok(None)
            }
        },
    }
}

/// 'parseACLGroupMappingJson' (Data.hs:179-180): Aeson.decodeStrict',
/// which requires end-of-input (FORMAT TRUTH).
fn parse_acl_group_mapping_json(bytes: &[u8]) -> Option<HashMap<String, u32>> {
    serde_json::from_slice(bytes).ok()
}

/// Data.Binary encode of [ByteString] (FORMAT TRUTH): Int64be count, then
/// per element an Int64be length and the bytes.
fn encode_binary_bytestring_list(items: &[Vec<u8>]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&(items.len() as i64).to_be_bytes());
    for item in items {
        out.extend_from_slice(&(item.len() as i64).to_be_bytes());
        out.extend_from_slice(item);
    }
    out
}

/// Data.Binary decodeOrFail of [ByteString]. The value is returned; the
/// source ignores the unconsumed rest and the offset, so trailing bytes are
/// accepted (FORMAT TRUTH, DIVERGENCES 1).
fn decode_binary_bytestring_list(bytes: &[u8]) -> Result<Vec<Vec<u8>>, String> {
    let mut cursor = 0usize;
    let count = read_binary_i64(bytes, &mut cursor)?;
    if count == 0 {
        return Ok(Vec::new());
    }
    if count < 0 {
        // binary's getMany would consume until exhaustion and fail with the
        // same message (DIVERGENCES 1).
        return Err(NOT_ENOUGH_BYTES.to_string());
    }
    let count = count as usize;
    // Every element needs at least its 8-byte length prefix.
    if count > (bytes.len() - cursor) / 8 {
        return Err(NOT_ENOUGH_BYTES.to_string());
    }
    let mut items = Vec::with_capacity(count);
    for _ in 0..count {
        let len = read_binary_i64(bytes, &mut cursor)?;
        // binary's getByteString returns empty for n <= 0 (Get.Internal:382).
        if len <= 0 {
            items.push(Vec::new());
            continue;
        }
        let len = len as usize;
        if bytes.len() - cursor < len {
            return Err(NOT_ENOUGH_BYTES.to_string());
        }
        items.push(bytes[cursor..cursor + len].to_vec());
        cursor += len;
    }
    Ok(items)
}

/// Data.Binary decodeOrFail of Word32 (4 bytes BE); trailing bytes ignored.
fn decode_binary_word32(bytes: &[u8]) -> Result<u32, String> {
    if bytes.len() < 4 {
        return Err(NOT_ENOUGH_BYTES.to_string());
    }
    let mut buf = [0u8; 4];
    buf.copy_from_slice(&bytes[..4]);
    Ok(u32::from_be_bytes(buf))
}

/// Read one Int64be (the Binary Int encoding, Class.hs:303).
fn read_binary_i64(bytes: &[u8], cursor: &mut usize) -> Result<i64, String> {
    if bytes.len() - *cursor < 8 {
        return Err(NOT_ENOUGH_BYTES.to_string());
    }
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&bytes[*cursor..*cursor + 8]);
    *cursor += 8;
    Ok(i64::from_be_bytes(buf))
}

// Coverage: 12/12 exports (Data.hs:9-22) — store/retrieveSchema,
// store/retrieveUnits, store/retrieveSlices, store/retrieveACLGroupMapping,
// store/retrieveFirstACLID, store/retrievePathACLConfig — and nothing else
// public. The six key constants and the five codec helpers are private like
// their sources (parseACLGroupMappingJson is private at :179).
//
// Marker breakdown (closed set — zero TODO(port) markers, zero BUG(port)
// markers, zero PERF(port) markers, zero INVENTORY-BUG-CANDIDATE markers):
//   * 0 TODO(port): the module is complete against the shipped siblings;
//     the generated Compact codec and the log seam are ASSUMED IMPORTS
//     (RULE §4 B4), not
//     deferrals.
//   * 0 BUG(port): no source defect is reproduced.
//   * 0 PERF(port): each encoder is a single pass.
//   * 0 INVENTORY-BUG-CANDIDATE: the file has zero inventory rows; the
//     construct rows it follows (RULE §2 Data.Binary, §1 JSON/thrift, row
//     142) are cited above.
// SAFETY count: 0 — no unsafe construct (the Slice codecs are safe
// wrappers in ownership.rs).
// PORT STATUS: confidence=medium todos=0
