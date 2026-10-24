// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

// MANIFEST UNIT: this file IS manifest row 93's target
// (glean/db/Glean/Query/Prune.hs -> rust/glean-db/src/query/prune.rs,
// migration/manifest.tsv:93). The source is 270 lines; the module header
// exports exactly one name (`module Glean.Query.Prune (pruneDerivations)
// where`, Prune.hs:9) — everything else in the file is private, and the
// same holds here (see EXPORT PARITY at the bottom).
//
// -----------------------------------------------------------------------------
// INVENTORY (RULE §5)
// -----------------------------------------------------------------------------
// ZERO rows. grep -n 'Query/Prune.hs' migration/inventory*.tsv (inventory.tsv,
// .ffi, .protocol, .sites, .stm, .codeflow) returns NOTHING; grep -rn
// 'Query/Prune.hs' migration/ hits HANDOFF.md:201/:225/:471, depmap/
// edges.tsv (:374 IN, :614-:620 OUT), depmap/order.txt:92 and
// manifest.tsv:93 — no
// inventory row exists, so no target_translation can diverge and no
// inventory-divergence flag is possible. Nothing in the file is an
// inventory-owned class either: no FFI, no STM, no unsafePerformIO, no
// dynamic exception, no I/O. The one partial function is a pure `error`
// call mapped by RULE §2's partial-functions row (Prune.hs:200); every other
// pattern match is exhaustive by type.
//
// -----------------------------------------------------------------------------
// DEPENDENCY TRUTH (migration/depmap/edges.tsv — checked, not assumed)
// -----------------------------------------------------------------------------
// OUT (7 edges; source import block Prune.hs:11-32):
//   :614 Prune.hs -> angle/Glean/Angle/Types.hs (Prune.hs:26-27
//        'import Glean.Angle.Types (PredicateId(..), DerivingInfo(..))' +
//        'qualified ... as Angle') -> crate glean-angle,
//        glean_angle::angle::types: PredicateId (:931), DerivingInfo
//        (:1039), DeriveWhen (:1059, the field of Derive), and Type_
//        (:749) for the PredicateTy guard at Prune.hs:187-189.
//   :615 Prune.hs -> db/Glean/Database/Schema/Types.hs (Prune.hs:24)
//        -> SAME CRATE, crate::database::schema::types: PredicateDetails
//        (:756) and its predicate_deriving field (:770).
//   :616 Prune.hs -> db/Glean/Query/Codegen/Types.hs (Prune.hs:28, the
//        unqualified whole-module import) -> SAME CRATE,
//        crate::query::codegen::types: QueryWithInfo (:148), Generator
//        (:232), Var (:323), Match (:378), Typed (:574).
//   :617 Prune.hs -> db/Glean/Query/Typecheck.hs (Prune.hs:29
//        'import Glean.Query.Typecheck (tcQueryDeps)') -> SAME CRATE,
//        crate::query::typecheck::tc_query_deps (:2396).
//   :618 Prune.hs -> db/Glean/Query/Typecheck/Types.hs (Prune.hs:30-31)
//        -> SAME CRATE, crate::query::typecheck::types: TcPat (:245),
//        TcTerm (:360), TcStatement (:190), TcQuery (:133),
//        TypecheckedQuery (:587).
//   :619 Prune.hs -> hs/Glean/RTS/Term.hs (Prune.hs:32
//        'import Glean.RTS.Term (Term(..))') -> crate glean-core,
//        glean_core::rts::term::Term (:130).
//   :620 Prune.hs -> hs/Glean/RTS/Types.hs (Prune.hs:25
//        'import Glean.RTS.Types (PidRef(..), Type)') -> crate glean-core,
//        glean_core::rts::types: PidRef (:185), Type (:250).
// IN (the consumer, not a dependency of this file): :374
//   db/Glean/Database/Schema.hs -> Prune.hs (Schema.hs:103 the import,
//   Schema.hs:719 the call `pruneDerivations hasFacts pmap`). That unit is
//   unshipped; its call-shape obligation is recorded under SIBLING-UNIT
//   CONTRACTS below.
//
// -----------------------------------------------------------------------------
// ASSUMED-IMPORT (RULE §4 B4)
// -----------------------------------------------------------------------------
// NONE. This unit consumes no thrift-generated type, no build.rs output and
// no forward-declared module: every symbol above is a shipped sibling whose
// signature was read on disk before use (line cites in DEPENDENCY TRUTH).
// The two non-obvious images:
//   * `PredicateId` is NOT `Copy` (angle/types.rs:931: a PredicateRef +
//     Hash, Clone only) — hence every callback below takes `&PredicateId`,
//     never a by-value key.
//   * `PredicateId`'s Eq and Ord disagree by design (angle/types.rs:919-929,
//     the BUG-rule reproduction: Eq compares the hash only, derived Ord
//     compares (ref, hash) lexicographically). This file touches both:
//     HashMap keying uses Eq+Hash (Prune.hs:46/:55/:73/:78), the graph
//     vertex order uses Ord (Prune.hs:89-93 via Data.Graph's internal
//     `sortBy (comparing fst)`). Both sides of the port use the same
//     instance the source uses, so the quirk travels unchanged. It cannot
//     make the sort ambiguous: two keys that Eq-equal are one HashMap key,
//     so the key set has no Ord-ties.
//
// -----------------------------------------------------------------------------
// SIBLING-UNIT CONTRACTS (what this file needs, and what the next unit owes)
// -----------------------------------------------------------------------------
// CONSUMED (signatures as shipped):
//   tc_query_deps(&TcQuery) -> BTreeSet<PredicateId>
//     — typecheck.rs:2396; BTreeSet is the image of `Set.toList`'s ascending
//       order at Prune.hs:93, which IS the DFS successor order (ORDERING).
//   QueryWithInfo<Q> { qi_query, qi_num_vars: i64, qi_generator:
//     Option<Generator>, qi_return_type: Type } — codegen/types.rs:148-155;
//     Prune.hs:107 binds `q _ gen t` positionally, and :215 rebuilds it.
//   TypecheckedQuery = QueryWithInfo<TcQuery> — types.rs:587.
//   TcQuery(Type, TcPat, Option<TcPat>, Vec<TcStatement>, Ordered)
//     — types.rs:133 (positional, 5 fields; Prune.hs:110-118).
//   TcStatement(Type, TcPat, TcPat) — types.rs:190.
//   TcPat = Term<Match<Typed<TcTerm>, Var>> — types.rs:245.
//   TcTerm (16 variants) — types.rs:360; its Box fields are rebuilt with
//     `Box::new` exactly where the source's constructors sit.
//   Match<Ext, V> (9 variants) — codegen/types.rs:378; Typed<X>(Type, X)
//     :574; Var { var_type, var_id: i64, var_orig_name } :323.
//   PredicateDetails.predicate_deriving: DerivingInfo<TypecheckedQuery>
//     — schema/types.rs:770.
//   Term<R> (8 variants) — term.rs:130; its `map` (:154) is the Functor
//     image used by rename_pat.
//   PidRef { pid, predicate_id } — rts/types.rs:185; Type — :250.
//
// OWED TO THE CONSUMER (row: db/Glean/Database/Schema.hs, Schema.hs:719):
//   The Haskell call is `pruneDerivations hasFacts pmap`, where `hasFacts`
//   CLOSES OVER `pmap` (Schema.hs:719-728). Rust cannot reproduce that call
//   literally: `details` is consumed here (it must be — PredicateDetails is
//   not Clone, schema/types.rs:753-755, and the source's `details { … }`
//   record update shares everything but the derivation), so a closure
//   borrowing the same map cannot be passed alongside it (E0505). The
//   consumer therefore precomputes the predicate's boolean answer — e.g. a
//   HashMap<PredicateId, bool> built from (pmap, transformations, stats) —
//   and passes a closure over THAT. Same question, same answers, same call
//   count per predicate; only the capture target moves. Recorded here
//   because the consumer's brief must not discover it at the call site.
//   The callback is `impl Fn(&PredicateId) -> bool` (the `filter_f`
//   precedent, database/catalog/filter.rs:521) rather than the source's
//   by-value arrow: PredicateId is not Copy and every caller only reads.
//
// ALSO OWED TO ROW 88's FILE: codegen/types.rs's bitraverse block records
//   that `bitraverse` (Match's Bitraversable instance) is NOT imaged there
//   and that its ONE consumer — Prune.hs:230 `renamePat = traverse
//   (bitraverse renameTyped renameVar)` — ports its own stateful traversal.
//   This file pays that debt: see `Renumber` below (hand-written traversal,
//   same visit order, no speculative `try_bimap` API).
//
// -----------------------------------------------------------------------------
// ORDERING — THE PART THAT IS SEMANTIC (Prune.hs:80-93)
// -----------------------------------------------------------------------------
// `topologicalSort` is not a layout helper: its output feeds
// `foldr add mempty` (Prune.hs:64-65), and `add` decides with
// `hasFacts` whether a child's derivation was KEPT (Prune.hs:75-78:
// `child == predId || hasStoredFacts child || child member
// derivedWithFacts`). foldr therefore visits the list from the RIGHT, so
// the rightmost element is pruned first and the leftmost last. A child
// must be on the right of its parent for the parent to see it in
// `derivedWithFacts` — i.e. the list must be parent-before-child
// (reverse topological), which is exactly what Data.Graph.topSort returns.
//
// The exact order is pinned, not assumed (GHC 9.4.7's containers, probes
// /tmp/probe_graph.hs and /tmp/probe_graph2.hs, and the replica comparison
// /tmp/probe_ts.rs):
//   1. `Graph.graphFromEdges` assigns vertex i to the i-th key in ASCENDING
//      `Ord PredicateId` order — it sorts the edge list by key before
//      numbering. Probe: the same four edges passed in reverse order give
//      the identical vertex assignment and identical adjacency
//      (probe_graph.hs prints fromV = key 1,2,3,4 for vertices 0,1,2,3
//      from both orders). So the RESULT DOES NOT DEPEND ON HashMap
//      ITERATION ORDER; no "deterministic refinement" is needed — sorting
//      the edge list by PredicateId is the faithful reproduction, not a
//      divergence. (The brief anticipated a hash-order dependence; the
//      probe refutes it. See the report.)
//   2. Adjacency preserves the dependency list's order (probe_graph2.hs:
//      deps [1,2] -> adj [1,2]; deps [2,1] -> adj [2,1]). Note this is
//      graphFromEdges' own builder, NOT buildG's `accumArray (flip (:))`,
//      which does reverse (probe_graph2.hs prints buildG's adj [2,1] for
//      edges [(0,1),(0,2)]). Our successor order is therefore
//      tcQueryDeps' BTreeSet order = ascending PredicateId
//      (typecheck.rs:2396), exactly `Set.toList` (Prune.hs:93).
//   3. `topSort = reverse . postOrd`: a DFS forest over roots 0..n-1 in
//      ascending order, successors in adjacency order, postorder, then
//      reversed. `post_order` below is the iterative form of that DFS
//      (same visit order, no stack-overflow ceiling); `top_sort` reverses
//      it.
// Evidence for 3: 400 pseudo-random graphs (3-11 vertices, random edges,
// fixed LCG seed in /tmp/gen_graphs.hs) were sorted by Data.Graph.topSort
// and by the exact code below; `diff` of the two 400-line outputs is
// empty. The 2-cycle case is pinned too: entries keyed 10,20,30,40 with
// 10->20, 20->10, 30->20 give topSort entries [40,30,10,20], so foldr
// processes 20 before 10 and the SECOND member of a cycle sees the first
// in `derivedWithFacts`. That asymmetry is inherent to the source's
// algorithm (a 2-cycle has no topological order); we reproduce it rather
// than "fix" it.
//
// -----------------------------------------------------------------------------
// THE LAZY KNOT (Prune.hs:40-58) — flattened, not cycled
// -----------------------------------------------------------------------------
// The source is a letrec: `pruned = pruneQueries hasStoredFacts
// derivations` (:52) and `updateDerivation` reads `pruned` (:46) while
// `derivations` is projected out of `details` (:55). It terminates
// because `pruneQueries` depends only on `derivations`, never on
// `pruned`. The port computes the three stages in that order —
// derivations, then pruned, then the record update — with no cycle and no
// fixpoint.
//
// The queries MOVE along that pipeline instead of being shared: the source
// shares one TypecheckedQuery between `derivations`, `pruned` and the
// rebuilt PredicateDetails, but QueryWithInfo derives nothing
// (codegen/types.rs:145-148) and PredicateDetails is not Clone
// (schema/types.rs:753-755), so there is nothing to clone from. Queries
// are lifted out with `mem::replace` (in `prune_derivations` below), the
// DeriveWhen tag is set aside, and the survivor is put back. That is a
// representation-level move of the same value, not a semantic change: the
// `when` written back is the ORIGINAL one (Prune.hs:44/:48), never the
// pruned query's.
//
// -----------------------------------------------------------------------------
// INVARIANTS THE PRUNER PRESERVES (source comment Prune.hs:96-105)
// -----------------------------------------------------------------------------
// "Pruning won't leave us with unbound variables because: if a statement in
// a sequence is set to fail, the entire sequence is removed; if one
// sequence is removed from a disjunction, all variables bound by it should
// also be bound by the other disjunction alternatives." Those two rules are
// the shape of `prune_sequence` (all-or-nothing `traverse`) and of the
// TcOr/TcIf `asum` arms, transcribed literally below. The comment is
// transcribed so a future reader does not "optimise" either one.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use glean_angle::angle::types::{DeriveWhen, DerivingInfo, PredicateId, Type_};
use glean_core::rts::term::Term;
use glean_core::rts::types::{PidRef, Type};

