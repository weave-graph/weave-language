#!/usr/bin/env python3
"""Verify actual compiler/runtime finite rule closure and proof composition."""
import argparse
import json
from pathlib import Path
import subprocess
import tempfile

parser = argparse.ArgumentParser()
parser.add_argument("--engine", type=Path, required=True)
args = parser.parse_args()
root = Path(__file__).resolve().parents[1]
plan = json.loads(subprocess.check_output(["cargo", "run", "--locked", "--quiet", "--", "plan", "examples/rules.weave"], cwd=root))
with tempfile.TemporaryDirectory(prefix="weave-rules-") as directory:
    work = Path(directory)
    path = work / "plan.json"
    path.write_text(json.dumps(plan))
    outputs = json.loads(subprocess.check_output([str(args.engine.resolve()), "run", "--db", str(work / "db"), "--actor", "reader", "--write", "Network", str(path)]))
    values = {c["name"]: r["result"] for c, r in zip(plan["commands"], outputs) if c["op"] == "bind"}
    closure, repeated, function = [values[n] for n in ("Closure", "Repeated", "ThroughFunction")]
    assert closure["graph"] == repeated["graph"] == function["graph"]
    assert closure["coverage"] == "complete"
    assert sorted(closure["source_revisions"], key=lambda s: (s["name"], s["revision"], s["digest"])) == sorted(plan["source_revisions"], key=lambda s: (s["name"], s["revision"], s["digest"]))
    ac = next(e for e in closure["graph"]["edges"] if e["predicate"] == "reach" and e["from"] == "a" and e["to"] == "c" and e["valid_time"] == {"start": 5, "end": 10})
    assert any({p["assertion_id"] for p in d["premises"]} == {"ab", "bc"} for d in ac["derivations"])
    assert all(d["parameters"]["module"]["revision"] == "1" for d in ac["derivations"])
    assert any(e["from"] == e["to"] for e in closure["graph"]["edges"] if e["predicate"] == "reach")
    assert all(e["predicate"] == "reach" and e["valid_time"]["start"] <= 7 < e["valid_time"]["end"] for e in values["AtSeven"]["graph"]["edges"])
    assert sum(c["op"] == "commit" for c in plan["commands"]) == 1
print("Rules CLI acceptance passed: cyclic fixed point, idempotence, function equivalence, temporal proof intersection, source identities, graph-valued filtering")
