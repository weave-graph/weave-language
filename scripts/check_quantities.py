#!/usr/bin/env python3
"""Exact compiler/runtime numeric persistence and whole-program rejection."""
import argparse
import copy
import json
from pathlib import Path
import subprocess
import tempfile

parser=argparse.ArgumentParser()
parser.add_argument("--engine",type=Path,required=True)
args=parser.parse_args()
root=Path(__file__).resolve().parents[1]
plan=json.loads(subprocess.check_output(["cargo","run","--locked","--quiet","--","plan","examples/quantities.weave"],cwd=root))
assert tuple(map(int, plan["version"].split("."))) >= (0,12,0), "numeric plans require protocol0.12"
with tempfile.TemporaryDirectory(prefix="weave-quantities-") as directory:
    work=Path(directory)
    def run(candidate,name,database=None):
        path=work/f"{name}.json"
        path.write_text(json.dumps(candidate))
        return subprocess.run([str(args.engine.resolve()),"run","--db",str(work/(database or name)),"--actor","reader","--write","Measurements","--write","Marker",str(path)],capture_output=True,text=True)
    result=run(plan,"valid")
    assert result.returncode==0,result.stderr
    output=json.loads(result.stdout)
    revision=output[0]["revision"]
    graph=output[1]["result"]["graph"]
    values=graph["nodes"][0]["properties"]
    assert values["exact_count"]=="9007199254740993"
    assert values["computed"]==values["exact_count"]
    assert values["converted"]==values["length"]
    assert values["length"]=={"amount":"1.23","unit":{"dimension_id":"length","unit_id":"metre","revision":"1"}}
    pinned={"version":plan["version"],"commands":[{"op":"query","query":{"graph_id":"Measurements","revision":revision}}]}
    replay=run(pinned,"replay","valid")
    assert replay.returncode==0,replay.stderr
    assert json.loads(replay.stdout)[0]["result"]["graph"]==graph
    marker={"op":"commit","graph_id":"Marker","data":{"nodes":[],"edges":[]}}
    for name,mutate in [
        ("decimal-number",lambda data:data["nodes"][0]["properties"].update(exact_count=9007199254740993)),
        ("noncanonical",lambda data:data["nodes"][0]["properties"].update(exact_count="1.20")),
        ("unit-revision",lambda data:data["nodes"][0]["properties"]["length"]["unit"].update(revision="2")),
    ]:
        bad=copy.deepcopy(plan)
        mutate(bad["commands"][0]["data"])
        bad["commands"].insert(0,marker)
        failure=run(bad,name)
        assert failure.returncode!=0 and "E_SCHEMA_PROPERTY_TYPE" in failure.stderr,failure.stderr
        # Successful fresh CAS(None) proves the preceding marker commit rolled back.
        clean=run({"version":plan["version"],"commands":[marker]},f"{name}-probe",name)
        assert clean.returncode==0,clean.stderr
    for version in ["0.11.0","0.1.0"]:
        old=copy.deepcopy(plan);old["version"]=version;old["commands"].insert(0,marker)
        name=f"old-{version}";failure=run(old,name)
        assert failure.returncode!=0 and "E_VERSION" in failure.stderr,failure.stderr
        clean=run({"version":plan["version"],"commands":[marker]},f"{name}-probe",name)
        assert clean.returncode==0,clean.stderr
print("Quantity CLI acceptance passed: exact canonical persistence, restart/pinned replay, descriptor rejection, old-profile rejection and whole-program rollback")
