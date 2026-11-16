"""Lush eucalyptus prototypes. Python authors control rings; Synth generates every mesh.

Two authored variants share one skeleton generator:
- near: articulated twigs, thousands of individually realised pendulous leaves and
  inner foliage clumps for crown density.
- mid:  the same limb architecture with foliage clumps at every twig, built for
  instanced distant woodland (no QEM decimation, so foliage is not stripped).
"""
from pathlib import Path
import sys, math, random, json, hashlib, time, subprocess
ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT.parent))
from native_bridge import SynthBridge
from build_art import lit, tube, loft, parse_dxf

TAU = math.tau
PAL = {
    'pale-bark': {'color': '#c3c6b4', 'roughness': .9, 'metalness': 0},
    'warm-bark': {'color': '#cdb89a', 'roughness': .92, 'metalness': 0},
    'old-bark': {'color': '#6f6a55', 'roughness': .98, 'metalness': 0},
    'twig': {'color': '#7d7558', 'roughness': .95, 'metalness': 0},
    'leaf-olive': {'color': '#3f6343', 'roughness': .8, 'metalness': 0},
    'leaf-blue': {'color': '#4d7468', 'roughness': .78, 'metalness': 0},
    'leaf-young': {'color': '#7a9460', 'roughness': .82, 'metalness': 0},
    'leaf-deep': {'color': '#2f4f36', 'roughness': .84, 'metalness': 0},
}
LEAF_MATS = ['leaf-olive', 'leaf-blue', 'leaf-young', 'leaf-deep']


def lerp(a, b, t): return [a[i] * (1 - t) + b[i] * t for i in range(3)]


def onpath(path, t):
    q = t * (len(path) - 1); i = min(int(q), len(path) - 2); return lerp(path[i], path[i + 1], q - i)


def centre(y): return [.05 * y + .2 * math.sin(y * .6), y, .07 * y + .12 * math.sin(y * .8)]


def clump_rings(seed, sides):
    """Irregular flattened foliage mass: three jittered rings, capped."""
    rng = random.Random(seed); rings = []
    twist = rng.uniform(0, TAU)
    for j, (y, r) in enumerate([(-.62, .42), (-.22, .98), (.18, .9), (.5, .38)]):
        ring = []
        for k in range(sides):
            a = twist + k * TAU / sides + j * .2 + rng.uniform(-.2, .2); rr = r * rng.uniform(.7, 1.25)
            ring.append([rr * math.cos(a), y + rng.uniform(-.1, .1), rr * math.sin(a)])
        rings.append(ring)
    return rings


def leaf_rings(angle):
    u = [math.cos(angle), 0, math.sin(angle)]; w = [-math.sin(angle), 0, math.cos(angle)]; rings = []
    for t, width in [(0, .003), (.4, .045), (1, .001)]:
        p = [u[0] * (.08 * math.sin(math.pi * t) + .12 * t), -.36 * t, u[2] * (.08 * math.sin(math.pi * t) + .12 * t)]
        fold = .008 * math.sin(math.pi * t)
        rings.append([[p[0] - w[0] * width, p[1], p[2] - w[2] * width], [p[0], p[1] + fold, p[2]],
                      [p[0] + w[0] * width, p[1], p[2] + w[2] * width], [p[0], p[1] - fold - .001, p[2]]])
    return rings


