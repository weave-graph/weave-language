#!/usr/bin/env python3
"""LB1 (weave-language#5) acceptance on the authenticated engine service (B3/B4).

The scripted agent repairs a query from diagnostics, as in
check_agent_repair.py, but uses only what the engine now serves and the
subprocess compiler mode a sandboxing host runs:

- the agent authenticates to `weave-service` with its own agent token;
- its visible manifest is `GET /v1/manifest` (`weave-visible/1`, graphs and
  the reader groups it may name), and the embedding spaces come from the MCP
  tool `describe_schema` (`POST /v1/mcp`);
- every source is compiled by `weave sdk` (one `weave-compiler-request/1` on
  stdin, one response on stdout) with the request `policy`
  `{untrusted, read_only, visible}`, run in an empty working directory with
  an empty environment;
- every plan runs through `POST /v1/execute` as the agent.

Graph, space, dimension, property, group and principal names are random per
run, and the agent's code names none of them. The agent:

1. reads its manifest and the space descriptors of its graphs;
2. writes a `nearest` query with its default embedding size; `weave sdk`
   reports `E_SPACE_MISMATCH` at the vector span; it re-embeds with the size
   stated in the diagnostic, compiles and runs the query; the hits equal an
   independent ranking of the embeddings it may see;
3. records a finding under a minted record ID: rejected as implicitly public
   (`E_VISIBILITY_REQUIRED`), repaired with a group from its manifest,
   visible to its owner and hidden from a non-member who can read the graph,
   with `on_behalf_of` the owner;
4. a guessed graph and a foreign group fail at compile time with
   `E_UNAVAILABLE`, and the engine rejects the same write without a manifest.
"""
import argparse, hashlib, inspect, json, math, os, random, re, socket, struct, subprocess, tempfile, time
import urllib.error, urllib.request
from pathlib import Path

p = argparse.ArgumentParser()
p.add_argument('--compiler', type=Path, required=True)
p.add_argument('--service', type=Path, required=True, help='the weave-service binary')
p.add_argument('--seed', type=int)
p.add_argument('--report', type=Path)
a = p.parse_args()
compiler, service = str(a.compiler.resolve()), str(a.service.resolve())
seed = a.seed if a.seed is not None else random.SystemRandom().randrange(1 << 30)
rng = random.Random(seed)
tmp = Path(tempfile.mkdtemp(prefix='weave-agent-service-'))
sandbox = tmp / 'empty'
sandbox.mkdir()
checks = []


def check(name, condition, detail=''):
    if not condition:
        raise SystemExit(f'FAILED {name} (seed {seed}): {detail}')
    checks.append(name)


def free_port():
    with socket.socket() as s:
        s.bind(('127.0.0.1', 0))
        return s.getsockname()[1]


PORT = free_port()
BASE = f'http://127.0.0.1:{PORT}'


def http(method, path, token, body=None):
    data = None if body is None else json.dumps(body).encode()
    request = urllib.request.Request(BASE + path, data=data, method=method, headers={
        'Authorization': f'Bearer {token}', 'content-type': 'application/json',
        'accept': 'application/json, text/event-stream'})
    try:
        with urllib.request.urlopen(request, timeout=60) as response:
            return response.status, json.loads(response.read() or b'null')
    except urllib.error.HTTPError as error:
        return error.code, json.loads(error.read() or b'null')


def weave_sdk(source, policy):
    """Compile through the subprocess mode: (exit code, response)."""
    request = {'format': 'weave-compiler-request/1', 'entry_id': 'agent', 'source': source,
               'modules': [], 'policy': policy}
    out = subprocess.run([compiler, 'sdk'], input=json.dumps(request).encode(), capture_output=True,
                         cwd=sandbox, env={}, timeout=60)
    check('weave sdk exits 0 or 1', out.returncode in (0, 1) and not out.stderr, out)
    response = json.loads(out.stdout)
    check('weave sdk exit code matches ok', (out.returncode == 0) == response['ok'])
    return response


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


# ======================= Host: a tenant with random names =======================
def token():
    return hashlib.sha256(str(rng.random()).encode()).hexdigest()[:8]


