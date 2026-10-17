// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// Ported from: glean/backend-api/Glean/Backend/Retry.hs (129 lines)
// Manifest unit: rust/glean-db/src/backend/retry.rs (migration/manifest.tsv
// line 43, crate glean-db). RULE §4: one source file -> one target file.
//
// The source is pure Backend-wrapper plumbing: two newtype records that
// each hold a `RetryPolicy` and an inner `Some Backend`, plus one
// `Backend` instance each that partitions the class's methods into a
// RETRIED set (each method body becomes
// `retryChannelExceptions policy $ <inner method>`) and a FORWARDED set
// (the body delegates to the inner backend unchanged):
//   * `RetryWritesBackend` (:20-74) — retries the WRITE-side operations,
//     11 of them: deriveStored (:38-39), kickOffDatabase (:41-42),
//     finishDatabase (:43-44), finalizeDatabase (:45-46),
//     updateProperties (:47-48), completePredicates_ (:49-50),
//     enqueueBatch (:55-56), enqueueJsonBatch (:57-58),
//     enqueueBatchDescriptor (:59-62), pollBatch (:63-64),
//     waitForWrites (:65-66); forwards the other 18 (incl. backend-
//     WithClientId, which re-wraps the INNER backend, :73-74).
//   * `RetryReadsBackend` (:76-129) — retries the READ-side operations,
//     10 of them: queryFact (:84-85), factIdRange (:86-87),
//     getSchemaInfo (:88-89), validateSchema (:90-91),
//     predicateStats (:92-93), listDatabases (:94-95), getDatabase
//     (:96-97), userQueryFacts (:98-99), userQuery (:100-101),
//     userQueryBatch (:102-103); forwards the other 19 (incl.
//     deriveStored, :104 — THE ASYMMETRY IS THE SPEC: writes retry it,
//     reads forward it).
// Both instances enumerate ALL 29 class methods (28 abstract + the
// backendWithClientId default); the per-method retry/forward partition
// with source lines is cited at each impl below. `retryChannelExceptions
// policy` appears 21 times in the source (11 + 10) — the exact count the
// amended util surface's header cites for this file
// (retry_channel_exception.rs:20-23, :157-161).
//
// ---------------------------------------------------------------------------
// FORWARD CONTRACT DISCHARGED (unit 41 pre-declaration)
// ---------------------------------------------------------------------------
//
// The struct names, genericity and field order are NOT a free design:
// unit 41's types.rs pre-declared them at
//   * types.rs:775 `RetryWrites(Box<crate::backend::retry::
//     RetryWritesBackend<R>>)` and types.rs:776 `RetryReads(...)` (the
//     `SomeBackend<R>` enum, :772-778), with the forward contract
//     comment above them (:768-771: "the impl compiles once `Env:
//     Backend` exists (row 147), the two wrapper impls exist (row 43)
//     ..."),
// so this file ships `pub struct RetryWritesBackend<R>` /
// `pub struct RetryReadsBackend<R>`, GENERIC over the inner backend `R`,
// with `pub policy: RetryPolicy` + `pub backend: R` in that order (the
// source's positional tuple `RetryPolicy (Some Backend)`, :20/:76; the
// fields are pub because the source export list is
// `RetryWritesBackend(..)` / `RetryReadsBackend(..)`, :10/:12).
//
// `Some Backend` materialization (the unit-38 closed-enum ruling
// applied): the source's existential field `Some Backend` (:20/:76) has
// no direct Rust image. RULE §2's GADT/existential row and the decision
// record (rust/glean-util/src/util/some.rs:57-66, listing
// "Backend/Retry.hs:20/76 RetryWritesBackend/RetryReadsBackend wrap
// one"; corrected for S3Backend/MockBackend by types.rs:281-304's
// RECORD DIVERGENCE block) close the backend set, and types.rs decision
// (B) (:321-327) materializes the existential as the closed
// `SomeBackend<R>` enum — the generic `R` HERE is the un-erased leg of
// that same decision ("glean-db cannot name ThriftBackend ... the enum
// is generic and glean-client instantiates SomeBackend<ThriftBackend>").
// The wrapper structs therefore hold the inner backend as plain generic
// `R`; the `Some` wrapping is the enum's
// `RetryWrites(Box::new(...))` / `RetryReads(Box::new(...))` variant
// construction, and the `Box` that breaks the SomeBackend <-> Retry*
// recursion lives on the ENUM side, not here (types.rs:760-766,
// scratch-verified there with probe_c leg C1). This file's part of the
// forward contract is discharged by these two structs and their two
// `impl<R: Backend> Backend` blocks.
// SUPERSEDES types.rs:760-763's prose: that Box note says the wrappers
// "hold `SomeBackend<R>` BY VALUE (Retry.hs:20/:76 field `Some
// Backend`)" — a description of the existential design the code moved
// past. The governing shape is the code on BOTH sides: the wrapper's
// field is plain `R` (below) and the enum boxes the WRAPPER
// (types.rs:775-776). Do not "fix" either file to match the stale
// sentence; a later unit touching that comment should reword it, not
// the structs.
//
// ---------------------------------------------------------------------------
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, module-level)
// ---------------------------------------------------------------------------
//
// Rows naming glean/backend-api/Glean/Backend/Retry.hs (grepped, not
// recalled):
//   OUTBOUND — exactly three, and each has a disposition below:
//     :15 -> glean/backend-api/Glean/Backend/Types.hs
//         = `crate::backend::types::{Backend, BackendError, ...}` (same
//           crate; the trait this file impls, the error channel its
//           methods return, the request/state types its signatures
//           mention);
//     :16 -> glean/util/Glean/Util/RetryChannelException.hs
//         = `glean_util::util::retry_channel_exception::
//           {retry_channel_exceptions, RetryPolicy}` (the amended
//           surface — see SIBLING-UNIT CONTRACTS);
//     :17 -> glean/util/Glean/Util/Some.hs
//         = consumed INDIRECTLY here: the import's only use in the
//           source is the `Some backend` field of the two records
//           (:20/:76, wrapped by the constructors :25/:81), and that
//           existential's Rust image is the `SomeBackend<R>` enum owned
//           by types.rs (decision (B), above) — so this file needs no
//           direct import of the Some surface; the edge discharges
//           through the enum whose wrapper variants name these structs.
//   INBOUND — exactly two, the consumers this surface must be
//   call-compatible with:
//     :96  glean/client/hs/Glean/Remote.hs (manifest 167 ->
//          rust/glean-client/src/remote.rs): feeds
//          backendRetryWrites/backendRetryReads and defaultRetryPolicy
//          (per retry_channel_exception.rs:173-178's contract);
//     :1886 glean/write/Glean/Write/SendQueue.hs (manifest 157 ->
//          rust/glean-client/src/write/send_queue.rs).
//
// ---------------------------------------------------------------------------
// INVENTORY (RULE §5)
// ---------------------------------------------------------------------------
//
// Grepped "Backend/Retry" across migration/inventory*.tsv — exactly two
// rows touch this file, neither diverged from:
//   * inventory.tsv:7 == inventory.protocol.tsv:23 (group row, "wrapper
//     initGlobalState pass-throughs"; cites Retry.hs:72 and :127): "every
//     Backend wrapper delegates initGlobalState to the inner backend ...
//     one adapter layer, do not invent per-wrapper batching". Both impls
//     forward `init_global_state` to `self.backend` unchanged (the
//     methods below) — followed exactly.
//   * inventory.sites.tsv:535/:536 — the two source sites of that row
//     (Retry.hs:72 / :127), covered by the same forwards.
// No row governs the retry/forward partition, the error handling, or
// the constructors (the nearest exception-taxonomy rows —
// inventory.tsv:13 == inventory.protocol.tsv:35 — govern the Haxl
// consumer, per retry_channel_exception.rs:46-56's analysis; this file
// follows their Rust half by routing retries through the distinct
// `ThriftError::Channel` leg, the same leg the amended util file
// scopes on). No INVENTORY-BUG-CANDIDATE: nothing diverges.
//
// ---------------------------------------------------------------------------
// SIBLING-UNIT CONTRACTS
// ---------------------------------------------------------------------------
//
//   * rust/glean-db/src/backend/types.rs (row 41, SHIPPED): owns the
//     `Backend` trait (29 methods, types.rs:444-640 — 28 abstract +
//     the `where Self: Sized` by-value `backend_with_client_id` default,
//     :633-639), `BackendError` (:415-420), `LogDerivationResult`
//     (:667-675), `GleanGetState`/:1693 + `GleanQueryState`/:1796, and
//     the `SomeBackend<R>` pre-declaration (:772-778) this file's
//     structs slot into. The method signatures below are MIRRORED from
//     its impl block (types.rs:784-1169) so the enum's forwarding calls
//     (`x.query_fact(repo, id)` through `&Box<RetryWritesBackend<R>>`,
//     :793) resolve against these impls unchanged — call-compatibility
//     probe-verified (/tmp/db43-probe/, check 7).
//   * rust/glean-util/src/util/retry_channel_exception.rs (row 42,
//     SHIPPED, AMENDED 2026-10-18 at unit 43 — the header note :16-28):
//     `pub fn retry_channel_exceptions<A>(r: &RetryPolicy, mut act:
//     impl FnMut() -> Result<A, ThriftError>) -> Result<A, ThriftError>`
//     (:347-350) and `pub struct RetryPolicy` (:239-254). The amendment
//     is why this file can exist as written: every retried body reuses
//     ONE `&self.policy` across `&self` methods (21 call sites), which
//     the amended borrowed-policy + `Fn` callback shape enables
//     (retry_channel_exception.rs:219-232). RetryPolicy is NOT Clone
//     (boxed-callback field) and must not need to be — it is only ever
//     shared-borrowed here. `default_retry_policy` is NOT consumed:
//     the source takes policies as arguments (:24/:80), it never
//     constructs one.
//   * rust/glean-util/src/util/some.rs (row 38, SHIPPED): the decision
//     record this file's Some-materialization note cites (:59-66).
//   * rust/glean-db/src/backend/logging.rs (row 148, FUTURE) and
//     rust/glean-db/src/database/types.rs (row 104, FUTURE): no
//     reference from this file; they complete the SomeBackend variant
//     set, not these wrappers.
//   * rust/glean-client/src/remote.rs (row 167, FUTURE) + rust/
//     glean-client/src/write/send_queue.rs (row 157, FUTURE): the two
//     inbound depmap consumers — they construct these wrappers via the
//     `backend_retry_writes` / `backend_retry_reads` constructors
//     (source argument order `backend policy`, :24-25/:80-81, kept).
//
// ---------------------------------------------------------------------------
// THE ERROR BRIDGE (the one translation decision this file owns)
// ---------------------------------------------------------------------------
//
// The source's retried bodies run an arbitrary `IO` action under
// `retryChannelExceptions` — the action may throw ANY exception; the
// wrapper catches ONLY ChannelException (retrying per policy) and lets
// everything else propagate UNTOUCHED (RetryChannelException.hs:58-63:
// `e <- try act` at :60 with the single `Left ex@ChannelException{}`
// pattern at :63). Under RULE §2's dynamic-exceptions row
// the action's error space is the `Backend` trait's channel,
// `BackendError` (types.rs:415-420: `Thrift(ThriftError)` /
// `Retry(thrift::Retry)` / `Other(...)`), while the shipped retry loop
// speaks `ThriftError` (its catch leg is `ThriftError::Channel`,
// retry_channel_exception.rs:374). The bridge must convert
// BackendError -> ThriftError for the action and back for the return,
// LOSSLESSLY — and the losslessness is load-bearing, not pedantry:
// Types.hs:333-337's `untilDone` catches the declared `Retry` exception
// OUT of backend methods it wraps (e.g. completePredicates,
// Types.hs:325-327, which retries through a `RetryWritesBackend` whose
// completePredicates_ may throw `Retry` from the server). If the
// wrapper degraded `BackendError::Retry` into some other shape,
// `until_done` (types.rs:1350-1365) would stop matching. Same for the
// open `Other` leg and for every non-Channel `ThriftError` leg.
//
// The shipped surfaces give exactly one lossless path, used here:
//   * ThriftError -> BackendError is total and lossless:
//     `BackendError::Thrift(te)` (the variant WRAPS ThriftError —
//     types.rs:417). Exhausted retries rethrow the ORIGINAL
//     ChannelException (retry_channel_exception.rs:375-383); re-wrapping
//     yields `BackendError::Thrift(Channel(ex))`, exactly the value the
//     inner call produced.
//   * BackendError -> ThriftError is lossless only through the enum's
//     OPEN leg `Service(Box<dyn std::error::Error + Send + Sync>)`
//     (thrift_service.rs:283-292 sketch: "Open set ... plus anything
//     else not discriminated by name"): the `Thrift` leg maps to
//     itself; the `Retry`/`Other` legs are boxed in a PRIVATE wrapper
//     (`NotRetried`, below) that the loop never acts on (non-Channel
//     legs pass through "UNTOUCHED, same variant, same payload",
//     retry_channel_exception.rs:369-371) and that this file recovers
//     by downcast on the way back. A `Service` box produced by the
//     INNER backend (a real declared exception, e.g. BadQuery) fails
//     the downcast and is restored as
//     `BackendError::Thrift(Service(box))` — the original value, byte
//     for byte. Round trip verified per leg in /tmp/db43-probe/
//     (checks 2-5).
//   * No `From` impl is added to any shipped surface: both directions
//     are PRIVATE fns below. A pub `impl From<ThriftError> for
//     BackendError` (or an `Error`/`Display` impl for BackendError,
//     needed to box it directly) would pre-empt the taxonomy TODO that
//     explicitly defers those impls to Step 4's catch-taxonomy table
//     (types.rs:407-414) — RULE §4 consumer-extension discipline says
//     extend nothing here.
//   * `Send + Sync` note for `NotRetried`: `BackendError` is Send+Sync
//     transitively under the assumed images — `ThriftError`'s legs are
//     two String newtypes, the generated ApplicationException (plain
//     data) and the already-`Send + Sync` `Service` box; `Retry` is an
//     f64 struct (types.rs:176-178); `Other`/`Service` boxes are
//     `Send + Sync` by construction.
//
// The ACTION side of each retried body: the closure re-runs the inner
// call per attempt, so owned thrift arguments are cloned per attempt —
// the pinned house pattern (types.rs:1334-1338, complete_predicates:
// "The retried action re-passes the same pure value, exactly like the
// source ... each retry clones"; the generated-call convention takes
// thrift arguments by value, types.rs:437-438, and the assumed
// generated images derive per gen-hs2 deriving, types.rs:221-224).
// Borrowed arguments (`&Repo`, `Option<&Repo>`, `StackedDbOpts`) are
// re-passed as-is.
//
// The ONE exception is `derive_stored`'s `log: LogDerivationResult` —
// `Box<dyn Fn ...>` is move-only and NOT Clone, while the source re-passes
// the same function value on every attempt (:38-39). types.rs:512-515
// already pins the image for exactly this site: "a caller that KEEPS its
// callback wraps it in a cheaply-cloneable handle (e.g. Arc)". The
// wrapper is that caller: the Box is moved into an `Arc` once
// (`Arc::from`, probe-verified to accept the unsized Box) and each
// attempt rebuilds a fresh `LogDerivationResult` through a closure
// adapter capturing an Arc clone. The adapter form is forced by rustc
// (probe /tmp/db43-probe/, check 1): `Arc<dyn Fn>` does NOT implement
// `Fn` on stable (E0277 on `Box::new(arc.clone())`), so
// `Box::new(move |arg| f(arg))` is the image — same callback, invoked
// once per attempt, exactly like the source.
//
// ---------------------------------------------------------------------------
// ASSUMED-IMPORT (RULE §4 B4) — none new
// ---------------------------------------------------------------------------
//
// No surface is sketched here. Every `thrift::*` name in the signatures
// below is consumed via the alias `use glean_thrift as thrift;`
// (types.rs:361's convention) and is already pinned by types.rs's
// ASSUMED-IMPORT block (its member list :99-147 + code sketches
// :149-226 cover every name this
// file touches: Repo, Id, SchemaId, Handle, DatabaseProperties,
// GetSchemaInfo, ValidateSchema, ListDatabases, KickOff,
// DerivePredicateQuery, CompletePredicates, EnqueueBatch,
// EnqueueBatchWaitPolicy, ComputedBatch, SendJsonBatch, and the
// response/result types, plus SchemaInfo/FactIdRange/PredicateStats/
// UserQuery*/Database). The per-attempt `Clone` on generated structs is
// the types.rs:1334-1338 precedent, not a new assumption. The thrift
// ServiceException/ThriftError sketch this file's bridge boxes into is
// owned by glean-util/src/util/thrift_service.rs:264-292 (first sketch,
// row 40).

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use glean_thrift as thrift;
use glean_util::util::retry_channel_exception::{retry_channel_exceptions, RetryPolicy};

