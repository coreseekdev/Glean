// Ported from: glean/util/Glean/Util/ThriftSource.hs (262 lines).
// Manifest unit: rust/glean-util/src/util/thrift_source.rs
// (migration/manifest.tsv line 81). RULE §4: one source file -> one target
// file; the manifest path is authoritative.
//
// WHAT THIS FILE IS: a SOURCE of a value that may change over time —
// either a constant, a one-shot IO action, an already-live Observed, or a
// path in a ConfigProvider that is subscribed to (with an optional default
// when the path is missing). `with_value` runs an action against the live
// Observed; `load` fetches the current value; `load_default` adds the
// Default fallback. 8 modules import it; it imports ConfigProvider (row 79)
// and Observed (row 80).
//
// ---------------------------------------------------------------------------
// INVENTORY (RULE §5; migration/inventory*.tsv)
// ---------------------------------------------------------------------------
//
// ONE row governs this file — inventory.tsv:129 (STM_GROUP, "confirmed"):
//   "TVar-cached config source with file/remote refresh -> Mutex<Option<T>> +
//    refresh fn; file read at ThriftSource.hs:158 is STRICT BS.readFile (not
//    lazy-IO)"
// Followed: the live value is an Observed (row 80's Mutex-backed cell), the
// file read is std::fs::read (strict, RULE §1's IO row), and the "refresh
// fn" is the ConfigProvider subscription. The lazy-IO site
// (inventory.sites.tsv:474 == inventory.protocol.tsv:15, ":158
// BS.readFile path") is reproduced as an eager read with the same error
// text (TRANSLATION NOTES).
//
// ---------------------------------------------------------------------------
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, checked not assumed)
// ---------------------------------------------------------------------------
//
//   * OUTBOUND (2), both SHIPPED: edges.tsv:1859 ConfigProvider (row 79,
//     ebe1796 + the c63bf3a amendment) and edges.tsv:1860 Observed (row
//     80). No other in-tree edges.
//   * OUTBOUND, outside the depmap because not in this tree:
//       Thrift.Protocol (ThriftSerializable) and Thrift.Protocol.JSON
//       (deserializeJSON) — hsthrift. The Rust image is serde_json on the
//       generated types (RULE §1 JSON row; the meta.rs/files.rs precedent:
//       `serde_json::from_slice::<T>`), with `A: DeserializeOwned` standing
//       for `ThriftSerializable a`.
//   * INBOUND (8), none edited by this unit: client/Glean.hs, client/
//     Remote.hs, client/local/LocalOrRemote.hs, db/Database/{Config,Env}.hs,
//     github/DefaultConfigs.hs (row 82), shell/Shell.hs, and
//     tools/disassemble/Disassemble.hs.
//
// ---------------------------------------------------------------------------
// ASSUMED IMPORTS (RULE §4 B4)
// ---------------------------------------------------------------------------
//
//   * `serde::de::DeserializeOwned` and `serde_json::from_slice` — the
//     hsthrift `ThriftSerializable`/`deserializeJSON` pair (see DEPENDENCY
//     TRUTH). glean-util's Cargo.toml gains the serde edges at Step 4, like
//     glean-db's meta.rs (which calls serde_json the same way).
//
// ---------------------------------------------------------------------------
// TRANSLATION NOTES
// ---------------------------------------------------------------------------
//
//   * The source's record of three RANK-2 fields —
//       thriftSourceWith :: forall cfg b. ConfigProvider cfg
//                         => cfg -> (Observed a -> IO b) -> IO b
//       thriftSourceLoad :: forall cfg. ConfigProvider cfg => cfg -> IO a
//     — cannot be stored in a Rust struct as written: a `dyn` field cannot
//     be generic. It is stored instead as two boxed closures with the
//     rank-2 variables REMOVED from the stored type:
//       with_value: Fn(&C, &mut dyn FnMut(&Observed<A>)) -> Result<(), _>
//       load_value: Fn(&C) -> Result<A, _>
//     `b` disappears because the action is passed IN and its result is
//     captured by the caller (with_value's Option<B>); `cfg` becomes the
//     struct's C parameter. RULE §2's existential row (a record whose only
//     producers are the module's own constructors -> a closed shape) is why
//     this is a struct of closures rather than an invented trait object.
//   * `ThriftSource` is exported WITHOUT its constructor (:9-32), so the
//     set of shapes is closed: every exported builder funnels into `once`,
//     `changes` or `genericConfig` (file/value/parse -> once;
//     mutable -> changes; config* -> genericConfig). The three closure
//     pairs below are therefore the complete image.
//   * `changingValue (error "...")` (:225) -> `changing_value_deferred`
//     (row 80's amendment): the value is bottom until subscribe's
//     synchronous callback fires, which row 156 does inside `subscribe`
//     (Impl/ConfigProvider.hs:80-82).
//   * The acquire/release bracket (:235) -> `CancelOnDrop`, so a panic
//     in the action still cancels.
//
// ---------------------------------------------------------------------------
// DIVERGENCES (deliberate)
// ---------------------------------------------------------------------------
//
//   1. `ThriftSource a` gains a second type parameter C: the ConfigProvider
//      the value is read from. The source's rank-2 `forall cfg` becomes a
//      fixed C because a boxed closure cannot be generic. Every in-tree use
//      site has exactly one cfg type, so this is a type-level annotation,
//      not a behaviour change. DIVERGENCE 4 records the Clone bound it
//      brings.
//   2. The `forall b` of `thriftSourceWith` is realised by `with_value`'s
//      own B parameter: the stored closure takes `&mut dyn FnMut(&Observed)`
//      and the caller's action runs inside it. A `FnOnce` action is
//      supported (it is moved into an Option and taken once).
//   3. `Show` (:83-84) -> `Display`, printing the stored
//      `thriftSourceShow` string verbatim ("ThriftSource Fixed" /
//      "ThriftSource Changes").
//   4. `C: Clone` is required by `generic_config` (and therefore by the
//      config/file constructors): the subscription's callback outlives the
//      call (row 79's `subscribe` stores it), so it cannot borrow `cfg`; it
//      captures a CLONE of the provider handle instead. Haskell shares the
//      cfg value. SIBLING CONTRACT: rows 85/156/384 must make their cfg
//      type cheaply Clone (an Arc handle inside), and the clone must share
//      the provider's state, not fork it.
//   5. `ThriftSourceException Text` (:95-98) -> a struct whose `Display`
//      reproduces the derived `show` shape
//      (`ThriftSourceException "<message>"`); the `Exception` instance is
//      std::error::Error. Text's Show escapes non-ASCII differently from
//      Rust's Debug, so an unusual message renders differently (no
//      inventory row governs it).
//   6. `parse`'s `break (==':')` (:200-210) -> `split_once(':')`, which
//      splits at the FIRST colon identically; the three error texts are
//      byte-identical.
//   7. `fileWithDeserializer`'s read (:156-162) is `std::fs::read` (strict,
//      RULE §1), and its message is the source's exact
//      `invalid Thrift value in file '<path>': <err>`; the IOException
//      case becomes the io::Error boxed into ConfigError.
//
// ---------------------------------------------------------------------------
// SIBLING-UNIT CONTRACTS
// ---------------------------------------------------------------------------
//
//   * row 82 (github/DefaultConfigs.hs) builds the default client/server
//     sources: it chooses the C parameter (ClientConfig / ServerConfig).
//   * row 85 (db/Database/Config.hs) is the biggest consumer: cfgSchemaHook
//     (:169) and cfgServerConfig (:177) store ThriftSource values, and
//     schemaSourceIndexConfig (:358) is the only in-tree `genericConfig`
//     caller with a cfg-dependent mkValue (loadInstances).
//   * row 156 (Impl/ConfigProvider.hs) must invoke the subscribe callback
//     synchronously at least once (it does today, :80-82) — the deferred
//     Observed depends on it, and so does the source (:225-236).
//   * rows 79/80 were amended in c63bf3a for this unit (Arc deserializer,
//     fallible callback, deferred Observed); no further change is expected.
//
// MARKERS: zero port todos, zero BUG(port), zero PERF(port), zero
// INVENTORY-BUG-CANDIDATE, zero SAFETY notes (no unsafe in this file).