GRAPH, HIDDEN = f'kb-{token()}', f'hr-{token()}'
SPACE_ID, PROPERTY, TEAM, OTHER = f'space-{token()}', f'obs_{token()}', f'team-{token()}', f'other-{token()}'
DIMS = rng.choice([5, 6, 7, 9, 11, 12])
OWNER, AGENT, OUTSIDER = f'user-{token()}', f'agent-{token()}', f'user-{token()}'
NOTES = ['bearing vibration high replaced bearing', 'firmware update reboot loop fixed',
         'fan noise bearing worn', 'power supply ripple replaced capacitor',
         'coolant leak resealed pump', 'bearing noise lubricated', 'sensor drift recalibrated']

data_dir = tmp / 'data'
created = subprocess.run([service, 'tenant', 'create', '--data', str(data_dir), '--tenant', 't1', '--admin', 'root'],
                         capture_output=True, check=True)
ROOT = json.loads(created.stdout)['token']['token']
server = subprocess.Popen([service, 'serve', '--data', str(data_dir), '--listen', f'127.0.0.1:{PORT}',
                           '--plaintext-loopback'], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
try:
    for _ in range(600):
        try:
            if http('GET', '/v1/health', ROOT)[0] == 200:
                break
        except OSError:
            time.sleep(0.05)
    else:
        raise SystemExit('weave-service did not start')

    for graph in [GRAPH, HIDDEN]:
        status, out = http('POST', '/v1/graphs', ROOT, {'graph_id': graph})
        check('graph created', status in (200, 201), out)
    status, out = http('POST', '/v1/registry', ROOT, {'changes': [
        {'change': 'add_principal', 'id': OWNER, 'kind': 'user'},
        {'change': 'add_principal', 'id': OUTSIDER, 'kind': 'user'},
        {'change': 'add_principal', 'id': TEAM, 'kind': 'group'},
        {'change': 'add_principal', 'id': OTHER, 'kind': 'group'},
        {'change': 'add_member', 'group': TEAM, 'member': OWNER},
        {'change': 'add_member', 'group': OTHER, 'member': OUTSIDER},
        {'change': 'grant', 'graph_id': GRAPH, 'subject': TEAM, 'permission': 'write'},
        {'change': 'grant', 'graph_id': GRAPH, 'subject': OUTSIDER, 'permission': 'read'},
        {'change': 'grant', 'graph_id': HIDDEN, 'subject': OTHER, 'permission': 'write'}]})
    check('registry populated', status == 200, out)
    owner_token = http('POST', '/v1/tokens', ROOT, {'principal': OWNER})[1]['token']
    outsider_token = http('POST', '/v1/tokens', ROOT, {'principal': OUTSIDER})[1]['token']
    status, out = http('POST', '/v1/registry', owner_token, {'changes': [
        {'change': 'add_agent', 'id': AGENT, 'owner': OWNER, 'delegation': {
            'scopes': [{'graph_id': GRAPH, 'permission': 'write'}],
            'exercise': {'kind': 'only', 'identities': [TEAM]}, 'may_delegate': False}}]})
    check('agent registered by its owner', status == 200, out)
    status, out = http('POST', '/v1/tokens', owner_token, {'principal': AGENT})
    check('owner issues the agent token', status in (200, 201), out)
    agent_token = out['token']

    space = {'kind': 'embedding_space', 'id': SPACE_ID, 'revision': '1', 'encoder': 'word-hash',
             'encoder_version': '1', 'preprocessing': 'words/v1', 'dimensions': DIMS, 'metric': 'cosine_distance'}
    changes = [{'change': 'assert', 'fact': space}]
    public_embeddings = {}
    for i, text in enumerate(NOTES):
        readers = [OWNER] if i % 3 == 2 else []
        changes.append({'change': 'assert', 'fact': {'kind': 'node', 'id': f'note-{i}', 'entity_id': f'NOTE-{i}',
                        'space_id': 'notes', 'properties': {PROPERTY: text}, 'metadata': [], 'readers': readers}})
        vector = embed(text, DIMS)
        changes.append({'change': 'assert', 'fact': {'kind': 'embedding', 'id': f'emb-{i}',
                        'space': {'id': SPACE_ID, 'revision': '1'}, 'subject': {'kind': 'node', 'node_id': f'note-{i}'},
                        'vector': vector, 'valid_time': {'start': 0, 'end': None}, 'readers': readers}})
        if not readers:
            public_embeddings[f'emb-{i}'] = vector
    for graph in [GRAPH, HIDDEN]:
        status, out = http('POST', '/v1/execute', ROOT, {'version': '1.1.0', 'commands': [
            {'op': 'write', 'graph_id': graph, 'branch_id': 'main', 'changes': changes}]})
        check('host data written', status == 200, out)

    # ================== The agent: authenticated, discovery only ==================
    class Agent:
        """Holds its token and an embedder; everything else comes from the service."""
        DEFAULT_DIMS = 4

        def __init__(self, credential):
            self.credential = credential
            self.dims = self.DEFAULT_DIMS
            self.log = []

        def tool(self, name, arguments):
            status, out = http('POST', '/v1/mcp', self.credential, {
                'jsonrpc': '2.0', 'id': len(self.log) + 1, 'method': 'tools/call',
                'params': {'name': name, 'arguments': arguments}})
            return out['result']['structuredContent']['result']

        def discover(self):
            status, manifest = http('GET', '/v1/manifest', self.credential)
            for graph in manifest['graphs']:
                described = self.tool('describe_schema', {'graph_id': graph['graph_id']})
                graph['embedding_spaces'] = [s['space'] for s in described.get('embedding_spaces', [])]
                graph['property_keys'] = sorted(e['key'] for e in described.get('node_properties', []))
            self.properties = {g['graph_id']: g.pop('property_keys') for g in manifest['graphs']}
            self.manifest = manifest
            return manifest

        def compile(self, source, read_only=True):
            response = weave_sdk(source, {'untrusted': True, 'read_only': read_only, 'visible': self.manifest})
            self.log.append({'source': source, 'response': response})
            return response

        def execute(self, response):
            return http('POST', '/v1/execute', self.credential, response['artifacts']['program'])

        def similar_notes(self, question, k=3):
            graph = next(g for g in self.manifest['graphs'] if g['embedding_spaces'])
            s = graph['embedding_spaces'][0]
            declaration = (f'embedding_space Space id {json.dumps(s["id"])} revision {json.dumps(s["revision"])} '
                           f'encoder {json.dumps(s["encoder"])} version {json.dumps(s["encoder_version"])} '
                           f'preprocessing {json.dumps(s["preprocessing"])} dimensions {s["dimensions"]} '
                           f'metric {s["metric"]};\n')
            for attempt in range(3):
                vector = ', '.join(repr(x) for x in embed(question, self.dims))
                source = (declaration + f'use G graph {json.dumps(graph["graph_id"])};\n'
                          f'nearest Hits from G {{ space Space; vector [{vector}]; k {k}; }}\n')
                response = self.compile(source)
                if response['ok']:
                    return graph['graph_id'], attempt, self.execute(response)
                if response['error']['code'] == 'E_SPACE_MISMATCH':
                    self.dims = int(re.search(r'\((\d+) dimensions', response['error']['message']).group(1))
                    continue
                raise SystemExit(f'unrepairable diagnostic {response["error"]}')
            raise SystemExit('no repair within 3 attempts')

        def record_finding(self, graph_id, text):
            status, minted = http('POST', '/v1/ids', self.credential, {'count': 1})
            record = minted['ids'][0]
            key = self.properties[graph_id][0]
            fact = (f'assert node {json.dumps(record)} entity {json.dumps(record.upper())} space "findings" '
                    f'property {json.dumps(key)} {json.dumps(text)}')
            visibility = ''
            for attempt in range(3):
                response = self.compile(f'write Finding graph {json.dumps(graph_id)} {{ {fact}{visibility}; }}\n',
                                        read_only=False)
                if response['ok']:
                    return record, attempt, self.execute(response)
                if response['error']['code'] == 'E_VISIBILITY_REQUIRED':
                    visibility = ' reader_groups ' + ', '.join(json.dumps(g) for g in self.manifest['reader_groups'])
                    continue
                raise SystemExit(f'unrepairable diagnostic {response["error"]}')
            raise SystemExit('no repair within 3 attempts')

    agent = Agent(agent_token)
    manifest = agent.discover()
    check('served manifest is weave-visible/1 for the agent',
          manifest['format'] == 'weave-visible/1' and manifest['principal'] == AGENT, manifest)
    check('served manifest lists only the delegated graph', [g['graph_id'] for g in manifest['graphs']] == [GRAPH], manifest)
    check('served manifest names the delegated group', manifest['reader_groups'] == [TEAM], manifest)
    check('describe_schema gives the stored space', manifest['graphs'][0]['embedding_spaces'] ==
          [{k: v for k, v in space.items() if k != 'kind'}], manifest['graphs'][0]['embedding_spaces'])
    check('describe_schema gives the property key', agent.properties[GRAPH] == [PROPERTY], agent.properties)
    check('agent code names no host identifier', all(name not in inspect.getsource(Agent) for name in
          [GRAPH, HIDDEN, SPACE_ID, PROPERTY, TEAM, OTHER, OWNER, AGENT]))

    question = 'bearing noise and vibration'
    graph_id, attempts, (status, result) = agent.similar_notes(question)
    first = agent.log[0]
    check('first query has a type error', not first['response']['ok']
          and first['response']['error']['code'] == 'E_SPACE_MISMATCH' and DIMS != Agent.DEFAULT_DIMS, first)
    error = first['response']['error']
    span = first['source'].encode()[error['start']:error['end']].decode()
    check('diagnostic points at the vector', span.startswith('[') and span.count(',') == Agent.DEFAULT_DIMS - 1, span)
    check('repaired after one diagnostic and run', attempts == 1 and status == 200, result)
    hits = [n['embedding_id'] for n in result['results'][0]['result']['neighbors']]
    query = embed(question, DIMS)
    oracle = sorted(public_embeddings, key=lambda e: (cosine_distance(query, public_embeddings[e]), e))[:3]
    check('hits equal an independent ranking of visible embeddings', hits == oracle, (hits, oracle))

    record, attempts, (status, written) = agent.record_finding(graph_id, 'bearing family shows recurring noise')
    check('implicit public finding rejected', agent.log[-2]['response']['error']['code'] == 'E_VISIBILITY_REQUIRED')
    check('finding repaired with a manifest group and written', attempts == 1 and status == 200, written)

    def lens(credential):
        response = weave_sdk(f'use G graph {json.dumps(GRAPH)}; lens All from G {{ }}', {'read_only': True})
        status, out = http('POST', '/v1/execute', credential, response['artifacts']['program'])
        return out['results'][0]['result']

    owner_view, outsider_view = lens(owner_token), lens(outsider_token)
    check('owner (a group member) sees the finding', any(n['id'] == record for n in owner_view['graph']['nodes']))
    check('non-member reader does not see the finding', all(n['id'] != record for n in outsider_view['graph']['nodes']))
    stamp = [f['provenance'] for f in owner_view['fact_provenance'] if f['fact'].get('node_id') == record]
    check('finding stamped with the delegation chain',
          stamp and stamp[0]['writer'] == AGENT and stamp[0]['on_behalf_of'] == [OWNER], stamp)

    response = agent.compile(f'use G graph {json.dumps(HIDDEN)}; lens L from G {{ }}')
    check('hidden graph rejected at compile time', response['error']['code'] == 'E_UNAVAILABLE', response)
    foreign = (f'write W graph {json.dumps(GRAPH)} {{ assert node {json.dumps(record)} entity "X" space "s" '
               f'reader_groups {json.dumps(OTHER)}; }}')
    response = agent.compile(foreign, read_only=False)
    check('foreign group rejected at compile time', response['error']['code'] == 'E_UNAVAILABLE', response)
    response = weave_sdk(foreign, {})
    status, out = agent.execute(response)
    check('the service rejects the same over-reach without a manifest', status != 200, out)
finally:
    server.terminate()
    server.wait(timeout=30)

report = {'profile': 'weave-language-lb1-agent-service/1', 'seed': seed, 'dimensions': DIMS, 'checks': len(checks),
          'transcript': [{'ok': e['response']['ok'], 'code': None if e['response']['ok'] else e['response']['error']['code']}
                         for e in agent.log], 'status': 'passed'}
if a.report:
    a.report.write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps(report))
