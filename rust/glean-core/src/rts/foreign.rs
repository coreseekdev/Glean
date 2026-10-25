// Module wiring for the rts/foreign units (RULE §4: one source file ->
// one target file; the manifest paths are authoritative).
//
// PROVENANCE NOTE (recorded because this file did NOT exist when unit
// manifest.tsv line 94 — RTS/Foreign/Query.hsc -> query.rs — was
// implemented): the unit brief for manifest line 94 instructed "append
// `pub mod query;` to rust/glean-core/src/rts/foreign.rs, +1 line".
// That file was ABSENT from the tree (grep-verified across rust/ —
// glean-core has no lib.rs/Cargo.toml yet and no `pub mod foreign;`
// declaration anywhere; the ten shipped units below are standalone
// drafts whose cross-references, e.g. define.rs's
// `use crate::rts::foreign::inventory::Inventory;`, anticipate exactly
// this wiring). The file was therefore CREATED by that unit with the
// full module set — the ten already-shipped siblings (bytecode, define,
// fact_set, inventory, lookup, lookup_cache, ownership, stacked, stats,
// subst) in the alphabetical order of the directory listing, plus
// `query` inserted in that same alphabetical position — so that the
// brief's end state ("the current pub mod set + query") is realized
// verbatim. The `pub mod foreign;` declaration that will hang this
// module off rts.rs belongs to the Step-4 consolidation wiring, like
// glean-core's lib.rs itself; nothing here pre-empts it.

pub mod bytecode;
pub mod define;
pub mod fact_set;
pub mod inventory;
pub mod lookup;
pub mod lookup_cache;
pub mod ownership;
pub mod query;
pub mod stacked;
pub mod stats;
pub mod subst;

// PORT STATUS: confidence=high todos=0
