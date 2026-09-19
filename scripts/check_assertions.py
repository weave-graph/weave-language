#!/usr/bin/env python3
"""Exercise explicit structure/claim separation through the real CLIs."""
import argparse
import json
from pathlib import Path
import subprocess
import tempfile

parser = argparse.ArgumentParser()
parser.add_argument("--engine", type=Path, required=True)
args = parser.parse_args()
root = Path(__file__).resolve().parents[1]
plan = json.loads(subprocess.check_output(["cargo", "run", "--locked", "--quiet", "--", "plan", "examples/assertions.weave"], cwd=root))
with tempfile.TemporaryDirectory(prefix="weave-assertions-") as directory:
    work = Path(directory)
    path = work / "plan.json"
    path.write_text(json.dumps(plan))
    engine = str(args.engine.resolve())
    result = json.loads(subprocess.check_output([engine, "run", "--db", str(work / "db"), "--actor", "reader", "--write", "Empty", "--write", "Claims", str(path)]))
    values = {c["name"]: r["result"] for c, r in zip(plan["commands"], result) if c["op"] == "bind"}
    assert not values["NoClaims"]["graph"]["edges"]
    assert values["NoEvidence"]["graph"]["nodes"][0]["properties"]["state"] == "unknown"
    claims = {e["id"]: e for e in values["Evidence"]["graph"]["edges"]}
    assert claims["source-positive"]["structural_ref"]["edge_id"] == claims["source-negative"]["structural_ref"]["edge_id"]
    assert claims["source-positive"]["assertion_source"] == "inspection-17"
    assert claims["source-positive"]["properties"]["category"] == "advisory"
    assert claims["source-positive"]["assertion_properties"]["method"] == "inspection"
    assert values["Scoped"]["graph"]["edges"][0]["assertion_context"] == {"graph_id": "Context", "revision": "r1"}
    assert values["Conflict"]["graph"]["nodes"][0]["properties"]["state"] == "conflicted"
    assert {p["assertion_id"] for p in values["Conflict"]["graph"]["edges"][0]["derived_from"]} == {"source-positive", "source-negative"}
    guarded = {"version": plan["version"], "commands": [{"op": "evaluate", "value": {"kind": "support", "input": {"kind": "query", "query": {"graph_id": "Claims"}}, "predicate": "conditional", "from": {"entity_id": "device-17", "space_id": "operations"}, "to": {"entity_id": "advisory-9", "space_id": "knowledge"}, "valid_at": 15}}]}
    path.write_text(json.dumps(guarded))
    denied = subprocess.run([engine, "run", "--db", str(work / "db"), "--actor", "reader", str(path)], text=True, capture_output=True)
    assert denied.returncode != 0 and "E_CONTEXT_REQUIRED" in denied.stderr
print("Explicit assertion CLI acceptance passed: no implicit claims, separate source attribution/properties, exact claim provenance, contextual preservation and fail-closed context consumption")