use crate::database::schema::types::PredicateDetails;
use crate::query::codegen::types::{Generator, Match, QueryWithInfo, Typed, Var};
use crate::query::typecheck::tc_query_deps;
use crate::query::typecheck::types::{TcPat, TcQuery, TcStatement, TcTerm, TypecheckedQuery};

// -----------------------------------------------------------------------------
// pruneDerivations (Prune.hs:36-58) — THE ONE EXPORT
// -----------------------------------------------------------------------------

/// `pruneDerivations :: (PredicateId -> Bool) -> HashMap PredicateId
/// PredicateDetails -> HashMap PredicateId PredicateDetails` (Prune.hs:36-39)
/// — "Remove all branches that we know will not succeed given the content of
/// the database" (source haddock, :34-35).
///
/// Three stages, in the source's order (see THE LAZY KNOT):
///   1. `derivations` (:55-58): every entry whose `predicateDeriving` is
///      `Derive _ query`, projected to its query. `NoDeriving` entries are
///      not derivations at all.
///   2. `pruned` (:52): `prune_queries` over that map.
///   3. `updateDerivation` (:43-49): for entries that HAD a `Derive when _`,
///      replace the whole DerivingInfo with `NoDeriving` when the query did
///      not survive, else `Derive when <pruned query>` (:45-48). Entries that
///      were `NoDeriving` are returned untouched (:43's `fromMaybe details`).
///
/// The `when` used in the rebuild is the original one, taken from `details`
/// (:44's `Derive when _ <- return $ predicateDeriving details`), NOT
/// anything recovered from the pruned query.
///
/// The callback shape and its consumer obligation are pinned in
/// SIBLING-UNIT CONTRACTS above.
pub fn prune_derivations(
    has_stored_facts: impl Fn(&PredicateId) -> bool,
    details: HashMap<PredicateId, PredicateDetails>,
) -> HashMap<PredicateId, PredicateDetails> {
    // derivations = flip HashMap.mapMaybe details $ \PredicateDetails{..} ->
    //   case predicateDeriving of
    //     NoDeriving -> Nothing
    //     Derive _ query -> Just query                       (Prune.hs:55-58)
    //
    // The query is MOVED out (TypecheckedQuery is not Clone — THE LAZY KNOT);
    // DeriveWhen is Copy (angle/types.rs:1058) and is carried alongside so
    // stage 3 can rebuild Derive when ... with the original tag.
    let mut derivations: HashMap<PredicateId, TypecheckedQuery> =
        HashMap::with_capacity(details.len());
    let mut entries: Vec<(PredicateId, PredicateDetails, Option<DeriveWhen>)> =
        Vec::with_capacity(details.len());

    for (pred_id, mut entry) in details {
        match std::mem::replace(&mut entry.predicate_deriving, DerivingInfo::NoDeriving) {
            DerivingInfo::NoDeriving => entries.push((pred_id, entry, None)),
            DerivingInfo::Derive(when, query) => {
                derivations.insert(pred_id.clone(), query);
                entries.push((pred_id, entry, Some(when)));
            }
        }
    }

    // pruned = pruneQueries hasStoredFacts derivations          (Prune.hs:52)
    let mut pruned = prune_queries(&has_stored_facts, derivations);

    // updateDerivation <$> details                              (Prune.hs:41)
    entries
        .into_iter()
        .map(|(pred_id, mut entry, when)| {
            if let Some(when) = when {
                // case HashMap.lookup (predicateId details) pruned of
                //   Nothing -> NoDeriving
                //   Just query -> Derive when query                 (:45-48)
                //
                // The lookup key is the FIELD, not the map key (:46): the
                // source's map is built with the field as its key
                // (Schema.hs:868 'HashMap.insert (predicateId details)
                // details'), so the two coincide — but the field is what the
                // source writes, so it is what this line reads.
                //
                // `remove`, not `get`: the query cannot be cloned out of the
                // map (TypecheckedQuery is not Clone), and every field value
                // is a unique key (Schema.hs:868), so taking the entry is the
                // image of the source's pure lookup.
                entry.predicate_deriving = match pruned.remove(&entry.predicate_id) {
                    None => DerivingInfo::NoDeriving,
                    Some(query) => DerivingInfo::Derive(when, query),
                };
            }
            (pred_id, entry)
        })
        .collect()
}

