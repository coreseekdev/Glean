// Ported from: glean/util/Glean/Util/Trace.hs (144 lines).
// Manifest unit: rust/glean-util/src/util/trace.rs (migration/manifest.tsv
// line 84). RULE §4: one source file -> one target file; the manifest path
// is authoritative.
//
// WHAT THIS FILE IS: the event-trace toolkit used by the backup tests. Two
// halves:
//   * `Listener` (:37-59) — a dynamically typed event sink
//     (`forall a. Typeable a => a -> STM ()`), composable via
//     Semigroup/Monoid, plus `recorder`, which turns one into an event
//     queue, and `notify`, which fires it.
//   * `Expect` (:62-144) — a regular-expression-like matcher over an event
//     sequence: `want`/`wants` for sequences, `combine` (`<>`) for
//     concatenation, `or_else` (`>|<`) for alternatives, `parallel_with`
//     (`|||`) for interleaving, `opt` for optionality, and `expect`, which
//     drains a queue until the expectation is satisfied.
//
// CONSUMERS (migration/depmap/edges.tsv, checked not assumed): three, none
// edited by this unit — db/Database/Backup.hs (the only notifier: :268,
// :280, :332, :352, :357, :394, :419, :428, :439, :479, :493, :523),
// db/Database/Config.hs (:180 `cfgListener`, :245 and :560 `mempty`) and
// db/Database/Types.hs (:283 `envListener`). The only caller of `recorder`
// and `expect` is test/tests/BackupTest.hs (:114, :191-395), which RULE §4
// does not translate — so the matcher half has no non-test consumer in the
// Rust tree yet.
//
// ---------------------------------------------------------------------------
// INVENTORY (RULE §5; migration/inventory*.tsv)
// ---------------------------------------------------------------------------
//
// ONE row governs this file — inventory.tsv:116 == inventory.stm.tsv:17
// (STM_GROUP, status "confirmed"):
//   "Listener -> Vec<Arc<dyn Fn(&Event)>> run under the emitter's caller
//    (Semigroup = concat; source composes in STM so order f then g is
//    preserved). recorder -> std mpsc (Sender clone per notifier, Receiver
//    drained by expect). expect: try_recv loop mirroring readTQueue/
//    unGetTQueue (put-back = push_front semantics via VecDeque)"
// DIVERGED on three points, each recorded below:
//   * the listener is NOT a `Vec` of typed callbacks but a single erased
//     sink over `dyn Any` (DIVERGENCES 1);
//   * `recorder` moves one `Sender` into that sink instead of cloning one
//     per notifier (DIVERGENCES 7);
//   * `expect` takes one blocking `Receiver::recv` per step rather than the
//     row's `try_recv` loop (DIVERGENCES 10).
// The row's `Vec` is the shape you get when the event type is fixed at the
// call site; the source's listener is rank-2 (`forall a. Typeable a =>`,
// :39) and `listener` is `mapM_ f . cast` (:49), i.e. dynamic dispatch that
// *silently ignores* events of other types. A `Vec` of typed callbacks
// cannot express that without a per-event type tag; a `dyn Any` downcast
// preserves it exactly. `Semigroup` is still concatenation in order
// (DIVERGENCES 2), and `Sender` is `Sync` with a non-blocking `send`, so
// one shared sender is equivalent to one clone per notifier.
//
// INVENTORY-BUG-CANDIDATE row 116: target_translation says
// `Vec<Arc<dyn Fn(&Event)>>` plus a `try_recv` loop; this port uses one
// erased `dyn Any` sink and a blocking `recv` — both are what the source
// actually does (DIVERGENCES 1/7/10). Row wording should read "single erased
// sink; blocking read".
//
// The five inventory STM rows behind the site cite (inventory.sites.tsv:384-388,
// :52/:66/:128/:131/:134) include three doc-comment or type-signature lines —
// only :134 is an `atomically`. All are covered by DIVERGENCES 3: there is
// no STM in the target, so `notify` is a direct call and the queue is mpsc +
// VecDeque.
//
// ---------------------------------------------------------------------------
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, checked not assumed)
// ---------------------------------------------------------------------------
//
//   * OUTBOUND: none in-repo. `Util.STM` (:35) is hsthrift's package, not a
//     module in this tree (verified: no STM.hs anywhere under glean/ — the
//     same check unit 80 recorded at observed.rs:39-40 for the same import).
//     `Data.HashMap.Lazy` is unordered-containers -> std::collections::HashMap
//     (the kit's standing mapping; RULEBOOK has no container row);
//     `Data.Hashable` -> Hash + Eq; `Data.Typeable` -> Any + 'static;
//     `Control.Monad` -> nothing (the `join` at :134 is the tail loop).
//   * INBOUND (3): Backup.hs, Config.hs, Types.hs — see CONSUMERS.
//
// ---------------------------------------------------------------------------
// SIBLING-UNIT CONTRACTS
// ---------------------------------------------------------------------------
//
//   * Event (the port of Backup.hs:79-91, owned by the Backup unit):
//     `recorder` requires `A: Clone + Eq + Hash` (DIVERGENCES 7/11), but the
//     Haskell `Event` derives only `(Eq, Generic, Show, Typeable)` (:91). The
//     Rust image must add `Clone` (all eleven constructors are plain data,
//     including the nullary `Waiting`) and implement `Hash`.
//   * Listener: `Send + Sync` (Listener, :179). `Env.hs:230` passes the
//     env's listener into `Warden.spawn_`, so the port's `Env` keeps that
//     bound when it stores one.
//   * `mempty` sites: Config.hs:245 and :560 become `Listener::default()`;
//     Config.hs:180's `cfgListener` field keeps type `Listener`.
//   * `recorder`/`expect` have no non-test consumer: the only caller is
//     BackupTest.hs (:114, :191-395), which RULE §4 does not translate. A
//     later port of that test may use only the `EventQueue` surface
//     (`read`/`unget`).
//
// ---------------------------------------------------------------------------
// ASSUMED IMPORTS (RULE §4 B4)
// ---------------------------------------------------------------------------
//
//   None. Only std is used (any, collections, fmt, hash, sync::mpsc,
//   sync::Arc).
//
// ---------------------------------------------------------------------------
// TRANSLATION NOTES
// ---------------------------------------------------------------------------
//
//   * `Listener` (:39) is `forall a. Typeable a => a -> STM ()`: a rank-2
//     field. Rust cannot store that, so the image is
//     `Arc<dyn Fn(&dyn Any) + Send + Sync>` — the `forall` becomes the
//     erased `dyn Any` parameter and the `Typeable` constraint becomes
//     `A: 'static` at the `listener`/`notify` call sites (the Typeable ->
//     'static mapping unit 79 recorded at config_provider.rs:112-115).
//   * `STM ()` (:39, :42, :45, :58) becomes `()`: the listener body runs
//     under the notifier's caller, which is what the source does too (its
//     STM action is executed by the caller's `atomically`, e.g.
//     Backup.hs:268).
//   * `TQueue a` (:52) has no in-tree port, so `recorder` returns an
//     `EventQueue<A>`: `writeTQueue` -> `Sender::send`, `readTQueue` ->
//     `Receiver::recv`, `unGetTQueue` -> `pending.push_front`. `Sender` is
//     `Sync` (std, Rust >= 1.72), so the sink needs no extra lock.
//   * `expect`'s recursion (:134-141) is a tail loop: the source returns
//     `expect queue p` out of the `atomically` block and `join`s it, i.e.
//     each step is a fresh transaction; the loop is the same sequence of
//     steps without stack growth.
//   * `Data.HashMap.Lazy` (:30-31) -> `std::collections::HashMap`.
//   * `deriving(Show)` (:70) -> a manual `Debug` (DIVERGENCES 6).
//   * `fail` (:133, :142) -> `Result<(), ExpectError>`: `expect :: ... ->
//     IO ()` throws an IOError; a Rust IO action that can fail returns
//     `Result` (DIVERGENCES 5).
//
// ---------------------------------------------------------------------------
// DIVERGENCES
// ---------------------------------------------------------------------------
//
//   1. `Listener` is one erased sink, not the inventory's `Vec` of typed
//      callbacks — see INVENTORY above. Observable difference: none. A
//      listener created for type `A` still ignores every event of another
//      type, as `cast` did (:49).
//   2. `Semigroup`/`Monoid` (:41-45) -> `Listener::combine` and
//      `impl Default for Listener` (Rust has no `<>`; the crate convention
//      is `combine` + `Default`, cf. glean-db/src/logger/database.rs:77).
//      `combine` runs `self` then `other`, preserving the source's order.
//   3. No STM: `notify` is a direct call and the queue is mpsc + VecDeque.
//      The source's `atomically $ notify ...` (Backup.hs:268) becomes a
//      plain call; because the sink runs inline, an event is enqueued
//      before the notifier proceeds. WINDOW: at 4 of the 12 notify sites the
//      enqueue shares a transaction with TVar writes — Backup.hs:326-333 and
//      :473-479 (modifyMeta then notify), :423-428 (exists/abortRestoring
//      then notify), :490-493 (notify then modifyMeta) — so in the source
//      the event commits or rolls back with the catalog write and a retried
//      transaction can re-emit it, while the port's `send` is immediate and
//      unrollbackable. No consumer observes the difference today (the only
//      `expect` caller drives the queue directly), so the divergence is
//      recorded rather than emulated; a future consumer that needs
//      commit-ordered events must route them through the catalog write's own
//      critical section.
//   4. `TQueue a` is not exposed as a general queue: `EventQueue` offers
//      `read`/`unget` only, the two operations `expect` uses. No source
//      caller uses any other TQueue operation — the sole caller
//      (BackupTest.hs:114) passes the queue straight to `expect`.
//   5. `expect` returns `Result<(), ExpectError>` instead of throwing.
//      Message text follows the source's shape (`"reject"`;
//      `"unexpected <event>; expected one of [<k>,<k>]"`, the list rendered
//      comma-without-space as Haskell's `show` does); elements still render
//      with Rust `Debug` where the source uses `Show` (DIVERGENCES 6).
//   6. `deriving(Show)` (:70) -> manual `Debug` printing the source's
//      constructor shape `Expect (fromList [(k,v)]) True`; element rendering
//      is Rust `Debug` rather than `Show`, and `fromList`'s order is
//      HashMap iteration order on both sides.
//   7. `recorder` requires `A: Clone`: the source's listener consumes the
//      event (`a -> STM ()`), the erased sink only borrows it, so the
//      recorder clones before enqueuing. The one caller records `Event`
//      (Backup.hs:79-91): eleven constructors, each nullary or wrapping a
//      `Repo`, including the nullary `Waiting`.
//   8. `recorder`'s sender and `EventQueue::read` tolerate a closed
//      channel (receiver dropped / all senders dropped): the send is
//      dropped and the read yields `ExpectError("event queue closed")`.
//      `TQueue` is unbounded, so `writeTQueue` never blocks; only
//      `readTQueue` blocks (forever) on an empty queue with no writers, and
//      that is the case this returns an error for. Reachable only when the
//      test drops the queue early, i.e. when the source would hang.
//   9. Naming: `>|<` -> `Expect::or_else`, `|||` -> `Expect::parallel_with`,
//      `<>` -> `Expect::combine`, `opt` -> `Expect::opt`;
//      `alt`/`parallel`/`want`/`wants`/`reject`/`accept` keep their names
//      as free functions / constructors. `notify` takes `&A` rather than
//      `A`: `cast` only needs a reference, and the sink must not consume
//      the event before the next listener sees it.
//  10. `expect` reads with one blocking `Receiver::recv` per step, not the
//      inventory row's `try_recv` loop. The source blocks inside
//      `atomically` (`readTQueue`), so a `try_recv` loop would have to
//      spin or report a spurious failure; blocking is the faithful image.
//      The only difference is that a caller cannot interleave other work
//      while waiting, and no in-tree caller does.
//  11. The `Expect` combinators carry `A: Clone` (:363). The source's
//      bounds are only `Eq`/`Hashable` (:98, :131); `Clone` appears because
//      `or_else`/`combine`/`parallel_with` rebuild a `HashMap` from an
//      `Arc`-shared one and must clone its keys. `expect` itself does not
//      need it and is in its own `impl<A: Eq + Hash>` block. Consumers must
//      derive `Clone` on the event type (SIBLING-UNIT CONTRACTS).
//
// ---------------------------------------------------------------------------
// PERF
// ---------------------------------------------------------------------------
//
// `or_else` and `parallel_with` build their result eagerly, where the source
// relies on laziness for both deferral and sharing. Every `Expect` node is
// `Arc`-shared, so the alternatives `[p | b]`/`[q | a]` (:121-122) and the
// `Expect xs False` rests (:118-119) are O(1) copies, but the maps they are
// spliced into are rebuilt at every step, so the constructed state grows
// multiplicatively with the number of interleaved branches — a product of
// the branch counts, not a sum. The shipped consumer is BackupTest.hs's
// `parallel` over one branch per repo (:137-149, three to five repos in the
// fixtures), where this is negligible; a large interleaving would blow up
// where the lazy source stays cheap. Fast version: keep the alternatives as
// an unevaluated list of `(rest, position)` pairs and union them on demand,
// or precompute the interleaving once per `Expect` as a state machine.
// Flagged as PERF(port) below.

