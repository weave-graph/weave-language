#!/usr/bin/env python3
"""LB1 (weave-language#5) acceptance: a scripted agent discovers a schema, writes a
query with a type error, repairs it from the compiler's diagnostics and runs it,
with no hard-coded field names.

The host (this script's setup half) builds a B1 registry store with randomly
named graphs, an embedding space of random dimensions and a random property
key, so nothing about the data can be known in advance. The agent is a
registered agent principal with delegated, narrowed authority. The agent half
(`Agent`) only uses what it discovers:

1. it lists the graphs it can discover (`weave-engine graphs`) and, for each
   readable one, compiles and runs a `changes ... since as_of 0` query, which
   returns the facts and the embedding-space descriptors it can see;
2. the host assembles the agent's visible manifest (`weave-visible/1`) from
   that discovery, plus the groups it delegated to the agent (engine B3/B4 will
   serve this manifest; see docs/PERMISSIONS.md);
3. the agent writes a `nearest` query with its own default embedding size,
   which does not match the stored space: the compiler, run with
   `--untrusted --read-only --visible`, reports `E_SPACE_MISMATCH` at the exact
   vector span; the agent reads the dimensions from the diagnostic, re-embeds,
   recompiles and runs the query; the hits equal an independent ranking of the
   embeddings the agent may see;
4. it records a finding with an implicitly public `write`: the untrusted policy
   rejects it (`E_VISIBILITY_REQUIRED`), the agent restates it with a group
   from its manifest, and the engine stores it hidden from a non-member who can
   read the graph, stamped `on_behalf_of` the agent's owner;
5. a guessed graph and a guessed group are rejected at compile time
   (`E_UNAVAILABLE`), and the hidden graph never appears in discovery.
"""
import argparse, hashlib, inspect, json, math, random, re, struct, subprocess, tempfile
from pathlib import Path

p = argparse.ArgumentParser()
p.add_argument('--compiler', type=Path, required=True)
p.add_argument('--engine', type=Path, required=True)
p.add_argument('--seed', type=int)
p.add_argument('--report', type=Path)
a = p.parse_args()
compiler, engine = str(a.compiler.resolve()), str(a.engine.resolve())
seed = a.seed if a.seed is not None else random.SystemRandom().randrange(1 << 30)
rng = random.Random(seed)
tmp = Path(tempfile.mkdtemp(prefix='weave-agent-'))
db = str(tmp / 'store.db')
checks = []
counter = [0]


def check(name, condition, detail=''):
    if not condition:
        raise SystemExit(f'FAILED {name} (seed {seed}): {detail}')
    checks.append(name)


def write_file(suffix, data):
    counter[0] += 1
    path = tmp / f'{counter[0]}.{suffix}'
    path.write_text(data)
    return path


def engine_cli(args, data=None):
    cmd = [engine, *args] + ([str(write_file('json', json.dumps(data)))] if data is not None else [])
    out = subprocess.run(cmd, capture_output=True)
    return (False, json.loads(out.stderr)) if out.returncode else (True, json.loads(out.stdout))


def compile_source(source, *flags):
    out = subprocess.run([compiler, 'plan', str(write_file('weave', source)), *flags], capture_output=True)
    return (False, json.loads(out.stderr)) if out.returncode else (True, json.loads(out.stdout))


def f32(x):
    return struct.unpack('f', struct.pack('f', x))[0]


def embed(text, dims):
    """Signed FNV-1a feature hashing, L2-normalized, binary32 (as check_phase_a.py)."""
    v = [0.0] * dims
    for word in re.split(r'[^0-9a-z]+', text.lower()):
        if len(word) <= 2:
            continue
        h = 0xcbf29ce484222325
        for b in word.encode():
            h = ((h ^ b) * 0x100000001b3) & 0xFFFFFFFFFFFFFFFF
        v[h % dims] += 1.0 if (h >> 40) & 1 == 0 else -1.0
    norm = math.sqrt(sum(x * x for x in v))
    if norm == 0:
        v[0] = 1.0
        return v
    return [f32(x / norm) for x in v]


def cosine_distance(x, y):
    dot = na = nb = 0.0
    for p_, q in zip(x, y):
        dot += p_ * q
        na += p_ * p_
        nb += q * q
    return min(max(1.0 - dot / (math.sqrt(na) * math.sqrt(nb)), 0.0), 2.0)


