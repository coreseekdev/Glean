// Ported from: glean/db/Glean/Database/ACLOwnership.hs (154 lines)
// Manifest unit: rust/glean-db/src/database/acl_ownership.rs
// (migration/manifest.tsv line 59, crate glean-db). RULE §4: one source
// file -> one target file; the manifest path is authoritative.
//
// TYPE-MAPPING TRUTH (this unit is pure algorithm over shipped handles):
//   * 'ACL' (ACLConfig.hs:47-49, imported here at :43) -> Acl, the
//     spelling acl_config.rs already owns (ACL -> Acl, the meta.rs AclMode
//     acronym re-casing); a newtype over Text -> tuple struct over String.
//   * 'Path' (imported at :42) -> Path; 'PathACLConfig' (imported at :41)
//     -> PathAclConfig (HashMap Path [ACL]).
//   * 'HashMap ACL UnitId' (:64) -> std HashMap<Acl, UnitId>.
//   * 'Word32' (:36, :123) -> u32; 'UnitId(..)' (:50) -> UnitId(pub u32).
//   * Text -> String, ByteString -> Vec<u8> (RULE §1).
//
// EXPORT MAPPING (1/1, module export list :26-28):
//   :27  augmentOwnershipWithACL -> augment_ownership_with_acl
// Nothing else is public: lookupGroupUnitId (:100-112) and
// buildUnitAssignments (:118-154) are private in the source and stay
// private here. The source's local scanPrefix (:142-154) is inlined into
// the fold so the decode stays deferred (see PARTIAL FUNCTIONS);
// decode_utf8_strict is a Rust-only helper for that partial function.
//
// INVENTORY (RULE §5): ZERO rows name this source file. Exact command, run
// at the repo root:
//   $ grep -n 'ACLOwnership' migration/inventory*.tsv
//   (no output; exit status 1)
// A broader 'grep -n ACL migration/inventory*.tsv' is likewise empty
// (exit 1), so the zero-hit claim is not an omission: no ACL row exists at
// all. The construct-level rows that DO govern the area are followed by
// the siblings this unit consumes, not here:
//   * inventory.tsv row 136 (Util/Some.hs 'Some c') — no existential is
//     used in this file; the ACL algorithm is fully monomorphic.
//   * inventory.tsv row 137 (Database/Types.hs 'Some DatabaseOps') — this
//     unit is generic over DatabaseOps and never unwraps a Some.
//   * inventory.tsv row 142 (internal.thrift generated types) — not
//     imported here.
//   * inventory.tsv row 24 (Exception.hs, EXC_TAXONOMY) — the DbError
//     funnel this unit uses; followed, no flag.
//   * inventory.tsv row 138 (group row: inlineVars error / partial
//     functions) — the class Text.decodeUtf8 belongs to; followed by
//     returning the SAME message as an explicit DbError, no flag.
//
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, checked not assumed):
//   * Outbound, edges.tsv:139-141 — ACLConfig.hs, Storage.hs,
//     RTS/Foreign/Ownership.hsc. All three are SHIPPED: acl_config.rs
//     (row 55), storage.rs (row 58, shipped at 311e6af), and
//     ownership.rs (row 57).
//   * Inbound, edges.tsv:194 — CompletePredicates.hs imports this module.
//     That unit (manifest line 117) is NOT shipped yet; it is the caller this
//     surface must satisfy (see SIBLING-UNIT CONTRACTS).
//   * Util.Log is an external package (hsthrift), so it has no edges.tsv
//     row; it is an ASSUMED import below.
//
// SIBLING-UNIT CONTRACTS (RULE §4; DONE siblings' real APIs read):
//   * crate::database::acl_config (row 55) — PathAclConfig, Path, Acl,
//     get_all_group_ids (:131-139), path_config_to_list (:119-124). Both
//     functions borrow the config and clone; the source retains it across
//     the two calls (:69 and :87), and so does this unit.
//   * crate::database::storage (row 58) — DatabaseOps, get_unit_id,
//     get_units_by_prefix. The latter returns Vec<(Vec<u8>, UnitId)>, the
//     raw name bytes the source decodes at :150.
//   * glean_core::rts::foreign::ownership (row 57) — ComputedOwnership,
//     UnitId, augment_with_acl (&ComputedOwnership, &[(u32, Vec<Vec<u32>>)])
//     -> Result<(), FfiError>. The source's 'augmentWithACL own
//     assignments' (:93) is that function; it short-circuits on an empty
//     assignment list, which is the source's own 'augmentWithACL _ []'
//     clause.
//   * crate::database::exception (row 53) — DbError. This unit adds the
//     From<FfiError> seam there (see DIVERGENCES 1).
//
// ASSUMED IMPORT (RULE §1 logging row (A10)): the glean_util::log seam.
// 'import Util.Log (logInfo)' (:38) has NO manifest unit. The convention
// (DECISIONS.md 2026-10-18 log-seam ruling; first sketch
// query/thrift/internal.rs:190-213, extended with logInfo at
// write/send_batch.rs:150-171) is a direct final call site against the
// pinned shape:
//     pub fn log_info(msg: impl FnOnce() -> String);
// Util.Log.String.hs:50-53 always forces its message, so each closure here
// is forced immediately inside the call; they capture only locals by
// reference, so no 'static demand arises. The module does not exist on
// disk yet; the swap when glean-util's log unit lands is import resolution
// only.
//
// PARTIAL FUNCTIONS (RULE §2; §1 strings row): ONE. 'Text.decodeUtf8
// nameBS' (:150) is strict: invalid UTF-8 throws a catchable
// UnicodeException. decode_utf8_strict returns DbError::Other carrying the
// text-2.0.2 message ("Cannot decode byte '\\xNN': Data.Text.Encoding:
// Invalid UTF-8 stream", first bad byte; the last byte for truncated
// input). The same text is shipped privately at
// query/thrift/internal.rs:1088-1109 and
// glean-typed/src/query/angle.rs:1085-1111, probed there against 16 invalid
// inputs; exotic invalid classes outside those inputs could position the
// reported byte differently (the decoders' DFA internals differ) — the
// message SHAPE is exact either way. It is a Result rather than a panic
// because the source exception is catchable and RULE §1 requires
// Text.Encoding errors to become explicit Result; the helper is copied
// rather than imported because those helpers are private and this unit has
// no depmap edge to the query area. Timing is preserved: the source's list
// comprehension is lazy, so every getUnitsByPrefix call runs before the
// fold forces the first decode; build_unit_assignments is two-phase for
// that reason (a later scan's DB error wins over an earlier bad name).
//
// DIVERGENCES (deliberate):
//   1. The source's IO exceptions and the FFI throw both funnel to DbError.
//      augment_with_acl returns Result<(), FfiError>; exception.rs gains
//      'impl From<FfiError> for DbError' (the SomeDBError leg), the same
//      additive form as unit 58's From<io::Error>.
//   2. The Haskell db argument is taken by reference (&D), matching every
//      DatabaseOps method (Storage.hs's class methods take db by value but
//      only borrow it).
//   3. 'HashMap.fromListWith (++)' (:126-127) prepends: for a unit that
//      appears in several config entries, insertWith applies f new old, so
//      the later entry's level list comes FIRST. The Rust fold uses
//      insert(0, ..) to preserve that order. HashMap iteration order is
//      unspecified on both sides, so the final toList order is not a
//      contract.
//   4. The source logs with String concatenation and 'show' of Int; the
//      format! calls reproduce the exact text (show Int == decimal).
//
// SIBLING-UNIT CONTRACTS FOR THE CALLER (CompletePredicates.hs, manifest
// line 117):
// the returned map is the source's 'groupMapping' (:56-58, :81) — every
// resolved group name to its "acl:<name>" unit id. The caller stores it;
// this unit does not. The augmentation mutates the ComputedOwnership
// handle in place through the C++ FFI (the Rust signature borrows it, like
// the source's 'own').