use crate::backend::types::{
    Backend, BackendError, GleanGetState, GleanQueryState, LogDerivationResult, StackedDbOpts,
};

// ---------------------------------------------------------------------------
// The error bridge — private, lossless (see the decision block above)
// ---------------------------------------------------------------------------

/// A `BackendError` leg the retry loop must not act on, boxed into the
/// open `ThriftError::Service` leg so the shipped loop passes it through
/// UNTOUCHED (non-Channel legs are never retried,
/// retry_channel_exception.rs:369-371) and this file can recover the
/// original value by downcast on the way back.
///
/// `Debug` is an `Error` supertrait requirement (probe v1: E0277
/// without it). `Display` is constructor-shaped per RULE §2's
/// deriving-Show row, but is UNOBSERVABLE in the source's semantics:
/// the retry machinery shows only ChannelExceptions (logRetry,
/// RetryChannelException.hs:47-52, runs on the Channel leg alone), and
/// every box this file creates is unwrapped before escaping the
/// wrapper — the text can never surface through this path.
#[derive(Debug)]
struct NotRetried(BackendError);

impl fmt::Display for NotRetried {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            BackendError::Retry(_) => write!(f, "Retry"),
            BackendError::Other(_) => write!(f, "Other"),
            BackendError::Thrift(_) => unreachable!("Thrift leg never boxed as NotRetried"),
        }
    }
}

