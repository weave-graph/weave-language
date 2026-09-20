#!/usr/bin/env python3
"""Execute arbitrary-source native Rust/native ABI/zero-import WASM SDK conformance."""
import argparse, ctypes, hashlib, json, subprocess, tempfile, time, sys
from pathlib import Path
p=argparse.ArgumentParser()
p.add_argument('--library',type=Path,default=Path('target/debug')/({'darwin':'libweave_compiler_sdk.dylib','win32':'weave_compiler_sdk.dll'}.get(sys.platform,'libweave_compiler_sdk.so')))
p.add_argument('--native',type=Path,required=True)
p.add_argument('--wasm',type=Path)
p.add_argument('--compiler',type=Path,required=True)
p.add_argument('--report',type=Path)
a=p.parse_args();root=Path(__file__).resolve().parents[1]
lib=ctypes.CDLL(str(a.library.resolve()))
def fn(name,n):
 f=getattr(lib,'weave_compiler_'+name);f.argtypes=[ctypes.c_uint32]*n;f.restype=ctypes.c_int32;return f
new,write,compile_,length,kind,read,drop=[fn(n,c) for n,c in [('input_new',1),('input_write',3),('compile',1),('output_len',1),('output_kind',1),('output_read',2),('drop',1)]]
assert fn('abi_version',0)()==1

def native_abi(data):
 t0=time.perf_counter();h=new(len(data));assert h>0,h
 for i in range(0,len(data),2):assert write(h,int.from_bytes(data[i:i+2],'little'),min(2,len(data)-i))==0
 t1=time.perf_counter();out=compile_(h);assert out>0,out;t2=time.perf_counter();assert drop(h)==-1
 try:
  size=length(out);assert size>=0;result=bytearray(size)
  for i in range(0,size,2):
   word=read(out,i);assert 0<=word<=65535
   result[i:i+2]=word.to_bytes(2,'little')[:min(2,size-i)]
  assert read(out,size)==-2 and write(out,0,1)==-1
  assert (kind(out)==0)==json.loads(result)['ok']
 finally:assert drop(out)==0
 assert drop(out)==-1
 t3=time.perf_counter()
 return bytes(result),{'transfer_ms':round(1000*(t1-t0+t3-t2),3),'compile_ms':round(1000*(t2-t1),3)}

