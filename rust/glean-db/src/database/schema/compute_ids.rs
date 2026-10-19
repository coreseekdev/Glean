// Ported from: glean/db/Glean/Database/Schema/ComputeIds.hs (342 lines).
// Manifest unit: rust/glean-db/src/database/schema/compute_ids.rs
// (migration/manifest.tsv line 66, crate glean-db). RULE §4: one source
// file -> one target file; the manifest path is authoritative.
//
// WHAT THIS FILE IS: the schema hasher. It takes the resolved schemas
// (ResolvedSchemaRef), assigns every type and predicate a PredicateId /
// TypeId, substitutes those ids for every PredicateRef / TypeRef inside
// every definition, and produces a HashedSchema: the id-keyed definition
// maps, the ref->id environment, the name environment, the "all" schema
// version, and the schema id. The ids are hashes of the definitions'
// Data.Binary encodings (:268-273, :340-342), so this unit is on the
// schema-compatibility linchpin: the computed SchemaId must equal what the
// old implementation stored (inventory.tsv:27).
//
// ---------------------------------------------------------------------------
// INVENTORY (RULE §5; migration/inventory*.tsv)
// ---------------------------------------------------------------------------
//
// GOVERNING ROW — migration/inventory.tsv:27 (HIDDEN_EFFECT, confirmed).
// Verbatim (TABs shown as \t):
//
//   glean/db/Glean/Database/Schema/ComputeIds.hs\tfingerprintDef\tData.Binary
//   encode -> fingerprint feeds schemaId\tHIDDEN_EFFECT\tserialize with
//   hand-written Data.Binary-format encoder (RULEBOOK 2 binary-fmt row), then
//   fingerprint; schema_id MUST equal old getSchemaInfo output\t
//   ComputeIds.hs:147,159 hash versions; :195 makeSchemaEnv -> schemaId;
//   :204 hashedSchemaId; validated by ...
//
// The four cited lines match the current tree: :147 is
// updateDefWithHash (fingerprintDef resolved) in the acyclic branch, :159 is
// map fingerprintDef defs1 in the cyclic branch, :195 is the makeSchemaEnv
// binding, :204 is hashedSchemaId. The row is followed: every hash goes
// through glean-angle's hand-written Binary encoders (hash.rs / types.rs),
// and the end-to-end parity obligation is marked at hash_name_env.
//
// ZERO rows in inventory.sites.tsv, inventory.stm.tsv,
// inventory.protocol.tsv, inventory.ffi.tsv and inventory.codeflow.tsv name
// this file. Exact command at the repo root:
//
//   $ grep -n 'ComputeIds' migration/inventory*.tsv
//
// returns inventory.tsv:27 plus four COMMENT/evidence lines:
// inventory.tsv:4 (the Hash.hs row's evidence names the ComputeIds
// fingerprint consumer), inventory.tsv:6 (the Angle/Types row's evidence
// names the ComputeIds hash chain), inventory.codeflow.tsv:2 (a header
// comment naming the row) and inventory.codeflow.tsv:21 (the Angle/Types
// Binary-instances row, same evidence). None of the four is a governing row
// for this file; rows 4 and 6 govern the sibling encoder units.
//
// No INVENTORY-BUG-CANDIDATE applies: the row is followed, and its line
// citations are current.
//
// ---------------------------------------------------------------------------
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, checked not assumed)
// ---------------------------------------------------------------------------
//
//   * OUTBOUND (4), all shipped in crate glean-angle:
//       edges.tsv:384 Angle/Hash.hs        -> row 2,  angle/hash.rs
//       edges.tsv:385 Angle/Types.hs       -> row 4,  angle/types.rs
//       edges.tsv:386 Angle/Schema/Types.hs-> row 6,  schema/types.rs
//       edges.tsv:387 Angle/Schema/Util.hs -> row 5,  schema/util.rs
//   * INBOUND (6), all future:
//       edges.tsv:221  Database/Config.hs      (manifest line 85)
//       edges.tsv:247  Database/Create.hs      (future)
//       edges.tsv:371  Database/Schema.hs      (manifest line 98)
//       edges.tsv:393  Database/Schema/Types.hs(manifest line 89)
//       edges.tsv:1557 schema/gen/.../Main.hs  (future)
//       edges.tsv:1630 shell/Glean/Shell.hs    (future)
//     They consume HashedSchema's public fields and empty_hashed_schema;
//     see SIBLING-UNIT CONTRACTS.
//   * The remaining imports are base: Control.Monad, Control.Monad.State,
//     Data.Bifunctor, Data.Bifoldable, Data.Graph, Data.HashMap.Strict,
//     Data.List, Data.Map, Data.Set, Data.Text, and the generated
//     Glean.Types (:33, an assumed import).
//
// ---------------------------------------------------------------------------
// ASSUMED IMPORTS (RULE §4 B4)
// ---------------------------------------------------------------------------
//
//   * glean_thrift::{PredicateRef, TypeRef, SchemaId, Version}. Pinned
//     shapes (glean/if/glean.thrift):
//         typedef i32 Version;                              // :39
//         struct PredicateRef { 1: PredicateName name; 2: Version version; }
//         struct TypeRef      { 1: TypeName name;      2: Version version; }
//         typedef string SchemaId (hs.newtype);             // :91
//     Rust images recorded at angle/types.rs:52-70 and
//     backend/types.rs:160-165: refs are Clone + Debug + Eq + Ord + Hash
//     with fields 'name'/'version'; SchemaId is String.
//   * The glean-angle sibling APIs consumed here, all shipped:
//       hash.rs: hash0, hash_binary, hash_byte_string, Hash.
//       angle/types.rs: Binary (trait + impls), encode_list, rm_loc_type,
//         rm_loc_query, Type_, TypeDef, PredicateDef, PredicateId, TypeId,
//         TypeDef_, PredicateDef_, DerivingInfo, DeriveWhen, Query_,
//         SourceRef, Name, Version.
//       schema/types.rs: RefTarget (with bimap), RefResolved,
//         ResolvedSchema, ResolvedSchemaRef, ResolvedPredicateDef,
//         ResolvedTypeDef, ResolvedQuery, NameEnv, map_name_env.
//       schema/util.rs: ShowRef (show_ref).
//
// CROSS-UNIT CORRECTION (sibling comment): angle/types.rs:132-137 (the
// '[Char]' note) and :362-369 (the encode_char_string doc) said ComputeIds
// hashes 'showRef ref :: String' and must use encode_char_string. The source declares
// 'showRef :: t -> Text' (Schema/Util.hs:88-89), so the value is Text and
// its Binary instance is the byte-count + UTF-8 encoding, which is this
// crate's 'impl Binary for String' (types.rs:246-251, text-2.1.1
// Data/Text.hs:389-390). encode_char_string is the Haskell '[Char]'
// encoding (char count + putStringUtf8) and is NOT used here. For ASCII
// names the two encodings coincide; for non-ASCII they do not, and Text is
// the source's type. The two sibling comments are corrected by this unit.
//
// ---------------------------------------------------------------------------
// SIBLING-UNIT CONTRACTS
// ---------------------------------------------------------------------------
//
//   * Hash (row 2): Hash = Fingerprint (two u64s), Display = 32 lowercase
//     hex digits. 'hashNameEnv' renders it with to_string into SchemaId.
//   * Angle/Types (row 4): the Data.Binary encoders and the bimap /
//     bifold_map / DerivingInfo::map walks this unit uses.
//   * Schema/Types (row 6): RefTarget, ResolvedSchema, NameEnv.
//   * Schema/Util (row 5): show_ref for PredicateRef/TypeRef.
//   * Config (row 85, future): 'procSchemaHashed :: HashedSchema'
//     (Config.hs:285) stores this unit's result.
//   * Create (future): 'HashedSchema{..} = procSchemaHashed proc'
//     (Create.hs:274) destructures the public fields.
//   * Database/Schema.hs (row 98, future): versions/typecheckSchema read
//     hashedSchemaId, hashedSchemaAllVersion, schemaRefToIdEnv,
//     hashedPreds, hashedTypes, hashedSchemaEnv (Schema.hs:186, :282,
//     :623, :807-811).
//   * Shell (future): emptyHashedSchema and the public fields
//     (Shell.hs:80, :255, :1376).
//
// ---------------------------------------------------------------------------
// EXPORT MAPPING (8/8 source exports, :10-18)
// ---------------------------------------------------------------------------
//
//   :11 computeIds          -> pub fn compute_ids
//   :12 RefToIdEnv(..)      -> pub struct RefToIdEnv (pub fields)
//   :13 emptyRefToIdEnv     -> pub fn empty_ref_to_id_env
//   :14 ResolvedSchemaId    -> pub type ResolvedSchemaId
//   :15 refsToIds           -> pub fn refs_to_ids (see COLLAPSE below)
//   :16 HashedSchema(..)    -> pub struct HashedSchema (pub fields)
//   :17 emptyHashedSchema   -> pub fn empty_hashed_schema
//   :18 RefTargetId         -> pub type RefTargetId
//
// Private, as in the source: attachDerivations, isDefaultDeriving,
// resolveQuery, fingerprintDef, makeSchemaEnv, hashNameEnv, and
// computeIds' where-block helpers (updateDefWithHash, extend, extends,
// resolveDef). Rust-only private aliases: DefResolved, Def, and the SCC
// helper (strongly_conn_comp, Component).
//
// COLLAPSE: the source's 'refsToIds :: Bifunctor f => ...' is polymorphic
// over RefTarget, Type_, SourceQuery_ and Maybe/Vec inside them. Rust has
// no Bifunctor trait in this crate (the sibling chose inherent bimap
// methods), so the image is three monomorphic helpers: the public
// refs_to_ids (RefTarget), private refs_to_ids_type (Type_), and private
// resolve_query (Query_). The SourceQuery_ bimap already walks its Maybe
// head and Vec statements. No external unit calls refsToIds (grep-verified:
// only ComputeIds.hs uses it), so the public shape is this module's choice.
//
// ---------------------------------------------------------------------------
// FIDELITY NOTES
// ---------------------------------------------------------------------------
//
//   * computeIds' first parameter, 'toList :: forall k v. HashMap k v ->
//     [(k,v)]', is a TEST SEAM: production callers pass HashMap.toList and
//     the tests pass a deterministic order. RULE §4 does not translate
//     tests, and every result is order-independent (hashes are per-def; the
//     output maps are unordered; cycles are sorted before hashing), so the
//     parameter is collapsed and the implementation iterates the maps
//     directly. This is the catalog.rs test-genericity precedent.
//   * State RefToIdEnv -> an explicit 'env' threaded by & and &mut. The
//     cyclic branch's 'withState' is a clone of env, seeded with the cycle
//     at hash0, used only to resolve the first pass; the outer env is then
//     updated with the final hashes (the source's State.modify).
//   * Data.Graph.stronglyConnComp -> strongly_conn_comp (Tarjan). The
//     source relies on dependencies being returned before dependents (a
//     dependent's refs must already be in env); Tarjan emits an SCC only
//     after every SCC reachable from it, so its output has the same
//     dependency-first property. Order among independent SCCs is not
//     observable (per-def hashes; unordered output maps). A single node is
//     Acyclic unless it has a self-edge, matching Data.Graph's self-loop
//     rule. graphFromEdges ignores edges to keys not in the vertex list;
//     the BTreeMap lookup here does the same. Duplicate keys are undefined
//     in graphFromEdges; this port keeps the first, and valid schemas have
//     unique refs.
//   * 'sortOn (bimap predicateDefRef typeDefRef) defs' -> the stable
//     'sort_by_key(def_ref)', where def_ref clones only the ref (the source
//     consumes the def through bimap, but the defs are needed afterwards).
//   * HashMap semantics, clause by clause:
//       - HashMap.fromList: later entries win (insert loop / collect).
//       - HashMap.unions (attachDerivations): left-biased (first schema's
//         derivation wins) -> or_insert_with.
//       - HashMap.union (makeSchemaEnv): left-biased (the mapped qualified
//         scope wins over the versioned refs) -> or_insert on the result.
//       - versionedNameEnv itself is fromList over predicates ++ types:
//         last wins. The source's order within each half is HashMap order
//         and is unspecified; a name that is both a predicate and a type at
//         the same version is therefore unspecified there too. This port
//         builds the versioned map with last-wins inserts and then applies
//         the left-biased union, which is faithful whenever the source is
//         deterministic (valid schemas: names are unique per namespace).
//       - attachDefaultDerivings is foldr: for a repeated id the FIRST list
//         element wins. The list is schema-major, so the earliest schema's
//         default deriving wins; 'seen' reproduces that.
//   * Binary encodings on the hash chain:
//       - fingerprintDef: hash_binary over (show_ref, rm_loc_type ...) and
//         the 4-tuple with the deriving. show_ref is Text (see CROSS-UNIT
//         CORRECTION); rm_loc_* strip spans to ().
//       - cycleHash: hashBinary hashes where hashes :: [Hash] is
//         hash_byte_string(encode_list(hashes)) (the list encoding is
//         Int64be length + elements).
//       - hashNameEnv: hashBinary (sort (HashMap.toList env)). The outer
//         list is Int64be length + pairs; each pair is SourceRef bytes
//         followed by the Set. Data.Binary's Set instance is the ascending
//         element list (Int64be size + elements), so encode_set uses
//         encode_list over the BTreeSet's ascending iteration.
//   * Partial functions -> panics with the SAME messages (RULE §2):
//     lookupPredicateId/lookupTypeId use 'error' in the source and panic
//     here; makeSchemaEnv's 'error "no \"all\" schema"' panics with the
//     same text.
//   * 'fromIntegral ver' (makeSchemaEnv) is an identity: Version = i32 on
//     both sides.
//   * RefToIdEnv derives Clone for the withState clone; the source needs no
//     instance (immutable sharing). No other derive is added: HashedSchema
//     and RefToIdEnv have no deriving clause in the source.
//   * No unsafe code.
//
// MARKERS: one port todo (the schema-id parity validation at hash_name_env,
// the governing row's acceptance criterion). Zero source-defect
// reproductions, zero performance deferrals, zero inventory divergences.
//
// PORT STATUS: confidence=medium todos=1

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use glean_angle::angle::hash::{hash0, hash_binary, hash_byte_string, Hash};
use glean_angle::angle::types::{
    encode_list, rm_loc_query, rm_loc_type, Binary, DeriveWhen, DerivingInfo, PredicateDef,
    PredicateDef_, PredicateId, Query_, SourceRef, TypeDef, TypeDef_, TypeId, Type_, Version,
};
use glean_angle::schema::types::{
    map_name_env, NameEnv, RefResolved, RefTarget, ResolvedPredicateDef, ResolvedQuery,
    ResolvedSchema, ResolvedSchemaRef, ResolvedTypeDef,
};
use glean_angle::schema::util::ShowRef;
use glean_thrift::{PredicateRef, SchemaId, TypeRef};

