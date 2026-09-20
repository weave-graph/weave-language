#!/usr/bin/env python3
"""Actual source Interval specialization and existing engine temporal semantics."""
import argparse
import json
from pathlib import Path
import subprocess
import tempfile

p = argparse.ArgumentParser()
p.add_argument('--compiler', type=Path, required=True)
p.add_argument('--engine', type=Path, required=True)
a = p.parse_args()
compiler, engine = str(a.compiler.resolve()), str(a.engine.resolve())
source = Path(__file__).resolve().parents[1] / 'examples/intervals.weave'
plan = json.loads(subprocess.check_output([compiler, 'plan', str(source)]))
values = json.loads(subprocess.check_output([compiler, 'values', str(source)]))['values']
assert plan['version'] == '0.18.0'
assert values['Window'] == {'type':'interval','value':{'start':15,'end':20}}
assert values['Start'] == {'type':'time','value':15}
assert values['End'] == {'type':'time','value':20}
assert values['Inside']['value'] and values['Meets']['value']
assert not values['Outside']['value'] and not values['Before']['value']
with tempfile.TemporaryDirectory(prefix='weave-intervals-') as directory:
    work = Path(directory)
    def run(program, name):
        path = work / f'{name}.json'
        path.write_text(json.dumps(program))
        return json.loads(subprocess.check_output([engine,'run','--db',str(work/'store.db'),'--actor','reader','--write','Windows',str(path)]))
    outputs = run(plan, 'seed')
    bound = {command['name']: result['result'] for command,result in zip(plan['commands'],outputs) if command['op']=='bind'}
    assert len(bound['AtStart']['graph']['edges']) == 1
    assert bound['AtEnd']['graph']['edges'] == []
    graph = bound['WindowMetadata']['graph']
    assert graph['nodes'][0]['properties']['window'] == values['Window']['value']
    assert graph['attachments'][0]['value']['value'] == values['Window']['value']
    revision = outputs[0]['revision']
    query = {'version':plan['version'],'commands':[{'op':'query','query':{'graph_id':'Windows','revision':revision,'valid_at':15,'include_metadata':True,'max_depth':1}}]}
    assert run(query,'restart')[0]['result']['graph'] == graph
    query['commands'][0]['query']['valid_at'] = 20
    excluded = run(query,'end')[0]['result']['graph']
    assert excluded['edges'] == [] and excluded.get('attachments', []) == []
    bad = work/'bad.weave'
    bad.write_text('value Invalid interval_intersection(interval(time 0,time 1),interval(time 1,time 2)); graph Windows {}')
    failed = subprocess.run([compiler,'plan',str(bad)],capture_output=True,text=True)
    assert failed.returncode and not failed.stdout and 'E_INTERVAL_EMPTY' in failed.stderr
    query['commands'][0]['query']['valid_at'] = 15
    assert run(query,'after-error')[0]['result']['graph'] == graph
print('Interval CLI acceptance passed: typed pure specialization, half-open valid-at/metadata filtering, canonical literal persistence, fresh-process exact revision replay, failed specialization emits no plan')