// -----------------------------------------------------------------------------
// pruneQueries (Prune.hs:60-78)
// -----------------------------------------------------------------------------

/// `pruneQueries :: (PredicateId -> Bool) -> HashMap PredicateId
/// TypecheckedQuery -> HashMap PredicateId TypecheckedQuery` (Prune.hs:60-63)
/// — `foldr add mempty $ topologicalSort derivations` (:64-65).
///
/// `foldr` is `for ... in ....rev()`: the RIGHTMOST entry of the topological
/// list is pruned first (ORDERING above), and `add`'s accumulated map is the
/// `derivedWithFacts` that `hasFacts` consults. `add` returns the map
/// unchanged when `prune` yields Nothing (:71-73) — the predicate then has no
/// entry in the result, which is what turns it into `NoDeriving` upstream.
fn prune_queries(
    has_stored_facts: &dyn Fn(&PredicateId) -> bool,
    derivations: HashMap<PredicateId, TypecheckedQuery>,
) -> HashMap<PredicateId, TypecheckedQuery> {
    let mut derived_with_facts: HashMap<PredicateId, TypecheckedQuery> = HashMap::new();

    for (pred_id, query) in topological_sort(derivations).into_iter().rev() {
        // hasFacts predId derivedWithFacts child =
        //   child == predId
        //     || hasStoredFacts child
        //     || child `HashMap.member` derivedWithFacts        (Prune.hs:75-78)
        //
        // All three disjuncts matter: the self-reference keeps a query whose
        // only generator is on its own predicate alive; the stored-facts arm
        // is the caller's DB knowledge; the membership arm is "a child was
        // already kept". Scoped so the borrow of derived_with_facts ends
        // before the insert below.
        let pruned = {
            let has_facts = |child: &PredicateId| {
                child == &pred_id
                    || has_stored_facts(child)
                    || derived_with_facts.contains_key(child)
            };
            prune(&has_facts, query)
        };

        if let Some(pruned) = pruned {
            derived_with_facts.insert(pred_id, pruned);
        }
    }

    derived_with_facts
}

