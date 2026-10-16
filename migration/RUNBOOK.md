# Scheduled-session runbook — Glean → Rust migration

Intended executor: an UNATTENDED scheduled session (or any fresh session)
acting as migration coordinator. Everything needed is on disk; no
conversation context is required. This file is operational only — all
translation policy lives in RULEBOOK.md; all scope/gate history lives in
DECISIONS.md; current status lives in HANDOFF.md.

Read order at session start: `migration/DECISIONS.md` → `migration/HANDOFF.md`
(STATUS block) → this file → `migration/RULEBOOK.md` (whole).

## Mission per session

Ship **1–2 manifest units** through the full pipeline, then refresh the
records and stop cleanly. Prefer stopping early over rushing review.
Quota (5h rolling subagent cap) is the binding constraint; a session
that ships 1 well-reviewed unit is a success.

## Step 0 — orient

1. Repo: `Glean/` under the session working directory; branch must be
   `migration/rust-port` (`git branch --show-current`). If not, STOP and
   log (see journal below) — never switch branches.
2. `git status --short`:
   - clean → proceed;
   - untracked/modified `rust/**` left by a DEAD session → per the
     unit-17/32 lessons, inspect before acting: check the tail for a
     terminal `// PORT STATUS:` trailer and grep for placeholder
     comments ("follow the same shape" etc.). Complete-but-unreviewed →
     route straight into Step 2 (review). Partial → dispatch a
     completion round (Step 1 prompt, listing the pinned defects).
     If mtimes are seconds-fresh (another LIVE session in flight), exit
     without touching anything and log.
3. Compute the queue (lowest manifest line first = dependency order):

   ```sh
   tail -n +2 migration/manifest.tsv | awk -F'\t' '{print NR+1"\t"$2}' \
     | while IFS=$'\t' read -r row tgt; do [ -f "$tgt" ] || echo "$row -> $tgt"; done
   ```

   Unit number = manifest FILE line (1-indexed including the header row).
   Lines 23/24 are BLOCKED (upstream `Glean.RTS.Bytecode.Gen.Issue`
   missing) — skip them. If the next unit's dependencies are somehow not
   all shipped (should not happen in manifest order), skip to the next
   unit whose deps are.

## Step 1 — implementer (ONE unit, one agent)

Dispatch ONE `general-purpose` agent, **foreground** (idle-time sessions
have no background agents; foreground-parallel only at Step 2). Prompt
skeleton (fill in per unit):

- Read RULEBOOK.md whole; the SOURCE file (whole); the shipped sibling
  files whose REAL APIs the unit consumes — with the standing facts:
  cross-crate imports are `use glean_<crate>::…`; glean-core's `rts`
  submodules (set/builder/types/…) are NOT re-exported at the rts root;
  `Int = i64` (LP64); thrift assumed-images follow the PINNED compiler
  (byte → Int8, hsthrift GenUtils.hs:231 — never intuition);
  `Data.Set` → `BTreeSet` (exactly `Ord`); `Data.Map` → `BTreeMap`.
