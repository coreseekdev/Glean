// Ported from: glean/angle/Glean/Schema/Types.hs (191 lines)
// Manifest unit: rust/glean-angle/src/schema/types.rs (crate glean-angle) —
// migration/manifest.tsv row 6. RULE §4 (naming): one source file → one
// target file; the manifest path is authoritative.
//
// INVENTORY (RULE §5): no row in any migration/inventory*.tsv names this
// unit (grep "Schema/Types", "RefTarget", "NameEnv", "SchemaRef" across
// inventory.tsv and the five per-area sweeps: zero rows — the only
// Types.hs row, inventory.tsv row 6, belongs to the SIBLING unit
// glean/angle/Glean/Angle/Types.hs). Nothing here diverges from a row
// because no row governs; no INVENTORY-BUG-CANDIDATE. The one coupling:
// this module's single Binary instance (`Binary (RefTarget p t)`,
// Schema/Types.hs:80) joins the hash chain that row 6's encoder module
// owns — see the impl below for the chain evidence and the inherited
// validation obligation.
//
// SIBLING CONTRACTS (RULE §4 assumed-imports; verified by reading the
// done siblings, never re-declared here):
//   - crate::angle::types owns Type_, FieldDef_, TypeDef_, PredicateDef_,
//     DerivingDef_, SourcePat_, SourceStatement_, SourceQuery_,
//     DerivingInfo, SrcSpan, Name, AngleVersion, the `Binary` trait (with
//     the pinned Data.Binary format contract: raw runPut stream, fixed
//     words BIG-ENDIAN, generic-derived sums = one-byte Word8 ctor tag =
//     0-based left-to-right declaration index, then fields in order), and
//     the pub re-exports `PredicateRef`/`TypeRef`/`Version` from
//     glean_thrift. This file only `use`s them.
//   - crate::schema::util owns ShowRef (show_ref) and re-exports SourceRef
//     (mirroring Util.hs's re-export, which this module's Haskell import
//     list rides on: `import Glean.Schema.Util`).
//
// Naming (RULE §0 structure-preserving + sibling conventions):
//   - Haskell positional constructors stay tuple structs (sibling
//     precedent: `Field(pub FieldName, pub SourcePat_)`).
//   - `deriving Show` → `#[derive(Debug)]` is the port-wide convention
//     (types.rs header); types whose Haskell declaration derived NOTHING
//     (LookupResult, ResolvedSchema, ResolvedSchemas) still get
//     derive(Debug) under RULE §2's deriving-Show row ("constructor-shaped
//     naming is still the default" for types with no Show), plus Clone
//     (the mechanical stand-in for Haskell's free immutable sharing).
//     Eq/Ord are derived ONLY where the source derived them.
//   - Primed Haskell aliases (`ResolvedPat'` …) cannot keep `'` in Rust.
//     Sibling types.rs's convention names an alias after what it fixes
//     (WithRefs/WithIds); the primed family here instead FREES the span
//     parameters (the unprimed names fix spans = SrcSpan), so they carry
//     the suffix `WithSpans` naming their one remaining free parameter.

use std::collections::{BTreeSet, HashMap};

// RULE §4 (imports): `use crate::…` within the crate (manifest path
// layout; same note as sibling util.rs on the rulebook's literal
// `crate::glean::…` template). Angle.Types import:
use crate::angle::types::{
    AngleVersion, Binary, DerivingDef_, DerivingInfo, FieldDef_, Name, PredicateDef_, SourcePat_,
    SourceQuery_, SourceStatement_, SrcSpan, Type_, TypeDef_,
};
// thrift types re-exported by the sibling types.rs (RULE §0: the source
// reached them through its Glean.Angle.Types import).
use crate::angle::types::{PredicateRef, TypeRef, Version};
// Schema.Util import: SourceRef comes in via util's re-export exactly as
// in the Haskell import list; ShowRef is util's own class.
use crate::schema::util::{ShowRef, SourceRef};

// ---------------------------------------------------------------------------
// Schema references
// ---------------------------------------------------------------------------

/// Identify a schema (source doc).
///
/// `data SchemaRef = SchemaRef Name Version deriving (Eq, Ord, Show,
/// Generic)` (Schema/Types.hs:67-68) — positional constructor, kept as a
/// tuple struct per the sibling convention. RULE §1 strings row:
/// `Name` = `Text` → `String`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SchemaRef(pub Name, pub Version);

