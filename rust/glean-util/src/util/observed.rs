// Ported from: glean/util/Glean/Util/Observed.hs (76 lines).
// Manifest unit: rust/glean-util/src/util/observed.rs
// (migration/manifest.tsv line 80). RULE §4: one source file -> one target
// file; the manifest path is authoritative.
//
// WHAT THIS FILE IS: a value that may change over time, plus an efficient
// way to read it and a hook that runs when it changes. `Observed a` holds a
// read action (an STM action in the source) and an optional mutable chain of
// callbacks. `changingValue` creates the mutable pair (the Observed and the
// update action the owner calls when the underlying value changes);
// `fixedValue` creates the immutable case; `doOnUpdate` appends a callback.
// 18 modules import it; every one of them reads config or schema state.
//
// ---------------------------------------------------------------------------
// INVENTORY (RULE §5; migration/inventory*.tsv)
// ---------------------------------------------------------------------------
//
// ONE row governs this file — inventory.tsv:114 == inventory.stm.tsv:15
// (STM_GROUP, status "confirmed"):
//   "struct Observed<A> { Mutex<(A, Vec<Arc<dyn Fn()>>)>, Condvar? } — get
//    returns clone of A; update fn = lock, mutate, run registered callbacks
//    AFTER value visible (source order: modifyTVar then callback,
//    Observed.hs:41-43). fixedValue -> const A. Callbacks may run
//    concurrently (documented) — spawn or run inline (inline is the
//    conservative default; source runs them inline too)"
// Followed, with one structural correction: the value and the callback
// chain live in SEPARATE cells (Mutex<A> + Mutex<Vec<...>>) rather than one
// Mutex<(A, Vec<...>)>, mirroring the source's two cells (TVar + IORef) and
// letting the callbacks run after the value lock is released. A single cell
// would either hold the value lock across user code or lose the source's
// "the Observed has already been updated" guarantee (:56-57).
// The inventory.sites.tsv STM rows are :382 (atomically modifyTVar, :42) and
// :383 (get = atomically . get, :75); both are covered by DIVERGENCES 3.
//
// ---------------------------------------------------------------------------
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, checked not assumed)
// ---------------------------------------------------------------------------
//
//   * OUTBOUND: none in the depmap. The one non-base import, Util.STM (:21),
//     is hsthrift's package, not a module in this tree (verified: no STM.hs
//     anywhere under glean/). Its three primitives map to std::sync:
//     newTVarIO -> Mutex::new, readTVar -> lock+clone, modifyTVar ->
//     lock+write (DIVERGENCES 3).
//   * INBOUND (18), none edited by this unit: db/Database/{Config,Env,
//     Create,Finish,Janitor,Open,Restore,Types,Writes,Write/Queue,Backup,
//     Backup/Locator}.hs, db/Query/{Derive,UserQuery}.hs, server/Server.hs,
//     server/Server/Sharding.hs, backend-local/Backend/Local.hs, and
//     util/ThriftSource.hs (row 81, the only creator: changingValue at
//     :183/:225 and fixedValue at :169).
//
// ---------------------------------------------------------------------------
// ASSUMED IMPORTS (RULE §4 B4)
// ---------------------------------------------------------------------------
//
//   None. Only std::sync is used.
//
// ---------------------------------------------------------------------------
// TRANSLATION NOTES
// ---------------------------------------------------------------------------
//
//   * `observedGet :: STM a` (:25) becomes `Arc<dyn Fn() -> A + Send +
//     Sync>`: an arbitrary read action, which is what both constructors
//     build (`readTVar tvar` and `return a`) and what `fmap` composes.
//   * `observedUpdated :: Maybe (IORef (IO ()))` (:26) becomes
//     `Option<Arc<Mutex<Vec<Arc<dyn Fn() + Send + Sync>>>>>`. The source
//     stores ONE composed action (`io >> action`, :63); a Vec keeps the same
//     registration order, and the Option keeps `fixedValue`'s no-op case.
//   * `changingValue` returns the update action as
//     `Arc<dyn Fn(&dyn Fn(&A) -> A) + Send + Sync>`: callers write
//     `on_update(&|_| v.clone())` for the source's `onUpdate (const v)`
//     (ThriftSource.hs:196) without boxing per call, and Arc because the
//     value is stored by consumers (ThriftSource's `mutable`, :183).
//   * The update takes the new value from a function applied to the current
//     one (`modifyTVar tvar fn`, :42), never from a bare value.
//
// ---------------------------------------------------------------------------
// DIVERGENCES (deliberate)
// ---------------------------------------------------------------------------
//
//   1. `class ObserveMonad m` (:65-75) has no Rust image. It abstracts over
//      STM and IO; there is no STM in the Rust port, so the class has one
//      instance left and `get` becomes an inherent method on `Observed`.
//      Consumers call `obs.get()` where they wrote `Observed.get obs`.
//   2. `instance Functor Observed` (:49-50) becomes the inherent
//      `Observed::map`: Rust has no higher-kinded type parameter, so this
//      cannot be the `Functor` trait. The composition is the source's —
//      map the read action, KEEP the same update chain (the IORef is shared
//      by the mapped value too).
//   3. TVar -> `Mutex<A>`, and `atomically (modifyTVar tvar fn)` (:42)
//      becomes one locked read-modify-write. Two consequences: (a) an
//      `Observed` read can no longer participate in a larger transaction —
//      checked against all 18 importers, none reads one inside an
//      `atomically` block (the only near miss, Backup.hs:451, is in IO), so
//      the loss has no in-tree user today; (b) std's Mutex is NOT
//      reentrant, so the update function must not call `get` on the same
//      `Observed` (STM would allow it). Both are recorded for the first
//      consumer that needs either property.
//   4. The callback chain is snapshotted (the Vec of Arcs is cloned) under
//      its lock and the callbacks run after the value lock is released.
//      That preserves the source's two documented properties (:56-58): the
//      value is already updated when a callback runs, and callbacks may run
//      concurrently (two updaters on different threads each run their own
//      snapshot). `do_on_update` from inside a callback therefore does not
//      deadlock.
//   5. `Observed` derives `Clone` (Arc clones). Haskell values are shared
//      implicitly; Rust needs an explicit handle. Cloning shares the update
//      chain on purpose — the source's IORef is shared too.
//   6. `Send + Sync` bounds: the source's TVar/IORef are usable from any
//      thread (the consumers read config from server and indexer threads),
//      so the read action and the callbacks are `Send + Sync` and
//      `changing_value`/`fixed_value` require `A: Send + Sync`. `get` itself
//      needs only `&self`.
//   7. `doOnUpdate` on a `fixedValue` is a no-op (:62 `forM_` over
//      `Nothing`); the Rust `if let Some` is the same. A `fixedValue`'s read
//      action clones the value on every call (Haskell `return a` shares it),
//      which is the one place this port copies data the source did not.
//
// ---------------------------------------------------------------------------
// SIBLING-UNIT CONTRACTS
// ---------------------------------------------------------------------------
//
//   * row 81 (util/ThriftSource.hs) is the only creator: `once` uses
//     `fixedValue` (:169) and `mutable`/`configWithValue` use
//     `changingValue` (:183, :225) then call the update action with
//     `const a`. Its `ThriftSource` record stores the pair.
//   * row 85 (db/Database/Config.hs) declares `Observed ServerConfig.Config`
//     (:186) and owns the config plumbing that feeds `changing_value`.
//   * The db consumers (Env.hs:79/:117/:190, Sharding.hs:88/:96/:244, ...)
//     call `get` on every read — the source's doc (:67-68) says that is the
//     intended cost. If a future unit needs a borrowed read, add
//     `get_ref(&self) -> MutexGuard` deliberately rather than changing
//     `get`'s signature here.
//
// MARKERS: zero port todos, zero BUG(port), zero PERF(port), zero
// INVENTORY-BUG-CANDIDATE, zero SAFETY notes (no unsafe in this file).