use std::any::Any;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::hash::Hash;
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Listener (:37-59)
// ---------------------------------------------------------------------------

/// 'Listener' (:39): 'forall a. Typeable a => a -> STM ()'.
///
/// Cloning shares the sink. The source has no 'Clone' instance, but every
/// consumer stores the value in a record field and 'Config.hs' builds one
/// with 'mempty', so sharing is the faithful image of copying the newtype.
/// 'Send + Sync' is required because listeners are fired from the backup
/// thread (Backup.hs:268 and friends) while the test thread drains the
/// queue.
#[derive(Clone)]
pub struct Listener(Arc<dyn Fn(&dyn Any) + Send + Sync>);

impl Listener {
    /// '(<>)' (:41-42): run 'self' then 'other' for every event.
    pub fn combine(self, other: Listener) -> Listener {
        let first = self.0;
        let second = other.0;
        Listener(Arc::new(move |event: &dyn Any| {
            first(event);
            second(event);
        }))
    }
}

impl Default for Listener {
    /// 'mempty' (:45): a listener that ignores every event.
    fn default() -> Listener {
        Listener(Arc::new(|_: &dyn Any| {}))
    }
}

/// 'listener :: Typeable a => (a -> STM ()) -> Listener' (:48-49).
///
/// 'mapM_ f . cast' (:49) is the downcast: an event of any other type is
/// silently ignored.
pub fn listener<A, F>(f: F) -> Listener
where
    A: 'static,
    F: Fn(&A) + Send + Sync + 'static,
{
    Listener(Arc::new(move |event: &dyn Any| {
        if let Some(event) = event.downcast_ref::<A>() {
            f(event);
        }
    }))
}

