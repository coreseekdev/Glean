// Ported from: glean/hs/Glean/RTS/Traverse.hs (119 lines)
// Manifest unit: rust/glean-core/src/rts/traverse.rs (migration/
// manifest.tsv line 23, crate glean-core). RULE §4 (naming): one source
// file → one target file; the manifest path is authoritative.
//
// Reconciliation (path/row truth, RULE §4: the manifest is authoritative):
// query/transform.rs's header cites this unit as "glean/rts/Glean/RTS/
// Traverse.hs", and HANDOFF §2.3 recorded manifest rows 23-24 as BLOCKED
// ("their module Glean.RTS.Bytecode.Gen.Issue is missing from the Haskell
// tree; skip these rows"). Both are superseded: the manifest row is
// `glean/hs/Glean/RTS/Traverse.hs` (present, 119 lines), and the generated
// Issue module EXISTS in the pinned build tree
// (~/glean-build/Glean/glean/hs/Glean/RTS/Bytecode/Gen/Issue.hs —
// jumpIf0 :: Register 'Word -> Label -> Code () at :198-201, select at
// :290-293, raise at :294-297, ret at :313-315, addConst/add at :174-181,
// callFun_2_0 at :258-261), its Rust symbol contract pinned as the
// ASSUMED-IMPORT below (the same seam query/transform.rs and
// rts/bytecode/code.rs already assume). The unblock is this batch's brief:
// "the Issue/label contract is now shipped in crate::rts::bytecode::code".
//
// INVENTORY (RULE §5): ONE row governs this unit — inventory.tsv row 80
// (merged from inventory.codeflow.tsv row 12), followed field-for-field:
//   sites:   "3 mdo sites (:48,:62,:76)" — confirmed: Traverse.hs:48
//            (ArrayRep `_ -> mdo`), :62 (SumRep otherwise arm), :76
//            (SetRep local $ \size -> mdo); 6 `x <- label` bind points
//            across them (:50/:53/:70/:74/:79/:82).
//   class:   CONTROL_FLOW.
//   target:  "two-pass label allocation (shared codegen IR); identity via
//            disassemble" — discharged as follows. Pass 1 = the pure twin
//            walker `go_allocations` (type-driven allocation counts; the
//            "shared codegen IR" is the LabelPlan mirror below) plus the
//            arithmetic knot predictions taken from `cur_label`; pass 2 =
//            the emitters, whose bind points allocate through the REAL
//            `Code::label` and whose every emission advances the plan in
//            lockstep. EMISSION DISCIPLINE (A2, review fix — stated
//            option): an emission operand names a block either (a) whose
//            bind has ALREADY run — then the operand is driven from the
//            REAL mirror_label value (Sites 1/3's DecrAndJumpIfNot0 loop
//            operand), or (b) not yet minted — a forward ref (the
//            JumpIf0/Select `end`/`alts`), where the real value cannot
//            exist before the emission, so the operand keeps its pass-1
//            prediction and the prediction-vs-real check at the bind is a
//            RELEASE-ACTIVE assert_eq! (not debug_assert_eq!): a
//            divergence panics inside gen_traversal, so wrong bytecode
//            can never reach a caller. What stays debug-only is the
//            plan-vs-Code canary inside mirror_label; the prediction
//            checks themselves are release-enforced. The old-vs-new
//            disassemble identity tables live at each MDO SITE block.
//   risk:    "feeds ownership — wrong bytecode = wrong traversal, judge
//            case on traversal output" — that is RULE §0 (A1)'s promotion
//            gate: the old-vs-new `disassemble` identity check runs on
//            this file's outputs before promotion (judge, Step 4). The
//            obligation is recorded here in the header, not as a marker
//            (the supply.rs/code.rs precedent).
// Divergence from the row: NONE — the row was followed field-for-field.
// (Review fix A1: the pass-1 twin's ArrayRep arm originally dropped go's
// `case elty of ByteRep` split — Traverse.hs:46-47, a ByteRep element body
// is `add size input`, ZERO label allocations — and transcribed the
// 2-allocation mdo for every element type; the mdo exists only in the
// `_ ->` arm, :48-54. A plain transcription defect, fixed in this pass —
// not a row divergence.) No INVENTORY-BUG-CANDIDATE.
//
// SIBLING CONTRACTS (RULE §4: DONE units' real APIs read, never
// re-declared):
//   - crate::rts::bytecode::code (SHIPPED): Code (the monad-as-struct;
//     constant/local/label/issue/issue_end_block/vlog), Optimised(..),
//     Label re-exported from glean_bytecode; the BIND-point label rule
//     (code.rs `Code::label` doc: fresh id iff the current block is
//     non-empty, else alias — Code.hs:483-488) and new_block's implicit
//     Jump (Code.hs:459-468) are the semantics this file's plan mirrors;
//     the `generate!` macro (#[macro_export]) is the flat-syntax form of
//     the CodeGen walk used by gen_traversal.
//   - crate::rts::bytecode::supply (SHIPPED): Supply trait +
//     RegSupply (+ peek_supply, not needed here — no peeks in Traverse.hs).
//   - crate::rts::foreign::bytecode (SHIPPED): Subroutine<S> (phantom
//     marker, ForeignPtr handle; Subroutine::with_ptr is NOT used here —
//     Traverse.hs only constructs the subroutine, never calls it).
//   - crate::rts::types (SHIPPED): Pid(pub i64), Type, Rep<ID> (ByteRep/
//     NatRep/StringRep/ArrayRep/TupleRep/SumRep/SetRep/PredicateRep —
//     exactly the source's constructors, Types.hs:185-194),
//     rep_type(&Type) -> Rep<Pid> (Types.hs:112-130).
//   - glean_bytecode::bytecode::types (SHIPPED): Register<T: TyTag>,
//     Label(pub i64), Literal, Fun<Args>/TyTuple, DataPtr/Word markers,
//     Addable — the instances `Addable<Word> for Word` and
//     `Addable<Word> for DataPtr` (Types.hs:40-42) are LOAD-BEARING here:
//     addConst/add at DataPtr destinations typecheck through them.
//
// ASSUMED-IMPORT (RULE §4 B4): `crate::rts::bytecode::gen::issue` — the
// generated Issue module (source Glean.RTS.Bytecode.Gen.Issue, generated
// by Generate/Haskell.hs genModule "Issue", Haskell.hs:72-79; owner: the
// generator units, manifest rows 150-152; build output — never hand-
// translated here, B4). The symbol contract is the one query/transform.rs
// pinned (its header's def-table list), EXTENDED here with the four
// wrappers this file adds (consumer-extension rule, RULE §4 B4: the
// extension is recorded by the consumer, cross-referencing the first
// sketch). All twelve confirmed against the generated file on disk:
//     input_nat(begin, end, dst)                InputNat    Issue.hs:72-75
//     input_skip_nat(begin, end)                InputSkipNat  :97-100
//     input_skip_trusted_string(begin, end)     InputSkipTrustedString
//                                                           :101-104
//     add_const(imm: u64, dst)                  AddConst    :174-177
//       Addable a 'Word => Word64 -> Register a -> Code () — the Rust
//       wrapper is generic over the class context exactly as the pinned
//       compiler's mapping requires (Generate/Haskell.hs:231-237 prepends
//       `context insnContext`): `pub fn add_const<A: Addable<Word>>(imm:
//       u64, dst: Register<A>, code: &mut Code)`.
//     add(src, dst)                             Add         :178-181
//       Addable a b => Register b -> Register a -> Code () — Rust image
//       `pub fn add<B: TyTag, A: Addable<B>>(src: Register<B>,
//       dst: Register<A>, code: &mut Code)` (Self = destination a, the
//       parameter = operand b, per the shipped Addable doc).
//     jump_if0(reg, tgt)                        JumpIf0     :198-201 (issue)
//     decr_and_jump_if_not0(reg, tgt)           DecrAndJumpIfNot0
//                                                           :230-234 (issue)
//     select(sel, tgts: &[Label])               Select      :290-293 (issue)
//     jump(tgt)                                 Jump        :190-193
//       (EndBlock effect, def/Instruction.hs:264-266 → issueEndBlock)
//     raise(msg: &[u8])                         Raise       :294-297
//       (EndBlock → issueEndBlock; ImmLit → msg_i <- literal msg first)
//     call_fun_2_0(fun, arg1, arg2)             CallFun_2_0 :258-261 (issue)
//       Register ('Fun '[ 'Word,'Word ]) -> Register 'Word ->
//       Register 'Word -> Code ()
//     ret()                                     Ret         :313-315
//       (EndBlock+Return → issueEndBlock)
// All take `&mut Code` LAST, return () (the in-tree Code-monad-action
// convention, transform.rs's pinned contract). None of these is
// hand-translated here.
//
// THE MDO TRANSLATION — the A1 contract (RULE §0 A1 / RULE §2
// RecursiveDo row: "IDs allocated at BIND (knot) points, never at mint
// points; every `x <- label` site emits its block split (implicit Jump)").
//
// The source's three mdo blocks tie their knots through HASKELL
// LAZINESS. Derived from the pinned sources (the old code is the spec):
//   - `issue insn` stores the instruction THUNK in a lazy cons field
//     (`s { csInsns = insn : csInsns }`, Code.hs:491-492): State.Strict
//     (Code.hs:39) forces the state RECORD to WHNF at each bind, never
//     its fields, so the JumpIf0/Select constructor applications are
//     deferred;
//   - the operand fields are strict (`{-# UNPACK #-} !Label` — generated
//     Instruction.hs:49 JumpIf0, :73 Raise; Select's `[Label]` offsets at
//     :72 are lazy spine-and-elements), so forcing the constructor forces
//     the knot value — but that forcing happens only when the layout pass
//     scrutinizes the instructions (insnSize/insnLabels/insnWords over
//     the laid-out stream, Code.hs:301-307 with layout :386-435), LONG
//     after every `x <- label` bind has executed;
//   - therefore each knot value is exactly the BIND-point `label` return
//     (Code.hs:483-488): the id of the block starting at the bind
//     position, with a fresh `succ csLabel` allocated (newBlock's
//     implicit Jump, Code.hs:459-468, `let !label = succ csLabel` :462)
//     IFF the current block is non-empty at the bind; on an empty current
//     block the CURRENT id is returned and NOTHING is allocated (the
//     alias case).
// (Corroboration, not evidence: stress round-2's translation probe
// ("A's LabelPlan made both D1/D14 mechanical verdicts",
// stress-test/report-r2.md) and the shipped sibling's GHC 9.4.7 replica
// record (query/transform.rs, THE MDO TRANSLATION block).)
//
// Two facts make the forward ids computable eagerly at every site — both
// derived from Traverse.hs itself:
//   1. Every bind point's close/alias choice is INDEPENDENT of the
//      mdo-entry block state: the Array/Set loop bind follows inputNat +
//      jumpIf0 (both `issue` into the current block, :45/:49 and
//      :77/:78), the Array/Set end bind follows decrAndJumpIfNot0
//      (:52/:81 — the decr is issued unconditionally after the body), and
//      the Sum alt/end binds each follow an EndBlock close (the raise :68
//      before the first alt; each `jump end` :72 before the next; the
//      last jump before the end bind) — issueEndBlock resets csInsns
//      (Code.hs:494-496 = newBlock . Just). So allocation ORDER is fixed
//      by the structure alone.
//   2. The intervening allocations are TYPE-DRIVEN: `go`'s emission is a
//      pure function of (refs, rep), so the count between a forward
//      reference and its bind is computed by the pure twin walker
//      `go_allocations` — structure-for-structure with `go`, same guards,
//      same arms.
// The LabelPlan is the mirror of Code's PRIVATE label state — the pair
// (cs_label, block-has-insns) — which is what makes `cur_label` readable
// from outside code.rs (the fields are private there by design). The
// pattern is the stress-validated one shipped in query/transform.rs
// (inventory.tsv row 42's "shared codegen IR"); it is re-hosted HERE
// because crate direction forbids the import (glean-db depends on
// glean-core, never the reverse — depmap/crates.txt). One shape delta
// from the sibling, same semantics: transform.rs holds its plans in a
// thread-local stack because manifest.tsv row 110's rank-2
// TransformPatternAction closures cannot carry a plan parameter (its own
// header records the reason). Traverse.hs's go is NOT closure-free —
// the three `local $ \r -> …` bodies (:44/:63/:76/:84) close over the
// traversal state — but its closures are plain Rust closures over an
// explicitly threaded `&mut LabelPlan` (immediately-consumed FnOnces
// reborrowing the plan, code.rs:332-336), so the explicit &mut threading
// holds at all 3 mdo sites: no stack, no scope discipline beyond the one
// call in gen_traversal. The lockstep rules are the sibling's verbatim:
// plan first, then Code; the bind checks per the A2 emission discipline
// recorded in the INVENTORY block (release-active prediction asserts at
// the mdo binds; the plan-vs-Code canary in mirror_label stays
// debug-only).
//
// DOWNSTREAM CONTRACT (pinned by the SHIPPED consumer, never re-declared):
// rust/glean-db/src/database/schema.rs:152-160 (manifest row 98) pins
//     pub fn gen_traversal(key: &Type, val: &Type)
//         -> Subroutine<CompiledTraversal>;   // rts/traverse.rs (row 23)
// with the ruling "IO -> infallible per the allocation-guard convention"
// (their call site :1916 consumes it bare). This file honors that shape.
//
// SECOND consumer — the manifest.tsv row-110 SEAM (for that unit's
// implementer; not yet shipped): manifest.tsv row 110 is
// glean/db/Glean/Query/Codegen.hs -> rust/glean-db/src/query/codegen.rs
// (depmap/edges.tsv:546 is the edge to this file). Codegen.hs:303-321
// calls genTraversal (:308) on the transformType qtrans-ed key/value
// types (:314-315) and packs the Subroutine into CompiledQuery
// (Codegen.hs:321). codegen.rs is currently a 73-line stub (created early
// as row 86's directory-module anchor) with NO recorded gen_traversal
// contract: row 110's implementer must pin the consumer half there — the
// traversal types are the TRANSFORMED types (the db-shape types, not the
// query-requested ones — their own comment at Codegen.hs:309-313 says
// why), and the Subroutine<CompiledTraversal> value flows into
// CompiledQuery.
//
// The one failure source of `generate` — the `fail "unterminated basic
// block"` check (Code.hs:279-280, ported at code.rs:725-729) — is
// STRUCTURALLY unreachable here: genTraversal's body ends in `ret`
// (Traverse.hs:99), an EndBlock instruction (issueEndBlock → newBlock
// empties csInsns, Code.hs:494-496/:459-468), so the final csInsns is
// empty by construction. The impossible Err is converted to a panic with
// the source's own message (RULE §2 partial-functions row: where the
// source would raise, panic with the same text — the Haskell `fail`
// propagates as an IO exception through `fmap snd $ generate ...`,
// Traverse.hs:95-96).
//
// KIND-TAG RECONCILIATION: `data CompiledTraversal` (Traverse.hs:24-25)
// is owned HERE (the source module exports it). The shipped
// rts/foreign/inventory.rs currently carries a stand-in
// `pub struct CompiledTraversal;` at inventory.rs:258-259 whose own TODO
// (:257) says "move to ... rts/traverse.rs (unit 19) when those units
// ship" — that unit's one-line re-export swap (`pub use crate::rts::
// traverse::CompiledTraversal;`) is ITS fix-pass obligation, recorded
// here as a cross-reference. Until the swap lands, the two paths are the
// live compile-error seam HANDOFF §2.4 already classified ("BLOCKED
// Traverse/Typecheck imports are LIVE uses (compile-error seams until
// rows 23/24)").

