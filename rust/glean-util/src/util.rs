// Copyright (c) Meta Platforms, Inc. and affiliates.
// All rights reserved.
//
// This source code is licensed under the BSD-style license found in the
// LICENSE file in the root directory of this source tree.

//! RUST-ONLY SCAFFOLDING: this file has NO Haskell counterpart — the
//! Haskell area has no `Glean.Util` root module (glean/util/Glean/Util/
//! holds the Util/*.hs sources). It exists so lib.rs's
//! `pub mod util;` resolves to the `src/util/` directory the manifest's
//! RULE §4 naming generates (glean/util/Glean/Util/Some.hs ->
//! src/util/some.rs). Owned by the same unit that owns the crate
//! scaffolding (src/util/some.rs, manifest line 38). Later units append
//! their own `pub mod` lines here.

// glean/util/Glean/Util/ConfigProvider.hs -> src/util/config_provider.rs
// (manifest line 79). Row 79's append: the ConfigProvider trait (+ the
// deferred parser method, port todo 2) and NullConfigProvider.
pub mod config_provider;

// glean/util/Glean/Util/Disk.hs -> src/util/disk.rs (manifest line 69)
pub mod disk;

// glean/util/Glean/Util/IO.hs -> src/util/io.rs (manifest line 63)
pub mod io;

// glean/util/Glean/Util/Observed.hs -> src/util/observed.rs (manifest line
// 80). Row 80's append: Observed, changing_value/fixed_value, do_on_update.
pub mod observed;

pub mod retry_channel_exception;
pub mod service;
pub mod some;
pub mod thrift_service;

// glean/util/Glean/Util/ThriftSource.hs -> src/util/thrift_source.rs
// (manifest line 81). Row 81's append: ThriftSource (+ its exception), the
// three constructor families and with_value/load/load_default.
pub mod thrift_source;

pub mod vector;

// Coverage: ten `pub mod` lines wiring every glean-util file shipped so
// far (util/config_provider.rs, util/disk.rs, util/io.rs,
// util/observed.rs, util/retry_channel_exception.rs, util/service.rs,
// util/some.rs, util/thrift_service.rs, util/thrift_source.rs,
// util/vector.rs) and nothing else — no re-exports (each submodule is its
// own facade). Row 69's (disk.rs), row 79's (config_provider.rs), row 80's
// (observed.rs) and row 81's (thrift_source.rs) appends are CONSUMED (past
// tense above).
//
// Marker breakdown (closed set — zero of every class): no port todos
// (wiring is complete for all shipped files), no source-defect
// reproductions (no Haskell source), no performance deferrals (no
// code), no inventory divergence (no inventory row can govern a file
// with no Haskell counterpart).
//
// PORT STATUS: confidence=medium todos=0
