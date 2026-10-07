// Ported from: glean/angle/Glean/Display.hs
// Manifest unit: rust/glean-angle/src/display.rs (crate glean-angle) —
// migration/manifest.tsv row 3. RULE §4 (naming): one source file → one
// target file; module path from the manifest is authoritative.
//
// The Haskell source is 65 lines defining a parameterised pretty-printing
// class. RULE §0: the old code is the spec — everything in the
// "Glean.Display proper" section below maps 1:1 to the source; the "Doc
// stand-in" section exists only because the source's one external import
// (`Compat.Prettyprinter`) has no owner in the port (see the gap banner).

// SIBLING-UNIT CONTRACT (RULE §4 B4, mirroring hash.rs's contract block):
// this module is the DISPLAY FOUNDATION for every unit whose Haskell had
// `instance Display` or called displayDefault/displayVerbose. Verified
// consumers (grep `instance Display` / `Glean.Display`):
//   - glean/angle/Glean/Angle/Types.rs    (in-crate; use crate::display::…)
//   - glean/angle/Glean/Schema/Evolve.rs  (in-crate, manifest row 7; calls
//     displayDefault at Schema/Evolve.hs:399-400 — a call site, no
//     instance)
//   - glean/db Query/{Typecheck,Codegen,Flatten}/Types.rs   (use glean_angle::display::…)
//   - glean/hs RTS/{Term,Types}.rs        (use glean_angle::display::…)
//   plus displayDefault/displayVerbose call sites in glean-db (Schema.rs,
//   Write/JSON.rs, Query/*) and glean-typed (Query/Angle.rs).
// Provided here: `Doc` + the combinator surface those instances use —
// verified by reading the instance sites, not just grepping names:
// pretty (+ string literals via `From`), (<>), (<+>), hsep, vsep, sep,
// cat, vcat, hcat, punctuate, nest, hang, align, parens, brackets,
// braces, and encloseSep (Angle/Types.hs:910, Codegen/Types.hs:383,
// RTS/Term.hs:55,58, RTS/Types.hs:200-201). NOT used by any sibling
// instance: group, list, line, nesting, column — an earlier draft
// claimed them from grep hits that were actually locals, comments, and
// string literals (`group` is a FlatStatementGroup local in
// Flatten/Types.hs:131; "line "/" column " are text in Angle/Types.hs:892;
// list has zero call sites — the RTS/Term.hs list-like sites are
// encloseSep). Those five stay in the algebra below only as internal
// machinery (line/line' feed vsep/vcat, group feeds sep/cat/enclose_sep)
// or prettyprinter API parity (list), not as verified sibling surface.
// Also provided: `DisplayOpts`/`PredicateStyle`, the `Display` trait
// with its default `display_atom`, and display_default/display_verbose.
// Sibling units implement `impl Display for TheirType` against THIS
// trait and NEVER re-declare it.
//
// THREADING (contract property): `Doc` is !Send + !Sync — the
// `column`/`nesting` variants hold `Rc<dyn Fn>`. This matches the
// source's usage: Docs are transient values built, `show`n, and
// discarded on one thread (every render site is a synchronous
// displayDefault/displayVerbose feeding a String); the Haskell never
// shares a Doc across threads. If a future owner ever needs
// cross-thread Docs, Rc→Arc is the mechanical switch — recorded here so
// that change is a visible decision, not a silent one.
//
// NOTE on the hash sibling: this file deliberately has NO code dependency
// on crate::angle::hash — Display.hs does not import Angle.Hash.
// `PredicateStyle::PredicateWithHash` only signals sibling instances to
// render the predicate's version hash; the hash rendering itself is
// already owned by rust/glean-angle/src/angle/hash.rs (Fingerprint's
// std::fmt::Display: exactly 32 lowercase hex digits). Sibling
// implementers of `Display for PredicateId` (Angle/Types.hs:739-744)
// should render that Display output when this style is selected.