/// `showSchemaRef :: SchemaRef -> Text` (Schema/Types.hs:70-72):
/// `name <> "." <> Text.pack (show version)`. `show` on the thrift
/// `Version` (Int32) is signed decimal — i32's Display. RULE §1 strings
/// row: Text → String. By-ref: the Haskell is pure; callers (e.g.
/// Evolve.hs:108-110 error text) render while still holding the schema.
pub fn show_schema_ref(schema_ref: &SchemaRef) -> String {
    format!("{}.{}", schema_ref.0, schema_ref.1)
}

// `instance Hashable SchemaRef` (Schema/Types.hs:74) is the bare
// generic-derived instance — no hashWithSalt override — so Rust's
// derived Hash above is the direct analog (the sibling only hand-rolls
// Hash where the source overrode hashWithSalt: PredicateId/TypeId).
// Consumers key ORDERED containers (Set/Map SchemaRef in Schema.hs:602,
// Evolve.hs:95), so the salt chain is never persisted; no parity
// obligation, no TODO.

// ---------------------------------------------------------------------------
// Name targets
// ---------------------------------------------------------------------------

/// The target of a reference (source doc).
///
/// `data RefTarget p t = RefPred p | RefType t deriving (Eq, Ord, Show,
/// Generic, Functor)` (Schema/Types.hs:77-78). RULE §2 deriving-Show row:
/// Debug keeps the constructor-shaped tags `RefPred`/`RefType`. Ord is
/// derived constructor-then-field lexicographic on both sides, as in the
/// source — REQUIRED downstream: hashNameEnv (ComputeIds.hs:342) sorts
//  `[(SourceRef, Set (RefTarget PredicateId TypeId))]`, so Ord on the
/// Id instantiation is on the schema-ID chain.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RefTarget<P, T> {
    RefPred(P),
    RefType(T),
}

impl<P, T> RefTarget<P, T> {
    /// `instance Bifunctor RefTarget where bimap f _ (RefPred p) =
    /// RefPred (f p); bimap _ g (RefType t) = RefType (g t)`
    /// (Schema/Types.hs:82-84). Inherent method taking `&mut` closures,
    /// the surface the sibling types.rs established for Bifunctor —
    /// refsToIds (ComputeIds.hs:76-89) is its engine via
    /// `bimap (lookupPredicateId pids) (lookupTypeId tids)`.
    /// The source's derived `Functor` (fmap over the LAST parameter `t`)
    /// is subsumed: `fmap f = bimap(&mut |p| p, &mut f)` (same fold the
    /// sibling applied to the AST Functor derivings).
    pub fn bimap<P2, T2, F, G>(self, f: &mut F, g: &mut G) -> RefTarget<P2, T2>
    where
        F: FnMut(P) -> P2,
        G: FnMut(T) -> T2,
    {
        match self {
            RefTarget::RefPred(p) => RefTarget::RefPred(f(p)),
            RefTarget::RefType(t) => RefTarget::RefType(g(t)),
        }
    }
}

/// `instance (Binary p, Binary t) => Binary (RefTarget p t)`
/// (Schema/Types.hs:80, bare = generic-derived default): sum of 2
/// constructors → ONE Word8 tag (binary-0.8.9.1 Generic.hs:86-93:
/// n <= 256 ctors ⇒ Word8; tag = 0-based left-to-right declaration
/// index: RefPred = 0, RefType = 1), then the single field. Format per
/// the sibling types.rs encoder contract (RULE §2 Data.Binary row); the
/// tag bytes are spelled inline because the sibling's put_ctor_tag/
/// enc_cat helpers are private to that module — same bytes, no
/// re-declaration of its API.
impl<P: Binary, T: Binary> Binary for RefTarget<P, T> {
    fn encode(&self) -> Vec<u8> {
        let (tag, payload): (u8, Vec<u8>) = match self {
            RefTarget::RefPred(p) => (0, p.encode()),
            RefTarget::RefType(t) => (1, t.encode()),
        };
        let mut out = Vec::with_capacity(1 + payload.len());
        // A-review: local tripwire mirroring the sibling's put_ctor_tag
        // debug_asserts (RefTarget has exactly 2 variants; a future variant
        // inserted between must renumber tags consciously).
        debug_assert!((tag as usize) < 2, "ctor tag out of range: N must equal RefTarget's variant count");
        out.push(tag);
        out.extend_from_slice(&payload);
        out
    }
}

