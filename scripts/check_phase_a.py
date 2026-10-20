#!/usr/bin/env python3
"""LA1 (weave-language#3) Phase A exit questions 1-5 from `.weave` source on a real engine.

A small synthetic fleet (sites, firmware, units, maintenance notes and
incidents; two units restricted to `fleet-admin`) is imported with one JSON
fact write. Then, using only compiled `examples/phase_a/*.weave` sources and
sources of the same syntax:

1. 1,000 incremental corrections and firmware rollouts, each its own `write`
   (one fact revision each), equal to the same writes made in JSON;
2. "which units at site S ran firmware F on date D, as we knew it on date K"
   and "what changed for unit U between K and the head";
3. notes similar to an incident within the 2-hop neighborhood of the affected
   unit, valid at the incident time (`nearest` over an `expand`);
4. similar-unit candidate relations absent from the graph, with provenance;
5. restricted units stay hidden in every answer above.

Embeddings come from a deterministic word-hashing embedder (as the engine's
`fleet_hybrid.rs`), written as embedding facts through a compiled `write`.
Every answer is compared with the engine's JSON reference plan and with an
independent oracle, cites fact provenance, and replays in a fresh process.
"""
import argparse, json, math, re, struct, subprocess, tempfile, time
from pathlib import Path

p = argparse.ArgumentParser()
p.add_argument('--compiler', type=Path, required=True)
p.add_argument('--engine', type=Path, required=True)
p.add_argument('--updates', type=int, default=1000)
p.add_argument('--report', type=Path)
a = p.parse_args()
compiler, engine = str(a.compiler.resolve()), str(a.engine.resolve())
root = Path(__file__).resolve().parents[1]
examples = root / 'examples/phase_a'
version = json.loads((root / 'vendor/manifest.json').read_text())['contract_version']
ADMIN, ANALYST, WRITER = 'fleet-admin', 'fleet-analyst', 'fleet-writer'
GRAPH = 'fleet'
DIMS = 64
processes = 0
checks = []


def now_ms():
    return time.time_ns() // 1_000_000


def f32(x):
    return struct.unpack('f', struct.pack('f', x))[0]


def embed(text):
    """Signed feature hashing of lowercase words (FNV-1a), L2-normalized, binary32."""
    v = [0.0] * DIMS
    for word in re.split(r'[^0-9a-z]+', text.lower()):
        if len(word) <= 2:
            continue
        h = 0xcbf29ce484222325
        for b in word.encode():
            h = ((h ^ b) * 0x100000001b3) & 0xFFFFFFFFFFFFFFFF
        v[h % DIMS] += 1.0 if (h >> 40) & 1 == 0 else -1.0
    norm = math.sqrt(sum(x * x for x in v))
    if norm == 0:
        v[0] = 1.0
        return v
    return [f32(x / norm) for x in v]


def distance(x, y):
    """The normative cosine distance: binary64 sums in component order, clamped."""
    dot = na = nb = 0.0
    for p_, q in zip(x, y):
        dot += p_ * q
        na += p_ * p_
        nb += q * q
    return min(max(1.0 - dot / (math.sqrt(na) * math.sqrt(nb)), 0.0), 2.0)


# ---- The synthetic fleet (deterministic).
SITES, FIRMWARE, UNITS = 3, 4, 12
RESTRICTED = {4, 9}
WORDS = ['bearing vibration high replaced bearing', 'firmware update reboot loop fixed',
         'fan noise cleaned filter', 'temperature sensor drift recalibrated',
         'bearing noise lubricated', 'power supply ripple replaced capacitor',
         'vibration alarm loose mount tightened', 'reboot after firmware rollout']
INCIDENTS = {1: 'unexpected reboot loop after firmware update', 4: 'bearing vibration alarm high noise',
             7: 'high vibration and bearing noise reported'}


def readers(unit):
    return [ADMIN] if unit in RESTRICTED else []


facts = {}  # id -> fact JSON, the oracle's view of the head


def put(fact):
    facts[fact['id']] = fact
    return {'change': 'assert', 'fact': fact}


def node(id, entity, space, props=None, valid=None, rd=()):
    f = {'kind': 'node', 'id': id, 'entity_id': entity, 'space_id': space,
         'properties': props or {}, 'metadata': [], 'readers': list(rd)}
    if valid is not None:
        f['valid_time'] = {'start': valid, 'end': None}
    return f


def edge(id, pred, src, dst, start=0, end=None, rd=()):
    return {'kind': 'edge', 'id': id, 'predicate': pred, 'from': src, 'to': dst,
            'valid_time': {'start': start, 'end': end}, 'polarity': 'positive', 'properties': {},
            'metadata': [], 'readers': list(rd), 'derived_from': []}


