// Ported from: glean/hs/Glean/RTS/Typecheck.hs (251 lines)
// Manifest unit: rust/glean-core/src/rts/typecheck.rs (migration/
// manifest.tsv line 24, crate glean-core). RULE §4 (naming): one source
// file → one target file; the manifest path is authoritative.
//
// Dependency truth (migration/depmap/edges.tsv — checked, not assumed):
//   OUT (Typecheck.hs is the source of edges 1296-1300):
//     :1296 → glean/angle/Glean/Angle/Types.hs — crate glean-core →
//             glean-angle. Typecheck.hs:18 is `import Glean.Angle.Types
//             hiding (Type)` — the constructors are consumed, the ANGLE
//             Type alias is deliberately NOT (the RTS instantiates its
//             own). Port image: `use glean_angle::angle::types::Type_`
//             only — no Angle `Type` import (the import-direction lesson;
//             the RTS `Type` alias lives in crate::rts::types, edge :1300).
//     :1297 → glean/bytecode/Glean/Bytecode/Types.hs — crate edge
//             glean-core → glean-bytecode (the edge sibling
//             rust/glean-bytecode/src/bytecode/types.rs was SHIPPED for).
//     :1298 → glean/hs/Glean/RTS/Bytecode/Code.hs — SHIPPED at
//             rust/glean-core/src/rts/bytecode/code.rs.
//     :1299 → glean/hs/Glean/RTS/Foreign/Bytecode.hs — SHIPPED at
//             rust/glean-core/src/rts/foreign/bytecode.rs.
//     :1300 → glean/hs/Glean/RTS/Types.hs — SHIPPED at
//             rust/glean-core/src/rts/types.rs.
//     The generated module Glean.RTS.Bytecode.Gen.Issue
//     (Typecheck.hs:23 `import Glean.RTS.Bytecode.Gen.Issue`) carries no
//     edges.tsv row: generated code is build output (RULE §4: "never in
//     the manifest"), owner = the generator units at manifest rows
//     150-152 — ASSUMED-IMPORT below (the same seam code.rs and
//     traverse.rs already assume).
//   IN (Typecheck.hs is the target of edges 382/402/1259/1288):
//     Database/Schema.hs (→ rust/glean-db/src/database/schema.rs,
//     SHIPPED — it pins this unit's public shape, see DOWNSTREAM
//     CONTRACT below), Database/Schema/Types.hs (→ rust/glean-db/src/
//     database/schema/types.rs, SHIPPED — edge 402's second shipped
//     consumer, the predicate_typecheck field, see DOWNSTREAM CONTRACT
//     below), RTS/Foreign/Inventory.hs (SHIPPED — holds the
//     CompiledTypecheck stand-in, see KIND-TAG RECONCILIATION),
//     RTS/Foreign/Typecheck.hs (not yet shipped).
//
// INVENTORY (RULE §5): ONE row governs this unit — inventory.tsv row 81
// (line 81), followed field-for-field:
//   sites:   "7 mdo sites (:59,:67,:92,:110,:124,:234,:242)" — confirmed
//            against the source (RecursiveDo, Typecheck.hs:9): :59
//            (ArrayTy `_ -> mdo`), :67 (SumTy), :92 (SetTy non-byte
//            `local $ \set -> mdo`), :110 (MaybeTy), :124 (tcEnum),
//            :234 (checkSignature's `local $ \key_size -> mdo`), :242
//            (check). 11 static `x <- label` bind points across them
//            (2+2+2+2+1+0+2 — site :234 binds NO labels; the Sum alt bind
//            is one static site inside forM).
//   class:   CONTROL_FLOW.
//   target:  "two-pass label allocation (shared codegen IR); identity via
//            disassemble; typecheck bytecode is on-disk-independent but
//            still verified old-vs-new" — discharged exactly as the
//            shipped sibling rts/traverse.rs (inventory.tsv row 80)
//            discharged its row: pass 1 = the pure twin walkers
//            `tc_allocations`/`tc_leaves_block_empty` plus the arithmetic knot
//            predictions taken from `cur_label` (the "shared codegen IR"
//            is the LabelPlan mirror, the row-42-validated pattern);
//            pass 2 = the emitters, whose bind points allocate through
//            the REAL `Code::label` and whose every emission advances the
//            plan in lockstep. EMISSION DISCIPLINE (the sibling's A2,
//            stated option, re-hosted): an emission operand names a
//            block either (a) whose bind has ALREADY run — then the
//            operand is driven from the REAL mirror_label value (the
//            ArrayTy/SetTy DecrAndJumpIfNot0 `loop` operand), or (b) not
//            yet minted — a forward ref (every JumpIf0 `end`, the Sum
//            Select's `alts`, the Maybe Select's `[end,just]`, each `jump
//            end`, tcEnum's `end`, check's `bad`/`ok`), where the operand
//            keeps its pass-1 prediction and the prediction-vs-real
//            check at the bind is a RELEASE-ACTIVE assert_eq! (not
//            debug_assert_eq!): a divergence panics inside
//            check_signature, so wrong bytecode can never reach a
//            caller. What stays debug-only is the plan-vs-Code canary
//            inside mirror_label; the prediction checks themselves are
//            release-enforced. The old-vs-new disassemble identity
//            tables live at each MDO SITE block; the row's "still
//            verified old-vs-new" is RULE §0 (A1)'s promotion gate — the
//            obligation is recorded here in the header, not as a marker
//            (the supply.rs/code.rs/traverse.rs precedent).
//   evidence column: ":67-78 tc (SumTy fields) = mdo — select sel alts
//            before alts <- forM ...; :110-118 MaybeTy select [end,just]
//            before just<-label/end<-label; :124+ tcEnum mdo; :234-239
//            local $ \key_size -> mdo knot; :242-248 check bad/ok
//            labels" — each is honored verbatim below; note the row's
//            own cross-file deltas this file keeps where Traverse.hs
//            differs: Typecheck's Sum `select sel alts` has NO fallback
//            target (Traverse's appends `unknownAlt = [end]`),
//            tcEnum's raise sits INSIDE the local (Sum's is outside),
//            and Typecheck's SetTy emits `newSet` BEFORE `jumpIf0`
//            (Traverse's SetRep has no newSet at all).
// Divergence from the row: NONE — followed field-for-field. No
// INVENTORY-BUG-CANDIDATE.
//
// SIBLING CONTRACTS (RULE §4; DONE units' real APIs read, never
// re-declared):
//   - crate::rts::bytecode::code (SHIPPED): Code (the monad-as-struct;
//     constant/local/output/label/issue/issue_end_block), Optimised(..),
//     Label re-exported from glean_bytecode, the `generate!` macro
//     (#[macro_export], crate-root path) — its BIND-point label rule
//     (`Code::label` doc: fresh id iff the current block is non-empty,
//     else alias — Code.hs:483-488) and new_block's implicit Jump
//     (Code.hs:459-468) are the semantics this file's plan mirrors.
//     `Code a` in the source's signatures maps to "&mut Code threaded
//     through the methods" (code.rs DESIGN (1)) — which is why this
//     file's every Code-monad action takes `&mut Code` as its context
//     parameter.
//   - crate::rts::bytecode::supply (SHIPPED): Supply trait + RegSupply —
//     Typecheck is a Supply consumer ONLY through Code::local/generate's
//     pull closures (Supply.hs has no direct call site here); pull1/
//     pull2 below are the one/two-binding (pull, body) forms of the
//     CodeGen walk (code.rs DESIGN (2)).
//   - crate::rts::types (SHIPPED): Type = Type_<(), PidRef, ExpandedType>
//     (Types.hs:107), FieldDef, Pid (pub i64), deref_type (Types.hs:
//     177-179). The Type CONSTRUCTORS come from glean_angle's Type_
//     (edge :1296), the RTS instantiation from here (edge :1300) —
//     Typecheck.hs:18's `hiding (Type)` maps to importing `Type_` and
//     NOT any Angle `Type` alias.
//   - crate::rts::foreign::bytecode (SHIPPED): Subroutine<S> generic in
//     the phantom marker (Bytecode.hs:55); `with_ptr` is NOT used here —
//     Typecheck.hs only CONSTRUCTS the subroutine, never calls it. Its
//     header (:113-116) pins that the phantom markers "live with their
//     owners' units, not here" — so `data CompiledTypecheck`
//     (Typecheck.hs:26) is defined by THIS file (KIND-TAG
//     RECONCILIATION below).
//   - glean_bytecode::bytecode::types (SHIPPED): Register<T: TyTag>,
//     Label(pub i64), Fun<Args>/TyTuple, cast_register, the Word/
//     WordPtr/DataPtr/BinaryOutputPtr markers, Ordered (the instances
//     Word+DataPtr are what jumpIfLt's context types through).
//   - crate::rts::traverse.rs (SHIPPED, manifest row 23): the mdo
//     translation PATTERN this file re-hosts — LabelPlan + mirror
//     helpers + cur_label + pass-1 twin walker + release-active bind
//     asserts. Same crate, but its plan/helpers are module-PRIVATE (the
//     sibling's own header records the re-host chain: transform.rs →
//     traverse.rs, and crate direction forbids nothing here — the
//     privacy does), so the pattern is re-hosted a second time rather
//     than imported. All semantic rulings are the sibling's verbatim;
//     the one NEW finding this file adds is recorded at the MaybeTy
//     site (the only DATA-DEPENDENT bind in the family — see THE MDO
//     TRANSLATION, fact 3).
//
// ASSUMED-IMPORT (RULE §4 B4 draft-stub policy; owner: the generator
// port, manifest rows 150-152 — build output, never hand-translated
// here): `crate::rts::bytecode::gen::issue` — the generated Issue module
// (source Glean.RTS.Bytecode.Gen.Issue, generated by
// Generate/Haskell.hs genModule "Issue", Haskell.hs:72-79). The symbol
// contract is the one query/transform.rs first sketched and
// rts/traverse.rs EXTENDED (its header pins 12 symbols, all
// cross-referenced, not repeated); THIS file extends the assumed
// surface further (consumer-extension rule, RULE §4 B4: recorded by the
// consumer, cross-referencing the first sketch). All thirteen confirmed
// against the generated file on disk, at the path traverse.rs pinned:
// ~/glean-build/Glean/glean/hs/Glean/RTS/Bytecode/Gen/Issue.hs
// (signatures verbatim, snake_cased per the sibling contract):
//     input_bytes(begin, end, size)            InputBytes    :80-83 (issue)
//       Register 'DataPtr -> Register 'DataPtr -> Register 'Word -> Code ()
//     input_skip_untrusted_string(begin, end)  InputSkipUntrustedString
//                                                            :84-87 (issue)
//       Register 'DataPtr -> Register 'DataPtr -> Code ()
//     output_nat(src, output)                  OutputNat    :109-112 (issue)
//       Register 'Word -> Register 'BinaryOutputPtr -> Code ()
//     output_bytes(ptr, end, output)           OutputBytes  :125-128 (issue)
//       Register 'DataPtr -> Register 'DataPtr -> Register 'BinaryOutputPtr
//       -> Code ()
//     get_output_size(output, dst)             GetOutputSize :149-152 (issue)
//       Register 'BinaryOutputPtr -> Register 'Word -> Code ()
//     move_(src, dst)                          Move         :162-165 (issue)
//       `move :: Register a -> Register a -> Code ()` — no context; the
//       Rust wrapper appends `_` (Rust keyword). Poly tag: both
//       arguments share one TyTag.
//     jump_if_ne(reg1, reg2, tgt)              JumpIfNe     :210-213 (issue)
//       `Register a -> Register a -> Label -> Code ()` — no context; poly
//       tag <T: TyTag>.
//     jump_if_lt(reg1, reg2, tgt)              JumpIfLt     :222-225 (issue)
//       `Ordered a => Register a -> Register a -> Label -> Code ()` — the
//       context rides the generic (<T: Ordered>), Generate/Haskell.hs:
//       231-237 prepends `context insnContext`.
//     call_fun_0_1(fun, args1)                 CallFun_0_1  :238-241 (issue)
//       Register ('Fun '[ 'WordPtr ]) -> Register 'Word -> Code ()
//     call_fun_1_1(fun, args1, args2)          CallFun_1_1  :246-249 (issue)
//       Register ('Fun '[ 'Word,'WordPtr ]) -> Register 'Word ->
//       Register 'Word -> Code ()
//     call_fun_1_0(fun, args1)                 CallFun_1_0  :250-253 (issue)
//       Register ('Fun '[ 'Word ]) -> Register 'Word -> Code ()
//     call_fun_2_1(fun, args1, args2, args3)   CallFun_2_1  :254-257 (issue)
//       Register ('Fun '[ 'Word,'Word,'WordPtr ]) -> Register 'Word x3
//       -> Code ()
//     call_fun_3_0(fun, args1, args2, args3)   CallFun_3_0  :262-265 (issue)
//       Register ('Fun '[ 'Word,'Word,'Word ]) -> Register 'Word x3
//       -> Code ()
// (call_fun_2_0 — CallFun_2_0 :258-261 — is already in traverse.rs's
// pinned list; this file consumes it without extending it.) All take
// `&mut Code` LAST, return () — the in-tree Code-monad-action
// convention (transform.rs's pinned contract, traverse.rs's re-host).
// Effects per def/Instruction.hs: Jump :264 / Raise :388 / Ret :407
// carry [EndBlock] → issueEndBlock; every symbol above is effect-less →
// issue.
//
// B5 (RULE §2 callFun row): the SysCalls record's callFun_N_M calls stay
// EMITTERS over the opaque `Register ('Fun …)` handles the generate
// supply hands out — nothing here types a raw C++ entry point as
// `extern "C" fn` (Rust never invokes them; the C++ evaluator does).
//
// THE MDO TRANSLATION — the A1 contract (RULE §0 A1 / RULE §2
// RecursiveDo row: "IDs allocated at BIND (knot) points, never at mint
// points; every `x <- label` site emits its block split (implicit
// Jump)").
//
// The source's seven mdo blocks tie their knots through HASKELL
// LAZINESS (the derivation is traverse.rs's, re-derived from the same
// pinned sources): `issue insn` stores the instruction THUNK in a lazy
// cons field (Code.hs:491-492; State.Strict forces the state RECORD to
// WHNF, never its fields), the `{-# UNPACK #-} !Label` operand fields
// (generated Instruction.hs) force the knot only when the layout pass
// scrutinizes the instructions (Code.hs:301-307), LONG after every
// `x <- label` bind has executed — so each knot value is exactly the
// BIND-point `label` return: the id of the block starting at the bind
// position, with a fresh `succ csLabel` allocated IFF the current block
// is non-empty at the bind, and NOTHING allocated (alias) on an empty
// one (Code.hs:483-488).
//
// Three facts make the forward ids computable eagerly at every site —
// facts 1-2 are traverse.rs's (adapted to this file's emissions), fact
// 3 is NEW here:
//   1. (sibling fact 1, adapted) Every bind point's close/alias choice
//      is forced by STRUCTURAL emissions alone — statements emitted
//      unconditionally by the source itself — for SIX of the seven
//      sites: the ArrayTy/SetTy loop binds follow inputNat+outputNat
//      (+newSet)+jumpIf0 (all `issue`, :53-54/:60, :93-94), their end
//      binds follow decrAndJumpIfNot0 (:63/:99 — issued unconditionally
//      after the body), the Sum/tcEnum/check binds each follow an
//      EndBlock close (raise :72/:130/:248, jump :76/:246 —
//      issueEndBlock resets csInsns, Code.hs:494-496).
//   2. (sibling fact 2, adapted) The intervening allocations are
//      TYPE-DRIVEN: tc's emission is a pure function of the Type, so
//      the count between a forward reference and its bind is computed
//      by the pure twin walker `tc_allocations` — structure-for-
//      structure with `tc`, same guards, same arms.
//   3. (NEW, this file) ONE bind in the family is DATA-DEPENDENT:
//      MaybeTy's `end <- label` (:118) is preceded by `tc ty` (:117)
//      and nothing else — so per the TRUE Code.hs:484-489 rule it
//      ALIASES (= the raise's L+1) iff the OPEN BLOCK IS EMPTY after
//      `tc ty`'s emissions, and CLOSES (allocates) iff the body left
//      instructions in it. The predicate is NOT "the body emitted
//      anything": every issueEndBlock close (raise/jump — newBlock
//      allocates UNCONDITIONALLY, even on an empty block, Code.hs:
//      459-468) and every closing end bind starts a fresh EMPTY block,
//      so bodies like tcEnum (BooleanTy/EnumeratedTy), SumTy, a nested
//      MaybeTy and ArrayTy-over-a-non-byte-element emit plenty yet end
//      on an empty open block (their own end bind or raise closed
//      theirs); only bodies that leave instructions in the open block —
//      Byte/Nat/String/Predicate (issue-only), ArrayTy over a
//      byte-deref element (:56-58), SetTy BOTH arms (byte :80-89
//      issue-only; the non-byte arm emits setToArray+freeSet AFTER its
//      end-close, :101-102), and records whose last state-forcing field
//      is one of those — close it. Hence the twin pair
//      `tc_allocations` + `tc_leaves_block_empty` and the conditional
//      end prediction at that site; every other bind's prediction is
//      unconditional.
// The plan (LabelPlan) is the mirror of Code's PRIVATE label state —
// the pair (cs_label, block-has-insns) — kept in lockstep by mirroring
// EVERY emission this file makes (the mirror helpers; plan FIRST, then
// Code — the sibling rule verbatim). The one emission code.rs makes
// internally is `output`'s ResetOutput (Code.hs:185-188); both of this
// file's output() sites mirror it with an explicit plan.issue() BEFORE
// the call (or, at check_signature, by initializing the plan at the
// callback-entry state), so the plan is exact at every point rather
// than self-healing.
//
// DOWNSTREAM CONTRACT (pinned by the SHIPPED consumer, never
// re-declared): rust/glean-db/src/database/schema.rs:156-157 (manifest
// row 98) pins
//     pub fn check_signature(key: &Type, val: &Type)
//         -> Subroutine<CompiledTypecheck>;   // rts/typecheck.rs (row 24)
// with the ruling "IO -> infallible per the allocation-guard convention
// — glean_inventory_new's abort shape"; its call site schema.rs:1913
// (`let typecheck = check_signature(&key_type, &value_type);`, the
// Schema.hs:846 site) consumes it bare. This file honors that shape;
// the impossible Err conversion is reasoned at check_signature.
//
// The SECOND shipped consumer (edge 402's target, Database/Schema/
// Types.hs → rust/glean-db/src/database/schema/types.rs): :524
// `use glean_core::rts::foreign::inventory::{CompiledTypecheck,
// CompiledTraversal, Inventory};` (via the stand-in until the KIND-TAG
// swap lands), :765-767
//     pub predicate_typecheck: Subroutine<CompiledTypecheck>,
// (Types.hs:171 — fed from this unit's check_signature output at
// schema.rs:1923), and its import-map entry for source :402 (types.rs:
// 86) pinning this unit as manifest row 24 — written "NOT SHIPPED" and
// superseded by this unit's landing.
//
// The companion `checkType` keeps the source's export list (:11-14:
// CompiledTypecheck, checkType, checkSignature — those three and
// nothing else are pub here; SysCalls/typecheck/tc/tcEnum/check are
// module-private exactly as the Haskell export list omits them).