/// 'type RefTargetId = RefTarget PredicateId TypeId' (:49).
pub type RefTargetId = RefTarget<PredicateId, TypeId>;

/// 'type ResolvedSchemaId = ResolvedSchema PredicateId TypeId' (:72).
pub type ResolvedSchemaId = ResolvedSchema<PredicateId, TypeId>;

/// 'type DefResolved = RefTarget ResolvedPredicateDef ResolvedTypeDef'
/// (:62). Private in the source.
type DefResolved = RefTarget<ResolvedPredicateDef, ResolvedTypeDef>;

/// 'type Def = RefTarget PredicateDef TypeDef' (:63). Private in the source.
type Def = RefTarget<PredicateDef, TypeDef>;

/// 'data HashedSchema' (:40-47). Every field is exported by the source
/// (:16), hence pub.
pub struct HashedSchema {
    pub hashed_types: HashMap<TypeId, TypeDef>,
    pub hashed_preds: HashMap<PredicateId, PredicateDef>,
    pub schema_ref_to_id_env: RefToIdEnv,
    pub hashed_schema_env: NameEnv<RefTargetId>,
    pub hashed_schema_all_version: Version,
    pub hashed_schema_id: SchemaId,
}

/// 'emptyHashedSchema' (:51-59).
pub fn empty_hashed_schema() -> HashedSchema {
    HashedSchema {
        hashed_types: HashMap::new(),
        hashed_preds: HashMap::new(),
        schema_ref_to_id_env: empty_ref_to_id_env(),
        hashed_schema_env: HashMap::new(),
        hashed_schema_all_version: 0,
        hashed_schema_id: String::new(),
    }
}