use glean_bytecode::bytecode::types::{Addable, DataPtr, Fun, Label, Register, TyTag, Word};

use crate::rts::bytecode::code::{Code, Optimised};
use crate::rts::bytecode::supply::{RegSupply, Supply};
use crate::rts::foreign::bytecode::Subroutine;
use crate::rts::types::{rep_type, Pid, Rep, Type};

// ASSUMED-IMPORT (contract in the header block; owner: the generator
// port, manifest rows 150-152).
use crate::rts::bytecode::gen::issue as generated;

// -----------------------------------------------------------------------------
// CompiledTraversal (Traverse.hs:24-25)
// -----------------------------------------------------------------------------

/// Type tag for Subroutine (source doc, Traverse.hs:24).
///
/// `data CompiledTraversal` — a phantom kind marker with no constructors'
/// payload; Subroutine is polymorphic in it ("to avoid us mixing up the
/// two", Foreign/Bytecode.hs:52-55). No deriving in the source, none here.
/// See the header KIND-TAG RECONCILIATION for the inventory.rs stand-in
/// that defers to this definition.
pub struct CompiledTraversal;

// -----------------------------------------------------------------------------
// The LabelPlan — the pass-1 mirror of Code's private label state
// (header: THE MDO TRANSLATION; inventory.tsv:80's "shared codegen IR")
// -----------------------------------------------------------------------------

