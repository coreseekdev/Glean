// Ported from: glean/util/Glean/Impl/MemoryReader.hs (16 lines).
// Manifest unit: rust/glean-util/src/impl/memory_reader.rs
// (migration/manifest.tsv line 68, crate glean-util). RULE §4: one source
// file -> one target file; the manifest path is authoritative.
//
// WHAT THIS FILE IS: the OSS build's memory-capacity probe. The upstream
// file is a TEMPLATE, not an implementation: its entire body is
// `totalMemCapacityKB = return Nothing` under the comment "template for open
// source build / not implemented yet" (:13-14). The non-OSS build supplies
// the real implementation (the comment's wording), and that code is not in
// this tree; the OSS tree is the migration's spec (RULE §0), so this port is
// the stub — it answers None unconditionally and must not be "improved" into
// a real probe (see FIDELITY).
//
// ---------------------------------------------------------------------------
// INVENTORY (RULE §5; migration/inventory*.tsv)
// ---------------------------------------------------------------------------
//
// ZERO rows name this source file. Exact command at the repo root:
//   $ grep -n 'MemoryReader' migration/inventory*.tsv
//   (no output; exit status 1)
// Repeated across all six inventory files (inventory.tsv,
// inventory.sites.tsv, inventory.codeflow.tsv, inventory.ffi.tsv,
// inventory.protocol.tsv, inventory.stm.tsv). A broader
// `grep -rn 'MemoryReader' migration/depmap migration/manifest.tsv` finds
// only the manifest row (line 68), the topological order (order.txt:67) and
// the two depmap edges (edges.tsv:428, :443) — no inventory row. (The same
// grep over all of migration/ additionally matches HANDOFF.md:151, :380,
// which are prose.)
//
// There is no construct-level row to apply either: the file has one total IO
// action and no partial function, no exception, no existential, no STM and no
// FFI. No inventory divergence applies.
//
// ---------------------------------------------------------------------------
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, checked not assumed)
// ---------------------------------------------------------------------------
//
//   * OUTBOUND: none. The source imports nothing at all — it is a module
//     header (:9-11), two comment lines (:13-14) and one definition (:15-16).
//     `Maybe` comes from the Prelude, not an import.
//   * INBOUND (2), both future units:
//       edges.tsv:428 Storage/LMDB.hs (manifest line 70, NOT shipped) imports
//         the module at LMDB.hs:42 and calls it at :55.
//       edges.tsv:443 Storage/RocksDB.hs (manifest line 72, NOT shipped)
//         imports it at RocksDB.hs:49 and calls it at :94.
//     glean/test/tests/StorageRocksDBTest.hs:20,46,95 is the third caller but
//     tests are not translated (RULE §4 tests row).
//
// ---------------------------------------------------------------------------
// ASSUMED IMPORTS (RULE §4 B4)
// ---------------------------------------------------------------------------
//
//   * NONE — the file needs no generated type, no thrift image and no
//     cross-crate import.
//
// ---------------------------------------------------------------------------
// TRANSLATION NOTES
// ---------------------------------------------------------------------------
//
//   * `totalMemCapacityKB :: IO (Maybe Int)` (:15-16) ->
//     `pub fn total_mem_capacity_kb() -> Option<i64>`:
//       - `Maybe Int` -> `Option<i64>` (RULE §2's numeric-conversions row,
//         RULEBOOK.md:81; GHC `Int` is 64-bit — the mapping recorded at
//         util/service.rs:139 and database/backup/backend.rs:109, and
//         already visible in the consumer trait storage.rs:373).
//       - `IO` adds no failure channel here. The body is `return Nothing`,
//         which cannot throw, so the Rust signature carries no `Result` —
//         the same ruling as glean-core's fact_set.rs:567-582
//         (`factCount :: FactSet -> IO Int` -> `pub fn fact_count(&self) ->
//         usize`, "No `Result` — there is nothing to fail"). Callers get
//         exactly the `Option` the source's type promises.
//   * Naming: `totalMemCapacityKB` -> `total_mem_capacity_kb` (the acronym
//     run lowercases; the consumers' own field names are `mem_capacity` and
//     `ramKB` in the source, and the Rust spelling keeps the trailing `kb`
//     rather than inventing `kib`).
//   * Units: the value is KiB — both consumers multiply by 1024
//     (LMDB.hs:59-60, RocksDB.hs:105-106). The name is the source's; do not
//     "correct" KB to KiB.
//
// ---------------------------------------------------------------------------
// FIDELITY
// ---------------------------------------------------------------------------
//
//   * DO NOT implement a real reader. There is no OSS code to translate: a
//     /proc/meminfo parse, a sysconf call, or a sysinfo crate would all be
//     NEW behavior, and the OSS build's observable contract is "capacity
//     unknown". The stub is load-bearing rather than dead: `Nothing` is what
//     selects the consumers' fallback paths (RocksDB.hs:86-90 falls back to
//     `config_db_rocksdb_cache_mb`, and both records leave the virtual disk
//     limit at Nothing).
//   * The function is deliberately neither `const` nor cached: the source is
//     a plain IO action evaluated per `newStorage` call. A Rust `const`
//     would change nothing observable but would misstate the shape.
//
// ---------------------------------------------------------------------------
// SIBLING-UNIT CONTRACTS
// ---------------------------------------------------------------------------
//
//   * row 70 (storage/lmdb.rs): binds `let mem_capacity =
//     total_mem_capacity_kb();` (LMDB.hs:55) and derives
//     `lmdb_max_disk_size: Option<i64>` (source `Maybe Int`, LMDB.hs:46) as
//     `None` when either the capacity or
//     `config_db_rocksdb_disk_mem_capacity_ratio_limit` is `Nothing`, else
//     `fromIntegral ratio * (mem * 1024)` (LMDB.hs:58-61). That ratio is a
//     generated `Maybe Int` (server_config.thrift:327, `optional i32`), so
//     the source's `fromIntegral` is the identity on the generated Int.
//   * row 72 (storage/rocks_db.rs — the manifest's underscore spelling,
//     line 72): the same binding (RocksDB.hs:94) feeds
//     `cacheSize` (RocksDB.hs:86-90) — when both are present it is
//     `round (fromIntegral ramKB * ratio * 1024)` and the ratio is a
//     generated `Maybe Float` (server_config.thrift:332, `optional float`),
//     so ALL of that arithmetic is f32 and `round` is ties-to-even; the Rust
//     must compute in `f32` and use `round_ties_even`, not `f32::round`
//     (ties away from zero). Otherwise the fallback is
//     `config_db_rocksdb_cache_mb * 1024 * 1024`. `rocksMaxDiskSize`
//     (`Maybe Int`, RocksDB.hs:71) uses the same expression as LMDB's
//     (RocksDB.hs:104-107).
//   * No other in-tree consumer exists; nothing else imports this module.
//
// MARKERS: zero of every class — 0 port todos, 0 source-defect
// reproductions, 0 performance deferrals, 0 inventory divergences, 0 SAFETY
// notes (no unsafe code).
//
// PORT STATUS: confidence=high todos=0

/// `totalMemCapacityKB :: IO (Maybe Int)` (MemoryReader.hs:15-16): the
/// machine's RAM in KiB, or None when the platform/build cannot tell.
///
/// The OSS source is a template whose body is `return Nothing` (:16), so
/// this port returns None unconditionally. The IO is total — no `Result`.
pub fn total_mem_capacity_kb() -> Option<i64> {
    // `return Nothing` (MemoryReader.hs:16). Deliberately not a real probe:
    // the OSS tree contains none, and RULE §0 makes the source the spec.
    None
}
