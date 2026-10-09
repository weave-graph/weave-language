#!/usr/bin/env python3
"""Compile scenario B fixture variants; no build, runtime call or authority setup."""
import argparse
import ctypes
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

parser = argparse.ArgumentParser()
parser.add_argument('--compiler', type=Path, required=True)
parser.add_argument('--sdk', type=Path, required=True)
parser.add_argument('--fixtures', type=Path, default=Path(__file__).resolve().parents[1] / 'examples/native_scenario')
parser.add_argument('--report', type=Path)
args = parser.parse_args()
compiler = str(args.compiler.resolve())
library = ctypes.CDLL(str(args.sdk.resolve()))


def exported(name, count):
    function = getattr(library, 'weave_compiler_' + name)
    function.argtypes = [ctypes.c_uint32] * count
    function.restype = ctypes.c_int32
    return function


new, write, compile_, length, read, drop = [exported(name, count) for name, count in [
    ('input_new', 1), ('input_write', 3), ('compile', 1), ('output_len', 1), ('output_read', 2), ('drop', 1)]]
assert exported('abi_version', 0)() == 1


def sdk(source, modules, entry):
    request = json.dumps({'format': 'weave-compiler-request/1', 'entry_id': entry,
                          'source': source, 'modules': modules}, ensure_ascii=False).encode('utf-8')
    handle = new(len(request))
    assert handle > 0
    output = 0
    try:
        for offset in range(0, len(request), 2):
            assert write(handle, int.from_bytes(request[offset:offset + 2], 'little'), min(2, len(request) - offset)) == 0
        output = compile_(handle)
        handle = 0
        assert output > 0
        size = length(output)
        assert 0 <= size <= 16 * 1024 * 1024 + 4096
        result = bytearray(size)
        for offset in range(0, size, 2):
            word = read(output, offset)
            assert 0 <= word <= 65535
            result[offset:offset + 2] = word.to_bytes(2, 'little')[:min(2, size - offset)]
        raw = bytes(result)
        return request, raw, json.loads(raw)
    finally:
        if output:
            assert drop(output) == 0
        if handle:
            drop(handle)


def text(path):
    return path.read_bytes().decode('utf-8')


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