/// The mirror of Code's private (cs_label, block-has-insns) state: `next`
/// mirrors cs_label; the next allocation mints cs_label+1 (Code.hs:462),
/// `block_open` says whether the current block holds instructions (non-empty
/// csInsns).
/// Kept true by mirroring EVERY emission this file makes (the mirror
/// helpers below); checked at the binds by the debug canary in
/// [`mirror_label`] and — for the prediction arithmetic the mdo sites read
/// off it — by the release-active prediction-vs-real asserts (A2). The
/// forward ids of all three mdo sites are read off it via [`cur_label`].
#[derive(Clone, Copy, Debug)]
struct LabelPlan {
    next: Label,
    block_open: bool,
}

impl LabelPlan {
    /// The initial state at a generate() scope, pinned by code.rs's
    /// generate (cs_label = Label(0), cs_insns empty — Code.hs:265-278).
    fn new() -> Self {
        LabelPlan {
            next: Label(0),
            block_open: false,
        }
    }

    /// The `issue` rule (Code.hs:491-492): the current block now holds an
    /// instruction.
    fn issue(&mut self) {
        self.block_open = true;
    }

    /// The `issueEndBlock` rule (Code.hs:494-496 = newBlock . Just,
    /// :459-468): `succ csLabel` allocated unconditionally, insns reset.
    fn issue_end_block(&mut self) {
        self.next = Label(self.next.0 + 1);
        self.block_open = false;
    }

    /// The `label` rule (Code.hs:483-488): close the block if non-empty
    /// (allocating `succ csLabel` for the implicit-Jump terminator), then
    /// yield the CURRENT id — alias, no allocation, on an empty block.
    fn label(&mut self) -> Label {
        if self.block_open {
            self.next = Label(self.next.0 + 1);
            self.block_open = false;
        }
        self.next
    }
}

/// The current label id (the mdo sites' L) — read-only: the binds, not
/// this projection, mutate the plan.
fn cur_label(plan: &LabelPlan) -> Label {
    plan.next
}

