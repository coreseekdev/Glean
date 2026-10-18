// Ported from: glean/angle/Glean/Schema/Resolve.hs (824 lines)
// Manifest unit: rust/glean-angle/src/schema/resolve.rs (crate glean-angle) —
// migration/manifest.tsv row 8. RULE §4 (naming): one source file → one
// target file; the manifest path is authoritative.
//
// INVENTORY (RULE §5): no row in migration/inventory.tsv names THIS unit
// (grep "Schema/Resolve": the single hit is row 6's evidence column, which
// cites Resolve.hs:102 — `hashByteString frag` — as one consumer on the
// ComputeIds/schema-ID hash chain owned by the row-6 encoder module). This
// unit's hash_byte_string call is exactly that site, ported through the
// DONE sibling angle/hash.rs (the chain owner's own function), used here
// ONLY as an in-memory parse-cache key (the fragment hash is never
// persisted and never feeds a schema ID), so no byte-format obligation
// originates in this file and there is no target_translation to diverge
// from. No INVENTORY-BUG-CANDIDATE.
//
// SIBLING CONTRACTS (RULE §4 assumed-imports; verified by reading the done
// siblings, never re-declared here):
//   - crate::angle::types owns the whole AST (SourcePat_/SourceQuery_/
//     Type_/FieldDef_/TypeDef_/PredicateDef_/DerivingDef_/DerivingInfo/
//     PrimOp/SeekSection/Field/NonEmpty, the Source* alias family,
//     AngleVersion, SrcSpan, Name, source_pat_span, Describe,
//     latest_supported_angle_version) and the glean_thrift re-exports
//     PredicateRef/TypeRef/Version. This file only `use`s them.
//   - crate::angle::hash owns Hash (= Fingerprint) and hash_byte_string.
//   - crate::display owns Doc, vcat, display_default (the port's single
//     pretty-printing surface: `pretty x` in the source = Display here).
//   - crate::schema::util owns ShowRef (show_ref), parse_ref, split_dot.
//   - crate::schema::types owns SourceRef-consuming machinery: NameEnv,
//     RefTarget/RefResolved, LookupResult, resolve_ref,
//     lookup_result_to_either, the ResolvedSchema/ResolvedSchemaRef/
//     ResolvedSchemas family and schema_ref.
//   - crate::schema::evolve owns validate_resolved_evolutions.
//
// ASSUMED-IMPORT (RULE §4 B4 stub policy): Glean.Angle.Parser — the
// Alex/Happy-generated parser (glean/angle/Glean/Angle/Lexer.x +
// Parser.y; a generated unit, not among the manifest's hand-written
// units). Owner: the future Parser.y/Lexer.x port unit (hand-written
// lexer+Pratt parser per RULE §2's Alex/Happy row). The import below
// assumes that module at crate::angle::parser with these signatures,
// pinned from Parser.y's export list (:7-9) and definitions (:435-453):
//     fn strip_angle_version(bs: &[u8]) -> (AngleVersion, Vec<u8>)
//         // stripAngleVersion: B.dropWhile isSpace BEFORE the
//         // "version: " prefix test, then readInt on the rest, else
//         // (latestAngleVersion, bs) (Parser.y:439-445)
//     fn parse_schema(bs: &[u8]) -> Result<SourceSchemas, String>
//         // parseSchema bs = parseSchemaWithVersion ver rest
//     fn parse_schema_with_version(
//         ver: AngleVersion, bs: &[u8]) -> Result<SourceSchemas, String>
// The AST it produces is crate::angle::types' (never re-declared here).
// TODO(port): parser integration — crate::angle::parser does not exist
// yet (owner: the Lexer.x/Parser.y port unit); this `use` is the draft
// stub and the file cannot compile until that unit lands.
//
// Naming (RULE §0 structure-preserving + sibling conventions):
//   - The source's `Resolve p t a = ReaderT (AngleVersion, NameEnv
//     (RefTarget p t)) (Except Text) a` becomes a borrowed context
//     struct ResolveCtx + Result<_, String> (RULE §1 strings row: the
//     Except Text payload is String). run_resolve mirrors
//     `runResolve ver scope act = runReaderT act (ver,scope)`.
//   - The Haskell's (ShowRef p, ShowRef t) generics on resolveType/
//     resolveQuery/resolvePat/... have TWO instantiation points in the
//     source tree, not one: (a) this module's resolveOneSchema (scope ::
//     NameEnv RefResolved = RefTarget PredicateRef TypeRef); (b) the
//     glean-db UserQuery unit — glean/db/Glean/Query/UserQuery.hs:976
//     (`runResolve latestAngleVersion scope $ resolveQuery parsed`) and
//     :1290 (`runResolve latestAngleVersion scope $ resolveType parsed`),
//     scope :: NameEnv RefTargetId = NameEnv (RefTarget PredicateId
//     TypeId) via schemaNameEnv (db/Glean/Database/Schema/Types.hs:194) +
//     addTmpPredicate, with `type RefTargetId = RefTarget PredicateId
//     TypeId` at db/Glean/Database/Schema/ComputeIds.hs:49. Instantiation
//     (b) exercises ShowRef PredicateId / ShowRef TypeId (both already
//     impl'd in sibling util.rs) in the Ambiguous error text through
//     lookupResultToEither. The Rust ports below are therefore generic
//     over P: Clone + ShowRef, T: Clone + ShowRef — the same shape as
//     the already-generic sibling lookup_result_to_either — and this
//     unit itself instantiates them only at RefResolved
//     (resolve_one_schema1's run_resolve calls), so its own behavior is
//     identical to a monomorphic port.
//   - HashMap.union is LEFT-biased; HashMap.unionWith f applies f to
//     (left-value, right-value); fromListWith f combines as
//     f newValue oldValue and fromList is last-wins. The union/insert
//     helpers below preserve each bias exactly. (fromListWith's
//     argument order was verified against this tree's GHC 9.4.7:
//     Map.fromListWith (++) [(1,"a"),(1,"b"),(1,"c")] == [(1,"cba")].)

use std::collections::{BTreeSet, HashMap, HashSet};

// RULE §4 (imports): `use crate::…` within the crate (the rulebook's
// literal `crate::glean::…` template predates the manifest path layout —
// same note as the sibling units).
use crate::angle::hash::{hash_byte_string, Hash};
// ASSUMED-IMPORT (RULE §4 B4): see the header block.
use crate::angle::parser::{parse_schema, parse_schema_with_version, strip_angle_version};
use crate::angle::types::{
    latest_supported_angle_version, source_pat_span, AngleVersion, Describe, DerivingDef_,
    DerivingInfo, Field, FieldDef_, Name, NonEmpty, PredicateDef_, PrimOp, SeekSection,
    SourceDecl_, SourceEvolves_, SourcePat_, SourcePredicateDef, SourceQuery_, SourceSchema,
    SourceSchema_, SourceSchemas, SourceSchemas_, SourceStatement_, SourceType, SourceTypeDef,
    SrcSpan, Type_, TypeDef_, Version,
};
// thrift types re-exported by the sibling types.rs (RULE §0: the source
// reached them through its Glean.Angle.Types import).
use crate::angle::types::{PredicateRef, SourceRef, TypeRef};
use crate::display::{display_default, vcat, Doc};
use crate::schema::evolve::validate_resolved_evolutions;
use crate::schema::types::{
    lookup_result_to_either, resolve_ref, schema_ref, LookupResult, NameEnv, RefResolved,
    RefTarget, ResolvedDerivingDef, ResolvedPredicateDef, ResolvedSchema, ResolvedSchemaRef,
    ResolvedSchemas, ResolvedTypeDef,
};
use crate::schema::util::{parse_ref, split_dot, ShowRef};

// `type SchemaParserCache = HashMap Hash SourceSchemas` (Resolve.hs:49).
// Hash-keyed; iteration order is never observed in the source (lookups
// only).
//
// TODO(port): this HashMap needs `Fingerprint: std::hash::Hash`, but the
// sibling hash.rs deliberately exposes hash_fingerprint() as a FUNCTION
// (the hashable-1.4.x chain port) and defines no `impl std::hash::Hash
// for Fingerprint`. Who owns that impl is a cross-unit question: hash.rs
// (beside its hashable-chain TODO) or a newtype wrapper here — NOT
// decided or implemented in this unit (sibling ownership; the choice
// belongs to the hash-chain owner and the Step-4 wiring survey).
pub type SchemaParserCache = HashMap<Hash, SourceSchemas>;

// ---------------------------------------------------------------------------
// High-level schema parsing / resolution APIs
// ---------------------------------------------------------------------------

/// Useful packaging of 'parseSchema' and 'resolveSchema'. Note that
/// parsing and resolution of a schema is a pure function. (Source doc,
/// Resolve.hs:54-58.)
///
/// `parseAndResolveSchema :: ByteString -> Either String (SourceSchemas,
/// ResolvedSchemas)` (Resolve.hs:58-66). RULE §1 strings/bytes row:
/// ByteString → &[u8] (the source shares it; parsing only reads).
pub fn parse_and_resolve_schema(str_: &[u8]) -> Result<(SourceSchemas, ResolvedSchemas), String> {
    match parse_schema(str_) {
        Err(err) => Err(err),
        Ok(ss) => match resolve_schema(&ss) {
            // Left txt -> Left (Text.unpack txt) — identity under the
            // Text→String mapping.
            Err(txt) => Err(txt),
            Ok(r) => Ok((ss, r)),
        },
    }
}

/// `breakSubstring` — the one ByteString primitive the source uses
/// (Resolve.hs:99): (prefix, suffix-including-needle); (whole, empty)
/// when absent. Needle is the constant b"\nschema" (never empty, so the
/// empty-needle special case is unreachable).
fn break_substring<'a>(needle: &[u8], hay: &'a [u8]) -> (&'a [u8], &'a [u8]) {
    match hay.windows(needle.len()).position(|w| w == needle) {
        Some(i) => (&hay[..i], &hay[i..]),
        None => (hay, &[]),
    }
}

/// `cutSchemas` (Resolve.hs:94-98): split at "\nschema" boundaries,
/// putting the "\n" back on the end of each fragment. Recursion made
/// iterative; fragments owned (each is hashed and handed to the parser).
fn cut_schemas(b: &[u8]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut b = b;
    loop {
        let (h, t) = break_substring(b"\nschema", b);
        let mut frag = h.to_vec();
        frag.push(b'\n'); // put the "\n" back on the end
        out.push(frag);
        if t.is_empty() {
            break; // ByteString.null t
        }
        b = &t[1..]; // drop the "\n"
    }
    out
}

