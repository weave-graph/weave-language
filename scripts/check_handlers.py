#!/usr/bin/env python3
"""Actual source artifact -> trusted host handler acceptance; never builds tools."""
import argparse
import json
from pathlib import Path
import subprocess
import tempfile
import hashlib
import copy
import sqlite3
from contextlib import nullcontext

p = argparse.ArgumentParser()
p.add_argument('--compiler', type=Path, required=True)
p.add_argument('--host', type=Path, required=True)
p.add_argument('--work-dir', type=Path, help='retain diagnostic files in a new empty directory')
a = p.parse_args()
if a.work_dir:
    a.work_dir.mkdir(exist_ok=False)
compiler, host_binary = str(a.compiler.resolve()), str(a.host.resolve())
root = Path(__file__).resolve().parents[1]
with (nullcontext(str(a.work_dir)) if a.work_dir else tempfile.TemporaryDirectory(prefix='weave-compiled-handlers-')) as directory:
    work = Path(directory)
    db = work / 'store.db'
    count = 0
    processes = {"compiler": 0, "native": 0, "controlled_deaths": 0}
    def file(value, suffix='.json'):
        global count
        count += 1
        target = work / f'input-{count}{suffix}'
        target.write_text(value if isinstance(value, str) else json.dumps(value))
        return target
    def invoke(argv, error=None):
        processes["compiler" if str(argv[0]) == compiler else "native"] += 1
        result = subprocess.run([str(x) for x in argv], capture_output=True, text=True)
        if error:
            assert result.returncode and not result.stdout, (argv, result.stdout, result.stderr)
            assert json.loads(result.stderr)['code'] == error, result.stderr
            return None
        assert result.returncode == 0, (argv, result.stdout, result.stderr)
        return json.loads(result.stdout)
    def host(mode, error=None, **fields):
        return invoke([host_binary, db, file({'mode': mode, **fields})], error)
    def compile_source(source, command='plan', extra=(), error=None):
        return invoke([compiler, command, file(source, '.weave'), *extra], error)
    def run(source, outsider=False):
        return host('run', program=compile_source(source), outsider=outsider)
    def result(source, outsider=False):
        return run(source + ' lens ReadResult from W {}', outsider)[-1]['result']

    source = (root/'examples/handlers.weave').read_text()
    prefix, declaration = source.split('handler Warning', 1)
    module = 'module "diagnostics" revision "1";\n' + prefix
    (work/'diagnostics.weave').write_text(module)
    mapping = file([{'id': 'diagnostics', 'revision': '1', 'path': 'diagnostics.weave'}])
    linked = ('import d module "diagnostics" revision "1" sha256 "' + hashlib.sha256(module.encode()).hexdigest() + '";\n'
              + 'value Answer integer_add(20,22);\nhandler Warning' + declaration.replace('using Diagnose', 'using d::Diagnose'))
    options = ['--modules', mapping]
    artifacts = compile_source(linked, 'artifacts', options)['artifacts']
    template = compile_source(linked, 'handler-plan', [*options, '--handler', 'Warning'])
    assert artifacts['handler_templates']['Warning'] == template
    assert artifacts['program']['commands'] == [] and artifacts['values']['Answer']['value'] == 42
    assert any(s['name'] == 'module:diagnostics' for s in template['source_revisions'])
    renamed = compile_source(linked.replace('import d ', 'import alias ').replace('using d::', 'using alias::'), 'handler-plan', [*options, '--handler', 'Warning'])
    assert renamed == template
    for command in ['plan', 'values', 'fingerprint', 'describe']:
        compile_source(linked, command, options, error='E_HOST_ARTIFACT_REQUIRED')
    compile_source(source.replace('graph.accepted', 'MetaGraphRebound'), 'artifacts', error='E_HANDLER_EVENT')
    setup = compile_source('''transaction initial {
      graph Quality {
        node "reading" entity "sensor-reading" space "quality";
        node "issue" entity "quality-issue" space "quality";
        edge "bad" from "reading" to "issue" relation "bad_quality" valid 0 until 100;
      }
      graph Installation {
        node "device" entity "device-17" space "operations";
        attachment "quality" on node "device" key "quality"
          graph "Quality" revision "logical:initial:Quality" valid 0 until infinity required;
      }
    }''')
    # Test-only trusted input restriction; source does not mint host authority or ACLs.
    for commit in setup['commands'][0]['commits']:
        for node in commit['data']['nodes']:
            node['readers'] = ['collector']
        for edge in commit['data']['edges']:
            edge['readers'] = ['collector']
    host('run', program=setup)
    host('install', id='diagnose', output='Warnings', template=template)
    event = host('poll', id='diagnose')
    assert event['event_type'] == 'graph.committed'
    args = dict(id='diagnose', event=event['id'], lease=event['lease'])
    host('raw', program={'version': template['protocol'], 'commands': []}, error='E_HANDLER_BOUND', **args)
    prepared = host('prepare', **args)
    assert not prepared['duplicate'] and prepared['definition_digest'] == template['definition_digest']
    repeated = host('prepare', **args)
    assert repeated == {**prepared, 'duplicate': True}
    receipt = host('complete', preparation=prepared['preparation_id'], **args)
    assert not receipt['duplicate']
    replay = host('complete', preparation=prepared['preparation_id'], **args)
    assert replay == {**receipt, 'duplicate': True}
    warning = result('use W graph "Warnings";')
    assert len(warning['graph']['edges']) == 1
    assert warning['graph']['edges'][0]['predicate'] == 'warning'
    pins = {(r['graph_id'], r['revision']) for r in warning['graph']['influence']['snapshots']}
    assert ('Installation', event['graph']['revision']) in pins and ('Quality', 'logical:initial:Quality') in pins
    assert warning['graph']['nodes'] and all(n['derived_snapshots'] for n in warning['graph']['nodes'])
    assert all(e['derived_snapshots'] for e in warning['graph']['edges'])
    attribution = [a for a in warning['graph']['attachments'] if a.get('value', {}).get('value', {}).get('format') == 'weave-handler-attribution/1']
    assert len(attribution) == 1 and attribution[0]['derived_snapshots']
    assert attribution[0]['value']['value']['template_digest'] == template['definition_digest']
    assert 'coverage' in attribution[0]['value']['value'] and 'diagnostics' in attribution[0]['value']['value']
    assert all(s in attribution[0]['value']['value']['source_revisions'] for s in template['source_revisions'])
    hidden = result('use W graph "Warnings";', outsider=True)
    assert not hidden['graph']['nodes'] and not hidden['graph']['edges'] and not hidden['graph'].get('attachments', [])
    # Copying generated records without output ACLs/envelope must retain input gates.
    detached = copy.deepcopy(warning['graph'])
    detached.pop('influence', None)
    for collection in ['nodes', 'edges', 'attachments']:
        for record in detached.get(collection, []):
            record.pop('readers', None)
    copy_plan = compile_source('graph Marker {}')
    copy_plan['commands'][0]['data'] = detached
    # A historical compiler may install its unchanged artifact, but cannot label
    # newly returned carrier fields as an old wire profile. The trusted copy uses
    # the current result's declared protocol after verifying old-profile rejection.
    if copy_plan['version'] != warning['version']:
        host('run', program=copy_plan, error='E_VERSION')
    copy_plan['version'] = warning['version']
    host('run', program=copy_plan)
    detached_hidden = result('use W graph "Marker";', outsider=True)
    assert not detached_hidden['graph']['nodes'] and not detached_hidden['graph']['edges'] and not detached_hidden['graph'].get('attachments', [])
    identity_source = r'''function Keep revision "1" (graph input) { return input; }
handler Identity revision "1" using Keep {
 input event graph "Quality" branch "main" metadata depth 0;
 on "graph.accepted", "graph.committed"; output slot "identity"; replay pinned;
}'''
    identity_template = compile_source(identity_source, 'handler-plan', ['--handler', 'Identity'])
    host('install', id='identity', output='Other', template=identity_template)
    identity_event = host('poll', id='identity')
    identity_args = dict(id='identity', event=identity_event['id'], lease=identity_event['lease'])
    identity_prepared = host('prepare', **identity_args)
    host('complete', preparation=identity_prepared['preparation_id'], **identity_args)
    identity_result = result('use W graph "Other";')
    identity_edge = identity_result['graph']['edges'][0]
    assert {'graph_id': 'Quality', 'revision': identity_event['graph']['revision'], 'assertion_id': 'bad'} in identity_edge['derived_from']
    for node in identity_result['graph']['nodes']:
        original = 'reading' if node['entity_id'] == 'sensor-reading' else 'issue'
        assert {'graph_id': 'Quality', 'revision': identity_event['graph']['revision'], 'node_id': original} in node['derived_nodes']
    assert {(n['entity_id'], n['space_id']) for n in identity_result['graph']['nodes']} == {('sensor-reading', 'quality'), ('quality-issue', 'quality')}
    # Acceptance of an already-current head is a no-op. Advance the head, handle
    # that genuine commit, then accept the original exact historical revision.
    changed = compile_source('''graph Installation {
      node "device" entity "device-17" space "operations" property "generation" 2;
      attachment "quality" on node "device" key "quality"
        graph "Quality" revision "logical:initial:Quality" valid 0 until infinity required;
    }''')
    changed['commands'][0]['expected_head'] = event['graph']['revision']
    host('run', program=changed)
    committed_again = host('poll', id='diagnose')
    assert committed_again['event_type'] == 'graph.committed'
    next_args = dict(id='diagnose', event=committed_again['id'], lease=committed_again['lease'])
    next_prepared = host('prepare', **next_args)
    host('complete', preparation=next_prepared['preparation_id'], **next_args)
    host('accept', reference=event['graph'], expected=committed_again['graph']['revision'])
    accepted = host('poll', id='diagnose')
    assert accepted['event_type'] == 'graph.accepted'
    accepted_args = dict(id='diagnose', event=accepted['id'], lease=accepted['lease'])
    accepted_prepared = host('prepare', **accepted_args)
    assert accepted_prepared['preparation_id'] != prepared['preparation_id']
    host('complete', preparation=accepted_prepared['preparation_id'], **accepted_args)
    assert len(result('use W graph "Warnings";')['graph']['edges']) == 1
    # A genuinely empty input has an exact snapshot dependency, without fabricated assertions.
    empty_source = '''function EmptyResult revision "1" (graph input) { project Nothing from input {} return Nothing; }
handler EmptyHandler revision "1" using EmptyResult {
 input event graph "Empty" branch "main" metadata depth 0;
 on "graph.accepted", "graph.committed"; output slot "empty"; replay pinned;
}'''
    empty_template = compile_source(empty_source, 'handler-plan', ['--handler', 'EmptyHandler'])
    run('graph Empty {}')
    host('install', id='empty', output='EmptyWarnings', template=empty_template)
    empty_event = host('poll', id='empty')
    empty_args = dict(id='empty', event=empty_event['id'], lease=empty_event['lease'])
    empty_prepared = host('prepare', **empty_args)
    host('complete', preparation=empty_prepared['preparation_id'], **empty_args)
    empty_result = result('use W graph "EmptyWarnings";')
    assert not empty_result['graph']['nodes'] and not empty_result['graph']['edges']
    assert empty_event['graph'] in empty_result['graph']['influence']['snapshots']
    scalar_source = empty_source.replace('project Nothing from input {}',
        'support Nothing from input relation "missing" from entity "A" space "s" to entity "B" space "s" at 5;')
    scalar_template = compile_source(scalar_source, 'handler-plan', ['--handler', 'EmptyHandler'])
    host('install', id='scalar', output='Other', template=scalar_template)
    scalar_event = host('poll', id='scalar')
    scalar_args = dict(id='scalar', event=scalar_event['id'], lease=scalar_event['lease'])
    scalar_prepared = host('prepare', **scalar_args)
    host('complete', preparation=scalar_prepared['preparation_id'], **scalar_args)
    scalar_result = result('use W graph "Other";')
    assert scalar_result['graph']['nodes'][0]['properties']['state'] == 'unknown'
    assert scalar_event['graph'] in scalar_result['graph']['nodes'][0]['derived_snapshots']
    # Real process deaths straddle each durable boundary; the SQLite inspection is
    # trusted fixture evidence only, never an engine admission/read API.
    db = work / 'recovery.db'
    run('graph Empty {}')
    host('install', id='recovery', output='EmptyWarnings', template=empty_template)
    pending = host('poll', id='recovery')
    recovery = dict(id='recovery', event=pending['id'], lease=pending['lease'])
    def killed(mode, expected, **fields):
        processes["native"] += 1
        processes["controlled_deaths"] += 1
        result = subprocess.run([host_binary, str(db), str(file({'mode': mode, **fields}))], capture_output=True, text=True)
        assert result.returncode == expected and not result.stdout, (result.returncode, result.stdout, result.stderr)
    def rows(table):
        with sqlite3.connect(db) as connection:
            return connection.execute('SELECT COUNT(*) FROM '+table).fetchone()[0]
    def body():
        with sqlite3.connect(db) as connection:
            return connection.execute('SELECT body,body_digest FROM handler_preparations WHERE adapter=?', ('recovery',)).fetchone()
    killed('prepare', 94, kill_before_commit=True, **recovery)
    assert rows('handler_preparations') == 0 and rows('handler_receipts') == 0
    killed('prepare', 92, kill_after_commit=True, **recovery)
    prepared_bytes = body()
    assert prepared_bytes and rows('handler_preparations') == 1
    recovered = host('prepare', **recovery)
    assert recovered['duplicate'] and body() == prepared_bytes
    recovery['preparation'] = recovered['preparation_id']
    before = rows('events')
    killed('complete', 94, kill_before_commit=True, **recovery)
    assert rows('handler_receipts') == 0 and rows('events') == before
    assert host('head', graph='EmptyWarnings')['head'] is None
    killed('complete', 92, kill_after_commit=True, **recovery)
    assert rows('handler_receipts') == 1 and rows('events') == before + 1
    recovered_receipt = host('complete', **recovery)
    assert recovered_receipt['duplicate'] and body() == prepared_bytes
    assert rows('events') == before + 1
    # A new lease retains preparation bytes and a captured stale CAS never rebases.
    db = work / 'stale.db'
    run('graph Empty {}')
    host('install', id='recovery', output='EmptyWarnings', template=empty_template)
    first = host('poll', id='recovery')
    first_args = dict(id='recovery', event=first['id'], lease=first['lease'])
    first_prepared = host('prepare', **first_args)
    original_bytes = body()
    renewed = host('poll', id='recovery', now=1021)
    assert renewed['id'] == first['id'] and renewed['lease'] != first['lease']
    renewed_args = dict(id='recovery', event=renewed['id'], lease=renewed['lease'], now=1021)
    renewed_prepared = host('prepare', **renewed_args)
    assert renewed_prepared == {**first_prepared, 'duplicate': True} and body() == original_bytes
    run('graph EmptyWarnings {}')
    before = rows('events')
    head = host('head', graph='EmptyWarnings')
    host('complete', preparation=renewed_prepared['preparation_id'], error='E_CONFLICT', **renewed_args)
    assert rows('handler_receipts') == 0 and rows('events') == before
    assert host('head', graph='EmptyWarnings') == head and body() == original_bytes
    print(json.dumps({
        'status': 'passed', 'profile': 'compiled-handler-source-acceptance-v1',
        'fixture_files': count, 'processes': processes,
        'verified': ['pinned_module_identity', 'complete_artifacts_and_legacy_rejection',
            'metadata_reason_recipe', 'committed_and_accepted_events', 'immutable_replay',
            'private_and_detached_record_visibility', 'identity_output_original_object_provenance', 'empty_and_scalar_snapshot_influence',
            'prepare_and_complete_crash_boundaries', 'renewed_lease_exact_preparation',
            'stale_cas_atomic_rollback', 'raw_completion_rejection'],
    }, sort_keys=True))
