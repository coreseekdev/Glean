// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// Ported from: glean/util/Glean/Util/ThriftService.hs (57 lines)
// Manifest unit: rust/glean-util/src/util/thrift_service.rs
// (migration/manifest.tsv line 40, crate glean-util). RULE §4: one source
// file -> one target file.
//
// `IsThriftService` is the seam between the Glean client and "whatever
// speaks thrift at the other end": the class a concrete service
// implements (the OSS thrift-over-HTTP service, Impl/ThriftService.hs,
// manifest row 166), the constructor helper, and the run-anywhere runner.
// The source module exports exactly:
//
//   * `ThriftServiceOptions(..)` — creation options (:27-30) plus the
//     `Default` instance (:32-33);
//   * `DbShard`                  — number-as-text alias (:35-36);
//   * `IsThriftService(..)`      — the class (:39-50), five methods;
//   * `thriftService`            — `mkThriftService` at `def` (:52-53);
//   * `runThriftInefficiently`   — event-base bracket per call (:55-57).
//
// Export-surface mapping (Haskell -> Rust):
//   ThriftServiceOptions(..) -> `pub struct ThriftServiceOptions` (both
//       fields pub: `queueTimeout` is imported AS A SELECTOR by
//       Write.hs:35 and record-updated at Write.hs:289);
//   IsThriftService(..)      -> `pub trait IsThriftService<S>`: `Self`
//       is the source's `t s`, the class's free `s` is the trait's `S`
//       (decision (c)); default methods translate to default bodies;
//   thriftService            -> `thrift_service` (decision (f));
//   DbShard                  -> `pub type DbShard = String` (decision (b));
//   runThriftInefficiently   -> `run_thrift_inefficiently` (decision (f)).
//
// Import surface (Haskell -> Rust):
//   import Data.Default     -> `impl Default` (:17, :32-33); `def ::
//       Maybe Double` is `Nothing` (probe:
//       /tmp/opencode/ts-probe/def_probe.txt; decision (a)); the in-tree
//       `Data.Default` image is std `Default` (define.rs:386-412);
//   import Data.Text (Text) -> `String` (RULE §1 strings row);
//   import Thrift.Api       -> ASSUMED-IMPORT, first sketch below: the
//       `Thrift s a` monad, its env, and the hsthrift error types live in
//       the assumed `glean_thrift` runtime (no manifest unit; hsthrift
//       dependency, `~/glean-build/Glean/hsthrift/lib/Thrift/Api.hs`);
//   import Util.EventBase   -> ASSUMED-IMPORT, second block below:
//       `EventBaseDataplane` + the raw FFI init/destroy pair are the
//       kit's; THIS file owns the bracket image
//       (`with_event_base_dataplane`, module-owned and pub — decision (e));
//   import Glean.Util.Service -> `crate::util::service` (row 39, shipped
//       as src/util/service.rs): `Service` (service.rs:153) and the
//       aliases `HostName`/`PortNumber` (service.rs:160, :164) are
//       imported, never redeclared.
//
// Consumers (source importers, verified by grep — the inbound set this
// export surface must keep satisfiable):
//   util/Glean/Impl/ThriftService.hs (row 166)  — the OSS instance
//       (:85-132);
//   client/hs/Glean/Remote.hs (row 167)         — `mkThriftService`
//       :162-181, `runThrift` call sites :318/:320-321/:329,
//       ChannelException fallback :303-312, `thriftServiceWithClientId`
//       :288-291;
//   tools/gleancli/GleanCLI/Write.hs (row 381)  — the `queueTimeout`
//       selector (:35, :289);
//   backend-api/Glean/Backend/Types.hs (row 41) — `DbShard` (:73, :470);
//   server/Glean/Server/Sharding.hs (row 356)   — `DbShard` (:52, :141);
//   glass/tools/.../DemoClient.hs               — whole module (a tool
//       without a manifest row; kept compiling by the same surface).
//
// Decisions, with evidence:
//
// (a) Types and derives. `Maybe Double` -> `Option<f64>` (`Double` IS
//     IEEE-754 binary64, and `f64` is exactly that type — no conversion
//     semantics are involved; RULE §2's numeric row governs conversions
//     (`fromIntegral` etc.), of which there are none here). `deriving
//     Show` -> `#[derive(Debug)]` per the port-wide Debug-vs-Show
//     convention (angle/types.rs:77-90). `Clone` is Rust plumbing for
//     the source's immutable value semantics (record-update consumers:
//     Remote.hs:172-181, Write.hs:289) — not an observable addition.
//     `instance Default ThriftServiceOptions where def =
//     ThriftServiceOptions def def` (:32-33) -> `impl Default` with
//     `None`/`None`: `def :: Maybe Double` is `Nothing` (GHC 9.4.7 /
//     base-4.17.2.0 probe: /tmp/opencode/ts-probe/def_probe.txt), and std
//     `Default` is the in-tree `Data.Default` image (define.rs:386-412,
//     `DefineFlags`).
//
// (b) `type DbShard = Text` -> `pub type DbShard = String` (RULE §1
//     strings row). The source comment "should be a number, I think"
//     (:36) is transcribed as a doc note: a convention about the string's
//     content, unenforced by the type — and unenforced here.
//
// (c) The class: `class IsThriftService t` with its methods used at
//     `t s`. Rust has no higher-kinded types, so the image dissolves one
//     application: `Self` is `t s`, and the free `s` becomes the trait's
//     type parameter `S`. One impl per service type, at its own tag —
//     row 166 is `t s` at `s = GleanService`, pinned by a struct field
//     (Remote.hs:92-95, `thriftBackendService :: ThriftService
//     GleanService`). The shape was rustc-probed: without the tag in the
//     service type, the faithful all-`s` instance form (a blanket impl —
//     Haskell has one `instance IsThriftService ThriftService`, which
//     must hold at every `s`) leaves `S` un-inferable at construction:
//     E0282 (/tmp/opencode/ts-probe/tag_shape_blanket_e0282.rs); a
//     per-tag impl infers `S` from the unique candidate, but then the
//     service type works at one tag only (it compiles and runs —
//     /tmp/opencode/ts-probe/tag_shape_per_tag.rs). Hence the phantom
//     tag, mirroring row 166's own `HTTPConfig p`
//     (Impl/ThriftService.hs:85-87). The `Clone` supertrait is Rust
//     plumbing for `-> Self` returns and the default body, not a
//     semantic addition. For an
//     implementor whose tag is a phantom: write the `Clone` impl by
//     hand (or a helper derive) — deriving `Clone` on the parameterized
//     type adds a spurious `S: Clone` bound the tag need not satisfy
//     (probe-observed: the manual impl is what makes the supertrait
//     hold at every tag).
//     Receivers: the transformers are `t s -> ... -> t s`, pure
//     value-in/value-out in the source (`thriftServiceWithDbShard t _ =
//     t`, row 166, does not consume `t`) -> `&self -> Self` with an
//     internal clone (the old value stays live, as in Haskell); the
//     runners -> `&self` (they only call methods on it).
//
//     Errors: both runners are `IO a` — they THROW. Per RULE §2's
//     dynamic-exceptions row Rust has no exceptions, so they return
//     `Result<_, ThriftError>`; the in-tree precedent for "Haskell
//     throws -> Rust returns Err" is ffi.rs:394-402 (`invoke` ->
//     `Result<(), FfiError>`, defined :316-320). The signature change is
//     not a semantic change: throws are `IO`'s failure channel, and no
//     caller can observe "returned vs threw" for a value of type `a`.
//
// (d) `Thrift s a` (Api.hs:17-20) -> the ASSUMED `ThriftAction<S, A>`
//     (first sketch below). `forall p c . (Protocol p, ClientChannel c)
//     =>` has no Rust image short of a generic METHOD: `fn run<P:
//     Protocol, C: ClientChannel<S>>(self, env: ...)`. Generated code is
//     structured exactly so (it is polymorphic in `p` —
//     `getSchemaInfo` :111-119, `getSchemaInfoIO` :121-135 — and calls
//     `Thrift.genStruct _proxy` at :176, :303, :411; gen-hs2
//     Glean/if/gen-hs2/Glean/GleanService/Client.hs, builder :170-183,
//     transport-error wrap :160-167).
//     The trait must live in `glean_thrift`, not here: generated code
//     imports the `Thrift.Api` image (Client.hs:99-101 `Thrift.Codegen` /
//     `Thrift.Protocol.ApplicationException.Types`) and generated code
//     cannot depend on glean-util
//     (dependency direction — the same reason service.rs:153 re-exports
//     generated types instead of defining them). The OSS transport picks
//     `p` dynamically from the runtime protocol id (`withProxy`,
//     Protocol/Id.hs:31-35, HTTP.hs:50) — in Rust that becomes a match
//     on the id choosing the concrete `P` for `run::<P, _>`; the
//     rank-2 image makes that a static dispatch point, which is exactly
//     why it is a generic method and not `dyn`.
//
// (e) `EventBaseDataplane`/`initExecutor`/`destroyExecutor` come from
//     `Util.EventBase` (no manifest unit): the handle type and the raw
//     FFI pair are the kit's (second sketch below); the bracket itself
//     (`withEventBaseDataplane = bracket initExecutor destroyExecutor`,
//     EventBase.hs:57-60) is THIS file's module-owned image, `pub` like
//     service.rs's `parse_uri` (service.rs:647): HANDOFF §4.2 requires
//     the unit to "locate/own their Rust images ... one owner, no forks",
//     and other units' ports consume the bracket (call sites below). The
//     `Drop` guard is Rust's bracket release: it runs on normal return
//     AND on panic-unwind (rustc probe: 2/2 destroys, normal + panic).
//     There is NO `unsafe` in this file (RULE §3); the raw FFI lives
//     with the kit-side owner noted in that sketch's block.
//
// (f) The two adapters keep the source's shapes and argument order; the
//     `Service` parameters are taken by reference (`&Service`) like the
//     sibling's interpreters (service.rs:205-215) — no path consumes the
//     union, and reference receivers are call-site plumbing with no
//     Haskell observable. `runThriftInefficiently`'s deliberately-verbose
//     name is kept verbatim: it means "create and destroy an event base
//     per call", which is exactly what the Rust bracket does.
//
// (g) Not-ported inventory: nothing is dropped or stubbed. The source
//     has no partiality (no `error`, no wildcards over meaning), no
//     language pragmas, no CPP. `mkThriftService`'s `Nothing` arm is
//     total by type. No source-defect reproduction arises.
//
// ---------------------------------------------------------------------------
// ASSUMED-IMPORT (RULE §1 thrift row (A10) + §4 B4): the `Thrift.Api`
// runtime surface.
//
// This is the FIRST SKETCH of the runtime half of the generated crate:
// no earlier unit sketched it (grep over rust/: zero `ThriftAction` /
// `ClientChannel` / `ThriftEnv` / `ThriftError` mentions before this
// file). Row 166 (impl/thrift_service.rs: the HTTP channel + concrete
// run) and the kit unit extend it, recording extensions in their own
// blocks and cross-referencing this sketch — never a second owner.
//
// Pinned hsthrift evidence (the build tree GHC probes compile against):
//   * Api.hs:17-20      `type Thrift s a = forall p c . (Protocol p,
//                       ClientChannel c) => ThriftM p c s a`;
//   * Monad.hsc:34-41   `ThriftM p c s = ReaderT (ThriftEnv p c s) IO`,
//                       `ThriftEnv { thriftProxy :: Proxy p, thriftChannel
//                       :: c s, thriftRpcOpts :: RpcOptions, thriftCounter
//                       :: Counter }`;
//   * Monad.hsc:61-66   `runThrift`/`runThriftWith` (the composition the
//                       concrete service performs);
//   * Monad.hsc:99-107  `type Counter = IO Int32` over an IORef,
//                       increment-on-read starting at 0;
//   * Channel.hs:30-42  `class ClientChannel (c :: * -> *)` (sendRequest/
//                       sendOnewayRequest with callbacks; `c t` for any
//                       service tag `t`);
//   * Channel.hs:81-83  `newtype ChannelException = ChannelException Text
//                       deriving (Show, Eq)` + `instance Exception`;
//   * Protocol.hs:83-85 `newtype ProtocolException = ProtocolException
//                       String` + `instance Exception`;
//   * Protocol.hs:105-201 `class Protocol p` (generators + parsers);
//   * hsthrift/lib/if/RpcOptions.thrift (:14-28): generated `Priority`
//                       enum (:14-20) + `RpcOptions` struct (:22-28;
//                       timeout/priority/chunkTimeout/queueTimeout/
//                       headers, `hs.prefix = "rpc_"`), generated into the
//                       runtime (gen-hs2 Thrift/Protocol/RpcOptions/Types.hs);
//   * hsthrift/lib/if/ApplicationException.thrift (:13-28 enum,
//                       :30-33 exception): generated
//                       `ApplicationExceptionType` (Unknown..InjectedFailure)
//                       + `exception ApplicationException { message, type }`.
// The assumed Rust image, flat at the crate root per the in-tree
// convention (`glean_thrift::{...}` — same as service.rs:153):
//
//     // The `Thrift s a` image: a command carrying its request, with a
//     // rank-2 runner. `P`/`C` are chosen by the CALLER of `run` (the
//     // concrete service), exactly like `runThriftWith` instantiates the
//     // `forall p c` of a Haskell `Thrift` value.
//     //
//     // CONTRACT — actions are SINGLE-USE: `run(self)` consumes the
//     // command (the source's `Thrift s a` is a reusable pure value).
//     // A site that feeds ONE action to TWO runs — unit 167's
//     // `Remote.withShard` fallback does exactly that (Remote.hs:318/
//     // :320-321), the DEFAULT USE_SHARDS_AND_FALLBACK mode — must add
//     // `+ Clone` at its own call site and clone once.
//     // /tmp/opencode/ts-probe/action_reuse_e0382.rs reproduces the
//     // move: `error[E0382]: use of moved value`, pointing at this
//     // file's `action: Action`; action_reuse_clone_ok.rs in the same
//     // directory is the same shape with the clone-once fix, and runs.
//     pub trait ThriftAction<S, A> {
//         fn run<P: Protocol, C: ClientChannel<S>>(
//             self,
//             env: ThriftEnv<P, C, S>,
//         ) -> Result<A, ThriftError>;
//     }
//
//     pub struct ThriftEnv<P, C, S> {
//         pub protocol: std::marker::PhantomData<P>, // thriftProxy
//         pub channel: C,                            // thriftChannel :: c s
//         pub rpc_opts: RpcOptions,                  // thriftRpcOpts
//         pub counter: Counter,                      // thriftCounter
//         pub service: std::marker::PhantomData<S>,  // the `s` of `c s`
//     }
//     pub struct Counter(/* AtomicI32, starts at 0 */);
//     impl Counter { pub fn next(&self) -> i32 { /* read, then +1 */ } }
//
//     // Class images; the method SETS are the kit's to pin (sendRequest/
//     // sendOnewayRequest callback plumbing, generator/parser calls); this
//     // sketch pins only the shapes the trait above needs:
//     pub trait Protocol { /* Protocol.hs:105-201, kit-owned */ }
//     pub trait ClientChannel<S> { /* Channel.hs:30-42, kit-owned */ }
//
//     // `RpcOptions`/`Priority`/`ApplicationException` are generated from
//     // the two runtime IDL files above (never hand-translated, RULE §1
//     // thrift row) — default RpcOptions is all-zero/None (Monad.hsc:44-50).
//
//     // The error union: hsthrift throws per-exception types (Channel.hs:
//     // 81-83, Protocol.hs:83-85, ApplicationException.thrift) and leaks
//     // everything else as `SomeException`; RULE §2's dynamic-exceptions
//     // row collapses them into one typed enum:
//     #[derive(Debug)]
//     pub enum ThriftError {
//         /// Channel.hs:81-83 — the retryable class. inventory.tsv:13
//         /// (Haxl.hs EXC_TAXONOMY) requires "a distinct ChannelError
//         /// variant" for the retry policy; Remote.hs:307-312 pattern-
//         /// matches it and RetryChannelException.hs:54-69 retries on it
//         /// only. `Show` text is load-bearing for Remote.hs:309's
//         /// `show e` log (probe: GHC renders the newtype as
//         /// `ChannelException "SELECTION_NO_HOST_FOR_SHARD"`).
//         Channel(ChannelException),
//         /// Protocol.hs:83-85 — decode/parse failures; generated code
//         /// constructs it (Client.hs:160-167).
//         Protocol(ProtocolException),
//         /// ApplicationException.thrift:13-28/:30-33 — thrift msg-type-3
//         /// application exceptions (Client.hs:229-231 parses them); the
//         /// glass classify table discriminates the Timeout variant
//         /// (ErrorClassification.hs:38-39, inventory.tsv:54).
//         Application(ApplicationException),
//         /// Open set: IDL-declared per-method exceptions (seventeen in
//         /// glean/if/*.thrift alone: glean.thrift:225-266 BadQuery/
//         /// Retry/UnknownDatabase/..., :602-618; index.thrift:63) plus
//         /// anything else not discriminated by name. RULE §2's GADT
//         /// row: `Box<dyn Trait>` "only where genuinely open" — this
//         /// set grows with the IDLs. (ffi.rs:316-320 is the in-crate
//         /// thiserror precedent for the derive style; constructor-shaped
//         /// `Display` per §2's deriving-Show row and the
//         /// `ChannelException "msg"` probe.)
//         Service(Box<dyn std::error::Error + Send + Sync>),
//     }
//     #[derive(Clone, Debug, PartialEq, Eq)]
//     pub struct ChannelException(pub String);   // newtype over Text
//     #[derive(Clone, Debug, PartialEq, Eq)]
//     pub struct ProtocolException(pub String);  // newtype over String
//
//     // `Show`-text parity + error plumbing — the kit extension owes
//     // these (§2's deriving-Show row; constructor-shaped, reproducing
//     // the `ChannelException "…"` probe's text; Remote.hs:309 logs
//     // `show e`; thiserror style, ffi.rs:316-320):
//     impl std::fmt::Display for ChannelException {
//         fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//             write!(f, "ChannelException {:?}", self.0)
//         }
//     }
//     impl std::error::Error for ChannelException {}
//     // …same for ProtocolException/ApplicationException; `ThriftError`'s
//     // `Display`/`Error` delegate to the contained leg.
//
// TODO(port): the transport/IO error leg. hsthrift's OSS transport
// throws Network.HTTP.Simple's HttpException out of `httpLbs` (HTTP.hs:
// 98) and wraps ONLY non-200 statuses into ChannelException (HTTP.hs:
// 101-106). No Glean consumer discriminates the former by name (only the
// server-side does, Glean/Server.hs:174-176), and the retry policy is
// ChannelException-scoped, so the CONSERVATIVE image folds transport/IO
// errors into `Service(..)` (retry-ineligible, matching the source) —
// but that is a placement, not a decision. Open question (kit, Step 4/5,
// + row 166): once the sync thrift-over-HTTP transport is pinned (RULE §1
// thrift row defers it to the Step-5 live handshake), decide whether the
// IO leg gets a dedicated variant, and record the extension in the
// kit's/row-166's assumed block.
//
// ---------------------------------------------------------------------------
// ASSUMED-IMPORT (RULE §4 B4): `Util.EventBase`'s handle + release pair.
//
// (No manifest unit; defined in the hsthrift dependency,
// `~/glean-build/Glean/hsthrift/common/util/Util/EventBase.hs`.)
//
// Pinned evidence:
//   * EventBase.hs:37    `newtype EventBaseDataplane = EventBaseDataplane
//                        (Ptr CppIOExecutor)` — an abstract handle, used
//                        opaquely across glean/ (no `.hs` consumer calls
//                        `getExecutor`/`getEventBaseDataplane`; in the OSS
//                        path the executor's only visible effect is
//                        thread-pool create/join at init/destroy);
//   * EventBase.hs:57-60 `withEventBaseDataplane = bracket initExecutor
//                        destroyExecutor`;
//   * EventBase.hs:62-66 `initExecutor = EventBaseDataplane <$>
//                        c_newExecutor`; `destroyExecutor (...) =
//                        c_destroyExecutor ptr`;
//   * EventBase.hs:75-79 the foreign imports
//                        `common_hs_eventbase_newExecutor` /
//                        `common_hs_eventbase_destroyExecutor`;
//   * common/util/cpp/EventBaseDataplane.cpp:16-20 `new
//                        folly::IOThreadPoolExecutor` under `noexcept` —
//                        allocation-guard (OOM aborts; no error value
//                        exists — RULE §2 error-recovery rule), so the
//                        Rust `init_executor` is infallible, NOT a
//                        Result; :22-27 `join(); delete` under `noexcept`
//                        — the destroy BLOCKS until the pool's threads
//                        exit.
//
// Kit contract (glean_thrift, per decision (e)):
//
//     pub struct EventBaseDataplane { /* kit-private */ }
//     pub fn init_executor() -> EventBaseDataplane;      // infallible; aborts on OOM
//     pub fn destroy_executor(evb: EventBaseDataplane);  // CONSUMES; joins + deletes
//
// `EventBaseDataplane` must NOT carry a `Drop` impl in the kit: the
// single releaser is the bracket in this file (a kit-side Drop would
// double-free under it — the same single-owner discipline ffi.rs:198
// documents for its `FreeOnDrop`). The kit side is where the raw binding
// lives; if it keeps it FFI-backed, that is the same §3-letter situation
// the migration already tracks as DEV-006 (an "already FFI in the
// Haskell" image outside glean-storage/glean-core, like glean-typed's
// build.rs precedent) — the flag belongs in the assumed block that ends
// up owning the kit surface (this file's, row 166's extension, or the
// Step-4 scaffold's), per RULE §4 B4's "records the extension in its own
// block"; do not absorb it silently here. Whether the image stays an
// FFI-backed folly executor (thread-pool parity) or becomes an opaque
// token once the sync Rust transport lands (RULE §1's async-runtime row
// bans tokio outside front-ends; the OSS HTTP channel never touched the
// executor) is a Step-4/5 kit decision, recorded there — either way this
// file only ever sees `&EventBaseDataplane`, so nothing here depends on
// the answer. KIT NOTE: in-tree consumers store the handle by value
// (Remote.hs:94, Glass/Env.hs:109, Sharding.hs:139/:194,
// GleanCLI/Types.hs:54/:65), so the kit unit must pin store-ownership
// before those units land: lifetime threading or a scoped-owner model —
// NOT `Clone` (two owned handles would mean two `destroy_executor`
// calls, i.e. a double free).
//
// `withEventBaseDataplane`'s image is module-owned (this file) and `pub`
// per decision (e); the source's other call sites are why consumers need
// it. 22 files carry an actual `withEventBaseDataplane $` call (grep over
// glean/ --include=*.hs: 21 besides this module, whose own use is :57); a
// representative sample: FactSetBench.hs:179, Example.hs:61,
// ExampleWriter.hs:48, Hyperlink.hs:536, Server.hs:81, Glass/Main.hs:119,
// GleanCLI.hs:164, LoadGenerator.hs:150, Schema/Basic.hs:348,
// GracefulShutdown.hs:163. Two further matches are doc-comment mentions
// (Glean.hs:25, LocalOrRemote.hs:25), tracked separately.
// Transcribed as an `Option`+`Drop` guard: Rust's structural `finally`,
// running the release on normal return AND panic-unwind (the case
// `bracket` handles that Rust can have; async-exception masking has no
// Rust analogue and no std-async cancellation to mirror).