use glean_angle::angle::types::Type_;
use glean_bytecode::bytecode::types::{
    cast_register, BinaryOutputPtr, DataPtr, Fun, Label, Ordered, Register, TyTag, Word, WordPtr,
};
use crate::ffi::FfiError;
use crate::rts::bytecode::code::{Code, Optimised};
use crate::rts::bytecode::gen::issue as generated;
use crate::rts::bytecode::supply::{RegSupply, Supply};
use crate::rts::foreign::bytecode::Subroutine;
use crate::rts::types::{deref_type, Pid, PidRef, Type};

// -----------------------------------------------------------------------------
// CompiledTypecheck (Typecheck.hs:25-26)
// -----------------------------------------------------------------------------

/// Type tag for Subroutine (source doc, Typecheck.hs:25).
///
/// `data CompiledTypecheck` — a phantom kind marker; Subroutine is
/// polymorphic in it ("to avoid us mixing up the two", Foreign/Bytecode.hs:
/// 52-54). No deriving in the source, none here. See the header KIND-TAG
/// RECONCILIATION for the inventory.rs stand-in that defers to this
/// definition.
pub struct CompiledTypecheck;

// -----------------------------------------------------------------------------
// The LabelPlan — the pass-1 mirror of Code's private label state
// (header: THE MDO TRANSLATION; inventory.tsv:81's "shared codegen IR")
// -----------------------------------------------------------------------------

/// The mirror of Code's private (cs_label, block-has-insns) state: `next`
/// mirrors cs_label; the next allocation mints cs_label+1 (Code.hs:462),
/// `block_open` says whether the current block holds instructions (non-empty
/// csInsns).
/// Kept true by mirroring EVERY emission this file makes (the mirror
/// helpers below); checked at the binds by the debug canary in
/// [`mirror_label`] and — for the prediction arithmetic the mdo sites read
/// off it — by the release-active prediction-vs-real asserts (header:
/// INVENTORY, EMISSION DISCIPLINE).
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
/// Code::local's (pull, body) pair (code.rs DESIGN (2)); every
/// single-register `local $ \r -> ...` in Typecheck.hs binds exactly one
/// register (:39/:44/:47/:52/:56/:81/:91/:104/:111/:126).
fn pull1<A>(sup: &mut RegSupply) -> A
where
    RegSupply: Supply<A>,
{
    let (a, next) = sup.supply();
    *sup = next;
    a
}

/// The two-binding form for `local $ \set ptr -> ...` (Typecheck.hs:82)
/// — one supply call per lambda parameter, in the lambda's order (set
/// then ptr), mixed tags per parameter exactly as the polymorphic
/// `Supply (Register a) RegSupply` instance allows (code.rs DESIGN (2),
/// the round-2 pilot's mixed-kind-local contract).
fn pull2<A, B>(sup: &mut RegSupply) -> (A, B)
where
    RegSupply: Supply<A>,
    RegSupply: Supply<B>,
{
    let (a, next) = sup.supply();
    *sup = next;
    let (b, next) = sup.supply();
    *sup = next;
    (a, b)
}

// -----------------------------------------------------------------------------
// The Issue wrappers under the LabelPlan — the source's unqualified names
// (Typecheck.hs uses the generated combinators bare via its :23 import),
// each delegating to the generated wrapper (header ASSUMED-IMPORT) with
// plan mirroring. Wrapper shapes follow traverse.rs's re-hosted contract.
// -----------------------------------------------------------------------------

/// `inputNat :: Register 'DataPtr -> Register 'DataPtr -> Register 'Word
/// -> Code ()` (generated Issue.hs:72-74; def/Instruction.hs:103-106).
fn input_nat(
    input: Register<DataPtr>,
    inputend: Register<DataPtr>,
    dst: Register<Word>,
    code: &mut Code,
    plan: &mut LabelPlan,
) {
    mirror_issue(code, plan, |code| generated::input_nat(input, inputend, dst, code));
}

/// `inputBytes :: Register 'DataPtr -> Register 'DataPtr -> Register 'Word
/// -> Code ()` (generated Issue.hs:80-83; def/Instruction.hs:115-118 —
/// "Advance begin by size bytes, and bounds-check against end").
fn input_bytes(
    begin: Register<DataPtr>,
    end: Register<DataPtr>,
    size: Register<Word>,
    code: &mut Code,
    plan: &mut LabelPlan,
) {
    mirror_issue(code, plan, |code| generated::input_bytes(begin, end, size, code));
}

