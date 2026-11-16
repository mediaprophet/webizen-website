"""Assemble the lush homepage world: denser woodland, fuller crowns, richer understorey.

- Tree prototypes come from build_trees.py (Synth-authored, no QEM foliage stripping).
- Candidate positions are authored here; native Vibe (`vibe_eval`) applies the existing
  ecology exclusions (camp, sight-lines, river, viewpoints), a tree line and the
  current `terrain_height`, exactly as the earlier ecology pass did.
- Prototypes are compiled with the shared native GLB -> compile_10d_pbr path,
  attested, and material-namespaced before entering the shared scene.
"""
from pathlib import Path
import sys, json, math, random, hashlib, importlib.util
ROOT = Path(__file__).resolve().parent
PROD = ROOT.parent; STAGE = PROD.parent
sys.path.insert(0, str(PROD / 'aaa-environment')); sys.path.insert(0, str(PROD)); sys.path.insert(0, str(STAGE / 'tooling'))
import assemble
from native_bridge import SynthBridge
from build_art import lit
spec = importlib.util.spec_from_file_location('namespace', PROD / 'aaa-proof/materials/namespace.py')
namespace = importlib.util.module_from_spec(spec); spec.loader.exec_module(namespace)
for d in ['inputs', 'recipes', 'groups', 'provenance', 'textures', 'artifacts', 'assets']: (ROOT / d).mkdir(exist_ok=True)
assemble.ROOT = ROOT; assemble.URL = '/production/lush-woodland/'


def read(p): return json.loads(Path(p).read_text(encoding='utf8'))
def sha(p): return hashlib.sha256(Path(p).read_bytes()).hexdigest()


def density(x, z):
    """Smooth woodland patches and clearings (0..1)."""
    v = .5 + .22 * math.sin(x * .061 + 1.3) * math.cos(z * .052 - .4) + .18 * math.sin((x + z) * .037 + 2.1) + .1 * math.cos(x * .13 - z * .09)
    return max(0, min(1, v))


def candidates():
    rng = random.Random(20261116); c = []
    def add(asset, x, z, r, s=1, yaw=0, offset=0, maxh=99.0):
        c.append({'asset': asset, 'id': 'lush-' + asset + '-' + str(len(c)), 'x': round(x, 4), 'z': round(z, 4), 'radius': r, 'scale': round(s, 4), 'yaw': round(yaw, 4), 'offset': offset, 'maxh': maxh})
    # Legacy camp trees, now rendered with the lush near/mid prototypes.
    for x, z, yaw, s in [(-35, 19, .3, .95), (-24, 11, 1.7, 1.05), (-10, 16, 2.4, .85), (-38, 32, 3.2, 1.0), (-35, 44, 4.4, .8), (-18, 4, 5.2, .9)]:
        add('lush-near', x, z, 2.5, s, yaw)
    for x, z, yaw, s in [(-50, -55, 0, .7), (-50, -5, .74, .94), (-50, 50, 1.48, .7), (-75, -55, 3.7, .94), (-75, -5, 4.44, .7), (-75, 50, 5.18, .94),
                         (35, -60, 0, .7), (35, -10, .74, .94), (35, 50, 1.48, .7), (70, -60, 3.7, .94), (70, -10, 4.44, .7), (70, 50, 5.18, .94)]:
        add('lush-mid', x, z, 3, s + .15, yaw, 0, 26.0)
    # Woodland: patchy density field over the whole catchment.
    for gx in range(-86, 87, 7):
        for gz in range(-86, 87, 7):
            x = gx + rng.uniform(-2.8, 2.8); z = gz + rng.uniform(-2.8, 2.8)
            if rng.random() < .18 + .75 * density(x, z):
                add('lush-mid', x, z, 3.2, .72 + .5 * rng.random(), rng.random() * math.tau, 0, 26.0)
    # A few more close, full-detail trees framing the camp and the bridge.
    for x, z in [(-44, 24), (-29, 52), (-6, 26), (-46, 8), (8, 12)]:
        add('lush-near', x + rng.uniform(-1.5, 1.5), z + rng.uniform(-1.5, 1.5), 2.5, .85 + .2 * rng.random(), rng.random() * math.tau)
    # Richer understorey around the woodland patches.
    for k in range(90):
        x = rng.uniform(-70, 60); z = rng.uniform(-50, 70)
        if rng.random() < .3 + .6 * density(x, z):
            for a in ['shrub-branches', 'shrub-leaves']: add(a, x, z, 1, 1, k * .7, -.02, 24.0)
    for k in range(260):
        x = rng.uniform(-60, 45); z = rng.uniform(-40, 66)
        add('grass-mid' if k % 5 else 'grass-near', x, z, .3, rng.uniform(.5, 1.0), rng.random() * math.tau, -.015, 24.0)
    for k in range(40): add('leaf-litter', rng.uniform(-55, 30), rng.uniform(-10, 60), .3, rng.uniform(.8, 1.3), rng.random() * math.tau, -.002, 24.0)
    return c