- Obligations: one file only, at the manifest path; header blocks
  (INVENTORY with grep of migration/inventory*.tsv for the source name,
  DEPENDENCY TRUTH, ASSUMED-IMPORT with thrift/compiler cites,
  SIBLING-UNIT CONTRACTS); export-list parity INCLUDING abstraction
  (an export without `(..)` means the port's field stays private);
  terminal `// PORT STATUS:` trailer with honest marker counts (tally
  bullets must not spell the literal marker strings); every file:line
  cite grep-verified — NEVER fabricate a quote; no cargo build/check/
  test/run; no git mutations; drafts need not compile but must be
  type-coherent against the real sibling signatures. Scratch rustc/GHC
  probes of ISOLATED snippets are sanctioned evidence (pinned GHC 9.4.7
  at ~/glean-build/ghc-9.4.7-install/bin/ghc).
- Require a report: changes with cites; disagreements with the brief
  (with evidence); open questions for reviewers; final marker count.

## Step 2 — reviewers (two agents, one message, foreground-parallel)

- **A (semantics)**: instance-by-instance behavioral fidelity vs the
  Haskell; FFI marshalling widths/directions vs the real extern decls;
  ordering/last-wins/laziness; panic-message parity (probe with the
  pinned GHC where plausible); every SAFETY claim's contract.
- **B (obedience)**: rulebook §-by-§ (manifest path, import form,
  export parity incl. abstraction, unsafe+SAFETY 1:1, marker counts vs
  trailer, inventory rows followed-or-flagged, assumed-import policy,
  whitelist, repo-pristine `git status`).
- Both: read-only, assume wrong, CONFIRMED vs SUSPECTED, severity
  CRITICAL/MAJOR/MINOR/NIT, verdict last line
  `A: PASS|PASS WITH MINOR|PARTIAL|FAIL` (same for B).
- If a reviewer cannot dispatch (ECONNRESET/rate limit): retry once,
  then perform INLINE against pinned sources per the DEV-001 precedent
  and LOG the substitution in the commit message.

## Step 3 — adjudicate + fix

- Verify EACH finding's evidence yourself before applying — reviewers'
  own cites have been wrong (unit 17: reviewer-B's off-by-one
  "corrections" were refuted by the fixer's grep). Refuted findings get
  logged, not applied.
- Purely mechanical finding sets → apply INLINE (sanctioned fixer-inline
  path). Anything judgment-shaped → dispatch a fixer agent with the
  confirmed findings only.
- Substantive conflicts → coordinator ruling recorded in DECISIONS.md
  (user-deferred/coordinator-default pattern); never edit RULEBOOK
  semantics mid-batch (transcribing already-approved rulings at batch
  boundaries is the exception).

## Step 4 — commit + records

1. Commit the unit: `migration: Step 3 unit <line>/386 — <crate> <path>
   (+ reviews, fix round)` with the established body — port summary,
   review verdicts + counts, fixes applied AND refuted, marker counts.
2. Refresh HANDOFF.md STATUS block (unit count, HEAD, shipped list,
   queue head) and commit (may fold into the unit commit's follow-up).
3. Update persistent memory: `glean-rust-migration-status.md` (HEAD,
   unit count, next unit) + the MEMORY.md index line, under the
   auto-memory directory shown in the session's memory instructions.
4. Journal: append one row to `migration/cost-log.tsv`
   (`step=<date>-auto[,unitN…]`, timestamp, wall minutes, tokens,
   subagents, model) and include it in the HANDOFF commit.

## Rails (hard)

- NEVER: cargo build/check/test/run on the project; git push; edits
  outside `rust/**` and `migration/**`; touching C/C++ sources; asking
  the user questions (defer decisions per Step 3).
- The surveyed repo stays pristine apart from `rust/` + `migration/`.
- If the Agent tool cannot dispatch AT ALL in this session: do not
  inline-implement unreviewed units — journal the outage and exit.
- Marker grammar is load-bearing: `TODO(port):`, `BUG(port):`,
  `PERF(port):`, `// SAFETY:`, `INVENTORY-BUG-CANDIDATE row N:`.
- All ui-visible progress claims must match `git log` reality.

## Session topology (2026-10-17) — idle-time execution

Migration units execute inside IDLE-TIME TASKS (free off-peak compute).
Platform rules for any idle-time run: dispatch agents FOREGROUND only
(no background agents in that mode), and NEVER create further
idle-time tasks or automations — replenishment is owned by the
scheduled SUPERVISOR automation (creation spec: migration/SUPERVISOR.md;
it fires on a cron, keeps the idle queue non-empty by pre-staging one
successor whenever a task is running with none queued, and ships a
single heartbeat unit itself only after an 8h commit stall). Paused
idle tasks are user-held spares — leave them alone. Collisions between
two concurrent runs are prevented by Step 0's seconds-fresh triage.