/// `inputSkipUntrustedString :: Register 'DataPtr -> Register 'DataPtr ->
/// Code ()` (generated Issue.hs:84-87; def/Instruction.hs:121-123).
fn input_skip_untrusted_string(
    begin: Register<DataPtr>,
    end: Register<DataPtr>,
    code: &mut Code,
    plan: &mut LabelPlan,
) {
    mirror_issue(code, plan, |code| generated::input_skip_untrusted_string(begin, end, code));
}

/// `outputNat :: Register 'Word -> Register 'BinaryOutputPtr -> Code ()`
/// (generated Issue.hs:109-112; def/Instruction.hs:157-159).
fn output_nat(src: Register<Word>, output: Register<BinaryOutputPtr>, code: &mut Code, plan: &mut LabelPlan) {
    mirror_issue(code, plan, |code| generated::output_nat(src, output, code));
}

/// `outputBytes :: Register 'DataPtr -> Register 'DataPtr ->
/// Register 'BinaryOutputPtr -> Code ()` (generated Issue.hs:125-128;
/// def/Instruction.hs:176-179).
fn output_bytes(
    ptr: Register<DataPtr>,
    end: Register<DataPtr>,
    output: Register<BinaryOutputPtr>,
    code: &mut Code,
    plan: &mut LabelPlan,
) {
    mirror_issue(code, plan, |code| generated::output_bytes(ptr, end, output, code));
}

/// `move :: Register a -> Register a -> Code ()` (generated Issue.hs:
/// 162-165; def/Instruction.hs:229-231, polyReg "a"). The `_` suffix is
/// the Rust-keyword rendering of the source name (header ASSUMED-IMPORT).
fn move_<T: TyTag>(src: Register<T>, dst: Register<T>, code: &mut Code, plan: &mut LabelPlan) {
    mirror_issue(code, plan, |code| generated::move_(src, dst, code));
}

/// `getOutputSize :: Register 'BinaryOutputPtr -> Register 'Word ->
/// Code ()` (generated Issue.hs:149-152; def/Instruction.hs:213-215).
fn get_output_size(output: Register<BinaryOutputPtr>, dst: Register<Word>, code: &mut Code, plan: &mut LabelPlan) {
    mirror_issue(code, plan, |code| generated::get_output_size(output, dst, code));
}

/// `jump :: Label -> Code ()` (generated Issue.hs:190-193; Jump,
/// def/Instruction.hs:264-266 — [EndBlock] → issueEndBlock).
fn jump(tgt: Label, code: &mut Code, plan: &mut LabelPlan) {
    mirror_issue_end_block(code, plan, |code| generated::jump(tgt, code));
}

/// `jumpIf0 :: Register 'Word -> Label -> Code ()` (generated Issue.hs:
/// 198-201; def/Instruction.hs:271-273 — no effects → issue).
fn jump_if0(reg: Register<Word>, tgt: Label, code: &mut Code, plan: &mut LabelPlan) {
    mirror_issue(code, plan, |code| generated::jump_if0(reg, tgt, code));
}

/// `jumpIfNe :: Register a -> Register a -> Label -> Code ()` (generated
/// Issue.hs:210-213; def/Instruction.hs:287-290 — no context, poly tag).
/// The source's `castRegister reg1` (Typecheck.hs:243) coerces to
/// jumpIfNe's `Register a` — the wrapper's T is that same coercion, so no
/// explicit cast_register call remains (RULE §2 A8 is about dispatch
/// SHAPE on named types; this is a tag-only coerce, sibling precedent:
/// traverse.rs's call_fun_2_0 sites).
fn jump_if_ne<T: TyTag>(reg1: Register<T>, reg2: Register<T>, tgt: Label, code: &mut Code, plan: &mut LabelPlan) {
    mirror_issue(code, plan, |code| generated::jump_if_ne(reg1, reg2, tgt, code));
}

/// `jumpIfLt :: Ordered a => Register a -> Register a -> Label -> Code ()`
/// (generated Issue.hs:222-225; def/Instruction.hs:305-308 — the
/// `Ordered a` context rides the generic, Generate/Haskell.hs:231-237).
/// Both call sites here are Register 'Word (:129/:243 via castRegister —
/// check's operands are DataPtr-cast-to-`a`), which types through the
/// shipped `Ordered for Word` instance.
fn jump_if_lt<T: Ordered>(reg1: Register<T>, reg2: Register<T>, tgt: Label, code: &mut Code, plan: &mut LabelPlan) {
    mirror_issue(code, plan, |code| generated::jump_if_lt(reg1, reg2, tgt, code));
}

/// `decrAndJumpIfNot0 :: Register 'Word -> Label -> Code ()` (generated
/// Issue.hs:230-232; def/Instruction.hs:317-319 — no effects → issue).
fn decr_and_jump_if_not0(reg: Register<Word>, tgt: Label, code: &mut Code, plan: &mut LabelPlan) {
    mirror_issue(code, plan, |code| generated::decr_and_jump_if_not0(reg, tgt, code));
}

/// `select :: Register 'Word -> [Label] -> Code ()` (generated Issue.hs:
/// 290-293; def/Instruction.hs:383-385 — "Does nothing if the index is
/// out of range", which is why Typecheck's Sum needs NO fallback target).
fn select(sel: Register<Word>, tgts: &[Label], code: &mut Code, plan: &mut LabelPlan) {
    mirror_issue(code, plan, |code| generated::select(sel, tgts, code));
}

/// `raise :: ByteString -> Code ()` (generated Issue.hs:294-297; Raise,
/// def/Instruction.hs:388-389 — [EndBlock] → issueEndBlock; the ImmLit
/// argument is hoisted through Code::literal INSIDE the generated
/// wrapper, label-neutral: literal touches only csLiterals, Code.hs:
/// 470-481 — no block state, so the plan's issue_end_block advance before
/// the call stays exact).
fn raise(msg: &[u8], code: &mut Code, plan: &mut LabelPlan) {
    mirror_issue_end_block(code, plan, |code| generated::raise(msg, code));
}

/// `callFun_0_1 :: Register ('Fun '[ 'WordPtr ]) -> Register 'Word ->
/// Code ()` (generated Issue.hs:238-241; def/Instruction.hs:326-328).
/// B5: an emitter over the opaque `Register ('Fun …)` handle — never an
/// extern "C" fn type.
fn call_fun_0_1(
    fun: Register<Fun<(WordPtr,)>>,
    args1: Register<Word>,
    code: &mut Code,
    plan: &mut LabelPlan,
) {
    mirror_issue(code, plan, |code| generated::call_fun_0_1(fun, args1, code));
}

/// `callFun_1_1 :: Register ('Fun '[ 'Word,'WordPtr ]) -> Register 'Word
/// -> Register 'Word -> Code ()` (generated Issue.hs:246-249;
/// def/Instruction.hs:334-336).
fn call_fun_1_1(
    fun: Register<Fun<(Word, WordPtr)>>,
    args1: Register<Word>,
    args2: Register<Word>,
    code: &mut Code,
    plan: &mut LabelPlan,
) {
    mirror_issue(code, plan, |code| generated::call_fun_1_1(fun, args1, args2, code));
}

/// `callFun_1_0 :: Register ('Fun '[ 'Word ]) -> Register 'Word ->
/// Code ()` (generated Issue.hs:250-253; def/Instruction.hs:338-340).
fn call_fun_1_0(fun: Register<Fun<(Word,)>>, args1: Register<Word>, code: &mut Code, plan: &mut LabelPlan) {
    mirror_issue(code, plan, |code| generated::call_fun_1_0(fun, args1, code));
}

/// `callFun_2_1 :: Register ('Fun '[ 'Word,'Word,'WordPtr ]) ->
/// Register 'Word -> Register 'Word -> Register 'Word -> Code ()`
/// (generated Issue.hs:254-257; def/Instruction.hs:344-346 — "Call an
/// std::function which takes two 64-bit arguments and returns one 64-bit
/// result", source comment).
fn call_fun_2_1(
    fun: Register<Fun<(Word, Word, WordPtr)>>,
    args1: Register<Word>,
    args2: Register<Word>,
    args3: Register<Word>,
    code: &mut Code,
    plan: &mut LabelPlan,
) {
    mirror_issue(code, plan, |code| generated::call_fun_2_1(fun, args1, args2, args3, code));
}

/// `callFun_2_0 :: Register ('Fun '[ 'Word,'Word ]) -> Register 'Word ->
/// Register 'Word -> Code ()` (generated Issue.hs:258-261;
/// def/Instruction.hs:348-350) — already pinned by traverse.rs's
/// assumed-import list; re-wrapped here with plan mirroring.
fn call_fun_2_0(
    fun: Register<Fun<(Word, Word)>>,
    args1: Register<Word>,
    args2: Register<Word>,
    code: &mut Code,
    plan: &mut LabelPlan,
) {
    mirror_issue(code, plan, |code| generated::call_fun_2_0(fun, args1, args2, code));
}

/// `callFun_3_0 :: Register ('Fun '[ 'Word,'Word,'Word ]) ->
/// Register 'Word -> Register 'Word -> Register 'Word -> Code ()`
/// (generated Issue.hs:262-265; def/Instruction.hs:352-354).
fn call_fun_3_0(
    fun: Register<Fun<(Word, Word, Word)>>,
    args1: Register<Word>,
    args2: Register<Word>,
    args3: Register<Word>,
    code: &mut Code,
    plan: &mut LabelPlan,
) {
    mirror_issue(code, plan, |code| generated::call_fun_3_0(fun, args1, args2, args3, code));
}

/// `ret :: Code ()` (generated Issue.hs:313-315; Ret,
/// def/Instruction.hs:407 — [EndBlock, Return] → issueEndBlock).
fn ret(code: &mut Code, plan: &mut LabelPlan) {
    mirror_issue_end_block(code, plan, |code| generated::ret(code));
}

// -----------------------------------------------------------------------------
// The pass-1 twin walkers (inventory.tsv:81's "two-pass label allocation")
// -----------------------------------------------------------------------------