// -----------------------------------------------------------------------------
// topologicalSort (Prune.hs:80-93) — see ORDERING
// -----------------------------------------------------------------------------

/// `topologicalSort :: HashMap PredicateId TypecheckedQuery ->
/// [(PredicateId, TypecheckedQuery)]` (Prune.hs:80-82).
///
/// The source builds `Graph.graphFromEdges` over `(entry, predId,
/// dependencies)` triples (:89-93) and projects `fromVertex` over
/// `Graph.topSort` (:84-87). `graphFromEdges`' three results are the graph,
/// the vertex->entry function and a key lookup; only the first two are used,
/// hence the port's two structures (a key->vertex BTreeMap and the entry
/// vector). The BTreeMap is not a source container image — it is the sorted
/// key table `graphFromEdges` builds internally for `key_vertex`, keyed by
/// Ord (ORDERING point 1). Edges to predicates that have no derivation are
/// dropped, exactly as `key_vertex` returning Nothing drops them (:92's
/// `Just w <- [key_vertex k]`).
fn topological_sort(
    derivations: HashMap<PredicateId, TypecheckedQuery>,
) -> Vec<(PredicateId, TypecheckedQuery)> {
    let mut edges: Vec<(PredicateId, TypecheckedQuery, BTreeSet<PredicateId>)> = derivations
        .into_iter()
        .map(|(pred_id, query)| {
            let dependencies = tc_query_deps(&query.qi_query);
            (pred_id, query, dependencies)
        })
        .collect();

    // graphFromEdges sorts by key before numbering vertices (ORDERING 1).
    // Ord, not the HashMap's Eq: angle/types.rs:919-929 pins that the two
    // disagree in the source too, and the source sorts with Ord here.
    edges.sort_by(|a, b| a.0.cmp(&b.0));

    let index: BTreeMap<PredicateId, usize> = edges
        .iter()
        .enumerate()
        .map(|(vertex, edge)| (edge.0.clone(), vertex))
        .collect();

    // Adjacency keeps the dependency list's order (ORDERING 2).
    let graph: Vec<Vec<usize>> = edges
        .iter()
        .map(|(_, _, dependencies)| {
            dependencies
                .iter()
                .filter_map(|dependency| index.get(dependency).copied())
                .collect()
        })
        .collect();

    let mut entries: Vec<Option<(PredicateId, TypecheckedQuery)>> = edges
        .into_iter()
        .map(|(pred_id, query, _)| Some((pred_id, query)))
        .collect();
    let mut sorted = Vec::with_capacity(entries.len());
    for vertex in top_sort(&graph) {
        // topSort emits a permutation of the vertex set: every vertex has
        // exactly one postorder emission in post_order.
        sorted.push(
            entries[vertex]
                .take()
                .expect("topSort emits each vertex exactly once"),
        );
    }
    sorted
}

/// `Graph.topSort` = `reverse . postOrd` (ORDERING 3).
fn top_sort(graph: &[Vec<usize>]) -> Vec<usize> {
    let mut order = post_order(graph);
    order.reverse();
    order
}

/// The postorder of the DFS forest over roots `0..n-1` with successors in
/// adjacency order. Iterative (explicit frame stack) so a deep dependency
/// chain cannot overflow the Rust stack; the visit order is the recursive
/// one — the `(vertex, next-successor)` frame is pushed back before the
/// successor is descended into, so a vertex's postorder emission follows
/// its whole subtree.
fn post_order(graph: &[Vec<usize>]) -> Vec<usize> {
    let mut visited = vec![false; graph.len()];
    let mut out = Vec::with_capacity(graph.len());

    for root in 0..graph.len() {
        if visited[root] {
            continue;
        }
        visited[root] = true;
        let mut stack: Vec<(usize, usize)> = vec![(root, 0)];
        while let Some((vertex, next)) = stack.pop() {
            if let Some(&successor) = graph[vertex].get(next) {
                stack.push((vertex, next + 1));
                if !visited[successor] {
                    visited[successor] = true;
                    stack.push((successor, 0));
                }
            } else {
                out.push(vertex);
            }
        }
    }

    out
}

// -----------------------------------------------------------------------------
// prune (Prune.hs:96-200)
// -----------------------------------------------------------------------------

/// `prune :: (PredicateId -> Bool) -> TypecheckedQuery -> Maybe
/// TypecheckedQuery` (Prune.hs:106).
///
/// `prune hasFacts (QueryWithInfo q _ gen t) = do renumberVars gen t <$>
/// pruneTcQuery q` (:107-108): the query is pruned, and if it survives it is
/// RENUMBERED — with the original generator and return type, which are not
/// pruned — before being re-wrapped. The old `qi_num_vars` is discarded
/// (`_`, :107) and replaced by renumberVars' count (:215).
fn prune(
    has_facts: &dyn Fn(&PredicateId) -> bool,
    query: TypecheckedQuery,
) -> Option<TypecheckedQuery> {
    let QueryWithInfo {
        qi_query,
        qi_num_vars: _, // Prune.hs:107's `_`
        qi_generator,
        qi_return_type,
    } = query;
    let query = prune_tc_query(has_facts, qi_query)?;
    Some(renumber_vars(qi_generator, qi_return_type, query))
}

/// `pruneTcQuery` (Prune.hs:110-118), a local `where`-binding in the source,
/// lifted to a private fn.
///
/// `TcQuery ty <$> prunePat keyPat <*> (case mvalPat of Nothing -> Just
/// Nothing; Just v -> Just <$> prunePat v) <*> pruneSequence stmts <*>
/// pure ord`: the return type and `ord` pass through untouched, a Nothing
/// value pattern stays a SUCCESSFUL Nothing, and the statement list is
/// all-or-nothing (`prune_sequence`).
fn prune_tc_query(
    has_facts: &dyn Fn(&PredicateId) -> bool,
    query: TcQuery,
) -> Option<TcQuery> {
    let TcQuery(ty, key, mval, stmts, ord) = query;
    let key = prune_pat(has_facts, key)?;
    let mval = match mval {
        None => None,
        Some(value) => Some(prune_pat(has_facts, value)?),
    };
    let stmts = prune_sequence(has_facts, stmts)?;
    Some(TcQuery(ty, key, mval, stmts, ord))
}

/// `pruneSequence = traverse pruneStmt` (Prune.hs:120-121) — the FIRST
/// doomed statement discards the whole sequence (the source comment's first
/// invariant, :102).
fn prune_sequence(
    has_facts: &dyn Fn(&PredicateId) -> bool,
    stmts: Vec<TcStatement>,
) -> Option<Vec<TcStatement>> {
    stmts
        .into_iter()
        .map(|stmt| prune_statement(has_facts, stmt))
        .collect()
}