/// 'data RefToIdEnv' (:66-69). Clone is the Rust stand-in for the source's
/// free sharing in withState (see FIDELITY NOTES).
#[derive(Clone)]
pub struct RefToIdEnv {
    pub type_ref_to_id: HashMap<TypeRef, TypeId>,
    pub pred_ref_to_id: HashMap<PredicateRef, PredicateId>,
}

/// 'emptyRefToIdEnv' (:71).
pub fn empty_ref_to_id_env() -> RefToIdEnv {
    RefToIdEnv {
        type_ref_to_id: HashMap::new(),
        pred_ref_to_id: HashMap::new(),
    }
}

/// 'refsToIds' (:76-89), RefTarget instantiation. The lookup helpers are
/// the source's where-block functions; their 'error' calls become panics
/// with the same message.
pub fn refs_to_ids(env: &RefToIdEnv, r: RefResolved) -> RefTargetId {
    r.bimap(
        &mut |p| lookup_predicate_id(&env.pred_ref_to_id, &p),
        &mut |t| lookup_type_id(&env.type_ref_to_id, &t),
    )
}

fn lookup_predicate_id(m: &HashMap<PredicateRef, PredicateId>, p: &PredicateRef) -> PredicateId {
    match m.get(p) {
        Some(id) => id.clone(),
        None => panic!("lookupPredicateId: {}", p.show_ref()),
    }
}

