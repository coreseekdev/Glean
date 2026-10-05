# Stress test — round 1 report (2026-10-05)

Prompt 03, structure-preserving migration. Round ends at the amendment queue.
Nothing translated here ships.

## Pre-checks

- settings.json: NOT installed → **explicit waiver recorded** (RULEBOOK §7
  DEV-004; user said "continue" at the Step-1b gate after being told
  installation was the prerequisite). In-loop bans honored behaviorally.
- Model tiers not selectable in this harness → DEV-005 (session model for all
  roles).

## File selection (mechanical scoring)

Score = min(inventory rows,6)×2 + min(sites/5,6) + §2 construct weights
(unsafe ×3, mdo ×3, fingerprint ×3, STM/CAF/type-families/Binary ×2, ccall ×1):

| file | score | why |
|---|---|---|
| glean/db/Glean/Query/Codegen.hs | 18 | mdo×30, unsafePerformIO×1 (preProcessPat), 31 sites |
| glean/hs/Glean/RTS.hs | 14 | unsafe×2, ccall×20 |
| glean/db/Glean/Database/Types.hs | 14 | STM/TVar×22, 31 sites |

(Selection with full scores: SELECTION.md)

## Bakeoff (dual translators)

- A (rulebook-faithful, citations): 3 outputs, 253 citations, 2 SILENT flags.
- B (native baseline, rulebook unseen): 3 outputs.
- rustfmt applied to both (2 prefix-syntax errors noted, non-blocking).
- Diff inspector verdicts: **27 rows — rulebook right ×10, native right ×4,
  both defensible ×12, both wrong ×1** (calibrated: B wins 4 outright,
  including 2 real A transcription bugs).

### Bakeoff amendment queue (evidence-backed)

| # | Rule change | Evidence |
|---|---|---|
| A1 | §2 RecursiveDo row: label IDs are allocated at BIND (knot) points, never at mint points; every `x <- label` site must emit its block-split. The disassemble-identity gate is the real referee — even a rule-following translation failed it via transcription slips (D1/D2). Add: "any label-bearing file MUST be judged by the identity gate before promotion" | inspector D1, D2 |
| A2 | §1 FFI row: raw `extern "C"` is the DEFAULT for flat C ABI symbols everywhere (cxx only when bridging C++ classes/inheritance). Flat `glean_*` symbols gain nothing from cxx (D13). Resolves queued amendment 4 with this ruling | inspector D13, B's rts.rs |
| A3 | NEW §2 rows: MVar → slot struct with BLOCKING put (blocks when full) / take (consuming) / read (repeated-read, not try-read); TMVar → Mutex<Option<T>>+Condvar latch (full/empty state preserved); STM `retry` loops need a Condvar on the guarding Mutex (D18, D22, D26; pilot R2 finding 4) | inspector D18/D22/D26 |
| A4 | §2 exceptions row: `deriving Show` tag output is parity-relevant (logs/goldens) — Display replicates constructor-shaped text, no invented prose (D27) | inspector D27 |
| A5 | §2 numeric row: CSize-returning FFI maps to usize with the negative-wraps protocol noted bug-for-bug; never `as usize` a signed check (pilot R1 finding 8) | pilot R1 #8 |
| A6 | §2 partial-functions row: error timing is semantic — source errors on ACCESS stay lazy (per-access check), eager construction-time validation is infidelity (D8) | inspector D8 |
| A7 | §3 addition: a SAFE signature must not launder an unbounded C++ write — unbounded contracts stay `unsafe` + SAFETY even when the Haskell hid it behind a safe type (D11) | inspector D11 |
| A8 | §0/§2: match the source's pattern SHAPE (syntactic), not the semantically-equivalent one — derefType-unwrapping changed SetTy dispatch (pilot R1 finding 4) | pilot R1 #4 |
| A9 | §5 process: divergences from inventory rows are flagged IN FILE as `// INVENTORY-BUG-CANDIDATE row N: ...` — silent divergence (even right-for-right-reason) = failed review (pilot R2 findings 2,3) | pilot R2 #2,#3 |
| A10 | §1 whitelist ruling needed (queued): time — std::SystemTime+Duration only, or admit a time crate? (A's SILENT flag); generated thrift crate name pin `glean_thrift`; Util.Log/glog bridge ownership | pilot SILENT flags |

## Pilot (production pipeline: implementer → 2 reviewers → fixer)

- Implementer: 3 outputs at manifest target paths, 54 rule citations,
  4 SILENT flags (converted to TODO(port) by fixer), honest confidence=low on
  codegen pending the identity gate.
- Reviewers (disjoint batches): **19 findings, all confirmed** —
  - R1 (codegen+rts): 3 INFIDELITY on the label contract (dead MatchAnd arm;
    3 labels allocated but never marked; 6-vs-5 locals), deref-vs-syntactic
    SetTy, missing `load` ownership, ABI/message/underflow minors.
  - R2 (dbtypes): lock-order contract missing the CATALOG edge, silent row-26
    divergence under an elided quote, silent row-136 dyn conflict, missing
    Condvar for STM retry, naming/usize minors.
- **Adherence verdict: PARTIAL both** — "right rules, incompletely executed."
  The pilot proves the pipeline catches exactly this; the fan-out depends on
  the gate (identity judge) + these process amendments.
- Fixer: 18/18 applied; trailers re-verified (todos= 4/3/11, accurate).

## Inventory/rulebook cross-defects found (for the human's amendment batch)

- INVENTORY-BUG-CANDIDATE row 26 (u32/Option vs source Int/TimePoint) — row
  wrong, code right.
- INVENTORY-BUG-CANDIDATE row 136 (logger dyn is genuinely open) — §2 carve-out
  needs a ruling.
- `Glean.Internal.Types` (StorageName) absent from checkout AND manifest —
  pinned provisionally in pilot; needs a ruling.
- `Glean.RTS.Bytecode.Gen.Issue` confirmed GENERATED (correctly not in
  manifest); `load`/code.rs contract comment in codegen.rs is its spec.

## Workspace deletions (confirmed)

- Translator A workspace /home/nzinfo/glean-stress/A/ — deleted below.
- Translator B workspace /home/nzinfo/glean-stress/B/ — deleted below.
- Pilot output /home/nzinfo/glean-stress/pilot/ — deleted below (nothing
  ships from the stress test).

## Round-1 exit

Amendment queue A1-A10 above (4 pre-existing queued items 1-7 in RULEBOOK §7
remain). Applying amendments is the human's act at this gate. After they are
applied, re-paste prompt 03: it resumes as the re-round (fresh files weighted
toward the amended sections + pilot against amended rules).

## Next-step estimate (translation fan-out, prompt 04)

386 manifest units; pilot per-file cost: implementer ~30-45 min/file, reviews
~15 min/file (2×, parallel), fixer ~10 min/file amortized → ~60-70 min/unit
wall with parallelism ≈ 2-4 weeks wall-clock for the full queue, active
attention ~10-15 h total (gate checks + adjudication). Referee price note: the
disassemble-identity gate per label-bearing file adds a judge run per unit —
batched per crate it is minutes, not hours.