/// `pruneStmt (TcStatement ty lhs rhs) = TcStatement ty <$> prunePat lhs
/// <*> prunePat rhs` (Prune.hs:123-125).
fn prune_statement(
    has_facts: &dyn Fn(&PredicateId) -> bool,
    stmt: TcStatement,
) -> Option<TcStatement> {
    let TcStatement(ty, lhs, rhs) = stmt;
    let lhs = prune_pat(has_facts, lhs)?;
    let rhs = prune_pat(has_facts, rhs)?;
    Some(TcStatement(ty, lhs, rhs))
}

/// `prunePat` (Prune.hs:127-200): the Term constructors. `Byte`/`Nat`/
/// `ByteArray`/`String` are leaves that always survive (:129-132); `Array`/
/// `Tuple` prune every element (:133-134); `Alt` prunes its payload (:135);
/// `Ref` hands to `prune_match` (:136).
fn prune_pat(has_facts: &dyn Fn(&PredicateId) -> bool, pat: TcPat) -> Option<TcPat> {
    match pat {
        Term::Byte(byte) => Some(Term::Byte(byte)),
        Term::Nat(nat) => Some(Term::Nat(nat)),
        Term::ByteArray(bytes) => Some(Term::ByteArray(bytes)),
        Term::String(string) => Some(Term::String(string)),
        Term::Array(elements) => Some(Term::Array(
            elements
                .into_iter()
                .map(|element| prune_pat(has_facts, element))
                .collect::<Option<Vec<_>>>()?,
        )),
        Term::Tuple(elements) => Some(Term::Tuple(
            elements
                .into_iter()
                .map(|element| prune_pat(has_facts, element))
                .collect::<Option<Vec<_>>>()?,
        )),
        Term::Alt(tag, element) => Some(Term::Alt(
            tag,
            Box::new(prune_pat(has_facts, *element)?),
        )),
        Term::Ref(m) => prune_match(has_facts, m),
    }
}

/// The `Ref m -> case m of ...` half of `prunePat` (Prune.hs:136-200). The
/// result is a whole TcPat (the source's `Maybe TcPat`), not a Match: the
/// TcIf/TcOr arms can replace the MatchExt with a MatchAnd or with the
/// else-branch pattern outright.
fn prune_match(
    has_facts: &dyn Fn(&PredicateId) -> bool,
    m: Match<Typed<TcTerm>, Var>,
) -> Option<TcPat> {
    match m {
        // MatchWild{} / MatchFid{} / MatchBind{} / MatchVar{} -> Just pat
        Match::MatchWild(ty) => Some(Term::Ref(Match::MatchWild(ty))),
        // MatchNever{} -> Nothing
        Match::MatchNever(_) => None,
        Match::MatchFid(fid) => Some(Term::Ref(Match::MatchFid(fid))),
        Match::MatchBind(var) => Some(Term::Ref(Match::MatchBind(var))),
        Match::MatchVar(var) => Some(Term::Ref(Match::MatchVar(var))),
        // MatchAnd a b -> Ref <$> (MatchAnd <$> prunePat a <*> prunePat b)
        Match::MatchAnd(a, b) => {
            let a = prune_pat(has_facts, *a)?;
            let b = prune_pat(has_facts, *b)?;
            Some(Term::Ref(Match::MatchAnd(Box::new(a), Box::new(b))))
        }
        // MatchPrefix s x -> Ref . MatchPrefix s <$> prunePat x
        Match::MatchPrefix(prefix, x) => {
            let x = prune_pat(has_facts, *x)?;
            Some(Term::Ref(Match::MatchPrefix(prefix, Box::new(x))))
        }
        // MatchArrayPrefix t xs all ->
        //   Ref <$> (MatchArrayPrefix t <$> traverse prunePat xs
        //                              <*> prunePat all)
        Match::MatchArrayPrefix(ty, prefix, all) => {
            let prefix = prefix
                .into_iter()
                .map(|element| prune_pat(has_facts, element))
                .collect::<Option<Vec<_>>>()?;
            let all = prune_pat(has_facts, *all)?;
            Some(Term::Ref(Match::MatchArrayPrefix(
                ty,
                prefix,
                Box::new(all),
            )))
        }
        // MatchExt (Typed ty tcterm) -> case tcterm of ...   (:146-200)
        Match::MatchExt(Typed(ty, term)) => prune_tc_term(has_facts, ty, term),
    }
}