# ======================= Host setup: random names and data =======================
def token():
    return hashlib.sha256(str(rng.random()).encode()).hexdigest()[:8]


GRAPH, HIDDEN = f'kb-{token()}', f'hr-{token()}'
SPACE_ID, PROPERTY, TEAM, OTHER = f'space-{token()}', f'obs_{token()}', f'team-{token()}', f'other-{token()}'
DIMS = rng.choice([5, 6, 7, 9, 11, 12])
OWNER, AGENT, OUTSIDER = f'user-{token()}', f'agent-{token()}', f'user-{token()}'
NOTES = ['bearing vibration high replaced bearing', 'firmware update reboot loop fixed',
         'fan noise bearing worn', 'power supply ripple replaced capacitor',
         'coolant leak resealed pump', 'bearing noise lubricated', 'sensor drift recalibrated']

check('registry enabled', engine_cli(['registry', 'enable', '--db', db, '--actor', 'root'])[0])
ok, out = engine_cli(['registry', 'apply', '--db', db, '--actor', 'root'], [
    {'change': 'add_principal', 'id': OWNER, 'kind': 'user'},
    {'change': 'add_principal', 'id': OUTSIDER, 'kind': 'user'},
    {'change': 'add_principal', 'id': TEAM, 'kind': 'group'},
    {'change': 'add_principal', 'id': OTHER, 'kind': 'group'},
    {'change': 'add_member', 'group': TEAM, 'member': OWNER},
    {'change': 'add_member', 'group': OTHER, 'member': OUTSIDER},
    {'change': 'grant', 'graph_id': GRAPH, 'subject': TEAM, 'permission': 'write'},
    {'change': 'grant', 'graph_id': GRAPH, 'subject': OUTSIDER, 'permission': 'read'},
    {'change': 'grant', 'graph_id': HIDDEN, 'subject': OTHER, 'permission': 'write'}])
check('registry populated', ok, out)
EXERCISED = [TEAM]
ok, out = engine_cli(['registry', 'apply', '--db', db, '--actor', OWNER], [
    {'change': 'add_agent', 'id': AGENT, 'owner': OWNER, 'delegation': {
        'scopes': [{'graph_id': GRAPH, 'permission': 'write'}],
        'exercise': {'kind': 'only', 'identities': EXERCISED}, 'may_delegate': False}}])
check('agent registered', ok, out)

space = {'kind': 'embedding_space', 'id': SPACE_ID, 'revision': '1', 'encoder': 'word-hash',
         'encoder_version': '1', 'preprocessing': 'words/v1', 'dimensions': DIMS, 'metric': 'cosine_distance'}
changes = [{'change': 'assert', 'fact': space}]
expected_embeddings = {}
for i, text in enumerate(NOTES):
    restricted = i % 3 == 2
    node = {'kind': 'node', 'id': f'note-{i}', 'entity_id': f'NOTE-{i}', 'space_id': 'notes',
            'properties': {PROPERTY: text}, 'metadata': [], 'readers': [OWNER] if restricted else []}
    embedding = {'kind': 'embedding', 'id': f'emb-{i}', 'space': {'id': SPACE_ID, 'revision': '1'},
                 'subject': {'kind': 'node', 'node_id': f'note-{i}'}, 'vector': embed(text, DIMS),
                 'valid_time': {'start': 0, 'end': None}, 'readers': [OWNER] if restricted else []}
    changes += [{'change': 'assert', 'fact': node}, {'change': 'assert', 'fact': embedding}]
for graph, actor in [(GRAPH, 'root'), (HIDDEN, 'root')]:
    ok, out = engine_cli(['run', '--db', db, '--actor', actor], {
        'version': '1.2.0', 'commands': [{'op': 'write', 'graph_id': graph, 'branch_id': 'main', 'changes': changes}]})
    check(f'host data written', ok, out)