use std::sync::{Arc, Mutex};

/// 'onUpdate :: (a -> a) -> IO ()' (:36, :41-43): the action the owner of a
/// changing value calls with a function that computes the new value from the
/// current one.
///
/// The function receives `Option<&A>`: `None` only for a DEFERRED Observed
/// (see `changing_value_deferred`) before its first update. The source passes
/// a bottom thunk there and relies on the update function being `const`
/// (which never forces it); Rust has no bottoms, so the absence is explicit.
pub type UpdateFn<A> = Arc<dyn Fn(&dyn Fn(Option<&A>) -> A) + Send + Sync>;

/// 'data Observed a' (:24-28) — a value that may change over time, and that
/// we can observe efficiently.
#[derive(Clone)]
pub struct Observed<A> {
    /// 'observedGet :: STM a' (:25). The read action; see DIVERGENCES 1.
    get: Arc<dyn Fn() -> A + Send + Sync>,
    /// 'observedUpdated :: Maybe (IORef (IO ()))' (:26): the callback chain,
    /// or None for a fixed value.
    updated: Option<Arc<Mutex<Vec<Arc<dyn Fn() + Send + Sync>>>>>,
}

impl<A: 'static> Observed<A> {
    /// 'get :: Observed a -> m a' (:69), IO instance (:74-75). Obtain the
    /// current value; cheap enough to call every time you need it (:67-68).
    pub fn get(&self) -> A {
        (self.get)()
    }

    /// 'fmap f (Observed stm upd) = Observed (fmap f stm) upd' (:49-50).
    /// Maps the read action and shares the update chain (DIVERGENCES 2).
    pub fn map<B, F>(self, f: F) -> Observed<B>
    where
        F: Fn(A) -> B + Send + Sync + 'static,
        B: 'static,
    {
        let get = self.get;
        Observed {
            get: Arc::new(move || f(get())),
            updated: self.updated,
        }
    }

    /// 'doOnUpdate :: Observed a -> IO () -> IO ()' (:60-63): run the action
    /// whenever the value is updated. A no-op for a fixed value (:62).
    pub fn do_on_update<F>(&self, action: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        if let Some(chain) = &self.updated {
            chain.lock().unwrap().push(Arc::new(action));
        }
    }
}

