"""Loader artwork: marks of human creativity, authored as control points; Synth makes every path.

Outputs `site/world/loader-art.json` containing the exact SVG path data exported by
Synth's locus/glyph morphisms, with production receipt digests. The loader page only
animates these paths; it does not author geometry.
"""
from pathlib import Path
import sys, math, json, hashlib, re
ROOT = Path(__file__).resolve().parent
STAGE = ROOT.parents[1]
sys.path.insert(0, str(ROOT.parent))
from native_bridge import SynthBridge
from build_art import lit

TAU = math.tau


def hand():
    # Open palm stencil, as blown ochre around a hand on a cave wall.
    pts = [(-.62, -1.6), (-.7, -.6), (-1.32, .05), (-1.42, .32), (-1.2, .36), (-.66, -.08), (-.62, .9), (-.72, 1.7), (-.6, 1.86),
           (-.44, 1.72), (-.32, .95), (-.24, 2.02), (-.08, 2.18), (.08, 2.02), (.1, .98), (.3, 1.92), (.46, 2.04), (.6, 1.88),
           (.48, .86), (.78, 1.46), (.94, 1.52), (1.04, 1.36), (.82, .52), (.7, -.4), (.6, -1.6)]
    return [[x * 40, -y * 40] for x, y in pts]


def spiral():
    phi = (1 + 5 ** .5) / 2; pts = []
    for i in range(160):
        t = i / 159 * 3.4 * math.pi; r = 3.2 * phi ** (t / (math.pi / 2))
        pts.append([r * math.cos(t), r * math.sin(t)])
    return pts


def wave():
    return [[-90 + i * 1.5, 22 * math.sin(i * .21) * math.exp(-((i - 60) / 52) ** 2) + 9 * math.sin(i * .63)] for i in range(121)]


def helix(phase):
    return [[-80 + i * 1.6, 28 * math.sin(i * .16 + phase)] for i in range(101)]


def orbit():
    return [[78 * math.cos(i / 96 * TAU), 30 * math.sin(i / 96 * TAU)] for i in range(97)]


def wedges():
    shapes = []
    for k, (x, y, a) in enumerate([(-40, -20, 0), (-10, -24, .3), (22, -18, -.2), (-30, 16, 1.57), (4, 14, 1.2), (34, 20, 1.75)]):
        tri = [(0, -16), (6, 14), (-6, 14)]
        shapes.append([[x + px * math.cos(a) - py * math.sin(a), y + px * math.sin(a) + py * math.cos(a)] for px, py in tri])
    return shapes


def network():
    rng = [(0, 0), (52, -26), (-48, -34), (40, 38), (-56, 30), (8, -62), (-6, 64), (78, 8), (-82, -2)]
    edges = [(0, 1), (0, 2), (0, 3), (0, 4), (1, 5), (2, 5), (3, 6), (4, 6), (1, 7), (3, 7), (2, 8), (4, 8)]
    return rng, edges


MARKS = [
    {'id': 'hand', 'era': 'c. 40,000 years ago', 'title': 'A hand on the wall', 'line': 'Someone pressed a palm to stone and blew ochre around it. I was here.'},
    {'id': 'wedges', 'era': 'c. 3,200 BCE', 'title': 'Marks that remember', 'line': 'Reeds pressed into clay, and speech outlived the speaker.'},
    {'id': 'wave', 'era': 'Before history, and always', 'title': 'Song', 'line': 'Breath shaped into pattern, carried from voice to voice.'},
    {'id': 'spiral', 'era': 'c. 300 BCE', 'title': 'Proportion', 'line': 'We found the shapes inside the shell, the seed head and the storm.'},
    {'id': 'glyph', 'era': '1450', 'title': 'Moveable type', 'line': 'Letters set free to travel, and ideas began to multiply.'},
    {'id': 'orbit', 'era': '1957', 'title': 'Reaching outward', 'line': 'A small sphere circled the Earth, and the sky became a place.'},
    {'id': 'helix', 'era': '1953', 'title': 'Reading life', 'line': 'Two strands, and the grammar of every living thing.'},
    {'id': 'network', 'era': '1989', 'title': 'A web of people', 'line': 'Documents linked to documents, and minds to minds.'},
    {'id': 'world', 'era': 'Now', 'title': 'Your turn', 'line': 'A living world, made with care, waiting for what you will make.'},
]