# ============================ The agent: discovery only ============================
class Agent:
    """Knows its principal and how to embed text; everything else is discovered."""
    DEFAULT_DIMS = 4

    def __init__(self, principal):
        self.principal = principal
        self.dims = self.DEFAULT_DIMS
        self.log = []

    def run(self, plan):
        return engine_cli(['run', '--db', db, '--actor', self.principal], plan)

    def discover(self):
        ok, listing = engine_cli(['graphs', '--db', db, '--actor', self.principal])
        found = {}
        for graph in listing:
            if graph['permission'] == 'discover':
                continue
            source = f'use G graph {json.dumps(graph["graph_id"])}; changes C from G {{ since as_of 0; }}'
            ok, plan = compile_source(source, '--untrusted', '--read-only')
            ok, result = self.run(plan)
            entries = result[0]['result']['changes']['entries']
            facts = [e['after'] for e in entries if e.get('after')]
            found[graph['graph_id']] = {
                'listing': graph,
                'spaces': [{k: v for k, v in f.items() if k != 'kind'} for f in facts if f['kind'] == 'embedding_space'],
                'properties': sorted({k for f in facts if f['kind'] == 'node' for k in f['properties']}),
                'embeddings': {f['id']: f for f in facts if f['kind'] == 'embedding'},
            }
        self.discovered = found
        return found

    def compile(self, source, manifest_path):
        ok, value = compile_source(source, '--untrusted', '--read-only', '--visible', manifest_path)
        self.log.append({'source': source, 'ok': ok, 'diagnostic': None if ok else value})
        return ok, value

    def similar_notes(self, question, manifest_path, k=3):
        """Ask every visible space for notes similar to `question`, repairing type errors."""
        graph_id, info = next((g, i) for g, i in self.discovered.items() if i['spaces'])
        s = info['spaces'][0]
        declaration = (f'embedding_space Space id {json.dumps(s["id"])} revision {json.dumps(s["revision"])} '
                       f'encoder {json.dumps(s["encoder"])} version {json.dumps(s["encoder_version"])} '
                       f'preprocessing {json.dumps(s["preprocessing"])} dimensions {s["dimensions"]} '
                       f'metric {s["metric"]};\n')
        for attempt in range(3):
            vector = ', '.join(repr(x) for x in embed(question, self.dims))
            source = (declaration + f'use G graph {json.dumps(graph_id)};\n'
                      f'nearest Hits from G {{ space Space; vector [{vector}]; k {k}; }}\n')
            ok, value = self.compile(source, manifest_path)
            if ok:
                ok, result = self.run(value)
                return graph_id, source, value, (ok, result), attempt
            if value['code'] == 'E_SPACE_MISMATCH':
                # Repair from the diagnostic alone: the space states its dimensions.
                self.dims = int(re.search(r'\((\d+) dimensions', value['message']).group(1))
                continue
            raise SystemExit(f'unrepairable diagnostic {value}')
        raise SystemExit('no repair within 3 attempts')

    def record_finding(self, graph_id, text, manifest, manifest_path):
        key = self.discovered[graph_id]['properties'][0]
        fact = (f'assert node "finding-1" entity "FINDING-1" space "findings" '
                f'property {json.dumps(key)} {json.dumps(text)}')
        visibility = ''
        for attempt in range(3):
            source = f'write Finding graph {json.dumps(graph_id)} {{ {fact}{visibility}; }}\n'
            ok, value = compile_source(source, '--untrusted', '--visible', manifest_path)
            self.log.append({'source': source, 'ok': ok, 'diagnostic': None if ok else value})
            if ok:
                return attempt, self.run(value)
            if value['code'] == 'E_VISIBILITY_REQUIRED':
                # Share with the groups this agent may name, never publicly.
                visibility = ' reader_groups ' + ', '.join(json.dumps(g) for g in manifest['reader_groups'])
                continue
            raise SystemExit(f'unrepairable diagnostic {value}')
        raise SystemExit('no repair within 3 attempts')


agent = Agent(AGENT)
discovered = agent.discover()
check('discovery lists exactly the delegated graph', list(discovered) == [GRAPH], list(discovered))
check('discovery finds the space and the property', discovered[GRAPH]['spaces'][0]['dimensions'] == DIMS
      and discovered[GRAPH]['properties'] == [PROPERTY])
check('agent never sees owner-only notes', all(int(e.split('-')[1]) % 3 != 2 for e in discovered[GRAPH]['embeddings']))
check('agent code names no host identifier', all(name not in inspect.getsource(Agent) for name in
      [GRAPH, HIDDEN, SPACE_ID, PROPERTY, TEAM, OTHER, OWNER, AGENT, str(DIMS) + ' dim']))