base = []
for s in range(1, SITES + 1):
    base.append(put(node(f'site-{s}', f'SITE-{s}', GRAPH)))
for f in range(1, FIRMWARE + 1):
    base.append(put(node(f'fw-{f}', f'FW-{f}', GRAPH)))
runs = {}  # unit -> (edge id, firmware, start, generation)
notes = []  # (node id, unit, text, valid from)
for u in range(1, UNITS + 1):
    rd = readers(u)
    base.append(put(node(f'unit-{u}', f'UNIT-{u}', GRAPH, {'model': f'M-{u % 3 + 1}', 'status': 'active'}, rd=rd)))
    base.append(put(edge(f'at:unit-{u}', 'located_at', f'unit-{u}', f'site-{(u - 1) % SITES + 1}', rd=rd)))
    fw = (u - 1) % 2 + 1
    base.append(put(edge(f'runs:unit-{u}:0', 'runs', f'unit-{u}', f'fw-{fw}', rd=rd)))
    runs[u] = (f'runs:unit-{u}:0', fw, 0, 0)
    for j in range(1, 4):
        text = WORDS[(u * 3 + j) % len(WORDS)]
        nid = f'note-{u}-{j}'
        base.append(put(node(nid, nid.upper(), 'notes', {'text': text}, valid=100 * j, rd=rd)))
        base.append(put(edge(f'on:{nid}', 'performed_on', nid, f'unit-{u}', 100 * j, rd=rd)))
        notes.append((nid, u, text, 100 * j))
for u, text in INCIDENTS.items():
    rd = readers(u)
    base.append(put(node(f'incident-{u}', f'INCIDENT-{u}', 'notes', {'text': text}, valid=1000, rd=rd)))
    base.append(put(edge(f'affected:incident-{u}', 'affected', f'incident-{u}', f'unit-{u}', 1000, rd=rd)))
    notes.append((f'incident-{u}', u, text, 1000))
public_units = [u for u in range(1, UNITS + 1) if u not in RESTRICTED]