// ---------------------------------------------------------------------------
// The Doc stand-in (the inventory gap — read this banner before reviewing)
// ---------------------------------------------------------------------------
//
// RULE §1 (third-party deps row): default NONE; `prettyprinter` is not on
// the whitelist, so no crate provides Doc. RULE §5: the site is not in
// migration/inventory.tsv (grep "Display"/"Prettyprinter" across all
// inventory*.tsv: zero rows), and no manifest unit corresponds to
// Compat.Prettyprinter (it is an external fbcode compat shim re-exporting
// the `prettyprinter` package, like Data.Text or Data.Map — but unlike
// those it has no §1 ecosystem row either). That is an inventory/rulebook
// gap: flagged here, not solved here.
//
// RULE §2 (UNKNOWN rule) applied: most conservative target representation
// + TODO(port) with the open question. "Most conservative" here means
// information-preserving: the sibling Display instances build documents
// with nest/hang/sep/cat (grouped internally) whose layout obligations
// survive until render (layoutCompact/layoutSmart at Database/Schema.hs:1220,
// Query/UserQuery.hs:883,1279, typed/Query/Angle.hs:121). Collapsing Doc
// to an eagerly-rendered String at this layer would destroy that structure
// and force every sibling instance into an unfixable infidelity. The
// algebra below can express everything the verified consumer surface
// needs; it changes nothing about the Haskell unit's own semantics.
//
// The representation is the classic Wadler "pretty" tree, which is what
// prettyprinter implements internally:
//   - Group(flat, broken) is prettyprinter's internal `Union x' x`, where
//     x' is the flattened form: pick `flat` when it fits the remaining
//     width, else `broken` (decision deferred to the future renderer).
//     Per the real 1.7.1 `group` (Internal.hs:613-624), an already-flat
//     or unflattenable document gets NO Union at all — `group` returns
//     it unchanged (see the note [Group: special flattening] there).
//   - a bare `Line` is `hardline`: `flatten` maps it to `Fail`
//     (Internal.hs:692) and `changesUponFlattening` reports it
//     `NeverFlat`, so `group` over anything containing a hardline
//     returns the document unchanged — no flat branch is ever offered
//     to the fits-check.
// RULE §1 (logging row) is unrelated here, but the same laziness
// philosophy applies: no rendering happens in this module; the Haskell
// unit also never renders (it only produces Docs).

use std::ops::Add;
use std::rc::Rc;

/// `Doc ann` from Compat.Prettyprinter, with the `ann` phantom dropped
/// (see the TODO below on glass's annotated documents). An immutable
/// pretty-printing tree — Clone is cheap (Rc'd closures), mirroring the
/// Haskell value semantics of freely-shared Docs.
#[derive(Clone)]
pub enum Doc {
    /// `mempty` / `empty`.
    Empty,
    /// A fragment of text with no line breaks (`pretty (t :: Text)`,
    /// `char`, string literals via OverloadedStrings). The no-newline
    /// invariant of prettyprinter's `Text` constructor; [`Doc::text`]
    /// enforces it by splitting (Internal.hs:470-479).
    Text(String),
    /// `hardline` — an unconditional line break.
    Line,
    /// prettyprinter's internal `Fail` (no public constructor in 1.7.1):
    /// the document that cannot be laid out flat. `flatten` produces it
    /// from a `Line` (Internal.hs:692); `changesUponFlattening` reads it
    /// as `NeverFlat`. A renderer treats it as a failed (unusable) flat
    /// branch.
    Fail,
    /// `x <> y` (`Concat`).
    Cat(Box<Doc>, Box<Doc>),
    /// `nest i x` — indentation added to line breaks inside `x`.
    Nest(i64, Box<Doc>),
    /// `column f` — `f` receives the current column at layout time.
    Column(Rc<dyn Fn(i64) -> Doc>),
    /// `nesting f` — `f` receives the current indent level at layout time.
    Nesting(Rc<dyn Fn(i64) -> Doc>),
    /// prettyprinter's internal `Union x' x`: `(flat, broken)`, with
    /// `x'` the flattened form. Constructed via [`group`] and the
    /// soft-break combinators.
    Group(Box<Doc>, Box<Doc>),
}

impl Doc {
    /// `empty :: Doc ann` (also `mempty`).
    pub fn empty() -> Doc {
        Doc::Empty
    }

    /// `pretty (t :: Text)` / a string literal used as a `Doc` via
    /// OverloadedStrings: raw, unquoted text — with prettyprinter's
    /// newline semantics (Internal.hs:455): @pretty = vsep . map
    /// unsafeTextWithoutNewlines . T.splitOn "\\n"@, i.e. every @\n@
    /// becomes a soft `line`, so a grouped multi-line string reflows to
    /// spaces (the doctest right above that instance: @group (pretty
    /// ("hello\\nworld"))@ renders @hello world@). Nothing else is
    /// transformed — the split is on @\n@ only.
    pub fn text(s: impl Into<String>) -> Doc {
        vsep(s.into().split('\n').map(|part| Doc::Text(part.to_owned())))
    }

