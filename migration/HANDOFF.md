# Glean → Rust migration handoff

**Audience**: a successor model/session (or human) taking over this migration.
No conversation context is required — everything is on disk. This file is the
entry point.

**Read order**: `migration/DECISIONS.md` (scope, gate sign-offs, adjudications)
→ **this file** (status + how to work) → `migration/RULEBOOK.md` (whole; the
constitution) → `migration/RUNBOOK.md` (unattended/scheduled-session manual) →
`migration/SUPERVISOR.md` (idle-queue automation spec). The git log on branch
`migration/rust-port` is the per-unit record of review findings + fixes.

## 1. Mission

Migrate Glean's Haskell code to Rust **file by file**, following
`migration/manifest.tsv` (386 units, one source file → one target file per
RULE §4 naming), in manifest order, skipping blocked rows. All Haskell is in
scope (including the hie-indexer); **C/C++ is NOT translated** — it stays
behind the FFI boundary. No dynamic features. Step 3 ships file-level drafts;
crate compilation is Step 4; parity testing is Step 6 (judge).

Kit: Claude Code Migration Kit, six steps. **Steps 0 (feasibility), 0b
(judge), 1 (depmap + RULEBOOK + inventory + allium spec) and 2 (stress-test)
are COMPLETE and signed off** (DECISIONS.md). Step 3 (translation fan-out) is
in flight. Step 4's `cargo check` is deliberately NOT in the transcript loop
(RULEBOOK §0); it is the batch-boundary referee.

## 2. STATUS — 2026-10-19 (HEAD `21505c8`, after unit 67)

- Repo `Glean/`, branch `migration/rust-port`, HEAD `21505c8`, 100 commits,
  working tree clean.
- Remote: the fork `fork` (`git@github.com:coreseekdev/Glean.git`) is verified
  in sync at `9251205`; `origin` is `facebookincubator/Glean` (reference only).
  Push protocol: the user pushes the fork after batch commits on request —
  local has since advanced 46 commits to `21505c8` (`git rev-list --count
  9251205..HEAD`; the last five: unit 63 `dd316cf`, unit 64 `4cb683c`,
  unit 65 `0fa926e`, unit 66 `ca5a639`, unit 67 `21505c8`; this records
  refresh makes 47, and the HANDOFF successor-refresh commit that follows
  makes 48); sync the fork at the next batch gate.
