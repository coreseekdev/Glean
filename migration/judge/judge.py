#!/usr/bin/env python3
"""Glean migration judge — portable parity harness (migration kit prompt 00b).

Drives any "glean-compatible CLI" (the original Haskell binary today, the Rust
port later) through the same external interface and diffs the observable
results. The old code is the executable spec; this harness is how you query it.

Semantics mirrored from the in-tree Haskell snapshot driver (do not loosen):
  - glean/test/regression/Glean/Regression/Snapshot/Query.hs   (output protocol:
    ["@generated", <fact>...] where facts are nuked of ids and transformed)
  - glean/test/regression/Glean/Regression/Snapshot/Transform.hs (sort/gensort/
    extract/normord/nukeIds — ported exactly, including Text.JSON Ord ranking)

Known, deliberate deviations (documented, not silently loosened):
  - JSON object key order is treated as NON-semantic (python dict equality).
    Array order IS semantic and strictly compared.
  - The Haskell driver makes ONE userQuery call and never follows
    continuations (Snapshot/Query.hs `collect` passes the continuation
    argument, never the response's), so a byte-truncated page silently
    returns fewer facts than max_results. The CLI always sets max_bytes and
    pages to completion, so the judge returns the full result set. Divergence
    is only possible on corpora large enough to hit byte truncation — none of
    the judge cases are.
  - no_base64_binary=True (driver) cannot be set through the CLI; corpus
    cases must avoid binary-typed fields. The Rust CLI must grow
    --no-base64-binary for the full golden corpus (port requirement).
  - `.query` fields the CLI cannot express (recursive: false, derive: true,
    perf) mark the case skipped-with-reason instead of running with the
    wrong semantics. perf stats are never compared (no v1 case uses them).
  - Number comparison uses python floats/ints (Text.JSON uses exact
    Rationals): two distinct decimal literals that round to the same float
    compare equal here. Impossible to hit with string-typed corpus facts;
    a numeric-heavy corpus would need a decimal-port of the comparator.
  - JSON float NaN cannot appear in goldens (aeson refuses NaN); python
    json.loads CAN parse NaN — a NaN in any sorted position would make
    sorting non-deterministic. Corpus contains none; flagged if seen.
  - Frozen goldens (.out.frozen written from old-code output at validation
    time) are self-fulfilling by construction for validate-old; they are
    eyeballed by the human at the 00b gate, and the mutation-selftest
    validates comparator sensitivity independently of any golden.
  - The driver creates its DBs with the test schema; the judge's DBs use the
    binary's default schema (no schema flags are passed). Golden parity for
    src.* facts therefore also pins the default schema's src.File predicate.
"""

import argparse
import json
import subprocess
import sys
import tempfile
import shutil
import os
from pathlib import Path

import yaml


class JudgeError(Exception):
    pass


class SkipCase(Exception):
    def __init__(self, reason):
        self.reason = reason
        super().__init__(reason)


# ---------------------------------------------------------------- Text.JSON Ord
# Text.JSON's JSValue derives Ord with constructor order:
#   JSNull < JSBool < JSRational < JSString < JSArray < JSObject
# Objects compare as assoc lists (keys lexicographically, then values).
# NOTE: bool check must precede number check (bool subclasses int in Python).
def _tsj_key(v):
    if v is None:
        return (0,)
    if isinstance(v, bool):
        return (1, 1 if v else 0)
    if isinstance(v, (int, float)):
        return (2, v)
    if isinstance(v, str):
        return (3, v)
    if isinstance(v, list):
        return (4, [_tsj_key(x) for x in v])
    if isinstance(v, dict):
        return (5, [(k, _tsj_key(x)) for k, x in v.items()])
    raise JudgeError(f"unsortable JSON value: {v!r}")


def _tsj_sort(facts):
    return sorted(facts, key=_tsj_key)


def _is_num(v):
    return isinstance(v, (int, float)) and not isinstance(v, bool)