    /// `line` — breaks the line when the enclosing group does not fit,
    /// renders as a single space when flattened. Implemented directly as
    /// the (flat, broken) union so no separate `flatAlt` is needed.
    pub fn line() -> Doc {
        Doc::Group(
            Box::new(Doc::Text(" ".to_owned())),
            Box::new(Doc::Line),
        )
    }

    /// `hardline` — always breaks, cannot be flattened by `group`.
    pub fn hardline() -> Doc {
        Doc::Line
    }
}

// OverloadedStrings literals: `"foo" <> display opts x` in the source
// becomes `Doc::from("foo").append(...)` here. RULE §1 (strings row):
// `Text`/`String` → `String`/`&str`. prettyprinter's IsString instance
// is `fromString = pretty . T.pack` (Internal.hs:246-247), so literals
// route through the SAME newline-splitting `pretty` — `Doc::from` must
// not bypass [`Doc::text`].
impl From<&str> for Doc {
    fn from(s: &str) -> Doc {
        Doc::text(s)
    }
}

impl From<String> for Doc {
    fn from(s: String) -> Doc {
        Doc::text(s)
    }
}

/// Semigroup `<>` on Doc. Exposed as a method (Rust has no `<>`); `+` is
/// provided below as syntactic sugar with the same meaning.
impl Doc {
    pub fn append(self, rhs: Doc) -> Doc {
        Doc::Cat(Box::new(self), Box::new(rhs))
    }
}

/// Sugar for `<>` (`Doc::append`) — this `+` IS the Haskell's `<>`,
/// not numeric addition.
impl Add for Doc {
    type Output = Doc;
    fn add(self, rhs: Doc) -> Doc {
        self.append(rhs)
    }
}

/// `x <+> y = x <> pretty ' ' <> y` — space-separated, never breaks.
pub fn append_space(lhs: Doc, rhs: Doc) -> Doc {
    lhs.append(Doc::text(" ")).append(rhs)
}

/// `group` — prettyprinter 1.7.1 verbatim (Internal.hs:613-624): a
/// `Union` comes back unchanged; otherwise `changes_upon_flattening`
/// decides — `Flattened x'` builds `Group(x', x)` (pick flat when it
/// fits), while `AlreadyFlat` and `NeverFlat` return `x` unchanged. The
/// unchanged cases are not an optimization nicety but the source's
/// semantics: no flat branch exists for a document that renders the
/// same either way, and none is OFFERED for one containing a hardline.
pub fn group(d: Doc) -> Doc {
    if let Doc::Group(..) = &d {
        return d; // group (Union x y) = Union x y
    }
    match changes_upon_flattening(&d) {
        FlattenResult::Flattened(flat) => Doc::Group(Box::new(flat), Box::new(d)),
        FlattenResult::AlreadyFlat | FlattenResult::NeverFlat => d,
    }
}

/// prettyprinter's `FlattenResult` (Internal.hs:639-645): a document
/// flattens to a renderably-different form, is already flat, or cannot
/// be flattened (it contains a hard `Line` or a `Fail`).
enum FlattenResult {
    Flattened(Doc),
    AlreadyFlat,
    NeverFlat,
}