use std::fmt;
use std::sync::Arc;

use serde::de::DeserializeOwned;

use super::config_provider::{ConfigError, ConfigPath, ConfigProvider};
use super::observed::{self, Observed, UpdateFn};

/// Re-exported because the source's export list includes it (:20):
/// `Deserializer` is ConfigProvider's type, surfaced here for callers that
/// build a source with a custom decoder.
pub use super::config_provider::Deserializer;

/// 'data ThriftSource a' (:50-57) — a source of values that may change over
/// time. See DIVERGENCES 1 for the C parameter.
pub struct ThriftSource<A, C> {
    with_value:
        Arc<dyn Fn(&C, &mut dyn FnMut(&Observed<A>)) -> Result<(), ConfigError> + Send + Sync>,
    load_value: Arc<dyn Fn(&C) -> Result<A, ConfigError> + Send + Sync>,
    show: &'static str,
}

impl<A, C> Clone for ThriftSource<A, C> {
    fn clone(&self) -> Self {
        ThriftSource {
            with_value: self.with_value.clone(),
            load_value: self.load_value.clone(),
            show: self.show,
        }
    }
}

impl<A, C> fmt::Display for ThriftSource<A, C> {
    /// 'instance Show (ThriftSource a) where show = thriftSourceShow'
    /// (:83-84).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.show)
    }
}

