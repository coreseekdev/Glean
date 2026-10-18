// Ported from: glean/hs/Glean/Repo/Text.hs (72 lines).
// Manifest unit: rust/glean-core/src/repo/text.rs (migration/manifest.tsv
// line 51, crate glean-core). RULE §4: one source file -> one target file;
// the manifest path is authoritative. This file is the first member of a
// NEW `repo/` area in glean-core; see repo.rs for the directory module.
//
// INVENTORY (RULE §5) — exact command, run at the repo root:
//   $ grep -n -E 'Repo/Text\.hs|showRepo|showRepoSep|repoToText|
//       repoToTextSep|readRepo|parseRepo|parseRepoText' migration/inventory*.tsv
//   (no output; exit status 1)
// Zero rows across inventory.tsv, inventory.ffi.tsv, inventory.sites.tsv,
// inventory.codeflow.tsv, inventory.protocol.tsv and inventory.stm.tsv.
// The source is eight pure string functions: no ownership, effect order,
// laziness, STM or JSON decision to look up, and no INVENTORY-BUG-
// CANDIDATE. The one partial-function decision (the empty-separator
// panic) is documented in FIDELITY NOTES; no inventory row governs it.
//
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, checked not assumed):
//   * Outbound edges: NONE. `awk -F'\t' '$1=="glean/hs/Glean/Repo/Text.hs"'
//     migration/depmap/edges.tsv` returns 0 lines. The source's three
//     imports are Data.Maybe (:21) and Data.Text (:22-23) — base/text, not
//     manifest units — plus Glean.Types (:25), which is generated thrift
//     (glean/if/glean.thrift:106-109) and therefore has no manifest unit
//     and no depmap node (RULE §4 B4 assumed-imports). `grep -n
//     'Repo/Text' migration/depmap/edges.tsv` returns exactly the 11
//     inbound rows below, so the file sits at a depmap source with no
//     outgoing module edge.
//   * Inbound edges (11), by edges.tsv line: 78 client/hs/Glean.hs;
//     110 client/hs/Glean/Repo.hs; 112 client/hs/Glean/Util/ShellPrint.hs;
//     164 db/Glean/Database/Backup/Locator.hs; 168
//     db/Glean/Database/Backup/Mock.hs; 208
//     db/Glean/Database/CompletePredicates.hs; 305
//     db/Glean/Database/Janitor.hs; 336 db/Glean/Database/Open.hs; 352
//     db/Glean/Database/Repo.hs; 419 db/Glean/Database/Storage/DB.hs;
//     434 db/Glean/Database/Storage/Memory.hs.
//     Verified call sites: Repo.hs:148/:151 showRepo; ShellPrint.hs:57
//     (import) and :212-224 showRepo; Locator.hs:51
//     parseRepoTextSep "." and :59 repoToTextSep "."; Mock.hs:89
//     showRepoSep "."; CompletePredicates.hs:55 (import) and :238
//     repoToText; Janitor.hs:238 showRepo; Open.hs:615/:620/:626/:834/:846
//     showRepo; Repo.hs:20-21 showRepoSep "/"; Storage/DB.hs:53 and
//     Storage/Memory.hs:88 repoToText; Logging.hs:43 (import) and :259
//     repoToText (glean/glass/Glean/Glass/Logging.hs); Indexer/External.hs
//     :128 showRepo (glean/index/Glean/Indexer/External.hs); clang
//     Derive/Types.hs:92 and hack Derive/Types.hs:75 readRepo "/";
//     lib/Derive.hs:59 showRepo; shell/Glean/Shell.hs:350/:373/:674/:697
//     parseRepo; bench/FactSetBench.hs:43/:151 parseRepo; test/tests/
//     BackupTest.hs:43/:334 showRepoSep; test/tests/CatalogTest.hs:56
//     showRepo; tools/derive/Glean/Derive/Digest.hs:45/:110, tools/diff/
//     Main.hs:54/:56, tools/gleancli/GleanCLI.hs:363, .../Backup.hs:42,
//     .../Common.hs:65/:70, .../Create.hs:149/:157, .../Restore.hs:199/
//     :248/:304, .../Write.hs:150, tools/loadgen/LoadGenerator.hs:99,
//     lang/angle/AngleIndexer/Main.hs:60 parseRepo and
//     lang/clang/Glean/Indexer/Cpp.hs:48/:293 showRepo (the last two via
//     the client facade). Non-edge consumers reach the names through the
//     client re-export (Glean.hs:64-71) — except the test files
//     (test/tests/BackupTest.hs:43, test/tests/CatalogTest.hs:38), which
//     import this module directly; tests are not translated (RULE §4).
//
// ASSUMED-IMPORT (RULE §4 B4): the generated thrift crate.
//   * The source's `import Glean.Types` (:25) supplies `Repo`, which the
//     generated Rust crate spells `glean_thrift::Repo`. This file uses ONE
//     spelling: `use glean_thrift as thrift;`, then `thrift::Repo`
//     everywhere (the established convention, e.g.
//     rust/glean-db/src/backend/types.rs:361).
//   * Pinned shape (glean/if/glean.thrift:106-109):
//         struct Repo { 1: string name; 2: string hash; }
//     Rust image (rust/glean-db/src/backend/types.rs:171-175):
//         #[derive(Clone, Debug, PartialEq, Eq, Hash)]
//         pub struct Repo { pub name: String, pub hash: String }
//     The Haskell type derives Eq, Show and Ord (gen-hs2
//     Glean/Types.hs:629-630: `deriving (Eq, Show, Ord)`). The Rust
//     sketch above carries Eq/Hash and Debug-as-Show but NOT
//     Ord/PartialOrd; `Map.Map Repo` at
//     db/Glean/Database/Retention.hs:150 needs Ord, so the generated crate
//     owes Ord + PartialOrd for that consumer at Step 4 (recorded, not
//     added here). This file only reads the two fields and constructs
//     owned values.
//   * glean-core has no Cargo.toml/lib.rs yet (Step 4 owns the scaffold),
//     so `glean_thrift` is not resolvable at this unit's own compile step;
//     that is the expected draft state for every generated-type consumer
//     (RULE §4 B4: import from the generated crate, never fork a hand
//     translation of it).
//
// SIBLING-UNIT CONTRACTS:
//   * Row 52 (migration/manifest.tsv:52) translates
//     glean/db/Glean/Database/Repo.hs -> rust/glean-db/src/database/repo.rs
//     and its `databaseSubdir = showRepoSep "/"` (Repo.hs:20-21) is this
//     file's `show_repo_sep("/", repo)`. Row 52 must import it as
//     `glean_core::repo::text::show_repo_sep` (RULE §4 cross-crate form)
//     and must not re-implement it. That target does not exist yet.
//   * The client facade re-exports all eight names (Glean.hs:64-71:
//     showRepo, showRepoSep, repoToText, repoToTextSep, readRepo,
//     parseRepo, parseRepoText, parseRepoTextSep). The eventual Rust client
//     facade should re-export the snake_case equivalents from this module
//     rather than duplicate them.
//   * Step-4 crate-root contract: glean-core has NO lib.rs/Cargo.toml
//     (HANDOFF §2/§4.2; Step 4 owns the workspace scaffold). This unit
//     creates repo.rs declaring `pub mod text;`; Step 4 must add
//     `pub mod repo;` to rust/glean-core/src/lib.rs. This unit does NOT
//     create lib.rs or Cargo.toml.
//
// EXPORT MAPPING (8/8, source order, source export list :10-19):
//   :28-29  showRepo          -> show_repo
//   :31-33  showRepoSep       -> show_repo_sep
//   :36-37  repoToText        -> repo_to_text
//   :39-40  repoToTextSep     -> repo_to_text_sep
//   :42-46  readRepo          -> read_repo
//   :50-51  parseRepo         -> parse_repo
//   :55-56  parseRepoText     -> parse_repo_text
//   :64-72  parseRepoTextSep  -> parse_repo_text_sep
//
// FIDELITY NOTES:
//   * Haskell `Repo ->` arguments become `&thrift::Repo` (the
//     rust/glean-db/src/database/catalog/store.rs convention: immutable
//     sharing, no clone at the call boundary). `String`/`Text` inputs
//     become `&str`.
//   * RULE §1 (strings row): every Text RETURN becomes String — repoToText
//     and repoToTextSep return `String`, not a Text type. Concatenation is
//     `format!`, the direct image of `<>`.
//   * `parseRepoTextSep` is the crux. Data.Text.breakOnEnd finds the LAST
//     occurrence, returning `(prefix-through-separator, suffix)`;
//     `Text.dropEnd (Text.length sep) prefix` removes the separator.
//     `str::rfind` returns the byte index of that same last occurrence;
//     `s[..i]` is the name and `s[i + sep.len()..]` is the hash. The
//     slicing indices are UTF-8 boundaries because `i` is a match start
//     and `i + sep.len()` is that match's end. The cut POSITION and the
//     removed separator coincide for a multi-byte separator; the numeric
//     indices differ (Text.length counts chars, sep.len() counts bytes),
//     but both remove exactly the matched separator.
//   * Edge cases were probed, not guessed, against text-2.0.2 (GHC 9.4.7)
//     and rustc; transcripts are at
//     /tmp/opencode/ts-probe/repo-text-ghc.txt and
//     /tmp/opencode/ts-probe/repo-text-rust.txt. Non-empty separator —
//     both sides agree exactly: absent -> None; "::" "a::b::c" ->
//     ("a::b","c"); "/x" -> ("","x"); "x/" -> ("x",""); repeated "."
//     "a.b.c.d" -> ("a.b.c","d"); "/" "a.b.c.d" -> None; "→" separator and
//     non-ASCII spec split at the same char; "/" "/" -> ("",""); "/" ""
//     -> None.
//   * EMPTY SEPARATOR is the one place the naive rfind image and the source
//     disagree: `"abc".rfind("")` is `Some(3)` (and `"".rfind("")` is
//     `Some(0)`), but text-2.0.2's `breakOnEnd "" _` calls
//     `emptyError "breakOn"` and throws
//     `error "Data.Text.breakOn: empty input"` — for every haystack,
//     including the empty one. The source is a partial function here, so
//     per RULE §2 (partial functions: "where the source would error \"msg\",
//     panic with the SAME message") the port panics with that exact
//     message instead of returning `Some`. This is the only deviation from
//     the rfind sketch, and it restores source behaviour rather than
//     changing it.
//   * `readRepo`'s `fromMaybe (Repo (pack s) "")` fallback is preserved:
//     when parsing fails the WHOLE spec is the name and the hash is empty.

