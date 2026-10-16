# Glean → Rust migration handoff

**Kit**: Claude Code Migration Kit (`~/.claude/skills/code-migration/`, six-step
process). Read DECISIONS.md first, then this file.

## STATUS (supersedes the stale gate line below — 2026-10-16)

**Step 3 (translation fan-out) IN FLIGHT: units 1-18, 21-23 and 28-32 on
disk + committed (26 units)** on branch `migration/rust-port`, HEAD `dfa9cd4`.
Steps 0b/1/2 are COMPLETE and signed off (see DECISIONS.md's dated entries;
RULEBOOK v0.3).

Unit numbering follows DISPATCH order (not manifest lines). Shipped:
1 hash.rs, 2 display.rs, 3 angle/types.rs, 4 schema/types.rs, 5 evolve.rs,
6 resolve.rs, 7 Schema/Resolve.hs, 8 Glean/FFI.hs, 9 RTS/Builder.hs,
10 RTS/Types.hs, 11 RTS/Term.hs, 12 Bytecode/Types.hs, 13 Bytecode/Supply.hs,
14 RTS/Bytecode/Code.hs, 15 RTS/Foreign/Bytecode.hs, 16 RTS.hs,
17 RTS/Set.hs, 18 RTS/Foreign/Lookup.hs (+ fix-round addendum in 3083c34),
21 RTS/Foreign/Inventory.hs, 22 RTS/Foreign/Subst.hs (+ as_ptr seam),
23 Util/Vector.hs, 28 RTS/Foreign/Define.hs, 29 RTS/Foreign/LookupCache.hs,
30 RTS/Foreign/Stacked.hs, 31 RTS/Foreign/Stats.hs,
32 RTS/Foreign/FactSet.hs.

Blocked: 19 (RTS/Traverse.hs) + 20 (RTS/Typecheck.hs) — their module
`Glean.RTS.Bytecode.Gen.Issue` is missing from the Haskell tree; semantics
reconstructable from glean/rts/bytecode/subroutine.cpp if ever needed.

Batch 2 (glean-core) remaining in manifest order: the glean-typed crate
starts next (typed/Build.hs, Id.hs, Binary.hs — Build.hs's deps FactSet/
Define are shipped). Queue = manifest rows with no target on disk; check
`grep -v` of shipped paths against manifest.tsv.

Pipeline per unit: implementer → 2 adversarial reviewers (semantics A /
obedience B, separate contexts, assume wrong, touch nothing) → fixer
(confirmed findings only) → commit. RULEBOOK v0.3 is authoritative; §0 bans
cargo build/check in-loop; markers TODO(port)/BUG(port)/PERF(port)/SAFETY are
load-bearing; the PORT STATUS trailer is terminal and must match the actual
marker count.

Operational notes from 2026-10-15/16 sessions:
- QUOTA is the live constraint (5h rolling subagent cap). Reviewer A for unit
  16 could not dispatch twice (ECONNRESET, then rate limit) and was performed
  INLINE by the coordinator against pinned ground truth (DEV-001 precedent);
  same for the units-28/30/31 review-round adjudication (ECONNRESET; bounds
  kept + merge enum chosen, rationale logged in 3083c34). Log such
  substitutions in the commit message. A combined fixer (or implementer) for
  a unit PAIR is acceptable (units 12+13 precedent; 30+31 batch).
- Never trust "verified-by-sibling" or "grep-verified zero/max" claims;
  verify against pinned on-disk sources (GHC 9.4.7 at
  ~/glean-build/ghc-9.4.7-install/bin/ghc — used for `!!` panic-message and
  `deriving newtype Show` probes this batch).
- EXPORT-LIST completeness: reviewers of unit 18 missed that `withCanLookup`
  (a source export, 10+ call sites) was absent from the shipped lookup.rs;
  the units-28-31 fix round caught it. When reviewing a leaf unit, tick off
  the source's export list against the target's `pub` items, not just the
  inventory row's prose.
- Generic callback traits that merge `invoke(...)?` MUST state
  `E: From<FfiError>` at the trait declaration — an impl cannot add it
  (E0276) and omitting it is E0277 (rustc-repro'd, unit-30 review H1). When
  a call site must keep a row-pinned concrete error type that has no such
  impl, use a private merge enum + `From<FfiError>` and dispose of the FFI
  arm just outside the bracket (define_fact / DefineFactFailure precedent,
  define.rs:459-481).
- Mutable-handle Send/Sync (unit-32 review B-F1): a handle is only `Sync` if
  `&self` cannot reach a mutation — trait impls count (FactSet's
  `CanDefine::with_define` hands out the mutable define surface, so
  `define_batch(&*facts, ...)` mutates through `&self`). The sibling
  `Define` is !Send/!Sync for the same reason (define.rs:310-313); FactSet
  now matches. When porting a mutable handle, audit every `&self` path
  (including trait impls) before adding Send/Sync.
- PERF(port) is mandated for slow-but-faithful copies (unit-32 review A-F3):
  the crate's `unsafe_malloced_byte_string` copies where the source's
  `unsafeMallocedByteString` was zero-copy — flagged at the use site.
- Subagent block-reports can be stale: the unit-32 implementer reported
  "blocked, no file written" yet the complete draft was on disk (found via
  `git status`). Always check the working-tree state before acting on a
  block report; land the missing seam inline (the unit-28 seam precedent)
  and re-dispatch.
- QUEUED (needs its own review pass, shipped-file doc alignment):
  rts/builder.rs claims "allocation failures abort inside C++ and never cross
  the boundary" while crate::ffi + rts.rs + rts/set.rs correctly record
  wrap.h's real behavior (every std::exception, bad_alloc included, crosses as
  a returned message — wrap.h:21-32). Disposition (abort) is the same; the
  builder.rs wording should be aligned.
- Marker-grep hygiene: breakdown bullets that spell the marker names
  (`TODO(port):` etc.) over-count on raw grep. Prefer prose that avoids the
  literal spellings inside the tally block.
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