/// 'instance Default a => Default (ThriftSource a) where def = value def'
/// (:92-93).
impl<A, C> Default for ThriftSource<A, C>
where
    A: Default + Clone + Send + Sync + 'static,
{
    fn default() -> Self {
        value(A::default())
    }
}

impl<A, C> ThriftSource<A, C>
where
    A: Clone + Send + Sync + 'static,
    C: ConfigProvider + 'static,
{
    /// 'withValue :: (ConfigProvider cfg, Typeable a) => cfg -> ThriftSource
    /// a -> (Observed a -> IO b) -> IO b' (:75-81).
    pub fn with_value<B, F>(&self, cfg: &C, f: F) -> Result<B, ConfigError>
    where
        F: FnOnce(&Observed<A>) -> B,
    {
        let mut out: Option<B> = None;
        let mut f = Some(f);
        (self.with_value)(cfg, &mut |ob: &Observed<A>| {
            if let Some(f) = f.take() {
                out = Some(f(ob));
            }
        })?;
        Ok(out.expect("ThriftSource::with_value: the action runs exactly once"))
    }

    /// 'load :: (ConfigProvider cfg, Typeable a) => cfg -> ThriftSource a ->
    /// IO a' (:62-66). The value is not cached — use with_value + get.
    pub fn load(&self, cfg: &C) -> Result<A, ConfigError> {
        (self.load_value)(cfg)
    }

    /// 'loadDefault :: (ConfigProvider cfg, Default a, Typeable a) => cfg ->
    /// ThriftSource a -> IO a' (:255-260).
    pub fn load_default(&self, cfg: &C) -> Result<A, ConfigError>
    where
        A: Default,
    {
        match self.load(cfg) {
            Ok(a) => Ok(a),
            Err(e) => {
                if cfg.is_config_failure(&*e) {
                    Ok(A::default())
                } else {
                    Err(e)
                }
            }
        }
    }

    /// 'instance Functor ThriftSource' (:86-90): map the loaded value and
    /// the Observed handed to the action.
    pub fn map<B, F>(self, f: F) -> ThriftSource<B, C>
    where
        B: Clone + Send + Sync + 'static,
        F: Fn(A) -> B + Clone + Send + Sync + 'static,
    {
        let load_value = self.load_value;
        let with_value = self.with_value;
        let f_load = f.clone();
        ThriftSource {
            with_value: Arc::new(
                move |cfg: &C, action: &mut dyn FnMut(&Observed<B>)| -> Result<(), ConfigError> {
                    let f = f.clone();
                    with_value(cfg, &mut |ob: &Observed<A>| {
                        action(&ob.clone().map(f.clone()))
                    })
                },
            ),
            load_value: Arc::new(move |cfg: &C| load_value(cfg).map(|a| f_load(a))),
            show: self.show,
        }
    }
}

