#!/usr/bin/env python3
"""LB1 (weave-language#5): source reader syntax on a real engine with the B1 registry.

The script builds a registry store (users alice, bob and carol; groups
`maintenance` = {alice, bob} and `finance` = {carol}; bob and carol hold
their own Read grants on the graphs, and carol is in no reader group), then, using only compiled `.weave` source:

1. alice runs `examples/permissions/restricted_writes.weave`: group-restricted,
   reader-restricted and public facts, an embedding, a correction and a
   snapshot. Members see the group facts; carol, a non-member reader of the
   same graph, sees only the public ones, in a lens, `expand`, `changes`,
   `nearest` and the snapshot. A literal `readers "bob"` record is visible to
   bob alone, and a group ID inside `readers` matches no one.
2. A `correct` without readers does not compile (`E_VISIBILITY_REQUIRED`);
   restating the group keeps the new version hidden from carol.
3. The engine rejects declarations that exceed the writer's authority: alice
   naming `finance` (a group she is not in) or an unknown group fails with the
   same `E_UNAVAILABLE`; agent `alice-blind`, which exercises none of alice's
   groups, cannot name `maintenance`, while `alice-bot`, which exercises it,
   can, and its fact records `on_behalf_of: ["alice"]`. Compiled against
   alice's visible manifest, the over-reaching source is rejected at compile
   time with the same code.
4. Removing bob from `maintenance` hides the group facts from him on his next
   request (he keeps his own Read grant on the graph); nothing is rewritten.

Every plan decodes with the contract's strict decoder before it runs (the
engine CLI decodes with `decode_program`).
"""
import argparse, json, subprocess, tempfile
from pathlib import Path

p = argparse.ArgumentParser()
p.add_argument('--compiler', type=Path, required=True)
p.add_argument('--engine', type=Path, required=True)
p.add_argument('--report', type=Path)
a = p.parse_args()
compiler, engine = str(a.compiler.resolve()), str(a.engine.resolve())
root = Path(__file__).resolve().parents[1]
checks = []
tmp = Path(tempfile.mkdtemp(prefix='weave-lb1-'))
db = str(tmp / 'store.db')
counter = [0]


def check(name, condition, detail=''):
    if not condition:
        raise SystemExit(f'FAILED {name}: {detail}')
    checks.append(name)


def write_file(suffix, data):
    counter[0] += 1
    path = tmp / f'{counter[0]}.{suffix}'
    path.write_bytes(data if isinstance(data, bytes) else data.encode())
    return path


def compile_source(source, *flags):
    """(ok, plan or diagnostic)."""
    out = subprocess.run([compiler, 'plan', str(write_file('weave', source)), *flags], capture_output=True)
    if out.returncode:
        return False, json.loads(out.stderr)
    return True, json.loads(out.stdout)


def plan_of(source, *flags):
    ok, value = compile_source(source, *flags)
    check('compiles', ok, f'{value}\n{source}')
    return value


def engine_run(args, data=None):
    cmd = [engine, *args]
    if data is not None:
        cmd.append(str(write_file('json', json.dumps(data))))
    out = subprocess.run(cmd, capture_output=True)
    if out.returncode:
        return False, json.loads(out.stderr)
    return True, json.loads(out.stdout)


def run(plan, actor, ok=True):
    success, value = engine_run(['run', '--db', db, '--actor', actor], plan)
    if ok:
        check('runs', success, f'{actor}: {value}\n{json.dumps(plan)[:600]}')
    return success, value


def registry(changes, actor='root'):
    success, value = engine_run(['registry', 'apply', '--db', db, '--actor', actor], changes)
    check('registry change', success, value)


def visible_nodes(actor, graph='fleet'):
    _, results = run(plan_of(f'use G graph "{graph}"; lens All from G {{ }}'), actor)
    graph_value = results[0]['result']['graph']
    return {n['id']: n for n in graph_value['nodes']}, {e['id']: e for e in graph_value['edges']}


