// Ported from: glean/util/Glean/Util/ConfigProvider.hs (110 lines).
// Manifest unit: rust/glean-util/src/util/config_provider.rs
// (migration/manifest.tsv line 79). RULE §4: one source file -> one target
// file; the manifest path is authoritative.
//
// WHAT THIS FILE IS: the configuration-provider INTERFACE. A
// `ConfigProvider cfg` is a handle to a source of configuration values that
// may change over time (files on disk in the OSS impl, row 156): it can
// subscribe to a path with a callback, cancel, and fetch-and-deserialize a
// value, and it can classify its own failures so callers can fall back to a
// default. The file also ships `TestConfigProvider` (a programmatic setter
// for tests) and `NullConfigProvider` (always fails — unit tests use it to
// prove they are not reading the environment). 23 modules import this one;
// it imports only base/packages plus Glean.Init.
//
// ---------------------------------------------------------------------------
// INVENTORY (RULE §5; migration/inventory*.tsv)
// ---------------------------------------------------------------------------
//
// ONE row governs this file — inventory.tsv:110 (EXC_TAXONOMY, status
// "unknown"):
//   "isConfigFailure: class method cfg -> SomeException -> Bool identifying
//    config-fetch exceptions (drives fallback-to-default) -> trait method
//    is_config_failure(&self, err: &Error) -> bool; fallback logic consults
//    it instead of exception-type dispatch. :70-71 ... TODO(port): inventory
//    the ConfigProvider impls and the exact exception set each classifies —
//    q: does the OSS file impl classify any non-IO error?"
// Followed: the method is on the trait, taking the dynamic-error image of
// SomeException; the row's TODO is carried verbatim as port todo 1 below
// (it is an inventory question, not a translation decision).
// inventory.tsv:109 is the Impl/ConfigProvider STM_GROUP row — a SIBLING's
// row (row 156), cited here only because its Mutex/inotify decision shapes
// what `subscribe` must accept (see DIVERGENCES 8).
//
// ---------------------------------------------------------------------------
// DEPENDENCY TRUTH (migration/depmap/edges.tsv, checked not assumed)
// ---------------------------------------------------------------------------
//
//   * OUTBOUND (1): edges.tsv:1857 Glean.Init (:30) — used by
//     withConfigOptions (:84) for the withOptions bracket. Init.hsc SHIPPED
//     at 64f6ff8 (manifest line 78), but its withOptions is itself deferred
//     (its TODO #1), so this is a real edge to a partially-shipped unit.
//   * OUTBOUND, outside the depmap because not in this tree:
//       Options.Applicative (Parser, ParserInfo — :28, used at :43 and
//       :78-88). Same blocker as row 78.
//   * INBOUND (23), none edited by this unit — the biggest consumers are
//     db/Database/Config.hs (row 85), db/Database/Env.hs, client/Remote.hs,
//     server/Server.hs, glass/Main.hs, shell/Shell.hs, tools/gleancli
//     (GleanCLI.hs + plugin/GleanCLI/Types.hs), the indexer mains, and
//     util/Impl/ConfigProvider.hs (row 156, the only in-tree instance).
//
// ---------------------------------------------------------------------------
// ASSUMED IMPORTS (RULE §4 B4)
// ---------------------------------------------------------------------------
//
//   None. The file needs no cross-crate types: ByteString becomes &[u8],
//   Text becomes String, and the dynamic exception is std's error trait
//   object. (Row 156's impl will need glean_util::io / std::fs.)
//
// ---------------------------------------------------------------------------
// TRANSLATION NOTES
// ---------------------------------------------------------------------------
//
//   * The class becomes a trait with two associated types: `Options`
//     (ConfigOptions, :36) and the GAT `Subscription<T>` (:51). GATs are the
//     direct image of a higher-kinded associated type.
//   * Every class method's leading `cfg` parameter becomes `&self`; the two
//     constructors (`defaultConfigOptions`, `withConfigProvider`) are
//     associated functions without a receiver, as in the source.
//   * `withConfigProvider` is a CPS bracket: `Options -> (cfg -> IO a) ->
//     IO a` (:49). The closure takes `&Self` so the handle is not moved into
//     it (Haskell shares the value).
//   * The subscribe callback takes `&T`, not `T`: Haskell hands over a
//     shared immutable value, so borrowing avoids forcing `T: Clone`.
//   * `isConfigFailure` keeps its exact role: the caller decides whether to
//     fall back to a default by ASKING the provider, never by matching an
//     error type. Do not collapse it into an enum.
//   * NullConfigProvider's two throws (:107, :109) are `ErrorCall` values,
//     whose `displayException` is the message verbatim; RULE §2's partial-fn
//     row makes them panics with the SAME text.
//
// ---------------------------------------------------------------------------
// DIVERGENCES (deliberate)
// ---------------------------------------------------------------------------
//
//   1. `type family ConfigOptions cfg = opts | opts -> cfg` (:36) is an
//      INJECTIVE type family. A Rust associated type expresses cfg -> opts
//      only; the reverse (opts -> cfg) cannot be stated. Sites that must
//      recover the cfg from an options value carry the cfg parameter
//      explicitly — row 85 (Database/Config.hs) and the program entries are
//      where that shows up (SIBLING-UNIT CONTRACTS).
//   2. `configOptions :: Parser (ConfigOptions cfg)` (:43) and
//      `withConfigOptions` (:78-88) are DEFERRED (port todo 2): optparse-
//      applicative's Parser/ParserInfo has no image in this tree — the same
//      blocker as row 78's TODO #1. The trait therefore lacks one method,
//      and the Null instance's `configOptions = pure
//      NullConfigProviderOptions` (:103) is not shipped either. The first
//      unit that owns a parser image adds both.
//   3. `IO a` becomes `Result<_, ConfigError>` for withConfigProvider,
//      subscribe, get and setTestConfig: the source throws, and the
//      fallback path needs the failure as a value so isConfigFailure can
//      classify it. `cancel` stays infallible (`IO ()` that only returns).
//   4. `SomeException` (:70) becomes
//      `&(dyn std::error::Error + Send + Sync + 'static)` — Rust's dynamic
//      exception image, and the referent of `ConfigError` (the boxed form
//      the fallible methods return). Which concrete errors classify is
//      inventory row 110's open question (port todo 1).
//   5. `Typeable a` (:57, :67) becomes `T: 'static`. It is what dynamic
//      downcasting (and row 156's per-path value cache) requires; it is not
//      a new restriction, because a value stored behind `dyn Any` must be
//      'static anyway.
//   6. `Deserializer a = ByteString -> Either String a` (:32) becomes
//      `Box<dyn Fn(&[u8]) -> Result<T, String> + Send>`: ByteString -> &[u8]
//      and Text (ConfigPath, :34) -> String per RULE §1's strings row. The
//      Either's String is the deserializer's own message, not a
//      ConfigError. `subscribe` takes it BY VALUE (the source's impl
//      captures it for later re-reads; a borrow could not outlive the
//      call), while `get` borrows it (used within the call). `+ Send` is
//      DIVERGENCE 8's threading requirement.
//   7. `data NullConfigSubscription a` (:98) has NO constructors. The Rust
//      image is a struct with one private PhantomData field: an empty enum
//      cannot use its type parameter (E0392), and the private field keeps
//      the type unconstructible outside this module — the observable content
//      of "no constructors". The NAME is `pub` where the source keeps it
//      out of the export list (:12-22), because Rust lints a private type
//      named in a public associated-type definition.
//   8. The subscribe callback gains `+ Send`: the source invokes callbacks
//      on the inotify thread (inventory.tsv:109), so the box must cross
//      threads. This is a type-system requirement, not new behavior; `Sync`
//      is deliberately NOT required (a Mutex needs only Send, and the
//      source permits callbacks that use IORefs).
//   9. `cancel`'s type parameter is not always inferable: when an impl's
//      `Subscription<T>` ignores `T` (NullConfigProvider's does), the call
//      site must write `cancel::<T>(...)`. Haskell infers `a` from the
//      argument's type. Real impls that mention `T` in the subscription
//      (row 156's will) infer it normally.
//
// ---------------------------------------------------------------------------
// SIBLING-UNIT CONTRACTS
// ---------------------------------------------------------------------------
//
//   * row 85 (db/Database/Config.hs) implements ConfigProvider for the
//     database Config. It is the first consumer that will hit DIVERGENCE 1
//     (it defines the options type AND the cfg type); it also owns the
//     first real `configOptions` once the parser question is settled, and
//     it must keep isConfigFailure's fallback semantics.
//   * row 156 (util/Impl/ConfigProvider.hs) is the only in-tree impl: it
//     must satisfy the `+ Send` callback bound (DIVERGENCE 8) and answer
//     inventory row 110's classification question.
//   * row 384 (util/Impl/TestConfigProvider.hs) implements
//     TestConfigProvider; its `set_test_config` must accept the same
//     ConfigPath type.
//   * rows 78/79 share the parser debt: whichever unit first defines a
//     Parser/ParserInfo image must add `config_options` here and
//     `with_options` in init.rs, and record the decision in DECISIONS.md.
//
// MARKERS: two port todos, zero BUG(port), zero PERF(port), zero
// INVENTORY-BUG-CANDIDATE, zero SAFETY notes (no unsafe in this file).