// ---------------------------------------------------------------------------
// Creation
// ---------------------------------------------------------------------------

/// 'value :: a -> ThriftSource a' (:164-165) — 'once . return'.
pub fn value<A, C>(a: A) -> ThriftSource<A, C>
where
    A: Clone + Send + Sync + 'static,
{
    once(Arc::new(move || Ok(a.clone())))
}

/// 'once :: IO a -> ThriftSource a' (:167-172). The action is RE-RUN on every
/// load and every with_value — the source does not cache it.
pub fn once<A, C>(io: Arc<dyn Fn() -> Result<A, ConfigError> + Send + Sync>) -> ThriftSource<A, C>
where
    A: Clone + Send + Sync + 'static,
{
    let with_io = io.clone();
    ThriftSource {
        with_value: Arc::new(
            move |_cfg: &C, action: &mut dyn FnMut(&Observed<A>)| -> Result<(), ConfigError> {
                let a = with_io()?;
                action(&observed::fixed_value(a));
                Ok(())
            },
        ),
        load_value: Arc::new(move |_cfg: &C| io()),
        show: "ThriftSource Fixed",
    }
}

/// 'changes :: Observed a -> ThriftSource a' (:174-179).
pub fn changes<A, C>(ob: Observed<A>) -> ThriftSource<A, C>
where
    A: Clone + Send + Sync + 'static,
{
    let load_ob = ob.clone();
    ThriftSource {
        with_value: Arc::new(
            move |_cfg: &C, action: &mut dyn FnMut(&Observed<A>)| -> Result<(), ConfigError> {
                action(&ob);
                Ok(())
            },
        ),
        load_value: Arc::new(move |_cfg: &C| Ok(load_ob.get())),
        show: "ThriftSource Changes",
    }
}

/// 'mutable :: a -> IO (ThriftSource a, (a -> a) -> IO ())' (:181-185).
pub fn mutable<A, C>(x: A) -> (ThriftSource<A, C>, UpdateFn<A>)
where
    A: Clone + Send + Sync + 'static,
{
    let (ob, on_update) = observed::changing_value(x);
    (changes(ob), on_update)
}

/// 'config :: (Typeable a, ThriftSerializable a) => Text -> ThriftSource a'
/// (:102-107).
pub fn config<A, C>(path: ConfigPath) -> ThriftSource<A, C>
where
    A: DeserializeOwned + Clone + Send + Sync + 'static,
    C: ConfigProvider + Clone + Send + Sync + 'static,
{
    config_with_deserializer(path, deserialize_json())
}

/// 'configDefault :: (Typeable a, Default a, ThriftSerializable a) => Text ->
/// ThriftSource a' (:111-116).
pub fn config_default<A, C>(path: ConfigPath) -> ThriftSource<A, C>
where
    A: DeserializeOwned + Default + Clone + Send + Sync + 'static,
    C: ConfigProvider + Clone + Send + Sync + 'static,
{
    config_with_deserializer_default(path, deserialize_json())
}

/// 'configWithDeserializer :: Typeable a => Text -> Deserializer a ->
/// ThriftSource a' (:121-126).
pub fn config_with_deserializer<A, C>(path: ConfigPath, d: Deserializer<A>) -> ThriftSource<A, C>
where
    A: Clone + Send + Sync + 'static,
    C: ConfigProvider + Clone + Send + Sync + 'static,
{
    generic_config(path, d, Arc::new(|_cfg: &C, c: A| Ok(c)), None)
}

