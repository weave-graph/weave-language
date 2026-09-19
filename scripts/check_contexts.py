#!/usr/bin/env python3
"""Execute exact contexts through compiler, metadata navigation, joins and rules."""
import argparse
import json
from pathlib import Path
import subprocess
import tempfile

parser = argparse.ArgumentParser()
parser.add_argument("--engine", type=Path, required=True)
args = parser.parse_args()
root = Path(__file__).resolve().parents[1]
plan = json.loads(subprocess.check_output(["cargo", "run", "--locked", "--quiet", "--", "plan", "examples/contexts.weave"], cwd=root))
with tempfile.TemporaryDirectory(prefix="weave-contexts-") as directory:
    work = Path(directory)
    path = work / "plan.json"
    path.write_text(json.dumps(plan))
    outputs = json.loads(subprocess.check_output([str(args.engine.resolve()), "run", "--db", str(work / "db"), "--actor", "reader", "--write", "World", "--write", "Evidence", "--write", "Claims", str(path)]))
    values = {c["name"]: r["result"] for c, r in zip(plan["commands"], outputs) if c["op"] == "bind"}
    assert [e["id"] for e in values["Default"]["graph"]["edges"]] == ["global"]
    assert [e["id"] for e in values["Scenario"]["graph"]["edges"]] == ["scoped"]
    assert values["Scenario"]["graph"] == values["SelectedFunction"]["graph"]
    assert [values[n]["graph"]["nodes"][0]["properties"]["state"] for n in ("DefaultStatus", "ScenarioStatus", "EmptyStatus")] == ["refuted", "supported", "unknown"]
    pin = {"graph_id": "World", "revision": "logical:worlds:World"}
    for name in ("Scenario", "ScenarioStatus", "Proof", "Validated", "Closure"):
        value = values[name]
        assert value["selected_context"] == {"kind": "pinned", "reference": pin}
        assert all(e.get("assertion_context") == pin for e in value["graph"]["edges"])
    assert values["EmptyStatus"]["selected_context"]["reference"]["revision"] == "not-selected"
    assert len(values["Validated"]["graph"]["edges"]) == 1
    assert {p["assertion_id"] for p in values["Validated"]["provenance"]} >= {"scoped", "proof", "review"}
    assert not values["Mixed"].get("selected_context")
    assert len(values["Mixed"]["graph"]["edges"]) == 2
    assert any(e["predicate"] == "q" for e in values["Closure"]["graph"]["edges"])
print("Context CLI acceptance passed: exact default/pinned selection, empty scope, four-valued support, contextual metadata/join/rule lineage and function equivalence")
