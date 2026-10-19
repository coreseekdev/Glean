// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// Ported from: glean/db/Glean/Database/ACLConfig.hs (61 lines).
// Manifest unit: rust/glean-db/src/database/acl_config.rs
// (migration/manifest.tsv line 55, crate glean-db). RULE §4: one source
// file -> one target file; the manifest path is authoritative.
//
// WHAT THIS FILE IS: ACL configuration validation and processing
// (ACLConfig.hs:12-24). The module handles the path -> ACL group IDs
// configuration supplied at `glean create`: each directory path maps to a
// list of ACL group ID strings, and the map is assumed small — fewer than
// 100 entries (:16-17). The source doc's create/complete flow (KickOff
// carries the map; `glean complete` reads it back and registers each group
// as an `acl:<name>` ownership unit; the server builds ACL ownership from
// those unit IDs, :19-24) is context only here — the flow lives in its
// consumer units (ACLOwnership.hs, CompletePredicates.hs).
//
// INVENTORY (RULE §5) — ZERO rows for this source file in every inventory
// file, checked not assumed:
//   $ for f in migration/inventory*.tsv; do
//       printf '%s ' "$f"; grep -c 'Database/ACLConfig\.hs' "$f"; done
//   migration/inventory.codeflow.tsv 0
//   migration/inventory.ffi.tsv 0
//   migration/inventory.protocol.tsv 0
//   migration/inventory.sites.tsv 0
//   migration/inventory.stm.tsv 0
//   migration/inventory.tsv 0
// A second pass for other spellings (`ACLConfig`, `acl_config`, `PathACL`)
// over the same glob also returns nothing. No INVENTORY-BUG-CANDIDATE
// applies: this unit has no ownership/lifetime, effect-order, exception,
// laziness, STM or JSON decision to look up — two newtypes over String,
// one type alias and two collection folds.
//
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, checked not assumed):
//   * Outbound: NONE. The source imports only Data.HashMap.Strict,
//     Data.HashSet, Data.Hashable and Data.Text (:35-39) — containers/base,
//     no manifest unit.
//   * Inbound: exactly two rows, and NEITHER consumer is edited by this
//     unit:
//       edges.tsv:139  ACLOwnership.hs        -> ACLConfig.hs
//       edges.tsv:193  CompletePredicates.hs  -> ACLConfig.hs
//     ACLOwnership.hs imports the module at :40-45 and uses
//     getAllGroupIds (:69) and pathConfigToList (:87); CompletePredicates.hs
//     imports it at :47-51, builds a PathACLConfig with the `Path`/`ACL`
//     constructors (:143) and uses getAllGroupIds (:156). Those files land
//     as their own manifest rows; this unit only defines the surface they
//     import.
//
// EXPORT MAPPING (5/5 source exports, ACLConfig.hs:26-33):
//   :27  Path(..)          -> `pub struct Path(pub String)` + derives
//   :28  ACL(..)           -> `pub struct Acl(pub String)` + derives
//   :29  PathACLConfig     -> `pub type PathAclConfig = HashMap<Path, Vec<Acl>>`
//   :30  pathConfigToList  -> `pub fn path_config_to_list`
//   :31  getAllGroupIds    -> `pub fn get_all_group_ids`
//
// TRANSLATION NOTES:
//   * Text -> String (RULE §1). Both newtypes wrap one Text field; the
//     tuple field is public in each because the source exports `Path(..)`
//     and `ACL(..)` (:27-28) and CompletePredicates.hs:143 applies both
//     constructors.
//   * Acronym re-casing: `ACL` -> `Acl`, the meta.rs:426-428 AclMode
//     precedent (house Rust UpperCamel; the file name is already
//     acl_config.rs). `Path` is not an acronym and keeps its spelling.
//   * Hashable -> `Hash` on both newtypes. `Clone` is mechanical: the map
//     is shared by reference and `path_config_to_list` hands back owned
//     pairs.
//   * `Path` derives `PartialOrd`/`Ord` because the source does (:43).
//     `Acl` does NOT: the source derives only Show and Eq plus Hashable
//     (:48-49). `Acl` is a value element and the HashSet element of
//     get_all_group_ids (:61), which needs Hash + Eq, never Ord; adding Ord
//     would be a wider contract than the source's.
//   * Show -> Debug (RULE §2). Both types derive Show in the source; Debug
//     is the stand-in. The punctuation differs inherently — Haskell prints
//     `Path "x"`, Rust prints `Path("x")` — the row-54 EntryAlreadyExists
//     precedent (catalog.rs:375-380). No custom Display is needed: no
//     consumer shows a Path or an Acl; CompletePredicates.hs:156 only
//     pattern-matches `ACL g`, which becomes a field access here.
//   * The two collection orders are unspecified on BOTH sides, and no order
//     contract is added or lost: Data.HashMap.toList vs std HashMap
//     iteration; Data.HashSet.toList vs std HashSet::into_iter. Consumers
//     treat get_all_group_ids as a set — ACLOwnership.hs:69-70 takes its
//     length and mapM's over it, CompletePredicates.hs:156 maps it to unit
//     names — and the group -> UnitId mapping is stored in a HashMap
//     (ACLOwnership.hs:63), so no consumer depends on the order.
//   * path_config_to_list borrows and clones. The source is persistent:
//     HashMap.toList does not consume its argument, and callers keep the
//     config afterwards (ACLOwnership.hs:69 reads it, :87 lists it). The
//     clone is forced by Rust ownership, not a deferred optimization —
//     hence no PERF marker.