def tree(variant, seed=4471):
    rng = random.Random(seed); near = variant == 'near'; parts = []
    leafsets = [[] for _ in range(12)]; leafscales = [[] for _ in range(12)]
    clumpsets = [[] for _ in range(6)]; clumpscales = [[] for _ in range(6)]
    twigsets = [[] for _ in range(12)]; twigscales = [[] for _ in range(12)]

    def branch(name, path, radii, mat, sides):
        p = loft(name, tube(path, radii, sides), mat); p['args']['samples'] = 2 if near else 1
        p['args']['interpolation'] = 'cubic' if near else 'linear'; parts.append(p)

    def clump(pos, size, flat=.82):
        b = rng.randrange(6); clumpsets[b].append(pos)
        clumpscales[b].append([size * rng.uniform(.85, 1.15), size * flat * rng.uniform(.8, 1.2), size * rng.uniform(.85, 1.15)])

    trunk = [centre(y) for y in [0, .3, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12.2, 13.4]]
    branch('bole', trunk, [.46, .42, .35, .31, .28, .25, .22, .2, .18, .15, .12, .09, .06, .03, .006], 'pale-bark', 14 if near else 8)
    for k in range(6):
        a = k * TAU / 6 + .1
        branch('root-' + str(k), [[.26 * math.cos(a), .24, .26 * math.sin(a)], [.6 * math.cos(a), .11, .6 * math.sin(a)], [.95 * math.cos(a), .015, .95 * math.sin(a)]], [.12, .07, .01], 'old-bark', 8 if near else 5)
    if near:
        for k in range(9):
            a = k * TAU / 9; top = 1.2 + rng.random() * 2.7; path = []
            for j in range(7):
                y = .08 + top * j / 6; c = centre(y); r = .43 - (.43 - .28) * min(y / 3, 1); aa = a + .09 * math.sin(j * .7 + k)
                path.append([c[0] + r * math.cos(aa), y, c[2] + r * math.sin(aa)])
            branch('bark-strip-' + str(k), path, [(.015, .045 + .025 * math.sin(j)) for j in range(7)], 'old-bark', 6)
    limbs = 11
    for k in range(limbs):
        a = k * 2.39996 + .18; sh = 3.8 + (k % 5) * .78; start = centre(sh); extent = 3.2 + 1.6 * rng.random()
        tip = [start[0] + math.cos(a) * extent, 10.2 + 3.4 * rng.random(), start[2] + math.sin(a) * extent]; path = []
        for j in range(9):
            t = j / 8; p = lerp(start, tip, t); p[0] += .35 * math.sin(math.pi * t) * math.sin(a + 1); p[2] += .42 * math.sin(math.pi * t) * math.cos(a + .5); p[1] += .55 * math.sin(math.pi * t); path.append(p)
        branch('limb-' + str(k), path, [.19 * (1 - j / 8) ** 1.25 + .008 for j in range(9)], 'warm-bark' if k % 3 == 0 else 'pale-bark', 10 if near else 6)
        clump(onpath(path, 1.0), (1.25 + .4 * rng.random()) * (.7 if near else 1))
        for j in range(6 if near else 5):
            t = .36 + j * (.12 if near else .14); origin = onpath(path, t); az = a + (-1 if j % 2 else 1) * (.6 + rng.random() * .8); span = 1.2 + rng.random() * 1.5
            end = [origin[0] + span * math.cos(az), origin[1] + .8 + rng.random() * .9, origin[2] + span * math.sin(az)]; sec = []
            for q in range(5):
                u = q / 4; p = lerp(origin, end, u); p[1] += .35 * math.sin(math.pi * u); sec.append(p)
            branch('secondary-%d-%d' % (k, j), sec, [.058, .046, .033, .02, .004], 'pale-bark', 7 if near else 4)
            clump(end, (.95 + .45 * rng.random()) * (.7 if near else 1))
            twigs = 4 if near else 3
            for tw in range(twigs):
                point = onpath(sec, .42 + tw * (.18 if near else .24)); az2 = az + (tw - (twigs - 1) / 2) * .9; span2 = .75 + rng.random() * .65; twig = []
                for q in range(5):
                    u = q / 4; twig.append([point[0] + span2 * math.cos(az2) * u, point[1] + .27 * math.sin(math.pi * u) - .32 * u, point[2] + span2 * math.sin(az2) * u])
                if near:
                    tb = int((az2 % TAU) / TAU * 12) % 12; twigsets[tb].append(point); twigscales[tb].append([span2, span2, span2])
                    clump(onpath(twig, .7), .42 + .2 * rng.random(), .7)
                    for node in range(10):
                        pos = onpath(twig, .08 + node * .095)
                        for side in [-1, 1]:
                            angle = az2 + side * (.7 + .25 * rng.random()); b = int((angle % TAU) / TAU * 12) % 12
                            off = .025 + rng.random() * .05
                            leafsets[b].append([pos[0] + off * math.cos(angle), pos[1] + .02 * rng.random(), pos[2] + off * math.sin(angle)])
                            s = .8 + rng.random() * .7; leafscales[b].append([s, s, s])
                else:
                    clump(onpath(twig, .75), .85 + .4 * rng.random())
    sides = 7 if near else 5
    for b in range(12):
        if not twigsets[b]: continue
        az = (b + .5) * TAU / 12; unit = [[math.cos(az) * u, .27 * math.sin(math.pi * u) - .32 * u, math.sin(az) * u] for u in [0, .25, .5, .75, 1]]
        p = loft('twigs-' + str(b), tube(unit, [.022, .017, .012, .007, .002], 5), 'twig'); p['args'].update(interpolation='linear', samples=1)
        p['points'] = twigsets[b]; p['scales'] = twigscales[b]; parts.append(p)
    for b in range(6):
        if not clumpsets[b]: continue
        p = loft('clump-' + str(b), clump_rings(seed * 7 + b, sides), LEAF_MATS[b % 4]); p['args'].update(interpolation='linear', samples=1)
        p['points'] = clumpsets[b]; p['scales'] = clumpscales[b]; parts.append(p)
    for b in range(12):
        if not leafsets[b]: continue
        p = loft('leaves-' + str(b), leaf_rings(b * TAU / 12), LEAF_MATS[b % 3], caps=False); p['args'].update(interpolation='linear', samples=1)
        p['points'] = leafsets[b]; p['scales'] = leafscales[b]; parts.append(p)
    stats = {'leaves': sum(len(x) for x in leafsets), 'clumps': sum(len(x) for x in clumpsets)}
    return parts, stats


