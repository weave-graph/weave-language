#!/usr/bin/env python3
"""Source-only Bytes/reference values persist as literal data, never graph traversal."""
import argparse,json,subprocess,tempfile
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--compiler',type=Path,required=True);p.add_argument('--engine',type=Path,required=True);a=p.parse_args()
compiler,engine=str(a.compiler.resolve()),str(a.engine.resolve())
source=Path(__file__).resolve().parents[1]/'examples/bytes_references.weave'
text=source.read_bytes().decode('utf-8')
with tempfile.TemporaryDirectory(prefix='weave-bytes-refs-') as temporary:
 root=Path(temporary);db=root/'store.db'
 def run(plan,name,actor='reader',writes=()):
  path=root/(name+'.json');path.write_bytes(json.dumps(plan,ensure_ascii=False).encode('utf-8'))
  command=[engine,'run','--db',str(db),'--actor',actor]
  for graph in writes:command+=['--write',graph]
  return json.loads(subprocess.check_output(command+[str(path)]))
 def compile_(source_text,name):
  path=root/(name+'.weave');path.write_bytes(source_text.encode('utf-8'))
  return json.loads(subprocess.check_output([compiler,'artifacts',str(path)]))['artifacts']
 def check(artifact,name):
  plan=artifact['program'];assert plan['version']=='0.18.0'
  graph=plan['commands'][0]['graph_id'];outputs=run(plan,name,writes=[graph]);revision=outputs[0]['revision']
  actual=outputs[-1]['result'];data=actual['graph']
  assert len(data['attachments'])==6 and len(data['nodes'])==1, actual
  assert actual['coverage']=='complete',actual
  pointers={x['key']:x['value'] for x in data['attachments']}
  assert all(v['kind']=='literal' for v in pointers.values())
  assert pointers['bytes']['value']=={'kind':'bytes','encoding':'hex','value':'0041ff'}
  assert pointers['node']['value']==data['nodes'][0]['properties']['pointer']
  assert pointers['node']['value']['kind']=='node_ref'
  assert pointers['edge']['value']['kind']=='edge_ref'
  assert pointers['claim']['value']['kind']=='assertion_ref'
  assert pointers['snapshot']['value']['kind']=='snapshot_ref'
  assert pointers['object']['value']['reference']['kind']=='node'
  assert {(x['graph_id'],x['revision']) for x in actual['input_snapshots']}=={(graph,revision)}
  query={'version':'0.18.0','commands':[{'op':'query','query':{'graph_id':graph,'revision':revision,'include_metadata':True,'max_depth':1}}]}
  assert run(query,name+'-restart')[0]['result']['graph']==data
  return pointers
 check(compile_(text,'missing'),'missing')
 private={'version':'0.18.0','commands':[{'op':'commit','graph_id':'Private','data':{'nodes':[{'id':'same-id','entity_id':'secret','space_id':'s','readers':['owner']}],'edges':[]}}]}
 private_revision=run(private,'private-seed','owner',['Private'])[0]['revision']
 private_text=text.replace('"unavailable"','"Private"').replace('"revision-λ"',json.dumps(private_revision)).replace('graph Values {','graph PrivatePointers {').replace('from Values {','from PrivatePointers {')
 pointers=check(compile_(private_text,'private-pointers'),'private-pointers')
 assert pointers['node']['value']['reference']=={'graph_id':'Private','revision':private_revision,'node_id':'same-id'}
 denied=run({'version':'0.18.0','commands':[{'op':'query','query':{'graph_id':'Private','revision':private_revision}}]},'not-authority')[0]['result']
 assert denied['graph']['nodes']==[]
print('Bytes/reference CLI acceptance passed: canonical hex and exact kind/revision persist; unavailable/private pointer literals add no traversal/dependency/grant; fresh-process exact replay')