// ---------------------------------------------------------------------------
// Imports (RULE §4): `use crate::…` within the crate; assumed kit surface
// flat at the generated crate's root, exactly as service.rs:153 consumes it.
// ---------------------------------------------------------------------------

use crate::util::service::{HostName, PortNumber, Service};
use glean_thrift::{destroy_executor, init_executor, EventBaseDataplane, ThriftAction, ThriftError};

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

/// Options for creating a Thrift service (ThriftService.hs:26-30).
#[derive(Clone, Debug)]
pub struct ThriftServiceOptions {
    /// `processingTimeout`, in seconds.
    pub processing_timeout: Option<f64>,
    /// `queueTimeout`, in seconds.
    pub queue_timeout: Option<f64>,
}

/// `instance Default ThriftServiceOptions where def = ThriftServiceOptions
/// def def` (:32-33): both fields' `def :: Maybe Double` is `Nothing`
/// (pinned `Data.Default`; /tmp/opencode/ts-probe/def_probe.txt).
/// `Data.Default` -> std `Default` (define.rs:386-412, decision (a)).
impl Default for ThriftServiceOptions {
    fn default() -> Self {
        ThriftServiceOptions {
            processing_timeout: None,
            queue_timeout: None,
        }
    }
}

/// Used by the Glean client to request a node containing a specific db
/// (:35-36). Source note, transcribed: "should be a number, I think".
pub type DbShard = String;