/// Like parseAndResolveSchema but uses a cache to avoid repeatedly
/// parsing the same schema fragments. (Source doc + caveats,
/// Resolve.hs:68-85: FILE annotations are lost, so don't use this when
/// you want error messages; "schema" should appear in column 0 or we
/// merely miss some caching.)
///
/// `parseAndResolveSchemaCached :: SchemaParserCache -> ByteString ->
/// Either String (SourceSchemas, ResolvedSchemas, SchemaParserCache)`
/// (Resolve.hs:86-125). The cache map moves in (the Haskell shares one
/// immutable map and returns the grown one; the Rust clone-per-entry is
/// the RULE §0 stand-in for that sharing).
pub fn parse_and_resolve_schema_cached(
    cache: SchemaParserCache,
    str_: &[u8],
) -> Result<(SourceSchemas, ResolvedSchemas, SchemaParserCache), String> {
    let (ver, rest) = strip_angle_version(str_);

    // parsed <- forM schemaFragments $ \(key, str) -> case HashMap.lookup
    // key cache of Nothing -> parseSchemaWithVersion ver str; Just cached
    // -> return (key, cached)   (Resolve.hs:105-110)
    //
    // schemaFragments = [ (hashByteString frag, frag) | frag <- cutSchemas
    // rest ] — Resolve.hs:101-103. This is the site inventory.tsv row 6
    // cites (Resolve.hs:102); hash_byte_string is the sibling hash-chain
    // owner's function, used here as a pure cache key only.
    let parsed: Vec<(Hash, SourceSchemas)> = cut_schemas(&rest)
        .into_iter()
        .map(|frag| {
            let key = hash_byte_string(&frag);
            match cache.get(&key) {
                None => parse_schema_with_version(ver, &frag).map(|ss| (key, ss)),
                Some(cached) => Ok((key, cached.clone())),
            }
        })
        .collect::<Result<_, String>>()?;

    // merged = SourceSchemas { srcAngleVersion = ver, schemas =
    // concatMap (srcSchemas . snd) parsed, evolves = concatMap (srcEvolves
    // . snd) parsed } — fragments in cut order.
    let merged = SourceSchemas_ {
        src_angle_version: ver,
        src_schemas: parsed
            .iter()
            .flat_map(|(_, ss)| ss.src_schemas.iter().cloned())
            .collect(),
        src_evolves: parsed
            .iter()
            .flat_map(|(_, ss)| ss.src_evolves.iter().cloned())
            .collect(),
    };

    // newCache = HashMap.union cache (HashMap.fromList parsed) — LEFT-biased:
    // a pre-existing cache entry survives a re-insert (the values would be
    // equal anyway: same fragment bytes ⇒ same hash ⇒ same parse).
    let mut new_cache = cache;
    for (key, ss) in &parsed {
        new_cache.entry(key.clone()).or_insert_with(|| ss.clone());
    }

    match resolve_schema(&merged) {
        Err(txt) => Err(txt),
        Ok(r) => Ok((merged, r, new_cache)),
    }
}

// ---------------------------------------------------------------------------
// resolveSchema
// ---------------------------------------------------------------------------