impl std::error::Error for NotRetried {}

/// Action side: `BackendError` -> the `ThriftError` the retry loop
/// speaks. The `Thrift` leg maps to itself (its `Channel` leg is the
/// loop's retry trigger; every other leg passes through untouched);
/// the `Retry`/`Other` legs are boxed for the round trip.
fn backend_error_for_retry(e: BackendError) -> ThriftError {
    match e {
        BackendError::Thrift(te) => te,
        other => ThriftError::Service(Box::new(NotRetried(other))),
    }
}

/// Return side: the loop's `ThriftError` -> the `BackendError` the
/// trait methods return. A `Service` box holding OUR wrapper is
/// unwrapped to the original leg; any other `Service` box came from
/// the inner backend (downcast failure — restored inside the `Thrift`
/// leg unchanged); every other leg is re-wrapped as-is.
fn retry_error_to_backend(e: ThriftError) -> BackendError {
    match e {
        ThriftError::Service(boxed) => match boxed.downcast::<NotRetried>() {
            Ok(inner) => inner.0,
            Err(boxed) => BackendError::Thrift(ThriftError::Service(boxed)),
        },
        te => BackendError::Thrift(te),
    }
}

// ---------------------------------------------------------------------------
// RetryWritesBackend (:20-25)
// ---------------------------------------------------------------------------