// ---------------------------------------------------------------------------
// The class
// ---------------------------------------------------------------------------

/// A Thrift service that can be called (ThriftService.hs:39-50).
///
/// `S` is the source class's free `s` (`Thrift s a`'s service tag):
/// a concrete service type implements this trait once, at its own tag
/// (decision (c)). The OSS instance is Impl/ThriftService.hs:101-132.
pub trait IsThriftService<S>: Clone {
    /// Nothing means this Service is not supported. (:40-41)
    fn mk_thrift_service(service: &Service, options: ThriftServiceOptions) -> Option<Self>;

    /// Request a node with a specific Db. (:42-43)
    ///
    /// `&self -> Self` per decision (c): the source is a pure
    /// value-in/value-out transformer; the clone is internal.
    fn thrift_service_with_db_shard(&self, shard: Option<DbShard>) -> Self;

    /// Tag outbound connections with a ServiceRouter `client_id` for
    /// per-caller attribution. Default is a no-op for services that
    /// don't support it. (:44-48)
    fn thrift_service_with_client_id(&self, _client_id: Option<String>) -> Self {
        self.clone()
    }

    /// `runThrift :: EventBaseDataplane -> t s -> Thrift s a -> IO a`
    /// (:49): the action is the assumed `ThriftAction<S, A>` image of
    /// `Thrift s a` (decision (d)); the source's `IO a` throws, so the
    /// Rust surface is `Result<A, ThriftError>` (decision (c)). The action
    /// is CONSUMED (single-use, `run(self)`): a site that runs one action
    /// twice must add `+ Clone` and clone once — see the `ThriftAction`
    /// sketch's CONTRACT note; unit 167's `withShard` fallback is the
    /// named case (Remote.hs:318/:320-321).
    fn run_thrift<A, Action: ThriftAction<S, A>>(
        &self,
        evb: &EventBaseDataplane,
        action: Action,
    ) -> Result<A, ThriftError>;