// The mirror helpers: every bytecode emission of this file goes through
// one of these, which advance the plan and the real Code in lockstep
// (plan FIRST, then Code — the sibling rule verbatim); the
// debug_assert_eq! at every label() site catches drift. RULE §3: no
// unsafe anywhere in this file; the helpers are pure state mirrors.

fn mirror_issue(code: &mut Code, plan: &mut LabelPlan, emit: impl FnOnce(&mut Code)) {
    plan.issue();
    emit(code);
}

fn mirror_issue_end_block(code: &mut Code, plan: &mut LabelPlan, emit: impl FnOnce(&mut Code)) {
    plan.issue_end_block();
    emit(code);
}

/// The one bind primitive: `x <- label` sites call THIS (never raw
/// `Code::label`), so the plan and the real allocation stay in lockstep
/// and the bind-point rule (RULE §0 A1: allocate at BIND, never at mint)
/// is enforced at the only place allocation happens.
fn mirror_label(code: &mut Code, plan: &mut LabelPlan) -> Label {
    let planned = plan.label();
    let real = code.label();
    debug_assert_eq!(planned, real, "LabelPlan drifted from Code's label state");
    real
}

/// One register pulled from the supply — the single-binding form of
/// Code::local's (pull, body) pair (code.rs DESIGN (2)); every `local $
/// \r -> ...` in Traverse.hs binds exactly one register (:44/:63/:76/:84).
fn pull1<A>(sup: &mut RegSupply) -> A
where
    RegSupply: Supply<A>,
{
    let (a, next) = sup.supply();
    *sup = next;
    a
}

// -----------------------------------------------------------------------------
// The Issue wrappers under the LabelPlan — the source's unqualified names
// (:41-43/:47/:49/:52/:65/:68/:72/:78/:81/:85/:87/:99), each delegating to
// the generated wrapper (header ASSUMED-IMPORT) with plan mirroring.
// -----------------------------------------------------------------------------

fn input_nat(
    input: Register<DataPtr>,
    inputend: Register<DataPtr>,
    dst: Register<Word>,
    code: &mut Code,
    plan: &mut LabelPlan,
) {
    mirror_issue(code, plan, |code| generated::input_nat(input, inputend, dst, code));
}

fn input_skip_nat(input: Register<DataPtr>, inputend: Register<DataPtr>, code: &mut Code, plan: &mut LabelPlan) {
    mirror_issue(code, plan, |code| generated::input_skip_nat(input, inputend, code));
}

fn input_skip_trusted_string(
    input: Register<DataPtr>,
    inputend: Register<DataPtr>,
    code: &mut Code,
    plan: &mut LabelPlan,
) {
    mirror_issue(code, plan, |code| {
        generated::input_skip_trusted_string(input, inputend, code)
    });
}

/// `addConst :: Addable a 'Word => Word64 -> Register a -> Code ()`
/// (generated Issue.hs:174-177) — the dst stays polymorphic exactly as
/// the source's class context allows; both call sites here (:41/:60) are
/// `Register 'DataPtr` (input), which types through the shipped
/// `Addable<Word> for DataPtr` instance (Bytecode/Types.hs:41-42).
fn add_const<A: Addable<Word>>(imm: u64, dst: Register<A>, code: &mut Code, plan: &mut LabelPlan) {
    mirror_issue(code, plan, |code| generated::add_const(imm, dst, code));
}

/// `add :: Addable a b => Register b -> Register a -> Code ()` (generated
/// Issue.hs:178-181). The one call site (:47, inside the ArrayRep
/// ByteRep arm) is `add size input` — src Register 'Word, dst Register
/// 'DataPtr.
fn add<B: TyTag, A: Addable<B>>(src: Register<B>, dst: Register<A>, code: &mut Code, plan: &mut LabelPlan) {
    mirror_issue(code, plan, |code| generated::add(src, dst, code));
}

fn jump_if0(reg: Register<Word>, tgt: Label, code: &mut Code, plan: &mut LabelPlan) {
    mirror_issue(code, plan, |code| generated::jump_if0(reg, tgt, code));
}

fn decr_and_jump_if_not0(reg: Register<Word>, tgt: Label, code: &mut Code, plan: &mut LabelPlan) {
    mirror_issue(code, plan, |code| generated::decr_and_jump_if_not0(reg, tgt, code));
}

fn select(sel: Register<Word>, tgts: &[Label], code: &mut Code, plan: &mut LabelPlan) {
    mirror_issue(code, plan, |code| generated::select(sel, tgts, code));
}

fn jump(tgt: Label, code: &mut Code, plan: &mut LabelPlan) {
    mirror_issue_end_block(code, plan, |code| generated::jump(tgt, code));
}

/// `raise :: ByteString -> Code ()` (generated Issue.hs:294-297): the
/// ImmLit argument is hoisted through Code::literal INSIDE the generated
/// wrapper (label-neutral: literal touches only csLiterals, Code.hs:
/// 470-481 — no block state, so the plan's issue_end_block advance before
/// the call stays exact).
fn raise(msg: &[u8], code: &mut Code, plan: &mut LabelPlan) {
    mirror_issue_end_block(code, plan, |code| generated::raise(msg, code));
}

fn call_fun_2_0(
    fun: Register<Fun<(Word, Word)>>,
    arg1: Register<Word>,
    arg2: Register<Word>,
    code: &mut Code,
    plan: &mut LabelPlan,
) {
    mirror_issue(code, plan, |code| generated::call_fun_2_0(fun, arg1, arg2, code));
}

fn ret(code: &mut Code, plan: &mut LabelPlan) {
    mirror_issue_end_block(code, plan, |code| generated::ret(code));
}

// -----------------------------------------------------------------------------
// The pass-1 twin walker (inventory.tsv:80's "two-pass label allocation")
// -----------------------------------------------------------------------------