# The host assembles the agent's manifest from the discovery and its delegation.
manifest = {'format': 'weave-visible/1', 'principal': AGENT,
            'graphs': [{'graph_id': g, 'permission': i['listing']['permission'], 'branches': i['listing']['branches'],
                        'embedding_spaces': i['spaces']} for g, i in discovered.items()],
            'reader_groups': EXERCISED}
manifest_path = str(write_file('json', json.dumps(manifest)))

question = 'bearing noise and vibration'
graph_id, source, plan, (ran, result), attempts = agent.similar_notes(question, manifest_path)
first = agent.log[0]
check('first query has a type error', not first['ok'] and first['diagnostic']['code'] == 'E_SPACE_MISMATCH'
      and DIMS != Agent.DEFAULT_DIMS, first)
span = first['source'][first['diagnostic']['start']:first['diagnostic']['end']]
check('diagnostic points at the vector', span.startswith('[') and span.endswith(']') and span.count(',') == Agent.DEFAULT_DIMS - 1, span)
check('repaired after one diagnostic', attempts == 1 and agent.dims == DIMS)
check('repaired query runs', ran, result)
hits = [n['embedding_id'] for n in result[0]['result']['neighbors']] if 'neighbors' in result[0]['result'] else None
query = embed(question, DIMS)
oracle = sorted(discovered[GRAPH]['embeddings'].values(), key=lambda e: (cosine_distance(query, e['vector']), e['id']))
check('hits equal an independent ranking of visible embeddings', hits == [e['id'] for e in oracle[:3]], (hits, result[0]['result'].get('neighbors')))

attempts, (ran, written) = agent.record_finding(graph_id, 'bearing family shows recurring noise', manifest, manifest_path)
second = agent.log[-2]
check('implicit public finding rejected', not second['ok'] and second['diagnostic']['code'] == 'E_VISIBILITY_REQUIRED')
check('finding repaired with a manifest group and written', attempts == 1 and ran, written)


def lens_nodes(actor):
    ok, plan = compile_source(f'use G graph {json.dumps(GRAPH)}; lens All from G {{ }}')
    ok, out = engine_cli(['run', '--db', db, '--actor', actor], plan)
    return out[0]['result']


owner_view, outsider_view = lens_nodes(OWNER), lens_nodes(OUTSIDER)
check('owner (a group member) sees the finding', any(n['id'] == 'finding-1' for n in owner_view['graph']['nodes']))
check('non-member reader does not see the finding', all(n['id'] != 'finding-1' for n in outsider_view['graph']['nodes']))
stamp = [f['provenance'] for f in owner_view['fact_provenance'] if f['fact'].get('node_id') == 'finding-1']
check('finding stamped with the delegation chain', stamp and stamp[0]['writer'] == AGENT and stamp[0]['on_behalf_of'] == [OWNER], stamp)

# Guesses are rejected at compile time with the engine's codes.
ok, diagnostic = agent.compile(f'use G graph {json.dumps(HIDDEN)}; lens L from G {{ }}', manifest_path)
check('hidden graph rejected at compile time', not ok and diagnostic['code'] == 'E_UNAVAILABLE', diagnostic)
ok, diagnostic = compile_source(f'write W graph {json.dumps(GRAPH)} {{ assert node "x" entity "X" space "s" reader_groups {json.dumps(OTHER)}; }}',
                                '--untrusted', '--visible', manifest_path)
check('foreign group rejected at compile time', not ok and diagnostic['code'] == 'E_UNAVAILABLE', diagnostic)
ok, plan = compile_source(f'write W graph {json.dumps(GRAPH)} {{ assert node "x" entity "X" space "s" reader_groups {json.dumps(OTHER)}; }}')
ok, error = agent.run(plan)
check('engine rejects the same over-reach without a manifest', not ok and error['code'] == 'E_UNAVAILABLE', error)

report = {'profile': 'weave-language-lb1-agent-repair/1', 'seed': seed, 'dimensions': DIMS, 'checks': len(checks),
          'transcript': [{'ok': e['ok'], 'code': e['diagnostic'] and e['diagnostic']['code']} for e in agent.log],
          'status': 'passed'}
if a.report:
    a.report.write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps(report))