/// 'configWithDeserializerDefault :: (Typeable a, Default a) => Text ->
/// Deserializer a -> ThriftSource a' (:130-136).
pub fn config_with_deserializer_default<A, C>(
    path: ConfigPath,
    d: Deserializer<A>,
) -> ThriftSource<A, C>
where
    A: Default + Clone + Send + Sync + 'static,
    C: ConfigProvider + Clone + Send + Sync + 'static,
{
    generic_config(
        path,
        d,
        Arc::new(|_cfg: &C, c: A| Ok(c)),
        Some(A::default()),
    )
}

/// 'file :: ThriftSerializable a => FilePath -> ThriftSource a' (:153-154).
pub fn file<A, C>(path: String) -> ThriftSource<A, C>
where
    A: DeserializeOwned + Clone + Send + Sync + 'static,
{
    file_with_deserializer(path, deserialize_json())
}

/// 'fileWithDeserializer :: FilePath -> Deserializer a -> ThriftSource a'
/// (:156-162). Strict read (:158), source-exact error text.
pub fn file_with_deserializer<A, C>(
    path: String,
    deserializer: Deserializer<A>,
) -> ThriftSource<A, C>
where
    A: Clone + Send + Sync + 'static,
{
    once(Arc::new(move || {
        let bytes = std::fs::read(&path).map_err(|e| -> ConfigError { Box::new(e) })?;
        deserializer(&bytes).map_err(|err| -> ConfigError {
            Box::new(ThriftSourceException(format!(
                "invalid Thrift value in file '{}': {}",
                path, err
            )))
        })
    }))
}

/// 'genericConfig :: Typeable c => Text -> Deserializer c ->
/// (forall cfg. ConfigProvider cfg => cfg -> c -> IO a) -> Maybe a ->
/// ThriftSource a' (:138-150).
///
/// The 'mkValue' argument is a boxed closure taking '&C': the source's
/// rank-2 'forall cfg' becomes the struct's C (DIVERGENCES 1, 4).
pub fn generic_config<A, C, C2>(
    key: ConfigPath,
    deserializer: Deserializer<C2>,
    mk_value: Arc<dyn Fn(&C, C2) -> Result<A, ConfigError> + Send + Sync>,
    default: Option<A>,
) -> ThriftSource<A, C>
where
    A: Clone + Send + Sync + 'static,
    C: ConfigProvider + Clone + Send + Sync + 'static,
    C2: Clone + Send + 'static,
{
    let load_value = {
        let key = key.clone();
        let deserializer = deserializer.clone();
        let mk_value = mk_value.clone();
        let default = default.clone();
        Arc::new(move |cfg: &C| -> Result<A, ConfigError> {
            // 'configLoad' (:238-249): get, then mkValue; a config failure
            // with a default falls back, anything else propagates.
            let attempt = cfg
                .get::<C2>(&key, &deserializer)
                .and_then(|c2| mk_value(cfg, c2));
            match attempt {
                Ok(a) => Ok(a),
                Err(e) => match default.as_ref() {
                    Some(d) if cfg.is_config_failure(&*e) => Ok(d.clone()),
                    _ => Err(e),
                },
            }
        })
    };

    let with_value = Arc::new(
        move |cfg: &C, action: &mut dyn FnMut(&Observed<A>)| -> Result<(), ConfigError> {
            // 'changingValue (error "ThriftSource.withValue: internal
            // error")' (:225).
            let (ob, on_update) =
                observed::changing_value_deferred("ThriftSource.withValue: internal error");

            // 'callback new = do a <- mkValue cfgapi new; onUpdate (const a)'
            // (:229). The provider handle is cloned in: the callback is
            // stored by the provider and outlives this call (DIVERGENCES 4).
            let callback = {
                let mk_value = mk_value.clone();
                let on_update = on_update.clone();
                let cfg = cfg.clone();
                Box::new(move |c2: &C2| -> Result<(), ConfigError> {
                    let a = mk_value(&cfg, c2.clone())?;
                    on_update(&|_| a.clone());
                    Ok(())
                })
            };

            // 'acquire = (Just <$> subscribe ...) `catch` ...' (:231-236).
            match cfg.subscribe::<C2>(&key, callback, deserializer.clone()) {
                Ok(subscription) => {
                    // 'bracket acquire (mapM (cancel cfgapi)) $ const $
                    // action ob' (:235).
                    let guard = CancelOnDrop::new(cfg, subscription);
                    action(&ob);
                    drop(guard);
                    Ok(())
                }
                Err(e) => {
                    if let (true, Some(d)) = (cfg.is_config_failure(&*e), default.as_ref()) {
                        on_update(&|_| d.clone());
                        action(&ob);
                        Ok(())
                    } else {
                        Err(e)
                    }
                }
            }
        },
    );

    ThriftSource {
        with_value,
        load_value,
        show: "ThriftSource Fixed",
    }
}