with tempfile.TemporaryDirectory(prefix='weave-phase-a-') as directory:
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
        out = subprocess.run([compiler, 'plan', str(write_file('weave', text.encode()))], capture_output=True)
        assert out.returncode == 0, (out.stderr.decode(), text[:400])
        plan = json.loads(out.stdout)
        assert plan['version'] == version
        return plan

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

    def inline(plan):
        """The engine's JSON reference form of a compiled plan: every reference to an
        earlier binding replaced by its expression, each value evaluated on its own."""
        bound = {}

        def expand_refs(value):
            if isinstance(value, dict):
                if value.get('kind') == 'reference':
                    return bound[value['name']]
                return {k: expand_refs(v) for k, v in value.items()}
            if isinstance(value, list):
                return [expand_refs(v) for v in value]
            return value
        commands = []
        for c in plan['commands']:
            if c['op'] == 'bind':
                bound[c['name']] = expand_refs(c['value'])
                commands.append({'op': 'evaluate', 'value': bound[c['name']]})
        return {'version': plan['version'], 'commands': commands}

    def program(*commands):
        return {'version': version, 'commands': list(commands)}

    db, ref_db = work / 'fleet.db', work / 'reference.db'
    for store in (db, ref_db):
        execute(store, program({'op': 'write', 'graph_id': GRAPH, 'changes': base}), WRITER, [GRAPH])

    # ---- Question 1: 1,000 individual fact writes from source, equal to JSON writes.
    def edge_source(change, f):
        end = 'infinity' if f['valid_time']['end'] is None else f['valid_time']['end']
        return (f'{change} edge "{f["id"]}" from "{f["from"]}" to "{f["to"]}" relation "{f["predicate"]}" '
                f'valid {f["valid_time"]["start"]} until {end};')

    def node_source(change, f):
        props = ' '.join(f'property {json.dumps(k)} {json.dumps(v)}' for k, v in sorted(f['properties'].items()))
        return f'{change} node "{f["id"]}" entity "{f["entity_id"]}" space "{f["space_id"]}" {props};'

    updates = []  # (source text, JSON changes)
    # The example itself is update 1: a rollout of unit 2 and a correction of unit 3.
    example = (examples / 'q1_fact_writes.weave').read_text()
    assert len(example.splitlines()) <= 15
    cur, fw, start, gen = runs[2]
    old = dict(facts[cur], valid_time={'start': start, 'end': 2000})
    new = edge('runs:unit-2:1', 'runs', 'unit-2', 'fw-3', 2000)
    corrected = dict(facts['unit-3'], properties={'model': 'M-1', 'status': 'serviced'})
    assert facts['unit-3']['properties']['model'] == 'M-1'
    updates.append((example, [[{'change': 'correct', 'fact': old}, {'change': 'assert', 'fact': new}],
                              [{'change': 'correct', 'fact': corrected}]]))
    facts[cur], facts[new['id']], facts['unit-3'] = old, new, corrected
    runs[2] = (new['id'], 3, 2000, 1)
    t = 2000
    for k in range(1, a.updates):
        t += 10
        u = public_units[(k * 7) % len(public_units)]
        if k % 3:  # a firmware rollout: close the current install, start the next
            cur, fw, start, gen = runs[u]
            old = dict(facts[cur], valid_time={'start': start, 'end': t})
            nfw = fw % FIRMWARE + 1
            new = edge(f'runs:unit-{u}:{gen + 1}', 'runs', f'unit-{u}', f'fw-{nfw}', t)
            src = ('write Rollout graph "fleet" {\n  ' + edge_source('correct', old) + '\n  '
                   + edge_source('assert', new) + '\n}\n')
            updates.append((src, [[{'change': 'correct', 'fact': old}, {'change': 'assert', 'fact': new}]]))
            facts[cur], facts[new['id']] = old, new
            runs[u] = (new['id'], nfw, t, gen + 1)
        else:  # an attribute correction
            fixed = dict(facts[f'unit-{u}'], properties=dict(facts[f'unit-{u}']['properties'], status=f'checked-{k}'))
            src = 'write Correction graph "fleet" { ' + node_source('correct', fixed) + ' }\n'
            updates.append((src, [[{'change': 'correct', 'fact': fixed}]]))
            facts[fixed['id']] = fixed
    compiled = []
    for src, changes in updates:
        plan = compile_source(src)
        writes = [c for c in plan['commands'] if c['op'] == 'write']
        assert [w['changes'] for w in writes] == changes, (src, writes, changes)
        compiled += writes
    total_writes = len(compiled)
    assert total_writes >= a.updates
    known = None
    known_index = len(compiled) // 2
    for i in range(0, len(compiled), 100):
        chunk = compiled[i:i + 100]
        if known is None and i >= known_index:
            time.sleep(0.05)
            known = now_ms()  # K: half of the updates are recorded
            time.sleep(0.05)
        out = execute(db, program(*chunk), WRITER, [GRAPH])
        assert [o['kind'] for o in out] == ['committed'] * len(chunk)
        assert len({o['revision'] for o in out}) == len(chunk), 'each update is its own fact revision'
        ref = execute(ref_db, program(*[{'op': 'write', 'graph_id': GRAPH, 'branch_id': 'main', 'changes': c['changes']}
                                        for c in chunk]), WRITER, [GRAPH])
        assert [o['kind'] for o in ref] == ['committed'] * len(chunk)
    head_query = program({'op': 'evaluate', 'value': {'kind': 'query', 'query': {'graph_id': GRAPH}}})
    for who in [ADMIN, ANALYST]:
        mine = execute(db, head_query, who)[0]['result']['graph']
        theirs = execute(ref_db, head_query, who)[0]['result']['graph']
        assert mine == theirs, 'source writes and JSON writes give the same head'
    admin_head = execute(db, head_query, ADMIN)[0]['result']['graph']
    assert {e['id']: e['valid_time'] for e in admin_head['edges']} == {
        i: f['valid_time'] for i, f in facts.items() if f['kind'] == 'edge'}
    checks += [f'q1-{len(updates)}-updates-as-{total_writes}-individual-writes', 'q1-json-reference-head-equal']

    # ---- Embeddings: a compiled write of caller-supplied vectors.
    space_lines = {
        'Notes': 'embedding_space Notes id "hw-notes" revision "1" encoder "weave-test-hash-embedder"\n'
                 '  version "1" preprocessing "lowercase-words/v1" dimensions 64 metric cosine_distance;',
        'Units': 'embedding_space Units id "hw-units" revision "1" encoder "weave-test-hash-embedder"\n'
                 '  version "1" preprocessing "unit-profile/v1" dimensions 64 metric cosine_distance;'}
    for name, line in space_lines.items():
        for example_name in ['q3_hybrid_retrieval', 'q4_candidate_relations', 'q5_restricted']:
            text = (examples / f'{example_name}.weave').read_text()
            if f'embedding_space {name} ' in text:
                assert line in text, (example_name, 'space declaration differs from the written one')
    note_vectors = {nid: (embed(text), u, start) for nid, u, text, start in notes}
    profiles = {}
    for nid, u, text, _ in notes:
        profiles[u] = profiles.get(u, f'model {facts[f"unit-{u}"]["properties"]["model"]}') + ' ' + text
    unit_vectors = {u: embed(profiles[u] + f' fw-{runs[u][1]}') for u in range(1, UNITS + 1)}

    def vec(v):
        return '[' + ', '.join(repr(x) for x in v) + ']'

    lines = [space_lines['Notes'], space_lines['Units'], 'write Embed graph "fleet" {', '  assert space Notes;',
             '  assert space Units;']
    for nid, (v, u, start) in note_vectors.items():
        lines.append(f'  assert embedding "emb:{nid}" in Notes subject node "{nid}" vector {vec(v)} valid {start} until infinity;')
    for u, v in unit_vectors.items():
        lines.append(f'  assert embedding "emb:unit-{u}" in Units subject node "unit-{u}" vector {vec(v)} valid 0 until infinity;')
    lines.append('}')
    embed_plan = compile_source('\n'.join(lines))
    stored = embed_plan['commands'][0]['changes']
    written = {**{f'emb:{n}': v for n, (v, _, _) in note_vectors.items()}, **{f'emb:unit-{u}': v for u, v in unit_vectors.items()}}
    for c in stored:
        if c['fact']['kind'] == 'embedding':  # the compiled vector is the same binary32 vector
            assert [f32(x) for x in c['fact']['vector']] == written[c['fact']['id']], c['fact']['id']
    out = execute(db, embed_plan, ADMIN, [GRAPH])
    assert out[0]['kind'] == 'committed'
    time.sleep(0.02)

    def visible(unit, who):
        return who == ADMIN or unit not in RESTRICTED

    def values(plan, who):
        return {c['name']: o['result'] for c, o in zip(plan['commands'], execute(db, plan, who)) if c['op'] == 'bind'}

    def no_restricted(result, where):
        text = json.dumps(result)
        for u in RESTRICTED:
            for token in [f'"unit-{u}"', f'"UNIT-{u}"', f'"note-{u}-', f'"incident-{u}"', f'emb:unit-{u}"', f'emb:note-{u}-',
                          f'emb:incident-{u}"', f'"runs:unit-{u}:', f'"at:unit-{u}"']:
                assert token not in text, (where, token)

    # ---- Question 2: as known at K, and what changed for a unit.
    text = (examples / 'q2_as_known.weave').read_text()
    assert len(text.splitlines()) <= 15
    text = text.replace('as_of 1760000000000', f'as_of {known}')
    q2 = compile_source(text)
    for who in [ADMIN, ANALYST]:
        got = values(q2, who)
        ref = execute(db, inline(q2), who)
        for c, r in zip(q2['commands'], ref):
            assert r['result'] == got[c['name']], ('Q2 JSON reference', c['name'])
        at_site = got['AtSite']
        units_at = {e['from'] for e in at_site['graph']['edges'] if e['predicate'] == 'located_at'}
        want = {f'unit-{u}' for u in range(1, UNITS + 1) if (u - 1) % SITES == 0 and visible(u, who)}
        assert units_at == want, (who, units_at, want)
        assert at_site['fact_provenance'] and got['WhyAtSite']['graph']['nodes']
        changed = sorted((e['change'], e['id']) for e in got['ForUnit']['changes']['entries'])
        assert changed and all(c in ('asserted', 'corrected') for c, _ in changed), changed
        if who == ANALYST:
            for name, value in got.items():
                no_restricted(value, f'Q2 {name}')
    checks += ['q2-as-known-json-reference', 'q2-changes-for-unit']

    # ---- Question 3: nearest notes over the 2-hop neighborhood, at the incident time.
    def q3_oracle(result_region, query_id, who):
        region = {n['id'] for n in result_region['graph']['nodes']}
        qv = note_vectors[query_id][0]
        want = sorted((distance(qv, v), nid, f'emb:{nid}') for nid, (v, u, start) in note_vectors.items()
                      if nid != query_id and nid in region and visible(u, who) and start <= 1000)
        return [(s, e) for s, _, e in want[:10]]

    text = (examples / 'q3_hybrid_retrieval.weave').read_text()
    assert len(text.splitlines()) <= 15
    q3 = compile_source(text)
    for who in [ADMIN, ANALYST]:
        got = values(q3, who)
        hits = [(n['score'], n['embedding_id']) for n in got['Similar']['neighbors']]
        assert hits == q3_oracle(got['Region'], 'incident-7', who), (who, hits)
        assert hits and got['Similar']['exactness'] == 'exact'
        # The engine's JSON reference: the candidates expansion inline, as EMBEDDINGS.md writes it.
        ref = execute(db, inline(q3), who)[1]['result']
        # Every hit's subject (the note node) is cited too: the answer explains its source facts.
        cited_nodes = {f['fact'].get('node_id') for f in got['Similar']['fact_provenance']}
        assert {e.removeprefix('emb:') for _, e in hits} <= cited_nodes, cited_nodes
        assert ref['neighbors'] == got['Similar']['neighbors'] and ref['graph'] == got['Similar']['graph']
        cited = {f['fact'].get('embedding_id') for f in got['Similar']['fact_provenance']}
        assert {e for _, e in hits} <= cited, (cited, hits)
        assert all(f['provenance']['writer'] == ADMIN for f in got['Similar']['fact_provenance']
                   if f['fact']['kind'] == 'embedding')
        assert values(q3, who) == got, 'replay in a fresh process'
        if who == ANALYST:
            for name, value in got.items():
                no_restricted(value, f'Q3 {name}')
    checks += ['q3-nearest-over-expand-oracle', 'q3-json-reference', 'q3-provenance', 'q3-replay']

    # ---- Question 4: similar-unit candidate relations absent from the graph.
    text = (examples / 'q4_candidate_relations.weave').read_text()
    assert len(text.splitlines()) <= 15
    q4 = compile_source(text)
    for who in [ADMIN, ANALYST]:
        got = values(q4, who)['SimilarUnits']
        seen = [u for u in range(1, UNITS + 1) if visible(u, who)]
        pairs = {}
        for s in seen:
            best = min((distance(unit_vectors[s], unit_vectors[t]), f'unit-{t}') for t in seen if t != s)
            key = tuple(sorted([f'unit-{s}', best[1]]))
            pairs[key] = best[0]
        cands = {(c['from'], c['to']): c['score'] for c in got['candidates']}
        assert cands == pairs, (who, cands, pairs)
        assert len(cands) >= 2 and all(c['predicate'] == 'weave:similar_to' for c in got['candidates'])
        cited = {f['fact'].get('embedding_id') for f in got['fact_provenance']}
        for c in got['candidates']:
            assert {o['embedding_id'] for o in c['origins']} <= cited, c
        ref = execute(db, program({'op': 'evaluate', 'value': q4['commands'][0]['value']}), who)[0]['result']
        assert ref['candidates'] == got['candidates']
        assert values(q4, who)['SimilarUnits'] == got
        if who == ANALYST:
            no_restricted(got, 'Q4')
    checks += ['q4-candidates-oracle', 'q4-provenance', 'q4-json-reference', 'q4-replay']

    # ---- Question 5: the restricted unit 4 in every operator.
    text = (examples / 'q5_restricted.weave').read_text()
    assert len(text.splitlines()) <= 15
    q5 = compile_source(text)
    admin, analyst = values(q5, ADMIN), values(q5, ANALYST)
    assert any('unit-4' in (c['from'], c['to']) for c in admin['LikeUnit4']['candidates'])
    assert admin['Region']['graph']['nodes'] and admin['ForUnit']['changes']['entries']
    assert analyst['Region']['coverage'] == 'partial' and analyst['Region']['graph']['nodes'] == []
    assert analyst['LikeUnit4'].get('candidates', []) == [] and analyst['ForUnit']['changes']['entries'] == []
    # Question 3 for the restricted unit's incident: the admin gets hits; for the
    # analyst the hidden query embedding fails exactly like a missing one.
    q3_hidden = compile_source((examples / 'q3_hybrid_retrieval.weave').read_text()
                               .replace('unit-7', 'unit-4').replace('incident-7', 'incident-4'))
    assert values(q3_hidden, ADMIN)['Similar']['neighbors']
    hidden = execute(db, q3_hidden, ANALYST, ok=False)
    missing = execute(db, compile_source((examples / 'q3_hybrid_retrieval.weave').read_text()
                                         .replace('incident-7', 'no-such-incident')), ANALYST, ok=False)
    assert hidden == missing and 'E_UNAVAILABLE' in hidden, (hidden, missing)
    for name, value in analyst.items():
        no_restricted(value, f'Q5 {name}')
    checks += ['q5-restricted-hidden-everywhere', 'q5-admin-sees-restricted']

report = {'profile': 'phase-a-source-native/1', 'status': 'passed', 'updates': a.updates,
          'processes': processes, 'checks': checks}
if a.report:
    a.report.write_bytes((json.dumps(report, indent=2) + '\n').encode())
print(json.dumps(report))
