// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// Ported from: glean/github/TestRunner.hs (70 lines)
// Manifest unit: rust/glean-util/src/test_runner.rs (migration/manifest.tsv
// line 77, crate glean-util). RULE §4: one source file -> one target file;
// the manifest path is authoritative.

//! HUnit -> tasty glue, and why this module has no code.
//!
//! The entire source is four declarations (RULE §0: the old code is the
//! spec):
//!
//! ```haskell
//! testRunner :: Test -> IO ()                  -- :21-25
//! testRunner t = Tasty.defaultMain $ Tasty.testGroup "test" $
//!                hUnitTestToTestTree t
//!
//! data TestAction = TestAction                 -- :27
//!
//! testRunnerAction :: TestAction -> Test -> IO ()   -- :41-46
//! testRunnerAction _ t = withArgs [] $ Tasty.defaultMain $ ...
//!
//! hUnitTestToTestTree :: HUB.Test -> [Tasty.TestTree]  -- :52-70
//! ```
//!
//! `testRunner` is the entry point every test executable calls: it runs the
//! HUnit `Test` tree under tasty, which the source's own comment (:19-20)
//! says buys parallel runs, test listing and test selection. `TestAction`
//! is an option-parser marker: `testRunnerAction _ t` is the same runner
//! with tasty's own CLI parser disabled (`withArgs []`, :43) so a program
//! can combine the runner with its own options without the circular
//! dependency described at :29-40 — environment variables such as
//! `TASTY_NUM_THREADS` still apply. `hUnitTestToTestTree` is a pure AST
//! conversion (a modified tasty-hunit-adapter): it threads a `desc`
//! prefix, joining labels with ":" (:56-58), flattens a top-level list
//! (:62) but turns a described list into a tasty group (:65), and re-wraps
//! HUnit's exception into tasty-hunit's with the formatted reason
//! (:67-70).
//!
//! # The decision: no Rust image
//!
//! DECISIONS.md §4 rules that test suites are NOT translated — "they die
//! with the language" — and RULEBOOK §3 sends tests to each crate's
//! `tests/` directory, judged by the `.query`/`.out` golden corpus and
//! dual-implementation parity. This module is pure test-harness plumbing:
//! it has no HUnit `Test` AST to convert (Rust has `#[test]` functions,
//! not a reified tree), no tasty `TestTree` to build, and no runner to
//! configure (`cargo test` supplies parallelism, listing and selection —
//! exactly the three things :19-20 asks tasty for). Porting it would mean
//! INVENTING a test framework and an AST for it, which RULE §0/§2 forbid:
//! there is no old code to be faithful to once the frameworks are gone.
//!
//! Consumer survey (`grep -rn 'TestRunner' glean/`): every importer is a
//! test executable — glass/test/*, haxl/tests/IntegrationTest.hs,
//! lib/tests/RangeTest.hs, test/tests/**, test/regression/**, and
//! interprocess/hs/Glean/Interprocess/Test/TestWorklist.hs — with ONE
//! non-test hit, glean/github/Glean/Init.hsc:33, whose
//! `withUnitTestOptions` (:103-104) only hands a `TestAction` to a binary's
//! test entry point (see SIBLING-UNIT CONTRACTS). Nothing outside the
//! Haskell test ecosystem consumes this module.
//!
//! # What replaces it
//!
//! Nothing, by design. A Rust test binary is `cargo test`; a test that
//! needs its own options parses them itself. The two runner functions and
//! the marker type have no call site in the ported tree, and the AST
//! conversion's remaining value (naming and exception re-wrapping) is the
//! framework's own job.
//!
//! This module therefore intentionally ships ZERO items, like
//! `glean_util::util::some` (row 38). The path is kept (RULE §4: the
//! manifest is authoritative) so this decision record sits where consumers
//! and reviewers look for it.

// Coverage: the 70-line source is fully accounted for — four declarations,
// all of them test-framework glue with no Rust image (see above); the
// module exports `module TestRunner`, i.e. exactly those four names and
// nothing else.
//
// Marker breakdown: zero of every class — 0 port todos (the decision is
// POLICY, cited to DECISIONS §4 + RULEBOOK §3, not a deferral), 0
// source-defect reproductions, 0 performance deferrals (no code), 0
// inventory divergences (no inventory row names this file:
// `grep -c 'TestRunner' migration/inventory*.tsv` is 0 in all six files),
// 0 SAFETY notes (no unsafe code).
//
// SIBLING-UNIT CONTRACTS
//   * glean/github/Glean/Init.hsc (manifest line 78, this crate): imports
//     TestRunner only for `withUnitTestOptions :: ParserInfo a -> (TestAction
//     -> a -> IO b) -> IO b` (:103-104), which applies the caller's
//     continuation to the marker value. Its port must DROP that combinator
//     (or accept the flag as a no-op): there is no TestAction to pass and no
//     embedded test runner to start. Recorded here because Init.hsc is the
//     only non-test importer.
//   * Every other importer is a Haskell test executable, which DECISIONS §4
//     leaves untranslated.
//
// PORT STATUS: confidence=high todos=0