# ------------------------------------------------------------------- transforms
def _select(fact, path):
    """select :: JSValue -> [Text] -> JSValue — dies on missing field, like the
    Haskell original (error $ "missing field ...")."""
    cur = fact
    for sel in path:
        if not isinstance(sel, str):
            raise JudgeError(f"non-string path element {sel!r}")
        if isinstance(cur, dict) and sel in cur:
            cur = cur[sel]
        else:
            raise JudgeError(f"missing field '{sel}' in {json.dumps(cur)[:200]}")
    return cur


def _check_arg(name, arg):
    """Match the FromJSON types of the Haskell transforms; a wrongly-typed
    argument must die here, not misbehave later (Aeson would fail the parse)."""
    if name in ("gensort", "normord"):
        if arg not in ([], None, {}):
            raise JudgeError(f"transform '{name}' takes no argument, got {arg!r}")
    elif name == "sort":
        if not (isinstance(arg, list) and
                all(isinstance(p, list) and
                    all(isinstance(s, str) for s in p) for p in arg)):
            raise JudgeError(f"transform 'sort' expects [[field]], got {arg!r}")
    elif name == "extract":
        if not (isinstance(arg, list) and
                all(isinstance(s, str) for s in arg)):
            raise JudgeError(f"transform 'extract' expects [field], got {arg!r}")


def xform_sort(paths, facts):
    if not paths:
        return facts
    return sorted(facts, key=lambda f: [_tsj_key(_select(f, p)) for p in paths])


def xform_gensort(_arg, facts):
    return _tsj_sort(facts)


def xform_extract(path, facts):
    return [_select(f, path) for f in facts]


def xform_normord(_arg, facts):
    def norm(v):
        if isinstance(v, list):
            # Haskell: JSArray (sort (map norm v)) — JSValue Ord, not python-natural
            return sorted((norm(x) for x in v), key=_tsj_key)
        if isinstance(v, dict):
            # Haskell: toJSObject (sort (map (fmap norm) o)) — pairs sorted by key
            return {k: norm(x) for k, x in sorted(v.items())}
        return v
    return _tsj_sort([norm(f) for f in facts])


TRANSFORMS = {
    "sort": xform_sort,
    "gensort": xform_gensort,
    "extract": xform_extract,
    "normord": xform_normord,
}


def nuke_ids(v):
    """Port of nukeIds: a NUMERIC "id" field is dropped from an object unless
    the object has some key outside {id, key, value}. Non-numeric ids (bool,
    string, null) are kept, exactly like the Haskell JSRational match."""
    if isinstance(v, list):
        return [nuke_ids(x) for x in v]
    if isinstance(v, dict):
        has_outer = any(k not in ("id", "key", "value") for k in v)
        return {
            k: nuke_ids(x)
            for k, x in v.items()
            if not (k == "id" and _is_num(x) and not has_outer)
        }
    return v


# ------------------------------------------------------------------- query files
def parse_query_file(path):
    """Fields per Snapshot/Query.hs's FromJSON: query (required), recursive
    (default True), perf (False), max_results, transform (list of name/arg
    pairs), derive (False)."""
    raw = yaml.safe_load(Path(path).read_text(encoding="utf-8"))
    if not isinstance(raw, dict) or "query" not in raw:
        raise JudgeError(f"{path}: not a query file (missing 'query')")
    transforms = []
    tq = raw.get("transform") or []
    if not isinstance(tq, list) or len(tq) % 2 != 0:
        raise JudgeError(f"{path}: transform must be [name, arg, ...] pairs")
    for i in range(0, len(tq), 2):
        name, arg = tq[i], tq[i + 1]
        if name == "xrefs":
            raise SkipCase("driver-supplied transform 'xrefs' not portable")
        if name not in TRANSFORMS:
            raise SkipCase(f"transform '{name}' not in defaultTransforms")
        _check_arg(name, arg)
        transforms.append((name, arg))
    if not raw.get("recursive", True):
        # The CLI cannot express recursive=false: `recurse = recursive ||
        # expand` forces both flags true. Judge refuses to fake it.
        raise SkipCase("recursive: false not expressible through the CLI")
    if raw.get("derive"):
        raise SkipCase("derive: true not expressible through the CLI")
    return {
        "query": raw["query"],
        "recursive": True,
        "max_results": raw.get("max_results"),
        "derive": False,
        "transforms": transforms,
    }