/// The `case tcterm of` block of `prunePat` (Prune.hs:146-200).
///
/// Every ordinary arm has the shape `Ref . MatchExt . Typed ty <$> ...`; the
/// arms that do NOT are called out in place (TcIf :168-176, TcOr :178-185,
/// TcPromote/TcDemote :198-199). `TcNegation{}` returns the pattern
/// UNTOUCHED on purpose — the source's comment (:160-161): "we dont' want to
/// handle negation here because if it tries to match against things that are
/// not in the database it should succeed."
fn prune_tc_term(
    has_facts: &dyn Fn(&PredicateId) -> bool,
    ty: Type,
    term: TcTerm,
) -> Option<TcPat> {
    // Ref . MatchExt . Typed ty <$> ... — the shared wrapper.
    fn ext(ty: Type, term: TcTerm) -> TcPat {
        Term::Ref(Match::MatchExt(Typed(ty, term)))
    }

    match term {
        // TcFactGen (PidRef _ predId) _ _ _
        //   | not $ hasFacts predId -> Nothing                 (:147-149)
        TcTerm::TcFactGen(pidref, _, _, _) if !has_facts(&pidref.predicate_id) => None,
        // TcFactGen pidref k v range -> Ref . MatchExt . Typed ty
        //   <$> (TcFactGen pidref <$> prunePat k <*> prunePat v
        //                      <*> pure range)                 (:150-151)
        TcTerm::TcFactGen(pidref, k, v, range) => {
            let k = prune_pat(has_facts, *k)?;
            let v = prune_pat(has_facts, *v)?;
            Some(ext(
                ty,
                TcTerm::TcFactGen(pidref, Box::new(k), Box::new(v), range),
            ))
        }
        // TcElementsOfArray x -> Ref . MatchExt . Typed ty . TcElementsOfArray
        //   <$> prunePat x                                        (:152-153)
        TcTerm::TcElementsOfArray(x) => {
            let x = prune_pat(has_facts, *x)?;
            Some(ext(ty, TcTerm::TcElementsOfArray(Box::new(x))))
        }
        // TcElementsOfSet x -> ...                               (:154-155)
        TcTerm::TcElementsOfSet(x) => {
            let x = prune_pat(has_facts, *x)?;
            Some(ext(ty, TcTerm::TcElementsOfSet(Box::new(x))))
        }
        // TcElementsUnresolved ty' x -> ...                      (:156-157)
        TcTerm::TcElementsUnresolved(ty2, x) => {
            let x = prune_pat(has_facts, *x)?;
            Some(ext(
                ty,
                TcTerm::TcElementsUnresolved(ty2, Box::new(x)),
            ))
        }
        // TcQueryGen q -> Ref . MatchExt . Typed ty . TcQueryGen
        //   <$> pruneTcQuery q                                    (:158-159)
        TcTerm::TcQueryGen(q) => {
            let q = prune_tc_query(has_facts, *q)?;
            Some(ext(ty, TcTerm::TcQueryGen(Box::new(q))))
        }
        // TcAll query -> Ref . MatchExt . Typed ty . TcAll
        //   <$> pruneTcQuery query                                (:162-163)
        TcTerm::TcAll(query) => {
            let query = prune_tc_query(has_facts, *query)?;
            Some(ext(ty, TcTerm::TcAll(Box::new(query))))
        }
        // TcNegation{} -> Just pat   (:164, rationale :160-161)
        TcTerm::TcNegation(stmts) => Some(ext(ty, TcTerm::TcNegation(stmts))),
        // TcPrimCall op xs -> Ref . MatchExt . Typed ty . TcPrimCall op
        //   <$> traverse prunePat xs                              (:165-166)
        TcTerm::TcPrimCall(op, args) => {
            let args = args
                .into_iter()
                .map(|arg| prune_pat(has_facts, arg))
                .collect::<Option<Vec<_>>>()?;
            Some(ext(ty, TcTerm::TcPrimCall(op, args)))
        }
        // TcIf (Typed t' c) t e ->                               (:167-177)
        //   let cond = prunePat c; condt = Typed t' <$> cond
        //       then_ = prunePat t; else_ = prunePat e
        //   in asum [ Ref . MatchExt . Typed ty
        //               <$> (TcIf <$> condt <*> then_ <*> else_)
        //           , Ref <$> (MatchAnd <$> cond <*> then_)
        //           , else_ ]
        //
        // asum picks the first Just. A survives only when all three
        // sub-patterns survive; B needs cond and then_ only; C is the pruned
        // else-branch, Nothing included. The three-arm match below is that
        // order, and the values are moved rather than shared (TcPat: Clone,
        // but no clone the source does not do).
        TcTerm::TcIf {
            cond,
            then_,
            else_,
        } => {
            let Typed(cond_ty, cond) = *cond;
            // The three asum alternatives are tried in order, and the
            // source's laziness is part of the shape: A cannot be built
            // without cond AND then_ AND else_, so a failing cond or then_
            // reaches C without ever forcing the branch that follows it.
            // The early returns below reproduce that forcing order (the
            // only observable difference would be when an unreachable
            // TcStructPat panics).
            let cond = match prune_pat(has_facts, cond) {
                None => return prune_pat(has_facts, *else_),
                Some(cond) => cond,
            };
            let then_ = match prune_pat(has_facts, *then_) {
                None => return prune_pat(has_facts, *else_),
                Some(then_) => then_,
            };
            match prune_pat(has_facts, *else_) {
                Some(else_) => Some(ext(
                    ty,
                    TcTerm::TcIf {
                        cond: Box::new(Typed(cond_ty, cond)),
                        then_: Box::new(then_),
                        else_: Box::new(else_),
                    },
                )),
                // B: Ref <$> (MatchAnd <$> cond <*> then_)
                None => Some(Term::Ref(Match::MatchAnd(
                    Box::new(cond),
                    Box::new(then_),
                ))),
            }
        }
        // TcOr a b ->                                            (:178-186)
        //   let pa = prunePat a; pb = prunePat b
        //   in asum [ Ref . MatchExt . Typed ty <$> (TcOr <$> pa <*> pb)
        //           , pa, pb ]
        //
        // Either branch surviving alone replaces the disjunction with that
        // branch (the source comment's second invariant: the removed branch
        // binds nothing the survivor does not).
        TcTerm::TcOr(a, b) => {
            let pa = prune_pat(has_facts, *a);
            let pb = prune_pat(has_facts, *b);
            match (pa, pb) {
                (Some(a), Some(b)) => {
                    Some(ext(ty, TcTerm::TcOr(Box::new(a), Box::new(b))))
                }
                (pa, pb) => pa.or(pb),
            }
        }
        // TcDeref ty' p                                          (:187-191)
        //   | Angle.PredicateTy _ (PidRef _ predId) <- ty'
        //   , not $ hasFacts predId -> Nothing
        //   | otherwise -> Ref . MatchExt . Typed ty . TcDeref ty'
        //                    <$> prunePat p
        //
        // The guard reads the TYPE, not the term: dereferencing a predicate
        // that has no facts can never yield a key.
        TcTerm::TcDeref(ty_prime, p) => {
            let doomed = matches!(
                &ty_prime,
                Type_::PredicateTy(_, PidRef { predicate_id, .. })
                    if !has_facts(predicate_id)
            );
            if doomed {
                None
            } else {
                let p = prune_pat(has_facts, *p)?;
                Some(ext(ty, TcTerm::TcDeref(ty_prime, Box::new(p))))
            }
        }
        // TcFieldSelect (Typed ty' p) f ->                       (:192-194)
        //   do p' <- prunePat p
        //      return $ Ref $ MatchExt $ Typed ty $
        //        TcFieldSelect (Typed ty' p') f
        TcTerm::TcFieldSelect(typed, field) => {
            let Typed(ty_prime, p) = *typed;
            let p = prune_pat(has_facts, p)?;
            Some(ext(
                ty,
                TcTerm::TcFieldSelect(Box::new(Typed(ty_prime, p)), field),
            ))
        }
        // TcAltSelect (Typed ty' p) f -> ...                     (:195-197)
        TcTerm::TcAltSelect(typed, field) => {
            let Typed(ty_prime, p) = *typed;
            let p = prune_pat(has_facts, p)?;
            Some(ext(
                ty,
                TcTerm::TcAltSelect(Box::new(Typed(ty_prime, p)), field),
            ))
        }
        // TcPromote _ p -> prunePat p   (:198) — the promote WRAPPER is
        // dropped and the pruned inner pattern is returned as-is.
        TcTerm::TcPromote(_, p) => prune_pat(has_facts, *p),
        // TcDemote _ p -> prunePat p    (:199) — same.
        TcTerm::TcDemote(_, p) => prune_pat(has_facts, *p),
        // TcStructPat{} -> error "prune: TcStructPat"   (:200)
        //
        // RULE §2 partial-functions row: the panic carries the SOURCE's
        // message verbatim. Unreachable in practice (TcStructPat is resolved
        // away before this phase); it stays a panic, not a silent drop.
        TcTerm::TcStructPat(_) => panic!("prune: TcStructPat"),
    }
}