/// 'changingValue :: a -> IO (Observed a, (a -> a) -> IO ())' (:34-44).
///
/// Create an 'Observed' that tracks an underlying value. The caller invokes
/// the returned action when the underlying value changes.
pub fn changing_value<A>(initial: A) -> (Observed<A>, UpdateFn<A>)
where
    A: Clone + Send + Sync + 'static,
{
    changing_value_with(Some(initial), "")
}

/// The image of `changingValue (error msg)` — a changing value whose initial
/// value is BOTTOM. `get` panics with `missing` until the first update.
///
/// ThriftSource's `configWithValue` (ThriftSource.hs:225) builds exactly
/// this: the value is only read after `subscribe` has invoked its callback
/// once, so the bottom is never forced (ThriftSource.hs:225-236).
///
/// Why a separate constructor: Rust has no bottom value of type `A` to
/// store, so the uninitialised state must be explicit (`Option<A>`), and the
/// update function must accept the absence (`UpdateFn`'s `Option<&A>`).
pub fn changing_value_deferred<A>(missing: &'static str) -> (Observed<A>, UpdateFn<A>)
where
    A: Clone + Send + Sync + 'static,
{
    changing_value_with(None, missing)
}

fn changing_value_with<A>(initial: Option<A>, missing: &'static str) -> (Observed<A>, UpdateFn<A>)
where
    A: Clone + Send + Sync + 'static,
{
    // 'tvar <- newTVarIO initial' (:38).
    let value = Arc::new(Mutex::new(initial));

    // 'Observed (readTVar tvar)' (:44).
    let read = value.clone();
    let get: Arc<dyn Fn() -> A + Send + Sync> = Arc::new(move || match &*read.lock().unwrap() {
        Some(v) => v.clone(),
        None => panic!("{}", missing),
    });

    // 'updateActionRef <- newIORef (return ())' (:39).
    let chain: Arc<Mutex<Vec<Arc<dyn Fn() + Send + Sync>>>> = Arc::new(Mutex::new(Vec::new()));

    let write = value.clone();
    let callbacks = chain.clone();
    // 'onUpdate fn = do atomically (modifyTVar tvar fn);
    //                  join $ readIORef updateActionRef' (:41-43).
    let on_update: UpdateFn<A> = Arc::new(move |f: &dyn Fn(Option<&A>) -> A| {
        {
            let mut current = write.lock().unwrap();
            *current = Some(f(current.as_ref()));
        }
        // The value is visible before any callback runs (:56-57); the
        // snapshot is taken under the chain's own lock, then released
        // (DIVERGENCES 4).
        let snapshot: Vec<Arc<dyn Fn() + Send + Sync>> = callbacks.lock().unwrap().clone();
        for callback in snapshot {
            callback();
        }
    });

    (
        Observed {
            get,
            updated: Some(chain),
        },
        on_update,
    )
}

/// 'fixedValue :: a -> Observed a' (:46-47): a value that never changes and
/// has no callbacks.
pub fn fixed_value<A>(a: A) -> Observed<A>
where
    A: Clone + Send + Sync + 'static,
{
    Observed {
        get: Arc::new(move || a.clone()),
        updated: None,
    }
}

// PORT STATUS: confidence=high todos=0