//! ACL ownership augmentation at database finalization time: resolve the
//! configured ACL group names to UnitIds, scan the ownership units by
//! directory prefix, and AND the resulting CNF into the computed
//! ownership through the C++ engine.

use std::collections::HashMap;

use glean_core::rts::foreign::ownership::{augment_with_acl, ComputedOwnership, UnitId};
use glean_util::log::log_info;

use crate::database::acl_config::{
    get_all_group_ids, path_config_to_list, Acl, Path, PathAclConfig,
};
use crate::database::exception::DbError;
use crate::database::storage::DatabaseOps;

/// 'augmentOwnershipWithACL' (ACLOwnership.hs:59-95): augment the computed
/// ownership with the ACL constraints from the accumulated PathACLConfig.
///
/// Returns the group-name -> UnitId mapping for storage by the caller. The
/// groups were registered as "acl:<name>" ownership units just before
/// computeOwnership (see syncCompletePredicates).
pub fn augment_ownership_with_acl<D: DatabaseOps>(
    db: &D,
    ownership: &ComputedOwnership,
    config: &PathAclConfig,
) -> Result<HashMap<Acl, UnitId>, DbError> {
    // Collect unique group names and look up their UnitIds.
    let all_group_names = get_all_group_ids(config);
    let mut group_unit_ids: Vec<(Acl, UnitId)> = Vec::new();
    for acl in &all_group_names {
        if let Some(pair) = lookup_group_unit_id(db, acl)? {
            group_unit_ids.push(pair);
        }
    }

    if group_unit_ids.len() != all_group_names.len() {
        log_info(|| {
            format!(
                "ACL augmentation WARNING: some groups not found as units. \
                 Expected {}, found {}",
                all_group_names.len(),
                group_unit_ids.len()
            )
        });
    }

    log_info(|| {
        format!(
            "ACL augmentation: {} groups resolved to UnitIds",
            group_unit_ids.len()
        )
    });

    // Build group name -> UnitId mapping. fromList's last-wins is
    // unreachable here because getAllGroupIds already deduplicates.
    let group_mapping: HashMap<Acl, UnitId> = group_unit_ids.into_iter().collect();
    let resolve_group_unit_id = |name: &Acl| group_mapping.get(name).copied();

    // Build per-unit ACL assignments by matching unit names against ACL
    // config directory prefixes.
    let assignments =
        build_unit_assignments(db, &path_config_to_list(config), &resolve_group_unit_id)?;

    log_info(|| {
        format!(
            "ACL augmentation: {} units matched ACL prefixes",
            assignments.len()
        )
    });

    // Call C++ augmentation.
    augment_with_acl(ownership, &assignments)?;

    Ok(group_mapping)
}