VIBE = '''pure fn ecology_clear(x:f64,z:f64,r:f64) -> bool {
 let camp=math.sqrt((x+22.0)*(x+22.0)+(z-28.0)*(z-28.0));
 if camp < 10.5+r { return false; }
 if x > -21.0-r && x < -5.0+r && z > 32.0-r && z < 55.0+r { return false; }
 if math.abs(z - 4.0) < 3.0 + r && math.abs(x - river_x(4.0)) < 10.0 + r { return false; }
 for stop in [[-17.0,6.0],[-20.0,-16.0],[20.0,-7.0],[-15.0,-32.0],[17.0,-37.0]] {
  if math.sqrt((x-stop[0])*(x-stop[0])+(z-stop[1])*(z-stop[1])) < 5.0+r { return false; }
 }
 if math.abs(x) > 88.0 || math.abs(z) > 88.0 { return false; }
 for v in [[0.0,16.0,12.0,30.0],[-20.0,-16.0,-10.0,-4.0],[-17.0,6.0,-8.0,15.0],[20.0,-7.0,30.0,4.0],[-15.0,-32.0,-4.0,-19.0],[17.0,-37.0,26.0,-27.0],[-22.0,28.0,-32.0,44.0]] {
  let ex=v[2]-v[0]; let ez=v[3]-v[1];
  let t=math.max(0.0,math.min(1.0,((x-v[0])*ex+(z-v[1])*ez)/(ex*ex+ez*ez)));
  let px=v[0]+ex*t-x; let pz=v[1]+ez*t-z;
  if r >= 2.0 && math.sqrt(px*px+pz*pz) < 5.0+r { return false; }
 }
 return true;
}
pure fn ecology_placements(candidates: List) budget(steps:2000000) -> List {
 let mut result=[];
 for p in candidates {
  let d=math.abs(p.x-river_x(p.z));
  let h=terrain_height(p.x,p.z);
  if ecology_clear(p.x,p.z,p.radius) && d > 3.8+p.radius && h < p.maxh {
   result=result.push({asset:p.asset,id:p.id,translation:[p.x,h+p.offset,p.z],rotation:[0.0,math.sin(p.yaw/2.0),0.0,math.cos(p.yaw/2.0)],scale:[p.scale,p.scale,p.scale]});
  }
 }
 return result;
}
'''


def placements(bridge):
    c = candidates(); (ROOT / 'inputs/candidates.json').write_text(json.dumps(c))
    (ROOT / 'recipes/lush-placement.vibe').write_text(VIBE)
    world = (STAGE / 'site/world/living-world.vibe').read_text(encoding='utf-8-sig').split('pure fn view(')[0]
    items = []; chunks = []
    for off in range(0, len(c), 40):
        src = world + '\n' + VIBE + '\nlet ecology=ecology_placements(' + lit(c[off:off + 40]) + ');'
        res = bridge.call('vibe_eval', {'source': src}); items += res['bindings']['ecology']
        chunks.append({'source_sha256': hashlib.sha256(src.encode()).hexdigest(), 'input_count': len(c[off:off + 40]), 'output_count': len(res['bindings']['ecology'])})
    out = {'schema': 'webizen.native-placement-evaluation/v1', 'epistemic_class': 'authored-artwork', 'frame': 'Existing fictional catchment, Y-up metres',
           'recipe_sha256': sha(ROOT / 'recipes/lush-placement.vibe'), 'evaluator': 'Native Poet/Vibe via synth vibe_eval', 'chunks': chunks, 'placements': items}
    (ROOT / 'assets/placement-evaluation.json').write_text(json.dumps(out, indent=1))
    return items