def request(source,modules=()):return json.dumps({'format':'weave-compiler-request/1','entry_id':'sdk-fixture','source':source,'modules':list(modules)},ensure_ascii=False,separators=(',',':')).encode()
def unit(id,source):return {'id':id,'revision':'1','source':source}
def imp(alias,id,source):return f'import {alias} module "{id}" revision "1" sha256 "{hashlib.sha256(source.encode()).hexdigest()}";'
core='module "core" revision "1"; function Keep revision "1" (integer input) returns integer {return param input;}'
left='module "left" revision "1";'+imp('c','core',core)+' function L revision "1" (integer input) returns integer {apply X from c::Keep {integer input param input;} return value X;}'
right='module "right" revision "1";'+imp('c','core',core)+' function R revision "1" (integer input) returns integer {apply X from c::Keep {integer input param input;} return value X;}'
entry=imp('l','left',left)+imp('r','right',right)+'apply A from l::L {integer input 9007199254740993;} apply B from r::R {integer input value A;}'
bad='module "bad" revision "1"; function F revision "1" () returns integer {return integer_div(1,0);}'
cases=[('exact-integers',request('value Min -9223372036854775808; value Max 9223372036854775807; value Above 9007199254740993; value D decimal "0.000000000000000001"; value U "λ🚀";')),
 ('diamond-modules',request(entry,[unit('core',core),unit('left',left),unit('right',right)])),
 ('missing-module',request(imp('c','core',core))),
 ('wrong-pin',request(imp('c','core',core).replace(hashlib.sha256(core.encode()).hexdigest(),'0'*64),[unit('core',core)])),
 ('original-module-diagnostic',request(imp('b','bad',bad),[unit('bad',bad)])),
 ('bad-reference',request('value X node_ref("G","","n");')),('bad-bytes',request('value X bytes "xy";')),('all-octets',request('value B bytes "'+bytes(range(256)).hex()+'";')),( 'source-error',request('value X integer_div(1,0);')),('invalid-utf8',b'\xff'),('invalid-json',b'{'),
 ('duplicate-fields',b'{"format":"weave-compiler-request/1","format":"weave-compiler-request/1","entry_id":"x","source":"","modules":[]}'),
 ('decoded-bound',request(' '*(1048576+1))),
]
temporal_module='module "temporal" revision "1"; function During revision "1" (graph input, interval span){window W from input during param span;return W;}'
temporal_entry=imp('t','temporal',temporal_module)+'use G graph "observed" revision "exact-r"; apply W from t::During {graph input G;interval span interval_open(time -9223372036854775808);}'
cases.append(('temporal-pinned-module',request(temporal_entry,[unit('temporal',temporal_module)])))
fixtures={name:root/'examples'/f'{name}.weave' for name in ['scalars','quantities','intervals','vectors','handlers','bytes_references','temporal']}
fixtures['view-template']=root/'examples/view_services/template.weave'
b='module "b" revision "1"; import a module "a" revision "1" sha256 "'+('0'*64)+'";'
a_unit='module "a" revision "1";'+imp('b','b',b)
cases.append(('cycle',request(imp('a','a',a_unit),[unit('a',a_unit),unit('b',b)])))
cases.append(('large-output',request('value Big "'+('x'*524288)+'";')))
for name,path in fixtures.items():cases.append((name,request(path.read_bytes().decode('utf-8'))))
# Exercise near the complete decoded aggregate bound, with exact 1 MiB units.
large_source='//'+('x'*(1048576-3))+'\n'
large_units=[unit(f'm{i}',f'module "m{i}" revision "1"; //'.ljust(1048576,'x')) for i in range(3)]
cases.append(('near-4MiB-decoded-budget',request(large_source,large_units)))
node=r'''
import fs from 'node:fs'; import {pathToFileURL} from 'node:url';
const [wasm,adapter,dir]=process.argv.slice(1);
const {createCompiler}=await import(pathToFileURL(adapter));
const module_=new WebAssembly.Module(fs.readFileSync(wasm));
if(WebAssembly.Module.imports(module_).length)throw Error('SDK must have zero imports');
const e=new WebAssembly.Instance(module_,{}).exports, compiler=createCompiler(e);
const stats=[];
for(const name of JSON.parse(fs.readFileSync(dir+'/cases.json','utf8'))) {
 const raw=fs.readFileSync(dir+'/'+name+'.request');const start=performance.now();
 const result=compiler.compileBytes(raw); const elapsed=performance.now()-start;
 fs.writeFileSync(dir+'/'+name+'.wasm',result.bytes);stats.push({name,total_ms:Number(elapsed.toFixed(3))});
}
// Exercise the supplied source adapter without parsing artifact numbers.
const result=compiler.compileSource({entryId:'sdk-fixture',source:'value Above 9007199254740993;'});
if(!result.ok||!result.text.includes('9007199254740993'))throw Error('exact integer adapter');
// Adapter finally releases input even on an incomplete/invalid fake export failure.
let released=0;
const mock={...e,weave_compiler_input_new:()=>1,weave_compiler_input_write:()=>-2,weave_compiler_drop:()=>{released++;return 0;}};
try{createCompiler(mock).compileBytes(new Uint8Array([1]));throw Error('expected rejection');}catch(err){if(err.status!==-2)throw err;}
if(released!==1)throw Error('adapter leaked input');
let allocations=0;
const guarded=createCompiler({...e,weave_compiler_input_new:()=>{allocations++;return -3;}});
for(const request of [
 {entryId:'x',source:'x'.repeat(1048577)},
 {entryId:'λ'.repeat(257),source:''},
 {entryId:'x',source:'🚀'.repeat(262145)},
 {entryId:'x',source:'',modules:[{id:'λ'.repeat(65),revision:'1',source:''}]},
 {entryId:'x',source:'x'.repeat(1048576),modules:Array.from({length:4},(_,i)=>({id:'m'+i,revision:'1',source:'x'.repeat(1048576)}))}
]) {try{guarded.compileSource(request);throw Error('expected preflight rejection');}catch(err){if(err.status!==-3)throw err;}}
if(allocations!==0)throw Error('convenience API reached ABI before source budget rejection');
// All handles are released after all requests, including diagnostics.
const handles=[]; for(let i=0;i<16;i++){const h=e.weave_compiler_input_new(0);if(h<=0)throw Error('leak');handles.push(h);}
if(e.weave_compiler_input_new(0)!==-3)throw Error('handle quota');
for(const h of handles){if(e.weave_compiler_drop(h)!==0||e.weave_compiler_drop(h)!==-1)throw Error('stale handle');}
process.stdout.write(JSON.stringify(stats));
'''
with tempfile.TemporaryDirectory(prefix='weave-sdk-') as tmp:
 d=Path(tmp);reports=[];total=0
 for name,raw in cases:
  (d/(name+'.request')).write_bytes(raw)
  safe=subprocess.check_output([str(a.native.resolve())],input=raw)
  actual,timing=native_abi(raw);assert safe==actual,name+' native ABI differs from safe Rust'
  (d/(name+'.native')).write_bytes(actual);total+=len(actual)
  if name in fixtures:
   expected=json.loads(subprocess.check_output([str(a.compiler.resolve()),'artifacts',str(fixtures[name])]))
   result=json.loads(actual);assert result['ok'] and result['artifacts']==expected['artifacts'] and result['artifact_fingerprint']==expected['artifact_fingerprint'],name
  if name in ['exact-integers','diamond-modules','near-4MiB-decoded-budget','large-output','all-octets']:assert json.loads(actual)['ok'],actual
  expected_errors={'bad-reference':'E_REFERENCE_VALUE','bad-bytes':'E_BYTES_LITERAL','missing-module':'E_MODULE_MISSING','wrong-pin':'E_MODULE_DIGEST','cycle':'E_MODULE_CYCLE','invalid-utf8':'E_SDK_UTF8','invalid-json':'E_SDK_REQUEST','duplicate-fields':'E_SDK_REQUEST','decoded-bound':'E_SDK_BUDGET'}
  if name in expected_errors:assert json.loads(actual)['error']['code']==expected_errors[name],actual
  if name in ['source-error','original-module-diagnostic']:assert json.loads(actual)['ok'] is False,actual
  reports.append({'name':name,'request_bytes':len(raw),'response_bytes':len(actual),'native_abi':timing})
 (d/'cases.json').write_text(json.dumps([name for name,_ in cases]))
 if a.wasm:
  stats=json.loads(subprocess.check_output(['node','--input-type=module','-e',node,str(a.wasm.resolve()),str(root/'sdk/weave-compiler.mjs'),str(d)]))
  for report,stat in zip(reports,stats):
   name=report['name'];assert (d/(name+'.native')).read_bytes()==(d/(name+'.wasm')).read_bytes(),name+' WASM differs'
   report['wasm']=stat
report={'profile':'arbitrary-source-compiler-sdk/1','cases':reports,'identical_response_bytes':total,'wasm_imports':0 if a.wasm else None,'status':'passed'}
if a.report:a.report.write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report))
