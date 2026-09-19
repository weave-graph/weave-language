#!/usr/bin/env python3
"""Run the composed-value example against a separately built Weave Engine CLI."""
import argparse
import json
from pathlib import Path
import shutil
import subprocess
import tempfile

parser = argparse.ArgumentParser()
parser.add_argument("--engine", default="weave-engine", help="Engine CLI executable or path")
args = parser.parse_args()
engine = shutil.which(args.engine)
if engine is None:
    raise SystemExit("Build Weave Engine and pass its executable with --engine")
root = Path(__file__).resolve().parents[1]
plan = subprocess.check_output(
    ["cargo", "run", "--quiet", "--locked", "--", "plan", "examples/composed.weave"], cwd=root
)
with tempfile.TemporaryDirectory(prefix="weave-composition-") as directory:
    work = Path(directory)
    (work / "plan.json").write_bytes(plan)
    result = json.loads(subprocess.check_output([
        engine, "run", "--db", str(work / "engine.db"), "--actor", "demo",
        "--write", "Operations", "--write", "Advisories", "--write", "Guidance",
        str(work / "plan.json"),
    ]))
    assert sum(row["kind"] == "committed" for row in result) == 3, "Derived values must not commit"
    final = result[-1]["result"]
    assert len(final["graph"]["edges"]) == 1
    edge = final["graph"]["edges"][0]
    assert edge["predicate"] == "needs_fix"
    assert edge["valid_time"] == {"start": 170, "end": 200}
    premises = final["edge_origins"][edge["id"]]
    assert {ref["assertion_id"] for ref in premises} == {"uses-model", "affected-model", "repair"}
    assert len(final["input_snapshots"]) == 3
    assert all(ref["revision"] for ref in premises)
    assert final["coverage"] == "complete"
    print("PASS: composed values preserve three leaf premises, time overlap, and snapshots without hidden commits")