// -----------------------------------------------------------------------------
// renumberVars (Prune.hs:202-270)
// -----------------------------------------------------------------------------

/// The source's `data S = S { nextVar :: Int, mappings :: IntMap Int }`
/// (Prune.hs:204-207) and its `type R a = State S a` (:202), fused into one
/// explicit state value: `runState (renameQuery q) (S 0 mempty)` (:213) is
/// `Renumber { next_var: 0, mappings: empty }` plus a `&mut self` walk.
///
/// `mappings` is `IntMap Int` — keyed by the old var id, valued by the new
/// one. The pinned IntMap image in this crate is `HashMap<i64, _>` keyed by
/// the wrapped i64 (database/schema/types.rs:251-257, from code.rs:272 and
/// query/thrift/internal.rs:241); only lookup/insert happen here, so the
/// ordering choice is unobservable and HashMap is the sibling shape.
struct Renumber {
    next_var: i64,
    mappings: HashMap<i64, i64>,
}

impl Renumber {
    /// `renameQuery (TcQuery ty key mval stmts ord) = TcQuery ty <$>
    /// renamePat key <*> traverse renamePat mval <*> traverse renameStmt
    /// stmts <*> pure ord` (Prune.hs:218-223). The ORDER of the effects is
    /// the State applicative's left-to-right sequencing: key, then the
    /// optional value pattern, then the statements in list order. Variable
    /// numbering is first-visit order, so this order is observable.
    fn rename_query(&mut self, query: TcQuery) -> TcQuery {
        let TcQuery(ty, key, mval, stmts, ord) = query;
        let key = self.rename_pat(key);
        let mval = mval.map(|value| self.rename_pat(value));
        let stmts = stmts
            .into_iter()
            .map(|stmt| self.rename_statement(stmt))
            .collect();
        TcQuery(ty, key, mval, stmts, ord)
    }

    /// `renameStmt (TcStatement ty lhs rhs) = TcStatement ty <$> renamePat
    /// lhs <*> renamePat rhs` (Prune.hs:225-227) — lhs first.
    fn rename_statement(&mut self, stmt: TcStatement) -> TcStatement {
        let TcStatement(ty, lhs, rhs) = stmt;
        let lhs = self.rename_pat(lhs);
        let rhs = self.rename_pat(rhs);
        TcStatement(ty, lhs, rhs)
    }

    /// `renamePat = traverse (bitraverse renameTyped renameVar)`
    /// (Prune.hs:229-230). `Term::map` is the Functor image of that
    /// `traverse` over Term (term.rs:154): it applies the closure at every
    /// Ref, in the Term Traversable order (Array/Tuple elements in order,
    /// Alt's payload). `rename_match` then plays the bitraverse half at
    /// each Ref.
    fn rename_pat(&mut self, pat: TcPat) -> TcPat {
        pat.map(&mut |m| self.rename_match(m))
    }

    /// `bitraverse renameTyped renameVar` over Match, in the Match
    /// Traversable order: MatchAnd's both sides (a then b), MatchPrefix's
    /// tail, MatchArrayPrefix's prefix list then its whole-array term,
    /// MatchExt's Typed TcTerm, MatchVar/MatchBind's var. This is the order
    /// codegen/types.rs:519-546 documents for `bifold_map`'s visit (the
    /// same derived Traversable), and it is what fixes the numbering.
    fn rename_match(&mut self, m: Match<Typed<TcTerm>, Var>) -> Match<Typed<TcTerm>, Var> {
        match m {
            Match::MatchWild(ty) => Match::MatchWild(ty),
            Match::MatchNever(ty) => Match::MatchNever(ty),
            Match::MatchFid(fid) => Match::MatchFid(fid),
            Match::MatchBind(var) => Match::MatchBind(self.rename_var(var)),
            Match::MatchVar(var) => Match::MatchVar(self.rename_var(var)),
            Match::MatchAnd(a, b) => {
                let a = (*a).map(&mut |m| self.rename_match(m));
                let b = (*b).map(&mut |m| self.rename_match(m));
                Match::MatchAnd(Box::new(a), Box::new(b))
            }
            Match::MatchPrefix(prefix, x) => {
                let x = (*x).map(&mut |m| self.rename_match(m));
                Match::MatchPrefix(prefix, Box::new(x))
            }
            Match::MatchArrayPrefix(ty, prefix, all) => {
                let prefix = prefix
                    .into_iter()
                    .map(|element| element.map(&mut |m| self.rename_match(m)))
                    .collect();
                let all = (*all).map(&mut |m| self.rename_match(m));
                Match::MatchArrayPrefix(ty, prefix, Box::new(all))
            }
            // renameTyped (Typed ty term) = Typed ty <$> renameTcTerm term
            // (Prune.hs:232-233)
            Match::MatchExt(Typed(ty, term)) => {
                Match::MatchExt(Typed(ty, self.rename_tc_term(term)))
            }
        }
    }