/// `changesUponFlattening` (Internal.hs:661-685): computes the flattened
/// form WITHOUT committing to it, so `group` can decline to build a
/// `Union`. `Line`/`Fail` are `NeverFlat`; `Empty`/`Text` are
/// `AlreadyFlat`; `Nest` maps `fmap (Nest i)` over the inner result —
/// the nest level is PRESERVED, not dropped (it is inert on one line
/// but the node is kept, exactly as the source keeps it); `Cat`
/// propagates `NeverFlat` from either side; `Column`/`Nesting` flatten
/// INSIDE the closure (`Flattened (Column (flatten . f))`).
fn changes_upon_flattening(doc: &Doc) -> FlattenResult {
    match doc {
        Doc::Line | Doc::Fail => FlattenResult::NeverFlat,
        Doc::Empty | Doc::Text(_) => FlattenResult::AlreadyFlat,
        // Union x _ -> Flattened x: the first field is flat by invariant.
        Doc::Group(first, _) => {
            let first: &Doc = first;
            FlattenResult::Flattened(first.clone())
        }
        Doc::Nest(i, x) => match changes_upon_flattening(x) {
            FlattenResult::Flattened(x) => {
                FlattenResult::Flattened(Doc::Nest(*i, Box::new(x)))
            }
            already_or_never => already_or_never,
        },
        Doc::Column(f) => {
            let f = f.clone();
            FlattenResult::Flattened(Doc::Column(Rc::new(move |c| flatten(&f(c)))))
        }
        Doc::Nesting(f) => {
            let f = f.clone();
            FlattenResult::Flattened(Doc::Nesting(Rc::new(move |c| flatten(&f(c)))))
        }
        Doc::Cat(x, y) => {
            // &Box<Doc> → &Doc, so the clones below yield `Doc`.
            let (x, y): (&Doc, &Doc) = (x, y);
            match (changes_upon_flattening(x), changes_upon_flattening(y)) {
                (FlattenResult::NeverFlat, _) | (_, FlattenResult::NeverFlat) => {
                    FlattenResult::NeverFlat
                }
                (FlattenResult::Flattened(x), FlattenResult::Flattened(y)) => {
                    FlattenResult::Flattened(Doc::Cat(Box::new(x), Box::new(y)))
                }
                (FlattenResult::Flattened(x), FlattenResult::AlreadyFlat) => {
                    FlattenResult::Flattened(Doc::Cat(Box::new(x), Box::new(y.clone())))
                }
                (FlattenResult::AlreadyFlat, FlattenResult::Flattened(y)) => {
                    FlattenResult::Flattened(Doc::Cat(Box::new(x.clone()), Box::new(y)))
                }
                (FlattenResult::AlreadyFlat, FlattenResult::AlreadyFlat) => {
                    FlattenResult::AlreadyFlat
                }
            }
        }
    }
}

/// The `flatten` bound inside `changesUponFlattening` (Internal.hs:687-
/// 702), also used to flatten `Column`/`Nesting` closure bodies: resolve
/// every soft break to its flat branch (`Union x _ -> flatten x`), map a
/// hard `Line` to `Fail`, and keep `Nest` nodes (`Nest i (flatten x)`).
fn flatten(doc: &Doc) -> Doc {
    match doc {
        Doc::Cat(x, y) => Doc::Cat(Box::new(flatten(x)), Box::new(flatten(y))),
        Doc::Nest(i, x) => Doc::Nest(*i, Box::new(flatten(x))),
        Doc::Line => Doc::Fail,
        Doc::Group(x, _) => flatten(x),
        Doc::Column(f) => {
            let f = f.clone();
            Doc::Column(Rc::new(move |c| flatten(&f(c))))
        }
        Doc::Nesting(f) => {
            let f = f.clone();
            Doc::Nesting(Rc::new(move |c| flatten(&f(c))))
        }
        Doc::Empty | Doc::Text(_) | Doc::Fail => doc.clone(),
    }
}

/// `nest i x`.
pub fn nest(i: i64, d: Doc) -> Doc {
    Doc::Nest(i, Box::new(d))
}

/// `align d = column (\k -> nesting (\i -> nest (k - i) d))` — the
/// prettyprinter definition, verbatim.
pub fn align(d: Doc) -> Doc {
    Doc::Column(Rc::new(move |k: i64| {
        let d = d.clone();
        Doc::Nesting(Rc::new(move |i: i64| {
            let d = d.clone();
            Doc::Nest(k - i, Box::new(d))
        }))
    }))
}

/// `hang i x = align (nest i x)` — the prettyprinter definition.
pub fn hang(i: i64, d: Doc) -> Doc {
    align(nest(i, d))
}

/// `column f` — `f` receives the current output column at layout time.
pub fn column(f: impl Fn(i64) -> Doc + 'static) -> Doc {
    Doc::Column(Rc::new(f))
}

/// `nesting f` — `f` receives the current indentation level at layout
/// time.
pub fn nesting(f: impl Fn(i64) -> Doc + 'static) -> Doc {
    Doc::Nesting(Rc::new(f))
}

/// `hcat = foldr (<>) mempty` — direct concatenation.
pub fn hcat<I: IntoIterator<Item = Doc>>(docs: I) -> Doc {
    let mut out = Doc::Empty;
    for d in docs {
        out = out.append(d);
    }
    out
}

