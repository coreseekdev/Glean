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

## 2. STATUS — 2026-10-18 (HEAD `63fd4fb`, after unit 43)

- Repo `Glean/`, branch `migration/rust-port`, HEAD `63fd4fb`, 67 commits,
  working tree clean.
- Remote: the fork `fork` (`git@github.com:coreseekdev/Glean.git`) is verified
  in sync at `9251205`; `origin` is `facebookincubator/Glean` (reference only).
  Push protocol: the user pushes the fork after batch commits on request —
  local has since advanced 11 commits to `63fd4fb` (HANDOFF rewrite `80ed78a`,
  unit 39 `db51448`, records refresh `f9491e2`, unit 40 `495ffac`, records
  refresh `df7255a`, hash.rs repair `9354cbb`, unit 41 `1aed2f6`, records
  refresh `00d279f`, unit 42 `3063cc5`, records refresh `1867a35`,
  retry-surface amendment `42117fc`, unit 43 `63fd4fb`; the next records
  refresh makes 12); sync the fork at the next batch gate.
- **Step 3 counters: 41 unit-commits / 40 distinct target files shipped;
  346 of 386 manifest units remain.** (`schema/util.rs` carries two commits:
  the early 352-line draft `551a3c8` + the completed 386-line rewrite
  `1f9a2fa` — the only double. Two non-unit fix commits: `9354cbb`
  (dead session's hash.rs derive-attachment repair) and `42117fc` (the
  unit-42 retry-surface amendment — `&RetryPolicy` + `Box<dyn Fn>`,
  coordinator ruling in DECISIONS.md 2026-10-18, forced by unit 43's
  21-site shared-policy consumer).)
- Steps 0/0b/1/2 records: see DECISIONS.md's dated entries and §7 below.
- RULEBOOK v0.3 + the §4 assumed-imports paragraph are transcribed and
  current (2026-10-16/17).
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
| glean-core | 17 | 12 | FFI/RTS/bytecode layer done; remaining rows 23/24 blocked, 51/57/94/101/109/113/114/139/278/279 |
| glean-bytecode | 1 | 5 | types.rs only; sys_calls/decode/generate/{instruction,cpp,haskell} remain |
| glean-typed | 8 | 1 | crate fully wired (lib.rs + Cargo.toml); remaining row 44 query/angle.rs |
| glean-util | 5 | 23 | service.rs + thrift_service.rs + some.rs + vector.rs + retry_channel_exception.rs + scaffolding |
| glean-db | 2 | 88 | rows 41+43 + scaffolding; crate wired per §4.2 (no [dependencies] yet) |
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

Next: **row 44** `glean/typed/Glean/Typed/Query/Angle.hs` →
`rust/glean-typed/src/query/angle.rs` (the LAST glean-typed row — will
need `pub mod query;`-level wiring in typed.rs/lib.rs per the unit-37
precedent if the manifest path demands the directory), then 45
`glean-db/src/query/thrift/internal.rs`. Brief fresh from the source +
inventory.
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

**glean-typed — 8/9, crate wired**
- `typed/build.rs` — `e4626cc` (line 15; reusable `Arc<dyn Fn>` decoder)
- `typed/id.rs` — `4382ebe` (line 16; PidOf/IdOf)
- `typed/binary.rs` — `ae1b1eb` (line 17; completed from a dead-session draft)
- `typed/predicate.rs` — `d54cd8a` (line 33)
- `typed/build_fact.rs` — `367f952` (line 34; insertWith NEW++OLD order probe-fixed)
- `typed/fact.rs` — `e879d8a` (line 35; decode_as_fact override verified vs Gen/Haskell.hs:327-328)
- `typed/prim.rs` — `ab55ceb` (line 36; fromNat/toNat Nat↔u64 two's-complement wraps, probe-verified)
- `typed.rs` crate-root facade + `lib.rs` + `Cargo.toml` — `82b4437` (line 37; NAMED re-exports, no globs)

**glean-util — 4/28, scaffolding shipped**
- `util/vector.rs` — `7f6c2ca` (dispatch 23)
- `util/some.rs` — `4bdf95a` (line 38; ZERO-ITEM decision record + `Cargo.toml`/`lib.rs`/`util.rs` scaffolding, retro-wiring vector.rs; review corrected 3 false record claims and reopened the consumer survey as a tracked TODO — Catalog.Store/Backup.Backend/Site x7/S3BucketApi)
- `util/service.rs` — `db51448` (line 39; the thrift `Service` union surface + `service_from_string` (stack-safe `reads @Int32` transcription) + the module-owned network-uri 2.6.4.2 `parseURI` stand-in (rows 40/166 consumers); 2 reviews + fix round; coordinator re-verified deep nesting vs the pinned GHC reference at N=2M)
- `util/thrift_service.rs` — `495ffac` (line 40; `ThriftServiceOptions` + std `Default` (`def` = all-`None`, GHC-probed), `DbShard`, `IsThriftService<S>: Clone` (5 methods; hand-written `Clone` impl so derived bounds cannot leak `S: Clone`; phantom tag — the all-`S` blanket-impl form is E0282-uninferable, per-tag impls work), `thrift_service` (`Nothing` arm = source's `mkThriftService _ _`), `run_thrift_inefficiently`; TWO first-in-tree ASSUMED-IMPORT sketches: the `glean_thrift` runtime (`ThriftAction` rank-2 `run<P: Protocol, C: ClientChannel<S>>` image of `forall p c`; `ThriftEnv`/`Counter` field-for-field per `Monad.hsc`; `ThriftError` union; CONTRACT: actions are SINGLE-USE) plus the `Util.EventBase` kit contract (opaque handle; infallible init; CONSUMING destroy; no kit `Drop`) with this file owning the `pub with_event_base_dataplane` bracket (parse_uri precedent; destroy-exactly-once incl. panic/nested); zero `unsafe` in-file, §3-letter kit situation flagged in-block (DEV-006 class); 2 reviews (A: deliverable compiled VERBATIM against an independent kit stub; B: PASS W/ MINOR) + fix round (evidence persisted under `/tmp/opencode/ts-probe/`). CARRY-FORWARD: units 166/167 add `+ Clone` on action reuse (E0382-reproduced, `Remote.withShard` fallback); the kit must pin stored-handle ownership before Remote/Glass/CLI/Sharding land — not `Clone`)

### 4.2 Immediate next-unit brief (row 41 — CONSUMED, shipped `1aed2f6`)

Row 41 shipped 2026-10-18 from a dead session's complete-but-unreviewed
draft (routed straight to review per RUNBOOK Step 0.2 — second clean
instance of the unit-17 pattern; the draft was intact, probes on disk,
brace-balanced). Review A PARTIAL→fixed (2 MAJOR: the GleanQuery dedupe
contract needed the result-aggregate `(r, Bool)` keying obligation —
QueryReq Types.hs:412-418, Haxl cache keys the full request type; and
Sum Int doc misattributed count — count consumes UniqueResults,
DataSource/Glean.hs:127-133), B PASS WITH MINOR; 15 coordinator-verified
fixes applied inline, all cite-hygiene or contract-prose (none changed a
dependency, an export, or a semantic outcome). Canonical row-9 flag
added for the batch-API reinterpretation (materialized across unit 41 +
row 146's flush). **Row 42** (`Util/RetryChannelException.hs`) is small;
brief it fresh from the source + inventory.

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
