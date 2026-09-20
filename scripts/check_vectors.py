#!/usr/bin/env python3
"""Typed compiler vectors reuse existing native geometry payloads and authority checks."""
import argparse
import copy
import json
from pathlib import Path
import subprocess
import tempfile
p=argparse.ArgumentParser()
p.add_argument('--compiler',type=Path,required=True)
p.add_argument('--engine',type=Path,required=True)
a=p.parse_args()
compiler,engine=str(a.compiler.resolve()),str(a.engine.resolve())
root=Path(__file__).resolve().parents[1]
def compile_(name,mode='plan'):
    return json.loads(subprocess.check_output([compiler,mode,str(root/'examples'/name)]))
plan=compile_('vectors.weave')
assert plan==compile_('geometry.weave'),'typed source values changed native geometry plan semantics'
values=compile_('vectors.weave','values')['values']
assert len(values)==5 and all(v['type']=='vector' for v in values.values())
assert values['CoordinateB']['value']['values']==[3.0,4.0,0.0]
with tempfile.TemporaryDirectory(prefix='weave-vectors-') as directory:
    work=Path(directory)
    def run(program,name):
        path=work/f'{name}.json';path.write_text(json.dumps(program))
        return subprocess.run([engine,'run','--db',str(work/f'{name}.db'),'--actor','reader','--write','Geometry',str(path)],capture_output=True,text=True)
    actual=run(plan,'vectors');assert actual.returncode==0,actual.stderr
    outputs=json.loads(actual.stdout)
    bound={c['name']:r['result'] for c,r in zip(plan['commands'],outputs) if c['op']=='bind'}
    def payload(name):return bound[name]['graph']['edges'][0]['assertion_properties']['weave.geometry']
    assert payload('Range')['value']==5.0 and payload('Range')['unit']=='metre'
    assert bound['Range']['graph']['edges'][0]['valid_time']=={'start':5,'end':15}
    assert payload('Moved')['values']==[100.0,0.0,0.0]
    assert payload('TargetRange')['value']==500.0
    assert payload('Similarity')['value']==1.0
    assert payload('Navigation')['kind']=='navigation_projection'
    assert bound['Range']['graph']==bound['ThroughFunction']['graph']
    cases=[]
    hidden=copy.deepcopy(plan)
    hidden['commands'][0]['data']['assertions'][0]['readers']=['other-principal']
    cases.append((hidden,'hidden','E_GEOMETRY_UNAVAILABLE'))
    expired=copy.deepcopy(plan)
    for command in expired['commands']:
        if command.get('name')=='Range':command['value']['valid_at']=20
    cases.append((expired,'expired','E_GEOMETRY_UNAVAILABLE'))
    mismatch=copy.deepcopy(plan)
    for claim in mismatch['commands'][0]['data']['assertions']:
        if claim['id']=='embedding-b':claim['properties']['weave.geometry']['space']['geometry']['encoder']='different-encoder'
    cases.append((mismatch,'descriptor','E_INCOMPATIBLE_SPACE'))
    projection=copy.deepcopy(plan)
    projection['commands'].append({'op':'bind','name':'InvalidMetric','value':{'kind':'geometry','operation':{'kind':'distance','left':{'input':{'kind':'reference','name':'Navigation'},'assertion_id':'value'},'right':{'input':{'kind':'reference','name':'Navigation'},'assertion_id':'value'}},'valid_at':7}})
    cases.append((projection,'projection','E_GEOMETRY_KIND'))
    for candidate,name,code in cases:
        failed=run(candidate,name)
        assert failed.returncode and code in failed.stderr,(name,failed.stderr)
print('Vector CLI acceptance passed: identical native geometry plan, typed physical/embedding values, distance/transform reuse/cosine, current visibility/time, exact descriptor mismatch and display-only projection rejection')