/// `hsep = concatWith (<+>)` — space-separated, never breaks.
pub fn hsep<I: IntoIterator<Item = Doc>>(docs: I) -> Doc {
    join(docs, |a, b| append_space(a, b))
}

/// `line'` (prettyprinter): like `line` but flattens to the empty document
/// instead of a space. Private: only `vcat` needs it on the verified
/// consumer surface.
fn line_prime() -> Doc {
    Doc::Group(Box::new(Doc::Empty), Box::new(Doc::Line))
}

/// `vcat = concatWith (\x y -> x <> line' <> y)` — always breaks
/// (until grouped).
pub fn vcat<I: IntoIterator<Item = Doc>>(docs: I) -> Doc {
    join(docs, |a, b| a.append(line_prime()).append(b))
}

/// `vsep = concatWith (\x y -> x <> line <> y)` — always breaks (until
/// grouped).
pub fn vsep<I: IntoIterator<Item = Doc>>(docs: I) -> Doc {
    join(docs, |a, b| a.append(Doc::line()).append(b))
}

/// `sep xs = group (vsep xs)` — one line if it fits, vertically otherwise.
pub fn sep<I: IntoIterator<Item = Doc>>(docs: I) -> Doc {
    group(vsep(docs))
}

/// `cat xs = group (vcat xs)` — one line if it fits, vertically otherwise
/// (no separator when flat).
pub fn cat<I: IntoIterator<Item = Doc>>(docs: I) -> Doc {
    group(vcat(docs))
}

fn join<I: IntoIterator<Item = Doc>, F>(docs: I, op: F) -> Doc
where
    F: Fn(Doc, Doc) -> Doc,
{
    let mut iter = docs.into_iter();
    let Some(first) = iter.next() else {
        return Doc::Empty;
    };
    let mut out = first;
    for d in iter {
        out = op(out, d);
    }
    out
}

/// `punctuate p [x1,…,xn] = [x1 <> p, …, x(n-1) <> p, xn]` — the
/// prettyprinter definition (separator AFTER each element except the
/// last). Glean relies on the trailing separator, e.g.
/// `vsep $ "{" : punctuate "," fields ++ ["}"]` (Angle/Types.hs:762).
pub fn punctuate(p: Doc, docs: Vec<Doc>) -> Vec<Doc> {
    let mut out = Vec::with_capacity(docs.len());
    let mut iter = docs.into_iter();
    if let Some(first) = iter.next() {
        let mut cur = first;
        for next in iter {
            let sep_p = p.clone();
            out.push(cur.append(sep_p));
            cur = next;
        }
        out.push(cur);
    }
    out
}

/// `encloseSep l r s ds` — prettyprinter 1.7.1 verbatim
/// (Internal.hs:872-881):
/// @[]  -> l <> r@; @[d] -> l <> d <> r@;
/// @_   -> cat (zipWith (<>) (l : repeat s) ds) <> r@ — i.e. horizontal
/// @l d1 s d2 …@ when it fits (via `cat` = group over `vcat`'s `line'`),
/// vertical with the separator prefixed to each continuation line
/// otherwise. On the verified sibling surface at Angle/Types.hs:910,
/// Codegen/Types.hs:383, RTS/Term.hs:55,58, RTS/Types.hs:200-201 — every
/// site except Angle/Types.hs:910 wraps it in `align`.
pub fn enclose_sep<I: IntoIterator<Item = Doc>>(l: Doc, r: Doc, s: Doc, ds: I) -> Doc {
    let ds: Vec<Doc> = ds.into_iter().collect();
    match ds.len() {
        0 => l.append(r),
        // len 1: into_iter yields exactly this element; the fallback is
        // unreachable.
        1 => l
            .append(ds.into_iter().next().unwrap_or(Doc::Empty))
            .append(r),
        _ => {
            // cat (zipWith (<>) (l : repeat s) ds) <> r
            let prefixed = std::iter::once(l)
                .chain(std::iter::repeat(s))
                .zip(ds)
                .map(|(sep, d)| sep.append(d));
            cat(prefixed).append(r)
        }
    }
}

/// `parens = enclose lparen rparen` — plain concatenation, NO grouping
/// (callers write `group (parens …)` themselves, matching the source).
pub fn parens(d: Doc) -> Doc {
    Doc::text("(").append(d).append(Doc::text(")"))
}

