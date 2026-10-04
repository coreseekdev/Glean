#!/usr/bin/env python3
"""A fake `glean` CLI that exercises the judge's plumbing without a real
build. NOT a semantic implementation: it answers src.File queries by reading
the batch files it was "given" at create/write time (state kept in a state
file under --db-root) and emits fact JSON lines, optionally perturbed via
env var JUDGE_MOCK_BUG (used to prove the judge catches a lying impl)."""
import sys, os, json, glob

root = None
args = sys.argv[1:]
# strip a leading global option pair like --db-root X
if args and args[0] == "--db-root":
    root = args[1]
    args = args[2:]

state = lambda: os.path.join(root, ".mock-state.json") if root else None

def facts():
    out = {}
    if state() and os.path.exists(state()):
        out = json.load(open(state()))
    return out

def save(d):
    json.dump(d, open(state(), "w"))

cmd = args[0]
rest = args[1:]
if cmd == "create":
    db = rest[rest.index("--db") + 1]
    files = [a for a in rest if a.endswith(".json")]
    d = facts()
    d.setdefault(db, [])
    for f in files:
        for batch in json.load(open(f)):
            for fact in batch["facts"]:
                d[db].append({"id": len(d[db]) + 1024,
                              "key": fact["key"]})
    save(d)
    print(f"Creating DB {db}", file=sys.stderr)
elif cmd == "write":
    db = rest[rest.index("--db") + 1]
    files = [a for a in rest if a.endswith(".json")]
    d = facts()
    for f in files:
        for batch in json.load(open(f)):
            for fact in batch["facts"]:
                d[db].append({"id": len(d[db]) + 1024,
                              "key": fact["key"]})
    save(d)
elif cmd == "finish":
    pass
elif cmd == "query":
    db = rest[rest.index("--db") + 1]
    bug = os.environ.get("JUDGE_MOCK_BUG", "")
    fs = facts().get(db, [])
    if bug == "drop-last":
        fs = fs[:-1]
    elif bug == "wrong-value":
        fs = [{**f, "key": f["key"] + "X"} for f in fs]
    elif bug == "reverse":
        fs = list(reversed(fs))
    for f in fs:
        print(json.dumps(f))
else:
    sys.exit(f"mock: unknown command {cmd}")
