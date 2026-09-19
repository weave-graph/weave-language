#!/usr/bin/env python3
"""Run the actual language and engine CLIs against graph-algebra semantics."""
import argparse
import json
from pathlib import Path
import subprocess
import tempfile

parser = argparse.ArgumentParser()
parser.add_argument("--engine", type=Path, required=True)
args = parser.parse_args()
root = Path(__file__).resolve().parents[1]
engine = args.engine.resolve()
plan = subprocess.check_output(["cargo", "run", "--locked", "--quiet", "--", "plan", "examples/algebra.weave"], cwd=root)
with tempfile.TemporaryDirectory(prefix="weave-algebra-") as directory:
    work = Path(directory)
    path = work / "plan.json"
    path.write_bytes(plan)
    result = json.loads(subprocess.check_output([str(engine), "run", "--db", str(work / "db"), "--actor", "reader", "--write", "Positive", "--write", "Negative", str(path)]))
    assert len(result) == 11
    projected = result[2]["result"]
    assert len(projected["graph"]["nodes"]) == 2
    assert len(projected["graph"]["edges"]) == 1
    removed = result[4]["result"]
    assert removed["graph"]["edges"][0]["polarity"] == "positive"
    assert any(a["value"] == {"kind": "literal", "value": "removed"} for a in removed["graph"]["attachments"])
    assert len(result[5]["result"]["graph"]["edges"]) == 2
    assert len(result[6]["result"]["graph"]["edges"]) == 2
    assert [r["result"]["graph"]["nodes"][0]["properties"]["state"] for r in result[-4:]] == ["supported", "conflicted", "refuted", "unknown"]
    conflict = result[8]["result"]
    group = conflict["graph"]["edges"][0]["derivations"][0]
    assert {p["graph_id"] for p in group["premises"]} == {"Positive", "Negative"}
    assert group["parameters"]["valid_at"] == 15
    assert len(group["parameters"]["inputs"]) == 2
    assert all(n["readers"] == ["reader"] for r in result[2:] for n in r["result"]["graph"]["nodes"])
print("Graph algebra CLI acceptance passed: projection, polarity-preserving diff, reusable union, four temporal support states, joint provenance and principal scope")