/// A(ty) — the number of Label allocations `tc ty` performs (the pass-1
/// twin of [`tc`; header: THE MDO TRANSLATION, fact 2). Every arm mirrors
/// tc's guard and structure exactly (source equation order preserved):
///   - Byte/Nat/String/Predicate: issue-only bodies (constant + local with
///     move/inputBytes/outputBytes :38-43; local inputNat/outputNat :44-46;
///     local move/inputSkipUntrustedString/outputBytes :47-50; constant +
///     local inputNat/rename/outputNat :104-108 — constant() touches only
///     the constant table, Code.hs:148-160) → 0;
///   - Array (guarded by the `case derefType elty` split :55): a ByteTy
///     element body is the issue-only local (:56-58), ZERO allocations;
///     other elements: 2 (the loop bind's close — inputNat+outputNat+
///     jumpIf0 precede it; the end bind's close — the decr precedes it) +
///     the element body (:62);
///   - Record: the fields in order (:66);
///   - Sum: 1 (the raise's close, :72) + n (one `jump end` close per alt,
///     :76) + the alternatives — the alt binds and the end bind ALIAS
///     (they follow EndBlock closes, fact 1);
///   - Set: ByteTy element → 0 (issue-only :80-89); otherwise 2 (same two
///     closes as Array: newSet+jumpIf0 precede the loop bind, the decr
///     precedes the end bind) + the element body (:97);
///   - Named: the expansion (:109);
///   - Maybe: 1 (the raise's close, :115) + A + [the body leaves the
///     open block NON-empty] — the end bind (:118) CLOSES iff `tc ty`
///     leaves instructions in the block and ALIASES iff it leaves it
///     empty (fact 3, the TRUE Code.hs:484-489 rule — NOT "emitted
///     anything": BooleanTy emits via tcEnum yet aliases after tcEnum's
///     own raise); the just bind (:116) aliases after the raise;
///   - Enumerated/Boolean: tcEnum → 1 (the raise's close, :130; the end
///     bind :131 aliases after it — the raise is INSIDE the local);
///   - TyVar/HasTy/HasKey/ElementsOf: the twin of tc's per-access
///     `error "typecheck"` (:122) — panic with the same message (RULE §2
///     partial-functions row / A6).
/// i64 (not traverse.rs's u64-then-cast twin shape): every consumer adds
/// the count to a `Label(pub i64)` — mechanics only, semantics identical.
fn tc_allocations(ty: &Type) -> i64 {
    match ty {
        Type_::ByteTy | Type_::NatTy | Type_::StringTy => 0,
        Type_::ArrayTy(elty) => {
            // tc's `case derefType elty of` split (:55): the ByteTy arm is
            // the issue-only local (:56-58) — the mdo exists only in the
            // `_ ->` arm (:59-65). The dispatch is on the DEREFERENCED
            // element (A8), the recursion (:62) on the ORIGINAL elty —
            // mirrored exactly.
            match deref_type(elty) {
                Type_::ByteTy => 0,
                _ => 2 + tc_allocations(elty),
            }
        }
        Type_::RecordTy(fields) => fields
            .iter()
            .map(|f| tc_allocations(&f.field_def_type))
            .sum(),
        Type_::SumTy(fields) => {
            1 + fields.len() as i64
                + fields
                    .iter()
                    .map(|f| tc_allocations(&f.field_def_type))
                    .sum::<i64>()
        }
        Type_::SetTy(t) => {
            // tc's SYNTACTIC SetTy split (:80 vs :90 — NO derefType, the
            // A8 asymmetry): the byte arm is issue-only, the mdo exists
            // only for the non-byte arm.
            if matches!(**t, Type_::ByteTy) {
                0
            } else {
                2 + tc_allocations(t)
            }
        }
        Type_::PredicateTy(..) => 0,
        Type_::NamedTy(_, expanded) => tc_allocations(&expanded.expanded_type_type),
        Type_::MaybeTy(t) => {
            // fact 3: the end bind (:118) CLOSES iff `tc ty` leaves the
            // open block NON-empty and ALIASES iff it leaves it EMPTY
            // (the TRUE Code.hs:484-489 rule — NOT "emitted anything":
            // BooleanTy emits via tcEnum yet aliases after tcEnum's own
            // raise). Twin totals — the hand-traced anchors at MDO
            // SITE 4:
            //   maybe bool        → 1 + 1 + 0 = 2
            //   maybe Nat         → 1 + 0 + 1 = 2
            //   maybe (maybe Nat) → 1 + 2 + 0 = 3
            let a = tc_allocations(t);
            1 + a + if tc_leaves_block_empty(t) { 0 } else { 1 }
        }
        Type_::EnumeratedTy(_) | Type_::BooleanTy => 1,
        Type_::TyVar(_) | Type_::HasTy(..) | Type_::HasKey(..) | Type_::ElementsOf(..) => {
            // RULE §2 (A6): the source errors here, per ACCESS (:122) —
            // the twin fires when this arm is dispatched during planning,
            // which is the same generation walk the error fires in.
            //
            // A6 TIMING SHIFT (documented, not restructured): this panic
            // fires during PASS-1 PLANNING — at the prediction sites (the
            // ArrayTy/SetTy/SumTy body counts and the MaybeTy end
            // prediction) — potentially BEFORE the emissions and bind
            // allocations the source's `error "typecheck"` would follow.
            // The source errors at BODY DISPATCH (:62/:97/:117 —
            // `tc elty` / `typecheck ... elty` / `tc ty`), i.e. AFTER the
            // enclosing arm's own inputNat/outputNat issues and, in the
            // Array/Set arms, AFTER the loop bind's allocation (an
            // ArrayTy over a wildcard element errors only after :52-53's
            // issues and the :61 loop bind; the twin panics at the
            // prediction before the :60 jumpIf0 even issues).
            // Unobservable — both this panic and the Haskell exception
            // abort check_signature before any Subroutine exists, and
            // generate discards its output on abort either way — but per
            // RULE §2 (A6) the timing is semantic: noted here, and the
            // emission stream is deliberately NOT restructured to
            // reproduce it.
            panic!("typecheck")
        }
    }
}

/// Is the open block EMPTY after `tc ty` runs? The pass-1 twin companion
/// fact 3 needs — the TRUE Code.hs:484-489 rule, replacing the WRONG
/// "emitted anything" predicate: `end <- label` ALIASES iff the CURRENT
/// OPEN BLOCK IS EMPTY after the body's emissions, and a body that
/// emits plenty can still leave it empty, because every issueEndBlock
/// close (newBlock allocates UNCONDITIONALLY, even on an empty block,
/// Code.hs:459-468/:494-496) and every closing end bind starts a fresh
/// EMPTY block.
///
/// Consumption contract: the twin is consulted ONLY for the MaybeTy end
/// prediction (the [`tc`] MDO SITE 4 and [`tc_allocations`]' MaybeTy
/// arm), where `tc ty` always starts on the MaybeTy raise's fresh EMPTY
/// block — the `true` seed of [`tc_leaves_block_empty_given`]. The +1
/// contribution the prediction consumes is `!tc_leaves_block_empty(t)`,
/// i.e. the end bind CLOSES for exactly these shapes (each
/// entry-independent, which is what makes the twin a pure function of
/// the type):
///   +1 (leaves instructions in the open block): Byte/Nat/String/
///      Predicate (issue-only bodies, :37-50/:104-108); ArrayTy over a
///      byte-deref element (:55-58); SetTy BOTH arms (byte :80-89
///      issue-only; the non-byte arm emits setToArray+freeSet AFTER its
///      end-close, :101-102); and a RecordTy whose last state-forcing
///      field is one of those (the fold below);
///   +0 (leaves the block empty): ArrayTy with a non-byte element (its
///      own end-close :64, nothing after it, :65); SumTy (its end bind
///      :78 aliases the fresh block the last `jump end` :76 allocated);
///      MaybeTy (its end bind :118 aliases an already-empty block or
///      closes one that held instructions — empty either way);
///      EnumeratedTy/BooleanTy (tcEnum's end bind :131 aliases the fresh
///      block its raise :130 allocated); and the instruction-less no-op
///      chains (`RecordTy []` :66, NamedTy over one :109 — the block
///      stands untouched, and it is EMPTY at the consumption site).
/// Hand-traced anchors live at MDO SITE 4 (maybe bool → 2, maybe Nat →
/// 2, maybe (maybe Nat) → 3); the table is additionally swept against a
/// code.rs-semantics replica (see the anchors' note there).
fn tc_leaves_block_empty(ty: &Type) -> bool {
    tc_leaves_block_empty_given(ty, true)
}

/// The entry-parameterized engine behind [`tc_leaves_block_empty`]: is
/// the open block empty after `tc ty`, given it was `entry_empty` on
/// entry? Every constructor except the no-op chains forces a DEFINITE
/// state (issue-only bodies → non-empty; bodies ending on their own
/// close-or-alias → empty), so the recursion is exact; the no-ops
/// (`RecordTy []`, NamedTy over one — :66/:109, `tc` emits nothing)
/// pass the entry state through. `RecordTy` therefore folds its fields
/// IN ORDER over the same open block — it is NOT "last field's value":
/// a no-op last field is transparent (e.g. `RecordTy [NatTy,
/// RecordTy []]` leaves the block NON-empty — NatTy's inputNat and
/// outputNat are still in it), so the earlier fields' state survives.
fn tc_leaves_block_empty_given(ty: &Type, entry_empty: bool) -> bool {
    match ty {
        // issue-only bodies (:37-50, :104-108): the instructions stay in
        // the open block
        Type_::ByteTy | Type_::NatTy | Type_::StringTy | Type_::PredicateTy(..) => false,
        // tc's `case derefType elty` split (:54-55, the A8 deref): a
        // byte element body is the issue-only local (:55-58); any other
        // element's mdo ends with its own end-close (:64) and NOTHING
        // after it (:65) — the fresh block stands empty
        Type_::ArrayTy(elty) => match deref_type(elty) {
            Type_::ByteTy => false,
            _ => true,
        },
        // the end bind (:78) ALIASES the fresh empty block the last
        // `jump end` (:76) allocated (for n = 0, the raise's :72) — see
        // MDO SITE 2's table
        Type_::SumTy(_) => true,
        // BOTH arms leave instructions in the open block: the byte arm
        // (:80-89) is issue-only; the non-byte arm emits setToArray +
        // freeSet AFTER its end-close (:101-102)
        Type_::SetTy(_) => false,
        // pure expansion (:109), no emissions of its own — pass through
        Type_::NamedTy(_, expanded) => {
            tc_leaves_block_empty_given(&expanded.expanded_type_type, entry_empty)
        }
        // the end bind (:118) ALIASES an already-empty block or CLOSES
        // one that held instructions — the open block is empty either way
        Type_::MaybeTy(_) => true,
        // tcEnum's end bind (:131) ALIASES the fresh empty block its
        // raise (:130, INSIDE the local) allocated — see MDO SITE 5's
        // table
        Type_::EnumeratedTy(_) | Type_::BooleanTy => true,
        // mapM_ in sequence (:66) over the SAME open block — the exit is
        // the left-to-right fold, not "last field's value" (a no-op last
        // field is transparent; see the doc comment)
        Type_::RecordTy(fields) => fields
            .iter()
            .fold(entry_empty, |st, f| tc_leaves_block_empty_given(&f.field_def_type, st)),
        // RULE §2 (A6): the source errors here, per ACCESS (:122) — same
        // twin-of-error discipline as tc_allocations' wildcard arm above,
        // INCLUDING its A6 TIMING SHIFT note there (a pass-1 panic,
        // unobservable — generate discards output on abort either way —
        // documented, not restructured).
        Type_::TyVar(_) | Type_::HasTy(..) | Type_::HasKey(..) | Type_::ElementsOf(..) => {
            panic!("typecheck")
        }
    }
}

// -----------------------------------------------------------------------------
// SysCalls (Typecheck.hs:134-182)
// -----------------------------------------------------------------------------

