#!/usr/bin/env python3
"""Independent finite-set oracle for exact half-open interval specialization."""
import argparse, json, os, random, subprocess, tempfile
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--compiler',type=Path,required=True);a=p.parse_args();binary=a.compiler.resolve();binary=binary.with_suffix('.exe') if os.name=='nt' and binary.suffix!='.exe' else binary;compiler=str(binary)
rng=random.Random(1820)
intervals=[(s,e) for s in range(-3,4) for e in [*range(s+1,5),None]]
def expr(i):
 s,e=i
 return f'interval_open(time {s})' if e is None else f'interval(time {s}, time {e})'
def points(i):
 s,e=i
 return {v for v in range(-5,9) if v>=s and (e is None or v<e)}
lines=[];expected={}
def add(expression,value):
 name=f'Check{len(expected)}';lines.append(f'value {name} {expression};');expected[name]=value
for _ in range(60):
 x,y=rng.choice(intervals),rng.choice(intervals);sx,sy=points(x),points(y)
 for op,want in [('interval_equal',sx==sy),('interval_overlaps',bool(sx&sy)),('interval_within',sx<=sy)]:add(f'{op}({expr(x)}, {expr(y)})',want)
 for t in [-4,x[0],x[1] if x[1] is not None else 7]:add(f'interval_contains({expr(x)}, time {t})',t in sx)
for op,x,y,want in [('interval_before',(0,1),(1,2),False),('interval_meets',(0,1),(1,2),True),('interval_before',(0,1),(2,None),True),('interval_meets',(0,1),(2,None),False),('interval_before',(0,None),(2,3),False)]:add(f'{op}({expr(x)}, {expr(y)})',want)
lo,hi=-(1<<63),(1<<63)-1
add(f'interval_contains(interval(time {lo}, time {hi}), time {hi})',False)
add(f'interval_contains(interval_open(time {lo}), time {hi})',True)
add(f'interval_contains(interval(time {lo}, time {hi}), time {lo})',True)
with tempfile.TemporaryDirectory(prefix='weave-root-interval-') as tmp:
 f=Path(tmp)/'oracle.weave';f.write_text('\n'.join(lines))
 run=subprocess.run([compiler,'values',str(f)],capture_output=True,text=True,timeout=30)
 assert run.returncode==0,run.stderr
 got=json.loads(run.stdout)['values'];assert len(got)==len(expected)
 for k,v in expected.items():assert got[k]=={'type':'boolean','value':v},(k,got[k],v)
 failures=[('interval(time 1, time 1)','E_INTERVAL_BOUNDS'),('interval(time 2, time 1)','E_INTERVAL_BOUNDS'),('interval_end(interval_open(time 0))','E_INTERVAL_UNBOUNDED'),('interval_intersection(interval(time 0, time 1), interval(time 1, time 2))','E_INTERVAL_EMPTY')]
 for i,(expression,code) in enumerate(failures):
  f.write_text(f'value Bad {expression};')
  run=subprocess.run([compiler,'values',str(f)],capture_output=True,text=True,timeout=30)
  assert run.returncode and not run.stdout and code in run.stderr,(i,run.stdout,run.stderr)
print(json.dumps({'profile':'root-interval-finite-set-oracle','boolean_assertions':len(expected),'explicit_failure_cases':len(failures),'status':'passed'}))