/// `brackets = enclose lbracket rbracket`.
pub fn brackets(d: Doc) -> Doc {
    Doc::text("[").append(d).append(Doc::text("]"))
}

/// `braces = enclose lbrace rbrace`.
pub fn braces(d: Doc) -> Doc {
    Doc::text("{").append(d).append(Doc::text("}"))
}

/// `list` (Internal.hs:896-899): @group . encloseSep (flatAlt "[ " "[")
/// (flatAlt " ]" "]") ", "@. Flat form `[1, 20, 300, 4000]` (unspaced
/// brackets, ", " separators); broken form — comma-prefixed continuation
/// lines with the closing bracket on the LAST item's line, per
/// prettyprinter's own doctest (Internal.hs:886-895, `putDocW 10`):
/// ```text
/// [ 1
/// , 20
/// , 300
/// , 4000 ]
/// ```
/// The branches are hand-built rather than derived from
/// [`enclose_sep`] because the source's `flatAlt` bracket widths
/// (`"[ "` collapsing to `"["` when flat) have no variant in this Doc
/// algebra. NOT on the verified sibling surface (zero call sites —
/// the RTS/Term.hs list-like renders use `encloseSep` directly); kept
/// for prettyprinter API parity.
pub fn list<I: IntoIterator<Item = Doc>>(docs: I) -> Doc {
    let docs: Vec<Doc> = docs.into_iter().collect();
    if docs.is_empty() {
        return Doc::text("[]");
    }
    let flat = Doc::text("[")
        .append(join(docs.clone(), |a, b| {
            a.append(Doc::text(", ")).append(b)
        }))
        .append(Doc::text("]"));
    let mut broken = Doc::text("[ ").append(docs[0].clone());
    for d in &docs[1..] {
        broken = broken
            .append(line_prime())
            .append(Doc::text(", "))
            .append(d.clone());
    }
    broken = broken.append(Doc::text(" ]"));
    Doc::Group(Box::new(flat), Box::new(broken))
}

// TODO(port): prettyprinter ownership + scope gap (RULE §5 flag, §2
// UNKNOWN rule): no inventory row and no manifest unit owns
// Compat.Prettyprinter, so this file hosts the Doc stand-in. Open
// question for Step 4: (a) whether Doc should move to a shared pretty
// module (glass/Glass/Pretty/* and the render-side db/typed units also
// depend on it — glass needs annotated Docs via reAnnotateS over
// SimpleDocStream, unrepresented in this ann-less algebra; moving is
// mechanical, every use goes through `use`).
//
// Sub-questions (b) and (c) are CLOSED (round-2 adversarial review,
// verified against the pinned prettyprinter-1.7.1 sources — kept here
// as the record): `punctuate`'s trailing-separator shape matches
// Internal.hs:1162-1167 (@go (d:ds) = (d <> p) : go ds@), `list`'s
// broken layout matches the Internal.hs:886-895 doctest, and the
// former "encloseSep generalization" gap is resolved — `enclose_sep`
// now transcribes Internal.hs:872-881 verbatim. What remains open
// about prettyprinter parity is render BEHAVIOR, not definitions,
// and that is exactly TODO(port) #2 below.

// TODO(port): render/layout parity has no owner anywhere in the manifest:
// layoutCompact/layoutSmart/layoutPretty + renderStrict call sites
// (Database/Schema.hs:1220, Query/UserQuery.hs:883,1279,
// typed/Query/Angle.hs:121, glass Pretty/*) are the parity-critical
// surface for Display output (error messages embed
// `show (displayDefault x)`; serializeType persists layoutCompact text
// into JSON). When those units land, the Wadler fits algorithm (default
// width 80, ribbon fraction) must reproduce prettyprinter byte-for-byte,
// judged against old-binary output. This module deliberately renders
// nothing — Display.hs renders nothing.

// ---------------------------------------------------------------------------
// Glean.Display proper (the actual unit — everything below maps 1:1 to
// glean/angle/Glean/Display.hs)
// ---------------------------------------------------------------------------

/// `data PredicateStyle = PredicateWithHash | PredicateWithoutHash`
/// (Display.hs:22; no deriving in source).
///
/// RULE §2 (deriving Show row, A4/B6): the source has no Show instance,
/// so constructor-shaped naming is the default — the derived Debug below
/// renders `PredicateWithHash`/`PredicateWithoutHash`, the tags a
/// Haskell consumer would see if Show were derived.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PredicateStyle {
    PredicateWithHash,
    PredicateWithoutHash,
}

