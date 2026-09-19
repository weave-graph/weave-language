#!/usr/bin/env python3
"""Compare higher-order, directly specialized and literal graph evaluation."""
import argparse
import json
from pathlib import Path
import subprocess
import tempfile

parser = argparse.ArgumentParser()
parser.add_argument("--engine", type=Path, required=True)
args = parser.parse_args()
root = Path(__file__).resolve().parents[1]
plan = json.loads(subprocess.check_output(["cargo", "run", "--locked", "--quiet", "--", "plan", "examples/functions.weave"], cwd=root))
with tempfile.TemporaryDirectory(prefix="weave-functions-") as directory:
    work = Path(directory)
    path = work / "plan.json"
    path.write_text(json.dumps(plan))
    result = json.loads(subprocess.check_output([str(args.engine.resolve()), "run", "--db", str(work / "db"), "--actor", "reader", "--write", "Evidence", str(path)]))
    values = {c["name"]: r["result"] for c, r in zip(plan["commands"], result) if c["op"] == "bind"}
    direct, higher, literal = [values[n] for n in ("Direct", "HigherOrder", "Reference")]
    assert direct == higher == literal
    assert len(higher["graph"]["edges"]) == 1
    assert higher["graph"]["edges"][0]["id"] == "claim"
    assert sum(c["op"] == "commit" for c in plan["commands"]) == 1
    assert len(higher["input_snapshots"]) == 1
print("Function CLI acceptance passed: exact result equality across literal, partial, and higher-order evaluation; one explicit source commit")