/// The syscall surface `tc` consumes (Typecheck.hs:134-182). NOT in the
/// source's export list (module list :10-14 exports only
/// CompiledTypecheck/checkType/checkSignature) — private here.
///
/// Port shape (two deltas, both recorded): (1) each field's `Code ()`
/// action becomes a closure taking `&mut Code` as its monad context
/// (code.rs DESIGN (1): `Code a` ↦ "&mut-threaded"); (2) the closures
/// additionally take `&mut LabelPlan` — the pass-1 mirror must advance on
/// EVERY emission, and these closures emit (B5 emitters). The field
/// order, names and register roles are the source's verbatim; each
/// closure is the source's right-hand side with callFun_N_M routed
/// through the plan-mirroring wrappers above.
struct SysCalls {
    /// `rename :: Register 'Word -> Register 'Word -> Register 'Word ->
    /// Code ()` (:136-139).
    rename: Box<dyn Fn(Register<Word>, Register<Word>, Register<Word>, &mut Code, &mut LabelPlan)>,
    /// `newSet :: Register 'Word -> Code ()` (:141-143) — "(output) set
    /// token" (source comment).
    new_set: Box<dyn Fn(Register<Word>, &mut Code, &mut LabelPlan)>,
    /// `insertOutputSet :: Register 'Word -> Register 'BinaryOutputPtr ->
    /// Code ()` (:145-148).
    insert_output_set:
        Box<dyn Fn(Register<Word>, Register<BinaryOutputPtr>, &mut Code, &mut LabelPlan)>,
    /// `setToArray :: Register 'Word -> Register 'BinaryOutputPtr ->
    /// Code ()` (:150-153).
    set_to_array: Box<dyn Fn(Register<Word>, Register<BinaryOutputPtr>, &mut Code, &mut LabelPlan)>,
    /// `freeSet :: Register 'Word -> Code ()` (:155-157) — "(invalid after
    /// this call)" (source comment).
    free_set: Box<dyn Fn(Register<Word>, &mut Code, &mut LabelPlan)>,
    /// `newWordSet :: Register 'Word -> Code ()` (:159-161) — "(output)
    /// set token" (source comment).
    new_word_set: Box<dyn Fn(Register<Word>, &mut Code, &mut LabelPlan)>,
    /// `insertBytesWordSet :: Register 'Word -> Register 'DataPtr ->
    /// Register 'DataPtr -> Code ()` (:163-167).
    insert_bytes_word_set:
        Box<dyn Fn(Register<Word>, Register<DataPtr>, Register<DataPtr>, &mut Code, &mut LabelPlan)>,
    /// `wordSetToArray :: Register 'Word -> Register 'BinaryOutputPtr ->
    /// Code ()` (:169-172). WIRED BUT UNUSED: tc never calls this field —
    /// the source record defines it and check_signature wires it
    /// (:227-228) with zero consumers (grep over Typecheck.hs: only the
    /// definition and the wiring). Kept structure-for-structure (RULE §0:
    /// translating, not improving); the never-read-field lint that Step
    /// 4's survey build will see is the honest rendering of that fact.
    #[allow(dead_code)]
    word_set_to_array:
        Box<dyn Fn(Register<Word>, Register<BinaryOutputPtr>, &mut Code, &mut LabelPlan)>,
    /// `byteSetToByteArray :: Register 'Word -> Register 'BinaryOutputPtr
    /// -> Code ()` (:174-177).
    byte_set_to_byte_array:
        Box<dyn Fn(Register<Word>, Register<BinaryOutputPtr>, &mut Code, &mut LabelPlan)>,
    /// `freeWordSet :: Register 'Word -> Code ()` (:179-181) — "(invalid
    /// after this call)" (source comment).
    free_word_set: Box<dyn Fn(Register<Word>, &mut Code, &mut LabelPlan)>,
}

// -----------------------------------------------------------------------------
// typecheck / tc (Typecheck.hs:28-132)
// -----------------------------------------------------------------------------

