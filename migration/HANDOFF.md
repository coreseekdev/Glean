# Glean → Rust migration handoff

**Kit**: Claude Code Migration Kit (`~/.claude/skills/code-migration/`, six-step
process). Read DECISIONS.md first, then this file.

## STATUS (supersedes the stale gate line below — 2026-10-17)

**Step 3 (translation fan-out) IN FLIGHT: 33 units on disk + committed**
on branch `migration/rust-port`, HEAD `e879d8a` (26 dispatch-order units
+ manifest-line units 5, 15, 16, 17, 33, 34, 35). Steps 0b/1/2 are
COMPLETE and signed off (see DECISIONS.md's dated entries; RULEBOOK
v0.3 + the §4 assumed-imports paragraph, transcribed 2026-10-16).

**Numbering convention changed mid-batch**: commits since 1f9a2fa cite
the manifest FILE LINE (1-indexed including the `source	target` header)
as the unit number — "unit 33/386" = `sed -n '33p' migration/manifest.tsv`
= Typed/Predicate.hs. Older commits used dispatch order; both records
are in the git log.

Shipped (dispatch-order names, then manifest-line units):
1 hash.rs, 2 display.rs, 3 angle/types.rs, 4 schema/types.rs, 5 evolve.rs,
6 resolve.rs, 7 Schema/Resolve.hs, 8 Glean/FFI.hs, 9 RTS/Builder.hs,
10 RTS/Types.hs, 11 RTS/Term.hs, 12 Bytecode/Types.hs, 13 Bytecode/Supply.hs,
14 RTS/Bytecode/Code.hs, 15 RTS/Foreign/Bytecode.hs, 16 RTS.hs,
17 RTS/Set.hs, 18 RTS/Foreign/Lookup.hs (+ addendum in 3083c34),
21 RTS/Foreign/Inventory.hs, 22 RTS/Foreign/Subst.hs, 23 Util/Vector.hs,
28 RTS/Foreign/Define.hs, 29 RTS/Foreign/LookupCache.hs,
30 RTS/Foreign/Stacked.hs, 31 RTS/Foreign/Stats.hs,
32 RTS/Foreign/FactSet.rs; manifest-line units: 5 schema/util.rs (1f9a2fa),
15 typed/build.rs (e4626cc), 16 typed/id.rs (4382ebe), 17 typed/binary.rs
(ae1b1eb, completed from a dead-session draft), 33 typed/predicate.rs
(d54cd8a), 34 typed/build_fact.rs (367f952, NewFact/Facts/FactsM/
buildBatch; the insertWith NEW++OLD order was probe-fixed in review),
35 typed/fact.rs (e879d8a, decodeFact/decodeRef; decode_as_fact
override contract verified against Gen/Haskell.hs:327-328).

Blocked: manifest lines 23-24 (RTS/Traverse.hs, RTS/Typecheck.hs) — their
module `Glean.RTS.Bytecode.Gen.Issue` is missing from the Haskell tree;
semantics reconstructable from glean/rts/bytecode/subroutine.cpp if ever
needed.

Queue = manifest lines with no target on disk (file-line numbering):
next is line 36 (Typed/Prim.hs — deps all shipped), then 37 Typed.hs
(the glean-typed crate root — will wire `pub mod`s and needs once_cell
in Cargo.toml), 38-40 glean-util (Some/Service/ThriftService), 41+
backend-api. Check with:
`tail -n +2 migration/manifest.tsv | awk -F'\t' '{print NR+1"\t"$2}' |
while IFS=$'\t' read r t; do [ -f "$t" ] || echo "$r $t"; done`.

Pipeline per unit: implementer → 2 adversarial reviewers (semantics A /
obedience B, separate contexts, assume wrong, touch nothing) → fixer
(confirmed findings only; INLINE fixer is sanctioned when the round is
purely mechanical) → commit. RULEBOOK v0.3 is authoritative; §0 bans
cargo build/check in-loop (scratch rustc/GHC probes of ISOLATED snippets
are the sanctioned pinned-toolchain evidence pattern); markers
TODO(port)/BUG(port)/PERF(port)/SAFETY are load-bearing; the PORT STATUS
trailer is terminal and must match the actual marker count.

Operational notes from 2026-10-15/16 sessions:
- UNATTENDED SESSIONS: idle-time tasks execute units following
  migration/RUNBOOK.md; a scheduled SUPERVISOR automation replenishes
  the idle queue (spec: migration/SUPERVISOR.md — create it from a
  FRESH chat; sessions belonging to scheduled/idle tasks cannot create
  automations, and idle runs must never self-replicate). The old
  worker-mode cron (automation-c6386d8d) is PAUSED with runCount 0.
