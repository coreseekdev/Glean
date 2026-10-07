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
