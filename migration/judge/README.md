# Glean migration judge (migration kit prompt 00b)

A portable parity harness: it drives **any** `glean`-compatible CLI (the
Haskell original today, the Rust port in Step 6) through the public interface
and diffs observable results. The old code is the executable spec; this
directory is how you query it.

## Components

- `judge.py` — the harness. Semantics ported exactly from
  `glean/test/regression/Glean/Regression/Snapshot/{Query,Transform}.hs`
  (output protocol, nukeIds, sort/gensort/extract/normord, Text.JSON Ord).
  Read its module docstring for the full list of deliberate deviations and
  interface gaps (base64 binary fields, one-shot vs paging collect, CLI flags
  the Rust port must grow).
- `cases/` — seed scenarios. Each case = `batch*.json` (facts, written to one
  fresh DB per case) + `*.query` + golden:
  - `main.out` — a real golden (copied from upstream fixtures; byte-truth).
  - `.out.frozen` — old-code output recorded at validation time (marked in
    reports as `frozen_golden: true`; eyeballed by the human at the 00b gate).
- `impls.sh` (created at validation) — the command templates for old/new.

## Modes

```sh
# 1. Old code must reproduce every real golden cleanly (spec passes its own spec)
python3 judge.py --cases cases validate-old \
  --impl-old '<glean-binary> --db-root {root}' --freeze

# 2. The judge must catch deliberately broken implementations
python3 judge.py --cases cases mutation-selftest \
  --impl-old '<glean-binary> --db-root {root}'

# 3. Step 6 referee: new vs old on identical inputs (+ new vs golden)
python3 judge.py --cases cases parity \
  --impl-old '<old> --db-root {root}' \
  --impl-new '<new> --db-root {root}'
```

Exit 0 = all checks passed; 1 = any FAIL / MISSED mutation / no-golden;
2 = no cases loaded. `--keep` preserves the per-impl DB roots for debugging.

## Validation status

See `migration/cost-log.tsv` and the 00b gate report. The judge is not
trusted until: (a) validate-old passes clean on the original binary,
(b) every mutation is CAUGHT, (c) both adversarial reviews are resolved
(protocol review done — 10 findings fixed; transforms review done inline —
normord sort key + argument typing fixed).

## Extending the corpus

The seed cases exercise write→finish→query over `src.File` with order
sensitive/insensitive comparisons, field matching, paging, and extract.
Grow breadth by adding a `batch*.json` + `.query` (+ `.out` produced by
running `validate-old --freeze` and reviewing the frozen output). Corpus
cases must avoid binary-typed fields until the port grows
`--no-base64-binary`.
