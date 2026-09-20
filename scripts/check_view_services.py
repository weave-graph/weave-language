#!/usr/bin/env python3
"""Actual compiler -> explicit native host artifact/read acceptance. Never builds tools."""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

p = argparse.ArgumentParser()
p.add_argument('--compiler', type=Path, required=True)
p.add_argument('--host', type=Path, required=True)
a = p.parse_args()
compiler, host_binary = str(a.compiler.resolve()), str(a.host.resolve())
with tempfile.TemporaryDirectory(prefix='weave-view-services-') as d:
    work = Path(d)
    db = work / 'store.db'
    counter = 0
    def file(value, suffix='.json'):
        global counter
        counter += 1
        path = work / f'fixture-{counter}{suffix}'
        path.write_text(value if isinstance(value, str) else json.dumps(value))
        return path
    def invoke(argv, error=None):
        result = subprocess.run([str(x) for x in argv], text=True, capture_output=True)
        if error:
            assert result.returncode and not result.stdout, (argv, result.stdout, result.stderr)
            assert json.loads(result.stderr)['code'] == error, result.stderr
            return None
        assert result.returncode == 0, (argv, result.stderr)
        return json.loads(result.stdout)
    def host(mode, *rest, error=None):
        return invoke([host_binary, db, mode, *rest], error)
    def compile_source(source, command='plan', extra=(), error=None):
        return invoke([compiler, command, file(source, '.weave'), *extra], error)
    def run(plan, mode='run', error=None):
        return host(mode, file(plan), error=error)
    seed = host('seed')
    decision = seed['accepted']['decision_id']
    accepted_source = f'accepted Approved view "team" decision "{decision}";'
    accepted_plan = compile_source(accepted_source)
    accepted = run(accepted_plan)[0]['result']
    assert accepted == host('accepted', 'team', decision)
    assert len(accepted['graph']['edges']) == 2
    assert accepted['graph']['influence']['assertions'], 'acceptance influence required'
    empty_source = f'accepted EmptyResult view "empty" decision "{seed["empty"]["decision_id"]}";'
    empty = run(compile_source(empty_source))[0]['result']
    assert not empty['graph']['nodes'] and empty['graph']['influence']['assertions']
    run(accepted_plan, 'outsider', error='E_GOV_UNAVAILABLE')
    run(accepted_plan, 'expired', error='E_GOV_UNAVAILABLE')

    # Source module manifests reach the actual installed artifact, not only a compiler unit test.
    module = 'module "tools" revision "1"; function Keep revision "1" (graph input) { return input; }'
    (work/'tools.weave').write_text(module)
    mapping = file([{'id':'tools','revision':'1','path':'tools.weave'}])
    prefix = f'import tools module "tools" revision "1" sha256 "{hashlib.sha256(module.encode()).hexdigest()}";\n'
    source = prefix + 'value Answer integer_add(20,22); live_handle Head graph "Fleet" branch "main"; view_template Active revision "1" from Head clock tick {match relation "connected"; at 0;}'
    extra = ['--modules', mapping]
    all_artifacts = compile_source(source, 'artifacts', extra)['artifacts']
    assert all_artifacts['values']['Answer']['value'] == 42
    assert all_artifacts['program']['commands'] == []
    template = compile_source(source, 'view-plan', [*extra, '--template', 'Active'])
    assert template == all_artifacts['view_templates']['Active']
    assert any(s['name'] == 'module:tools' for s in template['source_revisions'])
    renamed = compile_source(source.replace('import tools ', 'import renamed '), 'view-plan', [*extra, '--template', 'Active'])
    assert renamed == template
    for legacy in ['plan','values','fingerprint','describe']:
        compile_source(source, legacy, extra, error='E_HOST_ARTIFACT_REQUIRED')
    compile_source('function Forbidden revision "1" (graph input) {' + accepted_source + 'return input;}', error='E_FUNCTION_EFFECT')
    snapshot = host('register', file(template), 'fleet-active', 5)
    digest = template['definition_digest']
    current_source = f'view_current Cached view "fleet-active" definition "{digest}" tick 5;'
    current_plan = compile_source(current_source)
    before_events = host('events')
    assert run(current_plan)[0]['result'] == snapshot['result']
    assert run(current_plan)[0]['result'] == host('current','fleet-active',digest,5)['result']
    assert host('events') == before_events
    assert host('current','fleet-active',digest,5)['generation'] == snapshot['generation']
    assert snapshot['result']['source_revisions'] == template['source_revisions']
    run(current_plan, 'outsider', error='E_UNAVAILABLE')
    wrong = compile_source(current_source.replace(digest,'sha256:'+'0'*64))
    run(wrong, error='E_UNAVAILABLE')

    host('enroll','fleet-active')
    host('change')
    run(current_plan, error='E_FRESHNESS')
    atomic = compile_source('graph Marker {}\n' + current_source)
    run(atomic, error='E_FRESHNESS')
    assert host('head','Marker')['revision'] is None
    refreshed = host('refresh','fleet-active',5)
    assert run(current_plan)[0]['result'] == refreshed['result']
    assert refreshed['result']['source_revisions'] == template['source_revisions']
    assert refreshed['result']['graph']['nodes'][0]['properties']['turn'] == 1
    # Historical occurrence keeps the old source even after the host accepts a new one.
    host('publish')
    assert run(accepted_plan)[0]['result'] == accepted
    expired_time = compile_source(current_source.replace('tick 5','tick 10'))
    run(expired_time, error='E_FRESHNESS')
    expired_result = host('refresh','fleet-active',10)
    assert expired_result['result']['graph']['edges'] == []
    assert run(expired_time)[0]['result'] == expired_result['result']
    assert expired_result['result']['source_revisions'] == template['source_revisions']

    # Pure composition and persisted copies keep actual acceptance proof obligations.
    composed = compile_source('function Keep revision "1" (graph input) { return input; }\n'+accepted_source+'\napply Copy from Keep {graph input Approved;}\nexplain Why from Copy;')
    values = run(composed)
    assert values[-1]['result']['graph']['influence']['assertions']
    saved = host('save',file(values[-1]['result']['graph']))
    saved_plan = compile_source(f'use SavedSource graph "Saved" revision "{saved["revision"]}"; lens Read from SavedSource {{}}')
    assert run(saved_plan)[0]['result']['graph']['nodes']
    denied = run(saved_plan, 'expired')[0]['result']
    assert not denied['graph']['nodes'] and denied['coverage'] == 'partial'
    # Template tampering cannot install a registration under a matching source label.
    broken = copy.deepcopy(template)
    broken['expression']['query']['predicate'] = 'other'
    host('register',file(broken),'tampered',5,error='E_SOURCE_REVISION')
print('View services CLI acceptance passed: actual accepted/empty influence, module artifact identities, explicit registration, current/stale/expiry reads, incremental refresh sources, persisted proof denial and atomic rollback')