fn lookup_type_id(m: &HashMap<TypeRef, TypeId>, p: &TypeRef) -> TypeId {
    match m.get(p) {
        Some(id) => id.clone(),
        None => panic!("lookupTypeId: {}", p.show_ref()),
    }
}

/// The Type_ instantiation of refsToIds (the source's Bifunctor f = Type_).
fn refs_to_ids_type<ST>(
    env: &RefToIdEnv,
    ty: Type_<ST, PredicateRef, TypeRef>,
) -> Type_<ST, PredicateId, TypeId> {
    ty.bimap(
        &mut |p| lookup_predicate_id(&env.pred_ref_to_id, &p),
        &mut |t| lookup_type_id(&env.type_ref_to_id, &t),
    )
}

/// 'attachDerivations' (:91-103): apply each schema's non-default
/// derivations to the predicate definitions it names, using the union of
/// all schemas' deriving maps (left-biased, :96).
fn attach_derivations(
    schemas: &[ResolvedSchemaRef],
) -> Vec<HashMap<PredicateRef, ResolvedPredicateDef>> {
    let mut all_derivings: HashMap<PredicateRef, DerivingInfo<ResolvedQuery>> = HashMap::new();
    for schema in schemas {
        for (r, drv) in &schema.resolved_schema_deriving {
            all_derivings
                .entry(r.clone())
                .or_insert_with(|| drv.deriving_def_derive_info.clone());
        }
    }

    schemas
        .iter()
        .map(|schema| {
            schema
                .resolved_schema_predicates
                .iter()
                .map(|(r, def)| {
                    let mut def = def.clone();
                    if let Some(drv) = all_derivings.get(r) {
                        if !is_default_deriving(drv) {
                            def.predicate_def_deriving = drv.clone();
                        }
                    }
                    (r.clone(), def)
                })
                .collect()
        })
        .collect()
}