def recipe(parts):
    lines = ['requires [capability("capability.invoke")];']; files = []
    for i, p in enumerate(parts):
        lines.append(f'let m{i}=capability.invoke("{p["op"]}",{lit(p["args"])});'); name = f'm{i}'
        if 'points' in p:
            lines.append(f'let ins{i}=capability.invoke("hypermedia.manifold.instance_on_points",{{prototype:m{i},points:{lit(p["points"])},scales:{lit(p["scales"])}}});')
            lines.append(f'let real{i}=capability.invoke("hypermedia.manifold.realize_instances",{{instances:ins{i}}});'); name = f'real{i}'
        lines.append(f'let d{i}=capability.invoke("hypermedia.manifold.project_dxf",{{mesh:{name},layer:"{p["material"]}"}});'); files.append(f'{{key:"{p["name"]}.dxf",resource:d{i}}}')
    lines.append('let mut asset={files:[' + ','.join(files) + ']};')
    return '\n'.join(lines)


def build(variant, bridge, preview=False):
    for d in ['recipes', 'assets', 'previews']: (ROOT / d).mkdir(exist_ok=True)
    parts, stats = tree(variant); source = recipe(parts); name = 'eucalyptus-lush-' + variant
    (ROOT / 'recipes' / (name + '.vibe')).write_bytes(source.encode())
    start = time.time(); native = bridge.produce(source, name); assert native['receipt']['strict_replay_verified']
    resources = {r['key']: r for r in native['receipt']['resources']}; batches = {}
    for p in parts:
        r = resources[p['name'] + '.dxf']; positions, indices = parse_dxf(Path(r['path']))
        assert hashlib.sha256(Path(r['path']).read_bytes()).hexdigest() == r['sha256']
        b = batches.setdefault(p['material'], {'name': p['material'], 'material': p['material'], 'positions': [], 'indices': [], 'source_parts': []})
        off = len(b['positions']) // 3; b['positions'] += positions; b['indices'] += [v + off for v in indices]
        b['source_parts'].append({'name': p['name'], 'source_sha256': r['sha256'], 'triangles': len(indices) // 3})
    allpos = [x for b in batches.values() for x in b['positions']]
    bounds = {'min': [min(allpos[j::3]) for j in range(3)], 'max': [max(allpos[j::3]) for j in range(3)]}
    tris = sum(len(b['indices']) // 3 for b in batches.values())
    mesh = {'schema': 'webizen.native-mesh-adapter/v1', 'id': name, 'units': 'metres', 'up': '+Y', 'palette': {k: PAL[k] for k in batches},
            'parts': list(batches.values()), 'bounds': bounds, 'triangles': tris, 'stats': stats,
            'recipe': '/production/lush-woodland/recipes/' + name + '.vibe', 'source_sha256': hashlib.sha256(source.encode()).hexdigest(),
            'production_receipt': native['receipt'].get('hmc', {}).get('sha256'), 'strict_replay_verified': True,
            'epistemic_class': 'authored-artwork', 'note': 'Authored eucalypt form with clumped crown foliage; not a species-exact botanical reconstruction.'}
    out = ROOT / 'assets' / (name + '.json'); out.write_text(json.dumps(mesh, separators=(',', ':')))
    print('LUSH', name, tris, stats, round(time.time() - start, 1), 's', flush=True)
    if preview:
        obj = ROOT / 'recipes' / (name + '.obj'); verts = []; faces = []
        for b in batches.values():
            base = len(verts) // 3; verts += b['positions']; faces += [v + base for v in b['indices']]
        obj.write_text('\n'.join(['v %g %g %g' % tuple(verts[i:i + 3]) for i in range(0, len(verts), 3)] + ['f %d %d %d' % (faces[i] + 1, faces[i + 1] + 1, faces[i + 2] + 1) for i in range(0, len(faces), 3)]))
        seal = 'requires [capability("capability.invoke")];\nlet g=capability.invoke("hypermedia.manifold.import_mesh",{input:"tree.obj",format:"obj"});\nlet n=capability.invoke("hypermedia.manifold.compile_10d_pbr",{mesh:g,topology:false,spatial_index:false});\nlet mut asset={files:[{key:"' + name + '.10d",resource:n}]};'
        sealed = bridge.produce(seal, name + '-preview', [{'key': 'tree.obj', 'path': str(obj), 'kind': 'obj'}])
        model = next(r for r in sealed['receipt']['resources'] if r['key'].endswith('.10d'))
        target = ROOT / 'previews' / (name + '-' + str(int(time.time())))
        subprocess.run(['C:/github/synth/target-prod/debug/synth.exe', 'preview', '--asset', model['path'], '--out', str(target)], check=True)
        print('PREVIEW', target, flush=True)
    return mesh


if __name__ == '__main__':
    bridge = SynthBridge(ROOT)
    try:
        for v in (sys.argv[1:] or ['mid', 'near']):
            build(v.replace('--preview', '').strip() or 'mid', bridge, preview=True) if not v.startswith('--') else None
    finally:
        bridge.close()