/// A(ty) — the number of Label allocations `go refs rep` performs (the
/// pass-1 twin of `go`; header: THE MDO TRANSLATION, fact 2). Every arm
/// mirrors `go`'s guard and structure exactly:
///   - Byte/Nat/String/Predicate: issue-only bodies (addConst/
///     inputSkipNat/inputSkipTrustedString :41-43, the PredicateRep local
///     :84-87 — constant() touches only the constant table, Code.hs:
///     148-160) → 0;
///   - Array (guarded by `refs || hasRefs`): go's `case elty of` split
///     (:46-47) — a ByteRep element body is `add size input`, issue-only,
///     ZERO allocations (the 2-allocation mdo exists only in the `_ ->`
///     arm, :48-54); other elements: 2 (the loop bind's close —
///     inputNat+jumpIf0 precede it; the end bind's close — the decr
///     precedes it) + the element under refs=True (:51);
///   - Set (guarded by `refs || hasRefs`): 2 (the same two closes —
///     :78/:81) + the element under refs=True (:80);
///   - Tuple: the components under their scanr-derived flags (the flag
///     derivation at `go`'s TupleRep arm);
///   - Sum: the all-isUnit arm is issue-only → 0; the otherwise arm
///     (guarded by `refs || any hasRefs`) = 1 (the raise's close, :68) +
///     n (one `jump end` close per alt, :72) + the alternatives — the
///     alt binds and the end bind ALIAS (they follow EndBlock closes,
///     header fact 1).
fn go_allocations(refs: bool, rep: &Rep<Pid>) -> u64 {
    match rep {
        Rep::ByteRep | Rep::NatRep | Rep::StringRep => 0,
        Rep::ArrayRep(elty) => {
            if refs || has_refs(elty) {
                // go's `case elty of` split (:46-47): a ByteRep element
                // body is `add size input` — issue-only, ZERO label
                // allocations; the 2-allocation mdo exists only in the
                // `_ ->` arm (:48-54).
                if matches!(**elty, Rep::ByteRep) {
                    0
                } else {
                    2 + go_allocations(true, elty)
                }
            } else {
                0
            }
        }
        Rep::TupleRep(tys) => tys
            .iter()
            .enumerate()
            .map(|(i, ty)| {
                // component i's flag = refs || any hasRefs tys[i+1..] —
                // the scanr derivation recorded at go's TupleRep arm.
                let flag = refs || tys[i + 1..].iter().any(|t| has_refs(t));
                go_allocations(flag, ty)
            })
            .sum(),
        Rep::SumRep(tys) => {
            if tys.iter().all(|t| is_unit(t)) {
                0
            } else if refs || tys.iter().any(|t| has_refs(t)) {
                1 + tys.len() as u64 + tys.iter().map(|t| go_allocations(refs, t)).sum::<u64>()
            } else {
                0
            }
        }
        Rep::SetRep(elty) => {
            if refs || has_refs(elty) {
                2 + go_allocations(true, elty)
            } else {
                0
            }
        }
        Rep::PredicateRep(_) => 0,
    }
}

// -----------------------------------------------------------------------------
// traversal (Traverse.hs:27-87) — private in the source (module list
// exports only CompiledTraversal and genTraversal, :10-13), private here.
// -----------------------------------------------------------------------------

/// `traversal callback input inputend ty = go False (repType ty)`
/// (:33). The Type is taken by shared reference (the Haskell shares the
/// argument; repType consumes it once, :33) — `&Type -> Rep<Pid>` is the
/// shipped rep_type's own signature.
///
/// HasCallStack (:28) carries no observable effect in this module —
/// neither traversal nor genTraversal has an error/vlog call whose
/// caller-location it would feed (grep: no `error`/`vlog` in
/// Traverse.hs) — dropped with the coverage note at the trailer.
fn traversal(
    callback: Register<Fun<(Word, Word)>>,
    input: Register<DataPtr>,
    inputend: Register<DataPtr>,
    ty: &Type,
    code: &mut Code,
    plan: &mut LabelPlan,
) {
    let rep = rep_type(ty); // go False (repType ty) — evaluated once, shared
    go(false, &rep, input, inputend, callback, code, plan);
}