    /// `getSelection :: EventBaseDataplane -> t s -> Int -> IO
    /// [(HostName,PortNumber)]` (:50). `Int` maps to `i64` — the same
    /// mapping the sibling pinned for `PortNumber` (service.rs:164).
    fn get_selection(
        &self,
        evb: &EventBaseDataplane,
        n: i64,
    ) -> Result<Vec<(HostName, PortNumber)>, ThriftError>;
}

// ---------------------------------------------------------------------------
// Constructors / runners
// ---------------------------------------------------------------------------

/// `thriftService svc = mkThriftService svc def` (:52-53).
pub fn thrift_service<S, T: IsThriftService<S>>(service: &Service) -> Option<T> {
    T::mk_thrift_service(service, ThriftServiceOptions::default())
}

/// `withEventBaseDataplane` (Util/EventBase.hs:57-60), module-owned image
/// (decision (e)): init, run `f`, destroy on every exit path. The borrow
/// of the handle is scoped to the dynamic extent of `f` — a STRENGTHENING
/// of the source's documentation discipline, not of an enforcement its
/// type makes: `bracket :: IO a -> (a -> IO b) -> IO b` scopes nothing and
/// the handle can escape it (GHC probe,
/// /tmp/opencode/ts-probe/escape_probe.txt: the handle-returning case plus
/// a second `destroyExecutor` type-checks). All in-tree call sites use the
/// closure form (call-site inventory above), so parity holds.
pub fn with_event_base_dataplane<R>(f: impl FnOnce(&EventBaseDataplane) -> R) -> R {
    struct DestroyOnDrop(Option<EventBaseDataplane>);
    impl Drop for DestroyOnDrop {
        fn drop(&mut self) {
            // Runs on normal return AND panic-unwind — Rust's `finally`.
            if let Some(evb) = self.0.take() {
                destroy_executor(evb);
            }
        }
    }
    let guard = DestroyOnDrop(Some(init_executor()));
    let evb = guard
        .0
        .as_ref()
        .expect("the guard holds the executor until drop");
    f(evb)
}

