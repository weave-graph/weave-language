#!/usr/bin/env python3
"""Actual compiler→engine windows/sequence, pinned replay, metadata and proof gates."""
import argparse, copy, json, subprocess, tempfile
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--compiler',type=Path,required=True);p.add_argument('--engine',type=Path,required=True);p.add_argument('--report',type=Path);a=p.parse_args()
compiler,engine=str(a.compiler.resolve()),str(a.engine.resolve())
root=Path(__file__).resolve().parents[1];processes=0
with tempfile.TemporaryDirectory(prefix='weave-temporal-') as directory:
 work=Path(directory);db=work/'store.db';serial=0
 def compile_source(text):
  global serial,processes
  serial+=1;path=work/f'source-{serial}.weave';path.write_bytes(text.encode('utf-8'));processes+=1
  return json.loads(subprocess.check_output([compiler,'plan',str(path)]))
 def execute(plan,actor='reader',ok=True):
  global serial,processes
  serial+=1;path=work/f'plan-{serial}.json';path.write_bytes(json.dumps(plan,ensure_ascii=False).encode('utf-8'));processes+=1
  cmd=[engine,'run','--db',str(db),'--actor',actor]
  for graph in ['Started','Notes','EmptyNotes','Finished','Saved','Marker','Private','Detached','ScalarSaved']:cmd+=['--write',graph]
  out=subprocess.run(cmd+[str(path)],capture_output=True)
  if ok:assert out.returncode==0,out.stderr.decode('utf-8');return json.loads(out.stdout)
  assert out.returncode!=0;return out.stderr.decode('utf-8')
 def bound(plan,outputs):return {c['name']:r['result'] for c,r in zip(plan['commands'],outputs) if c['op']=='bind'}
 plan=compile_source((root/'examples/temporal.weave').read_bytes().decode('utf-8'));assert plan['version']=='0.19.0'
 outputs=execute(plan);values=bound(plan,outputs)
 revisions={c['graph_id']:r['revision'] for c,r in zip(plan['commands'],outputs) if c['op']=='commit'}
 for result in outputs:
  for receipt in result.get('commits',[]):revisions[receipt['graph_id']]=receipt['revision']
 ordered=values['Ordered'];clipped=values['Clipped']
 assert sorted((e['valid_time']['start'],e['valid_time']['end']) for e in ordered['graph']['edges'])==[(2,5),(10,13)]
 assert len(ordered['graph']['edges'])==2 and all(e['id'] not in ['begin','finish'] for e in ordered['graph']['edges'])
 assert all(e['valid_time']!={'start':2,'end':13} for e in ordered['graph']['edges'])
 assert values['Meets']['graph']['edges']==values['Overlaps']['graph']['edges']==[]
 assert clipped['graph']['edges'][0]['valid_time']=={'start':2,'end':5}
 assert values['ClippedAgain']['graph']==clipped['graph']
 assert any(n['id']=='isolated' for n in clipped['graph']['nodes'])
 assert values['Description']['graph']['nodes'] and values['Proof']['graph']['edges']
 assert values['EmptyMetadata']['graph']['nodes']==[] and values['EmptyMetadata']['graph']['influence']['derivations']
 assert values['EmptyState']['graph']['nodes'][0]['properties']['state']=='unknown'
 assert values['Empty']['graph']['edges']==[]
 assert len(values['Reused']['graph']['edges'])==2
 # Select returned wrapper ID explicitly; never infer aliases from proof refs.
 node_attachment=next(m for m in ordered['graph']['attachments'] if m['host']['kind']=='node')
 pinned='use L graph "Started" revision '+json.dumps(revisions['Started'])+'; use R graph "Finished" revision '+json.dumps(revisions['Finished'])+'; lens Loaded from L {metadata depth 2;} sequence S from Loaded to R before during interval(time 2,time 13);'
 navigation=compile_source(pinned+' metadata Note from S on node '+json.dumps(node_attachment['host']['id'])+' key "node-note";')
 nav=bound(navigation,execute(navigation));assert nav['Note']['graph']['nodes']
 # Exact fresh-process evaluation of the same pins remains stable.
 replay=compile_source(pinned);fresh=bound(replay,execute(replay))['S'];assert fresh['graph']==ordered['graph']
 # Source originals still retain their original interval/IDs after wrapper construction.
 original=execute({'version':'0.19.0','commands':[{'op':'query','query':{'graph_id':'Started','revision':revisions['Started']}}]})[0]['result']
 assert original['graph']['edges'][0]['id']=='begin' and original['graph']['edges'][0]['valid_time']=={'start':0,'end':5}
 # A later correction is a different exact snapshot; it does not rewrite the old observation.
 corrected=copy.deepcopy(next(c['data'] for command in plan['commands'] for c in command.get('commits',[]) if c['graph_id']=='Started'))
 corrected['edges'][0]['valid_time']={'start':12,'end':16}
 newer=execute({'version':'0.19.0','commands':[{'op':'commit','graph_id':'Started','expected_head':revisions['Started'],'data':corrected}]})[0]['revision']
 old_again=bound(replay,execute(replay))['S'];assert old_again['graph']==ordered['graph']
 revised=compile_source(pinned.replace(json.dumps(revisions['Started']),json.dumps(newer)))
 assert bound(revised,execute(revised))['S']['graph']['edges']==[]
 # Detached persistence retains conditional records after envelope/readers removal.
 detached=copy.deepcopy(ordered['graph']);detached.pop('influence',None);detached.pop('context_typing',None)
 for collection in ['nodes','edges','attachments']:
  for record in detached.get(collection,[]):record['readers']=[]
 saved=execute({'version':'0.19.0','commands':[{'op':'commit','graph_id':'Saved','data':detached}]})[0]['revision']
 persisted=execute({'version':'0.19.0','commands':[{'op':'query','query':{'graph_id':'Saved','revision':saved}}]})[0]['result']
 assert len(persisted['graph']['edges'])==2
 # Old wire rejects nested temporal syntax before an earlier write can commit.
 old={'version':'0.18.0','commands':[{'op':'commit','graph_id':'Marker','data':{'nodes':[],'edges':[]}},{'op':'bind','name':'Bad','value':{'kind':'window','input':{'kind':'query','query':{'graph_id':'Started','revision':revisions['Started']}},'window':{'start':0,'end':2}}}]}
 assert 'E_VERSION' in execute(old,ok=False)
 assert 'E_UNAVAILABLE' in execute({'version':'0.19.0','commands':[{'op':'query','query':{'graph_id':'Marker'}}]},ok=False)
 # A private paired source remains a restriction after envelope stripping.
 private=execute({'version':'0.19.0','commands':[{'op':'commit','graph_id':'Private','data':{'nodes':[{'id':'b','entity_id':'handoff','space_id':'events'},{'id':'c','entity_id':'finish','space_id':'events'}],'edges':[{'id':'secret','predicate':'phase','from':'b','to':'c','valid_time':{'start':10,'end':15},'readers':['owner']}]}}]},actor='owner')[0]['revision']
 query=compile_source('use L graph "Started" revision '+json.dumps(revisions['Started'])+'; use R graph "Private" revision '+json.dumps(private)+'; lens Loaded from L {metadata depth 2;} sequence S from Loaded to R before during interval(time 0,time 20); metadata Empty from S on graph key "empty"; support Scalar from Empty relation "absent" from entity "start" space "events" to entity "finish" space "events" at 3;')
 private_values=bound(query,execute(query,actor='owner'));private_result=private_values['S'];data=copy.deepcopy(private_result['graph']);data.pop('influence',None);data.pop('context_typing',None)
 for collection in ['nodes','edges','attachments']:
  for record in data.get(collection,[]):record['readers']=[]
 saved=execute({'version':'0.19.0','commands':[{'op':'commit','graph_id':'Detached','data':data}]},actor='owner')[0]['revision']
 denied=execute({'version':'0.19.0','commands':[{'op':'query','query':{'graph_id':'Detached','revision':saved}}]})[0]['result']
 assert denied['graph']['edges']==[] and denied['graph']['nodes']==[] and denied['graph'].get('attachments',[])==[]
 # Empty metadata selection still protects a generated scalar after detachment.
 scalar=copy.deepcopy(private_values['Scalar']['graph']);scalar.pop('influence',None);scalar.pop('context_typing',None)
 for collection in ['nodes','edges','attachments']:
  for record in scalar.get(collection,[]):record['readers']=[]
 saved=execute({'version':'0.19.0','commands':[{'op':'commit','graph_id':'ScalarSaved','data':scalar}]},actor='owner')[0]['revision']
 denied=execute({'version':'0.19.0','commands':[{'op':'query','query':{'graph_id':'ScalarSaved','revision':saved}}]})[0]['result']
 assert denied['graph']['nodes']==[] and denied['graph']['edges']==[]

report={'profile':'temporal-source-native/1','status':'passed','processes':processes,'checks':['half-open-window','window-idempotence','separate-sequence-occurrences','before-not-meets','current-wrapper-metadata','exact-pinned-replay','later-correction-separate-pin','unchanged-original','detached-persistence','old-wire-rollback','private-pair-gates','empty-metadata-detached-scalar-gates']}
if a.report:a.report.write_bytes((json.dumps(report,indent=2)+'\n').encode('utf-8'))
print(json.dumps(report))
