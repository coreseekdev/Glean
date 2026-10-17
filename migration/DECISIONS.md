# Migration decisions (gate sign-offs, newest last)

## 2026-10-04 — Step 0 feasibility verdict signed off with amendments (human)

1. **C/C++ components are KEPT AS-IS** (no rewrite): glean/rts (fact store + bytecode
   evaluator, bound via existing extern "C" ABI), glean/lang/clang (separate binary),
   lmdb-clib (vendored), storage wrappers, glean/cpp, client/swift, interprocess/cpp.
2. **ALL Haskell is translated to Rust** — no Haskell remains in the product, including
   hie-indexer (previously proposed as a kept-in-Haskell external binary).
   - hie-indexer approach: Rust parser for the GHC `.hie` binary file format
     (versioned header; produced by any GHC invocation) instead of GHC-the-library.
     Flagged highest-risk unit; if the spike shows the format is not tractable,
     ESCALATE TO HUMAN for the dynamic-feature discussion (per their instruction).
3. **Dynamic features**: none required so far (haxl → explicit batching layer; TH →
   proc-macro or plain functions; unsafePerformIO → explicit safe wrappers).
   If one becomes necessary mid-migration, stop and discuss per human instruction.
4. Test suites are NOT translated (they die with the language): the 1,945-pair
   `.query`/`.out` golden corpus + allium obligation tests are the judge
   (validated per prompts/00b before Step 1).
5. Order: prompts/00b (judge) → Step 1 (map+rules+inventory+allium spec corpus) → 02..06.

## Step 0 amendments carried into Step 1

- allium spec corpus (`migration/spec/*.allium`, 7 behavioral surfaces) is built in
  Step 1 and gates every spec edit via `allium check`; `allium plan` obligations bind
  to golden pairs or dual-implementation parity tests.
- Step 4 dissolves into Step 3 at the settings gate (cargo check is a cheap referee).
- v1 binaries: glean CLI, glean-server, glass-server, glean-lsp, gen-schema,
  gen-bytecode-{cpp,hs}, disassemble (all Rust; gen tools reuse
  schema/gen/Glean/Schema/Gen/Rust.hs as seed). Excluded: hack-derive, benches, demo.

## 2026-10-05 — Step 2 round-1 amendment batch approved (user, via "continue")

- Amendment queue A1-A10 (stress-test/report.md) APPROVED, applied to
  RULEBOOK.md as v0.2 by the orchestrator as the user's delegate.
- A10 rulings (recommended dispositions, unopposed): time = std
  SystemTime/Duration only (no time crate); generated thrift crate pinned as
  `glean_thrift`; Util.Log/glog bridge owned by a `glean_util::log` module
  with lazy message thunks; S3 backup feature-gated OUT of v1 (queued item 6).
- INVENTORY-BUG-CANDIDATE rows 26 and 136 fixed inventory-side (row 26 types
  corrected to source truth; row 136 logger dyn granted the genuinely-open
  carve-out).
- `Glean.Internal.Types` (StorageName): owns its own module in glean-db
  (generated-thrift adjacent), pinned in inventory.
- §7 queued items 1-3 ratified as written (inventory delegation rule,
  DEV-001/002 + re-review commitment, crate partition + naming).

## 2026-10-07 — Step 2 round-2 amendment batch approved; Step 3 fan-out signed off (user, via "continue 1 2 3")

- Amendment queue B1-B6 (stress-test/report-r2.md) APPROVED -> RULEBOOK v0.3.
- Inventory rows 120/121 fixed per the confirmed INVENTORY-BUG-CANDIDATEs.
- Step 3 (translation fan-out, prompt 04) authorized to start: serial
  background dispatch (quota lesson), DEV-002 skeptic re-review in the
  first batch.

## 2026-10-16 — Step 3 fix-round adjudications (units 28-31 + unit-18 addendum)