# -------------------------------------------------------------------- impl driver
class Impl:
    """An implementation = a command template containing {root}."""

    def __init__(self, name, template):
        self.name = name
        self.template = template

    def _run(self, args, stdin=None, cwd=None):
        cmd = self.template.format(root=cwd) + " " + " ".join(args)
        proc = subprocess.run(
            cmd, shell=True, input=stdin, cwd=cwd,
            capture_output=True, text=True, encoding="utf-8", timeout=300,
        )
        return proc

    def setup(self, root, repo, batch_files):
        """Create + write + finish through the public CLI. One DB per case:
        first batch via `create`, the rest via `write`, then `finish`."""
        os.makedirs(root, exist_ok=True)
        files = []
        for batch in batch_files:
            src = Path(batch)
            dest = Path(root) / src.name
            shutil.copy(src, dest)
            files.append(src.name)
        for i, name in enumerate(files):
            if i == 0:
                cmd = ["create", "--db", repo, "--file-format", "json", name]
            else:
                cmd = ["write", "--db", repo, "--file-format", "json", name]
            proc = self._run(cmd, cwd=root)
            if proc.returncode != 0:
                raise JudgeError(
                    f"[{self.name}] {'create' if i == 0 else 'write'} "
                    f"failed:\n{proc.stdout}\n{proc.stderr}")
        proc = self._run(["finish", "--db", repo], cwd=root)
        if proc.returncode != 0:
            raise JudgeError(
                f"[{self.name}] finish failed:\n{proc.stdout}\n{proc.stderr}")

    def query(self, root, repo, q):
        args = ["query", "--db", repo, "--recursive", "--expand"]
        if q["max_results"] is not None:
            args += ["--limit", str(q["max_results"])]
        args.append("-")
        proc = self._run(args, stdin=q["query"], cwd=root)
        if proc.returncode != 0:
            raise JudgeError(
                f"[{self.name}] query failed:\n{proc.stdout}\n{proc.stderr}")
        return proc.stdout

    def results(self, root, repo, q):
        """Run one query and normalize exactly like the snapshot driver."""
        out = self.query(root, repo, q)
        facts = []
        for line in out.splitlines():
            line = line.strip()
            if line:
                facts.append(nuke_ids(json.loads(line)))
        for name, arg in q["transforms"]:
            facts = TRANSFORMS[name](arg, facts)
        return ["@generated"] + facts


# ----------------------------------------------------------------------- cases
def load_cases(cases_dir):
    """A case = dir with batch*.json (written in sorted order, one DB) and
    *.query files with a matching .out golden (real) or .out.frozen (written
    from old-code output at validation time; see module docstring)."""
    cases = []
    for cdir in sorted(Path(cases_dir).iterdir()):
        if not cdir.is_dir() or cdir.name.startswith("."):
            continue
        batches = sorted(cdir.glob("batch*.json"))
        queries = sorted(cdir.glob("*.query"))
        if not batches or not queries:
            continue
        case = {
            "name": cdir.name, "dir": cdir,
            "batches": [str(b) for b in batches],
            "queries": [],
        }
        for qf in queries:
            try:
                spec = parse_query_file(qf)
            except SkipCase as e:
                case["queries"].append({
                    "file": qf, "spec": None, "golden": None,
                    "golden_frozen": False, "skip_reason": e.reason,
                })
                continue
            outf = qf.with_suffix(".out")
            frozenf = qf.with_suffix(".out.frozen")
            golden = None
            golden_frozen = False
            if outf.exists():
                golden = json.loads(outf.read_text(encoding="utf-8"))
            elif frozenf.exists():
                golden = json.loads(frozenf.read_text(encoding="utf-8"))
                golden_frozen = True
            case["queries"].append({
                "file": qf, "spec": spec, "golden": golden,
                "golden_frozen": golden_frozen, "skip_reason": None,
            })
        cases.append(case)
    return cases


