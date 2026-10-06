# Stress-test file selection (round 2 — weighted toward amended sections)

Weights: A1 labels ×3, A3 MVar/TMVar/retry ×3, A5/A7 FFI ×2, A4 ×1, A8 ×1, + inventory/site density. Fresh files only (round-1 files excluded).

- glean/db/Glean/Query/Transform.hs score=64 inv=1 sites=11: mdo/labels (A1)×28, syntactic pattern depth (A8)×30
- glean/write/Glean/Write/SendQueue.hs score=56 inv=2 sites=18: STM-retry (A3)×12, Show-tag types (A4)×1, syntactic pattern depth (A8)×11
- glean/hs/Glean/RTS/Typecheck.hs score=50 inv=1 sites=8: mdo/labels (A1)×19, syntactic pattern depth (A8)×1
- glean/hs/Glean/RTS/Traverse.hs score=34 inv=1 sites=4: mdo/labels (A1)×10, syntactic pattern depth (A8)×1
- glean/lsp/Glean/LSP.hs score=31 inv=3 sites=5: mdo/labels (A1)×3, Show-tag types (A4)×2, syntactic pattern depth (A8)×13
- glean/write/Glean/Write/Async.hs score=30 inv=1 sites=4: MVar/TMVar (A3)×4, STM-retry (A3)×4, syntactic pattern depth (A8)×3
- glean/client/hs/Glean/Remote.hs score=25 inv=2 sites=8: STM-retry (A3)×2, syntactic pattern depth (A8)×13
- glean/hs/Glean/RTS/Foreign/Lookup.hs score=19 inv=1 sites=6: FFI CSize/ownership (A5,A7)×8
- glean/tools/gleancli/GleanCLI/Derive.hs score=9 inv=1 sites=2: mdo/labels (A1)×2, syntactic pattern depth (A8)×1
- glean/glass/Glean/Glass/Env.hs score=5 inv=1 sites=2: STM-retry (A3)×1

Selected: `glean/db/Glean/Query/Transform.hs`, `glean/write/Glean/Write/SendQueue.hs`, `glean/hs/Glean/RTS/Typecheck.hs`
