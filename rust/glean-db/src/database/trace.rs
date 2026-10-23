// Ported from: glean/db/Glean/Database/Trace.hs (46 lines).
// Manifest unit: rust/glean-db/src/database/trace.rs
// (migration/manifest.tsv line 73). RULE §4: one source file -> one target
// file; the manifest path is authoritative.
//
// WHAT THIS FILE IS: the database trace vocabulary. The DB layer emits these
// three message types through Util.Trace's `Tracer` (row 84): a write event
// with its byte count (:21), an enqueue event with a batch size (:22), and
// the four download/restore markers (:23-26). It is DATA ONLY — no IO, no
// FFI, no unsafe. The two TextShow instances (:33-36, :45-46) are the only
// behaviour.
//
// ---------------------------------------------------------------------------
// INVENTORY (RULE §5; migration/inventory*.tsv)
// ---------------------------------------------------------------------------
//
// ZERO rows name this source file, by path or by symbol:
//   $ grep -n 'Database/Trace' migration/inventory*.tsv
//   (no output; exit status 1)
//   $ grep -c 'GleanTrace' migration/inventory*.tsv
//   inventory.codeflow.tsv:0  inventory.ffi.tsv:0  inventory.protocol.tsv:0
//   inventory.sites.tsv:0     inventory.stm.tsv:0  inventory.tsv:0
// The file declares no state, throws nothing, reads no config and imports
// no FFI, so no other inventory class applies either.
//
// ---------------------------------------------------------------------------
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, checked not assumed)
// ---------------------------------------------------------------------------
//
//   * OUTBOUND: none. `grep -P '^glean/db/Glean/Database/Trace\.hs\t'
//     migration/depmap/edges.tsv` is empty. The source's non-base imports
//     are `Data.Word` (:15) and `TextShow` (:16) — base and a library —
//     plus `Glean.Types as Thrift` (:18), generated thrift; none is a
//     manifest unit, so the depmap has nothing to record.
//   * INBOUND (5), none edited by this unit:
//       edges.tsv:153 Backup.hs (manifest line 125, NOT shipped) emits
//         GleanTraceDownload/GleanTraceSiteRestore/GleanTraceStorageRestore/
//         GleanTraceFinishRestore (Backup.hs:393, :412, :414, :417).
//       edges.tsv:226 Config.hs (manifest line 85, NOT shipped) types
//         cfgTracer :: Tracer GleanTrace (Config.hs:209).
//       edges.tsv:454 Types.hs (manifest line 104, NOT shipped) types
//         envTracer :: Tracer GleanTrace (Types.hs:301).
//       edges.tsv:481 Write/Batch.hs (manifest line 112, NOT shipped)
//         emits GleanTraceWrite repo event value (Batch.hs:343-345).
//       edges.tsv:511 Writes.hs (manifest line 116, NOT shipped) emits
//         GleanTraceEnqueue (Writes.hs:221, :276, :295).
//     No Haskell consumer pattern-matches any constructor
//     (`grep -rn 'GleanTraceWrite\|GleanTraceEnqueue\|GleanTraceDownload\|
//     GleanTraceStorageRestore\|GleanTraceSiteRestore\|
//     GleanTraceFinishRestore' glean/` finds only the construction sites
//     above), so the port adds no accessors and no rendering: the payloads
//     are public, as the source's are.
//
// ---------------------------------------------------------------------------
// ASSUMED IMPORTS (RULE §4 B4)
// ---------------------------------------------------------------------------
//
//   * `glean_thrift::Repo` — generated from glean/if/glean.thrift:106-109.
//     Pinned shape (database/repo.rs:45-47, row 51's convention):
//       pub struct Repo { pub name: String, pub hash: String }
//     deriving Eq, Hash and Debug. `!Thrift.Repo` is a strict field, so the
//     Rust payload is owned `Repo` (not `&Repo`): these values are
//     constructed once and moved into the tracer.
//
// ---------------------------------------------------------------------------
// TRANSLATION NOTES
// ---------------------------------------------------------------------------
//
//   * Constructor names are kept VERBATIM (RULE §2's constructor-shaped
//     naming, the `ItemStatus { ItemComplete, ... }` precedent), so
//     `GleanTrace::GleanTraceWrite` and `WriteTraceEvent::WriteTraceInput`
//     keep the source's prefixes. Renaming them would break the one-to-one
//     grep between the two trees.
//   * `!Word64` (:21) -> u64; `!Int` (:22) -> i64 (RULE §1's Int row).
//   * TextShow (:33-36, :45-46) -> Display (RULE §2's TextShow row). The
//     first instance is LITERAL text: showb WriteTraceInput = "input" —
//     these are the source's own tag strings, not derived names.
//   * EnqueueTraceEvent derives Show (:43) and defines
//     showb = fromString . show (:46), i.e. GHC's derived rendering of a
//     nullary constructor, which is the constructor name verbatim. Display
//     therefore prints `EnqueueBatch`, `EnqueueJsonBatch`,
//     `EnqueueBatchDescriptor`, `EnqueueJsonBatchBS` — NOT snake_case and
//     not the TextShow-lowered variants.
//   * GleanTrace has no Show/TextShow instance in the source, so it gets no
//     Display here (RULE §2: Display only where a consumer could display).
//     No derives either: the source declares no Eq/Ord, and a fieldless
//     enum's Copy would be an invented instance.
//
// ---------------------------------------------------------------------------
// DIVERGENCES (deliberate)
// ---------------------------------------------------------------------------
//
//   None. The file is three data declarations plus two renderings; the only
//   translation choices are the §1/§2 mappings listed above.
//
// ---------------------------------------------------------------------------
// SIBLING-UNIT CONTRACTS
// ---------------------------------------------------------------------------
//
//   * `Tracer<A>` has NO manifest unit: it is Control.Trace.Core's type,
//     external hsthrift. Row 84 (Util/Trace.hs) shipped WITHOUT it — that
//     file is Glean.Util.Trace's Listener. Its home is decided at Step 4;
//     until then config.rs owns the sketch, and database/config.rs (row 85)
//     plus database/types.rs (row 104) use it for the
//     `cfg_tracer`/`env_tracer` fields as `Tracer<GleanTrace>`.
//   * database/backup.rs (row 125), database/write/batch.rs (row 112) and
//     database/writes.rs (row 116) construct the variants listed under
//     DEPENDENCY TRUTH.
//
// MARKERS: zero of every class — 0 port todos, 0 source-defect
// reproductions, 0 performance deferrals, 0 inventory divergences, 0 SAFETY
// notes (no unsafe code).