/// 'notify :: Typeable a => Listener -> a -> STM ()' (:58-59).
pub fn notify<A: 'static>(listener: &Listener, event: &A) {
    (listener.0)(event);
}

/// 'recorder :: Typeable a => IO (Listener, TQueue a)' (:52-55).
pub fn recorder<A>() -> (Listener, EventQueue<A>)
where
    A: Clone + Send + 'static,
{
    let (sender, events) = channel();
    let listener = listener(move |event: &A| {
        // 'writeTQueue events' (:55). A closed receiver means the queue was
        // dropped; the source would block, so dropping the event is the
        // closest total behaviour (DIVERGENCES 8).
        let _ = sender.send(event.clone());
    });
    (
        listener,
        EventQueue {
            events,
            pending: VecDeque::new(),
        },
    )
}

// ---------------------------------------------------------------------------
// Event queue ('TQueue a', :52, :135, :140)
// ---------------------------------------------------------------------------

/// The 'TQueue a' returned by 'recorder' (:52).
///
/// 'Util.STM' is an external package with no in-tree port, so the queue is
/// a local image: an mpsc receiver plus the pushback end that
/// 'unGetTQueue' (:140) needs. 'read' is 'readTQueue' (:135).
pub struct EventQueue<A> {
    events: Receiver<A>,
    /// Events pushed back by 'unget', newest first. Always drained before
    /// 'events', which is the 'unGetTQueue' end of the queue.
    pending: VecDeque<A>,
}