/// `typecheck :: SysCalls -> Register 'DataPtr -> Register 'DataPtr ->
/// Register 'BinaryOutputPtr -> Type -> Code ()` (Typecheck.hs:28-35) —
/// the curried five-argument function whose body is `tc` (Typecheck.hs:
/// 36): `typecheck syscalls input inputend out = tc`. The curry collapses
/// into this fn's parameter list (the Code-a action TYPE disposition,
/// code.rs DESIGN (1)); the recursive call sites cite the source's
/// `typecheck ... elty` spelling (:97).
///
/// `syscalls@SysCalls{..}` (:35) — the record wildcard's ten field
/// bindings become plain field access on `&SysCalls`.
///
/// HasCallStack: none in the source; no error/vlog whose caller-location
/// it would feed beyond the per-access panics below (which carry the
/// source's exact message text). Dropped with the coverage note.
fn tc(
    syscalls: &SysCalls,
    input: Register<DataPtr>,
    inputend: Register<DataPtr>,
    out: Register<BinaryOutputPtr>,
    ty: &Type,
    code: &mut Code,
    plan: &mut LabelPlan,
) {
    // Equation ORDER is load-bearing: SetTy's byte arm precedes the
    // general SetTy arm; the wildcard is last. Arm-by-arm source lines in
    // the comments.
    match ty {
        // tc ByteTy = do size <- constant 1; local $ \ptr -> do ... (:37-43)
        Type_::ByteTy => {
            // size <- constant 1 (:38) — constant() is label-neutral
            // (Code.hs:148-160 touches only the constant table), so no
            // plan advance.
            let size = code.constant(1);
            code.local(pull1, move |ptr: Register<DataPtr>, code| {
                move_(input, ptr, code, plan); // :40
                input_bytes(input, inputend, size, code, plan); // :41
                output_bytes(ptr, input, out, code, plan); // :42
            });
        }
        // tc NatTy = local $ \reg -> do inputNat ...; outputNat ... (:43-45)
        Type_::NatTy => {
            code.local(pull1, |reg: Register<Word>, code| {
                input_nat(input, inputend, reg, code, plan); // :44
                output_nat(reg, out, code, plan); // :45
            });
        }
        // tc StringTy = local $ \ptr -> do move ...;
        //   inputSkipUntrustedString ...; outputBytes ... (:46-50)
        Type_::StringTy => {
            code.local(pull1, move |ptr: Register<DataPtr>, code| {
                move_(input, ptr, code, plan); // :48
                input_skip_untrusted_string(input, inputend, code, plan); // :49
                output_bytes(ptr, input, out, code, plan); // :50
            });
        }
        // tc (ArrayTy elty) = local $ \size -> do inputNat; outputNat;
        //   case derefType elty of ... (:51-65) — MDO SITE 1 of 7 lives in
        //   the case's `_ ->` arm.
        Type_::ArrayTy(elty) => {
            code.local(pull1, move |size: Register<Word>, code| {
                input_nat(input, inputend, size, code, plan); // :52
                output_nat(size, out, code, plan); // :53
                // RULE §2 (A8): the dispatch is on the DEREFERENCED
                // element (`case derefType elty of`, :54-55) — the exact
                // asymmetry the stress round-2 report validated against
                // SetTy's syntactic match (report-r2.md: "SetTy syntactic
                // vs ArrayTy deref asymmetry reproduced by both"). The
                // recursion at :62 is on the ORIGINAL elty.
                match deref_type(elty) {
                    // ByteTy -> local $ \ptr -> do ... (:55-58)
                    Type_::ByteTy => {
                        code.local(pull1, move |ptr: Register<DataPtr>, code| {
                            move_(input, ptr, code, plan); // :56
                            input_bytes(input, inputend, size, code, plan); // :57
                            output_bytes(ptr, input, out, code, plan); // :58
                        });
                    }
                    // _ -> mdo (:59-65) — MDO SITE 1 of 7 (inventory row 81
                    // site :59). See the site block below.
                    _ => {
                        // ----- MDO SITE 1 of 7 — ArrayTy (Typecheck.hs:
                        // :59-65; inventory.tsv row 81 site 1) -----
                        //
                        // OLD (the knot, :60-64):
                        //   jumpIf0 size end ; loop <- label ; tc elty ;
                        //   decrAndJumpIfNot0 size loop ; end <- label
                        // With csLabel = L at the jumpIf0 (inputNat+
                        // outputNat already emitted into block L, :52-53)
                        // and A = tc_allocations(elty) (the body's
                        // allocations, fact 2):
                        //
                        // OLD disassembly (label ids in BIND order — A1):
                        //   block L:       InputNat input inputend size
                        //                  OutputNat size out
                        //                  JumpIf0 size (L+2+A)
                        //                  Jump (L+1)      [loop bind close]
                        //   block L+1:     <tc elty>
                        //                  DecrAndJumpIfNot0 size (L+1)
                        //                  Jump (L+2+A)    [end bind close]
                        //   block L+2+A:   (continues after the array)
                        // loop = L+1 (the loop bind CLOSES: inputNat+
                        // outputNat+jumpIf0 are in the current block, fact
                        // 1); end = L+2+A (the end bind CLOSES: the decr
                        // was issued after the body unconditionally).
                        //
                        // NEW: identical instruction stream, identical ids.
                        // The binds allocate through mirror_label (the REAL
                        // Code::label — RULE §0 A1: at BIND points, never
                        // at mint). EMISSION DISCIPLINE (header): the
                        // decr's loop operand is the REAL bind value
                        // (loop_real — the knot's real allocation is known
                        // before that emission); the JumpIf0's forward
                        // `end` cannot use the real value (the body has
                        // not run when it is emitted), so it stays the
                        // pass-1 prediction (L+2+A) and is checked against
                        // the real at the end bind with a RELEASE-ACTIVE
                        // assert_eq!. [A1 disassemble-identity note]
                        let l = cur_label(plan);
                        let end = Label(l.0 + 2 + tc_allocations(elty));
                        jump_if0(size, end, code, plan); // :60 (forward end)
                        let loop_real = mirror_label(code, plan); // :61
                        tc(syscalls, input, inputend, out, elty, code, plan); // :62 —
                        // the source's `tc elty`; `typecheck ... elty`
                        // (:28-35/:97) IS this recursion.
                        decr_and_jump_if_not0(size, loop_real, code, plan); // :63 (REAL loop)
                        let end_real = mirror_label(code, plan); // :64
                        assert_eq!(
                            end_real, end,
                            "typecheck: LabelPlan prediction diverged from the real end bind (A2)",
                        );
                    }
                }
            });
        }
        // tc (RecordTy fields) = mapM_ (tc . fieldDefType) fields (:66)
        // — the `FieldDef _ ty` positional pattern (:73 uses the same
        // shape) maps to the record's type field (rts/types.rs precedent).
        Type_::RecordTy(fields) => {
            for f in fields {
                tc(syscalls, input, inputend, out, &f.field_def_type, code, plan);
            }
        }
        // tc (SumTy fields) = mdo ... (:67-79) — MDO SITE 2 of 7.
        Type_::SumTy(fields) => {
            // ----- MDO SITE 2 of 7 — SumTy (Typecheck.hs:67-79;
            // inventory.tsv row 81 site :67) -----
            //
            // OLD (the knot, :68-78):
            //   local $ \sel -> do
            //     inputNat input inputend sel
            //     outputNat sel out
            //     select sel alts                    -- FORWARD refs
            //   raise "selector out of range"        -- EndBlock
            //   alts <- forM fields $ \(FieldDef _ ty) -> do
            //     alt <- label ; tc ty ; jump end ; return alt
            //   end <- label
            // DELTA kept verbatim (row evidence + header): `select sel
            // alts` has NO fallback target — Traverse.hs's Sum appends
            // `unknownAlt = [end]`, Typecheck's does NOT (the Select's
            // out-of-range no-op, def/Instruction.hs:381-383, falls
            // through to the raise :72).
            // With csLabel = L at the select, A_i = tc_allocations(ty_i),
            // n = length fields:
            //
            // OLD disassembly (label ids in BIND order — A1):
            //   block L:        InputNat input inputend sel
            //                   OutputNat sel out
            //                   Select sel [alt_0..alt_{n-1}]
            //                   Raise lit          [raise close: allocates L+1]
            //   block alt_i:    <tc ty_i> ; Jump end
            //                                    [jump close: allocates
            //                                     alt_i + A_i + 1]
            //   ...
            //   block end:      (the block the LAST jump's close allocated
            //                    — empty at the end bind)
            // with
            //   alt_i = L+1 + Σ_{j<i} (A_j + 1)
            //   end   = L+1 + n + Σ_j A_j
            // and every allocation made by the raise (:72) and the jumps
            // (:76): the alt binds (:74) and the end bind (:78) each
            // follow an EndBlock close (fact 1), see an EMPTY current
            // block, and therefore ALIAS — they allocate NOTHING
            // (Code.hs:483-488's null-insns branch). Total allocations:
            // 1 + n + Σ A_i (= tc_allocations' SumTy arm). For n = 0 the
            // end bind follows the raise alone: end = L+1 — the formula
            // covers it (Σ over an empty range).
            //
            // NEW: identical instruction stream, identical ids. The
            // Select's targets are the pass-1 predictions, computed
            // BEFORE the select is emitted; every `jump end` is a forward
            // ref (the end bind has not run); each bind's
            // prediction-vs-real check is a RELEASE-ACTIVE assert_eq!.
            // [A1 disassemble-identity note]
            let l = cur_label(plan);
            let mut alts: Vec<Label> = Vec::with_capacity(fields.len());
            let mut pred = l.0 + 1; // alt_0: after the raise's close
            for f in fields {
                alts.push(Label(pred));
                // the alt body's allocations + the jump's close
                pred += tc_allocations(&f.field_def_type) + 1;
            }
            let end = Label(pred);
            // local $ \sel -> do ... (:68-71). NO move (the sibling's B1
            // review fix): the body is an immediately-consumed FnOnce
            // (code.rs:332-336), so reborrows suffice and `plan`/`alts`
            // survive for the raise and the alt loop after the local.
            code.local(pull1, |sel: Register<Word>, code| {
                input_nat(input, inputend, sel, code, plan); // :69
                output_nat(sel, out, code, plan); // :70
                select(sel, &alts, code, plan); // :71 (forward alts — NO fallback)
            });
            raise(b"selector out of range", code, plan); // :72
            // alts <- forM fields $ \(FieldDef _ ty) -> do ... (:73-77)
            for (f, alt) in fields.iter().zip(alts.iter()) {
                let alt_real = mirror_label(code, plan); // :74 (aliases)
                assert_eq!(
                    alt_real, *alt,
                    "typecheck: LabelPlan prediction diverged from a real alt bind (A2)",
                );
                tc(syscalls, input, inputend, out, &f.field_def_type, code, plan); // :75
                jump(end, code, plan); // :76 (closes; forward end)
            }
            let end_real = mirror_label(code, plan); // :78 (aliases)
            assert_eq!(
                end_real, end,
                "typecheck: LabelPlan prediction diverged from the real end bind (A2)",
            );
        }
        // tc (SetTy ByteTy) = local $ \size -> do ... (:80-89) — the arm
        // ORDER is load-bearing: this equation precedes the general
        // SetTy arm (:90). RULE §2 (A8): the match is SYNTACTIC on the
        // raw element — NO derefType here (the stress-validated asymmetry
        // vs ArrayTy's :54 dispatch; a NamedTy-wrapped byte still takes
        // the NON-byte arm, whose loop body then routes through tc's
        // NamedTy equation).
        Type_::SetTy(t) if matches!(**t, Type_::ByteTy) => {
            code.local(pull1, |size: Register<Word>, code| {
                input_nat(input, inputend, size, code, plan); // :81
                code.local(pull2, |(set, ptr): (Register<Word>, Register<DataPtr>), code| {
                    (syscalls.new_word_set)(set, code, plan); // :83
                    move_(input, ptr, code, plan); // :84
                    input_bytes(input, inputend, size, code, plan); // :85
                    (syscalls.insert_bytes_word_set)(set, ptr, input, code, plan); // :86
                    (syscalls.byte_set_to_byte_array)(set, out, code, plan); // :87
                    (syscalls.free_word_set)(set, code, plan); // :88
                });
                // return () (:89)
            });
        }
        // tc (SetTy elty) = local $ \size -> do inputNat ...;
        //   local $ \set -> mdo ... (:90-103) — MDO SITE 3 of 7.
        Type_::SetTy(t) => {
            code.local(pull1, move |size: Register<Word>, code| {
                input_nat(input, inputend, size, code, plan); // :91
                code.local(pull1, move |set: Register<Word>, code| {
                    // ----- MDO SITE 3 of 7 — SetTy (Typecheck.hs:92-102;
                    // inventory.tsv row 81 site :92) -----
                    //
                    // OLD (the knot, :93-100): the whole mdo is the inner
                    // local body:
                    //   newSet set ; jumpIf0 size end ; loop <- label ;
                    //   output $ \tempOut -> do
                    //     typecheck syscalls input inputend tempOut elty
                    //     insertOutputSet set tempOut
                    //     decrAndJumpIfNot0 size loop
                    //   end <- label ; setToArray set out ; freeSet set
                    // DELTAS kept verbatim (header): `newSet` is emitted
                    // BEFORE `jumpIf0` (Traverse.hs's SetRep has no
                    // newSet); the decr sits INSIDE the output callback
                    // (Traverse's is at the mdo top level). With
                    // csLabel = L at the newSet and A = tc_allocations(t)
                    // (the element body's allocations):
                    //
                    // OLD disassembly (label ids in BIND order — A1):
                    //   block L:       NewSet set
                    //                  JumpIf0 size (L+2+A)
                    //                  Jump (L+1)      [loop bind close]
                    //   block L+1:     ResetOutput tempOut
                    //                  <tc t — the element body>
                    //                  InsertOutputSet set tempOut
                    //                  DecrAndJumpIfNot0 size (L+1)
                    //                  Jump (L+2+A)    [end bind close]
                    //   block L+2+A:   SetToArray set out
                    //                  FreeSet set
                    //                  (continues after the set)
                    // loop = L+1 (CLOSES: newSet+jumpIf0 precede it, fact
                    // 1); end = L+2+A (CLOSES: the decr precedes it —
                    // insertOutputSet+decr are emitted unconditionally
                    // after the body INSIDE the callback, :98-99, so the
                    // choice is structural regardless of the body).
                    //
                    // NEW: identical instruction stream, identical ids;
                    // same A2 emission discipline as SITE 1 (the decr
                    // takes the REAL loop operand; the JumpIf0's forward
                    // `end` is the pass-1 prediction, release-checked at
                    // the end bind). `output`'s internal ResetOutput
                    // (Code.hs:185-188) is mirrored plan-first below.
                    // [A1 disassemble-identity note]
                    (syscalls.new_set)(set, code, plan); // :93 (BEFORE jumpIf0)
                    let l = cur_label(plan);
                    let end = Label(l.0 + 2 + tc_allocations(t));
                    jump_if0(size, end, code, plan); // :94 (forward end)
                    let loop_real = mirror_label(code, plan); // :95
                    // output $ \tempOut -> do ... (:96-99) — mirror the
                    // ResetOutput `output` will issue (Code.hs:185-188)
                    // BEFORE the call: plan first, then Code (the sibling
                    // rule verbatim; the emission is code.rs's own).
                    plan.issue();
                    code.output(move |temp_out: Register<BinaryOutputPtr>, code| {
                        // typecheck syscalls input inputend tempOut elty
                        // (:97) — the source's full-arity typecheck
                        // spelling; `typecheck syscalls input inputend
                        // out = tc` (:35) makes it exactly the tc
                        // recursion.
                        tc(syscalls, input, inputend, temp_out, t, code, plan); // :97
                        (syscalls.insert_output_set)(set, temp_out, code, plan); // :98
                        decr_and_jump_if_not0(size, loop_real, code, plan); // :99 (REAL loop, INSIDE the callback)
                    });
                    let end_real = mirror_label(code, plan); // :100
                    assert_eq!(
                        end_real, end,
                        "typecheck: LabelPlan prediction diverged from the real end bind (A2)",
                    );
                    (syscalls.set_to_array)(set, out, code, plan); // :101
                    (syscalls.free_set)(set, code, plan); // :102
                });
                // return () (:103)
            });
        }
        // tc (PredicateTy _ (PidRef (Pid pid) _)) = local $ \ide -> do
        //   ... (:104-108). The pattern SHAPE is preserved syntactically —
        //   the PidRef/Pid constructors are matched, not projected (RULE §2
        //   A8; traverse.rs's Rep::PredicateRep(Pid(pid)) precedent).
        Type_::PredicateTy(_, PidRef { pid: Pid(pid), .. }) => {
            code.local(pull1, move |ide: Register<Word>, code| {
                // t <- constant (fromIntegral pid) (:105): Pid wraps Int64
                // (rts/types.rs), fromIntegral to Word64 wraps — RULE §2
                // (numeric row): `as u64` is the same wrap. constant() is
                // label-neutral (Code.hs:148-160), no plan advance.
                let t = code.constant(*pid as u64);
                input_nat(input, inputend, ide, code, plan); // :106
                (syscalls.rename)(ide, t, ide, code, plan); // :107 —
                // `rename ide t ide` (:107): the SAME register is source
                // operand 1 and result slot (CallFun_2_1's Regs
                // [Word,Word,Word]); kept verbatim.
                output_nat(ide, out, code, plan); // :108
            });
        }
        // tc (NamedTy _ (ExpandedType _ ty)) = tc ty (:109)
        Type_::NamedTy(_, expanded) => {
            tc(syscalls, input, inputend, out, &expanded.expanded_type_type, code, plan);
        }
        // tc (MaybeTy ty) = mdo ... (:110-119) — MDO SITE 4 of 7.
        Type_::MaybeTy(t) => {
            // ----- MDO SITE 4 of 7 — MaybeTy (Typecheck.hs:110-119;
            // inventory.tsv row 81 site :110) -----
            //
            // OLD (the knot, :111-118):
            //   local $ \sel -> do
            //     inputNat input inputend sel
            //     outputNat sel out
            //     select sel [end,just]              -- FORWARD refs
            //   raise "maybe selector out of range"  -- EndBlock
            //   just <- label
            //   tc ty
            //   end <- label
            // The select list is `[end,just]` — end FIRST (row evidence:
            // "select [end,just] before just<-label/end<-label"; selector
            // 0 = Nothing → end, selector 1 = Just → just).
            // With csLabel = L at the select, A = tc_allocations(t):
            //
            // OLD disassembly (label ids in BIND order — A1):
            //   block L:       InputNat input inputend sel
            //                  OutputNat sel out
            //                  Select sel [end, just]
            //                  Raise lit          [raise close: allocates L+1]
            //   block L+1:     <tc ty>            (just = L+1, ALIAS after
            //                                     the raise's close, fact 1)
            //                  Jump (end)         [end bind close — iff tc
            //                                      ty left the block
            //                                      non-empty]
            //   block end:     (continues)
            // end = L+1 + A + [tc ty leaves the OPEN BLOCK NON-empty] —
            // THE ONE DATA-DEPENDENT BIND in this file (fact 3): the end
            // bind (:118) is preceded by `tc ty` (:117) and NOTHING else,
            // so per the TRUE Code.hs:484-489 rule it ALIASES (= L+1, the
            // raise's block) iff the open block is EMPTY after the body's
            // emissions, and CLOSES iff the body left instructions in it.
            // The predicate is NOT "the body emitted anything": every
            // issueEndBlock close (newBlock allocates UNCONDITIONALLY,
            // even on an empty block, Code.hs:459-468) and every closing
            // end bind starts a fresh EMPTY block, so a body that emits
            // plenty can still alias. Empty-leaving bodies: tcEnum
            // (BooleanTy/EnumeratedTy — its own end bind :131 aliases the
            // raise's fresh block), SumTy (:78), a nested MaybeTy,
            // ArrayTy with a non-byte element (its own end-close :64,
            // nothing after), and the instruction-less no-op chains
            // (`RecordTy []` :66 and NamedTy over one :109 — the block
            // stands untouched, and it is EMPTY here: the raise above
            // just opened it; then the Select's end target IS the
            // raise's block: the source's own shape, kept bug-for-bug).
            // Non-empty-leaving bodies: Byte/Nat/String/Predicate,
            // ArrayTy with a byte-deref element (:56-58), SetTy BOTH arms
            // (byte :80-89 issue-only; non-byte emits setToArray+
            // freeSet AFTER its end-close, :101-102), and records ending
            // in a state-forcing field of those (the twin's RecordTy
            // fold). Every other bind in this file is structural (facts
            // 1-2). Total allocations: 1 (raise) + A + [non-empty]
            // (= tc_allocations' MaybeTy arm).
            //
            // HAND-TRACED ANCHORS for this prediction (the A1 fix's
            // verification; each traced against code.rs's new_block/
            // label semantics — new_block allocates succ csLabel
            // UNCONDITIONALLY even on an empty block, label() aliases
            // iff csInsns is empty; the twin table was additionally
            // swept against a 46-shape replica of these semantics):
            //   tc (MaybeTy BooleanTy), entry csLabel = L: raise
            //     allocates L+1 (fresh EMPTY block); just <- label
            //     ALIASES L+1; tcEnum 2 issues inputNat+outputNat+
            //     jumpIfLt INTO block L+1, its raise allocates L+2
            //     (fresh EMPTY block L+2), its end bind (:131) ALIASES
            //     L+2 — tcEnum ends on an EMPTY open block; the end
            //     bind (:118) ALIASES → end = L+2; Select [L+2, L+1] —
            //     matching the old binary. Twin total: 1 + A(=1) + 0
            //     = 2.
            //   tc (MaybeTy NatTy), entry L: raise allocates L+1; just
            //     aliases L+1; tc NatTy issues inputNat+outputNat INTO
            //     block L+1 (never closes it); the end bind (:118) sees
            //     a NON-empty block → CLOSES → allocates L+2 → end =
            //     L+2; Select [L+2, L+1]. Twin total: 1 + A(=0) + 1 = 2.
            //   tc (MaybeTy (MaybeTy NatTy)), entry L: raise allocates
            //     L+1; just aliases L+1; the inner Maybe allocates 2
            //     (its raise → L+2; its Nat body leaves block L+2
            //     non-empty, so its end bind closes → L+3) and leaves
            //     the open block (L+3) EMPTY; the outer end bind (:118)
            //     ALIASES → end = L+3. Twin total: 1 + A(=2) + 0 = 3.
            //
            // NEW: identical instruction stream, identical ids; the
            // prediction below is the conditional one; just's and end's
            // prediction-vs-real checks are RELEASE-ACTIVE assert_eq!s.
            // [A1 disassemble-identity note]
            let l = cur_label(plan);
            let a = tc_allocations(t);
            let just = Label(l.0 + 1);
            let end = Label(l.0 + 1 + a + i64::from(!tc_leaves_block_empty(t)));
            code.local(pull1, |sel: Register<Word>, code| {
                input_nat(input, inputend, sel, code, plan); // :112
                output_nat(sel, out, code, plan); // :113
                select(sel, &[end, just], code, plan); // :114 — [end,just] ORDER verbatim
            });
            raise(b"maybe selector out of range", code, plan); // :115
            let just_real = mirror_label(code, plan); // :116 (aliases)
            assert_eq!(
                just_real, just,
                "typecheck: LabelPlan prediction diverged from the real just bind (A2)",
            );
            tc(syscalls, input, inputend, out, t, code, plan); // :117
            let end_real = mirror_label(code, plan); // :118 (ALIASES iff `tc ty` left the open block empty — tc_leaves_block_empty; e.g. a tcEnum body aliases after its own raise)
            assert_eq!(
                end_real, end,
                "typecheck: LabelPlan prediction diverged from the real end bind (A2)",
            );
        }
        // tc (EnumeratedTy names) = tcEnum $ fromIntegral $ length names
        // (:120) — fromIntegral :: Int -> Word64 wraps; `as u64` is the
        // same wrap (RULE §2 numeric row).
        Type_::EnumeratedTy(names) => {
            tc_enum(names.len() as u64, syscalls, input, inputend, out, code, plan);
        }
        // tc BooleanTy = tcEnum 2 (:121)
        Type_::BooleanTy => {
            tc_enum(2, syscalls, input, inputend, out, code, plan);
        }
        // tc _ = error "typecheck" (:122) — RULE §2 (TIMING row, A6): the
        // error is per-ACCESS — it fires when this arm is dispatched
        // during the generation walk (the Code action is forced at
        // sequence time), which is exactly where this panic stands. No
        // eager up-front validation exists anywhere in this file.
        Type_::TyVar(_) | Type_::HasTy(..) | Type_::HasKey(..) | Type_::ElementsOf(..) => {
            panic!("typecheck")
        }
    }
}

