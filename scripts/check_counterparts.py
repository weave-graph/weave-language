#!/usr/bin/env python3
"""Declared directed bridges stay reusable graph evidence, not accepted identity."""
import argparse
import json
from pathlib import Path
import subprocess
import tempfile

parser = argparse.ArgumentParser()
parser.add_argument("--engine", type=Path, required=True)
args = parser.parse_args()
root = Path(__file__).resolve().parents[1]
plan = json.loads(subprocess.check_output(["cargo", "run", "--locked", "--quiet", "--", "plan", "examples/counterparts.weave"], cwd=root))
with tempfile.TemporaryDirectory(prefix="weave-counterparts-") as directory:
    work = Path(directory)
    path = work / "plan.json"
    path.write_text(json.dumps(plan))
    outputs = json.loads(subprocess.check_output([str(args.engine.resolve()), "run", "--db", str(work / "db"), "--actor", "reader", "--write", "Mappings", str(path)]))
    values = {c["name"]: r["result"] for c, r in zip(plan["commands"], outputs) if c["op"] == "bind"}
    bridge = values["Bridge"]
    assert [e["id"] for e in bridge["graph"]["edges"]] == ["declared-bridge"]
    edge = bridge["graph"]["edges"][0]
    assert edge["structural_ref"]["edge_id"] == "bridge"
    assert edge["assertion_source"] == "mapping-proposal-1"
    assert edge["valid_time"] == {"start": 0, "end": 10}
    assert bridge["graph"]["schema"]["id"] == "Manifestations"
    nodes = {n["id"]: n for n in bridge["graph"]["nodes"]}
    assert nodes["physical-device"]["entity_id"] == nodes["operations-device"]["entity_id"] == "device-17"
    assert nodes["physical-device"]["space_id"] == "physical"
    assert nodes["operations-device"]["space_id"] == "operations"
    assert nodes["physical-device"]["properties"]["state"] == "observed"
    assert nodes["operations-device"]["properties"]["state"] == "registered"
    assert bridge["graph"] == values["ThroughFunction"]["graph"]
    assert not values["AtBoundary"]["graph"]["edges"] and not values["Reverse"]["graph"]["edges"]
    traversal = values["Traversal"]["graph"]["edges"]
    assert len(traversal) == 1 and traversal[0]["valid_time"] == {"start": 5, "end": 10}
    assert {p["assertion_id"] for p in traversal[0]["derived_from"]} == {"declared-bridge", "advisory-claim"}
    assert all(p["revision"] == edge["structural_ref"]["revision"] for p in bridge["edge_origins"]["declared-bridge"])
print("Counterpart CLI acceptance passed: explicit bridge/source pins, distinct manifestation state, half-open time, directed selection, typed graph reuse, joined lineage and function equivalence")