use std::marker::PhantomData;

/// 'type Deserializer a = ByteString -> Either String a' (:32).
///
/// The closure's `String` is the deserializer's own failure message; a
/// config-fetch failure is the separate `ConfigError` the trait methods
/// return. `+ Send` for the same reason as the subscribe callback
/// (DIVERGENCES 8): row 156 stores it beside the callback on the watcher
/// thread.
pub type Deserializer<T> = Box<dyn Fn(&[u8]) -> Result<T, String> + Send>;

/// 'type ConfigPath = Text' (:34). Text -> String per RULE §1.
pub type ConfigPath = String;

/// The dynamic error the fallible methods return — Rust's image of the
/// `SomeException` that `isConfigFailure` classifies (:70).
pub type ConfigError = Box<dyn std::error::Error + Send + Sync + 'static>;

/// 'class ConfigProvider cfg' (:40).
///
/// A provider of configuration values that might change over time.
pub trait ConfigProvider {
    /// 'type family ConfigOptions cfg' (:36). See DIVERGENCES 1 for the
    /// injectivity that cannot be expressed.
    type Options;

    /// 'type Subscription cfg :: * -> *' (:51).
    type Subscription<T>;

    // TODO(port): `configOptions :: Parser (ConfigOptions cfg)` (:43) —
    // deferred with `withConfigOptions` (:78-88) because optparse-
    // applicative's Parser/ParserInfo has no image in this tree (the same
    // blocker as row 78's TODO #1). The first unit that defines a parser
    // image adds the method here, the Null instance's
    // `config_options` (:103), and DECISIONS.md's record of the choice.