/// `tcEnum arity` (Typecheck.hs:124-132) — MDO SITE 5 of 7. Private in
/// the source (a where-binding of tc), private here; it closes over
/// syscalls/input/inputend/out, which become explicit parameters.
///
/// ----- MDO SITE 5 of 7 — tcEnum (Typecheck.hs:124-132; inventory.tsv
/// row 81 site :124) -----
///
/// OLD (the knot, :125-131):
///   k <- constant arity
///   local $ \sel -> do
///     inputNat input inputend sel
///     outputNat sel out
///     jumpIfLt sel k end        -- FORWARD ref
///     raise "selector out of range"
///   end <- label
/// DELTA kept verbatim (header): the raise is INSIDE the local body
/// (SumTy's raise is OUTSIDE its local — :72 after :68-71). With
/// csLabel = L at the constant (constant() is label-neutral,
/// Code.hs:148-160):
///
/// OLD disassembly (label ids in BIND order — A1):
///   block L:       InputNat input inputend sel
///                  OutputNat sel out
///                  JumpIfLt sel k (L+1)
///                  Raise lit          [raise close: allocates L+1]
///   block L+1:     (continues)        [end = L+1, ALIAS after the
///                                      raise's close (fact 1)]
/// Total allocations: 1 (= tc_allocations' Enumerated/Boolean arm).
///
/// NEW: identical instruction stream, identical ids; the forward `end`
/// is the pass-1 prediction, release-checked at the bind. [A1
/// disassemble-identity note]
fn tc_enum(
    arity: u64,
    _syscalls: &SysCalls,
    input: Register<DataPtr>,
    inputend: Register<DataPtr>,
    out: Register<BinaryOutputPtr>,
    code: &mut Code,
    plan: &mut LabelPlan,
) {
    // k <- constant arity (:125) — label-neutral, no plan advance.
    let k = code.constant(arity);
    let l = cur_label(plan);
    let end = Label(l.0 + 1);
    code.local(pull1, |sel: Register<Word>, code| {
        input_nat(input, inputend, sel, code, plan); // :127
        output_nat(sel, out, code, plan); // :128
        jump_if_lt(sel, k, end, code, plan); // :129 (forward end)
        raise(b"selector out of range", code, plan); // :130 — INSIDE the local
    });
    // (the source's tcEnum closes over the whole tc environment — syscalls
    // included — though its body uses none of the record: the `_` param is
    // that capture shape, kept rather than dropped.)
    let end_real = mirror_label(code, plan); // :131 (aliases)
    assert_eq!(
        end_real, end,
        "typecheck: LabelPlan prediction diverged from the real end bind (A2)",
    );
}

// -----------------------------------------------------------------------------
// checkSignature / check / checkType (Typecheck.hs:184-251)
// -----------------------------------------------------------------------------

/// Generate a subroutine which typechecks and substitutes a clause. It has
/// the following arguments (source doc, Typecheck.hs:194-205, verbatim in
/// substance):
///
/// * std::function<Id(Id id, Id type)> - fact substitution
/// * set syscalls.
/// * const void * - begin of clause/key
/// * const void * - end of key/begin of value
/// * const void * - end of clause/value
///
/// It returns the substituted clause in the first output and size of the
/// substituted key in the first local register.
///
/// `checkSignature :: Type -> Type -> IO (Subroutine CompiledTypecheck)`
/// (Typecheck.hs:206-240) — `fmap snd $ generate Optimised $ \rename_ …
/// clause_end -> output $ \out -> do ...`. The (pull, body) form of the
/// CodeGen walk via the `generate!` macro (code.rs DESIGN (2)); the
/// thirteen registers' tags are inferred from the body's uses (the ten
/// syscall handles at the callFun fun-argument shapes pinned in the
/// ASSUMED-IMPORT block, the three pointers at Register 'DataPtr), exactly
/// as the source's lambda parameters are.
///
/// The infallible shape is the SHIPPED consumer's pinned contract (header:
/// DOWNSTREAM CONTRACT; schema.rs:1913 consumes it bare). The one failure
/// source of `generate` — the `fail "unterminated basic block"` check
/// (Code.hs:279-280, ported at code.rs:725-729) — is STRUCTURALLY
/// unreachable here: the body's last emission is `ret` (:240), an
/// EndBlock instruction (issueEndBlock → newBlock empties csInsns,
/// Code.hs:494-496/:459-468), so the final csInsns is empty by
/// construction. The impossible Err is converted to a panic with the
/// source's own message (RULE §2 partial-functions row: where the source
/// would raise, panic with the same text — the Haskell `fail` propagates
/// as an IO exception through `fmap snd $ generate ...`).
pub fn check_signature(key_ty: &Type, val_ty: &Type) -> Subroutine<CompiledTypecheck> {
    let result = crate::generate!(code, Optimised::Optimised,
        |rename_,
         new_set_,
         insert_output_set_,
         set_to_array_,
         free_set_,
         new_word_set_,
         insert_bytes_word_set_,
         word_set_to_array_,
         byte_set_to_byte_array_,
         free_word_set_,
         clause_begin, key_end, clause_end| {
            // output $ \out -> do ... (:213) — outputUninitialized $ \out
            // -> do issue $ ResetOutput out; f out (Code.hs:185-188). The
            // plan's scope is the callback body, the whole generate()
            // scope in the source's mdo shape; its INITIAL STATE mirrors
            // Code's state AT THE CALLBACK ENTRY: generate starts at
            // (Label(0), empty block) and `output` has ALREADY issued
            // ResetOutput into the still-open block 0 (Code.hs:185-188) —
            // cs_label is still Label(0), the block is open. (The one
            // code.rs-internal emission is accounted here explicitly
            // rather than through a wrapper: plan first, then Code.)
            let mut plan = LabelPlan::new();
            plan.issue(); // the ResetOutput output() issued before this callback runs
            code.output(move |out: Register<BinaryOutputPtr>, code| {
                // let syscalls = SysCalls {...} (:214-232) — the record is
                // built INSIDE the generate body, capturing the ten
                // syscall handles, exactly as the source's let does.
                let syscalls = SysCalls {
                    // rename = \id pid reg -> callFun_2_1 rename_ id pid reg (:215-216)
                    rename: Box::new(move |id, pid, reg, code, plan| {
                        call_fun_2_1(rename_, id, pid, reg, code, plan)
                    }),
                    // newSet = callFun_0_1 newSet_ (:217)
                    new_set: Box::new(move |set, code, plan| {
                        call_fun_0_1(new_set_, set, code, plan)
                    }),
                    // insertOutputSet = \set out -> callFun_2_0
                    //   insertOutputSet_ set (castRegister out) (:218-219)
                    insert_output_set: Box::new(move |set, out, code, plan| {
                        call_fun_2_0(insert_output_set_, set, cast_register(out), code, plan)
                    }),
                    // setToArray = \set arr -> callFun_1_1 setToArray_ set
                    //   (castRegister arr) (:220-221)
                    set_to_array: Box::new(move |set, arr, code, plan| {
                        call_fun_1_1(set_to_array_, set, cast_register(arr), code, plan)
                    }),
                    // freeSet = callFun_1_0 freeSet_ (:222)
                    free_set: Box::new(move |set, code, plan| {
                        call_fun_1_0(free_set_, set, code, plan)
                    }),
                    // newWordSet = callFun_0_1 newWordSet_ (:223)
                    new_word_set: Box::new(move |set, code, plan| {
                        call_fun_0_1(new_word_set_, set, code, plan)
                    }),
                    // insertBytesWordSet = \set start end -> callFun_3_0
                    //   insertBytesWordSet_ set (castRegister start)
                    //   (castRegister end) (:224-226)
                    insert_bytes_word_set: Box::new(move |set, start, end, code, plan| {
                        call_fun_3_0(
                            insert_bytes_word_set_,
                            set,
                            cast_register(start),
                            cast_register(end),
                            code,
                            plan,
                        )
                    }),
                    // wordSetToArray = \set arr -> callFun_1_1
                    //   wordSetToArray_ set (castRegister arr) (:227-228) —
                    // wired but never consumed by tc (see the field doc).
                    word_set_to_array: Box::new(move |set, arr, code, plan| {
                        call_fun_1_1(word_set_to_array_, set, cast_register(arr), code, plan)
                    }),
                    // byteSetToByteArray = \set arr -> callFun_1_1
                    //   byteSetToByteArray_ set (castRegister arr) (:229-230)
                    byte_set_to_byte_array: Box::new(move |set, arr, code, plan| {
                        call_fun_1_1(byte_set_to_byte_array_, set, cast_register(arr), code, plan)
                    }),
                    // freeWordSet = callFun_1_0 freeWordSet_ (:231)
                    free_word_set: Box::new(move |set, code, plan| {
                        call_fun_1_0(free_word_set_, set, code, plan)
                    }),
                };
                // We return the key size in the first local register
                // (source comment, :233).
                //
                // ----- MDO SITE 6 of 7 — the :234 knot (Typecheck.hs:
                // :234-240; inventory.tsv row 81 site :234) -----
                //
                // `local $ \key_size -> mdo` (:234): the mdo binds NO
                // labels — VACUOUS (the stress round-2 trap list). Its six
                // statements reference `check` from the where clause
                // (:242), whose own mdo (site 7) carries the knot work;
                // nothing here binds or forward-references a label, so the
                // mdo degenerates to plain sequencing — the port writes
                // the local body as an ordinary block. The bind-point rule
                // (RULE §0 A1) is untouched: every `x <- label` inside the
                // two `check` calls still allocates at its bind through
                // mirror_label.
                //
                // OLD/NEW disassembly: no label of this site's own; the
                // two check sites' tables below apply verbatim.
                code.local(pull1, move |key_size: Register<Word>, code| {
                    // typecheck syscalls clause_begin key_end out key_ty
                    // (:235)
                    tc(&syscalls, clause_begin, key_end, out, key_ty, code, plan);
                    // check "key" clause_begin key_end (:236)
                    check(b"key", clause_begin, key_end, code, plan);
                    // getOutputSize out key_size (:237)
                    get_output_size(out, key_size, code, plan);
                    // typecheck syscalls clause_begin clause_end out
                    //   val_ty (:238)
                    tc(&syscalls, clause_begin, clause_end, out, val_ty, code, plan);
                    // check "value" clause_begin clause_end (:239)
                    check(b"value", clause_begin, clause_end, code, plan);
                    // ret (:240)
                    ret(code, plan);
                });
            });
        });
    // fmap snd $ generate ... (:207-208) — Meta is discarded, as in the
    // source.
    match result {
        Ok((_, sub)) => sub,
        // Structurally unreachable (header proof); the source's `fail`
        // would raise the message as an IO exception — RULE §2
        // (partial-functions row): panic with the same text.
        Err(e) => panic!("{}", e.message),
    }
}

