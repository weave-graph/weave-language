#!/usr/bin/env python3
"""LA1 (weave-language#3) compiler -> engine acceptance for the A3 query syntax.

1. Engine oracle fixtures: each `examples/traversal/<fixture>.weave` mirrors an
   engine core v1 fixture. The fixture's setup programs run on the real engine,
   the source is compiled, and every result is compared with the fixture's
   recorded results. Fixture clock times are mapped to real recorded times.
2. Phase A exit question 2 (`examples/traversal/fleet_questions.weave`) over a
   fleet written with fact writes: the compiled plan gives the same result as
   the engine's JSON reference plan and as an independent oracle, hides
   restricted units, cites provenance and replays after reopening the store.
"""
import argparse, json, re, subprocess, tempfile, time
from pathlib import Path

p = argparse.ArgumentParser()
p.add_argument('--compiler', type=Path, required=True)
p.add_argument('--engine', type=Path, required=True)
p.add_argument('--report', type=Path)
a = p.parse_args()
compiler, engine = str(a.compiler.resolve()), str(a.engine.resolve())
root = Path(__file__).resolve().parents[1]
fixtures = root / 'vendor/weave-contract/fixtures/core-v1/positive'
version = json.loads((root / 'vendor/manifest.json').read_text())['contract_version']
processes = 0
checks = []


def now_ms():
    return time.time_ns() // 1_000_000


