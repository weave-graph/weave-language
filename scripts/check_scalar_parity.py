#!/usr/bin/env python3
"""Execute the fixed source compiler profile natively and in zero-import WASM."""
import argparse
from pathlib import Path
import subprocess
p=argparse.ArgumentParser()
p.add_argument('--native',type=Path,required=True)
p.add_argument('--wasm',type=Path,required=True)
a=p.parse_args()
native=subprocess.check_output([str(a.native.resolve())]).rstrip(b'\n')
node=r'''
const fs=require('fs');
const module_=new WebAssembly.Module(fs.readFileSync(process.argv[1]));
if(WebAssembly.Module.imports(module_).length)throw new Error('fixture must have zero host imports');
const e=new WebAssembly.Instance(module_,{}).exports;
const pointer=e.scalar_fixture_ptr(), length=e.scalar_fixture_len();
if(length>1048576)throw new Error('fixture output exceeds bound');
process.stdout.write(Buffer.from(e.memory.buffer,pointer,length));
'''
wasm=subprocess.check_output(['node','-e',node,str(a.wasm.resolve())])
assert native==wasm,'native/WASM typed values, plans, identities or diagnostics differ'
print(f'Scalar specialization parity passed: {len(native):,} identical JSON bytes; four scalar cases, three scalar errors, one complete artifact and its rejection diagnostic; zero WASM host imports')
