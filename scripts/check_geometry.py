#!/usr/bin/env python3
"""Actual compiler/runtime graph-valued geometry and explanation acceptance."""
import argparse
import copy
import json
from pathlib import Path
import subprocess
import tempfile

parser = argparse.ArgumentParser()
parser.add_argument("--engine", type=Path, required=True)
args = parser.parse_args()
root = Path(__file__).resolve().parents[1]
plan = json.loads(subprocess.check_output(["cargo", "run", "--locked", "--quiet", "--", "plan", "examples/geometry.weave"], cwd=root))
with tempfile.TemporaryDirectory(prefix="weave-geometry-") as directory:
    work = Path(directory)
    def run(candidate, name):
        path = work / f"{name}.json"
        path.write_text(json.dumps(candidate))
        return subprocess.run([str(args.engine.resolve()), "run", "--db", str(work / name), "--actor", "reader", "--write", "Geometry", str(path)], capture_output=True, text=True)
    actual = run(plan, "good")
    assert actual.returncode == 0, actual.stderr
    outputs = json.loads(actual.stdout)
    values = {c["name"]: r["result"] for c, r in zip(plan["commands"], outputs) if c["op"] == "bind"}
    def payload(name):
        return values[name]["graph"]["edges"][0]["assertion_properties"]["weave.geometry"]
    assert payload("Range")["value"] == 5.0 and payload("Range")["unit"] == "metre"
    assert values["Range"]["graph"]["edges"][0]["valid_time"] == {"start": 5, "end": 15}
    assert values["Range"]["graph"] == values["ThroughFunction"]["graph"]
    assert payload("Moved")["values"] == [100.0, 0.0, 0.0]
    assert payload("TargetRange")["value"] == 500.0 and payload("TargetRange")["unit"] == "centimetre"
    assert {p["assertion_id"] for p in values["TargetRange"]["provenance"]} == {"coordinate-a", "mapping", "target-position"}
    assert payload("Similarity")["value"] == 1.0
    assert payload("Navigation")["kind"] == "navigation_projection" and payload("Navigation")["approximate"]
    assert payload("Navigation")["projection_revision"] == "axis-view-1"
    assert values["Proof"]["graph"]["edges"] and values["RepeatedProof"]["graph"]["nodes"]
    assert all(node.get("context_scope") == {"kind": "default"} for node in values["Proof"]["graph"]["nodes"])
    bad = copy.deepcopy(plan)
    bad["commands"].append({"op": "bind", "name": "InvalidMetric", "value": {"kind": "geometry", "operation": {"kind": "distance", "left": {"input": {"kind": "reference", "name": "Navigation"}, "assertion_id": "value"}, "right": {"input": {"kind": "reference", "name": "Navigation"}, "assertion_id": "value"}}, "valid_at": 7}})
    failure = run(bad, "projection-as-distance")
    assert failure.returncode != 0 and "E_GEOMETRY_KIND" in failure.stderr
print("Geometry CLI acceptance passed: physical/embedding distance, explicit frame transform, reusable result lineage, display-only projection rejection, graph-valued explanation and function equivalence")