/// `check str reg1 reg2` (Typecheck.hs:242-250) — MDO SITE 7 of 7; a
/// where-binding of checkSignature, private in the source, private here.
/// `str` is "key"/"value" as BYTES (the generated raise takes a
/// ByteString; RULE §1 strings row: Vec<u8>/&[u8]).
///
/// ----- MDO SITE 7 of 7 — check (Typecheck.hs:242-250; inventory.tsv
/// row 81 site :242) -----
///
/// OLD (the knot, :243-249):
///   jumpIfNe (castRegister reg1) (castRegister reg2) bad
///   -- The basic block layout algorithm will make the happy path
///   -- sequential and delete this jump.          (source comment, :244-245)
///   jump ok
///   bad <- label
///   raise $ "extra bytes at end of " <> str
///   ok <- label
/// With csLabel = L at the jumpIfNe:
///
/// OLD disassembly (label ids in BIND order — A1):
///   block L:       JumpIfNe reg1 reg2 (L+1)
///                  Jump (L+2)         [jump ok close: allocates L+1 — the
///                                      source's own comment above]
///   block L+1:     Raise lit          [raise close: allocates L+2]
///   block L+2:     (continues)
/// bad = L+1 (ALIAS after the jump's close, fact 1); ok = L+2 (ALIAS
/// after the raise's close). Total allocations: 2.
///
/// NEW: identical instruction stream, identical ids; both `bad` and `ok`
/// operands are forward predictions, release-checked at their binds.
/// [A1 disassemble-identity note]
fn check(
    str_: &[u8],
    reg1: Register<DataPtr>,
    reg2: Register<DataPtr>,
    code: &mut Code,
    plan: &mut LabelPlan,
) {
    let l = cur_label(plan);
    let bad = Label(l.0 + 1);
    let ok = Label(l.0 + 2);
    jump_if_ne(reg1, reg2, bad, code, plan); // :243 (forward bad; the castRegister coercion rides the poly tag)
    // The basic block layout algorithm will make the happy path
    // sequential and delete this jump. (source comment, :244-245)
    jump(ok, code, plan); // :246 (closes; forward ok)
    let bad_real = mirror_label(code, plan); // :247 (aliases)
    assert_eq!(
        bad_real, bad,
        "typecheck: LabelPlan prediction diverged from the real bad bind (A2)",
    );
    // raise $ "extra bytes at end of " <> str (:248) — the ByteString
    // concatenation becomes the two byte-slices joined (RULE §1 strings
    // row).
    let msg = [b"extra bytes at end of ".as_slice(), str_].concat();
    raise(&msg, code, plan); // :248 (closes)
    let ok_real = mirror_label(code, plan); // :249 (aliases)
    assert_eq!(
        ok_real, ok,
        "typecheck: LabelPlan prediction diverged from the real ok bind (A2)",
    );
    // return () (:250)
}

/// Generate a subroutine which typechecks and substitutes a value. It has
/// the following arguments (source doc, Typecheck.hs:184-190, verbatim in
/// substance):
///
/// * std::function<Id(Id id, Id type)> - fact substitution
/// * binary::Input * - value
/// * binary::Output * - substituted value
///
/// `checkType :: Type -> IO (Subroutine CompiledTypecheck)`
/// (Typecheck.hs:191-192) — `checkType ty = checkSignature ty $ RecordTy
/// []`; the empty record literal is `Type_::RecordTy(vec![])` (the RTS
/// Type instantiation, rts/types.rs:250).
pub fn check_type(key_ty: &Type) -> Subroutine<CompiledTypecheck> {
    check_signature(key_ty, &Type_::RecordTy(Vec::new()))
}

// -----------------------------------------------------------------------------
// Reconciliation blocks (header companions; no markers owed here)
// -----------------------------------------------------------------------------

// KIND-TAG RECONCILIATION: `data CompiledTypecheck` (Typecheck.hs:26) is
// owned HERE (the source module exports it). The shipped
// rts/foreign/inventory.rs currently carries a stand-in
// `pub struct CompiledTypecheck;` at inventory.rs:258 whose TODO (:667)
// says "move to rts/typecheck.rs (unit 20) and rts/traverse.rs (unit 19)
// when those units ship" — that unit's one-line re-export swap (`pub use
// crate::rts::typecheck::CompiledTypecheck;` for this half) is ITS
// fix-pass obligation, recorded here as a cross-reference (the same
// ruling traverse.rs's KIND-TAG RECONCILIATION block records for its
// CompiledTraversal half). Until the swap lands, the two paths are the
// live compile-error seam HANDOFF already classified for rows 23/24.
//
// MODULE-WIRING: lib.rs / rts.rs / rts/bytecode.rs module scaffolding is
// deliberately NOT a manifest unit; owned by the Step-4 survey build (do
// not create ad-hoc here — the supply.rs/code.rs/traverse.rs/types.rs
// note). The module path `glean_core::rts::typecheck` is already pinned
// by the shipped consumer (schema.rs:583 `use glean_core::rts::
// typecheck::check_signature;`).
//
// Marker inventory (reviewer counts the trailer against this): ZERO
// TODO(port) — the one port-device question this file faced (how the mdo
// forward refs resolve without laziness) is CLOSED by the derived
// two-pass discipline (header: THE MDO TRANSLATION; the re-hosted
// pattern's shape delta — explicit &mut threading instead of a
// thread-local, and the MaybeTy data-dependent bind — is a recorded
// design fact, not an open question), and there are no other rule gaps.
// ZERO BUG(port) — no source defect is reproduced (the MaybeTy
// instruction-less-body Select targeting the raise's own block is the
// source's plain semantics, not a crash to flag; the wordSetToArray
// wired-but-unused field is the source's own shape). ZERO PERF(port) —
// the twin walkers are O(type size), the alts Vec is n Labels, the
// check-message concat is two tiny slices, all linear in what the source
// allocates. ZERO unsafe blocks (RULE §3 vacuously satisfied — pure
// codegen; the one FFI seam, subroutine construction, crosses via the
// SHIPPED crate::rts::foreign::bytecode inside code.rs's generate). ZERO
// INVENTORY-BUG-CANDIDATE (row 81 is followed field-for-field; see the
// header INVENTORY block). The #[allow(dead_code)] on
// SysCalls::word_set_to_array is not a marker — it is the honest
// rendering of the source's wired-but-unused record field (RULE §0),
// documented at the field.
//
// The A1 promotion gate (RULE §0): the old-vs-new `disassemble` identity
// check runs on this file's outputs before promotion (inventory row 81:
// "typecheck bytecode is on-disk-independent but still verified
// old-vs-new") — the seven per-site identity tables above are the claims
// it verifies (11 static label bind points, ids in bind order; every `x
// <- label` site emits its block split through the REAL Code::label).
// Recorded in the header, not as a marker (the supply.rs/code.rs/
// traverse.rs precedent).
//
// Coverage: every export of the Haskell module has a Rust counterpart —
// CompiledTypecheck [pub struct, the kind tag; inventory.rs:258's
// stand-in (TODO :667) defers to this definition], checkType [check_type
// — the shipped consumer's pinned infallible shape], checkSignature
// [check_signature — generate! macro over the 13-register pull, the
// SysCalls record built inside the body exactly as the source's let].
// Private surface ported too: SysCalls(..) [all ten fields, order and
// register roles verbatim, incl. the wired-but-unused wordSetToArray],
// typecheck [fn tc — the curry collapsed to explicit parameters, arm
// order preserved], tcEnum [fn tc_enum], check [fn check, with the
// source's :244-245 comment verbatim], the 7 mdo sites [per-site
// identity tables + mirror_label binds]. Port devices (no silent gaps):
// LabelPlan + mirror helpers + cur_label + pull1/pull2 [the pass-1
// mirror, the row-42/80-validated pattern re-hosted from the shipped
// sibling — privacy, not crate direction, is why it is re-hosted],
// tc_allocations + tc_leaves_block_empty (plus the entry-parameterized
// fold engine behind it) [the pass-1 twins, incl. the NEW fact-3
// data-dependent MaybeTy bind]. Dropped Haskell artifacts (no
// silent gaps): RecursiveDo [the A1 two-pass discipline], the `mdo` at
// :234 [vacuous — plain sequencing, site block records it], `hiding
// (Type)` on the Angle import [the Type_ import image, header
// dependency-truth note], Control.Monad's mapM_/forM [for loops,
// mechanical], the lazy knot thunks [pass-1 predictions + the A2
// lockstep checks: release-active prediction asserts at the mdo binds,
// the debug plan-vs-Code canary in mirror_label], the curried
// `typecheck` signature [explicit parameters, code.rs DESIGN (1)],
// HasCallStack [none in the source module].
//
// Trailer confidence (B3): medium, not high — the A1 fix (the
// tc_leaves_block_empty twin: the TRUE Code.hs:484-489 alias-iff-open-
// block-empty rule, replacing the wrong "emitted anything" predicate)
// is verified by HAND-TRACE against code.rs's new_block/label semantics
// (the three anchors recorded at MDO SITE 4: maybe bool → 2, maybe Nat
// → 2, maybe (maybe Nat) → 3, plus the maybe-bool Select [L+2, L+1],
// and a 46-shape sweep of the twin table against a semantics replica),
// but the final proof remains the judge's old-vs-new disassemble
// identity gate (RULE §0 A1), which has not run on this file's outputs
// yet.
//
// PORT STATUS: confidence=medium todos=0