# ---- Registry: three users, two groups, graph ACLs and two agents of alice.
check('registry enabled', engine_run(['registry', 'enable', '--db', db, '--actor', 'root'])[0])
registry([{'change': 'add_principal', 'id': u, 'kind': 'user'} for u in ['alice', 'bob', 'carol']]
         + [{'change': 'add_principal', 'id': g, 'kind': 'group'} for g in ['maintenance', 'finance']]
         + [{'change': 'add_member', 'group': 'maintenance', 'member': m} for m in ['alice', 'bob']]
         + [{'change': 'add_member', 'group': 'finance', 'member': 'carol'}]
         + [{'change': 'grant', 'graph_id': g, 'subject': 'maintenance', 'permission': 'write'} for g in ['fleet', 'Workshop']]
         + [{'change': 'grant', 'graph_id': g, 'subject': u, 'permission': 'read'} for g in ['fleet', 'Workshop'] for u in ['bob', 'carol']])
delegation = {'scopes': [{'graph_id': 'fleet', 'permission': 'write'}], 'may_delegate': False}
registry([{'change': 'add_agent', 'id': 'alice-bot', 'owner': 'alice',
           'delegation': dict(delegation, exercise={'kind': 'only', 'identities': ['maintenance']})},
          {'change': 'add_agent', 'id': 'alice-blind', 'owner': 'alice', 'delegation': delegation}], actor='alice')

# ---- 1. Group-restricted source writes stay hidden from non-members.
example = (root / 'examples/permissions/restricted_writes.weave').read_text()
plan = plan_of(example)
facts = [c['fact'] for c in plan['commands'][0]['changes']]
check('example lowers reader_groups', facts[2]['reader_groups'] == ['maintenance'] and facts[5]['readers'] == ['bob'])
_, results = run(plan, 'alice')
check('example commits three revisions', [r['kind'] for r in results] == ['committed'] * 3, results)

bob_nodes, bob_edges = visible_nodes('bob')
carol_nodes, carol_edges = visible_nodes('carol')
alice_nodes, _ = visible_nodes('alice')
check('member sees group facts', {'note-1', 'unit-1', 'memo-1'} <= set(bob_nodes) and 'about:1' in bob_edges, bob_nodes)
check('member sees the corrected version', bob_nodes['note-1']['properties']['text'] == 'bearing replaced')
check('non-member reader sees only public facts', set(carol_nodes) == {'unit-1'} and not carol_edges, (carol_nodes, carol_edges))
check('literal readers: bob only, not even the writer', 'memo-1' not in alice_nodes and 'memo-1' not in carol_nodes)
check('group-restricted records keep their reader_groups', bob_nodes['note-1']['reader_groups'] == ['maintenance'])

for name, source in [
    ('expand', 'use G graph "fleet"; expand E from G { seeds "unit-1"; hops 1; direction both; }'),
    ('changes', 'use G graph "fleet"; changes C from G { since as_of 0; }'),
    ('nearest', 'embedding_space Notes id "maint-notes" revision "1" encoder "word-hash" version "1" '
                'preprocessing "words/v1" dimensions 3 metric cosine_distance;\n'
                'use G graph "fleet"; nearest N from G { space Notes; vector [0.9, 0.1, 0.0]; k 5; }'),
]:
    texts = {}
    for actor in ['bob', 'carol']:
        _, out = run(plan_of(source), actor)
        texts[actor] = json.dumps(out)
    check(f'{name}: member sees note-1', 'note-1' in texts['bob'], name)
    check(f'{name}: non-member never sees note-1, memo-1 or about:1',
          all(x not in texts['carol'] for x in ['note-1', 'memo-1', 'about:1', 'emb:note-1']), texts['carol'][:400])

bob_ws, bob_ws_edges = visible_nodes('bob', 'Workshop')
carol_ws, carol_ws_edges = visible_nodes('carol', 'Workshop')
check('snapshot: member sees group records', set(bob_ws) == {'bay-1', 'bay-2'} and 'next' in bob_ws_edges)
check('snapshot: non-member sees public records only', set(carol_ws) == {'bay-2'} and not carol_ws_edges)

# A group ID inside `readers` is a literal principal ID that matches no one.
run(plan_of('write W graph "fleet" { assert node "literal-1" entity "L1" space "notes" readers "maintenance"; }'), 'alice')
check('group ID in readers matches no member', 'literal-1' not in visible_nodes('bob')[0])