/// 'isDefaultDeriving' (:105-107): only 'Derive DeriveIfEmpty' counts.
fn is_default_deriving<Q>(info: &DerivingInfo<Q>) -> bool {
    matches!(info, DerivingInfo::Derive(DeriveWhen::DeriveIfEmpty, _))
}

/// 'computeIds' (:109-210). The 'toList' parameter is collapsed (see
/// FIDELITY NOTES).
pub fn compute_ids(
    schemas: &[ResolvedSchemaRef],
    versions: Option<(SchemaId, Version)>,
) -> HashedSchema {
    let preds = attach_derivations(schemas);

    // The dependency graph: (node, key, dependencies). Predicates first,
    // then types, exactly as the source's two list comprehensions (:122-131).
    let mut edges: Vec<(DefResolved, RefResolved, Vec<RefResolved>)> = Vec::new();
    for m in &preds {
        for def in m.values() {
            edges.push((
                RefTarget::RefPred(def.clone()),
                RefTarget::RefPred(def.predicate_def_ref.clone()),
                predicate_def_refs(def),
            ));
        }
    }
    for schema in schemas {
        for def in schema.resolved_schema_types.values() {
            edges.push((
                RefTarget::RefType(def.clone()),
                RefTarget::RefType(def.type_def_ref.clone()),
                type_def_refs(def),
            ));
        }
    }

    let mut env = empty_ref_to_id_env();
    let mut defs: Vec<Def> = Vec::new();

    for comp in strongly_conn_comp(&edges) {
        match comp {
            Component::Acyclic(i) => {
                let resolved = resolve_def(&env, edges[i].0.clone());
                let newdef = update_def_with_hash(fingerprint_def(&resolved), resolved);
                extend(&newdef, &mut env);
                defs.push(newdef);
            }
            Component::Cyclic(comp) => {
                // Sort by ref so the hashes are deterministic (:151-152).
                let mut sorted: Vec<DefResolved> =
                    comp.iter().map(|&i| edges[i].0.clone()).collect();
                sorted.sort_by_key(def_ref);

                // First pass: every ref in the cycle maps to hash0 (:154-157).
                let mut inner = env.clone();
                let seed: Vec<(DefResolved, Hash)> = comp
                    .iter()
                    .map(|&i| (edges[i].0.clone(), hash0()))
                    .collect();
                extends(&seed, &mut inner);

                let resolved: Vec<Def> = sorted
                    .iter()
                    .map(|d| resolve_def(&inner, d.clone()))
                    .collect();
                let hashes: Vec<Hash> = resolved.iter().map(fingerprint_def).collect();

                // hashBinary hashes: the list encoding (:159-161).
                let cycle_hash = hash_byte_string(&encode_list(&hashes));

                let hashes2: Vec<(DefResolved, Hash)> = sorted
                    .iter()
                    .cloned()
                    .zip(hashes.iter().map(|h1| hash_binary(&(*h1, cycle_hash))))
                    .collect();

                extends(&hashes2, &mut env);

                // Resolve again with the final ids (:174-175).
                for (def, hash) in hashes2 {
                    defs.push(update_def_with_hash(hash, resolve_def(&env, def)));
                }
            }
        }
    }

    // :178-181.
    let mut hashed_preds: HashMap<PredicateId, PredicateDef> = defs
        .iter()
        .filter_map(|def| match def {
            RefTarget::RefPred(p) => Some((p.predicate_def_ref.clone(), p.clone())),
            RefTarget::RefType(_) => None,
        })
        .collect();
    hashed_preds = attach_default_derivings(hashed_preds, schemas, &env);

    let hashed_types: HashMap<TypeId, TypeDef> = defs
        .iter()
        .filter_map(|def| match def {
            RefTarget::RefType(t) => Some((t.type_def_ref.clone(), t.clone())),
            RefTarget::RefPred(_) => None,
        })
        .collect();

    let (all_version, schema_id, schema_env) = make_schema_env(schemas, versions, &env);

    HashedSchema {
        hashed_types,
        hashed_preds,
        schema_ref_to_id_env: env,
        hashed_schema_env: schema_env,
        hashed_schema_all_version: all_version,
        hashed_schema_id: schema_id,
    }
}

// ---------------------------------------------------------------------------
// computeIds' where-block (:211-264)
// ---------------------------------------------------------------------------

/// 'updateDefWithHash' (:211-216).
fn update_def_with_hash(hash: Hash, def: Def) -> Def {
    match def {
        RefTarget::RefType(TypeDef_ {
            type_def_ref: TypeId { type_id_ref, .. },
            type_def_type,
            type_def_src_span,
        }) => RefTarget::RefType(TypeDef_ {
            type_def_ref: TypeId {
                type_id_ref,
                type_id_hash: hash,
            },
            type_def_type,
            type_def_src_span,
        }),
        RefTarget::RefPred(PredicateDef_ {
            predicate_def_ref: PredicateId {
                predicate_id_ref, ..
            },
            predicate_def_key_type,
            predicate_def_value_type,
            predicate_def_deriving,
            predicate_def_src_span,
        }) => RefTarget::RefPred(PredicateDef_ {
            predicate_def_ref: PredicateId {
                predicate_id_ref,
                predicate_id_hash: hash,
            },
            predicate_def_key_type,
            predicate_def_value_type,
            predicate_def_deriving,
            predicate_def_src_span,
        }),
    }
}