def first_diff(a, b, path="$"):
    # numbers compare numerically (Text.JSON uses JSRational for all of them);
    # bool must be excluded explicitly since bool subclasses int in Python
    if _is_num(a) and _is_num(b):
        return None if a == b else f"{path}: {a!r} != {b!r}"
    if type(a) is not type(b):
        return f"{path}: type {type(a).__name__} vs {type(b).__name__}"
    if isinstance(a, dict):
        for k in sorted(set(a) | set(b)):
            if k not in a:
                return f"{path}.{k}: missing in left"
            if k not in b:
                return f"{path}.{k}: missing in right"
            d = first_diff(a[k], b[k], f"{path}.{k}")
            if d:
                return d
        return None
    if isinstance(a, list):
        if len(a) != len(b):
            return f"{path}: length {len(a)} vs {len(b)}"
        for i, (x, y) in enumerate(zip(a, b)):
            d = first_diff(x, y, f"{path}[{i}]")
            if d:
                return d
        return None
    return None if a == b else f"{path}: {a!r} != {b!r}"


# ----------------------------------------------------------------------- runner
def run_impl_on_cases(impl, cases, workdir):
    """Returns {case.name: {query-file: normalized results | {'skipped': r}}}.
    Setup/query errors are per-case FAIL entries, never run-killers."""
    results = {}
    for case in cases:
        root = Path(workdir) / impl.name / case["name"]
        shutil.rmtree(root, ignore_errors=True)
        per_query = {}
        try:
            impl.setup(str(root), f"{case['name']}/1", case["batches"])
            for q in case["queries"]:
                if q["skip_reason"]:
                    per_query[str(q["file"])] = {
                        "skipped": q["skip_reason"]}
                    continue
                try:
                    per_query[str(q["file"])] = impl.results(
                        str(root), f"{case['name']}/1", q["spec"])
                except SkipCase as e:
                    per_query[str(q["file"])] = {"skipped": e.reason}
        except (JudgeError, Exception) as e:
            detail = f"setup failed: {type(e).__name__}: {e}"
            for q in case["queries"]:
                per_query[str(q["file"])] = {"skipped": detail}
        results[case["name"]] = per_query
    return results


def validate_old(impl_old, cases, workdir, freeze):
    """Old code is the spec: it must reproduce every real golden cleanly.
    Cases without a golden get their old output frozen (marked as such) when
    --freeze is passed — the frozen files are eyeballed by the human at the
    00b gate. With no --freeze, golden-less cases FAIL validation."""
    report = []
    out = run_impl_on_cases(impl_old, cases, workdir)
    for case in cases:
        for q in case["queries"]:
            key = str(q["file"])
            got = out[case["name"]][key]
            if isinstance(got, dict):  # skipped or setup error
                if "error" in got or got.get("skipped", "").startswith(
                        "setup failed"):
                    report.append({"case": case["name"], "query": key,
                                   "verdict": "FAIL",
                                   "diff": got.get("error")
                                   or got.get("skipped")})
                    continue
                report.append({"case": case["name"], "query": key,
                               "verdict": "skipped",
                               "detail": got.get("skipped")})
                continue
            if q["golden"] is None:
                if freeze:
                    qf = Path(key)
                    qf.with_suffix(".out.frozen").write_text(
                        json.dumps(got, ensure_ascii=False) + "\n",
                        encoding="utf-8")
                    q["golden"] = got
                    q["golden_frozen"] = True
                    report.append({"case": case["name"], "query": key,
                                   "verdict": "frozen",
                                   "detail": "golden written from old output; "
                                             "eyeball it at the gate"})
                else:
                    report.append({"case": case["name"], "query": key,
                                   "verdict": "no-golden",
                                   "detail": "re-run with --freeze to record "
                                             "old output as .out.frozen"})
                continue
            diff = first_diff(got, q["golden"])
            report.append({
                "case": case["name"], "query": key,
                "verdict": "pass" if diff is None else "FAIL",
                "diff": diff,
                "frozen_golden": q["golden_frozen"],
            })
    return report