/// `data RetryWritesBackend = RetryWritesBackend RetryPolicy (Some
/// Backend)` (:20). Field order is the source tuple's; the fields are
/// pub per the export `RetryWritesBackend(..)` (:10). The inner backend
/// is the generic `R` (the `Some Backend` existential is materialized
/// at the `SomeBackend<R>` enum layer — see the header note).
pub struct RetryWritesBackend<R> {
    /// The retry policy, shared-borrowed by every retried body (NOT
    /// consumed: `RetryPolicy` is not `Clone` and needs no clone here).
    pub policy: RetryPolicy,
    /// The wrapped backend (`Some Backend`, :20).
    pub backend: R,
}

/// `backendRetryWrites :: Backend b => b -> RetryPolicy ->
/// RetryWritesBackend` (:24-25) — argument order `backend policy` kept.
/// The source's `Some backend` wrapping is the consumer's
/// `SomeBackend::RetryWrites(Box::new(...))` (decision (B); no boxing
/// here).
pub fn backend_retry_writes<R: Backend>(backend: R, policy: RetryPolicy) -> RetryWritesBackend<R> {
    RetryWritesBackend { policy, backend }
}

/// The class instance (:27-74). Partition: 11 retried (the WRITE-side
/// operations), 18 forwarded — every one of the 29 methods enumerated
/// with its source line.
impl<R: Backend> Backend for RetryWritesBackend<R> {
    // forward (:28)
    fn query_fact(
        &self,
        repo: &thrift::Repo,
        id: thrift::Id,
    ) -> Result<Option<thrift::Fact>, BackendError> {
        self.backend.query_fact(repo, id)
    }