/// 'extend' (:218-225): insert one resolved definition's id by its ref.
fn extend(def: &Def, env: &mut RefToIdEnv) {
    match def {
        RefTarget::RefType(d) => {
            env.type_ref_to_id
                .insert(d.type_def_ref.type_id_ref.clone(), d.type_def_ref.clone());
        }
        RefTarget::RefPred(d) => {
            env.pred_ref_to_id.insert(
                d.predicate_def_ref.predicate_id_ref.clone(),
                d.predicate_def_ref.clone(),
            );
        }
    }
}

/// 'extends' (:227-237): insert a batch of (definition, hash) pairs by ref.
fn extends(list: &[(DefResolved, Hash)], env: &mut RefToIdEnv) {
    for (def, hash) in list {
        match def {
            RefTarget::RefType(d) => {
                env.type_ref_to_id.insert(
                    d.type_def_ref.clone(),
                    TypeId {
                        type_id_ref: d.type_def_ref.clone(),
                        type_id_hash: *hash,
                    },
                );
            }
            RefTarget::RefPred(d) => {
                env.pred_ref_to_id.insert(
                    d.predicate_def_ref.clone(),
                    PredicateId {
                        predicate_id_ref: d.predicate_def_ref.clone(),
                        predicate_id_hash: *hash,
                    },
                );
            }
        }
    }
}

/// 'resolveDef' (:239-256): substitute ids and seed the new id with hash0.
fn resolve_def(env: &RefToIdEnv, def: DefResolved) -> Def {
    match def {
        RefTarget::RefType(TypeDef_ {
            type_def_ref,
            type_def_type,
            type_def_src_span,
        }) => RefTarget::RefType(TypeDef_ {
            type_def_ref: TypeId {
                type_id_ref: type_def_ref,
                type_id_hash: hash0(),
            },
            type_def_type: refs_to_ids_type(env, type_def_type),
            type_def_src_span,
        }),
        RefTarget::RefPred(PredicateDef_ {
            predicate_def_ref,
            predicate_def_key_type,
            predicate_def_value_type,
            predicate_def_deriving,
            predicate_def_src_span,
        }) => RefTarget::RefPred(PredicateDef_ {
            predicate_def_ref: PredicateId {
                predicate_id_ref: predicate_def_ref,
                predicate_id_hash: hash0(),
            },
            predicate_def_key_type: refs_to_ids_type(env, predicate_def_key_type),
            predicate_def_value_type: refs_to_ids_type(env, predicate_def_value_type),
            predicate_def_deriving: predicate_def_deriving.map(&mut |q| resolve_query(env, q)),
            predicate_def_src_span,
        }),
    }
}

/// 'resolveQuery' (:258-263): the Query_ instantiation of refsToIds.
fn resolve_query(
    env: &RefToIdEnv,
    q: Query_<PredicateRef, TypeRef>,
) -> Query_<PredicateId, TypeId> {
    q.bimap(
        &mut |p| lookup_predicate_id(&env.pred_ref_to_id, &p),
        &mut |t| lookup_type_id(&env.type_ref_to_id, &t),
    )
}

// ---------------------------------------------------------------------------
// Dependency collection (:133-141)
// ---------------------------------------------------------------------------

/// 'collectRefs' (:133): bifoldMap ((:[]) . RefPred) ((:[]) . RefType).
fn collect_refs_type<ST>(ty: &Type_<ST, PredicateRef, TypeRef>) -> Vec<RefResolved> {
    let mut refs = Vec::new();
    ty.bifold_map(
        &mut refs,
        &mut |p: &PredicateRef, acc: &mut Vec<RefResolved>| acc.push(RefTarget::RefPred(p.clone())),
        &mut |t: &TypeRef, acc: &mut Vec<RefResolved>| acc.push(RefTarget::RefType(t.clone())),
    );
    refs
}

fn collect_refs_query(q: &ResolvedQuery) -> Vec<RefResolved> {
    let mut refs = Vec::new();
    q.bifold_map(
        &mut refs,
        &mut |p: &PredicateRef, acc: &mut Vec<RefResolved>| acc.push(RefTarget::RefPred(p.clone())),
        &mut |t: &TypeRef, acc: &mut Vec<RefResolved>| acc.push(RefTarget::RefType(t.clone())),
    );
    refs
}