/// 'lookupGroupUnitId' (ACLOwnership.hs:100-112): look up the UnitId for an
/// ACL group. Each group is registered in the DB as an "acl:<name>"
/// ownership unit just before computeOwnership. Returns None (and logs)
/// when the group has no corresponding unit.
fn lookup_group_unit_id<D: DatabaseOps>(
    db: &D,
    acl: &Acl,
) -> Result<Option<(Acl, UnitId)>, DbError> {
    let unit_name = format!("acl:{}", acl.0);
    match db.get_unit_id(unit_name.as_bytes())? {
        Some(uid) => Ok(Some((acl.clone(), uid))),
        None => {
            log_info(|| format!("ACL augmentation: group unit not found: {}", acl.0));
            Ok(None)
        }
    }
}

/// 'buildUnitAssignments' (ACLOwnership.hs:118-154): build per-unit ACL
/// assignments using prefix scans. For each config entry, scan the
/// ownership units by prefix, then group results by UnitId.
///
/// Two-phase on purpose: the source's list comprehension in scanPrefix is
/// lazy, so mapM processEntry runs every getUnitsByPrefix before the fold
/// forces any decode. A DB error from a later scan therefore wins over an
/// invalid UTF-8 name from an earlier one, and this structure reproduces
/// that timing.
fn build_unit_assignments<D: DatabaseOps, F: Fn(&Acl) -> Option<UnitId>>(
    db: &D,
    config_entries: &[(Path, Vec<Acl>)],
    resolve_group: F,
) -> Result<Vec<(u32, Vec<Vec<u32>>)>, DbError> {
    // Phase 1: every scan, no decoding.
    let mut pending: Vec<(String, Vec<u32>, Vec<(Vec<u8>, UnitId)>)> = Vec::new();
    for (path, group_names) in config_entries {
        let group_unit_ids: Vec<u32> = group_names
            .iter()
            .filter_map(|name| resolve_group(name))
            .map(|unit| unit.0)
            .collect();
        if group_unit_ids.is_empty() {
            continue;
        }
        let matches = db.get_units_by_prefix(path.0.as_bytes())?;
        pending.push((path.0.clone(), group_unit_ids, matches));
    }

    // Phase 2: decode and filter in the source's fold order.
    //
    // The per-(unit, level) clone of group_unit_ids is forced by Rust
    // ownership and augment_with_acl's owned Vec<Vec<u32>> signature (the
    // source shares one list per config entry); like acl_config.rs's forced
    // clone it is not a deferred optimization and gets no PERF marker.
    //
    // HashMap.fromListWith (++) applies f new old, so a later entry's level
    // list precedes the earlier ones for the same unit; pushing in fold
    // order and reversing each unit's list reproduces that without a front
    // insertion (see DIVERGENCES 3).
    let mut grouped: HashMap<u32, Vec<Vec<u32>>> = HashMap::new();
    for (prefix, group_unit_ids, matches) in pending {
        let prefix_slash = format!("{}/", prefix);
        for (name_bs, unit) in matches {
            let name = decode_utf8_strict(&name_bs)?;
            if !name.starts_with("acl:") && (name == prefix || name.starts_with(&prefix_slash)) {
                grouped
                    .entry(unit.0)
                    .or_default()
                    .push(group_unit_ids.clone());
            }
        }
    }
    for levels in grouped.values_mut() {
        levels.reverse();
    }
    Ok(grouped.into_iter().collect())
}

