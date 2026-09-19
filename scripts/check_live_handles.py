#!/usr/bin/env python3
"""Compiler/runtime live resolution, per-execution pins, time, visibility and CAS."""
import argparse
import json
from pathlib import Path
import subprocess
import tempfile

parser = argparse.ArgumentParser()
parser.add_argument("--engine", type=Path, required=True)
args = parser.parse_args()
root = Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory(prefix="weave-live-handles-") as directory:
    work = Path(directory)
    sequence = 0
    def compile_source(source):
        source_path = work / "source.weave"
        source_path.write_text(source)
        return json.loads(subprocess.check_output(["cargo", "run", "--locked", "--quiet", "--", "plan", str(source_path)], cwd=root))
    def run(plan, actor="reader"):
        global sequence
        sequence += 1
        path = work / f"plan-{sequence}.json"
        path.write_text(json.dumps(plan))
        return subprocess.run([str(args.engine.resolve()), "run", "--db", str(work / "db"), "--actor", actor,
            "--write", "Evidence", "--write", "Catalog", "--write", "Marker", str(path)], capture_output=True, text=True)
    def success(plan, actor="reader"):
        result = run(plan, actor)
        assert result.returncode == 0, result.stderr
        output = json.loads(result.stdout)
        return output, {c["name"]: r["result"] for c, r in zip(plan["commands"], output) if c["op"] == "bind"}
    def edges(value):
        return [e["id"] for e in value["graph"]["edges"]]
    def pin(value, key):
        return next(a["value"]["reference"]["revision"] for a in value["graph"]["attachments"] if a["key"] == key)
    seed = compile_source((root / "examples/live_handles.weave").read_text())
    _, initial = success(seed)
    assert edges(initial["Fixed"]) == edges(initial["Following"]) == ["old"]
    assert initial["Snapshot"]["graph"] == initial["Reused"]["graph"]
    old = "logical:live_seed:Evidence"
    header = 'live_handle H graph "Catalog" branch "main";\n'
    read = header + '''pin Snapshot from H at 5 metadata depth 2;
metadata Fixed from Snapshot on node "asset" key "fixed";
metadata Following from Snapshot on node "asset" key "following";'''
    replacement = '''graph Evidence replace revision "logical:live_seed:Evidence" {
node "a" entity "A" space "operations";
node "b" entity "B" space "operations";
edge "new" from "a" to "b" relation "observed" valid 0 until 10;
}'''
    ordered = compile_source(header + 'pin Before from H at 5 metadata depth 2;\n' + replacement + '''
pin After from H at 5 metadata depth 2;
metadata BeforeEvidence from Before on node "asset" key "following";
metadata AfterEvidence from After on node "asset" key "following";
metadata StillFixed from After on node "asset" key "fixed";''')
    output, changed = success(ordered)
    revision = next(r["revision"] for c, r in zip(ordered["commands"], output) if c["op"] == "commit")
    assert pin(changed["Before"], "following") == old
    assert pin(changed["After"], "following") == revision
    assert pin(changed["After"], "fixed") == old
    assert edges(changed["BeforeEvidence"]) == edges(changed["StillFixed"]) == ["old"]
    assert edges(changed["AfterEvidence"]) == ["new"]
    # A separately executed identical read repins; the first in-program result remains unchanged.
    _, replay = success(compile_source(read))
    assert edges(replay["Following"]) == ["new"] and edges(replay["Fixed"]) == ["old"]
    assert pin(initial["Snapshot"], "following") == old
    # Full snapshot replacement explicitly changes the fixed attachment, preserving the old Catalog revision.
    rebind = compile_source('''graph Catalog replace revision "logical:live_seed:Catalog" {
node "asset" entity "asset" space "operations";
edge "present" from "asset" to "asset" relation "catalog" valid 0 until infinity;
attachment "fixed" on node "asset" key "fixed" graph "Evidence" revision ''' + json.dumps(revision) + ''' valid 0 until 20;
attachment "following" on node "asset" key "following" live graph "Evidence" branch "main" valid 0 until 20;
}''')
    success(rebind)
    _, rebound = success(compile_source(read))
    assert edges(rebound["Fixed"]) == ["new"]
    historical = compile_source('''use Old graph "Catalog" revision "logical:live_seed:Catalog";
lens Snapshot from Old { at 5; metadata depth 2; }
metadata Fixed from Snapshot on node "asset" key "fixed";''')
    _, past = success(historical)
    assert edges(past["Fixed"]) == ["old"]
    # Trusted test setup restricts the next snapshot, without adding source permission syntax.
    private = compile_source(replacement.replace(old, revision).replace('"new"', '"private"'))
    private["commands"][0]["data"]["edges"][0]["readers"] = ["reader"]
    success(private)
    read_plan = compile_source(read)
    _, allowed = success(read_plan)
    _, denied = success(read_plan, "outsider")
    assert edges(allowed["Following"]) == ["private"]
    assert edges(denied["Following"]) == [], denied["Following"]
    assert edges(denied["Fixed"]) == ["new"]
    # Explicit query time controls half-open edge and attachment intervals; no wall clock is read.
    _, edge_expiry = success(compile_source(read.replace('at 5', 'at 10') + '\nlens Active from Following { at 10; }'))
    assert edges(edge_expiry["Active"]) == []
    _, attachment_expiry = success(compile_source(read.replace('at 5', 'at 20')))
    assert attachment_expiry["Snapshot"]["graph"].get("attachments", []) == []
    assert attachment_expiry["Following"]["coverage"] == "partial"
    # A missing top-level head fails generically, never a complete empty answer.
    missing = compile_source('live_handle H graph "Unavailable" branch "main"; pin Missing from H;')
    unavailable = run(missing)
    assert unavailable.returncode != 0 and "E_UNAVAILABLE" in unavailable.stderr, unavailable.stderr
    marker = compile_source('graph Marker {}')["commands"][0]
    stale = compile_source(replacement)
    stale["commands"].insert(0, marker)
    result = run(stale)
    assert result.returncode != 0 and "E_CONFLICT" in result.stderr, result.stderr
    success({"version": seed["version"], "commands": [marker]})
print("Live-handle CLI acceptance passed: ordered per-execution pins, live versus historical metadata, explicit full-snapshot rebind, current visibility, half-open time, unavailable heads and stale-CAS rollback")