with tempfile.TemporaryDirectory(prefix='weave-traversal-') as directory:
    work = Path(directory)
    serial = 0

    def write_file(suffix, data):
        global serial
        serial += 1
        path = work / f'{serial}.{suffix}'
        path.write_bytes(data)
        return path

    def compile_source(text):
        global processes
        processes += 1
        return json.loads(subprocess.check_output([compiler, 'plan', str(write_file('weave', text.encode()))]))

    def execute(db, plan, actor, writes=(), ok=True):
        global processes
        processes += 1
        cmd = [engine, 'run', '--db', str(db), '--actor', actor]
        for graph in writes:
            cmd += ['--write', graph]
        out = subprocess.run(cmd + [str(write_file('json', json.dumps(plan).encode()))], capture_output=True)
        if ok:
            assert out.returncode == 0, out.stderr.decode()
            return json.loads(out.stdout)
        assert out.returncode != 0
        return out.stderr.decode()

    def summary(output, expected):
        result = output['result']
        got = {'kind': output['kind'], 'coverage': result['coverage']}
        if 'nodes' in expected:
            got['nodes'] = sorted(n['id'] for n in result['graph']['nodes'])
        if 'edges' in expected:
            got['edges'] = sorted(e['id'] for e in result['graph']['edges'])
        if 'changes' in expected:
            got['changes'] = sorted([e['change'], e['kind'], e['id']] for e in result['changes']['entries'])
        want = dict(expected)
        for key in ['nodes', 'edges', 'changes']:
            if key in want:
                want[key] = sorted(want[key])
        return got, want

    # 1. Engine oracle fixtures.
    for name, source in [('expand-k-hop', 'expand_k_hop'),
                         ('expand-valid-time-as-of', 'expand_valid_time_as_of'),
                         ('changes-between-times', 'changes_between_times'),
                         ('as-of-partially-visible', 'as_of_partially_visible')]:
        fixture = json.loads((fixtures / f'{name}.json').read_text())
        setup = fixture['engine'].get('setup', [])
        db = work / f'{name}.db'
        clocks = []  # (fixture clock, real time before, real time after)
        for step in setup:
            time.sleep(0.05)
            before = now_ms()
            outputs = execute(db, step['program'], step['principal'], step['write_graphs'])
            after = now_ms()
            assert all(o['kind'] == 'committed' for o in outputs), outputs
            clocks.append((step['clock_ms'], before, after))
        time.sleep(0.05)

        def real(t):
            """Map a fixture recorded time to a real one on the same side of every write."""
            if not clocks:
                return t
            assert all(t != c for c, _, _ in clocks), 'a query time equal to a write time is ambiguous'
            if t < clocks[0][0]:
                return clocks[0][1] - 1
            for (c0, _, after0), (c1, before1, _) in zip(clocks, clocks[1:]):
                if c0 < t < c1:
                    return (after0 + before1) // 2
            return clocks[-1][2] + 10

        text = (root / f'examples/traversal/{source}.weave').read_text()
        text = re.sub(r'\bas_of (\d+)', lambda m: f'as_of {real(int(m.group(1)))}', text)
        plan = compile_source(text)
        assert plan['version'] == version
        inline = fixture['input']['commands']
        expected = fixture['engine']['results']
        prefix = [c for c in inline if c['op'] != 'evaluate']
        if prefix:  # The fixture's own writes run first, as JSON.
            outputs = execute(db, {'version': fixture['input']['version'], 'commands': prefix},
                              fixture['engine']['principal'], fixture['engine']['write_graphs'])
            assert [o['kind'] for o in outputs] == [r['kind'] for r in expected[:len(prefix)]]
            expected = expected[len(prefix):]
        binds = [c for c in plan['commands'] if c['op'] == 'bind']
        assert len(binds) == len(expected) == len(plan['commands']), name
        outputs = execute(db, plan, fixture['engine']['principal'])
        for command, output, want in zip(binds, outputs, expected):
            got, want = summary(output, want)
            assert got == want, (name, command['name'], got, want)
        # The engine's JSON reference (the fixture's own expressions at the mapped times) agrees.
        reference = {'version': version, 'commands': [
            {'op': 'evaluate', 'value': json.loads(re.sub(
                r'"(as_of|unix_millis)": (\d+)', lambda m: f'"{m.group(1)}": {real(int(m.group(2)))}',
                json.dumps(c['value'])))} for c in inline if c['op'] == 'evaluate']}
        assert [c['value'] for c in reference['commands']] == [c['value'] for c in binds], name
        referenced = execute(db, reference, fixture['engine']['principal'])
        assert [r['result'] for r in referenced] == [r['result'] for r in outputs], name
        checks.append(f'oracle-fixture:{name}')

    # 2. Phase A exit question 2 over a small fleet written with fact writes.
    db = work / 'fleet.db'

    def node(id, entity, readers=()):
        return {'change': 'assert', 'fact': {'kind': 'node', 'id': id, 'entity_id': entity, 'space_id': 'fleet',
                                             'properties': {}, 'metadata': [], 'readers': list(readers)}}

    def edge(id, predicate, src, dst, start=0, end=None, readers=(), change='assert'):
        return {'change': change, 'fact': {'kind': 'edge', 'id': id, 'predicate': predicate, 'from': src, 'to': dst,
                                           'valid_time': {'start': start, 'end': end}, 'polarity': 'positive',
                                           'properties': {}, 'metadata': [], 'readers': list(readers),
                                           'derived_from': []}}

    admin = ['fleet-admin']
    base = [node('site-1', 'SITE-1'), node('site-2', 'SITE-2'), node('fw-1', 'FW-1'), node('fw-2', 'FW-2'),
            node('unit-1', 'UNIT-1'), node('unit-2', 'UNIT-2'), node('unit-3', 'UNIT-3'),
            node('unit-4', 'UNIT-4', admin),
            edge('at-1', 'located_at', 'unit-1', 'site-1'), edge('at-2', 'located_at', 'unit-2', 'site-1'),
            edge('at-3', 'located_at', 'unit-3', 'site-2'), edge('at-4', 'located_at', 'unit-4', 'site-1', readers=admin),
            edge('runs-1', 'runs', 'unit-1', 'fw-1'), edge('runs-2', 'runs', 'unit-2', 'fw-2'),
            edge('runs-3', 'runs', 'unit-3', 'fw-1'), edge('runs-4', 'runs', 'unit-4', 'fw-1', readers=admin)]
    # A firmware rollout at valid time 500 for unit 2, recorded after K, plus a
    # restricted correction that ends unit 4's install at valid time 900.
    rollout = [edge('runs-2', 'runs', 'unit-2', 'fw-2', 0, 500, change='correct'),
               edge('runs-2b', 'runs', 'unit-2', 'fw-1', 500),
               edge('runs-4', 'runs', 'unit-4', 'fw-1', 0, 900, admin, change='correct')]
    first = now_ms()
    execute(db, {'version': version, 'commands': [{'op': 'write', 'graph_id': 'fleet', 'changes': base}]},
            'writer', ['fleet'])
    written = now_ms()
    time.sleep(0.05)
    known = now_ms()  # K: after the import, before the rollout was recorded
    time.sleep(0.05)
    execute(db, {'version': version, 'commands': [{'op': 'write', 'graph_id': 'fleet', 'changes': rollout}]},
            'writer', ['fleet'])
    time.sleep(0.05)
    source = (root / 'examples/traversal/fleet_questions.weave').read_text()
    assert len(source.splitlines()) <= 15
    expand_ref = {'kind': 'expand', 'graph_id': 'fleet', 'branch_id': 'main', 'seeds': ['site-1'], 'hops': 2,
                  'direction': 'both', 'predicates': ['located_at', 'runs'], 'valid_at': 1000, 'limit': 10000}
    changes_ref = {'kind': 'changes', 'graph_id': 'fleet', 'branch_id': 'main', 'from': {'kind': 'as_of'},
                   'to': {'kind': 'head'}, 'entities': ['UNIT-2'], 'limit': 10000}

    def running(result, firmware):
        edges = result['graph']['edges']
        at_site = {e['from'] for e in edges if e['predicate'] == 'located_at' and e['to'] == 'site-1'}
        return sorted(e['from'] for e in edges if e['predicate'] == 'runs' and e['to'] == firmware and e['from'] in at_site)

    oracle = {('fleet-analyst', 'known'): ['unit-1'], ('fleet-analyst', 'head'): ['unit-1', 'unit-2'],
              ('fleet-admin', 'known'): ['unit-1', 'unit-4'], ('fleet-admin', 'head'): ['unit-1', 'unit-2']}
    for actor in ['fleet-analyst', 'fleet-admin']:
        for state, recorded in [('known', known), ('head', None)]:
            text = source.replace('use Known graph "fleet" as_of 1760000000000;',
                                  f'use Known graph "fleet" as_of {recorded};' if recorded else 'use Known graph "fleet";')
            text = text.replace('as_of 1760000000000', f'as_of {known}')
            plan = compile_source(text)
            values = dict(zip([c['name'] for c in plan['commands']], execute(db, plan, actor)))
            at_site = values['AtSite']['result']
            assert running(at_site, 'fw-1') == oracle[(actor, state)], (actor, state, running(at_site, 'fw-1'))
            # The engine's JSON reference plan gives the same result.
            ref = dict(expand_ref, **({'as_of': recorded} if recorded else {}))
            ref_changes = dict(changes_ref, **{'from': {'kind': 'as_of', 'unix_millis': known}})
            reference = execute(db, {'version': version, 'commands': [
                {'op': 'evaluate', 'value': ref}, {'op': 'evaluate', 'value': {'kind': 'explain', 'input': ref}},
                {'op': 'evaluate', 'value': ref_changes}]}, actor)
            assert [c['value'] for c in plan['commands']][0] == ref
            assert reference[0]['result'] == at_site
            assert reference[1]['result']['graph'] == values['WhyAtSite']['result']['graph']
            assert reference[2]['result'] == values['ForUnit']['result']
            # Restricted units never reach the analyst, in any answer.
            if actor == 'fleet-analyst':
                for value in values.values():
                    text_value = json.dumps(value['result'])
                    assert 'unit-4' not in text_value and 'UNIT-4' not in text_value, state
            # Every element of the answer cites its engine-recorded provenance.
            cited = {f['fact'].get('node_id') or f['fact'].get('assertion_id') for f in at_site['fact_provenance']}
            assert {e['id'] for e in at_site['graph']['edges']} | {n['id'] for n in at_site['graph']['nodes']} <= cited, cited
            assert all(f['provenance']['writer'] == 'writer' for f in at_site['fact_provenance'])
            assert any(first <= f['provenance']['recorded_at_ms'] <= written for f in at_site['fact_provenance'])
            # Explain turns the expansion into an explicit derivation graph.
            assert values['WhyAtSite']['result']['graph']['nodes'], 'explain produced no derivations'
            # What changed for unit 2 between K and the head: the rollout.
            changes = values['ForUnit']['result']['changes']
            assert sorted((e['change'], e['id']) for e in changes['entries']) == [
                ('asserted', 'runs-2b'), ('corrected', 'runs-2')], changes
            # Reproducible after reopening the database: a fresh process gives the same answer.
            again = dict(zip([c['name'] for c in plan['commands']], execute(db, plan, actor)))
            assert again == values
    # The analyst sees no change for the restricted unit; the admin sees its correction.
    for actor, expected in [('fleet-analyst', []), ('fleet-admin', [('corrected', 'runs-4')])]:
        plan = compile_source(f'use Fleet graph "fleet"; changes ForUnit from Fleet {{ since as_of {known}; entities "UNIT-4"; }}')
        entries = execute(db, plan, actor)[0]['result']['changes']['entries']
        assert sorted((e['change'], e['id']) for e in entries) == expected, (actor, entries)
    checks += ['fleet-question-2-as-known', 'fleet-question-2-head', 'json-reference-equal', 'restricted-hidden',
               'fact-provenance', 'explain-expand', 'changes-for-unit', 'reopen-replay']

    # 3. A change set is terminal on the engine too: a hand-built operand fails.
    bad = {'version': version, 'commands': [
        {'op': 'bind', 'name': 'D', 'value': dict(changes_ref, **{'from': {'kind': 'head'}})},
        {'op': 'evaluate', 'value': {'kind': 'explain', 'input': {'kind': 'reference', 'name': 'D'}}}]}
    assert 'E_CHANGES_OPERAND' in execute(db, bad, 'fleet-admin', ok=False)
    checks.append('engine-changes-operand')

report = {'profile': 'traversal-source-native/1', 'status': 'passed', 'processes': processes, 'checks': checks}
if a.report:
    a.report.write_bytes((json.dumps(report, indent=2) + '\n').encode())
print(json.dumps(report))
