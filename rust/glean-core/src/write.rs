// Rust-only scaffolding: this directory module has no Haskell source.
// It exists because RULE §4 maps one source file to one target file and
// the source's module path (Glean.Write.Stats, manifest row 101 ->
// src/write/stats.rs) nests under a `write/` area that has no Rust parent
// module in glean-core. It declares exactly one child, `pub mod stats;`,
// so rust/glean-core/src/write/stats.rs can be reached as
// `crate::write::stats` once the crate root exists.
//
// Step-4 contract (HANDOFF §2 and §4.2; the repo.rs / rts/foreign.rs
// precedent): glean-core has NO lib.rs and NO Cargo.toml — Step 4 owns
// the crate scaffold, and neither exists as of this unit (grep-verified:
// rust/glean-core holds only src/ files and directories; rts.rs does not
// even declare its own `pub mod foreign;` child — that consolidation
// wiring is recorded as Step-4 debt in rts/foreign.rs's provenance note).
// The eventual crate root must add:
//
//     pub mod write;
//
// That line is deliberately NOT created here; this unit creates only
// write.rs (and write/stats.rs), never lib.rs or Cargo.toml.
//
// Coverage (grep 'Glean/Write/' migration/manifest.tsv): row 101 is the
// ONLY manifest row targeting a glean-core write/ path — every other
// Glean/Write/*.hs row lands in a different crate (glean-db
// src/write/send_batch.rs row 46 and src/write/json.rs row 115;
// glean-client src/write/{send_queue,async,send_and_rebase_queue,
// options}.rs rows 157-160 and src/write/simple_async.rs row 315). So
// this module stays a one-child directory module permanently unless a
// later manifest amendment moves rows in; no deferred sibling contracts
// are recorded here.

pub mod stats;

// PORT STATUS: confidence=high todos=0
