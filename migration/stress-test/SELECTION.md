# Stress-test file selection (round 1)

Scoring: inventory rows touching the file (×2, cap 6) + site density (cap 6) + rulebook §2 riskiest-construct weights (unsafe ×3, mdo ×3, fingerprint ×3, STM ×2, CAF ×2, type-families ×2, Binary ×2, ccall ×1).

- **SELECTED** `glean/db/Glean/Query/Codegen.hs` score=18 inventory_rows=3 sites=31 constructs: unsafe×1, mdo/RecursiveDo×30
- **SELECTED** `glean/hs/Glean/RTS.hs` score=14 inventory_rows=3 sites=22 constructs: unsafe×2, FFI ccall×20
- **SELECTED** `glean/db/Glean/Database/Types.hs` score=14 inventory_rows=3 sites=31 constructs: STM/TVar×22
-    `glean/typed/Glean/Typed/Predicate.hs` score=11 inventory_rows=2 sites=4 constructs: unsafe×3, CAF×2, type families×3
-    `glean/angle/Glean/Angle/Hash.hs` score=8 inventory_rows=1 sites=4 constructs: unsafe×2, fingerprint×4
-    `glean/hs/Glean/RTS/Foreign/Inventory.hs` score=8 inventory_rows=1 sites=13 constructs: unsafe×5, FFI ccall×8
-    `glean/angle/Glean/Angle/Types.hs` score=6 inventory_rows=1 sites=11 constructs: Binary×11
-    `glean/db/Glean/Database/Write/Queue.hs` score=5 inventory_rows=1 sites=5 constructs: STM/TVar×1

Selected top 3 (mechanical criteria, not taste):
1. `glean/db/Glean/Query/Codegen.hs` (score 18)
1. `glean/hs/Glean/RTS.hs` (score 14)
1. `glean/db/Glean/Database/Types.hs` (score 14)