    // forward (:29)
    fn fact_id_range(&self, repo: &thrift::Repo)
        -> Result<thrift::FactIdRange, BackendError> {
        self.backend.fact_id_range(repo)
    }

    // forward (:30)
    fn get_schema_info(
        &self,
        repo: Option<&thrift::Repo>,
        request: thrift::GetSchemaInfo,
    ) -> Result<thrift::SchemaInfo, BackendError> {
        self.backend.get_schema_info(repo, request)
    }

    // forward (:31)
    fn validate_schema(&self, request: thrift::ValidateSchema)
        -> Result<(), BackendError> {
        self.backend.validate_schema(request)
    }

    // forward (:32)
    fn predicate_stats(
        &self,
        repo: &thrift::Repo,
        opts: StackedDbOpts,
    ) -> Result<BTreeMap<thrift::Id, thrift::PredicateStats>, BackendError> {
        self.backend.predicate_stats(repo, opts)
    }

    // forward (:33)
    fn list_databases(&self, request: thrift::ListDatabases)
        -> Result<thrift::ListDatabasesResult, BackendError> {
        self.backend.list_databases(request)
    }

    // forward (:34)
    fn get_database(&self, repo: &thrift::Repo)
        -> Result<thrift::GetDatabaseResult, BackendError> {
        self.backend.get_database(repo)
    }

    // forward (:35)
    fn user_query_facts(
        &self,
        repo: &thrift::Repo,
        request: thrift::UserQueryFacts,
    ) -> Result<thrift::UserQueryResults, BackendError> {
        self.backend.user_query_facts(repo, request)
    }

    // forward (:36)
    fn user_query(
        &self,
        repo: &thrift::Repo,
        request: thrift::UserQuery,
    ) -> Result<thrift::UserQueryResults, BackendError> {
        self.backend.user_query(repo, request)
    }

    // forward (:37)
    fn user_query_batch(
        &self,
        repo: &thrift::Repo,
        request: thrift::UserQueryBatch,
    ) -> Result<Vec<thrift::UserQueryResultsOrException>, BackendError> {
        self.backend.user_query_batch(repo, request)
    }

    // retry (:38-39) — `retryChannelExceptions policy $ deriveStored
    // backend log repo q`. The move-only log callback is re-passed per
    // attempt via the Arc adapter (see the header note; types.rs:512-515).
    fn derive_stored(
        &self,
        log: LogDerivationResult,
        repo: &thrift::Repo,
        query: thrift::DerivePredicateQuery,
    ) -> Result<thrift::DerivationStatus, BackendError> {
        let log: Arc<
            dyn Fn(
                    &Result<
                        thrift::UserQueryStats,
                        (Duration, Box<dyn std::error::Error + Send + Sync>),
                    >,
                ) + Send
                + Sync,
        > = Arc::from(log);
        retry_channel_exceptions(&self.policy, || {
            // `Arc<dyn Fn>` is not `Fn` on stable (probe E0277): rebuild
            // the Box through a closure adapter capturing an Arc clone —
            // the same callback invoked once per attempt, like the source.
            let f = log.clone();
            let log: LogDerivationResult = Box::new(move |arg| f(arg));
            self.backend
                .derive_stored(log, repo, query.clone())
                .map_err(backend_error_for_retry)
        })
        .map_err(retry_error_to_backend)
    }

    // retry (:41-42)
    fn kick_off_database(&self, request: thrift::KickOff)
        -> Result<thrift::KickOffResponse, BackendError> {
        retry_channel_exceptions(&self.policy, || {
            self.backend
                .kick_off_database(request.clone())
                .map_err(backend_error_for_retry)
        })
        .map_err(retry_error_to_backend)
    }

    // retry (:43-44)
    fn finish_database(&self, repo: &thrift::Repo)
        -> Result<thrift::FinishDatabaseResponse, BackendError> {
        retry_channel_exceptions(&self.policy, || {
            self.backend
                .finish_database(repo)
                .map_err(backend_error_for_retry)
        })
        .map_err(retry_error_to_backend)
    }

    // retry (:45-46)
    fn finalize_database(&self, repo: &thrift::Repo)
        -> Result<thrift::FinalizeResponse, BackendError> {
        retry_channel_exceptions(&self.policy, || {
            self.backend
                .finalize_database(repo)
                .map_err(backend_error_for_retry)
        })
        .map_err(retry_error_to_backend)
    }

    // retry (:47-48)
    fn update_properties(
        &self,
        repo: &thrift::Repo,
        properties: thrift::DatabaseProperties,
        names: Vec<String>,
    ) -> Result<thrift::UpdatePropertiesResponse, BackendError> {
        retry_channel_exceptions(&self.policy, || {
            self.backend
                .update_properties(repo, properties.clone(), names.clone())
                .map_err(backend_error_for_retry)
        })
        .map_err(retry_error_to_backend)
    }