- H1 (rustc-repro'd by the unit-30 review): callback-bearing traits that
  merge `invoke(...)?` must state `E: From<FfiError>` at the TRAIT
  declaration (impls cannot add it — E0276; omission is E0277). Bound
  applied to CanLookup/CanDefine + the scoped helpers; recorded as an
  operational note in HANDOFF.
- Bound-vs-abort fork adjudicated INLINE (adjudication re-dispatch failed
  ECONNRESET — DEV-001 now covers adjudication dispatches): bounds kept
  because the withSnapshot/stacked paths sit under catch sites
  (Batch.hs `logExceptions`, SendAndRebaseQueue `rebase`) where a wholesale
  abort disposition would diverge from the source; `define_fact` keeps its
  row-pinned `Result<Fid, InvalidRedefinition>` via a private merge enum
  (DefineFactFailure: From<FfiError>), aborting the FFI arm just outside
  the bracket.
- Unit-18 addendum (found by a coordinator completeness survey, missed by
  both unit-18 reviewers): the `withCanLookup` export was absent from the
  shipped lookup.rs; restored in 3083c34. New review checklist item: tick
  the source's export list against the target's `pub` items.
- Marker-grep hygiene formalized: tally bullets avoid literal marker
  spellings; exactly one canonical flag line per flagged item.

## 2026-10-07 — Queued amendments: coordinator-default rulings (user deferred decision — "context incomplete, continue"; all reviewable/reversible at any gate)

Applied by the orchestrator under RULEBOOK v0.3's existing rules, no new policy invented:
1. prettyprinter Doc stand-in ownership -> RULEBOOK §1 gains a row: the
   stand-in lives in glean-angle/src/display.rs; inventory gains a row
   pinning it (like the Fingerprint/Binary precedents: byte-parity
   obligations via the judge's golden rendering at Step 6).
2. §4 assumed-imports body text -> the v0.3 changelog line gets its
   intended §4 paragraph (was announced, never written — three units cite
   it; this is transcription of the already-approved B4, not new policy).
3. Fingerprint std Hash impl -> owned by hash.rs behind the existing
   hashable-chain TODO(port); impl delegates to fingerprint1/fingerprint2
   (stable, in-memory only — no persisted surface).
4. cxx family (inventory rows 38/60/79/61 "cxx" wording) -> rows are
   superseded by RULEBOOK §1 FFI row (A2: raw extern "C" for flat ABI);
   rows annotated in-place, no code changes required (all four units
   already shipped extern-C).

## 2026-10-16 — Unit 17 (typed/binary.rs) adjudications

- Byte image ruling (reviewer A MAJOR, CONFIRMED): the thrift `byte` →
  `Int8` image is pinned by the hsthrift compiler itself
  (hsthrift/compiler/Thrift/Compiler/GenUtils.hs:231 `genType I8 =
  qualType "Int" "Int8"`) and by shipped set.rs:467-470/:576-578 for
  exactly the Typed/Binary.hs:198 sites. binary.rs's assumed `Byte(pub
  u8)` corrected to `pub i8` (decode `n as i8`, build `self.0 as u8`,
  set-insert sign-extends per the set.rs contract). Assumed-import images
  must be pinned to the compiler's mapping, not intuited — folded into
  RULEBOOK §4's assumed-imports paragraph.
- Reviewer false-finding refuted by the fixer (B-2, glean.thrift
  off-by-ones): the ORIGINAL cites were exact (`typedef binary Value` at
  :144, `struct Fact {` at :147); the reviewer's proposed numbers were
  the wrong ones. Lesson: the "never trust grep-verified claims" rule
  binds reviewers too — a reviewer's own line-number claims get verified
  before they're applied.
- RULEBOOK §4 assumed-imports paragraph finally transcribed (the
  2026-10-07 coordinator-default ruling item 2 recorded it as applied but
  the §4 body never received the text; executed now at the batch
  boundary — transcription of approved B4, no new policy).
- DEV-006 (user-deferred, reviewable at any gate): RULEBOOK §3's letter
  allows unsafe only in glean-storage/glean-core, but the shipped
  raw-pop-consumer contract (rts.rs:123-138) + build.rs's RULE §3
  CRATE-SCOPE paragraph sanction glean-typed's Decoder-consuming units to
  carry unsafe FFI wrappers (the Haskell there was already FFI — §3's
  intent clause). binary.rs follows the build.rs precedent with 18/18
  SAFETY notes. Default ruling: keep the in-file documented exception;
  §3's crate-list amendment stays QUEUED for the human (extend the list
  to name glean-typed's decoder units, or pre-build safe pop wrappers in
  glean-core at Step-4 consolidation).

## 2026-10-18 — Unit-42 surface amendment (coordinator ruling at unit 43)

- **retry_channel_exceptions signature amended post-ship (42117fc)**:
  `r: RetryPolicy` (by value) + `on_error: Box<dyn FnMut(...)>` ->
  `r: &RetryPolicy` + `Box<dyn Fn(...)>. The first draft's pairing was
  unusable by the function's own named heaviest consumer — row 43's
  Backend/Retry.hs reuses ONE policy across 21 `&self` call sites,
  inexpressible with a consuming signature and FnMut-through-&.
  Behavior unchanged (same loop/orderings/texts; probe-verified).
  Sanctioned under the unit-18-addendum precedent (post-ship surface
  repair at the consumer's discovery point). LESSON: when a unit ships
  a function whose contract cites future consumers, the reviewer should
  simulate the pinned consumer's call shape — unit 42's reviews never
  tried a second call with the same policy.
- **FnMut -> Fn for the on_error field**: the original survey (sole
  onError value = capture-free logRetry) still holds; Fn is the
  faithful image of the source's shared immutable callback. If a future
  unit introduces a stateful onError, that is a new survey fact and
  reopens the choice — flag, do not silently Box<FnMut> again.
- **SomeBackend<R> prose correction (types.rs:760-763)**: the Box note's
  "wrappers hold SomeBackend<R> BY VALUE" is stale prose superseded by
  the code on both sides (wrapper holds plain R; enum boxes the
  wrapper). retry.rs's header carries the SUPERSEDES note; reword the
  prose at the next touch of that comment.
