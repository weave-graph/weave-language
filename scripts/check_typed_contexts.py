#!/usr/bin/env python3
"""Actual source/runtime typed-context closure, private empty values and rollback."""
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
plan = json.loads(subprocess.check_output(
    ["cargo", "run", "--locked", "--quiet", "--", "plan", "examples/typed_contexts.weave"], cwd=root))
assert tuple(map(int, plan["version"].split("."))) >= (0, 14, 0)
pin = {"graph_id": "Estonia", "revision": "logical:typed_worlds:Estonia"}
# Trusted test setup restricts the descriptor; source declarations themselves do not mint authority.
descriptor = plan["commands"][0]["commits"][0]["data"]
assert descriptor["assertions"][0]["properties"]["weave.context"]["values"]["load"] == "0.75"
assert descriptor["assertions"][0]["source"] == "scenario-author"
descriptor["assertions"][0]["readers"] = ["reader"]
descriptor["nodes"][0]["readers"] = ["reader"]
with tempfile.TemporaryDirectory(prefix="weave-typed-contexts-") as directory:
    work = Path(directory)
    serial = 0
    def run(commands, actor="reader", version=None):
        global serial
        serial += 1
        path = work / f"plan-{serial}.json"
        path.write_text(json.dumps({"version": version or plan["version"], "commands": commands}))
        return subprocess.run([str(args.engine.resolve()), "run", "--db", str(work / "db"),
            "--actor", actor, "--write", "Estonia", "--write", "Claims", "--write", "Saved",
            "--write", "Empty", "--write", "Marker", str(path)], capture_output=True, text=True)
    initial = run(plan["commands"])
    assert initial.returncode == 0, initial.stderr
    outputs = json.loads(initial.stdout)
    values = {c["name"]: r["result"] for c, r in zip(plan["commands"], outputs) if c["op"] == "bind"}
    assert [e["id"] for e in values["Selected"]["graph"]["edges"]] == ["yes"]
    for name, state in [("Known", "supported"), ("Unknown", "unknown")]:
        assert values[name]["graph"]["nodes"][0]["properties"]["state"] == state
    assert values["Selected"]["graph"] == values["Reused"]["graph"]
    for name in ["Selected", "Known", "Unknown", "Proof", "Reused", "Repeated"]:
        value = values[name]
        typing = value["graph"]["context_typing"]
        assert value["selected_context"] == {"kind": "pinned", "reference": pin}
        assert typing["selected"] == pin and len(typing["witnesses"]) == 1
        witness = typing["witnesses"][0]
        assert witness["definition"] == dict(pin, assertion_id="definition")
        assert witness["anchor_nodes"] == [dict(pin, node_id="context")]
        assert set(witness["schema"]["axes"]) == {"region", "scenario", "load", "simulated"}
    saved = copy.deepcopy(values["Unknown"]["graph"])
    for node in saved["nodes"]:
        node["readers"] = []
    empty = copy.deepcopy(values["Selected"]["graph"])
    empty.update(nodes=[], edges=[], attachments=[])
    committed = run([{"op": "commit", "graph_id": name, "data": data}
                     for name, data in [("Saved", saved), ("Empty", empty)]])
    assert committed.returncode == 0, committed.stderr
    for name in ["Saved", "Empty"]:
        query = [{"op": "query", "query": {"graph_id": name}}]
        allowed = run(query)
        assert allowed.returncode == 0, allowed.stderr
        result = json.loads(allowed.stdout)[0]["result"]
        assert result["graph"]["context_typing"]["selected"] == pin
        denied = run(query, actor="other")
        assert denied.returncode == 0, denied.stderr
        withheld = json.loads(denied.stdout)[0]["result"]
        assert withheld["coverage"] == "partial"
        assert withheld["graph"]["nodes"] == withheld["graph"]["edges"] == []
        assert not withheld["graph"].get("context_typing") and not withheld["graph"].get("schema")
        assert "OperatingWorld" not in denied.stdout and "Estonia" not in denied.stdout
    selection = copy.deepcopy(plan["commands"][1])
    marker = {"op": "commit", "graph_id": "Marker", "data": {"nodes": [], "edges": []}}
    rejected = run([marker, selection], actor="other")
    assert rejected.returncode != 0 and "E_CONTEXT_UNAVAILABLE" in rejected.stderr, rejected.stderr
    wrong = copy.deepcopy(selection)
    wrong["value"]["expected_schema"]["axes"]["load"] = {"kind": "string"}
    rejected = run([marker, wrong])
    assert rejected.returncode != 0 and "E_CONTEXT_UNAVAILABLE" in rejected.stderr, rejected.stderr
    rejected = run([marker, selection], version="0.13.0")
    assert rejected.returncode != 0 and "E_VERSION" in rejected.stderr, rejected.stderr
    rejected = run([marker, {"op": "commit", "graph_id": "Saved", "data": saved}], version="0.13.0")
    assert rejected.returncode != 0 and "E_VERSION" in rejected.stderr, rejected.stderr
    # All failed programs must leave this initial compare-and-swap commit possible.
    probe = run([marker])
    assert probe.returncode == 0, probe.stderr
print("Typed-context CLI acceptance passed: canonical axes, exact descriptor witnesses, reusable support/explanation, private saved and empty values, old-profile rejection and atomic rollback")