def parity(impl_old, impl_new, cases, workdir):
    """The actual Step-6 referee: new vs old on identical inputs, plus new vs
    golden where a golden exists. Both comparisons gate the verdict."""
    old = run_impl_on_cases(impl_old, cases, workdir)
    new = run_impl_on_cases(impl_new, cases, workdir)
    report = []
    for case in cases:
        for q in case["queries"]:
            key = str(q["file"])
            o, n = old[case["name"]][key], new[case["name"]][key]
            entry = {"case": case["name"], "query": key}
            o_skip, n_skip = isinstance(o, dict), isinstance(n, dict)
            if o_skip != n_skip:
                # asymmetric skip IS a divergence: one side produced results
                # where the other refused
                entry["verdict"] = "FAIL"
                entry["detail"] = (
                    f"old={'skipped' if o_skip else 'ran'}, "
                    f"new={'skipped' if n_skip else 'ran'}")
                report.append(entry)
                continue
            if o_skip:
                entry["verdict"] = "skipped"
                entry["detail"] = n.get("skipped") or o.get("skipped")
                report.append(entry)
                continue
            d = first_diff(n, o)
            entry["verdict"] = "pass" if d is None else "FAIL"
            if d is not None:
                entry["detail"] = f"new vs old: {d}"
            if q["golden"] is not None:
                d2 = first_diff(n, q["golden"])
                entry["new_vs_golden"] = "match" if d2 is None else f"FAIL: {d2}"
                if d2 is not None:
                    entry["verdict"] = "FAIL"
            report.append(entry)
    return report


def mutation_selftest(impl_old, cases, workdir):
    """The judge must catch deliberately broken implementations. Each mutation
    below perturbs the old implementation's RAW query output (what a broken
    port would emit, before normalization) and the mutated impl is re-run
    through the full pipeline. Mutations that normalization legitimately
    hides (e.g. reversed lines under a gensort transform) are expected
    blind spots: each one must be caught ONLY where semantically visible.
    A mutation nothing catches = a blind spot with no covering case."""
    results = []
    for name, (description, mutate_lines, must_catch_any) in MUTATIONS.items():
        mutant = MutantImpl(impl_old, mutate_lines)
        new = run_impl_on_cases(mutant, cases, workdir)
        caught, errors = [], []
        for case in cases:
            for q in case["queries"]:
                key = str(q["file"])
                o = old_results(impl_old, cases, workdir, case, q, key)
                n = new[case["name"]][key]
                if isinstance(o, dict) or isinstance(n, dict):
                    continue
                try:
                    if first_diff(n, o) is not None:
                        caught.append(f"{case['name']}/{Path(key).name}")
                except Exception as e:
                    errors.append(f"{case['name']}/{Path(key).name}: {e}")
        blind = not caught
        verdict = "ERRORS" if errors else ("MISSED — blind spot" if blind
                                           else "CAUGHT")
        results.append({
            "mutation": name,
            "description": description,
            "caught_on": caught,
            "errors_on": errors,
            "blind_spot_acknowledged": blind and not errors,
            "verdict": verdict,
            "gate": ("MUST-CATCH violated" if blind and must_catch_any
                     else "ok"),
        })
    return results


def old_results(impl_old, cases, workdir, case, q, key, _cache={}):
    ck = (impl_old.template, case["name"], key)
    if ck not in _cache:
        root = str(Path(workdir) / impl_old.name / case["name"])
        _cache[ck] = impl_old.results(root, f"{case['name']}/1", q["spec"])
    return _cache[ck]