    /// 'defaultConfigOptions :: ConfigOptions cfg' (:46).
    fn default_config_options() -> Self::Options;

    /// 'withConfigProvider :: ConfigOptions cfg -> (cfg -> IO a) -> IO a'
    /// (:49). The bracket that creates the provider and hands it to `f`.
    fn with_config_provider<T, F>(options: Self::Options, f: F) -> Result<T, ConfigError>
    where
        F: FnOnce(&Self) -> T;

    /// 'subscribe :: Typeable a => cfg -> ConfigPath -> (a -> IO ()) ->
    /// Deserializer a -> IO (Subscription cfg a)' (:55-61).
    fn subscribe<T: 'static>(
        &self,
        path: &ConfigPath,
        callback: Box<dyn Fn(&T) + Send>,
        deserializer: Deserializer<T>,
    ) -> Result<Self::Subscription<T>, ConfigError>;

    /// 'cancel :: cfg -> Subscription cfg a -> IO ()' (:64).
    fn cancel<T>(&self, subscription: Self::Subscription<T>);

    /// 'get :: Typeable a => cfg -> ConfigPath -> Deserializer a -> IO a'
    /// (:67).
    fn get<T: 'static>(
        &self,
        path: &ConfigPath,
        deserializer: &Deserializer<T>,
    ) -> Result<T, ConfigError>;

    /// 'isConfigFailure :: cfg -> SomeException -> Bool' (:70). Identifies
    /// the exceptions thrown by `get`/`subscribe`; the fallback-to-default
    /// logic consults it instead of matching on error types.
    fn is_config_failure(
        &self,
        exception: &(dyn std::error::Error + Send + Sync + 'static),
    ) -> bool;
}