use std::collections::{HashMap, HashSet};

/// Source: Path (:42-44) — "A directory path used as a key in the ACL
/// configuration."
///
/// `Show, Eq, Ord` (:43) -> `Debug, PartialEq, Eq, PartialOrd, Ord`;
/// `Hashable` -> `Hash`; `Clone` per TRANSLATION NOTES.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Path(pub String);

/// Source: ACL (:47-49) — "An ACL group ID string" (source example: 1, 2).
///
/// `Show, Eq` (:48) -> `Debug, PartialEq, Eq`; `Hashable` -> `Hash`.
/// NO `PartialOrd`/`Ord`: the source does not derive them (:48).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Acl(pub String);

/// Source: PathACLConfig (:53) — `HashMap Path [ACL]`.
pub type PathAclConfig = HashMap<Path, Vec<Acl>>;

/// Source: pathConfigToList (:56-57) — `HashMap.toList`.
///
/// Borrows and clones; the source's HashMap is persistent and the caller
/// retains it (ACLOwnership.hs:69, then :87).
pub fn path_config_to_list(config: &PathAclConfig) -> Vec<(Path, Vec<Acl>)> {
    config
        .iter()
        .map(|(path, acls)| (path.clone(), acls.clone()))
        .collect()
}

/// Source: getAllGroupIds (:60-61) — `HashSet.toList . HashSet.fromList .
/// concat . HashMap.elems`.
///
/// Flatten every value, deduplicate through a HashSet, collect. Order is
/// unspecified on both sides (TRANSLATION NOTES).
pub fn get_all_group_ids(config: &PathAclConfig) -> Vec<Acl> {
    config
        .values()
        .flatten()
        .cloned()
        .collect::<HashSet<Acl>>()
        .into_iter()
        .collect()
}

//
// Coverage: five items — Path, Acl, PathAclConfig, path_config_to_list,
// get_all_group_ids — matching the five source exports (:27-31) and
// nothing else: no private helpers, no re-exports.
//
// Marker breakdown (closed set — zero of every class): no port todos (the
// translation is complete), no source-defect reproductions (the source has
// no defect), no performance deferrals (the one clone is forced by
// ownership, documented above), no inventory divergence (zero inventory
// rows exist for this file), no unsafe.
//
// PORT STATUS: confidence=high todos=0
