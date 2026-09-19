#!/usr/bin/env python3
"""Pinned module CLI compared with a direct pure source implementation on the real engine."""
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
fixture = root / "examples/modules"
engine = str(args.engine.resolve())
with tempfile.TemporaryDirectory(prefix="weave-modules-") as directory:
    work = Path(directory)
    def compile_file(path, mapped=False):
        command = ["cargo", "run", "--locked", "--quiet", "--", "plan", str(path)]
        if mapped:
            command += ["--modules", str(fixture / "map.json")]
        return json.loads(subprocess.check_output(command, cwd=root))
    linked = compile_file(fixture / "main.weave", True)
    # Explicit oracle inlines the same pure definitions. Its nominal schema ID is
    # local; align that one ID with the linked namespace before comparing execution.
    text = "\n".join(line for name in ["core.weave", "views.weave", "main.weave"]
                     for line in (fixture/name).read_text().splitlines()
                     if not line.startswith(("module ", "import ")))
    text = text.replace("core::", "").replace("views::", "")
    direct_path = work / "direct.weave"
    direct_path.write_text(text)
    direct = compile_file(direct_path)
    direct["commands"][0]["data"]["schema"]["id"] = "module:example.core:schema:Network"
    assert direct["commands"] == linked["commands"], "Imports must add no runtime reads or effects"
    assert len([r for r in linked["source_revisions"] if r["name"] == "module:example.core"]) == 1
    def run(plan, name):
        path = work / f"{name}.json"
        path.write_text(json.dumps(plan))
        output = subprocess.run([engine, "run", "--db", str(work/name), "--actor", "reader",
            "--write", "Network", str(path)], text=True, capture_output=True)
        assert output.returncode == 0, output.stderr
        return json.loads(output.stdout)
    actual, oracle = run(linked, "linked"), run(direct, "direct")
    for left, right in zip(actual, oracle):
        left, right = copy.deepcopy(left), copy.deepcopy(right)
        if left["kind"] == "queried":
            left["result"].pop("source_revisions", None)
            right["result"].pop("source_revisions", None)
        assert left == right
    empty = actual[-1]["result"]["graph"]
    assert empty["nodes"] == empty["edges"] == []
    assert empty["schema"]["id"] == "module:example.core:schema:Network"
print("Pinned module CLI acceptance passed: transitive capture, diamond deduplication, exact nominal schema, direct-operation equivalence and empty typed output")
