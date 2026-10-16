// Ported from: glean/util/Glean/Util/Vector.hs (22 lines)
// Manifest unit: rust/glean-util/src/util/vector.rs (migration/manifest.tsv
// line 27, crate glean-util). RULE §4: one source file -> one target file.
//
// The Haskell module exports `unsafeCoerceVector :: Coercible a b =>
// Vector a -> Vector b`, defined ONLY for `vector < 0.13.0` (the
// `#if !MIN_VERSION_vector(0,13,0)` guard). For vector >= 0.13.0 the
// module exports NOTHING — `Vector.Storable`'s `unsafeCoerce` is the
// in-library equivalent and needs no wrapper.
//
// The Rust port is therefore an empty module: there is no function to
// port. The `vector` crate's `unsafeCoerce` (or `bytemuck` for
// bytewise coercions) covers the pre-0.13 use case directly.
//
// INVENTORY (RULE §5): no inventory.tsv row covers this file (it is a
// utility module with no FFI surface). No divergence to flag.
//
// PORT STATUS: confidence=high todos=0