// TODO(port): validation obligation — this RefTarget encoding is on the
// schema-ID linchpin chain: hashNameEnv (ComputeIds.hs:340-342) does
// `hashBinary (sort (HashMap.toList env))` over the `NameEnv RefTargetId`
// built by makeSchemaEnv, and the resulting Hash is show-rendered into
// the persisted `schemaId` (ComputeIds.hs:307). Byte-exactness (tag
// order/values above) is a claim pinned from the same package sources as
// the sibling encoder until the old-binary getSchemaInfo judge case
// passes over the 96 .angle schema sources (the obligation sibling
// types.rs TODO #3 carries for ITS impls; this extends it to this
// module's one impl).

/// `type RefResolved = RefTarget PredicateRef TypeRef` (Schema/Types.hs:86).
pub type RefResolved = RefTarget<PredicateRef, TypeRef>;

/// `showRefTarget :: (ShowRef t, ShowRef p) => RefTarget p t -> Text`
/// (Schema/Types.hs:88-90). NOT in the module's export list — module
/// private in Haskell, private here. Arm order mirrors the source
/// (RefType case first; source arm order mirrored (Types.hs:89-90)).
fn show_ref_target<P: ShowRef, T: ShowRef>(target: &RefTarget<P, T>) -> String {
    match target {
        RefTarget::RefType(t) => t.show_ref(),
        RefTarget::RefPred(p) => p.show_ref(),
    }
}

// ---------------------------------------------------------------------------
// Name environments and resolution
// ---------------------------------------------------------------------------

/// An environment that describes which names are in scope. (Source doc.)
///
/// `type NameEnv t = HashMap SourceRef (Set t)` (Schema/Types.hs:93):
/// Data.HashMap.Strict → std HashMap (hash-keyed, iteration order never
/// observed in the source — all orderings that surface come from the
/// Set side), Data.Set → BTreeSet mirroring the crate's sorted-container
/// mapping (sibling types.rs: "Haskell's sorted Data.Map maps onto
/// BTreeMap") — REQUIRED because Set.toList's ASCENDING order is
/// observable through the Ambiguous payload below.
pub type NameEnv<T> = HashMap<SourceRef, BTreeSet<T>>;

/// `mapNameEnv :: Ord b => (a -> Maybe b) -> NameEnv a -> NameEnv b`
/// (Schema/Types.hs:95-98): `fmap g` over the HashMap's values where
/// `g set = Set.fromList [ b | a <- Set.toList set, Just b <- [f a] ]`.
/// Argument order kept 1:1 (f first). By-ref: the Haskell shares the
/// argument immutably and the one tree caller (ComputeIds.hs:315,
/// `mapNameEnv (Just . refsToIds refToIdEnv) resolvedSchemaQualScope`)
/// keeps reading the ResolvedSchema it came from — so the SourceRef keys
/// are cloned into the new map (RULE §0: the clone stands in for
/// Haskell's free sharing). The closure takes `&a` and returns Option —
/// `Maybe b` with None filtering the element out, exactly the
/// list-comprehension's `Just b <- [f a]`.
pub fn map_name_env<A, B, F>(f: F, env: &NameEnv<A>) -> NameEnv<B>
where
    B: Ord,
    F: FnMut(&A) -> Option<B>,
{
    env.iter()
        .map(|(source_ref, set)| {
            let mapped: BTreeSet<B> = set.iter().filter_map(&f).collect();
            (source_ref.clone(), mapped)
        })
        .collect()
}

/// `data LookupResult t = OutOfScope | Ambiguous [t] | ResolvesTo t`
/// (Schema/Types.hs:100-103) — NO deriving clause in the source; gets
/// Clone (sharing stand-in) + Debug (RULE §2 deriving-Show row's
/// constructor-shaped-naming default for Show-less types). Constructor
/// order preserved.
#[derive(Clone, Debug)]
pub enum LookupResult<T> {
    OutOfScope,
    Ambiguous(Vec<T>),
    ResolvesTo(T),
}

