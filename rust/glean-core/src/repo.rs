// Rust-only scaffolding: this directory module has no Haskell source.
// It exists because RULE §4 maps one source file to one target file and
// the source's module path (Glean.Repo.Text) nests under a `repo/` area
// that does not yet have a Rust parent module. It declares exactly one
// child, `pub mod text;`, so rust/glean-core/src/repo/text.rs can be
// reached as `crate::repo::text` once the crate root exists.
//
// Step-4 contract (HANDOFF §2 and §4.2): glean-core has NO lib.rs and NO
// Cargo.toml — Step 4 owns the crate scaffold. The eventual crate root
// must add:
//
//     pub mod repo;
//
// That line is deliberately NOT created here; this unit creates only
// repo.rs (and repo/text.rs), never lib.rs or Cargo.toml.
//
// Coverage: manifest line 51 (glean/hs/Glean/Repo/Text.hs ->
// rust/glean-core/src/repo/text.rs) is the sole member of this area.
// No markers: no port todos, no performance deferrals, no source-defect
// reproductions; the child's trailer records the port status.

pub mod text;

// PORT STATUS: confidence=high todos=0