    // retry (:49-50)
    fn complete_predicates_(
        &self,
        repo: &thrift::Repo,
        predicates: thrift::CompletePredicates,
    ) -> Result<thrift::CompletePredicatesResponse, BackendError> {
        retry_channel_exceptions(&self.policy, || {
            self.backend
                .complete_predicates_(repo, predicates.clone())
                .map_err(backend_error_for_retry)
        })
        .map_err(retry_error_to_backend)
    }

    // forward (:52)
    fn restore_database(&self, backup: &str) -> Result<(), BackendError> {
        self.backend.restore_database(backup)
    }

    // forward (:53)
    fn delete_database(&self, repo: &thrift::Repo)
        -> Result<thrift::DeleteDatabaseResult, BackendError> {
        self.backend.delete_database(repo)
    }

    // retry (:55-56)
    fn enqueue_batch(&self, batch: thrift::ComputedBatch)
        -> Result<thrift::SendResponse, BackendError> {
        retry_channel_exceptions(&self.policy, || {
            self.backend
                .enqueue_batch(batch.clone())
                .map_err(backend_error_for_retry)
        })
        .map_err(retry_error_to_backend)
    }

    // retry (:57-58)
    fn enqueue_json_batch(
        &self,
        repo: &thrift::Repo,
        batch: thrift::SendJsonBatch,
    ) -> Result<thrift::SendJsonBatchResponse, BackendError> {
        retry_channel_exceptions(&self.policy, || {
            self.backend
                .enqueue_json_batch(repo, batch.clone())
                .map_err(backend_error_for_retry)
        })
        .map_err(retry_error_to_backend)
    }

    // retry (:59-62)
    fn enqueue_batch_descriptor(
        &self,
        repo: &thrift::Repo,
        batch: thrift::EnqueueBatch,
        wait_policy: thrift::EnqueueBatchWaitPolicy,
    ) -> Result<thrift::EnqueueBatchResponse, BackendError> {
        retry_channel_exceptions(&self.policy, || {
            self.backend
                .enqueue_batch_descriptor(repo, batch.clone(), wait_policy.clone())
                .map_err(backend_error_for_retry)
        })
        .map_err(retry_error_to_backend)
    }

    // retry (:63-64)
    fn poll_batch(
        &self,
        repo: Option<&thrift::Repo>,
        handle: thrift::Handle,
    ) -> Result<thrift::FinishResponse, BackendError> {
        retry_channel_exceptions(&self.policy, || {
            self.backend
                .poll_batch(repo, handle.clone())
                .map_err(backend_error_for_retry)
        })
        .map_err(retry_error_to_backend)
    }

    // retry (:65-66)
    fn wait_for_writes(&self, repo: &thrift::Repo)
        -> Result<thrift::WaitForWritesResponse, BackendError> {
        retry_channel_exceptions(&self.policy, || {
            self.backend
                .wait_for_writes(repo)
                .map_err(backend_error_for_retry)
        })
        .map_err(retry_error_to_backend)
    }

    // forward (:68)
    fn display_backend(&self) -> String {
        self.backend.display_backend()
    }

    // forward (:69)
    fn has_database(&self, repo: &thrift::Repo) -> Result<bool, BackendError> {
        self.backend.has_database(repo)
    }

    // forward (:70)
    fn schema_id(&self) -> Option<thrift::SchemaId> {
        self.backend.schema_id()
    }

    // forward (:71)
    fn using_shards(&self) -> bool {
        self.backend.using_shards()
    }

    // forward (:72) — inventory.tsv:7 == inventory.protocol.tsv:23:
    // wrappers delegate initGlobalState to the inner backend unchanged.
    fn init_global_state(&self)
        -> Result<(GleanGetState, GleanQueryState), BackendError> {
        self.backend.init_global_state()
    }

    // re-wrap (:73-74): `backendWithClientId cid (RetryWritesBackend
    // policy backend) = RetryWritesBackend policy
    // (backendWithClientId cid backend)` — policy preserved (self is
    // consumed), inner re-wrapped.
    fn backend_with_client_id(self, client_id: &str) -> Self {
        let RetryWritesBackend { policy, backend } = self;
        RetryWritesBackend {
            policy,
            backend: backend.backend_with_client_id(client_id),
        }
    }
}

// ---------------------------------------------------------------------------
// RetryReadsBackend (:76-81)
// ---------------------------------------------------------------------------

/// `data RetryReadsBackend = RetryReadsBackend RetryPolicy (Some
/// Backend)` (:76). Same field shape as the writes twin.
pub struct RetryReadsBackend<R> {
    /// The retry policy (:76).
    pub policy: RetryPolicy,
    /// The wrapped backend (`Some Backend`, :76).
    pub backend: R,
}

/// `backendRetryReads :: Backend b => b -> RetryPolicy ->
/// RetryReadsBackend` (:80-81) — argument order kept.
pub fn backend_retry_reads<R: Backend>(backend: R, policy: RetryPolicy) -> RetryReadsBackend<R> {
    RetryReadsBackend { policy, backend }
}

