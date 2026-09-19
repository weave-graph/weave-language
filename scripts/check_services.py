#!/usr/bin/env python3
"""Actual source plans use native identity/clustering; authority stays in a trusted fixture."""
import argparse
import copy
import json
from pathlib import Path
import subprocess
import tempfile

parser=argparse.ArgumentParser()
parser.add_argument("--engine",type=Path,required=True)
parser.add_argument("--fixture",type=Path,required=True,help="trusted native_services_fixture example executable")
args=parser.parse_args()
root=Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory(prefix="weave-services-") as directory:
    work=Path(directory)
    def fixture(database,operation):
        return json.loads(subprocess.check_output([str(args.fixture.resolve()),str(work/database),operation]))
    def compile_text(text,name):
        path=work/f"{name}.weave";path.write_text(text)
        plan=json.loads(subprocess.check_output(["cargo","run","--locked","--quiet","--","plan",str(path)],cwd=root))
        assert plan["version"]=="0.13.0"
        return plan
    def run(plan,name,database):
        path=work/f"{name}.json";path.write_text(json.dumps(plan))
        return subprocess.run([str(args.engine.resolve()),"run","--db",str(work/database),"--actor","reader","--write","Network","--write","Marker",str(path)],capture_output=True,text=True)
    def results(plan,process):
        assert process.returncode==0,process.stderr
        return {c["name"]:r["result"] for c,r in zip(plan["commands"],json.loads(process.stdout)) if c["op"]=="bind"}
    seed=fixture("identity","seed")
    source=(root/"examples/accepted_identity.weave").read_text().replace("SOURCE_REVISION",seed["selection"]["source"]["revision"]).replace("MAPPING_REVISION",seed["selection"]["revision"])
    plan=compile_text(source,"identity")
    values=results(plan,run(plan,"identity-plan","identity"))
    assert values["Accepted"]["graph"]==seed["direct"]["graph"]
    assert len(values["Accepted"]["graph"]["edges"])==1
    assert {n["entity_id"] for n in values["Accepted"]["graph"]["nodes"]}=={"independent-A","independent-B"}
    assert values["Accepted"]["graph"]==values["ThroughFunction"]["graph"]
    assert values["Repeated"]["graph"]==values["Nested"]["graph"]
    assert fixture("identity","events")["events"]==seed["events"]
    marker={"op":"commit","graph_id":"Marker","data":{"nodes":[],"edges":[]}}
    fixture("identity","revoke")
    revoked=copy.deepcopy(plan);revoked["commands"].insert(0,marker)
    failure=run(revoked,"revoked","identity")
    assert failure.returncode!=0 and "E_IDENTITY_UNAVAILABLE" in failure.stderr,failure.stderr
    assert fixture("identity","events")["events"]==seed["events"]
    probe=run({"version":plan["version"],"commands":[marker]},"rollback-probe","identity")
    assert probe.returncode==0,probe.stderr
    cluster=compile_text((root/"examples/cluster_navigation.weave").read_text(),"cluster")
    values=results(cluster,run(cluster,"cluster-plan","cluster"))
    assert values["Navigation"]["coverage"]=="partial"
    assert values["Navigation"]["graph"]==values["ThroughFunction"]["graph"]
    assert values["Repeated"]["graph"]==values["Nested"]["graph"]
    assert not values["NoChange"]["graph"].get("attachments", [])
    assert any(n["properties"].get("kind")=="cluster" for n in values["Navigation"]["graph"]["nodes"])
    assert any(n["properties"].get("source_node",[{}])[0].get("node_id")=="isolated" for n in values["Navigation"]["graph"]["nodes"])
    assert fixture("cluster","events")["events"]==1, "only the explicit source transaction may publish"
    for name,template in [("identity",plan),("cluster",cluster)]:
        old=copy.deepcopy(template);old["version"]="0.12.0";old["commands"].insert(0,marker)
        database=f"old-{name}";failure=run(old,database,database)
        assert failure.returncode!=0 and "E_VERSION" in failure.stderr,failure.stderr
        assert run({"version":plan["version"],"commands":[marker]},f"probe-{name}",database).returncode==0
print("Native service CLI acceptance passed: explicit accepted identity pins, independent IDs, current revocation, reusable typed/navigation results, exact cluster snapshot, no hidden events and atomic old-profile rejection")