    /// `renameTcTerm` (Prune.hs:235-259) — the 16 variants, each recursing
    /// in the source's field order. `TcNegation` traverses its statements
    /// (:245); `TcStructPat` traverses its field list in order (:258-259).
    fn rename_tc_term(&mut self, term: TcTerm) -> TcTerm {
        match term {
            TcTerm::TcOr(a, b) => {
                let a = self.rename_pat(*a);
                let b = self.rename_pat(*b);
                TcTerm::TcOr(Box::new(a), Box::new(b))
            }
            TcTerm::TcFactGen(pidref, k, v, range) => {
                let k = self.rename_pat(*k);
                let v = self.rename_pat(*v);
                TcTerm::TcFactGen(pidref, Box::new(k), Box::new(v), range)
            }
            TcTerm::TcElementsOfArray(x) => {
                TcTerm::TcElementsOfArray(Box::new(self.rename_pat(*x)))
            }
            TcTerm::TcElementsOfSet(x) => {
                TcTerm::TcElementsOfSet(Box::new(self.rename_pat(*x)))
            }
            TcTerm::TcElementsUnresolved(ty, x) => {
                TcTerm::TcElementsUnresolved(ty, Box::new(self.rename_pat(*x)))
            }
            TcTerm::TcQueryGen(query) => TcTerm::TcQueryGen(Box::new(self.rename_query(*query))),
            TcTerm::TcAll(query) => TcTerm::TcAll(Box::new(self.rename_query(*query))),
            TcTerm::TcNegation(stmts) => TcTerm::TcNegation(
                stmts
                    .into_iter()
                    .map(|stmt| self.rename_statement(stmt))
                    .collect(),
            ),
            TcTerm::TcPrimCall(op, args) => TcTerm::TcPrimCall(
                op,
                args.into_iter()
                    .map(|arg| self.rename_pat(arg))
                    .collect(),
            ),
            // TcIf cond then_ else_ ->
            //   TcIf <$> traverse renamePat cond <*> renamePat then_
            //        <*> renamePat else_            (Prune.hs:247-248)
            TcTerm::TcIf {
                cond,
                then_,
                else_,
            } => {
                let Typed(cond_ty, cond) = *cond;
                let cond = self.rename_pat(cond);
                let then_ = self.rename_pat(*then_);
                let else_ = self.rename_pat(*else_);
                TcTerm::TcIf {
                    cond: Box::new(Typed(cond_ty, cond)),
                    then_: Box::new(then_),
                    else_: Box::new(else_),
                }
            }
            TcTerm::TcDeref(ty, p) => TcTerm::TcDeref(ty, Box::new(self.rename_pat(*p))),
            TcTerm::TcFieldSelect(typed, field) => {
                let Typed(ty, p) = *typed;
                let p = self.rename_pat(p);
                TcTerm::TcFieldSelect(Box::new(Typed(ty, p)), field)
            }
            TcTerm::TcAltSelect(typed, field) => {
                let Typed(ty, p) = *typed;
                let p = self.rename_pat(p);
                TcTerm::TcAltSelect(Box::new(Typed(ty, p)), field)
            }
            TcTerm::TcPromote(ty, p) => TcTerm::TcPromote(ty, Box::new(self.rename_pat(*p))),
            TcTerm::TcDemote(ty, p) => TcTerm::TcDemote(ty, Box::new(self.rename_pat(*p))),
            TcTerm::TcStructPat(fields) => TcTerm::TcStructPat(
                fields
                    .into_iter()
                    .map(|(name, pat)| (name, self.rename_pat(pat)))
                    .collect(),
            ),
        }
    }

    /// `renameVar (Var ty old n) = State.state $ \s -> case IntMap.lookup old
    /// (mappings s) of Just new -> (Var ty new n, s); Nothing -> let new =
    /// nextVar s ... in (Var ty new n, S (new+1) (insert old new ...))`
    /// (Prune.hs:261-270).
    ///
    /// A seen id maps to its first-assigned number; an unseen one takes the
    /// next number and bumps the counter. `var_type` and `var_orig_name`
    /// travel unchanged (:264/:270).
    fn rename_var(&mut self, var: Var) -> Var {
        let Var {
            var_type,
            var_id,
            var_orig_name,
        } = var;
        match self.mappings.get(&var_id).copied() {
            Some(new) => Var {
                var_type,
                var_id: new,
                var_orig_name,
            },
            None => {
                let new = self.next_var;
                self.mappings.insert(var_id, new);
                self.next_var = new + 1;
                Var {
                    var_type,
                    var_id: new,
                    var_orig_name,
                }
            }
        }
    }
}

/// `renumberVars :: Maybe Generator -> Type -> TcQuery -> TypecheckedQuery`
/// (Prune.hs:211-215):
///   `let (newQuery, S varCount _) = runState (renameQuery q) (S 0 mempty)
///    in QueryWithInfo newQuery varCount gen ty`
///
/// The generator and return type are the ORIGINAL ones (they were never
/// pruned); `qi_num_vars` becomes the final counter — the number of DISTINCT
/// var ids seen, which is the "maximum index + 1" the field documents
/// (codegen/types.rs:151) because ids are handed out densely from 0.
fn renumber_vars(gen: Option<Generator>, ty: Type, query: TcQuery) -> TypecheckedQuery {
    let mut state = Renumber {
        next_var: 0,
        mappings: HashMap::new(),
    };
    let query = state.rename_query(query);
    QueryWithInfo {
        qi_query: query,
        qi_num_vars: state.next_var,
        qi_generator: gen,
        qi_return_type: ty,
    }
}

// -----------------------------------------------------------------------------
// EXPORT PARITY (Prune.hs:9 — `module Glean.Query.Prune (pruneDerivations)
// where`)
// -----------------------------------------------------------------------------
//   pruneDerivations -> pub fn prune_derivations      OK
//
// Everything else private, as in the source. The source's top-level names:
// pruneQueries (:60), topologicalSort (:80), prune (:106), R (:202), S
// (:204), renumberVars (:211) — all `fn`/`struct` without `pub` here. The
// source's `where`-bindings, lifted to private fns in this file:
// updateDerivation (:43, inlined into prune_derivations' stage 3), add
// (:67) and hasFacts (:75) (inlined into prune_queries' loop), pruneTcQuery
// (:110) -> prune_tc_query, pruneSequence (:120) -> prune_sequence,
// pruneStmt (:123) -> prune_statement, prunePat (:127) -> prune_pat +
// prune_match + prune_tc_term, renameQuery (:218) -> Renumber::rename_query,
// renameStmt (:225) -> Renumber::rename_statement, renamePat (:229) ->
// Renumber::rename_pat + Renumber::rename_match, renameTyped (:232) ->
// the MatchExt arm of rename_match, renameTcTerm (:235) ->
// Renumber::rename_tc_term, renameVar (:261) -> Renumber::rename_var, and
// the two Graph projections (:84-87) -> topological_sort's entry vector.
// `ext` is a local helper inside prune_tc_term (the shared
// `Ref . MatchExt . Typed ty` wrapper), not a source name.
//
// No `pub use` re-exports: the source has none, and the module is a facade
// for one function.

// -----------------------------------------------------------------------------
// Marker breakdown (closed set)
// -----------------------------------------------------------------------------
// Port todos: 0 — no deferred work: the ordering is reproduced exactly
//   (ORDERING, probe-backed), the knot is flattened (THE LAZY KNOT), and the
//   bitraverse debt row 88 handed over is paid by Renumber (SIBLING-UNIT
//   CONTRACTS).
// Reproduced defects: 0 — the file contains no source defect to reproduce.
//   The one `error` (:200) is a partial function, not a defect, and panics
//   with the source's message.
// Performance deferrals: 0 — the port is O(n log n + edges) like the source;
//   no clone where the source shares (queries MOVE; see THE LAZY KNOT).
// Safety notes: 0 — no unsafe code.
// Inventory bug candidates: 0 — the inventory has no row for this file
//   (INVENTORY), so no divergence flag can exist.
//
// PORT STATUS: confidence=high todos=0
