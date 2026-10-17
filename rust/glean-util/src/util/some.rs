// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// Ported from: glean/util/Glean/Util/Some.hs (13 lines)
// Manifest unit: rust/glean-util/src/util/some.rs (migration/manifest.tsv
// line 38, crate glean-util). RULE §4: one source file -> one target file.

//! An existential over a CONSTRAINT, and why this module has no code.
//!
//! The entire Haskell source (RULE §0: the old code is the spec) is:
//!
//! ```haskell
//! data Some c where
//!   Some :: c a => a -> Some c
//! ```
//!
//! `Some c` hides a concrete type `a` while remembering only that `a`
//! satisfies the constraint (type class) `c`. The module exports
//! `Some(..)` and NOTHING else — no combinators (no `mapSome`/
//! `withSome`), no instances, no dependencies (leaf module; depmap
//! Some.hs:10-13 — the file has no import declarations). Haskell
//! consumers do exactly two things with it:
//!
//! * wrap: `Some x` where `x`'s type satisfies `c` in scope
//!   (demo/Hyperlink.hs:548 `stateBackend = Some backend`);
//! * unpack by pattern match, which re-exposes the constraint at the
//!   match site (glean/db/Glean/Logger.hs:40-41
//!   `case envServerLogger env of Some logger -> runLog logger …`).
//!
//! The one generic idiom consumers build on it — `instance C (Some C)`
//! forwarding every method after unpacking (glean/backend-api/Glean/
//! Backend/Types.hs:193 `queryFact (Some backend) = queryFact backend`;
//! glean/client/hs/local/Glean/LocalOrRemote.hs:211-216; glean/glass/
//! Glean/Glass/SourceControl.hs:69-74) — is pure boilerplate over the
//! unpack, not an export of this module.
//!
//! # The Rust modeling decision
//!
//! // RULE §2 (GADTs / existential `Some c` row) + RULE §5 (inventory.tsv
//! row 136, status confirmed): "closed enum at each use site;
//! `Box<dyn Trait>` only for truly-open sites — CARVE-OUT granted for
//! logger sinks (genuinely open, dyn allowed)". Rust has no
//! constraint-polymorphic existentials — there is no way to write a
//! `Some<C>` that erases the wrapped type while keeping its class
//! dictionary (and `Box<dyn Any>` is not it: consumers need the `c a`
//! dictionary, not `Typeable`). So the translation is decided AT EACH
//! USE SITE, per row 136, and NOTHING generic remains for this module
//! to export. Instantiations found by the consumer survey (survey
//! REOPENED per review A finding 3 — TODO(port) below)
//! (grep "import Glean.Util.Some" across glean/ — 53 files):
//!
//! * CLOSED-set sites -> a plain Rust `enum` at the use site, one
//!   variant per concrete inhabitant:
//!   - `Some Backend` (Backend/Types.hs:193 forwarding instance;
//!     Backend/Retry.hs:20/76 RetryWritesBackend/RetryReadsBackend wrap
//!     one; write/Glean/Write/Async.hs:43 `senderBackend`; demo/
//!     Hyperlink.hs:96) — the concrete backends are a closed set
//!     (Database.Env @ Local.hs:73, ThriftBackend @ Remote.hs:184,
//!     RetryWrites/RetryReads @ Retry.hs:27,83, LoggingBackend @
//!     backend-local/Logging.hs:47, S3Backend @ S3.hs:101, MockBackend @
//!     Backup/Mock.hs:44 — closed), enum lives with the
//!     Backend trait's crate (rust/glean-db/src/backend/types.rs,
//!     manifest line 41).
//!   - `Some LocalOrRemote` (shell/Glean/Shell/Types.hs:125 field
//!     `backend`, :164 unpack; LocalOrRemote.hs:211 forwarding
//!     instance) — closed {local, remote} pair.
//!   - `Some DatabaseOps` (db/Glean/Database/Types.hs:101 `odbHandle`)
//!     — the storage engines are a closed set (LMDB/RocksDB/Memory/
//!     no phantom "Null" engine — the fourth name is the shared-ops
//!     wrapper DB @ Storage/DB.hs:57, not an instance; matches inventory
//!     row 137's enum Storage {Rocks, Lmdb, Memory}).
//!   - `Some Storage` (Database/Types.hs:251
//!     `envStorage :: HashMap StorageName (Some Storage)`), same closed
//!     engine set.
//!   - `Some BatchLocation.Parser` (Database/Types.hs:248
//!     `envBatchLocationParser`) — closed parser set.
//!   - `Some SourceControl`, `Some SnapshotBackend` (glass/Glass/Env.hs
//!     :61-62/:72-77 config fields; Glass/SourceControl.hs:69-74 and
//!     SnapshotBackend.hs forwarding instances; the Nil* null variants
//!     are variants like any other) — closed per-site enums in
//!     glean-glass.
//!   - Remaining grep-only instantiations (no line cites needed for the
//!     decision; same rule applies): `Some Site` x7 — ALL in
//!     db/Glean/Database/Backup/{Backend.hs:41,87, Locator.hs:36,46,66,76,
//!     Mock.hs:41}, NOT glass; backend/
//!     logger wraps in Server.hs/Indexer.hs/cli wiring.
//! * GENUINELY-OPEN sites -> `Box<dyn Trait>`, the row-136 CARVE-OUT
//!   granted ONLY to the logger sinks: `Some GleanServerLogger` /
//!   `Some GleanDatabaseLogger` (Database/Types.hs:246-247
//!   `envServerLogger`/`envDatabaseLogger`; unpacked at Logger.hs:40-41
//!   and Database/Logger.hs:98-99) — plugin sinks, open by design.
//! * PRODUCTION (corrected per review A): `Some IsSrcSpan` is typechecker
//!   code, NOT tests — db/Glean/Query/Typecheck/Types.hs:54
//!   `type TcPred = (PidRef, Some IsSrcSpan)`, Typecheck/Monad.hs:106,
//!   Typecheck.hs:715 + :1246-1269 (construct + polymorphic unpack).
//!   Instance set closed ({SrcSpan @ Angle/Types.hs:875, SpanAngleDSL @
//!   typed/Glean/Query/Angle.hs:98}) -> enum-at-use-site in the
//!   query/typecheck units; span tracking must NOT be dropped.
//! * other regression-test uses are TESTS —
//!   // RULE §4: tests are not translated; not use sites of the port.
//!
//! This module therefore intentionally ships ZERO items. A generic
//! `struct Some<T>(pub T)` would not model the existential (the
//! concrete type would stay visible, and no row-136 site would import
//! it — each defines its own enum), so inventing one would fork the
//! canonical mapping: // RULE §5 — silent divergence, even
//! right-for-the-right-reason, is a failed review. The path
//! `glean_util::util::some` is kept (RULE §4: the manifest is
//! authoritative) so this decision record sits where consumers and
//! reviewers look for it.

// Coverage: the 13-line source is fully accounted for — the GADT maps
// to per-use-site enums / dyn logger sinks per the row above; nothing
// in the source is left unported (no exports besides `Some(..)`).
//
// Marker breakdown: ONE TODO(port) (reopened survey) — the review-corrected record carries it
// (the modeling question is CLOSED by inventory row 136, confirmed —
// nothing unknown remains); no PERF(port) (no code); no BUG(port) (no
// behavior); no INVENTORY-BUG-CANDIDATE (this file follows row 136
// exactly — the carve-out it grants is applied, not widened: logger
// sinks only).
//
// TODO(port): consumer survey REOPENED by review (finding 3): the four
// additions above (Catalog.Store / Backup.Backend / Some Site x7 /
// S3BucketApi) were found by re-grepping; any FURTHER missed
// instantiation is a record bug — apply row 136's closed-enum rule per
// site, and the S3 unit owes the second open-site dyn ruling.
// PORT STATUS: confidence=medium todos=1