/// 'predicateDefRefs' (:135-138).
fn predicate_def_refs(def: &ResolvedPredicateDef) -> Vec<RefResolved> {
    let mut refs = collect_refs_type(&def.predicate_def_key_type);
    refs.extend(collect_refs_type(&def.predicate_def_value_type));
    def.predicate_def_deriving.fold_map(
        &mut refs,
        &mut |q: &ResolvedQuery, acc: &mut Vec<RefResolved>| acc.extend(collect_refs_query(q)),
    );
    refs
}

/// 'typeDefRefs' (:140).
fn type_def_refs(def: &ResolvedTypeDef) -> Vec<RefResolved> {
    collect_refs_type(&def.type_def_type)
}

// ---------------------------------------------------------------------------
// Data.Graph.stronglyConnComp (base) -> Tarjan
// ---------------------------------------------------------------------------

enum Component {
    Acyclic(usize),
    Cyclic(Vec<usize>),
}

/// The dependency-first SCC decomposition of the edge list. See FIDELITY
/// NOTES for why Tarjan's emission order is the required one.
fn strongly_conn_comp(edges: &[(DefResolved, RefResolved, Vec<RefResolved>)]) -> Vec<Component> {
    let n = edges.len();

    // graphFromEdges: keys are unique (valid schemas); unknown edges are
    // dropped. BTreeMap because RefResolved has Ord but not Hash.
    let mut key_to_index: BTreeMap<RefResolved, usize> = BTreeMap::new();
    for (i, (_, key, _)) in edges.iter().enumerate() {
        key_to_index.entry(key.clone()).or_insert(i);
    }
    let adj: Vec<Vec<usize>> = edges
        .iter()
        .map(|(_, _, deps)| {
            deps.iter()
                .filter_map(|d| key_to_index.get(d).copied())
                .collect()
        })
        .collect();

    let mut index = vec![usize::MAX; n];
    let mut low = vec![0usize; n];
    let mut on_stack = vec![false; n];
    let mut stack: Vec<usize> = Vec::new();
    let mut next_index = 0usize;
    let mut comps: Vec<Vec<usize>> = Vec::new();

    struct Frame {
        v: usize,
        child: usize,
    }

    for start in 0..n {
        if index[start] != usize::MAX {
            continue;
        }
        index[start] = next_index;
        low[start] = next_index;
        next_index += 1;
        stack.push(start);
        on_stack[start] = true;

        let mut call = vec![Frame { v: start, child: 0 }];
        while let Some(frame) = call.last_mut() {
            let v = frame.v;
            if frame.child < adj[v].len() {
                let w = adj[v][frame.child];
                frame.child += 1;
                if index[w] == usize::MAX {
                    index[w] = next_index;
                    low[w] = next_index;
                    next_index += 1;
                    stack.push(w);
                    on_stack[w] = true;
                    call.push(Frame { v: w, child: 0 });
                } else if on_stack[w] {
                    low[v] = low[v].min(index[w]);
                }
            } else {
                call.pop();
                if let Some(parent) = call.last() {
                    low[parent.v] = low[parent.v].min(low[v]);
                }
                if low[v] == index[v] {
                    let mut comp = Vec::new();
                    loop {
                        let w = stack.pop().expect("SCC stack underflow");
                        on_stack[w] = false;
                        comp.push(w);
                        if w == v {
                            break;
                        }
                    }
                    comps.push(comp);
                }
            }
        }
    }

    comps
        .into_iter()
        .map(|comp| {
            if comp.len() == 1 && !adj[comp[0]].contains(&comp[0]) {
                Component::Acyclic(comp[0])
            } else {
                Component::Cyclic(comp)
            }
        })
        .collect()
}

/// The source's 'bimap predicateDefRef typeDefRef' sort key, without
/// consuming the definition.
fn def_ref(def: &DefResolved) -> RefResolved {
    match def {
        RefTarget::RefPred(d) => RefTarget::RefPred(d.predicate_def_ref.clone()),
        RefTarget::RefType(d) => RefTarget::RefType(d.type_def_ref.clone()),
    }
}

// ---------------------------------------------------------------------------
// fingerprintDef (:266-275)
// ---------------------------------------------------------------------------

/// Hash a definition's representation. The ref's rendering is Text
/// (CROSS-UNIT CORRECTION), and the spans are stripped with rm_loc_*.
fn fingerprint_def(def: &Def) -> Hash {
    match def {
        RefTarget::RefType(TypeDef_ {
            type_def_ref,
            type_def_type,
            ..
        }) => {
            let shown = type_def_ref.type_id_ref.show_ref();
            hash_binary(&(shown, rm_loc_type(type_def_type.clone())))
        }
        RefTarget::RefPred(PredicateDef_ {
            predicate_def_ref,
            predicate_def_key_type,
            predicate_def_value_type,
            predicate_def_deriving,
            ..
        }) => {
            let shown = predicate_def_ref.predicate_id_ref.show_ref();
            hash_binary(&(
                shown,
                rm_loc_type(predicate_def_key_type.clone()),
                rm_loc_type(predicate_def_value_type.clone()),
                predicate_def_deriving.clone().map(&mut |q| rm_loc_query(q)),
            ))
        }
    }
}