impl<A> EventQueue<A> {
    /// 'readTQueue queue' (:135).
    pub fn read(&mut self) -> Result<A, ExpectError> {
        if let Some(event) = self.pending.pop_front() {
            return Ok(event);
        }
        self.events
            .recv()
            .map_err(|_| ExpectError::new("event queue closed"))
    }

    /// 'unGetTQueue queue x' (:140).
    pub fn unget(&mut self, event: A) {
        self.pending.push_front(event);
    }
}

// ---------------------------------------------------------------------------
// Expect (:62-144)
// ---------------------------------------------------------------------------

/// 'data Expect a = Expect (HashMap a (Expect a)) Bool' (:67-70).
///
/// The map sits behind an 'Arc' so that 'or_else' and 'parallel_with' share
/// subtrees instead of copying them: the source gets the same sharing from
/// laziness, and a deep copy here would make 'parallel_with' exponential in
/// the number of branches even for a small expectation. Cloning an 'Expect'
/// is therefore O(1) and does not require 'A: Clone'.
pub struct Expect<A> {
    next: Arc<HashMap<A, Expect<A>>>,
    /// 'is this Expect optional' (:69).
    optional: bool,
}

impl<A> Clone for Expect<A> {
    fn clone(&self) -> Expect<A> {
        Expect {
            next: Arc::clone(&self.next),
            optional: self.optional,
        }
    }
}