use std::fmt;

use glean_thrift as thrift;

/// 'data GleanTrace' (:20-26) — what the DB layer traces. The payloads are
/// the repo, the event, and the size/value the event carries.
pub enum GleanTrace {
    /// 'GleanTraceWrite !Thrift.Repo !WriteTraceEvent !Word64' (:21):
    /// Write/Batch.hs:345 passes the number of bytes written.
    GleanTraceWrite(thrift::Repo, WriteTraceEvent, u64),
    /// 'GleanTraceEnqueue !Thrift.Repo EnqueueTraceEvent !Int' (:22):
    /// Writes.hs:221/:276 pass a batch size, :295 passes 0.
    GleanTraceEnqueue(thrift::Repo, EnqueueTraceEvent, i64),
    /// 'GleanTraceDownload !Thrift.Repo' (:23).
    GleanTraceDownload(thrift::Repo),
    /// 'GleanTraceStorageRestore' (:24).
    GleanTraceStorageRestore,
    /// 'GleanTraceSiteRestore' (:25).
    GleanTraceSiteRestore,
    /// 'GleanTraceFinishRestore' (:26) — the last of the restore markers.
    GleanTraceFinishRestore,
}

/// 'data WriteTraceEvent' (:28-31) — the three write phases.
pub enum WriteTraceEvent {
    /// 'WriteTraceInput' (:29).
    WriteTraceInput,
    /// 'WriteTraceRename' (:30).
    WriteTraceRename,
    /// 'WriteTraceCommit' (:31).
    WriteTraceCommit,
}

/// 'instance TextShow WriteTraceEvent' (:33-36): the tag strings are the
/// source's literals, not derived constructor names.
impl fmt::Display for WriteTraceEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            WriteTraceEvent::WriteTraceInput => "input",
            WriteTraceEvent::WriteTraceRename => "rename",
            WriteTraceEvent::WriteTraceCommit => "commit",
        })
    }
}

/// 'data EnqueueTraceEvent' (:38-43) — the four enqueue flavours.
pub enum EnqueueTraceEvent {
    /// 'EnqueueBatch' (:39).
    EnqueueBatch,
    /// 'EnqueueJsonBatch' (:40).
    EnqueueJsonBatch,
    /// 'EnqueueBatchDescriptor' (:41).
    EnqueueBatchDescriptor,
    /// 'EnqueueJsonBatchBS' (:42) — the ByteString (pre-serialised) variant.
    EnqueueJsonBatchBS,
}

/// 'instance TextShow EnqueueTraceEvent' (:45-46): 'showb = fromString .
/// show', i.e. GHC's derived Show, whose text for a nullary constructor is
/// the constructor name verbatim.
impl fmt::Display for EnqueueTraceEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            EnqueueTraceEvent::EnqueueBatch => "EnqueueBatch",
            EnqueueTraceEvent::EnqueueJsonBatch => "EnqueueJsonBatch",
            EnqueueTraceEvent::EnqueueBatchDescriptor => "EnqueueBatchDescriptor",
            EnqueueTraceEvent::EnqueueJsonBatchBS => "EnqueueJsonBatchBS",
        })
    }
}

// PORT STATUS: confidence=high todos=0