/// 'Text.decodeUtf8' (Data.Text.Encoding) for a byte string: valid UTF-8
/// becomes a String; invalid UTF-8 returns the source's catchable
/// UnicodeException as DbError::Other. The message is byte-exact against
/// text-2.0.2 (broken sequences report the first bad byte, truncated-at-end
/// inputs the LAST byte), the same text shipped privately at
/// glean-typed/src/query/angle.rs:1085-1111 (the SHAPE is exact; exotic
/// invalid classes outside the 16 probed inputs could position the byte
/// differently). Copied, not imported: those helpers are private and this
/// unit has no depmap edge to the query area.
fn decode_utf8_strict(b: &[u8]) -> Result<String, DbError> {
    match std::str::from_utf8(b) {
        Ok(s) => Ok(s.to_string()),
        Err(e) => {
            let offending = match e.error_len() {
                // Broken sequence: text reports its first byte.
                Some(_) => b[e.valid_up_to()],
                // Unexpected end of input: text reports the last byte.
                None => b[b.len() - 1],
            };
            Err(DbError::Other(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "Cannot decode byte '\\x{:02x}': Data.Text.Encoding: Invalid UTF-8 stream",
                    offending
                ),
            ))))
        }
    }
}

// Coverage: 1/1 exports (augment_ownership_with_acl); the two private
// helpers and decode_utf8_strict are private like their sources (decode is
// Rust-only, see PARTIAL FUNCTIONS).
//
// Marker breakdown (closed set — zero TODO(port) markers, zero BUG(port)
// markers, zero PERF(port) markers, zero INVENTORY-BUG-CANDIDATE markers):
//   * 0 TODO(port): the module is complete against the shipped siblings.
//   * 0 BUG(port): no source defect is reproduced.
//   * 0 PERF(port): the per-(unit, level) clone of group_unit_ids is
//     forced by Rust ownership and augment_with_acl's owned signature (the
//     source shares one list); no avoidable copy is added — the fold
//     pushes and reverses instead of front-inserting.
//   * 0 INVENTORY-BUG-CANDIDATE: the file has zero inventory rows; the
//     construct rows that could apply (24/136/137/138/142) are followed.
// SAFETY count: 0 — no unsafe construct (the FFI call is inside
// ownership.rs's augment_with_acl).
// PORT STATUS: confidence=high todos=0