reports = []
with tempfile.TemporaryDirectory(prefix='weave-scenario-sources-') as temporary:
    work = Path(temporary)
    for manifest_path in sorted(args.fixtures.glob('*/manifest.json')):
        directory = manifest_path.parent
        manifest = json.loads(manifest_path.read_bytes())
        assert manifest['format'] == 'weave-native-scenario-fixture/1'
        modules = []
        for module in manifest['modules']:
            raw = (directory / module['file']).read_bytes()
            assert sha(raw) == module['sha256']
            modules.append({'id': module['id'], 'revision': module['revision'], 'source': raw.decode('utf-8')})
        # Deliberately exercise quotes, backslashes and Unicode in opaque exact pins.
        pins = {'EVIDENCE_REVISION_JSON': 'exact-evidence:"λ\\pin',
                'INSTALLATION_REVISION_JSON': 'exact-installation:"λ\\pin',
                'WARNING_REVISION_JSON': 'exact-warning:"λ\\pin'}
        artifacts = {}
        evidence = {}
        for kind in ['seed', 'handler', 'offline', 'cluster']:
            source = text(directory / manifest['files'][kind])
            for token in manifest.get('substitutions', {}).get(kind, []):
                placeholder = manifest['tokens'][token]
                assert source.count(placeholder) == 1
                source = source.replace(placeholder, json.dumps(pins[token], ensure_ascii=False))
            assert '{{' not in source
            source_path = work / f'{manifest["variant"]}-{kind}.weave'
            source_path.write_bytes(source.encode('utf-8'))
            argv = [compiler, 'artifacts', str(source_path)]
            selected_modules = modules if kind == 'handler' else []
            if selected_modules:
                argv += ['--modules', str((directory / manifest['files']['modules']).resolve())]
            cli = json.loads(subprocess.check_output(argv))
            request, raw, response = sdk(source, selected_modules, manifest['variant'] + '/' + kind)
            assert response['ok'], response
            assert response['artifacts'] == cli['artifacts']
            assert response['artifact_fingerprint'] == cli['artifact_fingerprint']
            artifacts[kind] = response['artifacts']
            evidence[kind] = {'request_sha256': sha(request), 'response_sha256': sha(raw),
                              'artifact_fingerprint': response['artifact_fingerprint']}
        graphs = manifest['graphs']
        seed = artifacts['seed']['program']['commands']
        assert len(seed) == 1 and seed[0]['op'] == 'commit_batch'
        seed_graphs = {c['graph_id']: c for c in seed[0]['commits']}
        assert set(seed_graphs) == {graphs['evidence'], graphs['installation']}
        assert all(e['polarity'] == 'positive' for e in seed_graphs[graphs['evidence']]['data']['edges'])
        installation = seed_graphs[graphs['installation']]['data']
        assert next(n for n in installation['nodes'] if n['id'] == 'device')['properties']['serial'] == 9007199254740993
        assert len({n['space_id'] for n in installation['nodes'] if n['entity_id'] == 'device-17'}) == 2
        binding = installation['attachments'][0]
        assert binding['host'] == {'kind': 'edge', 'id': manifest['selection']['connection_id']}
        assert binding['value'] == {'kind': 'graph', 'reference': {'graph_id': graphs['evidence'], 'revision': 'logical:seed:' + graphs['evidence']}}
        handler = artifacts['handler']
        assert handler['program']['commands'] == []
        assert set(handler['handler_templates']) == set(manifest['handlers'].values())
        assert handler['values']['ExactCounter']['value'] == 9007199254740993
        assert handler['values']['Label']['value'] == 'λ scenario 🚀'
        selected = handler['handler_templates'][manifest['handlers']['selected']]
        assert selected['event_types'] == ['graph.accepted', 'graph.committed']
        assert selected['input']['graph_id'] == graphs['installation']
        assert selected['output_slot'] == manifest['output_slot']
        expressions = [b['value'] for b in selected['recipe']['bindings']]
        reason = next(e for e in expressions if e['kind'] == 'reason')
        rule = reason['rules']['rules'][0]
        assert rule['body'][0]['polarity'] == 'negative'
        assert rule['head']['polarity'] == 'positive' and rule['head']['predicate'] == manifest['warning_predicate']
        assert all(e['kind'] in ['reference', 'metadata', 'reason', 'filter'] for e in expressions)
        assert any(e['kind'] == 'filter' and e['valid_at'] == 7 for e in expressions)
        offline = artifacts['offline']['program']['commands']
        assert len(offline) == 1 and offline[0]['op'] == 'commit_batch'
        replacements = {c['graph_id']: c for c in offline[0]['commits']}
        assert replacements[graphs['evidence']]['expected_head'] == pins['EVIDENCE_REVISION_JSON']
        assert replacements[graphs['installation']]['expected_head'] == pins['INSTALLATION_REVISION_JSON']
        assertions = replacements[graphs['evidence']]['data']['edges']
        assert {e['polarity'] for e in assertions} == {'positive', 'negative'}
        negative = next(e for e in assertions if e['polarity'] == 'negative')
        assert negative['valid_time'] == {'start': 5, 'end': 20}
        rebound = replacements[graphs['installation']]['data']['attachments'][0]
        assert rebound['value']['reference'] == {'graph_id': graphs['evidence'], 'revision': 'logical:offline:' + graphs['evidence']}
        cluster = artifacts['cluster']['program']['commands']
        assert len(cluster) == 1 and cluster[0]['op'] == 'bind'
        assert cluster[0]['name'] == manifest['selection']['cluster_binding']
        selection = cluster[0]['value']['selection']
        assert cluster[0]['value']['kind'] == 'cluster'
        assert selection['source'] == {'graph_id': graphs['warnings'], 'revision': pins['WARNING_REVISION_JSON']}
        assert selection['predicate'] == manifest['warning_predicate'] and selection['valid_at'] == 7
        # Changed module bytes must fail against the original embedded content pin.
        altered = [{**m, 'source': m['source'] + '// changed bytes\n'} for m in modules]
        _, _, rejected = sdk(text(directory / manifest['files']['handler']), altered, 'bad-pin')
        assert not rejected['ok'] and rejected['error']['code'] == 'E_MODULE_DIGEST', rejected
        reports.append({'variant': manifest['variant'], 'module_sha256': manifest['modules'][0]['sha256'],
                        'definition_digest': selected['definition_digest'], 'compilations': evidence})
assert len(reports) == 2
assert reports[0]['definition_digest'] != reports[1]['definition_digest']
report = {'profile': 'native-scenario-source-fixtures/1', 'status': 'passed', 'variants': reports,
          'native_engine_executed': False, 'sdk_requests': 10, 'cli_compilations': 8}
if args.report:
    args.report.write_bytes((json.dumps(report, indent=2) + '\n').encode('utf-8'))
print(json.dumps(report))
