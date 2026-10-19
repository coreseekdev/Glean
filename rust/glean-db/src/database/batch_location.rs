// Ported from: glean/db/Glean/Database/BatchLocation.hs (31 lines).
// Manifest unit: rust/glean-db/src/database/batch_location.rs
// (migration/manifest.tsv line 62, crate glean-db). RULE §4: one source
// file -> one target file; the manifest path is authoritative.
//
// WHAT THIS FILE IS: the parser interface that turns a batch descriptor's
// location string into a Thrift Batch (:19-22), its existential wrapper
// (:24-25), and the default parser that refuses every input (:27-31). The
// interface exists so an installation can plug in its own location scheme;
// OSS ships only the refusing default (Config.hs:250).
//
// ---------------------------------------------------------------------------
// INVENTORY (RULE §5; migration/inventory*.tsv)
// ---------------------------------------------------------------------------
//
// ZERO rows name this source file. Exact command at the repo root:
//   $ grep -n 'BatchLocation' migration/inventory*.tsv
//   (no output; exit status 1)
// Repeated across all six inventory files. A broader
// 'grep -n BatchLocation migration/inventory*.tsv' is likewise empty.
//
// The construct-level row that DOES govern this file's existential is
// followed, not flagged:
//   * migration/inventory.tsv:136 ('Some c', existential over constraint,
//     MECHANICAL, confirmed): "closed enum at each use site; Box<dyn Trait>
//     only for truly-open sites". The Parser instance set is closed —
//     `grep -rn 'instance Parser' glean` finds only DefaultParser (:29) and
//     the Some forwarding instance (:24) — so SomeParser is a closed enum
//     (see TRANSLATION NOTES), not a trait object.
// No EXC_TAXONOMY row names this file: the throw at :30 is the generated
// Thrift exception, not DbError (see ERROR TYPE). No
// INVENTORY-BUG-CANDIDATE applies.
//
// ---------------------------------------------------------------------------
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, checked not assumed)
// ---------------------------------------------------------------------------
//
//   * OUTBOUND (1): edges.tsv:170 BatchLocation.hs -> Util/Some.hs
//     (manifest line 38, SHIPPED at 4bdf95a as
//     rust/glean-util/src/util/some.rs, which by design exports no code).
//     The closed enum below is the port of that existential, not an import.
//     Every other import is generated (Glean.Types) or base: Data.Text and
//     Control.Exception (throw).
//   * INBOUND (3), none edited by this unit:
//       edges.tsv:216 Config.hs (manifest line 85, NOT shipped) stores
//         `Some BatchLocation.DefaultParser` in cfgBatchLocationParser
//         (Config.hs:250).
//       edges.tsv:447 Types.hs (manifest line 104, NOT shipped) declares
//         envBatchLocationParser :: Some BatchLocation.Parser (Types.hs:248).
//       edges.tsv:497 Write/Queue.hs (manifest line 105, NOT shipped) calls
//         BatchLocation.fromString on that field (Queue.hs:201-202).
//
// ---------------------------------------------------------------------------
// ASSUMED IMPORTS (RULE §4 B4)
// ---------------------------------------------------------------------------
//
//   * glean_thrift::{Batch, BatchFormat, Exception} — generated from
//     glean/if/glean.thrift:
//       struct Batch        :159-214
//       enum BatchFormat    :909-912 (JSON = 0, Binary = 1); the pinned
//                           Rust image is
//                           write/send_batch.rs:265 `enum BatchFormat
//                           { Json, Binary }`
//       exception Exception :225-227, one field `1: string message`
//     The Rust images are `glean_thrift::Batch`, `glean_thrift::BatchFormat`
//     and `glean_thrift::Exception { message: String }`. The generated crate
//     must give Exception Display (the derived-Show image) and
//     std::error::Error; both are Step-4 generator obligations, the same
//     class as Repo's pinned derives in database/repo.rs. If the generated
//     spellings differ, the single struct literal below is the only place
//     to change: a compile error, never silent behavior.
//
// ---------------------------------------------------------------------------
// ERROR TYPE
// ---------------------------------------------------------------------------
//
// The source throws `Thrift.Exception "..."` (:30-31), the generated
// RPC-facing exception — NOT a DbError. Two reasons this unit does not use
// the database funnel: there is no repo to prefix (database/exception.rs's
// db_error takes one), and the server maps a Thrift exception to an error
// response, so the exception type is observable. The Rust image is
// therefore Result<Batch, thrift::Exception>, with the message exactly
// "Batch location's parser is not implemented".
//
// ---------------------------------------------------------------------------
// TRANSLATION NOTES
// ---------------------------------------------------------------------------
//
//   * class Parser a (:20-22) -> pub trait Parser, source order preserved:
//     from_string(&self, loc: &str, format: thrift::BatchFormat)
//     -> Result<thrift::Batch, thrift::Exception>. Text -> &str (RULE §1
//     strings row); the format stays by value, as in the source (a small
//     fieldless enum).
//   * instance Parser (Some Parser) (:24-25) -> impl Parser for SomeParser,
//     forwarding to the wrapped variant.
//   * data DefaultParser = DefaultParser (:27) -> pub struct DefaultParser
//     (unit struct). The source exports DefaultParser(..), and Config.hs:250
//     constructs it, so the value must be constructible.
//   * Some Parser (:24) -> pub enum SomeParser { Default(DefaultParser) }.
//     Rust-only, like SomeBackupBackend; it must be pub because Types.hs:248's
//     future field stores it. The variant set is closed by the instance grep
//     above.
//   * throw (:30) -> Err; no panic and no IO effect remains, so the IO
//     return becomes Result.
//
// ---------------------------------------------------------------------------
// DIVERGENCES (deliberate)
// ---------------------------------------------------------------------------
//
//   1. IO -> Result. The source's only effect is the throw; nothing else is
//      lost.
//   2. The exception's Show text is the generated type's Display obligation,
//      not this file's. Queue.hs:67 renders `show exc` into
//      DatabaseBroken ("write", ...), so the Step-4 generator's Display must
//      match hsthrift's derived Show for the exception.
//
// ---------------------------------------------------------------------------
// SIBLING-UNIT CONTRACTS
// ---------------------------------------------------------------------------
//
//   * database/types.rs (row 104, future): env_batch_location_parser:
//     SomeParser.
//   * database/config.rs (row 85, future): cfg_batch_location_parser:
//     SomeParser; its default is SomeParser::Default(DefaultParser)
//     (Config.hs:250).
//   * database/write/queue.rs (row 105, future):
//     parser.from_string(&location, format)? inside
//     download_batch_from_location; the error joins the caller's catch-all
//     and becomes the DatabaseBroken reason string (Queue.hs:62-68).
//   * glean-util's some.rs (row 38): documents the closed-enum decision and
//     exports no code; nothing is imported from it.
//
// MARKERS: zero of every class — 0 port todos, 0 source-defect
// reproductions, 0 performance deferrals, 0 inventory divergences, 0 SAFETY
// notes (no unsafe code).
//
// PORT STATUS: confidence=high todos=0