/// 'parse :: (Typeable a, ThriftSerializable a) => String -> Either String
/// (ThriftSource a)' (:187-191).
pub fn parse<A, C>(s: &str) -> Result<ThriftSource<A, C>, String>
where
    A: DeserializeOwned + Clone + Send + Sync + 'static,
    C: ConfigProvider + Clone + Send + Sync + 'static,
{
    parse_with_deserializer(s, deserialize_json())
}

/// 'parseWithDeserializer :: Typeable a => String -> Deserializer a ->
/// Either String (ThriftSource a)' (:195-210).
pub fn parse_with_deserializer<A, C>(
    s: &str,
    des: Deserializer<A>,
) -> Result<ThriftSource<A, C>, String>
where
    A: Clone + Send + Sync + 'static,
    C: ConfigProvider + Clone + Send + Sync + 'static,
{
    // 'case break (==':') s of' (:200).
    match s.split_once(':') {
        Some(("config", path)) => Ok(config_with_deserializer(path.to_string(), des)),
        Some(("file", path)) => Ok(file_with_deserializer(path.to_string(), des)),
        Some((p, _)) => Err(format!("unsupported Thrift provider '{}'", p)),
        None => Err(format!("invalid Thrift source '{}'", s)),
    }
}

// ---------------------------------------------------------------------------
// ThriftSourceException (:95-98)
// ---------------------------------------------------------------------------

/// 'newtype ThriftSourceException = ThriftSourceException Text
/// deriving (Show)' (:95-96), plus the Exception instance (:98).
#[derive(Debug)]
pub struct ThriftSourceException(pub String);

impl fmt::Display for ThriftSourceException {
    /// The derived 'show' shape: the constructor name, then the Text's
    /// quoted rendering (DIVERGENCES 5).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ThriftSourceException {:?}", self.0)
    }
}

impl std::error::Error for ThriftSourceException {}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// The image of 'ThriftSerializable a''s 'deserializeJSON' (hsthrift): serde
/// on the generated type (RULE §1 JSON row; ASSUMED IMPORTS).
fn deserialize_json<A>() -> Deserializer<A>
where
    A: DeserializeOwned,
{
    Arc::new(|bytes: &[u8]| serde_json::from_slice::<A>(bytes).map_err(|e| e.to_string()))
}

/// The 'bracket acquire (mapM (cancel cfgapi))' release half (:235): cancel
/// the subscription when the action is done, INCLUDING on panic.
struct CancelOnDrop<'a, C: ConfigProvider, T> {
    cfg: &'a C,
    subscription: Option<C::Subscription<T>>,
}

impl<'a, C: ConfigProvider, T> CancelOnDrop<'a, C, T> {
    fn new(cfg: &'a C, subscription: C::Subscription<T>) -> Self {
        CancelOnDrop {
            cfg,
            subscription: Some(subscription),
        }
    }
}

impl<'a, C: ConfigProvider, T> Drop for CancelOnDrop<'a, C, T> {
    fn drop(&mut self) {
        if let Some(subscription) = self.subscription.take() {
            self.cfg.cancel::<T>(subscription);
        }
    }
}

// PORT STATUS: confidence=medium todos=0