/// The class instance (:83-129). Partition: 10 retried (the READ-side
/// operations), 19 forwarded — every one of the 29 methods enumerated
/// with its source line. NOTE deriveStored is FORWARDED here (:104)
/// while the writes twin RETRIES it (:38-39) — the asymmetry is the
/// spec.
impl<R: Backend> Backend for RetryReadsBackend<R> {
    // retry (:84-85)
    fn query_fact(
        &self,
        repo: &thrift::Repo,
        id: thrift::Id,
    ) -> Result<Option<thrift::Fact>, BackendError> {
        retry_channel_exceptions(&self.policy, || {
            self.backend
                .query_fact(repo, id)
                .map_err(backend_error_for_retry)
        })
        .map_err(retry_error_to_backend)
    }

    // retry (:86-87)
    fn fact_id_range(&self, repo: &thrift::Repo)
        -> Result<thrift::FactIdRange, BackendError> {
        retry_channel_exceptions(&self.policy, || {
            self.backend
                .fact_id_range(repo)
                .map_err(backend_error_for_retry)
        })
        .map_err(retry_error_to_backend)
    }

    // retry (:88-89)
    fn get_schema_info(
        &self,
        repo: Option<&thrift::Repo>,
        request: thrift::GetSchemaInfo,
    ) -> Result<thrift::SchemaInfo, BackendError> {
        retry_channel_exceptions(&self.policy, || {
            self.backend
                .get_schema_info(repo, request.clone())
                .map_err(backend_error_for_retry)
        })
        .map_err(retry_error_to_backend)
    }

    // retry (:90-91)
    fn validate_schema(&self, request: thrift::ValidateSchema)
        -> Result<(), BackendError> {
        retry_channel_exceptions(&self.policy, || {
            self.backend
                .validate_schema(request.clone())
                .map_err(backend_error_for_retry)
        })
        .map_err(retry_error_to_backend)
    }

    // retry (:92-93)
    fn predicate_stats(
        &self,
        repo: &thrift::Repo,
        opts: StackedDbOpts,
    ) -> Result<BTreeMap<thrift::Id, thrift::PredicateStats>, BackendError> {
        retry_channel_exceptions(&self.policy, || {
            self.backend
                .predicate_stats(repo, opts)
                .map_err(backend_error_for_retry)
        })
        .map_err(retry_error_to_backend)
    }

    // retry (:94-95)
    fn list_databases(&self, request: thrift::ListDatabases)
        -> Result<thrift::ListDatabasesResult, BackendError> {
        retry_channel_exceptions(&self.policy, || {
            self.backend
                .list_databases(request.clone())
                .map_err(backend_error_for_retry)
        })
        .map_err(retry_error_to_backend)
    }

    // retry (:96-97)
    fn get_database(&self, repo: &thrift::Repo)
        -> Result<thrift::GetDatabaseResult, BackendError> {
        retry_channel_exceptions(&self.policy, || {
            self.backend
                .get_database(repo)
                .map_err(backend_error_for_retry)
        })
        .map_err(retry_error_to_backend)
    }

    // retry (:98-99)
    fn user_query_facts(
        &self,
        repo: &thrift::Repo,
        request: thrift::UserQueryFacts,
    ) -> Result<thrift::UserQueryResults, BackendError> {
        retry_channel_exceptions(&self.policy, || {
            self.backend
                .user_query_facts(repo, request.clone())
                .map_err(backend_error_for_retry)
        })
        .map_err(retry_error_to_backend)
    }

    // retry (:100-101)
    fn user_query(
        &self,
        repo: &thrift::Repo,
        request: thrift::UserQuery,
    ) -> Result<thrift::UserQueryResults, BackendError> {
        retry_channel_exceptions(&self.policy, || {
            self.backend
                .user_query(repo, request.clone())
                .map_err(backend_error_for_retry)
        })
        .map_err(retry_error_to_backend)
    }

    // retry (:102-103)
    fn user_query_batch(
        &self,
        repo: &thrift::Repo,
        request: thrift::UserQueryBatch,
    ) -> Result<Vec<thrift::UserQueryResultsOrException>, BackendError> {
        retry_channel_exceptions(&self.policy, || {
            self.backend
                .user_query_batch(repo, request.clone())
                .map_err(backend_error_for_retry)
        })
        .map_err(retry_error_to_backend)
    }

    // forward (:104) — THE ASYMMETRY: the writes twin retries this
    // method (:38-39); reads pass it through untouched.
    fn derive_stored(
        &self,
        log: LogDerivationResult,
        repo: &thrift::Repo,
        query: thrift::DerivePredicateQuery,
    ) -> Result<thrift::DerivationStatus, BackendError> {
        self.backend.derive_stored(log, repo, query)
    }

    // forward (:106)
    fn kick_off_database(&self, request: thrift::KickOff)
        -> Result<thrift::KickOffResponse, BackendError> {
        self.backend.kick_off_database(request)
    }

    // forward (:107)
    fn finish_database(&self, repo: &thrift::Repo)
        -> Result<thrift::FinishDatabaseResponse, BackendError> {
        self.backend.finish_database(repo)
    }

    // forward (:108)
    fn finalize_database(&self, repo: &thrift::Repo)
        -> Result<thrift::FinalizeResponse, BackendError> {
        self.backend.finalize_database(repo)
    }

    // forward (:109)
    fn update_properties(
        &self,
        repo: &thrift::Repo,
        properties: thrift::DatabaseProperties,
        names: Vec<String>,
    ) -> Result<thrift::UpdatePropertiesResponse, BackendError> {
        self.backend.update_properties(repo, properties, names)
    }