// ---------------------------------------------------------------------------
// attachDefaultDerivings (:183-193)
// ---------------------------------------------------------------------------

/// Re-apply the default derivations that fingerprintDef deliberately
/// ignores. foldr means the FIRST list element wins for a repeated id; the
/// list is schema-major, hence 'seen'.
fn attach_default_derivings(
    mut preds: HashMap<PredicateId, PredicateDef>,
    schemas: &[ResolvedSchemaRef],
    env: &RefToIdEnv,
) -> HashMap<PredicateId, PredicateDef> {
    let mut seen: HashSet<PredicateId> = HashSet::new();
    for schema in schemas {
        for (r, drv) in &schema.resolved_schema_deriving {
            let info = &drv.deriving_def_derive_info;
            if !is_default_deriving(info) {
                continue;
            }
            if let Some(id) = env.pred_ref_to_id.get(r) {
                if seen.insert(id.clone()) {
                    if let Some(def) = preds.get_mut(id) {
                        def.predicate_def_deriving =
                            info.clone().map(&mut |q| resolve_query(env, q));
                    }
                }
            }
        }
    }
    preds
}

// ---------------------------------------------------------------------------
// makeSchemaEnv (:295-338)
// ---------------------------------------------------------------------------

/// Pick the "all" schema, build its name environment, and compute the
/// schema id when the caller did not supply one.
fn make_schema_env(
    resolved: &[ResolvedSchemaRef],
    versions: Option<(SchemaId, Version)>,
    ref_to_id_env: &RefToIdEnv,
) -> (Version, SchemaId, NameEnv<RefTargetId>) {
    let mut resolved_alls: BTreeMap<Version, &ResolvedSchemaRef> = BTreeMap::new();
    for schema in resolved {
        if schema.resolved_schema_name == "all" {
            // Map.fromList: a later duplicate version wins.
            resolved_alls.insert(schema.resolved_schema_version, schema);
        }
    }

    if let Some((schema_id, ver)) = versions {
        if let Some(schema) = resolved_alls.get(&ver) {
            return (ver, schema_id, mk_env(schema, ref_to_id_env));
        }
    }

    if let Some((&ver, schema)) = resolved_alls.iter().next_back() {
        let env = mk_env(schema, ref_to_id_env);
        let schema_id = hash_name_env(&env).to_string();
        return (ver, schema_id, env);
    }

    panic!("no \"all\" schema")
}

/// 'mkEnv' (:313-320) plus versionedNameEnv (:330-338).
fn mk_env(schema: &ResolvedSchemaRef, ref_to_id_env: &RefToIdEnv) -> NameEnv<RefTargetId> {
    let mut env = map_name_env(
        |r: &RefResolved| Some(refs_to_ids(ref_to_id_env, r.clone())),
        &schema.resolved_schema_qual_scope,
    );

    // HashMap.fromList over predicates ++ types: last wins. Built separately
    // so the union below stays left-biased (FIDELITY NOTES).
    let mut versioned: HashMap<SourceRef, BTreeSet<RefTargetId>> = HashMap::new();
    for (r, id) in &ref_to_id_env.pred_ref_to_id {
        versioned.insert(
            SourceRef {
                source_ref_name: r.name.clone(),
                source_ref_version: Some(r.version),
            },
            BTreeSet::from([RefTarget::RefPred(id.clone())]),
        );
    }
    for (r, id) in &ref_to_id_env.type_ref_to_id {
        versioned.insert(
            SourceRef {
                source_ref_name: r.name.clone(),
                source_ref_version: Some(r.version),
            },
            BTreeSet::from([RefTarget::RefType(id.clone())]),
        );
    }

    // HashMap.union: the mapped qualified scope wins.
    for (k, v) in versioned {
        env.entry(k).or_insert(v);
    }
    env
}

/// 'hashNameEnv' (:340-342): hashBinary (sort (HashMap.toList env)).
fn hash_name_env(env: &NameEnv<RefTargetId>) -> Hash {
    let mut pairs: Vec<(&SourceRef, &BTreeSet<RefTargetId>)> = env.iter().collect();
    pairs.sort();

    // TODO(port): validation obligation — the governing row's acceptance
    // criterion is that this SchemaId equals the old getSchemaInfo output
    // over all 96 schema sources. The encoding below is derived from
    // binary-0.8.9.1 (list = Int64be length + elements; Set = ascending
    // elements with the same shape) and text-2.1.1, but the end-to-end
    // judge case is Step 4's.
    let mut bytes = (pairs.len() as i64).encode();
    for (source_ref, targets) in pairs {
        bytes.extend_from_slice(&source_ref.encode());
        bytes.extend_from_slice(&encode_set(targets));
    }
    hash_byte_string(&bytes)
}

/// Data.Binary's Set instance: the ascending element list.
fn encode_set<T: Binary + Clone>(set: &BTreeSet<T>) -> Vec<u8> {
    let xs: Vec<T> = set.iter().cloned().collect();
    encode_list(&xs)
}