- QUOTA is the live constraint (5h rolling subagent cap; this host also
  runs idle-time sessions where background agents are unavailable —
  dispatch foreground, 2 reviewers per message). Reviewer A for unit 16
  could not dispatch twice (ECONNRESET, then rate limit) and was performed
  INLINE by the coordinator against pinned ground truth (DEV-001
  precedent); same for the units-28/30/31 review-round adjudication.
  Log such substitutions in the commit message.
- Never trust "verified-by-sibling" or "grep-verified zero/max" claims —
  this now binds REVIEWERS too: unit 33's reviewer-B proposed cite
  "corrections" (glean.thrift off-by-ones) that were themselves wrong;
  the fixer's grep-refutation stood after coordinator re-verification.
  Unit 35 recurred it: a reviewer's Binary.hs:235-236 "off-by-one" was
  refuted by the coordinator's sed — the cite was exact.
- Coordinator BRIEFS must not assert library semantics (unit 34,
  2026-10-17): the brief's parenthetical claimed Data.HashMap's
  insertWith combines as old++new; the truth is f NEW OLD (fresh value
  PREPENDED) — reviewer A's pinned-GHC probe caught it and the
  coordinator re-probed independently (unordered-containers-0.2.20 from
  the baseline cabal store) before applying. Briefs pin APIs and cites;
  a semantics claim needs a probe or a library cite. (Unit 35's brief
  also mis-stated thrift `Fact` field 2 as `Byte` — the implementer's
  correction to `Value` stood.)
- The supervisor automation edits migration/SUPERVISOR.md in-flight
  (its own queue bookkeeping; mtimes of other migration/ files can be
  touched with no content change). It is NOT a rival unit session:
  verify the edit's accuracy (CronList), commit it verbatim at records
  time, and expect the Edit tool's staleness check to fire on files it
  touched — re-read and continue.
- Assumed-import images must match the PINNED thrift compiler, never
  intuition: unit 17's `Byte(pub u8)` was wrong — hsthrift GenUtils.hs:231
  maps thrift byte → Int8 (set.rs:467-470 had already pinned it for the
  same call sites). §4's assumed-imports paragraph now requires this.
- EXPORT-LIST completeness cuts both ways (unit-18 lesson + unit 33):
  tick the source's export list against the target's pub items AND check
  ABSTRACTION — `Predicates` exported without `(..)` means the port's
  field must be private, constructor reachable only through the module's
  own functions.
- Generic callback traits that merge `invoke(...)?` MUST state
  `E: From<FfiError>` at the trait declaration (define.rs:459-481
  precedent for the merge-enum alternative).
- Mutable-handle Send/Sync (unit-32 review B-F1): audit every `&self`
  path (including trait impls) before adding Send/Sync.
- PERF(port) is mandated for slow-but-faithful copies.
- Subagent block-reports can be stale (unit-32): check the working tree
  before acting; unit 17's draft from a dead session was found complete-
  except-tuples on disk and finished by a completion round rather than
  restarted.
- DEV-006 (user-deferred): RULEBOOK §3's crate list (glean-storage/
  glean-core only) vs the shipped raw-pop-consumer contract that
  sanctions glean-typed's decoder units carrying unsafe (build.rs,
  binary.rs). §3 amendment queued for the human; see DECISIONS 2026-10-16.
- builder.rs's allocation-failure wording was aligned to the wrap.h
  story (20c59d4): allocation failures CROSS as returned messages; the
  abort is the Rust-side disposition per inventory row 61. QUEUED item
  closed.
- Marker-grep hygiene: breakdown bullets avoid the literal marker
  spellings inside the tally block.
- Handoff history: git log on branch migration/rust-port is the record of
  every unit's review findings + resolutions.

**Kit**: Claude Code Migration Kit (`~/.claude/skills/code-migration/`, six-step
process). **Gate status: Step 0b (judge) COMPLETE, awaiting human sign-off to
start Step 1.** Read DECISIONS.md first, then this file.

## Where everything lives

| Artifact | Path |
|---|---|
| Scope decisions + gate sign-offs | `migration/DECISIONS.md` |
| Feasibility report (Step 0) | delivered in session 2026-10-04; verdict "migrate now", cost band 1–3×10⁸ tokens, 4–7 weeks |
| Cost log | `migration/cost-log.tsv` |
| Judge harness | `migration/judge/{judge.py,README.md,impls.sh,mock-glean.py}` |
| Judge cases | `migration/judge/cases/` (6 cases: 1 upstream golden + 5 frozen) |
| Build tree (toolchain + built Haskell binary) | `~/glean-build/` (see below) |
| Build log with all stage timings | `~/glean-build/build-baseline.log` |

