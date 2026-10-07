# Stress test — round 2 report (2026-10-07)

Prompt 03 re-round (v0.2 amendments applied between rounds; fresh files
weighted toward the amended sections). Round ends at the amendment queue.

## Selection (SELECTION-r2.md)

Mechanical scoring weighted to v0.2-amended sections:
| file | score | amended sections exercised |
|---|---|---|
| glean/db/Glean/Query/Transform.hs | 64 | mdo/labels ×28 (A1), syntactic match ×30 (A8) |
| glean/write/Glean/Write/SendQueue.hs | 56 | STM-retry ×12 (A3), Show-tag (A4) |
| glean/hs/Glean/RTS/Typecheck.hs | 50 | mdo/labels ×19 (A1), lazy error timing (A6) |

## Bakeoff (A rulebook-faithful v0.2 vs B native baseline)

B executed by a fresh session (quota killed the in-session dispatches twice;
the task was packaged as ~/glean-stress/TASK-B.md and run manually with
subagent fan-out — B never saw the rulebook, validated by its divergent
philosophy). Outputs: A 3,988 lines / B 3,381 lines (rustfmt'd).

**Verdicts: 16 rows — rulebook right ×4 (D1,D7,D8,D14), native right ×5
(D2,D3,D4,D10,D11), both defensible ×7, both wrong ×0.**
Round 1 was 10/4/12/1 of 27.

### Round-over-round delta

- **A1 (bind-point labels) WORKED as a rule**: B mint-allocates in both
  label-bearing files exactly as round-1 predicted (now unambiguous
  violations); A's LabelPlan made both D1/D14 mechanical verdicts. But A1
  did NOT retire the identity gate — A stayed A1-compliant on numbering
  while still emitting the Record discards before contents (D3) and a
  Term::Ref double-wrap (D4): transcription slips only the
  disassemble-identity gate catches. The gate is load-bearing.
- **A3 (concurrency) CONFIRMED by convergence**: B independently arrived at
  the A3 mapping (one Mutex over the whole STM tuple + condvar re-check
  including status re-read, std::thread, zero async) on the most STM-dense
  file — the natural target, validated. Residue: callback placement (D6)
  and async cancellation (D9/D11) remain unmapped.
- **A9 (INVENTORY-BUG-CANDIDATE protocol) CAUGHT 2 real defects**:
  row 120 (requeue is FIFO append, not push_front — SendQueue.hs:245) and
  row 121 (invents a Finished queue status; SendQueueFinished is an event).
  Both verified against source by the inspector AND reviewer 2.
- **One v0.2 amendment was itself wrong**: §1 Time row's
  "{SystemTime, Duration} ONLY" wording steered A to a wall clock for a
  monotonic TimePoint (D10) — an infidelity caused by the rulebook text.
  Amendment queue item 3 fixes it.
- **A8/A6 exercised and passed on BOTH sides** (SetTy syntactic vs ArrayTy
  deref asymmetry reproduced by both; error "typecheck" lazy in both).

### Round-2 amendment queue (NEW rule gaps — queued for the human)

| # | Rule change | Evidence |
|---|---|---|
| B1 | §2 STM row: STM-invoked callbacks run AFTER the state mutation, OUTSIDE the lock (source atomicity is unreproducible; in-lock calls deadlock on re-entry, out-of-lock opens a callback-run/credit-not-released window — document the divergence) | D6 |
| B2 | §2/§1: async-cancellation translation — canonical approximations for Async.concurrently cancel-loser semantics (fail-the-queue vs let-drain: DEFAULT let-drain + TODO marker) and withAsync ticker cancel (interruptible sleep via wait_timeout) | D9, D11; pilot TODO family |
| B3 | §1 Time row CORRECTION: "TimePoint/monotonic → std::time::Instant; wall clock → SystemTime" — v0.2's own wording caused an infidelity | D10 |
| B4 | §4: draft stub policy — sibling-unit items are ASSUMED-IMPORTS with an in-file contract comment (A's pattern), never in-file todo!() type re-declarations (B's pattern forks shared types across 383 units and pins generic APIs) | D16 |
| B5 | §2: callFun_N_M stays an emitter over an opaque SysCall handle — do not type raw C++ entry points as extern "C" fn unless Rust itself invokes them | D15 |
| B6 | §2 Show-tag row scope: non-Show sum types (e.g. SendQueueEvent) get constructor-shaped Display when any consumer could Display; plus a §2 note that bimap/bifoldMap on Type_ walk the FULL tree (nested predicates evolve) | D13, D2 |

## Pilot (production pipeline: implementer → 2 adversarial reviewers → fixer)

- Implementer: 3 outputs, 72 rule citations; all 4 known-hard spots handled
  (bind orders incl. the vacuous :234 knot; full-tree recursion;
  contents-before-discards; Instant). Honest bonus finds:
  transitiveDeps emits its root; SendQueueFinished constructed nowhere.
- Reviewer 1 (transform+typecheck): **OBEDIENT** — all 17 mdo knots correct,
  cross-file deltas kept verbatim, 6 findings (1 SILENT-GAP Debug-parity
  obligation, 5 MINOR incl. a real mixed-kind-local contract fix).
- Reviewer 2 (send_queue): **PARTIAL** — 8 findings; 3 substantive:
  non-UnknownBatchHandle poll failure delivered the original error where
  the source escapes PatternMatchFail (BUG-rule violation); the
  action→worker kill direction missing and un-flagged; two row-121 sketch
  divergences silent (§5/A9 violations). Credit/select/close algebra exact.
- Fixer: 14/14 applied, incl. the PatternMatchFail bug-for-bug reproduction
  (BUG(port) marker), both kill-direction TODOs, both row-121 flags,
  heterogeneous locals, visibility parity, wait_while predicates,
  show-shaped exception rendering, action-error precedence. Trailers
  accurate: transform todos=1, typecheck todos=0, send_queue todos=5
  (+1 BUG, +1 PERF, +4 INVENTORY-BUG-CANDIDATE).

Adherence verdicts: OBEDIENT (r1) / PARTIAL (r2) — improved from round 1's
PARTIAL/PARTIAL; the amendments and the fix-loop are compounding.

## INVENTORY-BUG-CANDIDATE (confirmed, to fix in the amendment batch)

- row 120: requeue is push_back (FIFO); row text says push_front.
- row 121: no Finished queue status; SendQueueFinished is an event;
  SharedError(Arc)/Callback(Result) shapes flagged as sketch-vs-impl notes.

## Workspace deletions (confirmed)

- /home/nzinfo/glean-stress/A — deleted
- /home/nzinfo/glean-stress/B — deleted
- /home/nzinfo/glean-stress/pilot — deleted
Nothing ships from the stress test.

## Round-2 exit

Amendment queue B1-B6 + the two inventory-row fixes above are queued for the
human. No section was re-amended twice (the early-stop trigger did not fire;
B3 amends v0.2's A10 wording once). Sign-off gates the Step-3 fan-out.

## Next-step estimate (Step 3 translation fan-out, prompt 04)

386 units; round-2 pilot per-file: implementer ~30 min/file, reviews
~15 min/file (2× parallel), fixer ~10 min amortized → ~55-65 min/unit with
pipeline parallelism ≈ 2-3.5 weeks wall-clock; active attention 8-12 h
(gate checks + adjudication). Quality trend r1→r2 (PARTIAL/PARTIAL →
OBEDIENT/PARTIAL, fix-loop closing gaps) supports proceeding.