/// The where-clause walk of `traversal` (:34-87). The refs discipline is
/// the source's own doc (:35-40, verbatim in substance):
///   - if refs is True, we *must* leave the input pointer pointing after
///     the value, because we're going to traverse more data;
///   - if refs is False, we *must not* traverse the value unless
///     (hasRefs ty) is True, because we might not have traversed the
///     previous value.
fn go(
    refs: bool,
    rep: &Rep<Pid>,
    input: Register<DataPtr>,
    inputend: Register<DataPtr>,
    callback: Register<Fun<(Word, Word)>>,
    code: &mut Code,
    plan: &mut LabelPlan,
) {
    match rep {
        // go refs ByteRep = when refs $ addConst 1 input (:41)
        Rep::ByteRep => {
            if refs {
                add_const(1, input, code, plan);
            }
        }
        // go refs NatRep = when refs $ inputSkipNat input inputend (:42)
        Rep::NatRep => {
            if refs {
                input_skip_nat(input, inputend, code, plan);
            }
        }
        // go refs StringRep = when refs $ inputSkipTrustedString input
        //   inputend (:43)
        Rep::StringRep => {
            if refs {
                input_skip_trusted_string(input, inputend, code, plan);
            }
        }
        // go refs (ArrayRep elty) = when (refs || hasRefs elty) $
        //   local $ \size -> do ... (:44-54)
        Rep::ArrayRep(elty) => {
            if refs || has_refs(elty) {
                code.local(pull1, move |size: Register<Word>, code| {
                    input_nat(input, inputend, size, code, plan); // :45
                    // case elty of
                    //   ByteRep -> add size input (:46-47)
                    //   _ -> mdo ... (:48-54) — MDO SITE 1 of 3, below
                    if matches!(**elty, Rep::ByteRep) {
                        add(size, input, code, plan); // :47
                    } else {
                        // ----- MDO SITE 1 of 3 — ArrayRep (Traverse.hs:48-54;
                        // inventory.tsv:80 site 1) -----
                        //
                        // OLD (the knot, :49-53):
                        //   jumpIf0 size end ; loop <- label ; go True elty ;
                        //   decrAndJumpIfNot0 size loop ; end <- label
                        // `end` is stored as a thunk in the JumpIf0 (lazy cons
                        // field, Code.hs:491-492; forced only at layout,
                        // Code.hs:301-307) and resolves to the END bind's
                        // value. With csLabel = L at the jumpIf0 and
                        // A = go_allocations(true, elty) (the body's
                        // allocations, header fact 2):
                        //
                        // OLD disassembly (label ids in BIND order — A1):
                        //   block L:       InputNat input inputend size
                        //                  ; JumpIf0 size (L+2+A)
                        //                  ; Jump (L+1)      [loop bind close]
                        //   block L+1:     <go True elty>
                        //                  ; DecrAndJumpIfNot0 size (L+1)
                        //                  ; Jump (L+2+A)    [end bind close]
                        //   block L+2+A:   (continues after the traversal)
                        // loop = L+1 (the loop bind CLOSES: inputNat+jumpIf0
                        // are in the current block, header fact 1);
                        // end = L+2+A (the end bind CLOSES: the decr was
                        // issued after the body unconditionally).
                        //
                        // NEW: identical instruction stream, identical ids.
                        // The binds allocate through mirror_label (the REAL
                        // Code::label — RULE §0 A1: at BIND points, never
                        // at mint). EMISSION DISCIPLINE (A2): the decr's
                        // loop operand is the REAL bind value (loop_real —
                        // the knot's real allocation is known before that
                        // emission, so no prediction is used); the JumpIf0's
                        // forward `end` cannot use the real value (the body
                        // has not run when it is emitted), so it stays the
                        // pass-1 prediction (L+2+A) and is checked against
                        // the real at the end bind with a RELEASE-ACTIVE
                        // assert_eq! — a divergence panics inside
                        // gen_traversal, so wrong bytecode can never reach a
                        // caller. [A1 disassemble-identity note]
                        let l = cur_label(plan);
                        let end = Label(l.0 + 2 + go_allocations(true, elty) as i64);
                        jump_if0(size, end, code, plan); // :49 (forward end)
                        let loop_real = mirror_label(code, plan); // :50
                        go(true, elty, input, inputend, callback, code, plan); // :51
                        decr_and_jump_if_not0(size, loop_real, code, plan); // :52 (REAL loop, A2)
                        let end_real = mirror_label(code, plan); // :53
                        assert_eq!(
                            end_real, end,
                            "traverse: LabelPlan prediction diverged from the real end bind (A2)",
                        );
                    }
                });
            }
        }
        // go refs (TupleRep tys) = mapM_ (uncurry go) $ zip
        //   (drop 1 $ scanr (\ty refs -> hasRefs ty || refs) refs tys) tys
        //   (:55-57). scanr produces [x_0..x_n] with x_n = refs,
        //   x_i = hasRefs ty_i || x_{i+1}; drop 1 zips x_{i+1} with ty_i,
        //   so component i's flag = refs || any hasRefs tys[i+1..] — the
        //   "keep the pointer alive for the LATER siblings that still
        //   need it" discipline. The last component's flag is refs.
        Rep::TupleRep(tys) => {
            for (i, ty) in tys.iter().enumerate() {
                let flag = refs || tys[i + 1..].iter().any(|t| has_refs(t));
                go(flag, ty, input, inputend, callback, code, plan);
            }
        }
        // go refs (SumRep tys) (:58-75) — MDO SITE 2 of 3 lives in the
        // otherwise arm.
        Rep::SumRep(tys) => {
            // | all isUnit tys = when refs $ if length tys <= 127
            //   then addConst 1 input else inputSkipNat input inputend
            //   (:59-61). Issue-only, no labels.
            if tys.iter().all(|t| is_unit(t)) {
                if refs {
                    if tys.len() <= 127 {
                        add_const(1, input, code, plan); // :60
                    } else {
                        input_skip_nat(input, inputend, code, plan); // :61
                    }
                }
            }
            // | otherwise = when (refs || any hasRefs tys) $ mdo ... (:62)
            else if refs || tys.iter().any(|t| has_refs(t)) {
                // ----- MDO SITE 2 of 3 — SumRep (Traverse.hs:62-75;
                // inventory.tsv:80 site 2) -----
                //
                // OLD (the knot, :63-74):
                //   local $ \sel -> do
                //     inputNat input inputend sel
                //     let unknownAlt = [end]
                //     select sel (alts ++ unknownAlt)      -- FORWARD refs
                //   raise "selector out of range"          -- EndBlock
                //   alts <- forM tys $ \ty -> do
                //     alt <- label ; go refs ty ; jump end ; return alt
                //   end <- label
                // `alts` and `end` are thunks inside the Select's lazy
                // [Label] list (Instruction.hs:72 — no bang), forced only
                // at layout. With csLabel = L at the select,
                // A_i = go_allocations(refs, ty_i), n = length tys:
                //
                // OLD disassembly (label ids in BIND order — A1):
                //   block L:        InputNat input inputend sel
                //                   ; Select sel [L+1, ..., end]
                //                   ; Raise lit          [raise close: the
                //                                         raise is the
                //                                         TERMINATOR; its
                //                                         issueEndBlock
                //                                         allocates L+1]
                //   block L+1:      <go refs ty_0> ; Jump end
                //                                        [jump close:
                //                                         allocates L+2+A_0]
                //   block L+2+A_0:  <go refs ty_1> ; Jump end
                //                                        [allocates
                //                                         L+2+A_0+A_1+1]
                //   ...
                //   block end:      (the block the LAST jump's close
                //                    allocated — empty at the end bind)
                // with
                //   alt_i = L+1 + Σ_{j<i} (A_j + 1)
                //   end   = L+1 + n + Σ_j A_j
                // and every allocation made by the raise (:68) and the
                // jumps (:72): the alt binds (:70) and the end bind (:74)
                // each follow an EndBlock close (header fact 1), see an
                // EMPTY current block, and therefore ALIAS — they allocate
                // NOTHING (Code.hs:483-488's null-insns branch). Total
                // allocations: 1 + n + Σ A_i (= go_allocations' SumRep
                // arm).
                //
                // NEW: identical instruction stream, identical ids. The
                // Select's targets are the pass-1 predictions, computed
                // BEFORE the select is emitted — exactly as the knot
                // delivers them at forcing time. EMISSION DISCIPLINE (A2):
                // unlike SITES 1/3, no emission here can be driven from a
                // REAL bind value — the Select and every alt's `jump end`
                // name blocks the binds have not minted yet — so all
                // forward ids stay predictions and each bind's
                // prediction-vs-real check is a RELEASE-ACTIVE assert_eq!
                // (a divergence panics inside gen_traversal before any
                // caller can see the bytecode).
                // [A1 disassemble-identity note]
                let l = cur_label(plan);
                let mut alts: Vec<Label> = Vec::with_capacity(tys.len());
                let mut cur = l.0 + 1; // alt_0: after the raise's close
                for t in tys {
                    alts.push(Label(cur));
                    // the alt body's allocations + the jump's close
                    cur += go_allocations(refs, t) as i64 + 1;
                }
                let end = Label(cur);
                // local $ \sel -> do ... (:63-67). NO move (B1 review fix):
                // the body is an immediately-consumed FnOnce (code.rs:
                // 332-336), so reborrows suffice and `plan`/`alts` survive
                // for the bind sites after the local.
                code.local(pull1, |sel: Register<Word>, code| {
                    input_nat(input, inputend, sel, code, plan); // :65
                    let mut targets = alts.clone();
                    targets.push(end); // let unknownAlt = [end] (:66)
                    select(sel, &targets, code, plan); // :67
                    // (the source's own TODO, transcribed verbatim, :64 —
                    // a source annotation, not a port marker: the
                    // schema/types.rs:480 ruling, same as transform.rs's
                    // two bare spellings)
                    // TODO: read byte rather than Nat if arity <= 127
                });
                raise(b"selector out of range", code, plan); // :68
                // alts <- forM tys $ \ty -> do ... (:69-73)
                for (ty, alt) in tys.iter().zip(alts.iter()) {
                    let alt_real = mirror_label(code, plan); // :70 (aliases)
                    assert_eq!(
                        alt_real, *alt,
                        "traverse: LabelPlan prediction diverged from a real alt bind (A2)",
                    );
                    go(refs, ty, input, inputend, callback, code, plan); // :71
                    jump(end, code, plan); // :72 (closes; forward end)
                }
                let end_real = mirror_label(code, plan); // :74 (aliases)
                assert_eq!(
                    end_real, end,
                    "traverse: LabelPlan prediction diverged from the real end bind (A2)",
                );
            }
        }
        // go refs (SetRep elty) = when (refs || hasRefs elty) $
        //   local $ \size -> mdo ... (:76-83) — MDO SITE 3 of 3.
        Rep::SetRep(elty) => {
            if refs || has_refs(elty) {
                code.local(pull1, move |size: Register<Word>, code| {
                    // ----- MDO SITE 3 of 3 — SetRep (Traverse.hs:76-83;
                    // inventory.tsv:80 site 3) -----
                    //
                    // OLD (the knot, :77-82): the whole mdo is the local
                    // body:
                    //   inputNat input inputend size ; jumpIf0 size end ;
                    //   loop <- label ; go True elty ;
                    //   decrAndJumpIfNot0 size loop ; end <- label
                    // Same shape as SITE 1 (only the guard differs: SetRep
                    // has no ByteRep special arm), with csLabel = L at the
                    // jumpIf0 and A = go_allocations(true, elty):
                    //
                    // OLD disassembly (label ids in BIND order — A1):
                    //   block L:       InputNat input inputend size
                    //                  ; JumpIf0 size (L+2+A)
                    //                  ; Jump (L+1)      [loop bind close]
                    //   block L+1:     <go True elty>
                    //                  ; DecrAndJumpIfNot0 size (L+1)
                    //                  ; Jump (L+2+A)    [end bind close]
                    //   block L+2+A:   (continues after the traversal)
                    //
                    // NEW: identical — same A2 emission discipline as
                    // SITE 1 (the decr takes the REAL loop operand; the
                    // JumpIf0's forward `end` is the pass-1 prediction,
                    // release-checked at the end bind).
                    // [A1 disassemble-identity note]
                    input_nat(input, inputend, size, code, plan); // :77
                    let l = cur_label(plan);
                    let end = Label(l.0 + 2 + go_allocations(true, elty) as i64);
                    jump_if0(size, end, code, plan); // :78 (forward end)
                    let loop_real = mirror_label(code, plan); // :79
                    go(true, elty, input, inputend, callback, code, plan); // :80
                    decr_and_jump_if_not0(size, loop_real, code, plan); // :81 (REAL loop, A2)
                    let end_real = mirror_label(code, plan); // :82
                    assert_eq!(
                        end_real, end,
                        "traverse: LabelPlan prediction diverged from the real end bind (A2)",
                    );
                });
            }
        }
        // go _ (PredicateRep (Pid pid)) = local $ \ide -> do ...
        //   (:84-87). The pattern SHAPE is preserved syntactically — the
        //   Pid constructor is matched, not projected (RULE §2 A8). ide is
        //   a Register 'Word: it is InputNat's dst (the generated
        //   Issue.hs:72-74 signature), the decoded pid itself.
        Rep::PredicateRep(Pid(pid)) => {
            code.local(pull1, move |ide: Register<Word>, code| {
                input_nat(input, inputend, ide, code, plan); // :85
                // pidr <- constant (fromIntegral pid) (:86): Pid wraps
                // Int64 (rts/types.rs), fromIntegral to Word64 wraps —
                // RULE §2 (numeric row): `as u64` is the same wrap.
                // constant() is label-neutral (Code.hs:148-160 touches
                // only the constant table), so no plan advance.
                let pidr = code.constant(*pid as u64);
                call_fun_2_0(callback, ide, pidr, code, plan); // :87
            });
        }
    }
}

