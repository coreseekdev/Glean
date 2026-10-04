#!/usr/bin/env python3
"""Build migration/manifest.tsv from depmap order.txt — Haskell adaptation.

Adapted from the kit's scripts/make_manifest.py (per prompts/01: "adapt one
instead of writing from scratch"). Difference from stock: targets are derived
from each source file's MODULE HEADER via the depmap's crate partition
(RULEBOOK.md §4), because Haskell module paths do not mirror directory paths
and Rust targets are snake_cased:

    <area>/Glean/<Module.Path>.hs  ->  rust/<crate>/src/<module.path.snaked>.rs

where <crate> = depmap_haskell.crate_of(source path). Modules not starting
with "Glean." keep their full dotted path (Derive.Env -> derive/env.rs).

Deterministic: same tree -> byte-identical manifest. Refuses (exit 1) any
target that equals its source.
"""

import argparse
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from depmap_haskell import crate_of, find_hs_files, module_of  # noqa: E402

WORD_RE = re.compile(r"[A-Z]+(?=[A-Z][a-z0-9])|[A-Z]?[a-z0-9]+|[A-Z]+")


def snake(name: str) -> str:
    return "_".join(w.lower() for w in WORD_RE.findall(name)) or name.lower()


def target_for(src_rel: Path, root: Path) -> str:
    mod = module_of(root / src_rel)
    if not mod:
        raise SystemExit(f"no module header: {src_rel}")
    crate = crate_of(src_rel.as_posix())
    parts = mod.split(".")
    if parts[0] == "Glean":
        parts = parts[1:]
    snaked = "/".join(snake(p).replace("'", "_prime") for p in parts)
    return f"rust/{crate}/src/{snaked}.rs"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--order", required=True, type=Path)
    ap.add_argument("--root", required=True, type=Path)
    ap.add_argument("--out", required=True, type=Path)
    args = ap.parse_args()
    root = args.root.resolve()

    # validate the unit set is exactly the depmap's (order.txt is authority)
    known = {p.relative_to(root).as_posix() for p in find_hs_files(root)}
    rows, seen = [], set()
    for line in args.order.read_text().splitlines():
        if not line.strip() or line.startswith("#"):
            continue
        for src in line.split("\t"):
            if src in seen:
                continue
            seen.add(src)
            if src not in known:
                raise SystemExit(f"order.txt file not in unit set: {src}")
            tgt = target_for(Path(src), root)
            if tgt == src:
                raise SystemExit(f"target == source for {src}")
            rows.append((src, tgt))

    args.out.parent.mkdir(parents=True, exist_ok=True)
    with args.out.open("w") as f:
        f.write("source\ttarget\n")
        for s, t in rows:
            f.write(f"{s}\t{t}\n")
    print(f"manifest rows: {len(rows)}")


if __name__ == "__main__":
    sys.exit(main())