use glean_thrift as thrift;

/// Source: showRepo (:28-29) — `showRepo = showRepoSep "/"`.
pub fn show_repo(repo: &thrift::Repo) -> String {
    show_repo_sep("/", repo)
}

/// Source: showRepoSep (:31-33) —
/// `Text.unpack name <> sep <> Text.unpack hash`.
pub fn show_repo_sep(sep: &str, repo: &thrift::Repo) -> String {
    format!("{}{}{}", repo.name, sep, repo.hash)
}

/// Source: repoToText (:36-37) — `name <> "/" <> hash`.
pub fn repo_to_text(repo: &thrift::Repo) -> String {
    format!("{}/{}", repo.name, repo.hash)
}

/// Source: repoToTextSep (:39-40) — `name <> sep <> hash`.
pub fn repo_to_text_sep(sep: &str, repo: &thrift::Repo) -> String {
    format!("{}{}{}", repo.name, sep, repo.hash)
}

/// Source: readRepo (:42-46) — parse, falling back to the whole spec as
/// the name with an empty hash.
pub fn read_repo(sep: &str, s: &str) -> thrift::Repo {
    parse_repo_text_sep(sep, s).unwrap_or_else(|| thrift::Repo {
        name: s.to_string(),
        hash: String::new(),
    })
}

/// Source: parseRepo (:50-51) — `parseRepoTextSep "/" . Text.pack`.
pub fn parse_repo(s: &str) -> Option<thrift::Repo> {
    parse_repo_text_sep("/", s)
}

/// Source: parseRepoText (:55-56) — `parseRepoTextSep "/"`.
pub fn parse_repo_text(s: &str) -> Option<thrift::Repo> {
    parse_repo_text_sep("/", s)
}

/// Source: parseRepoTextSep (:64-72) — everything before the LAST
/// separator is the name, everything after is the hash; `Nothing` when the
/// separator does not occur.
pub fn parse_repo_text_sep(sep: &str, s: &str) -> Option<thrift::Repo> {
    // Data.Text.breakOnEnd errors on an empty needle in text-2.0.2
    // ("Data.Text.breakOn: empty input"); reproduce the source's partial
    // function with the same message (RULE §2). See FIDELITY NOTES.
    if sep.is_empty() {
        panic!("Data.Text.breakOn: empty input");
    }
    match s.rfind(sep) {
        None => None,
        Some(i) => Some(thrift::Repo {
            name: s[..i].to_string(),
            hash: s[i + sep.len()..].to_string(),
        }),
    }
}

//
// PORT STATUS: confidence=high todos=0
