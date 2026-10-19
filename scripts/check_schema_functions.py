#!/usr/bin/env python3
"""Actual annotated/unconstrained compiler and runtime schema-preserving equivalence."""
import argparse
import copy
import json
from pathlib import Path
import subprocess
import tempfile

parser = argparse.ArgumentParser()
parser.add_argument("--engine", type=Path, required=True)
parser.add_argument("--compiler", type=Path)
args = parser.parse_args()
root = Path(__file__).resolve().parents[1]
source = (root / "examples/schema_functions.weave").read_text()
# Results of 0.22.0+ programs cite engine-recorded fact provenance. Two
# independent stores record different times, so equivalence ignores only
# `recorded_at_ms`, after checking that the engine recorded one.
def without_recorded_time(output):
    if isinstance(output.get("result"), dict):
        for entry in output["result"].get("fact_provenance", []):
            assert isinstance(entry["provenance"].pop("recorded_at_ms"), int), entry
    return output
with tempfile.TemporaryDirectory(prefix="weave-schema-functions-") as directory:
    work = Path(directory)
    def compile_source(text, name):
        path = work / f"{name}.weave"
        path.write_text(text)
        command = [str(args.compiler.resolve()), "plan", str(path)] if args.compiler else ["cargo", "run", "--locked", "--quiet", "--", "plan", str(path)]
        return json.loads(subprocess.check_output(command, cwd=root))
    def run(plan, name, database=None):
        path = work / f"{name}.json"
        path.write_text(json.dumps(plan))
        result = subprocess.run([str(args.engine.resolve()), "run", "--db", str(work / (database or name)),
            "--actor", "reader", "--write", "Network", "--write", "Saved", str(path)], capture_output=True, text=True)
        assert result.returncode == 0, result.stderr
        return json.loads(result.stdout)
    annotated = compile_source(source, "annotated")
    plain = compile_source(source.replace("graph input schema Infrastructure", "graph input").replace("returns graph schema Infrastructure", ""), "plain")
    assert annotated["version"] == plain["version"] == json.loads((Path(__file__).resolve().parents[1] / "vendor/manifest.json").read_text())["contract_version"]
    assert annotated["commands"] == plain["commands"], "Static annotations must not introduce reads or effects"
    actual = run(annotated, "annotated")
    oracle = run(plain, "plain")
    values = {c["name"]: r["result"] for c, r in zip(annotated["commands"], actual) if c["op"] == "bind"}
    for a, b in zip(actual, oracle):
        a, b = copy.deepcopy(a), copy.deepcopy(b)
        if a["kind"] == "queried":
            a["result"].pop("source_revisions", None)
            b["result"].pop("source_revisions", None)
        assert without_recorded_time(a) == without_recorded_time(b)
    schema = annotated["commands"][0]["commits"][0]["data"]["schema"]
    reference = values["Reference"]
    for name in ["HigherOrder", "Direct", "ThroughCapture"]:
        assert values[name]["graph"] == reference["graph"]
        assert values[name]["graph"]["schema"] == schema
        assert values[name]["provenance"] == reference["provenance"]
        assert values[name]["input_snapshots"] == reference["input_snapshots"]
    empty = values["Empty"]["graph"]
    assert empty["nodes"] == empty["edges"] == [] and empty["schema"] == schema
    saved = run({"version": annotated["version"], "commands": [
        {"op": "commit", "graph_id": "Saved", "data": empty},
        {"op": "query", "query": {"graph_id": "Saved"}}]}, "save-empty", "annotated")
    assert saved[1]["result"]["graph"]["schema"] == schema
print("Schema-function CLI acceptance passed: exact descriptor preservation, higher-order/partial/captured equivalence, no annotation-induced effects and empty-result persistence")