/// `lookupResultToEither :: (ShowRef t, ShowRef p) => SourceRef ->
/// LookupResult (RefTarget p t) -> Either Text (RefTarget p t)`
/// (Schema/Types.hs:105-115). `Either Text` → `Result<_, String>`
/// (RULE §1 strings row for the Text payload); the LookupResult moves in
/// (transient value — both tree callers build it from resolveRef and
/// consume immediately: Resolve.hs:547, Resolve.hs:784), the SourceRef is
/// only rendered, so it borrows. Error strings byte-for-byte:
/// "not in scope: " <> showRef ref, and showRef ref <> " is ambiguous.
/// It could refer to: " <> intercalate ", " targets.
pub fn lookup_result_to_either<P, T>(
    ref_: &SourceRef,
    result: LookupResult<RefTarget<P, T>>,
) -> Result<RefTarget<P, T>, String>
where
    P: ShowRef,
    T: ShowRef,
{
    match result {
        LookupResult::OutOfScope => Err(format!("not in scope: {}", ref_.show_ref())),
        LookupResult::Ambiguous(targets) => Err(format!(
            "{} is ambiguous. It could refer to: {}",
            ref_.show_ref(),
            {
                // Text.intercalate ", " (map showRefTarget targets) —
                // join with ", " over the ASCENDING targets (the
                // BTreeSet order resolve_ref_filtered preserved).
                let shown: Vec<String> = targets.iter().map(show_ref_target).collect();
                shown.join(", ")
            }
        )),
        // lookupResultToEither _ (ResolvesTo target) = Right target
        LookupResult::ResolvesTo(target) => Ok(target),
    }
}

/// Resolve a name (@SourceRef@) with respect to a scope (@NameEnv@)
/// (source doc, Schema/Types.hs:117-119):
/// `resolveRef scope ref = resolveRefFiltered scope (const True) ref`.
pub fn resolve_ref<T: Clone>(scope: &NameEnv<T>, ref_: &SourceRef) -> LookupResult<T> {
    resolve_ref_filtered(scope, &|_: &T| true, ref_)
}