// -----------------------------------------------------------------------------
// genTraversal (Traverse.hs:89-99)
// -----------------------------------------------------------------------------

/// Generate a subroutine which traverses a clause (fact key + value) and
/// invokes the supplied callback function for each fact ID contained in
/// it. NOTE: The subroutine assumes that the clause is type correct.
/// (Source doc, Traverse.hs:89-93, verbatim.)
///
/// `genTraversal :: HasCallStack => Type -> Type -> IO (Subroutine
/// CompiledTraversal)` (:94-99) — `fmap snd $ generate Optimised $
/// \handler clause_begin key_end clause_end -> do ...`. The (pull, body)
/// form of the CodeGen walk via the `generate!` macro (code.rs DESIGN
/// (2)); the four registers' tags are inferred from the body's uses
/// (handler at call_fun_2_0's Register ('Fun '[ 'Word,'Word ]), the three
/// pointers at traversal's Register 'DataPtr), exactly as the source's
/// lambda parameters are.
///
/// The infallible shape is the SHIPPED consumer's pinned contract
/// (header: DOWNSTREAM CONTRACT); the impossible Err panic carries the
/// source's own message. HasCallStack: see the coverage note.
pub fn gen_traversal(key_ty: &Type, val_ty: &Type) -> Subroutine<CompiledTraversal> {
    let result = crate::generate!(code, Optimised::Optimised, |handler, clause_begin, key_end, clause_end| {
        // The LabelPlan's initial state is pinned by code.rs's generate
        // (cs_label = Label(0), cs_insns empty — Code.hs:265-278); the
        // plan lives exactly one generate() scope, like the sibling's
        // push/pop discipline (query/transform.rs) with the scope being
        // this body.
        let mut plan = LabelPlan::new();
        // traversal handler clause_begin key_end key_ty (:97)
        traversal(handler, clause_begin, key_end, key_ty, code, &mut plan);
        // traversal handler key_end clause_end val_ty (:98)
        traversal(handler, key_end, clause_end, val_ty, code, &mut plan);
        // ret (:99) — issueEndBlock $ Ret: empties csInsns, which is what
        // makes generate's "unterminated basic block" check pass (see the
        // header DOWNSTREAM CONTRACT for the unreachability proof).
        ret(code, &mut plan);
    });
    // fmap snd $ generate ... (:95-96) — Meta is discarded, as in the
    // source.
    match result {
        Ok((_, sub)) => sub,
        // Structurally unreachable (header proof); the source's `fail`
        // would raise the message as an IO exception — RULE §2
        // (partial-functions row): panic with the same text.
        Err(e) => panic!("{}", e.message),
    }
}