/// `newtype DisplayOpts = DisplayOpts { predicateStyle :: PredicateStyle }`
/// (Display.hs:24-27; the `-- more later` comment is preserved verbatim
/// in spirit: the record is expected to grow fields).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DisplayOpts {
    pub predicate_style: PredicateStyle,
    // more later
}

/// `defaultDisplayOpts` (Display.hs:29-33): a "human readable" style, for
/// things like error messages.
pub const fn default_display_opts() -> DisplayOpts {
    DisplayOpts {
        predicate_style: PredicateStyle::PredicateWithoutHash,
    }
}

/// `verboseDisplayOpts` (Display.hs:35-38): all the details, for
/// debugging.
pub const fn verbose_display_opts() -> DisplayOpts {
    DisplayOpts {
        predicate_style: PredicateStyle::PredicateWithHash,
    }
}

/// `displayDefault = display defaultDisplayOpts` (Display.hs:41-42).
pub fn display_default<T: Display + ?Sized>(x: &T) -> Doc {
    x.display(&default_display_opts())
}

/// `displayVerbose = display verboseDisplayOpts` (Display.hs:44-45).
pub fn display_verbose<T: Display + ?Sized>(x: &T) -> Doc {
    x.display(&verbose_display_opts())
}

/// A class for paramterising pretty-printers with some options to
/// control what gets printed, for example to select different levels
/// of verbosity. (Display.hs:47-51 — typo "paramterising" is the
/// source's.)
///
/// Naming: this trait shares its name with `std::fmt::Display`. That is
/// the faithful translation of `class Display` (RULE §0:
/// structure-preserving); consumers disambiguate via the module path
/// (`use glean_angle::display::Display;` — no glob re-exports, RULE §4).
/// `std::fmt::Display` remains the RENDERED-string trait (see hash.rs's
/// Fingerprint); this trait is the Doc-producing Glean class.
pub trait Display {
    /// `display :: DisplayOpts -> a -> Doc ann`. The Haskell passes opts
    /// by value; Rust takes it by shared reference (DisplayOpts is Copy).
    fn display(&self, opts: &DisplayOpts) -> Doc;

    /// If necessary, parenthesise the output to make it suitable for
    /// use in an "atomic" position such as a function argument.
    /// (Display.hs:53-56.) The source's default method is
    /// `displayAtom opts = display opts` — kept verbatim; sibling
    /// instances (e.g. Angle/Types.hs:793,959) override it.
    fn display_atom(&self, opts: &DisplayOpts) -> Doc {
        self.display(opts)
    }
}

/// `instance Display () where display _ = pretty` (Display.hs:58-59).
/// prettyprinter's `Pretty ()` is `viaShow`, rendering `()`.
impl Display for () {
    fn display(&self, _opts: &DisplayOpts) -> Doc {
        Doc::text("()")
    }
}

// RULE §1 (strings row): both Haskell instances below map onto the Rust
// `String`/`str` surface — `instance Display String` (Display.hs:61-62)
// and `instance Display Text` (Display.hs:64-65) collapse into the two
// impls here. `pretty` on a string is raw and unquoted, but NOT
// verbatim: prettyprinter's `Pretty Text` turns each "\n" into a soft
// `line` (Internal.hs:455), so a grouped multi-line string reflows to a
// single line — `Doc::text` reproduces exactly that, no more, no less.
impl Display for String {
    fn display(&self, _opts: &DisplayOpts) -> Doc {
        Doc::text(self.clone())
    }
}

impl Display for str {
    fn display(&self, _opts: &DisplayOpts) -> Doc {
        Doc::text(self)
    }
}

// Marker breakdown (2 open TODO(port) markers, both recorded above the
// Glean.Display section): (1) the prettyprinter ownership gap that
// forced the in-file Doc stand-in — its definitional sub-questions
// (punctuate/list/encloseSep) were closed in round 2, only the (a)
// ownership move remains open; (2) the unowned render/layout parity
// surface. No PERF/BUG/SAFETY markers — nothing here is perf-critical,
// the source has no defects to reproduce in this unit, and there is no
// unsafe code.
// PORT STATUS: confidence=medium todos=2