/// 'class ConfigProvider cfg => TestConfigProvider cfg' (:74): a provider
/// whose values can be set programmatically, for tests.
pub trait TestConfigProvider: ConfigProvider {
    /// 'setTestConfig :: cfg -> ConfigPath -> ByteString -> IO ()' (:75).
    fn set_test_config(&self, path: &ConfigPath, value: &[u8]) -> Result<(), ConfigError>;
}

// ---------------------------------------------------------------------------
// The empty config provider that always fails (:92-110)
// ---------------------------------------------------------------------------

/// 'data NullConfigProvider = NullConfigProvider' (:96).
pub struct NullConfigProvider;

/// 'data NullConfigProviderOptions = NullConfigProviderOptions' (:97).
pub struct NullConfigProviderOptions;

/// 'data NullConfigSubscription a' (:98) — a type with NO constructors; see
/// DIVERGENCES 7 for why this is a struct rather than an empty enum.
pub struct NullConfigSubscription<T>(PhantomData<T>);

impl ConfigProvider for NullConfigProvider {
    type Options = NullConfigProviderOptions;
    type Subscription<T> = NullConfigSubscription<T>;

    // `configOptions = pure NullConfigProviderOptions` (:103) is deferred
    // with the parser surface — see the trait's TODO.

    fn default_config_options() -> NullConfigProviderOptions {
        NullConfigProviderOptions
    }

    fn with_config_provider<T, F>(
        _options: NullConfigProviderOptions,
        f: F,
    ) -> Result<T, ConfigError>
    where
        F: FnOnce(&Self) -> T,
    {
        Ok(f(&NullConfigProvider))
    }

    fn subscribe<T: 'static>(
        &self,
        _path: &ConfigPath,
        _callback: Box<dyn Fn(&T) + Send>,
        _deserializer: Deserializer<T>,
    ) -> Result<NullConfigSubscription<T>, ConfigError> {
        // 'subscribe _ _ _ _ = throwIO $ ErrorCall "NullConfigProvider:
        // subscribe"' (:107).
        panic!("NullConfigProvider: subscribe")
    }

    fn cancel<T>(&self, _subscription: NullConfigSubscription<T>) {
        // 'cancel _ _ = return ()' (:108).
    }

    fn get<T: 'static>(
        &self,
        _path: &ConfigPath,
        _deserializer: &Deserializer<T>,
    ) -> Result<T, ConfigError> {
        // 'get _ _ _ = throwIO $ ErrorCall "NullConfigProvider: get"' (:109).
        panic!("NullConfigProvider: get")
    }

    fn is_config_failure(
        &self,
        _exception: &(dyn std::error::Error + Send + Sync + 'static),
    ) -> bool {
        // 'isConfigFailure _ _ = True' (:110).
        true
    }
}

// TODO(port): inventory.tsv:110's open question — "inventory the
// ConfigProvider impls and the exact exception set each classifies; q: does
// the OSS file impl classify any non-IO error?" Answer it at row 156 (the
// only in-tree impl) and narrow `is_config_failure` there; this trait keeps
// the source's dynamic signature until then.

// PORT STATUS: confidence=medium todos=2