/// Haskell's `show` for a list: `[a,b]`, comma without a space (§2 Show
/// row, A4/B6; DIVERGENCES 5).
fn show_list<'a, A: fmt::Debug + 'a>(items: impl Iterator<Item = &'a A>) -> String {
    let mut out = String::from("[");
    for (index, item) in items.enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&format!("{:?}", item));
    }
    out.push(']');
    out
}

/// 'deriving(Show)' (:70) prints `Expect (fromList [(k,v)]) True`; this
/// reproduces the constructor shape instead of inventing field names
/// (RULE §2 Show row, A4/B6).
impl<A: fmt::Debug> fmt::Debug for Expect<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut pairs = String::from("[");
        for (index, (key, value)) in self.next.iter().enumerate() {
            if index > 0 {
                pairs.push(',');
            }
            pairs.push_str(&format!("({:?},{:?})", key, value));
        }
        pairs.push(']');
        write!(
            f,
            "Expect (fromList {}) {}",
            pairs,
            if self.optional { "True" } else { "False" }
        )
    }
}

impl<A: Eq + Hash> Expect<A> {
    /// 'reject' (:80-81): an 'Expect' that always fails.
    pub fn reject() -> Expect<A> {
        Expect {
            next: Arc::new(HashMap::new()),
            optional: false,
        }
    }

    /// 'accept' (:84-85), i.e. 'mempty' (:77): an 'Expect' that always
    /// succeeds.
    pub fn accept() -> Expect<A> {
        Expect {
            next: Arc::new(HashMap::new()),
            optional: true,
        }
    }

    /// 'want x' (:88-89): expect the specific event 'event'.
    pub fn want(event: A) -> Expect<A> {
        Expect::singleton(event, Expect::accept(), false)
    }

    /// 'wants [a, b, ...]' (:92-93): expect the events in order.
    pub fn wants<I>(events: I) -> Expect<A>
    where
        I: IntoIterator<Item = A>,
    {
        let events: Vec<A> = events.into_iter().collect();
        let mut expectation = Expect::accept();
        for event in events.into_iter().rev() {
            expectation = Expect::singleton(event, expectation, false);
        }
        expectation
    }

    /// 'Expect (HashMap.singleton x p) False' — the shape every sequence
    /// constructor builds (:89, :93).
    fn singleton(event: A, next: Expect<A>, optional: bool) -> Expect<A> {
        let mut map = HashMap::new();
        map.insert(event, next);
        Expect {
            next: Arc::new(map),
            optional,
        }
    }
}

impl<A: Clone + Eq + Hash> Expect<A> {
    /// 'opt p' (:102-103): 'p' or nothing.
    pub fn opt(self) -> Expect<A> {
        self.or_else(Expect::accept())
    }

    /// 'p >|< q' (:98-99): alternative.
    ///
    /// 'HashMap.unionWith (>|<)' (:99) keeps the first map's value for a
    /// duplicated event and folds the second into it.
    pub fn or_else(self, other: Expect<A>) -> Expect<A> {
        let mut next = HashMap::with_capacity(self.next.len() + other.next.len());
        for (event, expectation) in self.next.iter() {
            next.insert(event.clone(), expectation.clone());
        }
        for (event, expectation) in other.next.iter() {
            match next.entry(event.clone()) {
                Entry::Occupied(mut entry) => {
                    let left = entry.get().clone();
                    entry.insert(left.or_else(expectation.clone()));
                }
                Entry::Vacant(entry) => {
                    entry.insert(expectation.clone());
                }
            }
        }
        Expect {
            next: Arc::new(next),
            optional: self.optional || other.optional,
        }
    }

    /// 'p <> q' (:72-74): 'p' followed by 'q'.
    pub fn combine(self, other: Expect<A>) -> Expect<A> {
        let mut next = HashMap::with_capacity(self.next.len());
        for (event, expectation) in self.next.iter() {
            next.insert(event.clone(), expectation.clone().combine(other.clone()));
        }
        let node = Expect {
            next: Arc::new(next),
            optional: self.optional && other.optional,
        };
        let mut alternatives = vec![node];
        if self.optional {
            alternatives.push(other);
        }
        alt(alternatives)
    }

