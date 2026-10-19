// Ported from: glean/github/Glean/Database/AclKnobs.hs (30 lines).
// Manifest unit: rust/glean-db/src/database/acl_knobs.rs
// (migration/manifest.tsv line 76). RULE §4: one source file -> one target
// file; the manifest path is authoritative.
//
// WHAT THIS FILE IS: the OSS stub for the Facebook-internal ACL JustKnobs
// (:9-13). The FB build supplies the real implementation from the
// same-named module under glean/facebook; the OSS build answers False to all
// three questions, which disables ACL handling end to end: no ownership
// augmentation at `glean complete` (:20-22), no query-time ACL filtering
// (:24-26), and no `--acl-groups` against a local DB (:28-30).
//
// ---------------------------------------------------------------------------
// INVENTORY (RULE §5; migration/inventory*.tsv)
// ---------------------------------------------------------------------------
//
// ZERO rows name this source file:
//   $ grep -c 'AclKnobs' migration/inventory*.tsv
//   inventory.codeflow.tsv:0  inventory.ffi.tsv:0  inventory.protocol.tsv:0
//   inventory.sites.tsv:0     inventory.stm.tsv:0  inventory.tsv:0
// The three knobs are plain IO actions over a constant; no state, no
// exception, no config read, no FFI — no other inventory class applies.
//
// ---------------------------------------------------------------------------
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, checked not assumed)
// ---------------------------------------------------------------------------
//
//   * OUTBOUND: none (`awk -F'\t' '$1=="glean/github/Glean/Database/
//     AclKnobs.hs"' migration/depmap/edges.tsv` is empty; the file imports
//     nothing at all).
//   * INBOUND (2), none edited by this unit:
//       edges.tsv:229 Config.hs (manifest line 85, NOT shipped):
//         cfgAclCalculateEnabled = aclCalculateEnabled and
//         cfgAclCheckEnabled = aclCheckEnabled (Config.hs:254-255) — the
//         ACTIONS are stored, not their results.
//       edges.tsv:1772 GleanCLI/Query.hs (NOT a manifest unit in this
//         crate's range): `allowed <- aclLocalTestingAllowed`
//         (Query.hs:138).
//
// ---------------------------------------------------------------------------
// ASSUMED IMPORTS (RULE §4 B4)
// ---------------------------------------------------------------------------
//
//   None. The port needs nothing beyond `std`.
//
// ---------------------------------------------------------------------------
// TRANSLATION NOTES
// ---------------------------------------------------------------------------
//
//   * `aclCalculateEnabled :: IO Bool` (:21), `aclCheckEnabled` (:25),
//     `aclLocalTestingAllowed` (:29) -> three `pub fn …() ->
//     bool`. IO -> the plain return type because the OSS action is TOTAL
//     (`return False`): there is no exception to report and no effect to
//     sequence, so a `Result` would invent an error channel (row 68's
//     `total_mem_capacity_kb` precedent).
//   * They stay FUNCTIONS, not `const`s and not a cached `static`: the
//     source is an IO action re-run at each use, and the FB build's body
//     reads live knobs. A `const` would be observably different the moment
//     the real implementation replaces this stub, and the call sites would
//     not change — the whole point of the stub is that they can't.
//
// ---------------------------------------------------------------------------
// DIVERGENCES (deliberate)
// ---------------------------------------------------------------------------
//
//   None. Three constant-valued total actions map to three functions; the
//   `IO` disappearance is the §2 total-action mapping, not a behaviour
//   change (nothing can observe when a False is produced).
//
// ---------------------------------------------------------------------------
// SIBLING-UNIT CONTRACTS
// ---------------------------------------------------------------------------
//
//   * database/config.rs (row 85): `cfgAclCalculateEnabled`/
//     `cfgAclCheckEnabled` are `IO Bool` VALUES in the source
//     (Config.hs:254-255 store the actions, not their results). The Rust
//     fields must therefore be `fn() -> bool` (a plain fn pointer), not an
//     eagerly-evaluated `bool`: a bool would freeze the answer at Config
//     construction, while the source re-runs the knob on every read.
//   * glean-cli's Query.hs:138 consumer is outside this crate's manifest
//     range; it will call `acl_local_testing_allowed()` the same way.
//
// MARKERS: zero of every class — 0 port todos, 0 source-defect
// reproductions, 0 performance deferrals, 0 inventory divergences, 0 SAFETY
// notes (no unsafe code).

/// 'aclCalculateEnabled :: IO Bool' (:21): whether ACL ownership
/// augmentation runs at `glean complete`. False in the OSS build.
pub fn acl_calculate_enabled() -> bool {
    false
}

/// 'aclCheckEnabled :: IO Bool' (:25): whether the query server filters
/// results by ACL group membership. False in the OSS build.
pub fn acl_check_enabled() -> bool {
    false
}

/// 'aclLocalTestingAllowed :: IO Bool' (:29): whether `--acl-groups` may
/// be used against a local DB. False in the OSS build.
pub fn acl_local_testing_allowed() -> bool {
    false
}

// PORT STATUS: confidence=high todos=0