def main():
    bridge = SynthBridge(ROOT)
    try:
        protos = {}
        for key in ['lush-mid', 'lush-near']:
            data = read(ROOT / 'assets' / ('eucalyptus-' + key + '.json'))
            rec = assemble.compile_group(bridge, 'prototype-eucalyptus-' + key, data['parts'], data['palette'], [{'id': key, 'translation': [0, 0, 0], 'rotation': [0, 0, 0, 1], 'scale': [1, 1, 1]}], None)
            output = STAGE / rec['url'].lstrip('/')
            rec = namespace.namespace_asset(rec, output, PROD / 'aaa-proof/assets' / (rec['id'] + '-' + sha(output)[:12] + '.10d'))
            rec = {k: rec[k] for k in ['id', 'url', 'sha256', 'bytes', 'vertices', 'triangles', 'colour', 'material_namespace_receipt', 'provenance_receipt', 'recipe']}
            rec['tree_source'] = '/production/lush-woodland/assets/eucalyptus-' + key + '.json'; protos[key] = rec
        items = placements(bridge)
    finally:
        bridge.close()
    base = read(PROD / 'aaa-instancing/scene.json')
    drop = lambda a: a['id'].startswith('eucalyptus') or a['id'].startswith('prototype-eucalyptus')
    removed = [a['id'] for a in base['assets'] if drop(a)]
    base['assets'] = [a for a in base['assets'] if not drop(a)]
    byid = {a['id']: a for a in base['assets']}
    counts = {}
    for p in items:
        key = p['asset']; counts[key] = counts.get(key, 0) + 1
        target = protos[key] if key in protos else byid['prototype-' + key]
        target.setdefault('placements', []).append({'id': p['id'], 'translation': p['translation'], 'rotation': p['rotation'], 'scale': p['scale']})
    for key in ['lush-near', 'lush-mid']: base['assets'].append(protos[key])
    for a in base['assets']:
        f = STAGE / a['url'].lstrip('/'); assert sha(f) == a['sha256'] and f.stat().st_size == a['bytes'], a['id']
    base['triangle_count'] = sum(a['triangles'] for a in base['assets'])
    base['vertex_count'] = sum(a['vertices'] for a in base['assets'])
    base['expanded_triangle_count'] = sum(a['triangles'] * max(1, len(a.get('placements', []))) for a in base['assets'])
    eco = sum(len(a.get('placements', [])) for a in base['assets'])
    base['totals'] = {'assets': len(base['assets']), 'bytes': sum(a['bytes'] for a in base['assets']), 'vertices': base['vertex_count'], 'triangles': base['triangle_count'],
                      'vertex_count': base['vertex_count'], 'triangle_count': base['triangle_count'], 'expanded_triangles': base['expanded_triangle_count'],
                      'ecology_placements': eco, 'native_instance_records': sum(max(1, len(a.get('placements', []))) for a in base['assets'])}
    base['woodland_lod'] = {'near_prototype': protos['lush-near']['id'], 'mid_prototype': protos['lush-mid']['id'], 'near_instances': counts.get('lush-near', 0),
                            'mid_instances': counts.get('lush-mid', 0), 'method': 'Synth-authored crown variants; clumped foliage at every twig; no QEM foliage stripping'}
    base['lush_world'] = {'schema': 'webizen.lush-world/v1', 'removed_baked_assets': removed, 'placement_counts': counts,
                          'placements': '/production/lush-woodland/assets/placement-evaluation.json', 'production_status': 'Lush homepage world; visual acceptance pending'}
    base['production_status'] = 'Lush homepage world; visual acceptance pending'
    assert base['totals']['native_instance_records'] <= 10000
    (ROOT / 'scene.json').write_text(json.dumps(base, indent=1))
    print(json.dumps({'counts': counts, 'totals': base['totals']}, indent=1))


if __name__ == '__main__':
    main()