    // forward (:110-111)
    fn complete_predicates_(
        &self,
        repo: &thrift::Repo,
        predicates: thrift::CompletePredicates,
    ) -> Result<thrift::CompletePredicatesResponse, BackendError> {
        self.backend.complete_predicates_(repo, predicates)
    }

    // forward (:113)
    fn restore_database(&self, backup: &str) -> Result<(), BackendError> {
        self.backend.restore_database(backup)
    }

    // forward (:114)
    fn delete_database(&self, repo: &thrift::Repo)
        -> Result<thrift::DeleteDatabaseResult, BackendError> {
        self.backend.delete_database(repo)
    }

    // forward (:116)
    fn enqueue_batch(&self, batch: thrift::ComputedBatch)
        -> Result<thrift::SendResponse, BackendError> {
        self.backend.enqueue_batch(batch)
    }

    // forward (:117)
    fn enqueue_json_batch(
        &self,
        repo: &thrift::Repo,
        batch: thrift::SendJsonBatch,
    ) -> Result<thrift::SendJsonBatchResponse, BackendError> {
        self.backend.enqueue_json_batch(repo, batch)
    }

    // forward (:118-119)
    fn enqueue_batch_descriptor(
        &self,
        repo: &thrift::Repo,
        batch: thrift::EnqueueBatch,
        wait_policy: thrift::EnqueueBatchWaitPolicy,
    ) -> Result<thrift::EnqueueBatchResponse, BackendError> {
        self.backend.enqueue_batch_descriptor(repo, batch, wait_policy)
    }

    // forward (:120)
    fn poll_batch(
        &self,
        repo: Option<&thrift::Repo>,
        handle: thrift::Handle,
    ) -> Result<thrift::FinishResponse, BackendError> {
        self.backend.poll_batch(repo, handle)
    }

    // forward (:121)
    fn wait_for_writes(&self, repo: &thrift::Repo)
        -> Result<thrift::WaitForWritesResponse, BackendError> {
        self.backend.wait_for_writes(repo)
    }

    // forward (:123)
    fn display_backend(&self) -> String {
        self.backend.display_backend()
    }

    // forward (:124)
    fn has_database(&self, repo: &thrift::Repo) -> Result<bool, BackendError> {
        self.backend.has_database(repo)
    }

    // forward (:125)
    fn schema_id(&self) -> Option<thrift::SchemaId> {
        self.backend.schema_id()
    }

    // forward (:126)
    fn using_shards(&self) -> bool {
        self.backend.using_shards()
    }

    // forward (:127) — inventory.tsv:7 == inventory.protocol.tsv:23.
    fn init_global_state(&self)
        -> Result<(GleanGetState, GleanQueryState), BackendError> {
        self.backend.init_global_state()
    }

    // re-wrap (:128-129): policy preserved, inner re-wrapped.
    fn backend_with_client_id(self, client_id: &str) -> Self {
        let RetryReadsBackend { policy, backend } = self;
        RetryReadsBackend {
            policy,
            backend: backend.backend_with_client_id(client_id),
        }
    }
}

// ===========================================================================
// Coverage / markers / status
// ===========================================================================
//
// Coverage: the 129-line source is fully accounted for — the two
// records (:20, :76) -> the two structs (pub fields per the exported
// constructors, :10/:12); the two smart constructors (:24-25, :80-81)
// -> `backend_retry_writes`/`backend_retry_reads` (argument order kept;
// the `Some` wrapping lands at the SomeBackend enum layer, decision
// (B)); both class instances (:27-74, :83-129) -> method-for-method
// impls with ALL 29 methods each and the retry/forward partition cited
// per method (11 retried in writes, 10 in reads, 21 retry sites total —
// matching the count the amended util surface's header cites for this
// file); backendWithClientId (:73-74, :128-129) -> the re-wrapping
// by-value method in both. The import block (:16-18) is disposed in
// DEPENDENCY TRUTH (the Glean.Util.Some edge indirectly, through the
// types.rs-owned enum). Nothing is dropped, stubbed, or deferred.
//
// Marker breakdown (closed set — zero of every class):
//   * port todos: zero. The one novel mechanism (the NotRetried box +
//     downcast round trip) is a decided, probe-verified image of the
//     source's dynamic-exception pass-through, not an open question;
//     the taxonomy todos that touch this surface belong to types.rs
//     (:407-414) and thrift_service.rs (:311-322) and are cited, not
//     duplicated.
//   * source-defect reproductions: zero — the source is total (no
//     partial functions, no `error`, no lazy IO; the only throw is the
//     retry loop's own exhaustion rethrow, which the consumed function
//     owns).
//   * performance deferrals: zero — each retried body pays one closure
//     call and (where arguments are owned) the clone the source's
//     re-executed action pays semantically; the Arc adapter in
//     derive_stored is one refcount bump per attempt against the
//     source's shared function value.
//   * safety comments: zero — no unsafe.
//
// No inventory divergence: both governing rows are followed verbatim
// (INVENTORY block); the two source sites of the group row
// (inventory.sites.tsv:535/:536) are the init_global_state forwards.
//
// PORT STATUS: confidence=high todos=0
