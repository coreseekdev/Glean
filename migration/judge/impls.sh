#!/usr/bin/env bash
# Judge implementation adapters (recorded at 00b validation, 2026-10-05).
#
# old: the original Haskell `glean` CLI built from ~/glean-build/Glean.
#   --schema is REQUIRED in build-tree form because cabal build does not
#   install data-files (the binary otherwise looks in ~/.cabal/share/...).
#   PORT REQUIREMENT: the Rust CLI must embed the schema sources or resolve
#   them reliably (see migration/judge/README.md).
#
# new: the Rust port's glean-compatible CLI (to be filled in Step 3+).
#   Must accept the same surface: create/write/finish/query + --db/--db-root
#   + --file-format json + query via stdin with --recursive --expand.

GLEAN_OLD=/home/nzinfo/glean-build/Glean/dist-newstyle/build/x86_64-linux/ghc-9.4.7/glean-0.2.0.1/x/glean/build/glean/glean
GLEAN_SCHEMA=/home/nzinfo/glean-build/Glean/glean/schema/source

# Usage examples (run from migration/judge/):
#   export LD_LIBRARY_PATH=$HOME/.hsthrift/lib
#   python3 judge.py --cases cases validate-old \
#     --impl-old "$GLEAN_OLD --db-root {root} --schema $GLEAN_SCHEMA" --freeze
#   python3 judge.py --cases cases mutation-selftest \
#     --impl-old "$GLEAN_OLD --db-root {root} --schema $GLEAN_SCHEMA"
#   python3 judge.py --cases cases parity \
#     --impl-old "$GLEAN_OLD --db-root {root} --schema $GLEAN_SCHEMA" \
#     --impl-new "<rust-glean> --db-root {root} --schema <schema-dir>"