- **Step 3 counters: 65 unit-commits / 64 distinct target files shipped;
  322 of 386 manifest units remain.** (`schema/util.rs` carries two commits:
  the early 352-line draft `551a3c8` + the completed 386-line rewrite
  `1f9a2fa` — the only double. Three non-unit fix commits: `9354cbb`
  (dead session's hash.rs derive-attachment repair), `42117fc` (the
  unit-42 retry-surface amendment — `&RetryPolicy` + `Box<dyn Fn>`,
  DECISIONS.md 2026-10-18), `41905c2` (the unit-3 AST's ArrayPrefix
  E0072 repair — `Box<NonEmpty<…>>`, discovered at unit 44's review).
- Units 51–67 (all committed; one records-refresh commit `e3c971c` at unit
  50): 51 repo/text.rs, 52 database/repo.rs, 53 database/exception.rs, 54
  database/catalog.rs, 55 acl_config.rs, 56 backup/backend.rs, 57
  rts/foreign/ownership.rs, 58 database/storage.rs, 59 acl_ownership.rs, 60
  data.rs, 61 backup/mock.rs, 62 batch_location.rs, 63 util/io.rs, 64
  catalog/local/files.rs, 65 catalog/local/memory.rs, 66
  schema/compute_ids.rs, 67 storage/db.rs. The Catalog/Local pair (64/65)
  closed the `CatalogStore` enum's last forward reference; `StoreError`
  gained the typed `Db(DbError)` arm at 64; 66 is the schema hasher (the
  schema-id linchpin, one TODO for old-binary parity); 67 is the RocksDB
  DatabaseOps layer (29 externs incl. the shared `database_free`).
- Steps 0/0b/1/2 records: see DECISIONS.md's dated entries and §7 below.
- RULEBOOK v0.3 + the §4 assumed-imports paragraph are transcribed and
  current (2026-10-16/17).
- Convention item settled 2026-10-18 (earlier refresh): the **Util.Log
  seam** — units 45/46 shipped the direct final-call-site form (`use
  glean_util::log::…` against the pinned lazy-thunk shape; no stand-in,
  no todo) as the convention going forward; unit 43's private stand-in +
  TODO in `retry_channel_exception.rs:311-324` remains that file's to
  delete when the log module lands. Recorded in DECISIONS.md 2026-10-18.
- Open doc-staleness item: `backend/types.rs:255-258`'s sibling-contract
  sentence "These paths do not exist in the tree yet" is stale for
  `crate::backend::retry` (shipped unit 43). types.rs is frozen for
  byte-identity; reword at its next touch (name it in that unit's brief).
  Same class, queued at unit 47's review: `write.rs:19-20`'s "whose parent
  src/query/thrift.rs waits for row 47" is stale (row 47 shipped); reword
  at write.rs's next touch.
- Open Step-4 architecture items (unit 48): the generated-module homes —
  `crate::internal::types` (DECISIONS 2026-10-05 pin; glean-schema's
  Gen/Main.hs is a cross-crate consumer, so the shared generated crate may
  win at Step 4 — the inventory's new internal.thrift row records both
  options) and `thrift::DBVersion` (the A10 one-crate assumption;
  server_config.thrift declares `namespace rust glean_server_config`), plus
  the first-sketched `glean_util::time_sec` seam (unit 48; home unpinned —
  the log-seam treatment). Every failure mode is a compile error, never
  silent.
- Open JSON-parity oracle (unit 48): the generated Meta's serde codec must
  reproduce hsthrift's bytes — oracle (5 sample Metas + the leniency table)
  under `/tmp/opencode/db48-probe/` (I/I2/J outputs) and
  `/tmp/opencode/db48-reviewA/`; measured divergences: U+0008/U+000C escape
  as `\u0008`/`\u000c` in aeson vs `\b`/`\f` in serde (round-trip safe
  both ways), and aeson is lenient top-level (type-skip + default-fill)
  where serde is strict. No inventory JSON_PARITY row exists yet; add one
  when the generated codec lands.
- Open unit-49 fidelity item: `database/catalog/filter.rs`'s `group_f`
  carries the one TODO for cross-group HashMap iteration order (the exact
  Data.Hashable/HAMT order is not reproduced); audit the order-sensitive
  consumers (Backup.hs:123/:132/:137 head selections; Catalog.hs:427-430
  listMostRecent) at their units / Step 6.
- Open unit-50 carry-forwards: `database/catalog/store.rs` records (a) the
  single closed-enum owner for `Some Catalog.Store` (row 54 adds it, row
  85 imports it), (b) row 53's DBError should extend `StoreError` with a
  typed arm, and (c) the `Files.get` double-join source defect
  (Files.hs:90/:94 reads `.../meta/meta`) is to be reproduced bug-for-bug
  at row 64.
- Open policy item: **DEV-006** (queued for the human) — RULEBOOK §3's
  crate-list letter allows `unsafe` only in glean-core; the shipped
  raw-pop-consumer contract (`rts.rs:123-138`) + build.rs/binary.rs RULE §3
  CRATE-SCOPE paragraphs sanction glean-typed's decoder units carrying FFI
  `unsafe` (18/18 SAFETY notes in binary.rs). Default ruling: keep the
  in-file documented exception; amend §3's list or pre-build safe pop
  wrappers in glean-core at Step 4.

### 2.1 Numbering convention (read this before trusting "unit N")

Commits since the 2026-10-16 batch cite the **manifest FILE LINE**
(1-indexed *including* the `source<TAB>target` header) as the unit number:
`unit 39/386` = `sed -n '39p' migration/manifest.tsv` = Service.hs. The
earliest batch (`bf7b43a`…`dfa9cd4`, messages "unit 1..32") used **dispatch
order**; that numbering coincides with manifest lines only through unit 11
and then diverges (e.g. dispatch-16 = `rts.rs` = manifest line 13; dispatch-12
= `bytecode/types.rs` = manifest line 20). Trust §4's file-centric inventory,
not old commit numbers. Going forward: manifest file lines only.

### 2.2 Crate dashboard (shipped / manifest rows)

| crate | shipped | remaining | notes |
|---|---|---|---|
| glean-angle | 7 | 0 | COMPLETE (hash, display, angle/types, schema/{util,types,evolve,resolve}) |
| glean-core | 19 | 10 | FFI/RTS/bytecode layer done + repo/text (51) + ownership (57); remaining rows 23/24 blocked, 94/101/109/113/114/139/278/279 |
| glean-bytecode | 1 | 5 | types.rs only; sys_calls/decode/generate/{instruction,cpp,haskell} remain |
| glean-typed | 9 | 0 | COMPLETE (lib.rs + Cargo.toml + query-module wiring at lib.rs per the source-shape rule; Angle DSL = unit 44) |
| glean-util | 6 | 22 | service.rs + thrift_service.rs + some.rs + vector.rs + retry_channel_exception.rs + util/io.rs (63) + scaffolding |
| glean-db | 22 | 68 | rows 41/43/45-50 + 52-56/58-62/64-67 (database area: exception/repo/catalog(+local files,memory)/acl_config/acl_ownership/storage(+db)/backup(+backend,mock)/data/batch_location/schema(+compute_ids)); crate wired per §4.2 (no [dependencies] yet) |
| glean-glass | 0 | 88 | not started |
| glean-client | 0 | 27 | not started |
| glean-cli | 0 | 26 | not started |
| glean-schema | 0 | 9 | not started |
| glean-server | 0 | 7 | not started |
| glean-fb303 | 0 | 5 | not started |
| indexers (clang 10, lsif 9, hack 7, haskell 4, angle 4, scip 2, api 2, + 11 singleton crates) | 0 | 49 | not started |
| glean-lsp / glean-storage / glean-registry | 0 | 3 / 2 / 1 | not started |

### 2.3 Blocked

Manifest rows 23–24 (`glean/rts/Glean/RTS/Traverse.hs`,
`RTS/Typecheck.hs`): their module `Glean.RTS.Bytecode.Gen.Issue` is missing
from the Haskell tree. Semantics are reconstructable from
`glean/rts/bytecode/subroutine.cpp` if ever needed. Skip these rows; do not
attempt them unless the Issue module appears.

### 2.4 Next units and the queue

Next: **row 68** `glean/util/Glean/Impl/MemoryReader.hs` (16 lines) →
`rust/glean-util/src/impl/memory_reader.rs`. Orientation only — brief it
fresh per §3.1:
- The source is the OPEN-SOURCE STUB: one export
  (`totalMemCapacityKB :: IO (Maybe Int)`, :11-16) whose body is
  `return Nothing` ("template for open source build / not implemented
  yet"). Port the stub, not a real /proc/meminfo reader; the unit is a
  fidelity trap in the opposite direction (do NOT "improve" it).
- **Wiring**: a NEW `Impl/` area. `impl` is a Rust keyword, so the
  crate-root line is `pub mod r#impl;` (the raw identifier's file is
  `src/impl.rs`); that file declares `pub mod memory_reader;`. Append
  both lines at this unit and record the raw-identifier choice.
- Then rows 69 (`Util/Disk.hs`: three `df --output=… -B1 '<path>'`
  shell-outs, `Int` → i64, IOException on failure), 70 (LMDB), 71
  (Memory), 72 (RocksDB — consumes `DB::from_raw`/`checkpoint` from
  unit 67 and must NOT attach a second finalizer).
Working style: 1–2 units per session, then a records-refresh commit.

Recompute the queue any time (manifest file-line numbers):

```sh
tail -n +2 migration/manifest.tsv | awk -F'\t' '{print NR+1"\t"$2}' |
while IFS=$'\t' read r t; do [ -f "$t" ] || echo "$r $t"; done
```

State check at session start: `git branch --show-current` (must be
`migration/rust-port`), `git status --short` (clean), `git log --oneline -5`.

## 3. How to work a unit

### 3.1 Pipeline (per unit)

1. **Implementer** — translate the one source file to the one target file.
   Header block: ported-from, manifest row, INVENTORY citation(s), dependency
   truth, assumed-imports (pinned to the real compiler/format), sibling-unit
   contracts (real APIs read, never re-declared). Terminal `PORT STATUS`
   trailer: `confidence=… todos=N`, matching actual markers.
2. **Two adversarial reviewers** (separate contexts, assume the port is
   wrong, touch nothing): **A = semantics** (against the pinned source +
   sibling APIs), **B = obedience** (RULEBOOK §2/§3/§4, markers, trailer,
   export lists, abstraction). Reviewers verify against pinned on-disk
   sources, never "verified-by-sibling".
3. **Fixer** — applies CONFIRMED findings only (re-verify each; reviewers can
   be wrong — see §5). Fully mechanical rounds may be fixed INLINE by the
   coordinator.
4. **Commit** — `migration: Step 3 unit N/386 — <target path> (+ reviews,
   inline fix round)`; the body summarizes findings/resolutions. Records
   refreshes are separate commits (`… records refresh — …`).

### 3.2 Dispatch and the fallback (DEV-001/002/003, execution windows)

- Dispatch implementer and reviewers as subagents, **foreground**, at most
  2 per message (5h rolling quota is the binding constraint; some hosts have
  no background agents). Prefer stopping early over rushing review.
- **DEV-001**: when a dispatch fails (quota, ECONNRESET, endpoint outage,
  execution-window expiry), the coordinator performs that role INLINE against
  pinned on-disk sources and logs the substitution in the commit message.
  Adjudication dispatches are covered too.
- **Window expiry**: an interrupted run resumes in a fresh window. Drafts
  persist on disk untouched; on resume re-verify git status/HEAD first;
  re-dispatch cancelled agents WHOLE (a cancelled agent produced NO report —
  never cite one). **Always check the working tree before trusting a failure
  report** — dead-session drafts have been found complete on disk twice
  (units 5, 17).
- **DEV-002**: skeptical re-review of the first batch's ships. **DEV-003**:
  depmap low-link fix (script history; no action pending).

### 3.3 RULEBOOK essentials that bite

- **§0**: never run `cargo build`/`cargo check` in the transcript loop.
  Scratch `rustc --edition 2021 --crate-type lib --emit=metadata <isolated>.rs`
  probes of isolated snippets and GHC probes ARE the sanctioned
  pinned-toolchain evidence pattern.
- Markers `TODO(port):` / `BUG(port):` / `PERF(port):` / `// SAFETY:` are
  load-bearing. Exactly ONE canonical flag line per flagged item; the tally
  block must match the actual count; tally bullets avoid the literal marker
  spellings. Inventory-row divergence gets an INVENTORY-BUG-CANDIDATE flag.
- **§4 export lists cut both ways**: tick the source's export list against
  the target's `pub` items AND check abstraction (a `Foo` exported without
  `(..)` means fields private, constructor reachable only via module fns).
- **§4 assumed imports must be pinned to the real compiler/toolchain**, never
  intuited (thrift `byte` → `Int8`, not `u8` — hsthrift GenUtils.hs:231).
- **§1 thrift row (A10)**: thrift types are generated by build.rs into the
  pinned `glean_thrift` crate; never hand-translate generated code. File-level
  drafts import it as an assumed import.
- Callback-bearing traits that merge `invoke(...)?` must state
  `E: From<FfiError>` at the TRAIT declaration (E0276/E0277 precedent).
  Boxed closure trait objects propagate `'static` (unit 15): use reusable
  `Arc<dyn for<'a> Fn(&mut DecoderEnv<'a>) -> Result<A, DecodeError>>` with
  `'static` bounds on every type param.
- Mutable-handle `Send`/`Sync`: audit every `&self` path (including trait
  impls) before adding. `PERF(port)` is mandated for slow-but-faithful
  copies. Release≡drop semantics must be documented for FFI handles.
- `glean-typed`/`glean-util` scaffolding exists; glean-angle/core/bytecode
  crate scaffolding (Cargo.toml/lib.rs) is owned by Step 4's workspace
  scaffold ("no crate in rust/ compiles yet"; third-party versions pinned at
  Step 4). New glean-util units append their own `pub mod` line to
  `rust/glean-util/src/util.rs`.
- **Direct-child wiring** (rows 46/48 precedent): when a unit creates the
  directory-module file for its area, it declares its OWN direct child
  `pub mod` line immediately (`write.rs` -> `pub mod send_batch;`,
  `database.rs` -> `pub mod meta;`; the glean-typed `query.rs:20`
  shape). Deferred prose contracts (`query.rs` row 47, `write.rs` row 115)
  are only for children whose FILES do not exist yet; when a promised line
  ships, rewrite its contract to the CONSUMED (past-tense) form.
- **Row-49 function-newtype images** (Catalog/Filter.hs): `Value a =
  Value (Item -> a)` -> a fn pointer with bound-free manual
  `Copy`/`Clone` (`Value<A>(fn(&Item) -> A)`), and `Filter a = Filter
  (S.State [Item] a)` -> `Arc<dyn Fn(Vec<Item>) -> (A, Vec<Item>)>` with
  inherent `map`/`and_then`/`pure`/`ap` (the Decoder precedent). `sortOn`
  computes keys once -> `sort_by_cached_key`; `Down` ->
  `std::cmp::Reverse`; `HashMap.fromListWith (++)` combines NEW ++ OLD
  (GHC-probed), so prepend+reverse cancels to input order. Cross-group
  HashMap order stays hash-dependent and gets the single TODO.
- **Log seam** (DECISIONS 2026-10-18; units 45/46/47/48): `glean_util::log`
  consumers write the FINAL call site at the pinned lazy-thunk shape
  (`vlog(level, || msg)`, `log_info(|| msg)`) with a direct import — no
  stand-ins, no port todo; unit 43's stand-in in
  `retry_channel_exception.rs` remains that file's to delete. `Util.Log.Text`
  is the Text-typed twin (Text -> String; laziness is vlog-only).
- **B2 `concurrently`** (first instance: row 47 `query/thrift.rs`): port
  `Async.concurrently` as real `std::thread::scope` pipelining, let-drain
  default, with the mandated `TODO(port): async-cancellation divergence`
  marker at the site (the marker text documents the error preference and
  the drain property); `B: Sync` bounds are the Rust-only price.
- **Generated modules beyond `glean_thrift`**: `Glean.Internal.Types`
  (gen-hs2 of `glean/if/internal.thrift`; NO manifest row) -> assumed
  `crate::internal::types` per DECISIONS 2026-10-05 + the inventory row;
  `Glean.ServerConfig.Types` -> assumed `thrift::DBVersion` (A10 default;
  home open at Step 4); `Util.TimeSec` -> first-sketched
  `glean_util::time_sec` (`Time` + `to_utc_time` consumed; wider surface
  owed). Never hand-write generated code; document assumed shapes in-file
  (RULE §4 B4) with the Step-4 compile-error fallback.
- **Class with IO methods + a shared existential** (row 50): a Haskell
  class whose methods are `a -> … IO` becomes a `&self` trait with
  `Result<_, <Area>Error>` (area-local enum, constructor-shaped
  `Display`/`Error` — the RunQueryError precedent; never rename another
  owner's error type). When one existential value flows across consumers
  (`Some Catalog.Store` -> `Catalog.open` -> `catLocal`), use ONE closed
  enum owned by the trait's module (the SomeBackend placement), added by
  the first consumer with forward references — never one enum per use
  site.

### 3.4 Records discipline

After each batch: a `records refresh` commit covering `migration/cost-log.tsv`
(new row: step, timestamp, wall clock, tokens, subagents, model),
this file's STATUS block, and any supervisor bookkeeping. The supervisor
automation edits `migration/SUPERVISOR.md` in-flight — verify the edit
(CronList), commit it verbatim at records time, and expect the Edit tool's
staleness check to fire on files it touched (re-read and continue).

## 4. Shipped inventory (authoritative — from the git log)

### 4.1 Shipped units

**glean-angle — 7/7 COMPLETE**
- `angle/hash.rs` — `bf7b43a` (dispatch 1; hash linchpin)
- `display.rs` — `5fdf8ec` (dispatch 2)
- `angle/types.rs` — `ac935a1` (dispatch 3; + cross-unit fix `7179c3e`)
- `schema/util.rs` — `551a3c8` draft + `1f9a2fa` completed (manifest line 5; 386 ln)
- `schema/types.rs` — `0ed4aa1` (dispatch 5)
- `schema/evolve.rs` — `669d6c5` (dispatch 6)
- `schema/resolve.rs` — `0159681` (dispatch 7; batch 1 complete)

**glean-core — 17/29**
- `ffi.rs` + `rts/builder.rs` — `fcbca85` (dispatch 8–9); builder allocation
  wording aligned `20c59d4`
- `rts/types.rs` — `6abba13` (dispatch 10)
- `rts/term.rs` — `7179c3e` (dispatch 11)
- `rts/bytecode/supply.rs` + `glean-bytecode/types.rs` — `d8d37d9` (dispatch 12–13)
- `rts/bytecode/code.rs` — `c42f29a` (dispatch 14)
- `rts/foreign/bytecode.rs` — `5784ca8` (dispatch 15)
- `rts.rs` — `4911a6e` (dispatch 16)
- `rts/set.rs` — `ace571d` (dispatch 17)
- `rts/foreign/lookup.rs` — `466f7be` (dispatch 18) + addendum `3083c34`
- `rts/foreign/inventory.rs` + `rts/foreign/subst.rs` — `7f6c2ca` (dispatch 21–22)
- `rts/foreign/define.rs`, `lookup_cache.rs`, `stacked.rs`, `stats.rs` — `3083c34` (dispatch 28–31)
- `rts/foreign/fact_set.rs` — `dfa9cd4` (dispatch 32; `Subst::as_ptr` seam)

**glean-bytecode — 1/6**: `bytecode/types.rs` (`d8d37d9`).

**glean-typed — 9/9 COMPLETE, crate wired**
- `typed/build.rs` — `e4626cc` (line 15; reusable `Arc<dyn Fn>` decoder)
- `typed/id.rs` — `4382ebe` (line 16; PidOf/IdOf)
- `typed/binary.rs` — `ae1b1eb` (line 17; completed from a dead-session draft)
- `typed/predicate.rs` — `d54cd8a` (line 33)
- `typed/build_fact.rs` — `367f952` (line 34; insertWith NEW++OLD order probe-fixed)
- `typed/fact.rs` — `e879d8a` (line 35; decode_as_fact override verified vs Gen/Haskell.hs:327-328)
- `typed/prim.rs` — `ab55ceb` (line 36; fromNat/toNat Nat↔u64 two's-complement wraps, probe-verified)
- `typed.rs` crate-root facade + `lib.rs` + `Cargo.toml` — `82b4437` (line 37; NAMED re-exports, no globs)
- `query/angle.rs` — `8314b35` (line 44; the typed Angle DSL — last glean-typed row; shipped with the source-shape query wiring at lib.rs/query.rs; 2 reviews + inline fix round incl. the ArrayPrefix E0072 repair `41905c2`)

**glean-util — 5/28, scaffolding shipped**
- `util/vector.rs` — `7f6c2ca` (dispatch 23)
- `util/some.rs` — `4bdf95a` (line 38; ZERO-ITEM decision record + `Cargo.toml`/`lib.rs`/`util.rs` scaffolding, retro-wiring vector.rs; review corrected 3 false record claims and reopened the consumer survey as a tracked TODO — Catalog.Store/Backup.Backend/Site x7/S3BucketApi)
- `util/service.rs` — `db51448` (line 39; the thrift `Service` union surface + `service_from_string` (stack-safe `reads @Int32` transcription) + the module-owned network-uri 2.6.4.2 `parseURI` stand-in (rows 40/166 consumers); 2 reviews + fix round; coordinator re-verified deep nesting vs the pinned GHC reference at N=2M)
- `util/thrift_service.rs` — `495ffac` (line 40; `ThriftServiceOptions` + std `Default` (`def` = all-`None`, GHC-probed), `DbShard`, `IsThriftService<S>: Clone` (5 methods; hand-written `Clone` impl so derived bounds cannot leak `S: Clone`; phantom tag — the all-`S` blanket-impl form is E0282-uninferable, per-tag impls work), `thrift_service` (`Nothing` arm = source's `mkThriftService _ _`), `run_thrift_inefficiently`; TWO first-in-tree ASSUMED-IMPORT sketches: the `glean_thrift` runtime (`ThriftAction` rank-2 `run<P: Protocol, C: ClientChannel<S>>` image of `forall p c`; `ThriftEnv`/`Counter` field-for-field per `Monad.hsc`; `ThriftError` union; CONTRACT: actions are SINGLE-USE) plus the `Util.EventBase` kit contract (opaque handle; infallible init; CONSUMING destroy; no kit `Drop`) with this file owning the `pub with_event_base_dataplane` bracket (parse_uri precedent; destroy-exactly-once incl. panic/nested); zero `unsafe` in-file, §3-letter kit situation flagged in-block (DEV-006 class); 2 reviews (A: deliverable compiled VERBATIM against an independent kit stub; B: PASS W/ MINOR) + fix round (evidence persisted under `/tmp/opencode/ts-probe/`). CARRY-FORWARD: units 166/167 add `+ Clone` on action reuse (E0382-reproduced, `Remote.withShard` fallback); the kit must pin stored-handle ownership before Remote/Glass/CLI/Sharding land — not `Clone`)
- `util/retry_channel_exception.rs` — `3063cc5` (line 42; `RetryPolicy` + `retry_channel_exceptions`; surface amended post-ship at `42117fc` (`&RetryPolicy` + `Box<dyn Fn>` — DECISIONS.md 2026-10-18); carries the private `log_warning` stand-in + TODO, which this refresh's log-seam ruling scopes to that file's own deletion — see §2)

**glean-db — 8/90, crate wired**
- `backend/types.rs` — `1aed2f6` (line 41; the `Backend` class + `BackendError` + `until_done`; shipped with lib.rs/backend.rs/Cargo.toml scaffolding + the first `glean_thrift` assumed-import sketch; 2 reviews + 15 inline cite/contract fixes)
- `backend/retry.rs` — `63fd4fb` (line 43; RetryWrites/RetryReads wrappers + the lossless `ThriftError::Service` round-trip bridge; reviews + inline fix round; supplies the carrier encoding unit 46's BatchAlreadyProcessed downcast consumes)
- `query/thrift/internal.rs` — `b0f22b9` (line 45; decode_results/display/combinators/stats helpers; shipped `pub mod query;` + src/query.rs recording row 47's contract; 2 reviews + inline doc fixes)
- `write/send_batch.rs` — `431dee2` (line 46; the synchronous write layer, 6 exports; shipped `pub mod write;` + src/write.rs, which declares `pub mod send_batch;` and records row 115's future `pub mod json;`; reviewer A caught the initially missing inner declaration; the NaN sleep docs were corrected to a no-divergence note against the runtime probe)
- `query/thrift.rs` — `3e2be5a` (line 47; the query facade (19/19 exports) + the area link — src/query.rs declares `pub mod thrift;`, the file declares `pub mod internal;`; the migration's FIRST `Async.concurrently` port: std::thread::scope pipelining + the B2 let-drain marker, both-fail preferring the processing error (site-documented); reviewer A compiled and ran the REAL facade against a mock backend; 2 reviews + fix round)
- `database/meta.rs` — `67cf160` (line 48; the DB metadata layer (21/21 exports) + the database-area wiring (`pub mod database;` + src/database.rs declaring `pub mod meta;`); the generated-module dispositions (crate::internal::types per DECISIONS vs the glean-schema cross-crate tension, thrift::DBVersion per A10) and the first `glean_util::time_sec` seam sketch; the Meta-JSON ground-truth oracle captured for the serde obligation; reviewer A reproduced the rounding/ACL/Database mappings byte-exactly against the real generated Haskell packages; 2 reviews + fix round incl. the Meta(..) re-export)
- `database/catalog/filter.rs` — `9874c48` (line 49; the catalog filter DSL (23/23 exports); `Value` -> a fn pointer with bound-free manual `Copy`/`Clone`, `Filter` -> `Arc<dyn Fn(Vec<Item>) -> (A, Vec<Item>)>` with `map`/`and_then`/`pure`/`ap`; `groupF` carries the one TODO for cross-group HashMap order (Backup.hs:123/:132/:137 and Catalog.hs:427-430 audit); `sortF` uses `sort_by_cached_key` + `std::cmp::Reverse`; wiring: `pub mod catalog;` in database.rs + the new catalog.rs declaring `pub mod filter;`; reviewer A/B dispatch hit 429 twice, both reviews performed INLINE per DEV-001; inline fix round)
- `database/catalog/store.rs` — `0461423` (line 50; the `Store` class (1/1 source export) -> `pub trait Store` with five `&self` methods; `IO` -> `Result<_, StoreError>` (`Io`/`Other`, constructor-shaped `Display` + `Error`, RunQueryError precedent); `Repo`/`Meta` by reference, `list` returns an owned HashMap (one PERF marker for the deep clone); ONE closed enum for the `Some Catalog.Store` / `forall local. Store local => Catalog` sites, owned by this module and added by row 54; wiring: `pub mod store;` appended in catalog.rs; 2 reviews (A: PARTIAL, B: PASS WITH MINOR) + fix round incl. the catchAll correction and the Files `get` double-join carry-forward)
- `repo/text.rs` — `6ce4e79` (line 51; the 8 pure Repo text helpers; NEW `repo/` area: src/repo.rs declares `pub mod text;`, crate-root `pub mod repo;` recorded as a Step-4 contract)
- `database/repo.rs` — `94787f9` (line 52; `databasePath`/`databaseSubdir`/`inRepo` + the `</>` image; row 51's trailer fix rode along)
- `database/exception.rs` — `84bbbba` (line 53; `DbError` Msg/Other, `dbException`/`dbError`, `DBAlreadyExists` From/TryFrom; the single error funnel)
- `database/catalog.rs` — `5789c9a` (line 54; the Catalog layer: closed `CatalogStore`/`Some Catalog.Store` enum, `Catalog` record, `startRestoring`/restore paths; wiring `pub mod catalog;`)
- `database/acl_config.rs` — `b127b29` (line 55; ACL config parse/merge; A/B reviews inline)
- `database/backup/backend.rs` — `3e37497` (line 56; `Data`/`RestoreSource`/`remove_path_forcibly`/`next_unique`/`RemoveOnDrop` seams; A/B reviews inline)
- `rts/foreign/ownership.rs` — `b19c72e` (line 57; Ownership/ComputedOwnership/DefineOwnership/Slice, UnitIterator, compute/computeDerivedOwnership, 48 unsafe constructs itemized; 2 reviews + fix round)
- `database/storage.rs` — `311e6af` (line 58; the `Storage`/`DatabaseOps` traits, Mode/CreateSchema/WriteLock, canOpenVersion/currentVersion, withMaterializedFile; the Some DatabaseOps forwarding impls deferred to row 104)
- `database/acl_ownership.rs` — `2d1dfd4` (line 59; augmentWithACL; `From<FfiError> for DbError` seam)
- `database/data.rs` — `8ec2141` (line 60; the Data record)
- `database/backup/mock.rs` — `c1b69d5` (line 61; the in-memory backup backend; Send+Sync carry-forward)
- `database/batch_location.rs` — `624a030` (line 62; BatchLocation parse/render)
- `util/io.rs` — `dd316cf` (line 63; read/write/withTempFile/backupOnError; the HandleIO classification row)
- `catalog/local/files.rs` — `4cb683c` (line 64; the Files backend; reproduces the `.../meta/meta` double-join with a source-defect marker; adds `StoreError::Db`)
- `catalog/local/memory.rs` — `0fa926e` (line 65; `Mutex<HashMap<Repo,Meta>>` per the STM_GROUP row; closes the CatalogStore enum)
- `schema/compute_ids.rs` — `ca5a639` (line 66; the schema hasher: Tarjan SCCs, Data.Binary encodings, Text-vs-[Char] cross-unit correction; 1 TODO for old-binary schema-id parity)
- `storage/db.rs` — `21505c8` (line 67; the RocksDB DatabaseOps layer: 29 externs incl. the shared `database_free`, `DB::from_raw` owns the finalizer, checkpoint bracketOnError + makeAbsolute normalise; 2 A/B reviews 429'd, inline review)

### 4.2 Immediate next-unit orientation (row 68 — NEXT UP)

**Row 68**: `glean/util/Glean/Impl/MemoryReader.hs` (16 lines) ->
`rust/glean-util/src/impl/memory_reader.rs`. The source is the
open-source STUB (`totalMemCapacityKB = return Nothing`); port the stub.
NEW `Impl/` area: the crate-root line is `pub mod r#impl;` (keyword) and
`src/impl.rs` declares `pub mod memory_reader;`. Then rows 69 (Disk.hs),
70 (LMDB), 71 (Memory), 72 (RocksDB). §2.4 has the same orientation; brief
every unit fresh from the source + inventory per §3.1.


## 5. Operational lessons (behavior-changing; carried forward)

1. **Never trust memory, briefs, or sibling claims over the pinned source.**
   Verify cites with `sed`/`grep` before repeating them. This binds REVIEWERS
   too: unit 33's reviewer proposed thrift "corrections" that were wrong; the
   fixer's refutation stood. A reviewer's own line-number claims get verified
   before application. Reviewer PROSE can also slip where their raw harness
   evidence is right (unit 39: a report line said `Tier` for an input its own
   harness logged as `Uri`) — re-derive decisive claims from the raw evidence.
2. **Briefs pin APIs and cites; a semantics claim needs a probe or a library
   cite.** (Unit 34: `insertWith` combines f NEW OLD — the brief's
   old++new was wrong; pinned-GHC probe caught it. Unit 35: `Fact` field 2 is
   `Value`, not `Byte`.)
3. **Assumed-import images must match the pinned compiler** (Byte→Int8 above).
4. **Export-list completeness cuts both ways** (unit 18 addendum: a missing
   `withCanLookup` restored at `3083c34`; abstraction checks per §3.3).
5. **Stale reports happen**: subagents have reported "blocked" with complete
   drafts on disk, and block-reports can predate the fixer. Check the tree;
   re-verify before restarting work.
6. **Mutable-handle Send/Sync audit** (unit 32: `unsafe impl Send/Sync` was
   correctly removed for FactSet — define_batch mutates through `&self`).
7. **PERF(port) for slow-but-faithful copies**; document release≡drop for
   handles; keep raw-pop unsafe behind documented contracts.
8. **Marker-grep hygiene**: one canonical flag line per item; tallies match
   reality; avoid literal marker spellings inside the tally block.
9. **Cost/quota**: prefer 1–2 well-reviewed units per session; dispatch
   foreground; stagger fan-outs (2 at a time); records-refresh commits keep
   the budget visible.
10. **Kit sketches' CONTRACT lines and probe artifacts are deliverables.**
    A first-consumer assumed-import sketch must spell its downstream
    obligations: unit 40's thrift actions are SINGLE-USE (`run(self)`) →
    units 166/167 add `+ Clone` at reuse sites (E0382-reproduced at
    `Remote.withShard`'s default fallback path), and the event-base kit must
    pin STORED-handle ownership before Remote/Glass/CLI/Sharding land (not
    `Clone` — that double-destroys). Carry both into §4.1 and the consumer's
    brief. And every probe a comment cites must exist on disk (review B-6
    killed a dangling "record in the port log" pointer): persist transcripts
    under `/tmp/opencode/ts-probe/` and cite the path.
11. **rustfmt hygiene on crate roots**: never bare-`rustfmt` a lib.rs or
    module-root draft — rustfmt traverses the module tree and rewrites
    sibling files (unit 46 hit this; the siblings had to be restored
    byte-exactly from HEAD). Use `rustfmt --edition 2021 --check --config
    skip_children=true` for crate roots, and after ANY rustfmt run verify
    the untouched siblings via `git status` + md5 against `git show
    HEAD:<path>`.

## 6. Infrastructure and environment

| Artifact | Path |
|---|---|
| Scope, gates, adjudications | `migration/DECISIONS.md` |
| Constitution | `migration/RULEBOOK.md` (v0.3) |
| Unattended-session manual | `migration/RUNBOOK.md` |
| Idle-queue scheduler spec | `migration/SUPERVISOR.md` |
| Cost log | `migration/cost-log.tsv` |
| Unit manifest (386) | `migration/manifest.tsv` |
| Inventories (main + ffi/protocol/sites/stm/codeflow) | `migration/inventory*.tsv` |
| Depmap (script + edges/crates/cycles/order) | `migration/depmap/`, `migration/scripts/` |
| Allium spec corpus (7 surfaces) | `migration/spec/*.allium` |
| Judge (Step 6 referee) | `migration/judge/{judge.py,README.md,impls.sh,mock-glean.py,cases/}` |
| Stress-test reports (Step 2) | `migration/stress-test/` |

- **Toolchains**: GHC 9.4.7 at `~/glean-build/ghc-9.4.7-install/bin/ghc`
  (semantics probes); rustc for scratch snippet probes (no cargo in-loop);
  `allium check` gates spec edits.
- **Build tree**: `~/glean-build/` (separate copy — NEVER build Haskell in
  the surveyed repo). Old binary:
  `~/glean-build/Glean/dist-newstyle/build/x86_64-linux/ghc-9.4.7/glean-0.2.0.1/x/glean/build/glean/glean`
  (needs `LD_LIBRARY_PATH=$HOME/.hsthrift/lib` and
  `--schema ~/glean-build/Glean/glean/schema/source`).
- **Rust tree**: `rust/{glean-angle,glean-bytecode,glean-core,glean-typed,glean-util}`;
  `Cargo.toml` exists for glean-typed + glean-util only (Step 4 owns the rest);
  `rust/glean-angle/tests/hash.rs` is the first re-expressed test (tests are
  not translated 1:1; RULE §4).
- **Automation**: idle-queue supervisor automation
  `automation-a60b5841-7b60-43c2-9cc6-47a58c624c40` (spec: SUPERVISOR.md);
  old worker automation deleted. Unattended runs follow RUNBOOK.md.

## 7. History: gates, judge, build baseline

- **Step 0b judge validation**: old-code `validate-old` 6/6 (real binary
  reproduces the upstream golden); mutation selftest 5/5 CAUGHT with expected
  sensitivity boundaries; determinism 6/6; adversarial review fixed 10
  findings. (Residual nice-to-have: transforms re-review — not gate-blocking.)
- **Step 3 fan-out** authorized 2026-10-07 ("continue 1 2 3"): serial
  background dispatch cargo-culted then amended for quota (foreground, 2 at a
  time); DEV-002 skeptic re-review in batch 1.
- **Build baseline** (measured, 20 cores / 91 GB / Ubuntu 26.04): cold path
  ≈ 62 min machine time — GHC bindist ~6, getdeps fmt/fast_float/folly ~18,
  pcre 8.39 self-build ~3, codegen ~2, `cabal build all` ~34 (58 pkgs; 65
  test-suites; excludes glean-clang + thrift-tests). Script
  `~/glean-build/attempt6.sh` (idempotent, resumable); fixes (Boost 1.90 /
  folly-clib flags / remove system libfmt-dev / `--schema` at runtime) are
  recorded there and in the build log `~/glean-build/build-baseline.log`.

## 8. Gotchas

- The surveyed repo `Glean/` is where migration artifacts + `rust/` drafts
  live (branch `migration/rust-port`); the Haskell BUILD copy is
  `~/glean-build/Glean` — never build Haskell in the surveyed repo.
- `.query` files are YAML: `query`, `recursive` (default true),
  `max_results`, `transform: [name, arg, ...]`. The CLI cannot express
  `recursive: false`, `derive`, or `no_base64_binary`; the Rust CLI must add
  them (requirements in `migration/judge/judge.py` docstring).
- `src.File`'s key is a bare Text: patterns are `src.File "foo"`, NOT
  `src.File { key = "foo" }` (type error).
- Reviews run against PINNED on-disk sources; absolute paths under
  `/home/nzinfo/src.parser` are blocked for tooling — use repo-relative paths.
- Subagent dispatch hits a 5h rolling quota; on endpoint outage/rate limits,
  DEV-001 inline execution applies (log it in the commit message).