/// Turn 'SourceSchemas' into a 'ResolvedSchemas' by resolving all the
/// references and checking for validity. (Source doc, Resolve.hs:127-131.)
///
/// `resolveSchema :: SourceSchemas -> Either Text ResolvedSchemas`
/// (Resolve.hs:131-190). By-ref input (the Haskell shares it and callers
/// like Schema.hs keep the SourceSchemas); the resolved copies are clones
/// (RULE §0 stand-in).
pub fn resolve_schema(schemas: &SourceSchemas) -> Result<ResolvedSchemas, String> {
    // checkAngleVersion (srcAngleVersion schemas) — FIRST, before any ref
    // resolution (source order; error precedence).
    check_angle_version(schemas.src_angle_version)?;

    // SourceSchemas{..} <- resolveSchemaRefs schemas
    let SourceSchemas_ {
        src_angle_version,
        src_schemas,
        src_evolves,
    } = resolve_schema_refs(schemas)?;

    // evolves = HashMap.fromListWith (++) [ (new, [old]) | SourceEvolves _
    // new old <- srcEvolves ] — fromListWith combines as f new old, i.e.
    // the LATER occurrence PREPENDS (verified against the pinned GHC:
    // fromListWith (++) [(1,"a"),(1,"b"),(1,"c")] == [(1,"cba")]) — hence
    // insert at 0.
    let mut evolves: HashMap<SourceRef, Vec<SourceRef>> = HashMap::new();
    for e in &src_evolves {
        evolves
            .entry(e.evolves_new.clone())
            .or_insert_with(Vec::new)
            .insert(0, e.evolves_old.clone());
    }
    // evolvesOf name = HashMap.lookupDefault [] name evolves
    let evolves_of = |name: &SourceRef| -> Vec<SourceRef> {
        evolves.get(name).cloned().unwrap_or_default()
    };

    // dependency analysis: we want to process schemas in dependency
    // order, and detect cycles in evolves declarations. (Source comment.)
    //
    // sccs = stronglyConnComp [ (schema, schemaName schema, out schema)
    //                          | schema <- srcSchemas ]
    // out s = schemaDependencies s <> evolvesOf (schemaName s)
    // schemaDependencies SourceSchema{..} = schemaInherits ++
    //   [ name | SourceImport name _ <- schemaDecls ]
    let edges: Vec<(SourceSchema, SourceRef, Vec<SourceRef>)> = src_schemas
        .iter()
        .map(|schema| {
            let mut out: Vec<SourceRef> = schema.schema_inherits.clone();
            out.extend(
                schema
                    .schema_decls
                    .iter()
                    .filter_map(|decl| match decl {
                        SourceDecl_::SourceImport(name, _) => Some(name.clone()),
                        _ => None,
                    }),
            );
            out.extend(evolves_of(&schema.schema_name));
            (schema.clone(), schema.schema_name.clone(), out)
        })
        .collect();

    let sccs = strongly_conn_comp(edges);

    // Resolve all the references in each individual schema (source
    // comment) — resolveSchemas: fold over the SCC list in order; each
    // AcyclicSCC is resolved against the env accumulated so far and
    // inserted; a CyclicSCC aborts the fold with the cycle error
    // (short-circuit on the FIRST CyclicSCC, exactly as throwError
    // inside the fold does).
    let mut final_env: Environment = HashMap::new();
    for scc in &sccs {
        match scc {
            Scc::Acyclic(one) => {
                let schema_evolves = evolves_of(&one.schema_name);
                let resolved =
                    resolve_one_schema(&final_env, src_angle_version, &schema_evolves, one.clone())?;
                final_env.insert(one.schema_name.clone(), resolved);
            }
            Scc::Cyclic(some) => {
                // "cycle in schema definitions between: " <> intercalate
                // ", " (map (showRef . schemaName) some)
                return Err(format!(
                    "cycle in schema definitions between: {}",
                    some.iter()
                        .map(|s| s.schema_name.show_ref())
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
        }
    }

    // resolved = [ schema | AcyclicSCC one <- sccs, Just schema <-
    // [HashMap.lookup (schemaName one) finalEnv] ] — re-walks the SCC
    // list (NOT the HashMap), so schemasResolved keeps SCC (dependency)
    // order; the `schemasResolved` field doc "Resolved schemas in
    // dependency order" (Schema/Types.hs:189) rides on this.
    let resolved: Vec<ResolvedSchemaRef> = sccs
        .iter()
        .filter_map(|scc| match scc {
            Scc::Acyclic(one) => final_env.get(&one.schema_name).cloned(),
            Scc::Cyclic(_) => None,
        })
        .collect();

    // allSchemas = [ schema | schema@ResolvedSchema{..} <-
    // HashMap.elems finalEnv, resolvedSchemaName == "all" ]
    let all_schemas: Vec<&ResolvedSchemaRef> = final_env
        .values()
        .filter(|schema| schema.resolved_schema_name == "all")
        .collect();

    // when (srcAngleVersion >= AngleVersion 6) $ liftEither $
    // validateResolvedEvolutions resolved
    if src_angle_version >= AngleVersion(6) {
        validate_resolved_evolutions(&resolved)?;
    }

    Ok(ResolvedSchemas {
        schemas_highest_version: if all_schemas.is_empty() {
            None
        } else {
            // maximum $ map resolvedSchemaVersion allSchemas — Ord on the
            // thrift Version (i32).
            Some(
                all_schemas
                    .iter()
                    .map(|schema| schema.resolved_schema_version)
                    .max()
                    .expect("nonempty allSchemas"),
            )
        },
        schemas_resolved: resolved,
    })
}

// `data SCC vertex = AcyclicSCC vertex | CyclicSCC [vertex]`
// (containers-0.6.7 Data.Graph). Private: only resolve_schema consumes it.
enum Scc<Node> {
    Acyclic(Node),
    Cyclic(Vec<Node>),
}

// stronglyConnComp — the containers-0.6.7 Data.Graph algorithm
// (Resolve.hs:140 uses it via `import Data.Graph`), ported here for the
// general multi-out-edge graph. SPEC, pinned from the hyperlinked source
// in this machine's GHC install (share/doc/ghc-9.4.7/html/libraries/
// containers-0.6.7/src/Data.Graph.html — the same pin the sibling
// evolve.rs states for its out-degree<=1 specialization `cycles`; this
// is the general form, kept private here, a duplicate by the draft norm
// like show_doc below):
//   - stronglyConnCompR [] = [] (empty-input special case).
//   - graphFromEdges numbers vertices by ASCENDING KEY: sortBy
//     (comparing key) over the edge triples (stable). Duplicate keys ARE
//     reachable at this call site: duplicate schema decls pass
//     resolve_schema_refs (nothing there rejects them) and reach the SCC
//     analysis; only the HashSet/HashMap collapses LATER in
//     resolve_schema (ref_map's singleton sets, final_env's name keys)
//     hide the duplication. The ORIGINAL graph is built with plain
//     `array`, NOT buildG: g!v = mapMaybe key_vertex ks — the out-list
//     order is preserved, unknown keys dropped, duplicates kept.
//   - transposeG = buildG (reverseE g): reverseE enumerates edges g — v
//     ASCENDING, w in g!v order — and buildG's accumArray (flip (:))
//     PREPENDS, so each transpose adjacency lists its in-neighbors in
//     DESCENDING v order.
//   - scc g = dfs g (reverse (postOrd (transposeG g))) — two-pass
//     Kosaraju. Pass 1 (postOrd/dff): DFS the transpose, roots 0..n-1
//     ascending, successors in adjacency order, a vertex emitted AFTER
//     its subtree; a vertex is marked before its successors are scanned.
//     Pass 2: DFS the ORIGINAL graph from roots in reverse postorder;
//     each tree is one SCC.
//   - decode (stronglyConnCompR): a singleton tree Node v [] is
//     CyclicSCC [v] iff mentions_itself v (v elem g!v), else
//     AcyclicSCC v; any tree with children is CyclicSCC of its dec
//     preorder — dec (Node v ts) vs = v : foldr dec vs ts — which is the
//     DFS visit (mark) order of the tree.
// Verified against the containers doc example (Data.Graph.html):
//   stronglyConnComp [("a",0,[1]),("b",1,[2,3]),("c",2,[1]),("d",3,[3])]
//     == [CyclicSCC ["d"],CyclicSCC ["b","c"],AcyclicSCC "a"]
// (hand-walked through this port: transpose in-lists t(1)=[2,0] etc.
// give postOrd [0,2,1,3], reversed roots [3,1,2,0], trees [d],[b,c],[a]).
fn strongly_conn_comp<Node: Clone, Key: Ord + std::hash::Hash>(
    edges: Vec<(Node, Key, Vec<Key>)>,
) -> Vec<Scc<Node>> {
    if edges.is_empty() {
        return Vec::new(); // stronglyConnCompR [] = []
    }
    // sorted_edges = L.sortBy (comparing key) — stable; vertex v ↔
    // sorted[v], keys ascending.
    let mut sorted = edges;
    sorted.sort_by(|a, b| a.1.cmp(&b.1));
    let n = sorted.len();
    // key_vertex. Duplicate keys are reachable (see the spec block
    // above), so the mapping must be deterministic. Rust's
    // binary_search_by leaves the choice among equal keys
    // implementation-defined, so the port pins an explicit LAST-wins
    // map in one pass over `sorted` — matching the verified sibling
    // evolve.rs port (its vertex_of HashMap collect is last-wins over
    // the ascending keys) and Data.Map fromList semantics on the sorted
    // triples. (The pinned containers-0.6.7 itself resolves key_vertex
    // by a midpoint binary search over the sorted key array; on a
    // duplicate run its result is deterministic but
    // midpoint-dependent — last-wins is the port's pinned choice, per
    // review finding A2.)
    let mut key_vertex: HashMap<&Key, usize> = HashMap::new();
    for (v, triple) in sorted.iter().enumerate() {
        key_vertex.insert(&triple.1, v); // later (higher) v wins
    }

    // The original graph's adjacency: g!v = out-list order, unknown keys
    // dropped (mapMaybe key_vertex), duplicates preserved.
    let g: Vec<Vec<usize>> = sorted
        .iter()
        .map(|(_, _, ks)| {
            ks.iter()
                .filter_map(|k| key_vertex.get(k).copied())
                .collect()
        })
        .collect();

    // transposeG: in-neighbor lists, DESCENDING v (append v ascending,
    // then reverse — the pin above).
    let mut transpose: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (v, succs) in g.iter().enumerate() {
        for &w in succs {
            transpose[w].push(v);
        }
    }
    for adj in &mut transpose {
        adj.reverse();
    }

    // Pass 1 — postOrd (transposeG g): the same explicit (vertex,
    // next-successor-index) stack the sibling evolve.rs uses; it
    // reproduces Data.Graph's recursive `go` (mark before scanning
    // successors; emit after the subtree; re-check visited on resume).
    let mut visited = vec![false; n];
    let mut postorder: Vec<usize> = Vec::with_capacity(n);
    for root in 0..n {
        if visited[root] {
            continue;
        }
        visited[root] = true;
        let mut stack: Vec<(usize, usize)> = vec![(root, 0)];
        while let Some(frame) = stack.last_mut() {
            let v = frame.0;
            let adj = &transpose[v];
            let mut i = frame.1;
            while i < adj.len() && visited[adj[i]] {
                i += 1;
            }
            frame.1 = i;
            if i < adj.len() {
                let s = adj[i];
                visited[s] = true;
                stack.push((s, 0));
            } else {
                postorder.push(v);
                stack.pop();
            }
        }
    }

    // Pass 2 — dfs g (reverse postOrd …): roots last-finished-first; the
    // preorder (mark order) of each tree IS decode's dec order; a
    // singleton tree is the tree that visited no other vertex.
    let mut visited2 = vec![false; n];
    let mut result: Vec<Scc<Node>> = Vec::new();
    for &root in postorder.iter().rev() {
        if visited2[root] {
            continue;
        }
        let mut preorder: Vec<usize> = Vec::new();
        visited2[root] = true;
        preorder.push(root);
        let mut stack: Vec<(usize, usize)> = vec![(root, 0)];
        while let Some(frame) = stack.last_mut() {
            let v = frame.0;
            let adj = &g[v];
            let mut i = frame.1;
            while i < adj.len() && visited2[adj[i]] {
                i += 1;
            }
            frame.1 = i;
            if i < adj.len() {
                let s = adj[i];
                visited2[s] = true;
                preorder.push(s);
                stack.push((s, 0));
            } else {
                stack.pop();
            }
        }
        // decode (Node v []) | mentions_itself v = CyclicSCC [v]
        //                | otherwise      = AcyclicSCC v
        // decode other                    = CyclicSCC (dec preorder other)
        if let [v] = preorder.as_slice() {
            let v = *v;
            if g[v].contains(&v) {
                result.push(Scc::Cyclic(vec![sorted[v].0.clone()]));
            } else {
                result.push(Scc::Acyclic(sorted[v].0.clone()));
            }
        } else {
            result.push(Scc::Cyclic(
                preorder.iter().map(|&v| sorted[v].0.clone()).collect(),
            ));
        }
    }
    result
}

// ---------------------------------------------------------------------------
// resolveSchemaRefs
// ---------------------------------------------------------------------------

/// `resolveSchemaRefs :: SourceSchemas -> Except Text SourceSchemas`
/// (Resolve.hs:193-249; exported): resolve schema names (unqualified →
/// qualified, versionless → versioned) in inherits/imports/evolves
/// against the set of declared schemas.
pub fn resolve_schema_refs(schemas: &SourceSchemas) -> Result<SourceSchemas, String> {
    let src_angle_version = schemas.src_angle_version;

    // unqualMap | srcAngleVersion >= AngleVersion 10 = fromListWith (<>)
    //   [ (SourceRef name Nothing, HashSet.singleton ref)
    //   | SourceSchema{..} <- srcSchemas, let ref@(SourceRef name _) = schemaName ]
    //   | otherwise = HashMap.empty
    // fromListWith (<>) over singletons: HashSet union, commutative.
    let mut unqual_map: HashMap<SourceRef, HashSet<SourceRef>> = HashMap::new();
    if src_angle_version >= AngleVersion(10) {
        for schema in &schemas.src_schemas {
            unqual_map
                .entry(SourceRef {
                    source_ref_name: schema.schema_name.source_ref_name.clone(),
                    source_ref_version: None,
                })
                .or_insert_with(HashSet::new)
                .insert(schema.schema_name.clone());
        }
    }

    // refMap = unqualMap <> HashMap.fromList
    //   [ (schemaName, HashSet.singleton schemaName) | SourceSchema{..} <- srcSchemas ]
    // — HashMap.union is LEFT-biased (unqualMap wins on a shared key), and
    // fromList is last-wins on duplicate keys: build the right-hand map
    // first, then overwrite with unqual_map entries.
    let mut ref_map: HashMap<SourceRef, HashSet<SourceRef>> = HashMap::new();
    for schema in &schemas.src_schemas {
        ref_map.insert(
            schema.schema_name.clone(),
            HashSet::from([schema.schema_name.clone()]),
        );
    }
    for (k, v) in &unqual_map {
        ref_map.insert(k.clone(), v.clone());
    }

    // TODO(port): hash-order error selection (closed set of three sites
    // in this unit; single-failure outputs are byte-identical): (1) HERE
    // — HashSet.toList order in the "ambiguous schema" error fixes the
    // ORDER of the candidate list when more than one schema matches an
    // unversioned ref (Data.HashSet hash order vs std RandomState);
    // (2) resolve_one_schema's numRefs iteration — WHICH name's
    // "multiple definitions" error is reported first when several names
    // clash; (3) resolve_one_schema's unqualInheritedScope iteration —
    // WHICH ref's "inherited schemas give multiple definitions" error is
    // reported first when several clash. Each mirrors a HashMap/HashSet
    // toList the source itself iterates in hash order, so the source's
    // own output varies across hash seeds in exactly the same positions.
    let schema_by_name = |ref_: &SourceRef| -> Result<SourceRef, String> {
        match ref_map.get(ref_) {
            // unknown = throwError $ "unknown schema: " <> showRef ref
            None => Err(format!("unknown schema: {}", ref_.show_ref())),
            Some(s) => {
                let names: Vec<&SourceRef> = s.iter().collect();
                match names.as_slice() {
                    [one] => Ok((*one).clone()),
                    // names -> throwError $ "ambiguous schema " <> showRef ref
                    //   <> ", could be one of " <> intercalate ", " (map showRef names)
                    names => Err(format!(
                        "ambiguous schema {}, could be one of {}",
                        ref_.show_ref(),
                        names
                            .iter()
                            .map(|n| n.show_ref())
                            .collect::<Vec<_>>()
                            .join(", ")
                    )),
                }
            }
        }
    };

    // schemaByNameWithNamespace parentSchema ref =
    //   withExcept (<> " within schema: " <> showRef parentSchema) (schemaByName ref)
    let schema_by_name_with_namespace =
        |parent_schema: &SourceRef, ref_: &SourceRef| -> Result<SourceRef, String> {
            schema_by_name(ref_)
                .map_err(|e| format!("{} within schema: {}", e, parent_schema.show_ref()))
        };

    // resolveDecl parentSchema (SourceImport r s) = SourceImport <$>
    //   schemaByNameWithNamespace parentSchema r <*> pure s
    // resolveDecl _ decl = return decl
    let resolve_decl = |parent_schema: &SourceRef,
                        decl: &SourceDecl_|
     -> Result<SourceDecl_, String> {
        match decl {
            SourceDecl_::SourceImport(r, s) => Ok(SourceDecl_::SourceImport(
                schema_by_name_with_namespace(parent_schema, r)?,
                s.clone(),
            )),
            other => Ok(other.clone()),
        }
    };

    // resolveSchema SourceSchema{..}: inherits/decls resolved under the
    // schema's own namespace; name/srcSpan untouched.
    let new_schemas: Vec<SourceSchema> = schemas
        .src_schemas
        .iter()
        .map(|schema| {
            let inherits = schema
                .schema_inherits
                .iter()
                .map(|r| schema_by_name_with_namespace(&schema.schema_name, r))
                .collect::<Result<Vec<_>, _>>()?;
            let decls = schema
                .schema_decls
                .iter()
                .map(|decl| resolve_decl(&schema.schema_name, decl))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(SourceSchema_ {
                schema_name: schema.schema_name.clone(),
                schema_inherits: inherits,
                schema_decls: decls,
                schema_src_span: schema.schema_src_span,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    // resolveEvolve (SourceEvolves l n o) = SourceEvolves l <$>
    //   schemaByName n <*> schemaByName o — NO namespace suffix (unlike
    // imports/inherits). Traversed AFTER the schemas (source statement
    // order; a schema error outranks an evolves error).
    let new_evolves: Vec<SourceEvolves_> = schemas
        .src_evolves
        .iter()
        .map(|e| {
            Ok(SourceEvolves_ {
                evolves_span: e.evolves_span,
                evolves_new: schema_by_name(&e.evolves_new)?,
                evolves_old: schema_by_name(&e.evolves_old)?,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(SourceSchemas_ {
        src_angle_version,
        src_schemas: new_schemas,
        src_evolves: new_evolves,
    })
}

// `type Environment = HashMap SourceRef ResolvedSchemaRef` (Resolve.hs:251).
type Environment = HashMap<SourceRef, ResolvedSchemaRef>;

// ---------------------------------------------------------------------------
// resolveOneSchema and its scope-building helpers
// ---------------------------------------------------------------------------

/// HashMap.union — LEFT-biased (the left operand's entry survives a key
/// clash). RULE §0: the clone stands in for Haskell's shared immutable
/// maps.
fn union_name_env_left(
    left: &NameEnv<RefResolved>,
    right: &NameEnv<RefResolved>,
) -> NameEnv<RefResolved> {
    let mut out = right.clone();
    out.extend(left.iter().map(|(k, v)| (k.clone(), v.clone())));
    out
}

/// HashMap.unionWith Set.union.
fn union_name_env_with_sets(
    left: &NameEnv<RefResolved>,
    right: &NameEnv<RefResolved>,
) -> NameEnv<RefResolved> {
    let mut out = right.clone();
    for (k, v) in left {
        match out.get_mut(k) {
            Some(existing) => {
                // Set.union — the BTreeSet mapping of the crate's
                // sorted-container rule; ascending order preserved (the
                // Ambiguous payload order is observable through
                // lookup_result_to_either's error text).
                let merged: BTreeSet<RefResolved> = existing.union(v).cloned().collect();
                *existing = merged;
            }
            None => {
                out.insert(k.clone(), v.clone());
            }
        }
    }
    out
}

/// unionNameEnvs = foldl' (HashMap.unionWith Set.union) HashMap.empty
/// (Resolve.hs:342-343).
fn union_name_envs<'a>(
    envs: impl Iterator<Item = &'a NameEnv<RefResolved>>,
) -> NameEnv<RefResolved> {
    let mut out: NameEnv<RefResolved> = HashMap::new();
    for env in envs {
        out = union_name_env_with_sets(&out, env);
    }
    out
}

/// `qualify x = namespace <> "." <> x` (Resolve.hs:284-285).
fn qualify(namespace: &str, x: &str) -> Name {
    format!("{namespace}.{x}")
}

/// qualifyNameEnv :: NameEnv t -> NameEnv t (Resolve.hs:287-290):
/// re-key every (SourceRef name ver) as SourceRef (qualify name) ver —
/// the Maybe Version is carried through untouched.
fn qualify_name_env(env: &NameEnv<RefResolved>, namespace: &str) -> NameEnv<RefResolved> {
    env.iter()
        .map(|(name, target)| {
            (
                SourceRef {
                    source_ref_name: qualify(namespace, &name.source_ref_name),
                    source_ref_version: name.source_ref_version,
                },
                target.clone(),
            )
        })
        .collect()
}

/// nameEntries (Resolve.hs:345-348): both the unversioned and the
/// versioned SourceRef key map to {target}.
fn name_entries(
    name: &str,
    ver: Version,
    target: &RefResolved,
) -> Vec<(SourceRef, BTreeSet<RefResolved>)> {
    let set: BTreeSet<RefResolved> = Some(target.clone()).into_iter().collect();
    vec![
        (
            SourceRef {
                source_ref_name: name.to_owned(),
                source_ref_version: None,
            },
            set.clone(),
        ),
        (
            SourceRef {
                source_ref_name: name.to_owned(),
                source_ref_version: Some(ver),
            },
            set,
        ),
    ]
}

/// `resolveOneSchema :: Environment -> AngleVersion -> [SourceRef] ->
/// SourceSchema -> Except Text ResolvedSchemaRef` (Resolve.hs:253-499).
/// Module-private in the source (not in the export list).
///
/// The whole body is wrapped in `flip catchError inSchema` with
/// `inSchema e = "In " <> showRef schemaName <> ":\n  " <> e`
/// (Resolve.hs:260-262): every error from the body gets the prefix. The
/// name is captured before the schema moves into the inner call.
fn resolve_one_schema(
    env: &Environment,
    angle_version: AngleVersion,
    evolves: &[SourceRef],
    schema: SourceSchema,
) -> Result<ResolvedSchemaRef, String> {
    let in_schema_name = schema.schema_name.clone();
    resolve_one_schema1(env, angle_version, evolves, schema)
        .map_err(|e| format!("In {}:\n  {}", in_schema_name.show_ref(), e))
}

fn resolve_one_schema1(
    env: &Environment,
    angle_version: AngleVersion,
    evolves: &[SourceRef],
    schema: SourceSchema,
) -> Result<ResolvedSchemaRef, String> {
    // let SourceRef namespace maybeVer = schemaName
    let SourceRef {
        source_ref_name: namespace,
        source_ref_version: maybe_ver,
    } = schema.schema_name.clone();

    // schemaByName name = case HashMap.lookup name env of
    //   Nothing -> throwError $ "unknown schema: " <> showRef name
    //   Just one -> return one
    // Cloned out of env (the Haskell shares; RULE §0 stand-in).
    let schema_by_name = |name: &SourceRef| -> Result<ResolvedSchemaRef, String> {
        match env.get(name) {
            None => Err(format!("unknown schema: {}", name.show_ref())),
            Some(one) => Ok(one.clone()),
        }
    };

    check_namespace(&namespace)?;

    // Version of this schema
    let version = match maybe_ver {
        None => {
            return Err(format!(
                "missing version: {}",
                schema.schema_name.show_ref()
            ))
        }
        Some(v) => v,
    };

    // All the schemas we're inheriting from
    let inherits: Vec<ResolvedSchemaRef> = schema
        .schema_inherits
        .iter()
        .map(|name| schema_by_name(name))
        .collect::<Result<Vec<_>, _>>()?;

    // All the schemas we imported
    let imports: Vec<ResolvedSchemaRef> = schema
        .schema_decls
        .iter()
        .filter_map(|decl| match decl {
            SourceDecl_::SourceImport(name, _) => Some(name),
            _ => None,
        })
        .map(|name| schema_by_name(name))
        .collect::<Result<Vec<_>, _>>()?;

    // localPreds (Resolve.hs:292-300): (unqualified name, qualified
    // PredicateRef with explicit-or-schema version, the source def) per
    // SourcePredicate decl, in declaration order. thisVersion =
    // fromMaybe version explicitVersion.
    let local_preds: Vec<(Name, PredicateRef, SourcePredicateDef)> = schema
        .schema_decls
        .iter()
        .filter_map(|decl| match decl {
            SourceDecl_::SourcePredicate(p) => {
                let SourceRef {
                    source_ref_name: name,
                    source_ref_version: explicit_version,
                } = &p.predicate_def_ref;
                let this_version = explicit_version.unwrap_or(version);
                let qname = qualify(&namespace, name);
                Some((
                    name.clone(),
                    PredicateRef {
                        name: qname,
                        version: this_version,
                    },
                    p.clone(),
                ))
            }
            _ => None,
        })
        .collect();

    // localTypes (Resolve.hs:302-310): same shape over SourceType decls.
    let local_types_list: Vec<(Name, TypeRef, SourceTypeDef)> = schema
        .schema_decls
        .iter()
        .filter_map(|decl| match decl {
            SourceDecl_::SourceType(p) => {
                let SourceRef {
                    source_ref_name: name,
                    source_ref_version: explicit_version,
                } = &p.type_def_ref;
                let this_version = explicit_version.unwrap_or(version);
                let qname = qualify(&namespace, name);
                Some((
                    name.clone(),
                    TypeRef {
                        name: qname,
                        version: this_version,
                    },
                    p.clone(),
                ))
            }
            _ => None,
        })
        .collect();

    // Check for multiple definitions of the same name/version. (Source
    // comment: multiple definitions of the same name is OK — an
    // unqualified reference will be rejected as ambiguous, but can be
    // resolved by using an explicit version.)
    //
    // numRefs = HashMap.fromListWith (+) (preds ++ types) — counts
    // across BOTH lists; the error is "multiple definitions for: " <>
    // name <> "." <> showt ver (showt on the thrift i32 is signed
    // decimal).
    let mut num_refs: HashMap<(&Name, Version), i64> = HashMap::new();
    for (name, r, _) in &local_preds {
        *num_refs.entry((name, r.version)).or_insert(0) += 1;
    }
    for (name, r, _) in &local_types_list {
        *num_refs.entry((name, r.version)).or_insert(0) += 1;
    }
    for ((name, ver), num) in &num_refs {
        if *num > 1 {
            return Err(format!("multiple definitions for: {name}.{ver}"));
        }
    }

    // Build the scope: a mapping from unversioned names to RefTarget.
    // (Source comment.) The union nesting below is exactly the source's
    // (Resolve.hs:369-374); the helpers state each union's bias.
    //
    // inherited definitions are in scope unqualified and qualified
    let qual_inherited_scope =
        union_name_envs(inherits.iter().map(|s| &s.resolved_schema_qual_scope));
    let unqual_inherited_scope =
        union_name_envs(inherits.iter().map(|s| &s.resolved_schema_unqual_scope));
    // imported names are in scope qualified only
    let imported_scope = union_name_envs(imports.iter().map(|s| &s.resolved_schema_qual_scope));

    // local definitions are in scope unqualified and qualified
    // unqualLocalScope = HashMap.fromListWith Set.union unqualLocalEntities
    // where the entities are P and P.1 per local predicate then T and
    // T.1 per local type (source order: predicates first).
    let mut unqual_local_scope: NameEnv<RefResolved> = HashMap::new();
    for (name, r, _) in &local_preds {
        for entry in name_entries(name, r.version, &RefTarget::RefPred(r.clone())) {
            unqual_local_scope
                .entry(entry.0)
                .or_insert_with(BTreeSet::new)
                .extend(entry.1);
        }
    }
    for (name, r, _) in &local_types_list {
        for entry in name_entries(name, r.version, &RefTarget::RefType(r.clone())) {
            unqual_local_scope
                .entry(entry.0)
                .or_insert_with(BTreeSet::new)
                .extend(entry.1);
        }
    }
    let qual_local_scope = qualify_name_env(&unqual_local_scope, &namespace);

    // scope = HashMap.union unqualLocalScope $ HashMap.union
    // unqualInheritedScope $ HashMap.unionWith Set.union qualLocalScope $
    // HashMap.unionWith Set.union qualInheritedScope importedScope
    let scope = union_name_env_left(
        &unqual_local_scope,
        &union_name_env_left(
            &unqual_inherited_scope,
            &union_name_env_with_sets(
                &qual_local_scope,
                &union_name_env_with_sets(&qual_inherited_scope, &imported_scope),
            ),
        ),
    );

    // Check for inheriting multiple versions of a predicate/types.
    //
    // As a special case, if the schema is called "all" then allow name
    // clashes in the inheritance set (source comment: the "all.N"
    // convention inevitably clashes, but we don't care about what it
    // exports). case Set.elems targets of (_:_:_) -> error — i.e. >= 2
    // targets.
    if namespace != "all" {
        for (ref_, targets) in &unqual_inherited_scope {
            if targets.len() >= 2 {
                return Err(format!(
                    "inherited schemas give multiple definitions for: {}",
                    ref_.show_ref()
                ));
            }
        }
    }

    // resolve type definitions
    let types: Vec<(Name, ResolvedTypeDef)> = local_types_list
        .into_iter()
        .map(|(name, ref_, def)| {
            check_name(&name)?;
            let ty =
                run_resolve(angle_version, &scope, |ctx| resolve_type(ctx, def.type_def_type))?;
            Ok((
                name,
                TypeDef_ {
                    type_def_ref: ref_,
                    type_def_type: ty,
                    type_def_src_span: def.type_def_src_span,
                },
            ))
        })
        .collect::<Result<Vec<_>, _>>()?;

    // resolve predicate definitions — predicateDefDeriving = NoDeriving
    // (deriving is attached below via the schema's SourceDeriving decls)
    let predicates: Vec<(Name, ResolvedPredicateDef)> = local_preds
        .into_iter()
        .map(|(name, ref_, def)| {
            check_name(&name)?;
            run_resolve(angle_version, &scope, |ctx| {
                let key = resolve_type(ctx, def.predicate_def_key_type)?;
                let value = resolve_type(ctx, def.predicate_def_value_type)?;
                Ok((
                    name.clone(),
                    PredicateDef_ {
                        predicate_def_ref: ref_.clone(),
                        predicate_def_key_type: key,
                        predicate_def_value_type: value,
                        predicate_def_deriving: DerivingInfo::NoDeriving,
                        predicate_def_src_span: def.predicate_def_src_span,
                    },
                ))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    // scope of predicates that we can specify queries for. Namely
    // locally-defined predicates and inherited predicates. (Source
    // comment.) predScope is the NameEnv itself; `resolveRef $` applies
    // it at each use.
    let pred_scope = union_name_env_left(
        &unqual_local_scope,
        &union_name_env_left(
            &unqual_inherited_scope,
            &union_name_env_with_sets(&qual_local_scope, &qual_inherited_scope),
        ),
    );

    // resolve queries: localDeriving over [ (derivingDefRef, derive) |
    // SourceDeriving derive@DerivingDef{..} <- schemaDecls ] — look the
    // ref up in predScope, insist it is a predicate (`showRef ref <> " is
    // not a predicate"` renders the SOURCE ref), resolve the deriving
    // query. The output DerivingDef carries the RESOLVED PredicateRef.
    let local_deriving: Vec<(PredicateRef, ResolvedDerivingDef)> = schema
        .schema_decls
        .iter()
        .filter_map(|decl| match decl {
            SourceDecl_::SourceDeriving(d) => Some(d.clone()),
            _ => None,
        })
        .map(|derive| {
            let ref_ = &derive.deriving_def_ref;
            let ty = lookup_result_to_except(ref_, resolve_ref(&pred_scope, ref_))?;
            // ref <- case ty of RefPred ref -> return ref; _ -> error
            let pred_ref = match ty {
                RefTarget::RefPred(r) => r,
                _ => return Err(format!("{} is not a predicate", ref_.show_ref())),
            };
            let resolved_der_info = run_resolve(angle_version, &scope, |ctx| {
                resolve_deriving(ctx, derive.deriving_def_derive_info.clone())
            })?;
            Ok((
                pred_ref.clone(),
                DerivingDef_ {
                    deriving_def_ref: pred_ref,
                    deriving_def_derive_info: resolved_der_info,
                    deriving_def_src_span: derive.deriving_def_src_span,
                },
            ))
        })
        .collect::<Result<Vec<_>, _>>()?;

    // localTypeNames = HashSet.fromList $ map (typeRef_name . typeDefRef
    // . snd) types — membership only, order unobservable.
    let local_type_names: HashSet<Name> = types
        .iter()
        .map(|(_, def)| def.type_def_ref.name.clone())
        .collect();

    // localTypes (the HashMap; the source shadows the list name) =
    // HashMap.fromList [(typeDefRef def, def) | (_, def) <- types]
    let local_types: HashMap<TypeRef, ResolvedTypeDef> = types
        .iter()
        .map(|(_, def)| (def.type_def_ref.clone(), def.clone()))
        .collect();

    // The types we re-export from this schema are all the types that are
    // inherited but not shadowed by a locally-defined type. (Source
    // comment.) `resolvedSchemaTypes schema <> resolvedSchemaReExportedTypes
    // schema` is a LEFT-biased HashMap union (own types win); fromList
    // is last-wins.
    let mut re_exported_types: HashMap<TypeRef, ResolvedTypeDef> = HashMap::new();
    for inherited in &inherits {
        let mut combined: HashMap<TypeRef, ResolvedTypeDef> =
            inherited.resolved_schema_re_exported_types.clone();
        for (k, v) in &inherited.resolved_schema_types {
            combined.insert(k.clone(), v.clone());
        }
        for (ref_, def) in combined {
            if !local_type_names.contains(&ref_.name) {
                re_exported_types.insert(ref_, def);
            }
        }
    }

    // localPredicateNames = HashSet.fromList [ predicateRef_name
    // (predicateDefRef def) | (_, def) <- predicates ]
    let local_predicate_names: HashSet<Name> = predicates
        .iter()
        .map(|(_, def)| def.predicate_def_ref.name.clone())
        .collect();

    // localPredicates = HashMap.fromList [(predicateDefRef def, def) | (_, def) <- predicates]
    let local_predicates: HashMap<PredicateRef, ResolvedPredicateDef> = predicates
        .iter()
        .map(|(_, def)| (def.predicate_def_ref.clone(), def.clone()))
        .collect();

    // Similarly for predicates. (Source comment.)
    let mut re_exported_predicates: HashMap<PredicateRef, ResolvedPredicateDef> = HashMap::new();
    for inherited in &inherits {
        let mut combined: HashMap<PredicateRef, ResolvedPredicateDef> =
            inherited.resolved_schema_re_exported_predicates.clone();
        for (k, v) in &inherited.resolved_schema_predicates {
            combined.insert(k.clone(), v.clone());
        }
        for (ref_, def) in combined {
            if !local_predicate_names.contains(&ref_.name) {
                re_exported_predicates.insert(ref_, def);
            }
        }
    }

    // exportedUnqualScope = HashMap.union unqualLocalScope
    // unqualInheritedScope — Note: local names override inherited names.
    let exported_unqual_scope = union_name_env_left(&unqual_local_scope, &unqual_inherited_scope);

    // Qualified exported names. The "all" schema is special: we don't
    // re-export things qualified with "all.", only with the original
    // schema name. (Source comment; NB. local overrides inherited.)
    let exported_qual_scope = if namespace == "all" {
        qual_inherited_scope
    } else {
        union_name_env_left(
            &qualify_name_env(&exported_unqual_scope, &namespace),
            &qual_inherited_scope,
        )
    };

    // schemaEvolves <- traverse schemaByName evolves — every evolve target
    // was canonicalized by resolveSchemaRefs, so the env lookup succeeds;
    // the unknown-schema arm is kept for shape.
    let schema_evolves: Vec<ResolvedSchemaRef> = evolves
        .iter()
        .map(|name| schema_by_name(name))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ResolvedSchema {
        resolved_schema_name: namespace,
        resolved_schema_version: version,
        resolved_schema_angle_version: angle_version,
        resolved_schema_types: local_types,
        resolved_schema_re_exported_types: re_exported_types,
        resolved_schema_predicates: local_predicates,
        resolved_schema_re_exported_predicates: re_exported_predicates,
        resolved_schema_unqual_scope: exported_unqual_scope,
        resolved_schema_qual_scope: exported_qual_scope,
        resolved_schema_deriving: local_deriving.into_iter().collect(),
        // Set.fromList (schemaRef <$> schemaEvolves) — BTreeSet.
        resolved_schema_evolves: schema_evolves.iter().map(schema_ref).collect(),
    })
}

// ---------------------------------------------------------------------------
// Resolving types
// ---------------------------------------------------------------------------

/// `resolveType :: (ShowRef t, ShowRef p) => SourceType -> Resolve p t
/// (Type_ SrcSpan p t)` (Resolve.hs:501-538; exported). The Rust port
/// keeps the generics (bounds P: Clone + ShowRef, T: Clone + ShowRef —
/// see the header note: TWO source instantiation points, this unit's
/// RefResolved and glean-db UserQuery's RefTargetId) and threads the
/// Reader environment as ResolveCtx. The type MOVES: the Haskell
/// consumes it and rebuilds (SourceRef payloads become p/t refs).
pub fn resolve_type<P, T>(
    ctx: &ResolveCtx<'_, P, T>,
    typ: SourceType,
) -> Result<Type_<SrcSpan, P, T>, String>
where
    P: Clone + ShowRef,
    T: Clone + ShowRef,
{
    match typ {
        Type_::ByteTy => Ok(Type_::ByteTy),
        Type_::NatTy => Ok(Type_::NatTy),
        Type_::StringTy => Ok(Type_::StringTy),
        Type_::ArrayTy(ty) => Ok(Type_::ArrayTy(Box::new(resolve_type(ctx, *ty)?))),
        Type_::RecordTy(fields) => {
            check_fields(&fields)?;
            Ok(Type_::RecordTy(
                fields
                    .into_iter()
                    .map(|field| go_field(ctx, field))
                    .collect::<Result<Vec<_>, _>>()?,
            ))
        }
        Type_::SumTy(fields) => {
            check_fields(&fields)?;
            Ok(Type_::SumTy(
                fields
                    .into_iter()
                    .map(|field| go_field(ctx, field))
                    .collect::<Result<Vec<_>, _>>()?,
            ))
        }
        Type_::SetTy(ty) => Ok(Type_::SetTy(Box::new(resolve_type(ctx, *ty)?))),
        Type_::PredicateTy(s, ref_) => go_ref(ctx, ref_, s),
        // NamedTy s ref -> goRef ref s -- shouldn't happen, but handle it
        // anyway (source comment)
        Type_::NamedTy(s, ref_) => go_ref(ctx, ref_, s),
        Type_::MaybeTy(ty) => Ok(Type_::MaybeTy(Box::new(resolve_type(ctx, *ty)?))),
        Type_::EnumeratedTy(names) => {
            // lift $ do mapM_ checkName names >> return (EnumeratedTy names)
            for name in &names {
                check_name(name)?;
            }
            Ok(Type_::EnumeratedTy(names))
        }
        Type_::BooleanTy => Ok(Type_::BooleanTy),
        // `_ -> error "resolveType"` — the catch-all over the
        // typechecker-only constructors (TyVar/HasTy/HasKey/ElementsOf).
        // Dead arm: the parser never produces these and resolveOneSchema
        // feeds resolveType only parser output, so the eager panic is
        // observationally identical to the source's lazy `error` thunk.
        // RULE §2 partial-functions row: same message.
        Type_::TyVar(_) | Type_::HasTy(..) | Type_::HasKey(..) | Type_::ElementsOf(..) => {
            panic!("resolveType")
        }
    }
}

/// `goRef ref s = do scope <- getScope; target <- lift $
/// lookupResultToExcept ref $ resolveRef scope ref; case target of
/// RefType ref -> NamedTy s ref; RefPred ref -> PredicateTy s ref`
/// (Resolve.hs:522-528).
fn go_ref<P, T>(
    ctx: &ResolveCtx<'_, P, T>,
    ref_: SourceRef,
    s: SrcSpan,
) -> Result<Type_<SrcSpan, P, T>, String>
where
    P: Clone + ShowRef,
    T: Clone + ShowRef,
{
    let scope = ctx.get_scope();
    let target = lookup_result_to_except(&ref_, resolve_ref(scope, &ref_))?;
    match target {
        RefTarget::RefType(ref_) => Ok(Type_::NamedTy(s, ref_)),
        RefTarget::RefPred(ref_) => Ok(Type_::PredicateTy(s, ref_)),
    }
}

/// `goField (FieldDef name ty) = FieldDef name <$> go ty` (Resolve.hs:538).
fn go_field<P, T>(
    ctx: &ResolveCtx<'_, P, T>,
    field: FieldDef_<SrcSpan, SourceRef, SourceRef>,
) -> Result<FieldDef_<SrcSpan, P, T>, String>
where
    P: Clone + ShowRef,
    T: Clone + ShowRef,
{
    Ok(FieldDef_ {
        field_def_name: field.field_def_name,
        field_def_type: resolve_type(ctx, field.field_def_type)?,
    })
}

/// `checkFields fields = sequence_ [ throwError $ "duplicate field: " <>
/// x | x:_:_ <- group $ sort $ map fieldDefName fields ] >> lift (mapM_
/// (checkName . fieldDefName) fields >> mapM_ (checkFieldName .
/// fieldDefName) fields)` (Resolve.hs:530-536). Duplicate detection runs
/// FIRST and reports the FIRST duplicate in ascending name order
/// (Data.List.sort = sortBy compare; group = consecutive equals).
fn check_fields(fields: &[FieldDef_<SrcSpan, SourceRef, SourceRef>]) -> Result<(), String> {
    let mut names: Vec<&Name> = fields.iter().map(|f| &f.field_def_name).collect();
    names.sort();
    for group in names.chunk_by(|a, b| a == b) {
        // x:_:_ — a group of at least two
        if group.len() >= 2 {
            return Err(format!("duplicate field: {}", group[0]));
        }
    }
    // mapM_ checkName over ALL fields, THEN mapM_ checkFieldName over
    // all fields (Resolve.hs:534-536: the two traversals are
    // sequential, not interleaved — a reserved-word error anywhere in
    // the field list outranks a case error on an EARLIER field).
    for field in fields {
        check_name(&field.field_def_name)?;
    }
    for field in fields {
        check_field_name(&field.field_def_name)?;
    }
    Ok(())
}

/// `lookupResultToExcept` (Resolve.hs:541-547): either throwError or
/// return over lookupResultToEither. Thin adapter over the sibling's
/// generic lookup_result_to_either (crate::schema::types owns the error
/// strings).
fn lookup_result_to_except<P, T>(
    ref_: &SourceRef,
    res: LookupResult<RefTarget<P, T>>,
) -> Result<RefTarget<P, T>, String>
where
    P: ShowRef,
    T: ShowRef,
{
    lookup_result_to_either(ref_, res)
}

// ---------------------------------------------------------------------------
// Checks
// ---------------------------------------------------------------------------

/// `checkAngleVersion v = unless (v >= latestSupportedAngleVersion) $
/// throwError $ "Angle version " <> Text.pack (show (pretty v)) <> " is
/// not supported"` (Resolve.hs:549-553). `pretty` on AngleVersion is
/// deriving-newtype Pretty Int — decimal — matching the sibling types.rs
/// Display impl's rendering.
fn check_angle_version(v: AngleVersion) -> Result<(), String> {
    if v < latest_supported_angle_version() {
        // ver = Text.pack $ show $ pretty v
        let ver = v.0.to_string();
        Err(format!("Angle version {ver} is not supported"))
    } else {
        Ok(())
    }
}

/// `checkFieldName n = when (isUpper (Text.head n)) $ throwError $ "field
/// names must begin with a lowercase letter: " <> n` (Resolve.hs:555-558).
/// Data.Char.isUpper is the Unicode Uppercase property (Lu +
/// Other_Uppercase) — exactly Rust's char::is_uppercase. Text.head is
/// partial on the empty string: RULE §2 partial-functions row, panic with
/// text-2.0.2's exact message — "Data.Text.Internal.Fusion.Common.head:
/// Empty stream" (NOT text-1.x's "Data.Text.head: empty input"; verified
/// by running the pinned text-2.0.2 under this tree's GHC 9.4.7).
/// Unreachable from the parser (field names are identifiers), and
/// faithfully eager when it did fire — the source forces the same check
/// at the same point.
fn check_field_name(n: &Name) -> Result<(), String> {
    match n.chars().next() {
        None => panic!("Data.Text.Internal.Fusion.Common.head: Empty stream"),
        Some(c) if c.is_uppercase() => {
            Err(format!("field names must begin with a lowercase letter: {n}"))
        }
        Some(_) => Ok(()),
    }
}

/// `checkName n = when (n `HashSet.member` reservedWords) $ throwError $
/// n <> " is a reserved word, it cannot be used"` (Resolve.hs:560-563).
fn check_name(n: &Name) -> Result<(), String> {
    if is_reserved_word(n) {
        Err(format!("{n} is a reserved word, it cannot be used"))
    } else {
        Ok(())
    }
}

/// `checkNameSpace n = mapM_ checkName (splitDot n)` (Resolve.hs:565-566).
///
/// splitDot returns a PAIR `(NameSpaces, Text)` and mapM_ over a tuple
/// traverses ONLY the second component (base's Foldable ((,) a) folds
/// the tail) — verified against this tree's GHC 9.4.7: mapM_ checkName
/// (splitDot "if.a") visits only "a".
///
/// BUG(port): checkNameSpace therefore checks ONLY the LAST dot-segment
/// of a namespace — a reserved word in a NON-final segment goes
/// undetected (reproduced bug-for-bug per RULE §2). Repro:
///   `schema if.a.1 { }` resolves without error even though "if" is in
///   reservedWords (only "a" is checked);
///   `schema a.if.1 { }` errors with "if is a reserved word, it cannot
///   be used".
/// Fix ships only as a flagged post-parity change.
fn check_namespace(n: &Name) -> Result<(), String> {
    // mapM_ checkName (splitDot n) == checkName (snd (splitDot n))
    let (_, last_segment) = split_dot(n);
    check_name(&last_segment)
}

/// `reservedWords` (Resolve.hs:568-660): the word list verbatim, source
/// order (order is unobservable — membership only). The source's own
/// `{- TODO: "Enum" -}` is a source note, not an open port item: the
/// word was never added, so it is absent here too.
fn is_reserved_word(n: &str) -> bool {
    const RESERVED_WORDS: &[&str] = &[
        // Thrift reserved words, seems like a good idea to avoid these
        "binary", "bool", "byte", "const", "cpp_include",
        "double", "enum", "exception", "extends", "false", "float",
        "hash_map", "hash_set", "hs_include", "i16", "i32", "i64", "include",
        "list", "map", "namespace", "oneway", "optional", "required", "senum",
        "service", "set", "stream", "string", "struct", "throws", "true",
        "typedef", "union", "view", "void",
        // keywords that the fbthrift compiler rejects (see
        // thrift/compiler/parse/thrift.ll)
        "abstract",
        "and",
        "args",
        "as",
        "assert",
        "auto",
        "break",
        "case",
        "char",
        "class",
        "continue",
        "declare",
        "def",
        "default",
        "del",
        "do",
        "elif",
        "else",
        "elseif",
        "except",
        "exec",
        "extern",
        "finally",
        "for",
        "foreach",
        "function",
        "global",
        "goto",
        "if",
        "implements",
        "import",
        "in",
        "int",
        "inline",
        "instanceof",
        "interface",
        "is",
        "lambda",
        "long",
        "native",
        "new",
        "not",
        "or",
        "pass",
        "public",
        "print",
        "private",
        "protected",
        "raise",
        "register",
        "return",
        "short",
        "signed",
        "sizeof",
        "static",
        "switch",
        "synchronized",
        "template",
        "this",
        "throw",
        "transient",
        "try",
        "unsigned",
        "var",
        "virtual",
        "volatile",
        "while",
        "with",
        "yield",
        "Object",
        "Client",
        "IFace",
        "Processor",
        // Other words to avoid, because they lead to Thrift-generated code that
        // does not compile for one or more languages.
        "None", "operator", // source note, not taken: "Enum"
        // Reserved for temporary predicates
        "_tmp_",
    ];
    RESERVED_WORDS.contains(&n)
}

// ---------------------------------------------------------------------------
// Resolving queries
// ---------------------------------------------------------------------------

/// `runResolve ver scope act = runReaderT act (ver,scope)`
/// (Resolve.hs:666-671; exported). The Reader environment as a borrowed
/// struct; `act` is the Resolve action. Generic over the ref-target pair
/// (see the header note): this unit instantiates at RefResolved, the
/// glean-db UserQuery port at RefTarget PredicateId TypeId.
pub fn run_resolve<A, F, P, T>(
    ver: AngleVersion,
    scope: &NameEnv<RefTarget<P, T>>,
    act: F,
) -> Result<A, String>
where
    F: FnOnce(&ResolveCtx<'_, P, T>) -> Result<A, String>,
{
    let ctx = ResolveCtx {
        angle_version: ver,
        scope,
    };
    act(&ctx)
}

/// `type Resolve p t a = ReaderT (AngleVersion, NameEnv (RefTarget p t))
/// (Except Text) a` (Resolve.hs:673-674) — generic over the ref-target
/// pair (see the header note). Pub because the exported resolve_type /
/// resolve_query take it.
pub struct ResolveCtx<'a, P, T> {
    pub angle_version: AngleVersion,
    scope: &'a NameEnv<RefTarget<P, T>>,
}

impl<P, T> ResolveCtx<'_, P, T> {
    /// `getScope = asks snd` (Resolve.hs:676-677; module-private).
    fn get_scope(&self) -> &NameEnv<RefTarget<P, T>> {
        self.scope
    }
}

/// `resolveDeriving` (Resolve.hs:679-684): NoDeriving passes through;
/// Derive keeps its when and resolves the query.
fn resolve_deriving<P, T>(
    ctx: &ResolveCtx<'_, P, T>,
    info: DerivingInfo<SourceQuery_<SrcSpan, SrcSpan, SourceRef, SourceRef>>,
) -> Result<DerivingInfo<SourceQuery_<SrcSpan, SrcSpan, P, T>>, String>
where
    P: Clone + ShowRef,
    T: Clone + ShowRef,
{
    match info {
        DerivingInfo::NoDeriving => Ok(DerivingInfo::NoDeriving),
        DerivingInfo::Derive(when, query) => {
            Ok(DerivingInfo::Derive(when, resolve_query(ctx, query)?))
        }
    }
}

/// `resolveQuery (SourceQuery head stmts ord) = SourceQuery <$>
/// mapM resolvePat head <*> mapM resolveStatement stmts <*> pure ord`
/// (Resolve.hs:686-694; exported). Generic over the ref-target pair
/// (header note).
pub fn resolve_query<P, T>(
    ctx: &ResolveCtx<'_, P, T>,
    query: SourceQuery_<SrcSpan, SrcSpan, SourceRef, SourceRef>,
) -> Result<SourceQuery_<SrcSpan, SrcSpan, P, T>, String>
where
    P: Clone + ShowRef,
    T: Clone + ShowRef,
{
    Ok(SourceQuery_ {
        src_query_head: query
            .src_query_head
            .map(|head| resolve_pat(ctx, head))
            .transpose()?,
        src_query_stmts: query
            .src_query_stmts
            .into_iter()
            .map(|stmt| resolve_statement(ctx, stmt))
            .collect::<Result<Vec<_>, _>>()?,
        src_query_ordered: query.src_query_ordered,
    })
}

/// `resolvePat` (Resolve.hs:696-774; module-private — not in the export
/// list). Arm order mirrors the source's declaration order. Generic over
/// the ref-target pair (header note).
fn resolve_pat<P, T>(
    ctx: &ResolveCtx<'_, P, T>,
    pat: SourcePat_<SrcSpan, SrcSpan, SourceRef, SourceRef>,
) -> Result<SourcePat_<SrcSpan, SrcSpan, P, T>, String>
where
    P: Clone + ShowRef,
    T: Clone + ShowRef,
{
    // Clause{}/Prim{} -> internal — needs the ORIGINAL pattern for the
    // rendered message; these resolver-introduced forms never come out of
    // the parser, so hoisting the check above the match changes nothing
    // observable.
    if matches!(pat, SourcePat_::Clause(..) | SourcePat_::Prim(..)) {
        return Err(internal_error(&pat));
    }
    // span of the whole pattern, for `prettyErrorIn pat`
    let span = *source_pat_span(&pat);
    match pat {
        SourcePat_::Nat(s, i) => Ok(SourcePat_::Nat(s, i)),
        SourcePat_::String(s, t) => Ok(SourcePat_::String(s, t)),
        SourcePat_::StringPrefix(s, t) => Ok(SourcePat_::StringPrefix(s, t)),
        SourcePat_::ByteArray(s, b) => Ok(SourcePat_::ByteArray(s, b)),
        SourcePat_::Array(s, pats) => Ok(SourcePat_::Array(
            s,
            pats
                .into_iter()
                .map(|p| resolve_pat(ctx, p))
                .collect::<Result<Vec<_>, _>>()?,
        )),
        SourcePat_::ArrayPrefix(s, pats) => Ok(SourcePat_::ArrayPrefix(
            s,
            // Box: the ArrayPrefix payload is Box<NonEmpty<_>> (E0072
            // repair, angle/types.rs:483-488) — pats auto-derefs.
            Box::new(NonEmpty {
                head: resolve_pat(ctx, pats.head)?,
                tail: pats
                    .tail
                    .into_iter()
                    .map(|p| resolve_pat(ctx, p))
                    .collect::<Result<Vec<_>, _>>()?,
            }),
        )),
        SourcePat_::Tuple(s, pats) => Ok(SourcePat_::Tuple(
            s,
            pats
                .into_iter()
                .map(|p| resolve_pat(ctx, p))
                .collect::<Result<Vec<_>, _>>()?,
        )),
        SourcePat_::All(s, qs) => Ok(SourcePat_::All(s, Box::new(resolve_pat(ctx, *qs)?))),
        SourcePat_::Elements(s, pat) => Ok(SourcePat_::Elements(
            s,
            Box::new(resolve_pat(ctx, *pat)?),
        )),
        SourcePat_::Wildcard(s) => Ok(SourcePat_::Wildcard(s)),
        SourcePat_::TypeSignature(s, pat, ty) => Ok(SourcePat_::TypeSignature(
            s,
            Box::new(resolve_pat(ctx, *pat)?),
            Box::new(resolve_type(ctx, *ty)?),
        )),
        SourcePat_::Variable(s, n) => Ok(SourcePat_::Variable(s, n)),
        SourcePat_::FactId(s, None, id) => Ok(SourcePat_::FactId(s, None, id)),
        SourcePat_::FactId(s, Some(pred), id) => {
            let res = resolve_type_or_pred(ctx, s, &pred)?;
            match res {
                RefTarget::RefPred(ref_) => Ok(SourcePat_::TypeSignature(
                    s,
                    Box::new(SourcePat_::FactId(s, None, id)),
                    Box::new(Type_::PredicateTy(s, ref_)),
                )),
                // _other -> prettyErrorIn pat $ "not a predicate: " <>
                // pretty pred — `pretty pred` on a Name is raw text.
                _other => pretty_error_at(
                    span,
                    Doc::text("not a predicate: ").append(Doc::text(pred.clone())),
                ),
            }
        }
        SourcePat_::OrPattern(s, l, r) => Ok(SourcePat_::OrPattern(
            s,
            Box::new(resolve_pat(ctx, *l)?),
            Box::new(resolve_pat(ctx, *r)?),
        )),
        SourcePat_::Negation(s, pat) => Ok(SourcePat_::Negation(
            s,
            Box::new(resolve_pat(ctx, *pat)?),
        )),
        SourcePat_::Never(s) => Ok(SourcePat_::Never(s)),
        SourcePat_::IfPattern {
            span: s,
            cond,
            then_,
            else_,
        } => Ok(SourcePat_::IfPattern {
            span: s,
            cond: Box::new(resolve_pat(ctx, *cond)?),
            then_: Box::new(resolve_pat(ctx, *then_)?),
            else_: Box::new(resolve_pat(ctx, *else_)?),
        }),
        SourcePat_::ElementsOfArray(s, pat) => Ok(SourcePat_::ElementsOfArray(
            s,
            Box::new(resolve_pat(ctx, *pat)?),
        )),
        SourcePat_::KeyValue(s, k, v) => Ok(SourcePat_::KeyValue(
            s,
            Box::new(resolve_pat(ctx, *k)?),
            Box::new(resolve_pat(ctx, *v)?),
        )),
        SourcePat_::NestedQuery(s, q) => Ok(SourcePat_::NestedQuery(
            s,
            Box::new(resolve_query(ctx, *q)?),
        )),
        SourcePat_::Struct(s, fields) => Ok(SourcePat_::Struct(
            s,
            fields
                .into_iter()
                .map(|Field(n, pat)| Ok(Field(n, resolve_pat(ctx, pat)?)))
                .collect::<Result<Vec<_>, _>>()?,
        )),
        SourcePat_::App(s, f, args) => match *f {
            // App s (Variable svar txt) args — the primop table first
            // (no arity check on Prim), else the single-arg
            // clause/annotation path, else "unexpected extra
            // argument(s)" (the Variable arm's | otherwise catches every
            // non-primitive application of a variable).
            SourcePat_::Variable(svar, txt) => {
                if let Some(prim_op) = primitive(&txt) {
                    Ok(SourcePat_::Prim(
                        s,
                        prim_op,
                        args
                            .into_iter()
                            .map(|p| resolve_pat(ctx, p))
                            .collect::<Result<Vec<_>, _>>()?,
                    ))
                } else {
                    let mut args_iter = args.into_iter();
                    match (args_iter.next(), args_iter.next()) {
                        (Some(arg), None) => {
                            // arg' <- resolvePat arg
                            let arg2 = resolve_pat(ctx, arg)?;
                            // let (range, txt') = if
                            //   | Just rem <- Text.stripSuffix "#new" txt -> (SeekOnStacked, rem)
                            //   | Just rem <- Text.stripSuffix "#old" txt -> (SeekOnBase, rem)
                            //   | otherwise -> (SeekOnAllFacts, txt)
                            let (range, txt2): (SeekSection, Name) =
                                if let Some(rem) = txt.strip_suffix("#new") {
                                    (SeekSection::SeekOnStacked, rem.to_owned())
                                } else if let Some(rem) = txt.strip_suffix("#old") {
                                    (SeekSection::SeekOnBase, rem.to_owned())
                                } else {
                                    (SeekSection::SeekOnAllFacts, txt.clone())
                                };
                            let res = resolve_type_or_pred(ctx, svar, &txt2)?;
                            match res {
                                // RefPred ref -> Clause s svar ref arg' range
                                RefTarget::RefPred(ref_) => Ok(SourcePat_::Clause(
                                    s,
                                    svar,
                                    ref_,
                                    Box::new(arg2),
                                    range,
                                )),
                                // RefType ref | SeekOnAllFacts <- range ->
                                //   TypeSignature s arg' (NamedTy svar ref)
                                // — the "T pat" desugaring for "pat : T"
                                //   (source comment: something we might
                                //   deprecate later).
                                RefTarget::RefType(ref_)
                                    if range == SeekSection::SeekOnAllFacts =>
                                {
                                    Ok(SourcePat_::TypeSignature(
                                        s,
                                        Box::new(arg2),
                                        Box::new(Type_::NamedTy(svar, ref_)),
                                    ))
                                }
                                // | otherwise -> "cannot use #new or #old with types"
                                RefTarget::RefType(_) => pretty_error_at(
                                    span,
                                    Doc::text("cannot use #new or #old with types"),
                                ),
                            }
                        }
                        // _ -> prettyErrorIn pat "unexpected extra argument(s)"
                        _ => pretty_error_at(span, Doc::text("unexpected extra argument(s)")),
                    }
                }
            }
            // App s (StringPrefix a b) [pat] — exactly one argument
            // (other arities fall to the invalid-pattern arm, mirroring
            // the source's fall-through).
            head @ SourcePat_::StringPrefix(..) => {
                if args.len() == 1 {
                    let SourcePat_::StringPrefix(a, b) = head else {
                        unreachable!("matched StringPrefix above")
                    };
                    let pat2 = resolve_pat(ctx, args.into_iter().next().expect("len checked"))?;
                    Ok(SourcePat_::App(
                        s,
                        Box::new(SourcePat_::StringPrefix(a, b)),
                        vec![pat2],
                    ))
                } else {
                    // App _ f _ -> prettyErrorIn pat $ "invalid pattern:
                    // cannot apply " <> describe f <> " to arguments"
                    pretty_error_at(
                        span,
                        Doc::text("invalid pattern: cannot apply ")
                            .append(head.describe())
                            .append(Doc::text(" to arguments")),
                    )
                }
            }
            // App _ f _ -> prettyErrorIn pat $ "invalid pattern: cannot
            // apply " <> describe f <> " to arguments"
            head => pretty_error_at(
                span,
                Doc::text("invalid pattern: cannot apply ")
                    .append(head.describe())
                    .append(Doc::text(" to arguments")),
            ),
        },
        SourcePat_::FieldSelect(s, pat, field, q) => Ok(SourcePat_::FieldSelect(
            s,
            Box::new(resolve_pat(ctx, *pat)?),
            field,
            q,
        )),
        SourcePat_::Deref(s, pat) => Ok(SourcePat_::Deref(
            s,
            Box::new(resolve_pat(ctx, *pat)?),
        )),
        SourcePat_::Enum(s, n) => Ok(SourcePat_::Enum(s, n)),
        SourcePat_::Clause(..) | SourcePat_::Prim(..) => {
            unreachable!("Clause/Prim hoisted above the match")
        }
    }
}

/// `internal = throwError $ "internal: unexpected: " <> Text.pack (show
/// (pretty pat))` (Resolve.hs:772-774).
fn internal_error(pat: &SourcePat_<SrcSpan, SrcSpan, SourceRef, SourceRef>) -> String {
    format!("internal: unexpected: {}", show_doc(&display_default(pat)))
}

/// `resolveTypeOrPred span txt = do scope <- getScope; let ref = parseRef
/// txt; case lookupResultToEither ref $ resolveRef scope ref of Left s ->
/// prettyErrorAt span (pretty s); Right r -> return r`
/// (Resolve.hs:776-786). `pretty s` on the error Text is raw text.
/// parse_ref's digit-string panic (sibling util.rs's BUG(port)) fires
/// here at the same forced point the Haskell forces it.
fn resolve_type_or_pred<P, T>(
    ctx: &ResolveCtx<'_, P, T>,
    span: SrcSpan,
    txt: &Name,
) -> Result<RefTarget<P, T>, String>
where
    P: Clone + ShowRef,
    T: Clone + ShowRef,
{
    let scope = ctx.get_scope();
    let ref_ = parse_ref(txt);
    match lookup_result_to_either(&ref_, resolve_ref(scope, &ref_)) {
        Err(s) => pretty_error_at(span, Doc::text(s)),
        Ok(r) => Ok(r),
    }
}

/// `prettyError = throwError . Text.pack . show` (Resolve.hs:788-789).
fn pretty_error<T>(doc: Doc) -> Result<T, String> {
    Err(show_doc(&doc))
}

/// `prettyErrorIn pat doc = prettyErrorAt (sourcePatSpan pat) doc`
/// (Resolve.hs:791-792) — inlined at its two call sites (the span is
/// read before the match consumes the pattern); the shape is preserved
/// via pretty_error_at(span, doc) with the whole-pattern span.
///
/// `prettyErrorAt span doc = prettyError $ vcat [ pretty span, doc ]`
/// (Resolve.hs:794-798) — `pretty span` is the Display for SrcSpan.
fn pretty_error_at<T>(span: SrcSpan, doc: Doc) -> Result<T, String> {
    pretty_error(vcat([display_default(&span), doc]))
}

/// `resolveStatement (SourceStatement l r) = SourceStatement <$>
/// resolvePat l <*> resolvePat r` (Resolve.hs:800-805).
fn resolve_statement<P, T>(
    ctx: &ResolveCtx<'_, P, T>,
    stmt: SourceStatement_<SrcSpan, SrcSpan, SourceRef, SourceRef>,
) -> Result<SourceStatement_<SrcSpan, SrcSpan, P, T>, String>
where
    P: Clone + ShowRef,
    T: Clone + ShowRef,
{
    Ok(SourceStatement_(
        resolve_pat(ctx, stmt.0)?,
        resolve_pat(ctx, stmt.1)?,
    ))
}

/// `primitives = HashMap.fromList [...]` (Resolve.hs:807-824) — a lookup
/// table; a match over the same 15 pairs in source order is the direct
/// Rust analog (RULE §0: same pairs, no allocation).
fn primitive(txt: &str) -> Option<PrimOp> {
    Some(match txt {
        "prim.toLower" => PrimOp::PrimOpToLower,
        "prim.length" => PrimOp::PrimOpLength,
        "prim.relToAbsByteSpans" => PrimOp::PrimOpRelToAbsByteSpans,
        "prim.unpackByteSpans" => PrimOp::PrimOpUnpackByteSpans,
        "prim.zip" => PrimOp::PrimOpZip,
        "prim.concat" => PrimOp::PrimOpConcat,
        "prim.reverse" => PrimOp::PrimOpReverse,
        "prim.size" => PrimOp::PrimOpSize,
        "prim.gtNat" => PrimOp::PrimOpGtNat,
        "prim.geNat" => PrimOp::PrimOpGeNat,
        "prim.ltNat" => PrimOp::PrimOpLtNat,
        "prim.leNat" => PrimOp::PrimOpLeNat,
        "prim.neNat" => PrimOp::PrimOpNeNat,
        "prim.addNat" => PrimOp::PrimOpAddNat,
        "prim.neExpr" => PrimOp::PrimOpNeExpr,
        _ => return None,
    })
}

// TODO(port): `show (pretty …)` render parity — pretty_error_at,
// internal_error and their call sites render Docs via prettyprinter
// 1.7.1's `Show (Doc ann)`: renderShowS (layoutPretty
// defaultLayoutOptions doc), 80-column Wadler layout. That surface is
// UNOWNED in the manifest (display.rs's open render TODO); this private
// flat-branch stand-in duplicates the sibling evolve.rs show_doc (also
// private there — no shared owner exists yet) and renders the FLAT
// branch of every group: error text matches the old binary exactly
// whenever the flat form fits 80 columns, differing only in wrapping
// otherwise. When display.rs's renderer lands, delete this copy and
// route both units through it.
fn show_doc(doc: &Doc) -> String {
    fn go(doc: &Doc, out: &mut String) {
        match doc {
            Doc::Empty | Doc::Fail => {}
            Doc::Text(s) => out.push_str(s),
            Doc::Line => out.push('\n'),
            Doc::Cat(a, b) => {
                go(a, out);
                go(b, out);
            }
            Doc::Nest(_, x) => go(x, out), // indentation is inert on the flat branch
            Doc::Column(f) => {
                // current column = chars since the last newline
                let col = out.chars().rev().take_while(|c| *c != '\n').count() as i64;
                go(&f(col), out);
            }
            Doc::Nesting(f) => go(&f(0), out), // flat branch nests at level 0
            Doc::Group(flat, _) => go(flat, out),
        }
    }
    let mut out = String::new();
    go(doc, &mut out);
    out
}

// ---------------------------------------------------------------------------
// Marker breakdown (RULE §6: reviewers check the trailer against this)
// ---------------------------------------------------------------------------
//
// Open TODO(port) markers: 4
//   1. Parser assumed-import integration (at the `use crate::angle::parser`):
//      the Alex/Happy parser unit (Lexer.x/Parser.y owner) does not exist
//      yet; the three signatures are pinned in the header contract
//      comment. The file cannot compile until that unit lands.
//   2. show_doc (at the fn): `show (pretty …)` render parity — flat-branch
//      stand-in until display.rs's unowned render surface lands; private
//      duplicate of sibling evolve.rs's show_doc (no shared owner yet).
//   3. Hash-order error selection (in resolve_schema_refs at
//      schema_by_name): the closed three-site set (ambiguous-schema
//      candidate list order; which "multiple definitions" error first;
//      which "inherited schemas give multiple definitions" error first).
//      Single-failure outputs are byte-identical; each site mirrors a
//      HashMap/HashSet toList the source itself iterates in hash order.
//   4. SchemaParserCache's Fingerprint key (at the type alias): the cache
//      is a HashMap keyed by Hash (= Fingerprint), but the sibling hash.rs
//      exposes hash_fingerprint() as a function and defines no
//      `impl std::hash::Hash for Fingerprint` — who owns that impl
//      (hash.rs beside its hashable-chain TODO, or a newtype here) is a
//      cross-unit question deliberately left to the hash-chain owner and
//      the Step-4 wiring survey; NOT implemented in this unit.
//
// BUG(port) markers: 1 — check_namespace (at the fn): mapM_ over the
// splitDot PAIR checks only the LAST dot-segment (base's Foldable ((,)
// a) folds the tail); reserved words in non-final namespace segments go
// undetected. Verified against this tree's GHC 9.4.7 (mapM_ over splitDot
// "if.a" visits only "a"); reproduced bug-for-bug; repro in the comment.
// Fix ships only as a flagged post-parity change.
//
// No PERF(port): the clone-heavy scope/re-export rebuilds are the shared
// first-pass norm (RULE §0 stand-in for Haskell's immutable sharing; a
// borrowing pass would avoid them). No SAFETY markers: no unsafe code.
//
// Partial functions (not markers): resolve_type's dead `_ -> error
// "resolveType"` arm panics with the source's exact message (RULE §2);
// check_field_name's Text.head panics with text-2.0.2's exact message —
// "Data.Text.Internal.Fusion.Common.head: Empty stream" (NOT text-1.x's
// "Data.Text.head: empty input"; verified under this tree's GHC 9.4.7) —
// on the empty string (unreachable from the parser; strictly forced in
// the source too).
//
// Coverage: every export of the Haskell module has a Rust counterpart —
// parseAndResolveSchema, parseAndResolveSchemaCached (+ SchemaParserCache),
// resolveType, resolveSchema, runResolve, resolveQuery, resolveSchemaRefs;
// plus the module-private machinery: break_substring/cut_schemas,
// strongly_conn_comp (+ Scc), union_name_env_left/union_name_env_with_sets/
// union_name_envs/qualify/qualify_name_env/name_entries, resolve_one_schema
// (resolve_one_schema1 is its catchError inSchema-wrapped body — the
// wrapper applies the "In <schema>:\n  " prefix exactly as catchError
// does), go_ref/go_field/check_fields, lookup_result_to_except,
// check_angle_version/check_field_name/check_name/check_namespace,
// reserved_words (is_reserved_word), ResolveCtx/get_scope,
// resolve_deriving, resolve_pat (all 27 arms; the Clause/Prim internal
// arms hoisted above the match with a comment; the App arms keep the
// source's primop-guard / single-arg / StringPrefix-single-arg /
// invalid-pattern order), internal_error, resolve_type_or_pred,
// pretty_error/pretty_error_at (pretty_error_in inlined at its call
// sites after the span read — noted at the fn), resolve_statement,
// primitive (the 15-entry table), show_doc. resolve_type / resolve_query /
// run_resolve / ResolveCtx (and the private resolve_pat /
// resolve_statement / resolve_deriving / go_ref / go_field /
// resolve_type_or_pred / lookup_result_to_except beneath them) keep the
// source's (ShowRef p, ShowRef t) generics — bounds P: Clone + ShowRef,
// T: Clone + ShowRef — mirroring the sibling lookup_result_to_either;
// this unit instantiates them only at RefResolved (header note: the
// glean-db UserQuery unit instantiates at RefTarget PredicateId TypeId).
// Field orders and error
// strings are byte-for-byte (the ResolvedSchema literal is in declaration
// order). Tests are not translated (RULE §4; the judge is the referee).
//
// MODULE-WIRING: lib.rs / schema/mod.rs scaffolding is deliberately NOT
// a manifest unit; owned by the Step-4 survey build (do not create
// ad-hoc here) — same note as the sibling units.
// PORT STATUS: confidence=high todos=4