    /// 'p ||| q' (:113-122): the two expectations may interleave arbitrarily,
    /// with the order inside each one preserved.
    pub fn parallel_with(self, other: Expect<A>) -> Expect<A> {
        let self_optional = self.optional;
        let other_optional = other.optional;
        let self_map = Arc::clone(&self.next);
        let other_map = Arc::clone(&other.next);
        let mut next = HashMap::with_capacity(self.next.len() + other.next.len());
        for (event, expectation) in self.next.iter() {
            let rest = Expect {
                next: Arc::clone(&other_map),
                optional: false,
            };
            next.insert(event.clone(), expectation.clone().parallel_with(rest));
        }
        for (event, expectation) in other.next.iter() {
            let rest = Expect {
                next: Arc::clone(&self_map),
                optional: false,
            };
            let value = rest.parallel_with(expectation.clone());
            match next.entry(event.clone()) {
                Entry::Occupied(mut entry) => {
                    let left = entry.get().clone();
                    entry.insert(left.or_else(value));
                }
                Entry::Vacant(entry) => {
                    entry.insert(value);
                }
            }
        }
        let node = Expect {
            next: Arc::new(next),
            optional: self_optional && other_optional,
        };
        let mut alternatives = vec![node];
        if other_optional {
            alternatives.push(self);
        }
        if self_optional {
            alternatives.push(other);
        }
        alt(alternatives)
    }
}

impl<A: Eq + Hash> Expect<A> {
    /// 'expect queue p' (:131-144): consume events until 'self' is
    /// satisfied, failing on an unexpected one. Always consumes the
    /// smallest possible number of events.
    pub fn expect(&self, queue: &mut EventQueue<A>) -> Result<(), ExpectError>
    where
        A: fmt::Debug,
    {
        let mut expectation = self.clone();
        loop {
            if expectation.next.is_empty() {
                return if expectation.optional {
                    Ok(())
                } else {
                    Err(ExpectError::new("reject"))
                };
            }
            let event = queue.read()?;
            match expectation.next.get(&event).cloned() {
                Some(next) => expectation = next,
                None => {
                    if expectation.optional {
                        queue.unget(event);
                        return Ok(());
                    }
                    return Err(ExpectError::new(format!(
                        "unexpected {:?}; expected one of {}",
                        event,
                        show_list(expectation.next.keys())
                    )));
                }
            }
        }
    }
}

/// 'alt' (:106-107): multiple alternatives, i.e. 'foldr (>|<) reject'.
pub fn alt<A>(alternatives: impl IntoIterator<Item = Expect<A>>) -> Expect<A>
where
    A: Clone + Eq + Hash,
{
    let mut result = Expect::reject();
    let alternatives: Vec<Expect<A>> = alternatives.into_iter().collect();
    for alternative in alternatives.into_iter().rev() {
        result = alternative.or_else(result);
    }
    result
}

/// 'parallel' (:125-126): interleave multiple expectations, i.e.
/// 'foldr (|||) accept'.
pub fn parallel<A>(expectations: impl IntoIterator<Item = Expect<A>>) -> Expect<A>
where
    A: Clone + Eq + Hash,
{
    let mut result = Expect::accept();
    let expectations: Vec<Expect<A>> = expectations.into_iter().collect();
    for expectation in expectations.into_iter().rev() {
        result = expectation.parallel_with(result);
    }
    result
}

// ---------------------------------------------------------------------------
// ExpectError (the 'fail' of :133/:142)
// ---------------------------------------------------------------------------

/// The failure 'expect' reports where the source calls 'fail' (:133, :142).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExpectError(String);

impl ExpectError {
    fn new(message: impl Into<String>) -> ExpectError {
        ExpectError(message.into())
    }

    /// The message the source passes to 'fail'.
    pub fn message(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ExpectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ExpectError {}

// PERF(port): 'alt'/'or_else' and 'parallel'/'parallel_with' construct their
// result eagerly where the source relies on laziness; nodes are Arc-shared so
// the alternatives are O(1) copies, but the spliced maps are rebuilt eagerly
// (see PERF above). Owner: the unit that ports BackupTest.hs, if the fixture
// set ever grows past a handful of repos.
//
// Marker breakdown: one perf deferral (the eager alternation above, owner as
// stated), one inventory-row divergence flag (row 116: the erased sink and
// the blocking read, INVENTORY above), zero port todos.
//
// PORT STATUS: confidence=medium todos=0