// -----------------------------------------------------------------------------
// hasRefs / isUnit (Traverse.hs:101-119) — module-local helpers, generic
// over the Rep's id parameter exactly as the source's `Rep a -> Bool`
// signatures are.
// -----------------------------------------------------------------------------

/// `hasRefs :: Rep a -> Bool` (:101-109), arms in source order.
fn has_refs<ID>(rep: &Rep<ID>) -> bool {
    match rep {
        Rep::ByteRep => false,   // :102
        Rep::NatRep => false,    // :103
        Rep::StringRep => false, // :104
        Rep::ArrayRep(elty) => has_refs(elty), // :105
        Rep::TupleRep(tys) => tys.iter().any(|t| has_refs(t)), // :106
        Rep::SumRep(tys) => tys.iter().any(|t| has_refs(t)), // :107
        Rep::SetRep(elty) => has_refs(elty), // :108
        Rep::PredicateRep(_) => true, // :109
    }
}

/// `isUnit :: Rep a -> Bool` (:111-119), arms in source order. (The
/// source's `isUnit ArrayRep{}` / `isUnit SumRep{}` / `isUnit SetRep{}`
/// wildcard arms match those constructors at ANY payload — spelled with
/// the explicit constructors here; same dispatch.)
fn is_unit<ID>(rep: &Rep<ID>) -> bool {
    match rep {
        Rep::ByteRep => false,     // :112
        Rep::NatRep => false,      // :113
        Rep::StringRep => false,   // :114
        Rep::ArrayRep(_) => false, // :115
        Rep::TupleRep(tys) => tys.iter().all(|t| is_unit(t)), // :116
        Rep::SumRep(_) => false,   // :117
        Rep::SetRep(_) => false,   // :118
        Rep::PredicateRep(_) => false, // :119
    }
}

// Marker inventory (reviewer counts the trailer against this): ZERO
// TODO(port) — the one port-device question this file faced (how the mdo
// forward refs resolve without laziness) is CLOSED by the derived
// two-pass discipline (header: THE MDO TRANSLATION; the sibling's
// thread_local-vs-parameter shape delta is a recorded design fact, not an
// open question), and there are no other rule gaps. ZERO BUG(port) — no
// source defect is reproduced (the empty-sum `all isUnit [] = True →
// addConst` behavior of :59-60 is the source's plain semantics, not a
// crash to flag). ZERO PERF(port) — the twin walker is O(rep size), the
// alts clone is n Labels, both linear in what the source allocates. ZERO
// unsafe blocks (RULE §3 vacuously satisfied — pure codegen; the one FFI
// seam, subroutine construction, crosses via the SHIPPED
// crate::rts::foreign::bytecode inside code.rs's generate). ZERO
// INVENTORY-BUG-CANDIDATE (row 80 is followed field-for-field; see the
// header INVENTORY block). The bare "TODO:" at the SumRep site is the
// SOURCE's own annotation transcribed verbatim (Traverse.hs:64), not a
// port marker — the schema/types.rs:480 ruling, as applied in
// query/transform.rs.
//
// The A1 promotion gate (RULE §0): the old-vs-new `disassemble` identity
// check runs on this file's outputs before promotion — the three
// per-site identity tables above are the claims it verifies (6 label
// bind points, ids in bind order; every `x <- label` site emits its
// block split through the REAL Code::label). Recorded in the header, not
// as a marker (the supply.rs/code.rs precedent).
//
// Coverage: every export of the Haskell module has a Rust counterpart —
// CompiledTraversal [pub struct, the kind tag; inventory.rs:258-259's
// stand-in (TODO :257) defers to this definition], genTraversal
// [gen_traversal — the shipped consumer's pinned infallible shape,
// generate! macro over the 4-register pull]. Private surface ported too:
// traversal [fn traversal + the HasCallStack drop note], go [fn go with
// all 8 arms at :41-87, including the TupleRep scanr flag derivation and
// the SumRep all-isUnit 127-ary split], the 3 mdo sites [per-site identity
// tables + mirror_label binds], hasRefs/isUnit [generic over the id like
// the source]. Port devices (no silent gaps): LabelPlan + mirror helpers
// + pull1 [the pass-1 mirror of Code's private state, the row-42-validated
// pattern re-hosted with the shape delta recorded], go_allocations [the
// pass-1 twin]. Dropped Haskell artifacts (no silent gaps): RecursiveDo
// [the A1 two-pass discipline], HasCallStack [no observable use in the
// module — no error/vlog call exists in Traverse.hs], Control.Monad's
// when/mapM_/zip/uncurry [if/for/enumerate, mechanical], the lazy knot
// thunks [pass-1 predictions + the A2 lockstep checks: release-active
// prediction asserts at the mdo binds, the debug plan-vs-Code canary in
// mirror_label].
//
// MODULE-WIRING: lib.rs / rts.rs module scaffolding is deliberately NOT a
// manifest unit; owned by the Step-4 survey build (do not create ad-hoc
// here — the supply.rs/code.rs/transform.rs note). The module path
// `glean_core::rts::traverse` is already pinned by the shipped consumer
// (schema.rs:582 `use glean_core::rts::traverse::gen_traversal;`).
// REVIEW-PASS NOTE (fix round): findings A1/A2/A3/A4/B1-B4/B6 applied in
// one pass. confidence=medium pending re-review of the A1 twin fix, the
// A2 emission-discipline restructure, and the B1 borrow fix. No new
// markers: the A2 guard is a plain assert_eq! (not a TODO/BUG/PERF(port)
// marker), so the marker counts above still read ZERO.
// PORT STATUS: confidence=medium todos=0
