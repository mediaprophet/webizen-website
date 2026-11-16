"""Exercise the real Synth HTTP contract; no substitute producer or fixtures."""
from pathlib import Path
import base64
import hashlib
import json
import sys
import urllib.error
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
BASE = sys.argv[1] if len(sys.argv) > 1 else 'http://127.0.0.1:8802'
OUT = ROOT / 'evidence/aaa-production-http'
OUT.mkdir(parents=True, exist_ok=True)

def request(path, body=None, headers=None):
    req = urllib.request.Request(BASE + path, data=body, headers=headers or {})
    try:
        with urllib.request.urlopen(req, timeout=180) as response:
            return response.status, dict(response.headers), response.read()
    except urllib.error.HTTPError as error:
        return error.code, dict(error.headers), error.read()

def produce(source):
    status, _, raw = request('/api/produce', json.dumps({
        'source': source, 'profile': 'files', 'include_html': False,
        'include_resources': True, 'include_package': True,
    }).encode(), {'Content-Type': 'application/json'})
    assert status == 200, status
    result = json.loads(raw)
    assert result['ok'], result
    assert result['strict_replay_verified'] and 'cards_html' not in result
    for resource in result['resources']:
        data = base64.b64decode(resource['base64'], validate=True)
        assert len(data) == resource['byte_length']
        assert hashlib.sha256(data).hexdigest() == resource['sha256']
    package = base64.b64decode(result['package_base64'], validate=True)
    assert len(package) == result['package_byte_length']
    assert hashlib.sha256(package).hexdigest() == result['package_sha256']
    return result, package

checks = []
status, _, raw = request('/api/capabilities')
capabilities = json.loads(raw)
assert status == 200 and capabilities['morphisms']['count'] == 150
assert len(capabilities['morphisms']['morphisms']) == 150
for route, mime in [('/aaa-instanced-proof.html', 'text/html'),
                    ('/atlas/', 'text/html'),
                    ('/pkg/native-world/webizen_world_bg.wasm', 'application/wasm')]:
    status, headers, data = request(route)
    assert status == 200 and mime in headers['Content-Type'], (route, status, headers)
    assert headers['Cross-Origin-Opener-Policy'] == 'same-origin'
    assert headers['Cross-Origin-Embedder-Policy'] == 'require-corp'
    assert headers['Cross-Origin-Resource-Policy'] == 'same-origin'
    checks.append({'route': route, 'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()})
status, headers, part = request('/pkg/native-world/webizen_world_bg.wasm', headers={'Range': 'bytes=0-7'})
assert status == 206 and part == b'\x00asm\x01\x00\x00\x00'
for route in ['/world/%2e%2e/%2e%2e/Cargo.toml', '/.git/config', '/data/acquisition/Timeline.json']:
    status, _, _ = request(route)
    assert status == 404, (route, status)

catalogue = json.loads((ROOT / 'data/publications/aaa-production/catalogue.json').read_text(encoding='utf-8'))
results = []
for recipe in catalogue['recipes']:
    source = recipe['source']
    for param in recipe['parameters']:
        source = source.replace('__' + param['key'] + '__', json.dumps(param['default']))
    first, package = produce(source)
    repeated, _ = produce(source)
    assert first['package_sha256'] == repeated['package_sha256']
    assert len(first['resources']) == {'catchment': 2, 'lettering': 1, 'tone': 1, 'surface': 3}[recipe['id']]
    param = recipe['parameters'][0]
    changed_value = 'Our living world' if param['kind'] == 'text' else param['default'] + param.get('step', 1)
    changed_source = recipe['source']
    for p in recipe['parameters']:
        value = changed_value if p['key'] == param['key'] else p['default']
        changed_source = changed_source.replace('__' + p['key'] + '__', json.dumps(value))
    changed, _ = produce(changed_source)
    assert changed['package_sha256'] != first['package_sha256']
    assert [r['sha256'] for r in changed['resources']] != [r['sha256'] for r in first['resources']]
    (OUT / (recipe['id'] + '.hmc')).write_bytes(package)
    result = {k: v for k, v in first.items() if k not in ('resources', 'package_base64')}
    result['recipe'] = recipe['id']
    result['resources'] = [{k: v for k, v in r.items() if k != 'base64'} for r in first['resources']]
    result['changed_package_sha256'] = changed['package_sha256']
    results.append(result)
    print(json.dumps({'recipe': recipe['id'], 'replay': True, 'changed_pixels_or_audio': True}), flush=True)

status, _, raw = request('/api/produce', json.dumps({'source': 'this is not valid VibeScript'}).encode(), {'Content-Type': 'application/json'})
refusal = json.loads(raw)
assert status == 200 and refusal['ok'] is False and refusal.get('error')
report = {'schema': 'webizen.native-production-http/v1', 'base': BASE, 'static_checks': checks,
          'native_catalogue_morphisms': 150, 'range_and_private_path_checks': True,
          'recipes': results, 'invalid_source_refused': True}
(OUT / 'results.json').write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
print('Native production HTTP checks passed', flush=True)