/// `runThriftInefficiently ts action = withEventBaseDataplane $ \evb ->
/// runThrift evb ts action` (:55-57): one executor per call, by design
/// (the name says so; decision (f)).
pub fn run_thrift_inefficiently<S, T, A, Action>(ts: &T, action: Action) -> Result<A, ThriftError>
where
    T: IsThriftService<S>,
    Action: ThriftAction<S, A>,
{
    with_event_base_dataplane(|evb| ts.run_thrift(evb, action))
}

// ===========================================================================
// Coverage / markers / status
// ===========================================================================
//
// Coverage: the 57-line source is fully accounted for — options + Default
// (:26-33) -> the struct + impl; DbShard (:35-36) -> the alias; the class
// (:39-50) -> the trait with all five methods including the default body;
// the two exports (:52-57) -> `thrift_service` and
// `run_thrift_inefficiently`. Imported surfaces with no manifest unit:
// `Thrift.Api` -> the `glean_thrift` runtime sketch; `Util.EventBase` ->
// the kit handle sketch + this file's bracket image
// (`with_event_base_dataplane`, decision (e)) — neither is a re-export of
// generated code, both are located here per HANDOFF §4.2.
// Transcriptions: `Data.Text` -> String (RULE §1);
// `Data.Default` -> `Default`; `deriving Show` -> `Debug`
// (angle/types.rs:77-90). Nothing in the source is dropped, stubbed, or
// deferred beyond the one open leg flagged above.
//
// INVENTORY (RULE §5): no inventory.tsv row governs this file (a
// case-insensitive substring grep over every migration/inventory*.tsv for
// ThriftService returns zero governing rows; the only two hits —
// inventory.tsv:10 and inventory.codeflow.tsv:22 — are the dbShardWord
// row mentioning `thriftServiceWithDbShard` in its evidence text, and the
// port's `Option<DbShard>` transformer honors that row's routing use).
// inventory.tsv:13 (Haxl.hs EXC_TAXONOMY) and inventory.tsv:54 (glass
// classify) constrain the error union this file sketches first, but
// govern other files: row 13's "distinct ChannelError variant" requirement
// is satisfied by the sketch; row 54's Timeout mapping is satisfied by
// the `Application` leg.
//
// Marker breakdown (closed set — exactly one port todo, zero others):
// the port todo is the transport/IO error leg of the assumed union (its
// canonical flag line sits in the first assumed block above); no
// source-defect reproductions (the source has no partiality or defects
// to reproduce); no performance deferrals (the per-call bracket is the
// source's own semantics, not a shortcut); no inventory divergence (no
// row can diverge from a file no row governs).
//
// PORT STATUS: confidence=medium todos=1