## Judge validation record (Step 0b evidence)

- **Old-code validation**: `validate-old` 6/6 — the real Haskell binary
  reproduces the upstream golden (`glean-write-1/main.out`) through the judge
  pipeline, and 5 frozen goldens were recorded from its output and eyeballed.
- **Mutation selftest**: 5/5 CAUGHT on raw CLI output, with the expected
  sensitivity boundaries (reverse-lines blind through gensort — covered by
  the unsorted-order case; phantom-field blind through extract — by design;
  zero-fact cases have nothing to mutate).
- **Determinism**: independent re-run (fresh DBs) reproduces all frozen
  goldens 6/6.
- **Reviews**: protocol-face adversarial review confirmed 10 findings
  (1 CRITICAL + 6 MAJOR + 3 MINOR) — all fixed in judge.py. Transforms face
  reviewed inline (normord sort-key bug + transform-arg typing fixed).
  Residual task: re-run transforms adversarial review via subagent when the
  5h quota resets (was 2026-10-05 02:49) — nice-to-have, not gate-blocking
  given mock + real-binary + mutation validation.

## Build baseline (measured, this box: 20 cores / 91 GB / Ubuntu 26.04)

Reproducible cold-path ≈ **62 min machine time**, excluding network
yak-shaving (this host blocks github.com/raw.githubusercontent/hackage
intermittently — every workaround below is recorded so the path is replayable):

| Stage | Time |
|---|---|
| GHC 9.4.7 bindist (fedora33, direct from downloads.haskell.org) | ~6 min |
| getdeps fmt/fast_float/folly (~/.hsthrift) | ~18 min |
| pcre 8.39 self-build (pcre1 removed from Ubuntu 26.04; source from archive.ubuntu.com pool) | ~3 min |
| thrift/bytecode/schema codegen (make thrift, gen-bytecode, gen-schema, thrift-schema-hs) | ~2 min |
| `cabal build all` (58 pkgs, 50 glean libs, C++ RTS, 17 exes, 65 test-suites; excludes glean-clang + thrift-tests per scope) | ~34 min |

Scripts: `~/glean-build/attempt6.sh` (idempotent, resumable). Old binary:
`~/glean-build/Glean/dist-newstyle/build/x86_64-linux/ghc-9.4.7/glean-0.2.0.1/x/glean/build/glean/glean`
(needs `LD_LIBRARY_PATH=$HOME/.hsthrift/lib` and `--schema
~/glean-build/Glean/glean/schema/source` — data-files are not installed in
build-tree form; the Rust CLI must do better, see judge/README.md).

Key build fixes (all in attempt6.sh / cabal.project of the build copy):
system Boost 1.90 lets folly skip its FetchContent boost download;
`make setup-folly-0` + `package folly-clib flags: -bundled-folly` so the
empty folly-clib still carries `pkgconfig-depends: libfolly`; remove system
libfmt-dev (double fmt major-version conflict); `--schema` flag at runtime.

## Next steps (in order)

1. **Human sign-off on the judge** (this gate) — read `migration/judge/README.md`,
   run the three commands in `migration/judge/impls.sh` if desired.
2. **Step 1** (`prompts/01` + `02`): depmap (write `depmap_haskell.py` — parse
   module headers, file + cargo-crate granularity), RULEBOOK.md draft,
   inventory.tsv, allium spec corpus (`migration/spec/*.allium`, 7 behavioral
   surfaces; `allium check` gates every edit; `allium plan` obligations bind
   to goldens or dual-impl parity tests).
3. Steps 2–6 per the kit. Step 4 dissolves into Step 3 (cargo check is a
   cheap referee) at the settings gate.
4. In Step 6 the judge's `parity` mode is the referee; `--impl-new` is the
   Rust CLI. The 1,945-pair upstream golden corpus is the full-corpus burndown
   target (the seed cases are its smoke subset).

## Non-obvious gotchas for any future session

- The surveyed repo `/home/nzinfo/src.parser/Glean` is PRISTINE (read-only
  surveys + migration/ artifacts only). The BUILD tree is the separate copy
  `~/glean-build/Glean` — never build in the surveyed repo.
- `.query` files are YAML: `query`, `recursive` (default true), `max_results`,
  `transform: [name, arg, ...]`. The CLI cannot express `recursive: false`,
  `derive`, or `no_base64_binary` — judge skips those; Rust CLI must add them
  (port requirements in judge.py docstring).
- `src.File`'s key is a bare Text: patterns are `src.File "foo"`, NOT
  `src.File { key = "foo" }` (that's a type error).
- Subagent dispatch hits a 5h rolling quota; stagger fan-outs (2 at a time).