use glean_thrift as thrift;

/// 'class Parser a' (:20-22): parse a location string, in the given batch
/// format, into a Batch. The source's IO is only the throw, so the method
/// returns Result.
pub trait Parser {
    fn from_string(
        &self,
        loc: &str,
        format: thrift::BatchFormat,
    ) -> Result<thrift::Batch, thrift::Exception>;
}

/// 'instance Parser (Some Parser)' (:24-25): forward to the wrapped parser.
///
/// The Haskell existential is a closed enum here (inventory.tsv:136): the
/// only Parser instances in the tree are DefaultParser and this forwarding
/// instance.
pub enum SomeParser {
    Default(DefaultParser),
}

impl Parser for SomeParser {
    fn from_string(
        &self,
        loc: &str,
        format: thrift::BatchFormat,
    ) -> Result<thrift::Batch, thrift::Exception> {
        match self {
            SomeParser::Default(parser) => parser.from_string(loc, format),
        }
    }
}

/// 'data DefaultParser = DefaultParser' (:27): the OSS parser, which rejects
/// every location.
pub struct DefaultParser;

/// 'instance Parser DefaultParser' (:29-31): both arguments are ignored.
impl Parser for DefaultParser {
    fn from_string(
        &self,
        _loc: &str,
        _format: thrift::BatchFormat,
    ) -> Result<thrift::Batch, thrift::Exception> {
        Err(thrift::Exception {
            message: "Batch location's parser is not implemented".to_string(),
        })
    }
}