def main():
    bridge = SynthBridge(ROOT)
    try:
        L = ['requires [capability("capability.invoke")];']; files = []; n = [0]
        def emit(key, profile_expr, fill):
            n[0] += 1; h = f'p{n[0]}'; s = f's{n[0]}'
            L.append(f'let {h}={profile_expr};')
            L.append(f'let {s}=capability.invoke("hypermedia.locus.project_svg",{{profile:{h},fill:{lit(fill)},padding:0.0}});')
            files.append(f'{{key:"{key}.svg",resource:{s}}}')
        def stroke(pts, w): return f'capability.invoke("hypermedia.locus.stroke_to_locus",{{points:{lit([[float(a), float(b)] for a, b in pts])},width:{w}}})'
        emit('hand-0', f'capability.invoke("hypermedia.locus.spline",{{points:{lit(hand())},closed:true,tolerance:0.2}})', [201, 122, 64, 255])
        for i, w in enumerate(wedges()):
            emit(f'wedges-{i}', f'capability.invoke("hypermedia.locus.construct",{{shape:"polygon",points:{lit(w)}}})', [214, 190, 140, 255])
        emit('wave-0', stroke(wave(), 3.2), [236, 214, 160, 255])
        emit('spiral-0', stroke(spiral(), 3.0), [236, 214, 160, 255])
        emit('orbit-0', stroke(orbit(), 2.4), [236, 214, 160, 255])
        emit('orbit-1', 'capability.invoke("hypermedia.locus.construct",{shape:"circle",radius:16.0,segments:40})', [120, 170, 210, 255])
        emit('helix-0', stroke(helix(0), 3.0), [140, 206, 190, 255])
        emit('helix-1', stroke(helix(math.pi), 3.0), [140, 206, 190, 255])
        for i in range(13):
            x = -80 + (i * 7.7 + 3) * 1.6; y = 28 * math.sin((i * 7.7 + 3) * .16)
            emit(f'helix-{i + 2}', stroke([(x, y), (x, -y)], 1.4), [232, 200, 120, 255])
        nodes, edges = network()
        for i, (a, b) in enumerate(edges):
            emit(f'network-{i}', stroke([nodes[a], nodes[b]], 2.0), [160, 200, 236, 255])
        for i, (x, y) in enumerate(nodes):
            L.append(f'let c{i}=capability.invoke("hypermedia.locus.construct",{{shape:"circle",radius:{7.0 if i == 0 else 4.5},segments:24}});')
            emit(f'network-{len(edges) + i}', f'capability.invoke("hypermedia.locus.transform",{{profile:c{i},matrix:[1.0,0.0,0.0,1.0,{float(x)},{float(y)}]}})', [236, 214, 160, 255])
        for key, text, size in [('glyph-0', 'A', 120.0), ('wordmark-0', 'webizen', 64.0)]:
            n[0] += 1
            L.append(f'let g{n[0]}=capability.invoke("hypermedia.glyph.project_svg",{{text:"{text}",font_size:{size},padding:0.0,stroke_width:1.0,color:[236,214,160,255],background:[0,0,0,0]}});')
            files.append(f'{{key:"{key}.svg",resource:g{n[0]}}}')
        L.append('let mut asset={files:[' + ','.join(files) + ']};')
        source = '\n'.join(L); (ROOT / 'loader-art.vibe').write_text(source, encoding='utf-8')
        result = bridge.produce(source, 'loader-art'); receipt = result['receipt']; assert receipt['strict_replay_verified']
        art = {}
        for r in sorted(receipt['resources'], key=lambda r: (r['key'].split('-')[0], int(r['key'].removesuffix('.svg').split('-')[1]))):
            svg = Path(str(r['path']).removeprefix('\\\\?\\')).read_text(encoding='utf-8')
            group = r['key'].split('-')[0]; entry = art.setdefault(group, {'d': [], 'sha256': []})
            for pts in re.findall(r'<polygon[^>]*\spoints="([^"]+)"', svg):
                coords = [c.split(',') for c in pts.split()]
                entry['d'].append('M' + 'L'.join('%.2f,%.2f' % (float(x), float(y)) for x, y in coords) + 'Z')
            entry['d'] += re.findall(r'<path[^>]*\sd="([^"]+)"', svg)
            entry['sha256'].append(r['sha256'])
            if group in ('glyph', 'wordmark'): entry['viewBox'] = re.search(r'viewBox="([^"]+)"', svg).group(1); entry['svg'] = svg
        out = {'schema': 'webizen.loader-art/v1', 'producer': 'Synth native locus/glyph morphisms via MCP produce', 'strict_replay_verified': True,
               'recipe': '/production/loader/loader-art.vibe', 'recipe_sha256': hashlib.sha256(source.encode()).hexdigest(),
               'package_sha256': receipt.get('hmc', {}).get('sha256'), 'marks': MARKS, 'art': art,
               'epistemic_class': 'authored-artwork', 'dates': 'Approximate, for an artistic sequence; not a scholarly chronology.'}
        target = STAGE / 'site/world/loader-art.json'; target.write_text(json.dumps(out, indent=1), encoding='utf-8')
        print(json.dumps({k: len(v['d']) for k, v in art.items()}))
    finally:
        bridge.close()


if __name__ == '__main__':
    main()