/// `resolveRefFiltered :: NameEnv t -> (t -> Bool) -> SourceRef ->
/// LookupResult t` (Schema/Types.hs:121-128). HashMap.lookup → get;
/// `filter p $ Set.toList set` walks the set in ASCENDING order (kept by
/// BTreeSet), so the Ambiguous list is ascending — observable in the
/// "could refer to" error text. `T: Clone` only to hand ownership of the
/// matched elements to the caller (the Haskell shares them); by-ref
/// scope/ref mirror the pure source signature (RULE §0).
pub fn resolve_ref_filtered<T: Clone, F>(
    scope: &NameEnv<T>,
    p: &F,
    ref_: &SourceRef,
) -> LookupResult<T>
where
    F: Fn(&T) -> bool,
{
    match scope.get(ref_) {
        None => LookupResult::OutOfScope,
        Some(set) => {
            let matched: Vec<T> = set.iter().filter(|t| p(t)).cloned().collect();
            match matched.as_slice() {
                // [] -> OutOfScope
                [] => LookupResult::OutOfScope,
                // [one] -> ResolvesTo one (clone = the ownership handoff)
                [one] => LookupResult::ResolvesTo(one.clone()),
                // many -> Ambiguous many
                many => LookupResult::Ambiguous(many.to_vec()),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Resolved abstract syntax
// ---------------------------------------------------------------------------
//
// All nine unprimed aliases instantiate the sibling angle::types AST at
// (PredicateRef, TypeRef) with spans fixed to SrcSpan — 1:1 with
// Schema/Types.hs:132-140. The refs arrive through the sibling's
// glean_thrift re-export.

/// `type ResolvedType = Type_ SrcSpan PredicateRef TypeRef`.
pub type ResolvedType = Type_<SrcSpan, PredicateRef, TypeRef>;
/// `type ResolvedFieldDef = FieldDef_ SrcSpan PredicateRef TypeRef`.
pub type ResolvedFieldDef = FieldDef_<SrcSpan, PredicateRef, TypeRef>;
/// `type ResolvedTypeDef = TypeDef_ SrcSpan PredicateRef TypeRef`.
pub type ResolvedTypeDef = TypeDef_<SrcSpan, PredicateRef, TypeRef>;
/// `type ResolvedPredicateDef = PredicateDef_ SrcSpan SrcSpan PredicateRef TypeRef`.
pub type ResolvedPredicateDef = PredicateDef_<SrcSpan, SrcSpan, PredicateRef, TypeRef>;
/// `type ResolvedDerivingDef = DerivingDef_ SrcSpan SrcSpan PredicateRef TypeRef`.
pub type ResolvedDerivingDef = DerivingDef_<SrcSpan, SrcSpan, PredicateRef, TypeRef>;
/// `type ResolvedPat = SourcePat_ SrcSpan SrcSpan PredicateRef TypeRef`.
pub type ResolvedPat = SourcePat_<SrcSpan, SrcSpan, PredicateRef, TypeRef>;
/// `type ResolvedStatement = SourceStatement_ SrcSpan SrcSpan PredicateRef TypeRef`.
pub type ResolvedStatement = SourceStatement_<SrcSpan, SrcSpan, PredicateRef, TypeRef>;
/// `type ResolvedQuery = SourceQuery_ SrcSpan SrcSpan PredicateRef TypeRef`.
pub type ResolvedQuery = SourceQuery_<SrcSpan, SrcSpan, PredicateRef, TypeRef>;
/// `type ResolvedDeriving = DerivingInfo ResolvedQuery`.
pub type ResolvedDeriving = DerivingInfo<ResolvedQuery>;

// Versions of the above types abstracted over the source spans
// (Schema/Types.hs:142-149). Haskell primes (`ResolvedPat'` …) become
// `WithSpans` suffixed aliases per the header note: the span parameters
// are the free ones, refs stay (PredicateRef, TypeRef).

/// `type ResolvedPat' s = SourcePat_ s s PredicateRef TypeRef`.
pub type ResolvedPatWithSpans<S> = SourcePat_<S, S, PredicateRef, TypeRef>;
/// `type ResolvedStatement' s = SourceStatement_ s s PredicateRef TypeRef`.
pub type ResolvedStatementWithSpans<S> = SourceStatement_<S, S, PredicateRef, TypeRef>;
/// `type ResolvedQuery' s = SourceQuery_ s s PredicateRef TypeRef`.
pub type ResolvedQueryWithSpans<S> = SourceQuery_<S, S, PredicateRef, TypeRef>;
/// `type ResolvedDeriving' s = DerivingInfo (ResolvedQuery' s)`.
pub type ResolvedDerivingWithSpans<S> = DerivingInfo<ResolvedQueryWithSpans<S>>;
/// `type ResolvedTypeDef' s = TypeDef_ s PredicateRef TypeRef`.
pub type ResolvedTypeDefWithSpans<S> = TypeDef_<S, PredicateRef, TypeRef>;
/// `type ResolvedType' s = Type_ s PredicateRef TypeRef`.
pub type ResolvedTypeWithSpans<S> = Type_<S, PredicateRef, TypeRef>;
/// `type ResolvedFieldDef' s = FieldDef_ s PredicateRef TypeRef`.
pub type ResolvedFieldDefWithSpans<S> = FieldDef_<S, PredicateRef, TypeRef>;

// ---------------------------------------------------------------------------
// Resolved schemas
// ---------------------------------------------------------------------------

/// A 'ResolvedSchema' is used during schema resolution to resolve
/// schemas that import or inherit from this schema. (Source doc,
/// Schema/Types.hs:150-151.) The Haskell declaration has NO deriving
/// clause; Clone (sharing stand-in) + Debug per the RULE §2
/// deriving-Show row's Show-less-types default. Field order and the
/// per-field source comments are 1:1.
#[derive(Clone, Debug)]
pub struct ResolvedSchema<P, T> {
    pub resolved_schema_name: Name,
    pub resolved_schema_version: Version,
    pub resolved_schema_angle_version: AngleVersion,
    /// types that are defined by this schema
    pub resolved_schema_types: HashMap<TypeRef, TypeDef_<SrcSpan, P, T>>,
    /// types that are inherited and re-exported by this schema
    pub resolved_schema_re_exported_types: HashMap<TypeRef, TypeDef_<SrcSpan, P, T>>,
    /// predicates that are defined by this schema
    pub resolved_schema_predicates: HashMap<PredicateRef, PredicateDef_<SrcSpan, SrcSpan, P, T>>,
    /// predicates that are inherited and re-exported by this schema
    pub resolved_schema_re_exported_predicates:
        HashMap<PredicateRef, PredicateDef_<SrcSpan, SrcSpan, P, T>>,
    /// The scope exposed by this schema, unqualified. This will be
    /// used when the schema is inherited.
    pub resolved_schema_unqual_scope: NameEnv<RefTarget<P, T>>,
    /// The scope exposed by this schema, qualified. This will be
    /// used when the schema is inherited or imported.
    pub resolved_schema_qual_scope: NameEnv<RefTarget<P, T>>,
    /// deriving declarations, for predicates defined in this schema
    /// or an inherited schema.
    pub resolved_schema_deriving: HashMap<PredicateRef, DerivingDef_<SrcSpan, SrcSpan, P, T>>,
    /// schemas evolves by this schema.
    pub resolved_schema_evolves: BTreeSet<SchemaRef>,
}

/// `type ResolvedSchemaRef = ResolvedSchema PredicateRef TypeRef`
/// (Schema/Types.hs:180).
pub type ResolvedSchemaRef = ResolvedSchema<PredicateRef, TypeRef>;

/// `schemaRef :: ResolvedSchema p t -> SchemaRef` (Schema/Types.hs:182-184):
/// `SchemaRef resolvedSchemaName resolvedSchemaVersion` — the record
/// wildcard read of exactly those two fields. By-ref (the tree callers,
/// e.g. Schema.hs:621 `schemaRef resolved`, keep the schema); Version is
/// Copy, Name clones once.
pub fn schema_ref<P, T>(schema: &ResolvedSchema<P, T>) -> SchemaRef {
    SchemaRef(
        schema.resolved_schema_name.clone(),
        schema.resolved_schema_version,
    )
}

/// A set of schemas after name resolution (source doc,
/// Schema/Types.hs:186-191). No deriving clause in the source; Clone +
/// Debug as above.
#[derive(Clone, Debug)]
pub struct ResolvedSchemas {
    pub schemas_highest_version: Option<Version>,
    /// Resolved schemas in dependency order
    pub schemas_resolved: Vec<ResolvedSchemaRef>,
}

// ---------------------------------------------------------------------------
// Marker breakdown (RULE §6: reviewers check the trailer against this)
// ---------------------------------------------------------------------------
//
// Open TODO(port) markers: 1
//   1. The Binary (RefTarget p t) byte-exactness validation obligation
//      (the TODO at the impl): the encoding is on the persisted
//      schema-ID chain via hashNameEnv (ComputeIds.hs:340-342) and is
//      pinned from the same package sources as the sibling encoder
//      module, but proof waits on the old-binary getSchemaInfo judge
//      case over the 96 .angle schema sources.
//
// No BUG(port) markers: the module contains no partial functions and no
// reproducible source defects (checked: mapNameEnv/resolveRef*/
// lookupResultToEither/showSchemaRef are total; the Text.init hazard
// lived in Util.hs's parseRef, flagged in the sibling util.rs).
// No PERF(port) or SAFETY markers: allocations are the shared first-pass
// norm and there is no unsafe code.
//
// Coverage: every export of the Haskell module has a Rust counterpart —
// SchemaRef(..)/showSchemaRef (+ Hashable as derived Hash, Generic as
// derivation machinery with no Rust surface), RefTarget(..)/RefResolved
// (+ Binary and Bifunctor instances, Functor subsumed into bimap),
// all nine unprimed Resolved* aliases and all seven primed ones
// (WithSpans suffix), NameEnv/mapNameEnv/LookupResult(..)/resolveRef/
// resolveRefFiltered/lookupResultToEither, and ResolvedSchemas(..)/
// ResolvedSchema(..)/ResolvedSchemaRef/schemaRef. The module-private
// showRefTarget stays private. Tests are not translated (RULE §4; the
// judge is the referee).
//
// MODULE-WIRING: lib.rs / schema/mod.rs scaffolding is deliberately NOT
// a manifest unit; owned by the Step-4 survey build (do not create
// ad-hoc here) — same note as the sibling types.rs/util.rs.
// confidence note: high on this unit's own logic; the inherited byte-exactness
// obligation is the encoder owner's medium (angle/types.rs) — same judge run.
// PORT STATUS: confidence=high todos=1