class MutantImpl(Impl):
    """Wraps an Impl, perturbing its raw query stdout (line-level, simulating
    a broken port's output before any judge normalization)."""

    def __init__(self, base, mutate_lines):
        super().__init__(base.name, base.template)  # same name → same DB roots
        self.base = base
        self.mutate_lines = mutate_lines

    def query(self, root, repo, q):
        out = self.base.query(root, repo, q)
        lines = [l for l in out.splitlines() if l.strip()]
        lines = self.mutate_lines(lines)
        return "\n".join(lines) + ("\n" if lines else "")


# Raw-output mutations: simulate ways a port could be subtly wrong BEFORE the
# judge normalizes. (name, description, mutator(lines) -> lines, must_catch_any)
def _mut_line(fn):
    return lambda lines: [fn(i, l) for i, l in enumerate(lines)]


MUTATIONS = {
    "drop-last-result": (
        "port drops the final result (truncated paging)",
        lambda lines: lines[:-1],
        True,
    ),
    "mutate-first-value": (
        "port returns a wrong field value",
        _mut_line(lambda i, l: json.dumps({
            **json.loads(l), "key": json.loads(l)["key"] + "X"},
            ensure_ascii=False) if i == 0 else l),
        True,
    ),
    "reverse-lines": (
        "port emits results in reversed order. Blind through any sort "
        "transform by design (gensort hides input order); the unsorted-order "
        "case must catch it.",
        lambda lines: list(reversed(lines)),
        True,
    ),
    "phantom-field": (
        "port emits an extra field per fact",
        _mut_line(lambda i, l: json.dumps({**json.loads(l), "phantom": 1},
                                          ensure_ascii=False)),
        True,
    ),
    "fabricated-fact": (
        "port invents a result that does not exist",
        lambda lines: lines + [json.dumps({"id": 9999, "key": "PHANTOM"})],
        True,
    ),
}

# Verdicts that do NOT count as a passed check.
_BAD = ("FAIL", "MISSED", "ERRORS", "no-golden", "error")


def _row_bad(row):
    v = str(row.get("verdict") or "")
    return any(b in v for b in _BAD) or "FAIL" in str(row.get("new_vs_golden", ""))


# -------------------------------------------------------------------------- cli
def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--cases", required=True)
    ap.add_argument("--work", default=None, help="work dir (default: mktemp)")
    ap.add_argument("--keep", action="store_true")
    sub = ap.add_subparsers(dest="mode", required=True)

    po = sub.add_parser("validate-old")
    po.add_argument("--impl-old", required=True)
    po.add_argument("--freeze", action="store_true",
                    help="freeze old output as .out.frozen for golden-less cases")

    pp = sub.add_parser("parity")
    pp.add_argument("--impl-old", required=True)
    pp.add_argument("--impl-new", required=True)

    pm = sub.add_parser("mutation-selftest")
    pm.add_argument("--impl-old", required=True)

    args = ap.parse_args()
    cases = load_cases(args.cases)
    if not cases:
        print(f"no cases found under {args.cases}", file=sys.stderr)
        return 2

    workdir = args.work or tempfile.mkdtemp(prefix="glean-judge-")
    try:
        if args.mode == "validate-old":
            impl = Impl("old", args.impl_old)
            report = validate_old(impl, cases, workdir, args.freeze)
        elif args.mode == "parity":
            report = parity(Impl("old", args.impl_old),
                            Impl("new", args.impl_new), cases, workdir)
        else:
            report = mutation_selftest(Impl("old", args.impl_old), cases,
                                       workdir)

        n_bad = 0
        for row in report:
            bad = _row_bad(row)
            n_bad += bad
            print(("FAIL " if bad else "ok   ") + json.dumps(
                row, ensure_ascii=False))
        counts = {}
        for row in report:
            counts[row.get("verdict", "FAIL")] = \
                counts.get(row.get("verdict", "FAIL"), 0) + 1
        print(f"\nverdict counts: {counts}")
        print("RESULT: PASS" if n_bad == 0 else f"RESULT: FAIL ({n_bad} bad)")
        return 0 if n_bad == 0 else 1
    finally:
        if not args.keep:
            shutil.rmtree(workdir, ignore_errors=True)


if __name__ == "__main__":
    sys.exit(main())