# ---- 2. Corrections must restate readers.
ok, diagnostic = compile_source('write W graph "fleet" { correct node "note-1" entity "NOTE-1" space "notes" '
                                'property "text" "leaked"; }')
check('correct without readers does not compile', not ok and diagnostic['code'] == 'E_VISIBILITY_REQUIRED', diagnostic)
run(plan_of('write W graph "fleet" { correct node "note-1" entity "NOTE-1" space "notes" '
            'property "text" "bearing torqued" reader_groups "maintenance"; }'), 'bob')
check('restated correction stays hidden', 'note-1' not in visible_nodes('carol')[0])
check('restated correction is the live version', visible_nodes('bob')[0]['note-1']['properties']['text'] == 'bearing torqued')

# ---- 3. The engine rejects declarations beyond the writer's authority.
def restricted(node, group):
    return f'write W graph "fleet" {{ assert node "{node}" entity "{node.upper()}" space "notes" reader_groups "{group}"; }}'


errors = {}
for group in ['finance', 'no-such-group']:
    success, error = run(plan_of(restricted('over-1', group)), 'alice', ok=False)
    errors[group] = error
    check(f'engine rejects reader group {group}', not success and error['code'] == 'E_UNAVAILABLE', error)
check('hidden and unknown groups fail identically', errors['finance'] == errors['no-such-group'], errors)
success, error = run(plan_of(restricted('agent-1', 'maintenance')), 'alice-blind', ok=False)
check('agent cannot name a group it does not exercise', not success and error['code'] == 'E_UNAVAILABLE', error)
run(plan_of(restricted('agent-2', 'maintenance')), 'alice-bot')
_, results = run(plan_of('use G graph "fleet"; lens All from G { }'), 'bob')
agent_fact = [f for f in results[0]['result']['fact_provenance'] if f['fact'].get('node_id') == 'agent-2']
check('agent write is visible to members', len(agent_fact) == 1)
check('agent write records the delegation chain',
      agent_fact[0]['provenance'].get('writer') == 'alice-bot' and agent_fact[0]['provenance'].get('on_behalf_of') == ['alice'],
      agent_fact[0])
check('agent write hidden from non-members', 'agent-2' not in visible_nodes('carol')[0])

# The same rejection at compile time, from alice's visible manifest.
_, listing = engine_run(['graphs', '--db', db, '--actor', 'alice'])
_, membership = run(plan_of('use R graph "weave:registry"; lens M from R { match relation "weave:member"; }'), 'alice')
groups = sorted({e['to'] for e in membership[0]['result']['graph']['edges'] if e['from'] == 'alice'})
check('alice discovers her groups from the registry graph', groups == ['maintenance'], groups)
manifest = {'format': 'weave-visible/1', 'principal': 'alice',
            'graphs': [{'graph_id': g['graph_id'], 'permission': g['permission'], 'branches': g['branches']} for g in listing],
            'reader_groups': groups}
manifest_path = str(write_file('json', json.dumps(manifest)))
ok, diagnostic = compile_source(restricted('over-2', 'finance'), '--visible', manifest_path)
check('manifest rejects the hidden group at compile time', not ok and diagnostic['code'] == 'E_UNAVAILABLE', diagnostic)
ok, diagnostic = compile_source('use P graph "payroll"; lens L from P { }', '--visible', manifest_path)
check('manifest rejects an undiscoverable graph', not ok and diagnostic['code'] == 'E_UNAVAILABLE', diagnostic)
ok, value = compile_source(restricted('ok-1', 'maintenance'), '--visible', manifest_path, '--untrusted')
check('manifest admits a group alice may name', ok, value)
run(value, 'alice')

# ---- 4. Revocation takes effect on the next request.
registry([{'change': 'remove_member', 'group': 'maintenance', 'member': 'bob'}])
after, _ = visible_nodes('bob')
check('revoked member loses group facts', 'note-1' not in after and 'ok-1' not in after and 'unit-1' in after, sorted(after))
check('revoked member keeps literal readers', 'memo-1' in after)

report = {'profile': 'weave-language-lb1-permissions/1', 'contract': plan['version'], 'checks': len(checks),
          'status': 'passed'}
if a.report:
    a.report.write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps(report))
