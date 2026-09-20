#!/usr/bin/env python3
"""Compiler scalars lower to ordinary exact graph properties; restart oracle."""
import argparse
import json
from pathlib import Path
import subprocess
import tempfile
p=argparse.ArgumentParser()
p.add_argument('--compiler',type=Path,required=True)
p.add_argument('--engine',type=Path,required=True)
a=p.parse_args()
root=Path(__file__).resolve().parents[1]
compiler=str(a.compiler.resolve())
source=root/'examples/scalars.weave'
plan=json.loads(subprocess.check_output([compiler,'plan',str(source)]))
values=json.loads(subprocess.check_output([compiler,'values',str(source)]))['values']
assert len(plan['commands'])==2 and plan['commands'][0]['op']=='commit' and plan['commands'][1]['op']=='bind'
with tempfile.TemporaryDirectory(prefix='weave-scalars-') as d:
    work=Path(d); db=work/'store.db'
    def run(program,name):
        path=work/f'{name}.json';path.write_text(json.dumps(program))
        return json.loads(subprocess.check_output([str(a.engine.resolve()),'run','--db',str(db),'--actor','reader','--write','Results',str(path)]))
    result=run(plan,'seed');graph=result[1]['result']['graph'];properties=graph['nodes'][0]['properties']
    assert properties=={'total':'0.3','count':9007199254740994,'flag':True,'label':'length ✓','length':{'amount':'2.5','unit':{'dimension_id':'length','unit_id':'metre','revision':'1'}}}
    assert properties['total']==values['Total']['value']
    assert properties['length']==values['Length']['value']
    query={'version':plan['version'],'commands':[{'op':'query','query':{'graph_id':'Results','revision':result[0]['revision']}}]}
    assert run(query,'restart')[0]['result']['graph']==graph
    bad=work/'bad.weave';bad.write_text('value Bad integer_div(3,2); graph Results {}')
    failed=subprocess.run([compiler,'plan',str(bad)],capture_output=True,text=True)
    assert failed.returncode and not failed.stdout and 'E_NUMERIC' in failed.stderr
    assert run(query,'after-error')[0]['result']['graph']==graph
print('Scalar CLI acceptance passed: exact typed values, canonical schema persistence, fresh-process pinned replay, failed specialization emits no plan')
