"""Build multi-LOD and flora rhizosphere/aboveground .10d packs for the 13 verified SE Australian species.

Uses Webizen AI Synth CLI (`synth.exe produce`) to import, simplify (`mode="optimize"`),
plane-bisect (for flora rhizosphere/aboveground), and compile `.10d` containers, then transfers
authored `SurfaceReading` vertex colors from each species' canonical `dist/<slug>.10d` with
canonical section ordering and CRC-32C sealing.
"""
from __future__ import annotations

import argparse
import collections
import hashlib
import json
import math
import struct
import subprocess
import time
import uuid
from pathlib import Path

SYNTH_EXE = Path(r"C:\Users\Hp\AppData\Local\Programs\Webizen AI Synth\synth.exe")
ECO_INTENT_ROOT = Path(r"C:\github\eco-intent")
SYNTH_PROD_ROOT = Path(r"C:\github\synth\production")
WEBIZEN_ROOT = Path(r"C:\github\webizen-website")
COHORT_DIR = WEBIZEN_ROOT / "production" / "species-cohort"

FLORA_SPECIES = (
    "eucalyptus-camaldulensis",
    "eucalyptus-dalrympleana",
    "eucalyptus-regnans",
    "eucalyptus-melliodora",
    "banksia-integrifolia",
    "acacia-pycnantha",
    "dicksonia-antarctica",
)

FAUNA_SPECIES = (
    "wallabia-bicolor",
    "phascolarctos-cinereus",
    "vombatus-ursinus",
    "macropus-giganteus",
    "ornithorhynchus-anatinus",
    "dacelo-novaeguineae",
)

ALL_SPECIES = FLORA_SPECIES + FAUNA_SPECIES

SECTION_NAMES = {
    1: "QuantizedMesh",
    2: "Tensor10DNodes",
    3: "Reconstruction",
    7: "ProvenanceSidecar",
    8: "FieldSidecar",
    9: "CorrespondenceMap",
    10: "Topology",
    11: "SpatialIndex",
    12: "SurfaceReading",
    13: "Materials",
    14: "TextureCoordinates",
    15: "ManifoldIdentityV2",
}

ALIGN_BYTES = {0: 1, 1: 4, 2: 16, 3: 64}


def _build_crc32c_table() -> list[int]:
    table = []
    for i in range(256):
        c = i
        for _ in range(8):
            c = (c >> 1) ^ (0x82F63B78 if (c & 1) else 0)
        table.append(c)
    return table


CRC32C_TABLE = _build_crc32c_table()


def crc32c(data: bytes) -> int:
    """Compute reflected CRC-32C (Castagnoli) matching QualiaDB container_10d."""
    crc = 0xFFFFFFFF
    for b in data:
        crc = CRC32C_TABLE[(crc ^ b) & 0xFF] ^ (crc >> 8)
    return crc ^ 0xFFFFFFFF


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def parse_10d(data: bytes) -> tuple[bytearray, list[tuple]]:
    """Parse header and section records from a `.10d` container."""
    magic, ver, _ = struct.unpack_from("<4sHH", data, 0)
    if magic != b"10d\0" or ver not in (1, 2):
        raise ValueError(f"Invalid .10d header: magic={magic!r} ver={ver}")
    sec_off, sec_cnt = struct.unpack_from("<II", data, 56)
    sections = []
    for i in range(sec_cnt):
        o = sec_off + i * 24
        stype, align, res16, boff, blen, stride, elem_cnt, scrc = struct.unpack_from(
            "<BBHIIIII", data, o
        )
        sections.append(
            (stype, align, res16, boff, blen, stride, elem_cnt, scrc, data[boff : boff + blen])
        )
    return bytearray(data[:64]), sections


def decode_quantized_mesh(payload: bytes) -> dict:
    """Decode QuantizedMesh mini-header, vertex positions, and triangle indices."""
    mflags, _, nv, nt, minx, miny, minz, maxx, maxy, maxz, _ = struct.unpack_from(
        "<HHIIffffffI", payload, 0
    )
    u16_idx = (mflags & 1) != 0
    sx = (maxx - minx) / 65535.0
    sy = (maxy - miny) / 65535.0
    sz = (maxz - minz) / 65535.0
    verts = []
    v_off = 40
    for i in range(nv):
        qx, qy, qz = struct.unpack_from("<HHH", payload, v_off + i * 6)
        verts.append((minx + qx * sx, miny + qy * sy, minz + qz * sz))
    i_off = v_off + nv * 6
    fmt = "<HHH" if u16_idx else "<III"
    istride = 6 if u16_idx else 12
    tris = [struct.unpack_from(fmt, payload, i_off + t * istride) for t in range(nt)]
    return {
        "vertex_count": nv,
        "triangle_count": nt,
        "bounds_min": [minx, miny, minz],
        "bounds_max": [maxx, maxy, maxz],
        "vertices": verts,
        "triangles": tris,
    }


def inspect_10d_summary(path: Path) -> dict:
    """Read exact byte size, SHA-256, vertex/triangle counts, bounds, and sections of a `.10d`."""
    data = path.read_bytes()
    _, secs = parse_10d(data)
    qmesh = decode_quantized_mesh(next(s[8] for s in secs if s[0] == 1))
    return {
        "bytes": len(data),
        "sha256": sha256_bytes(data),
        "vertices": qmesh["vertex_count"],
        "triangles": qmesh["triangle_count"],
        "bounds_min": qmesh["bounds_min"],
        "bounds_max": qmesh["bounds_max"],
        "sections": [SECTION_NAMES.get(s[0], f"Section({s[0]})") for s in secs],
    }


def write_obj_from_10d(src_10d: Path, out_obj: Path) -> dict:
    """Decode `src_10d` QuantizedMesh to a geometry-only OBJ (`v`/`f` only) for Synth import."""
    data = src_10d.read_bytes()
    _, secs = parse_10d(data)
    qmesh = decode_quantized_mesh(next(s[8] for s in secs if s[0] == 1))
    lines = [f"v {x:.6f} {y:.6f} {z:.6f}\n" for (x, y, z) in qmesh["vertices"]]
    lines.extend(f"f {a + 1} {b + 1} {c + 1}\n" for (a, b, c) in qmesh["triangles"])
    out_obj.write_text("".join(lines), encoding="utf-8")
    return qmesh


def transfer_surface_reading(
    target_pts: list[tuple[float, float, float]],
    ref_pts: list[tuple[float, float, float]],
    ref_srd_payload: bytes,
) -> bytes:
    """Build an SRD1 SurfaceReading section via fast spatial-grid nearest-vertex transfer."""
    nv_t = len(target_pts)
    out = bytearray(8 + nv_t * 4)
    out[0:4] = b"SRD1"
    struct.pack_into("<I", out, 4, nv_t)
    if nv_t == len(ref_pts) and nv_t > 0 and target_pts[0] == ref_pts[0] and target_pts[-1] == ref_pts[-1]:
        out[8:] = ref_srd_payload[8 : 8 + nv_t * 4]
        return bytes(out)

    xs = [p[0] for p in ref_pts]
    ys = [p[1] for p in ref_pts]
    zs = [p[2] for p in ref_pts]
    span = max(max(xs) - min(xs), max(ys) - min(ys), max(zs) - min(zs), 1e-3)
    cell = max(span / 40.0, 0.01)
    inv = 1.0 / cell
    grid: dict[tuple[int, int, int], list[int]] = {}
    for idx, (x, y, z) in enumerate(ref_pts):
        key = (math.floor(x * inv), math.floor(y * inv), math.floor(z * inv))
        grid.setdefault(key, []).append(idx)

    for i, (tx, ty, tz) in enumerate(target_pts):
        cx, cy, cz = math.floor(tx * inv), math.floor(ty * inv), math.floor(tz * inv)
        best_d2 = 1e30
        best_j = 0
        found = False
        for r in range(0, 5):
            for dx in range(-r, r + 1):
                for dy in range(-r, r + 1):
                    for dz in range(-r, r + 1):
                        if r > 0 and max(abs(dx), abs(dy), abs(dz)) < r:
                            continue
                        bucket = grid.get((cx + dx, cy + dy, cz + dz))
                        if not bucket:
                            continue
                        for j in bucket:
                            rx, ry, rz = ref_pts[j]
                            d2 = (tx - rx) ** 2 + (ty - ry) ** 2 + (tz - rz) ** 2
                            if d2 < best_d2:
                                best_d2 = d2
                                best_j = j
                                found = True
            if found:
                break
        if not found:
            for j, (rx, ry, rz) in enumerate(ref_pts):
                d2 = (tx - rx) ** 2 + (ty - ry) ** 2 + (tz - rz) ** 2
                if d2 < best_d2:
                    best_d2 = d2
                    best_j = j
        out[8 + i * 4 : 8 + (i + 1) * 4] = ref_srd_payload[8 + best_j * 4 : 8 + (best_j + 1) * 4]
    return bytes(out)


def encode_10d(header: bytearray, sections: list[tuple]) -> bytes:
    """Encode a canonical `.10d` container with ascending section types and CRC-32C seals."""
    secs_sorted = sorted(sections, key=lambda s: s[0])
    sec_off = 64
    cursor = sec_off + len(secs_sorted) * 24
    descs = bytearray()
    body = bytearray()
    for stype, align, res16, stride, elem_cnt, payload in secs_sorted:
        ab = ALIGN_BYTES[align]
        pad = (ab - (cursor % ab)) % ab
        body.extend(b"\x00" * pad)
        cursor += pad
        boff = cursor
        descs.extend(
            struct.pack(
                "<BBHIIIII",
                stype,
                align,
                res16,
                boff,
                len(payload),
                stride,
                elem_cnt,
                crc32c(payload),
            )
        )
        body.extend(payload)
        cursor += len(payload)
    struct.pack_into("<I", header, 52, 0)
    struct.pack_into("<II", header, 56, sec_off, len(secs_sorted))
    full = bytearray(header) + descs + body
    struct.pack_into("<I", full, 52, crc32c(bytes(full)))
    return bytes(full)


def build_vibe_source(slug: str, is_flora: bool) -> str:
    """Generate the VibeScript LOD production recipe for a species."""
    if is_flora:
        return (
            'requires [capability("capability.invoke")];\n'
            'let w = capability.invoke("hypermedia.manifold.import_mesh", { input: "wood.obj", format: "obj" });\n'
            'let w_lod0 = capability.invoke("hypermedia.manifold.simplify", { mesh: w, target_triangles: 15000, mode: "optimize", attribute_policy: "discard" });\n'
            'let w_lod1 = capability.invoke("hypermedia.manifold.simplify", { mesh: w_lod0, target_triangles: 3800, mode: "optimize" });\n'
            'let w_lod2 = capability.invoke("hypermedia.manifold.simplify", { mesh: w_lod1, target_triangles: 1050, mode: "optimize" });\n'
            'let w_abv_raw = capability.invoke("hypermedia.manifold.bisect_plane", { mesh: w_lod0, plane_origin: [0.0, -0.30, 0.0], plane_normal: [0.0, 1.0, 0.0], clear_inner: true, clear_outer: false, fill_cap: false });\n'
            'let w_abv_s = capability.invoke("hypermedia.manifold.simplify", { mesh: w_abv_raw, target_triangles: 4500, mode: "optimize" });\n'
            'let w_abv = capability.invoke("hypermedia.manifold.bisect_plane", { mesh: w_abv_s, plane_origin: [0.0, -0.30, 0.0], plane_normal: [0.0, 1.0, 0.0], clear_inner: true, clear_outer: false, fill_cap: false });\n'
            'let rhz_m = capability.invoke("hypermedia.manifold.bisect_plane", { mesh: w_lod0, plane_origin: [0.0, 0.15, 0.0], plane_normal: [0.0, 1.0, 0.0], clear_inner: false, clear_outer: true, fill_cap: false });\n'
            'let f = capability.invoke("hypermedia.manifold.import_mesh", { input: "canopy.obj", format: "obj" });\n'
            'let f_lod0 = capability.invoke("hypermedia.manifold.simplify", { mesh: f, target_triangles: 50000, mode: "decimate", attribute_policy: "discard" });\n'
            'let w0_10d = capability.invoke("hypermedia.manifold.compile_10d_pbr", { mesh: w_lod0, topology: false, spatial_index: false });\n'
            'let w1_10d = capability.invoke("hypermedia.manifold.compile_10d_pbr", { mesh: w_lod1, topology: false, spatial_index: false });\n'
            'let w2_10d = capability.invoke("hypermedia.manifold.compile_10d_pbr", { mesh: w_lod2, topology: false, spatial_index: false });\n'
            'let wa_10d = capability.invoke("hypermedia.manifold.compile_10d_pbr", { mesh: w_abv, topology: false, spatial_index: false });\n'
            'let f0_10d = capability.invoke("hypermedia.manifold.compile_10d_pbr", { mesh: f_lod0, topology: false, spatial_index: false });\n'
            'let rhz_10d = capability.invoke("hypermedia.manifold.compile_10d_pbr", { mesh: rhz_m, topology: false, spatial_index: false });\n'
            'let mut asset = { files: [\n'
            '    { key: "w_lod0.10d", resource: w0_10d },\n'
            '    { key: "w_lod1.10d", resource: w1_10d },\n'
            '    { key: "w_lod2.10d", resource: w2_10d },\n'
            '    { key: "w_abv.10d", resource: wa_10d },\n'
            '    { key: "f_lod0.10d", resource: f0_10d },\n'
            f'    {{ key: "{slug}-rhizosphere.10d", resource: rhz_10d }}\n'
            ']};\n'
        )
    return f'''requires [capability("capability.invoke")];
let m = capability.invoke("hypermedia.manifold.import_mesh", {{ input: "source.obj", format: "obj" }});
let lod1_m = capability.invoke("hypermedia.manifold.simplify", {{ mesh: m, target_triangles: 7200, mode: "optimize", attribute_policy: "discard" }});
let lod2_m = capability.invoke("hypermedia.manifold.simplify", {{ mesh: lod1_m, target_triangles: 2200, mode: "optimize" }});
let lod0_10d = capability.invoke("hypermedia.manifold.compile_10d_pbr", {{ mesh: m, topology: false, spatial_index: false }});
let lod1_10d = capability.invoke("hypermedia.manifold.compile_10d_pbr", {{ mesh: lod1_m, topology: false, spatial_index: false }});
let lod2_10d = capability.invoke("hypermedia.manifold.compile_10d_pbr", {{ mesh: lod2_m, topology: false, spatial_index: false }});
let mut asset = {{ files: [
    {{ key: "{slug}-lod0.10d", resource: lod0_10d }},
    {{ key: "{slug}-lod1.10d", resource: lod1_10d }},
    {{ key: "{slug}-lod2.10d", resource: lod2_10d }}
]}};
'''


def write_pruned_obj_from_10d(src_10d: Path, out_obj: Path, max_components: int) -> tuple[int, int]:
    """Keep the largest connected components by triangle count/extent for far-field LOD2."""
    data = src_10d.read_bytes()
    _, secs = parse_10d(data)
    qmesh = decode_quantized_mesh(next(s[8] for s in secs if s[0] == 1))
    verts = qmesh["vertices"]
    tris = qmesh["triangles"]
    nv = len(verts)

    parent = list(range(nv))

    def find(x: int) -> int:
        while parent[x] != x:
            parent[x] = parent[parent[x]]
            x = parent[x]
        return x

    for a, b, c in tris:
        ra, rb, rc = find(a), find(b), find(c)
        if ra != rb:
            parent[rb] = ra
        if ra != rc:
            parent[rc] = ra

    comp_tris: dict[int, list[tuple[int, int, int]]] = {}
    comp_area: dict[int, float] = {}
    for a, b, c in tris:
        r = find(a)
        comp_tris.setdefault(r, []).append((a, b, c))
        ax, ay, az = verts[a]
        bx, by, bz = verts[b]
        cx, cy, cz = verts[c]
        ux, uy, uz = bx - ax, by - ay, bz - az
        vx, vy, vz = cx - ax, cy - ay, cz - az
        nx = uy * vz - uz * vy
        ny = uz * vx - ux * vz
        nz = ux * vy - uy * vx
        area = 0.5 * math.sqrt(nx * nx + ny * ny + nz * nz)
        comp_area[r] = comp_area.get(r, 0.0) + area

    ranked = sorted(comp_tris.keys(), key=lambda r: (comp_area[r], len(comp_tris[r])), reverse=True)
    keep_roots = set(ranked[:max_components])

    kept_tris = [t for r in ranked[:max_components] for t in comp_tris[r]]
    used_verts = sorted({v for t in kept_tris for v in t})
    remap = {old: new for new, old in enumerate(used_verts)}
    lines = [f"v {verts[v][0]:.6f} {verts[v][1]:.6f} {verts[v][2]:.6f}\n" for v in used_verts]
    lines.extend(f"f {remap[a] + 1} {remap[b] + 1} {remap[c] + 1}\n" for a, b, c in kept_tris)
    out_obj.write_text("".join(lines), encoding="utf-8")
    return len(used_verts), len(kept_tris)


def attach_colors_and_save(
    raw_10d: Path,
    dest_10d: Path,
    ref_pts: list[tuple[float, float, float]],
    ref_srd: bytes | None,
) -> dict:
    """Attach nearest-vertex SurfaceReading colors (if available) and save sealed `.10d`."""
    raw = raw_10d.read_bytes()
    hdr, secs = parse_10d(raw)
    if ref_srd is not None:
        qmesh = decode_quantized_mesh(next(s[8] for s in secs if s[0] == 1))
        srd_payload = transfer_surface_reading(qmesh["vertices"], ref_pts, ref_srd)
        new_secs = [(s[0], s[1], s[2], s[5], s[6], s[8]) for s in secs if s[0] != 12]
        new_secs.append((12, 2, 0, 0, 0, srd_payload))
        sealed = encode_10d(hdr, new_secs)
    else:
        sealed = raw
    dest_10d.parent.mkdir(parents=True, exist_ok=True)
    dest_10d.write_bytes(sealed)
    return inspect_10d_summary(dest_10d)


def validate_lod_budgets(slug: str, is_flora: bool, lods: dict[str, dict]) -> None:
    """Assert WebGL2 mobile triangle budgets for all emitted LOD packs."""
    if is_flora:
        assert 40000 <= lods["lod0"]["triangles"] <= 80000, (slug, lods["lod0"]["triangles"])
        assert lods["lod1"]["triangles"] <= 18000, (slug, lods["lod1"]["triangles"])
        assert lods["lod2"]["triangles"] <= 4000, (slug, lods["lod2"]["triangles"])
        assert lods["aboveground_lod1"]["triangles"] <= 25000, (slug, lods["aboveground_lod1"]["triangles"])
        assert 1 <= lods["rhizosphere"]["triangles"] <= 18000, (slug, lods["rhizosphere"]["triangles"])
        assert lods["aboveground_lod1"]["bounds_min"][1] >= -0.32, (slug, lods["aboveground_lod1"]["bounds_min"])
        assert lods["rhizosphere"]["bounds_max"][1] <= 0.20, (slug, lods["rhizosphere"]["bounds_max"])
    else:
        assert 12000 <= lods["lod0"]["triangles"] <= 48000, (slug, lods["lod0"]["triangles"])
        assert lods["lod1"]["triangles"] <= 8000, (slug, lods["lod1"]["triangles"])
        assert lods["lod2"]["triangles"] <= 2500, (slug, lods["lod2"]["triangles"])


def _vec_sub(a: tuple[float, float, float], b: tuple[float, float, float]) -> tuple[float, float, float]:
    return (a[0] - b[0], a[1] - b[1], a[2] - b[2])


def _vec_add(a: tuple[float, float, float], b: tuple[float, float, float]) -> tuple[float, float, float]:
    return (a[0] + b[0], a[1] + b[1], a[2] + b[2])


def _vec_mul(a: tuple[float, float, float], s: float) -> tuple[float, float, float]:
    return (a[0] * s, a[1] * s, a[2] * s)


def _vec_dot(a: tuple[float, float, float], b: tuple[float, float, float]) -> float:
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]


def _vec_cross(a: tuple[float, float, float], b: tuple[float, float, float]) -> tuple[float, float, float]:
    return (
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    )


def _vec_norm(a: tuple[float, float, float]) -> tuple[float, float, float]:
    ln = math.sqrt(a[0] * a[0] + a[1] * a[1] + a[2] * a[2])
    return (a[0] / ln, a[1] / ln, a[2] / ln) if ln > 1e-8 else (0.0, 1.0, 0.0)


def _write_obj(pts: list[tuple[float, float, float]], tris: list[tuple[int, int, int]], out_path: Path) -> list[int]:
    """Write a geometry-only OBJ from indexed triangles and return the sorted kept original vertex indices."""
    used = sorted({i for t in tris for i in t})
    remap = {old: new for new, old in enumerate(used)}
    lines = [f"v {pts[i][0]:.6f} {pts[i][1]:.6f} {pts[i][2]:.6f}\n" for i in used]
    lines.extend(f"f {remap[a] + 1} {remap[b] + 1} {remap[c] + 1}\n" for a, b, c in tris)
    out_path.write_text("".join(lines), encoding="utf-8")
    return used


def _classify_flora_components(
    pts: list[tuple[float, float, float]],
    tris: list[tuple[int, int, int]],
    srd_bytes: bytes,
) -> dict:
    """Split canonical flora triangles into green leaf clusters, blossom clusters, and wood/roots."""
    nv = len(pts)
    adj: list[list[int]] = [[] for _ in range(nv)]
    for i0, i1, i2 in tris:
        adj[i0].extend((i1, i2))
        adj[i1].extend((i0, i2))
        adj[i2].extend((i0, i1))
    comp = [-1] * nv
    cid = 0
    for s in range(nv):
        if comp[s] != -1:
            continue
        q = [s]
        comp[s] = cid
        while q:
            u = q.pop()
            for v in adj[u]:
                if comp[v] == -1:
                    comp[v] = cid
                    q.append(v)
        cid += 1

    comp_tris: dict[int, list[tuple[int, int, int]]] = collections.defaultdict(list)
    fol_votes: collections.Counter[int] = collections.Counter()
    bloom_votes: collections.Counter[int] = collections.Counter()
    comp_ymin: dict[int, float] = {}
    comp_ymax: dict[int, float] = {}
    for t in tris:
        c = comp[t[0]]
        comp_tris[c].append(t)
        ys = (pts[t[0]][1], pts[t[1]][1], pts[t[2]][1])
        comp_ymin[c] = min(comp_ymin.get(c, 1e9), min(ys))
        comp_ymax[c] = max(comp_ymax.get(c, -1e9), max(ys))
        r, g, bl = srd_bytes[t[0] * 4 : t[0] * 4 + 3]
        if g > r + 6 and g > bl + 3:
            fol_votes[c] += 1
        elif r > 180 and g > 140 and (min(r, g) - bl) >= 95:
            bloom_votes[c] += 1

    fol_cids: list[int] = []
    bloom_cids: list[int] = []
    wood_cids: list[int] = []
    for c, ts in comp_tris.items():
        if comp_ymax[c] >= -0.30 and fol_votes[c] > len(ts) * 0.5:
            fol_cids.append(c)
        elif comp_ymin[c] >= 0.60 and len(ts) <= 240 and bloom_votes[c] > len(ts) * 0.5:
            bloom_cids.append(c)
        else:
            wood_cids.append(c)

    return {
        "comp_tris": comp_tris,
        "fol_cids": fol_cids,
        "bloom_cids": bloom_cids,
        "wood_cids": wood_cids,
    }


def _stratified_canopy_sample(
    cids: list[int],
    comp_tris: dict[int, list[tuple[int, int, int]]],
    pts: list[tuple[float, float, float]],
    max_count: int,
    voxel_size: float = 1.2,
) -> list[int]:
    """Select up to `max_count` canopy components evenly across 3D crown voxels (outer envelope first)."""
    if len(cids) <= max_count:
        return list(cids)
    centroids: dict[int, tuple[float, float, float]] = {}
    vox: dict[tuple[int, int, int], list[int]] = collections.defaultdict(list)
    for c in cids:
        vs = {i for t in comp_tris[c] for i in t}
        cx = sum(pts[i][0] for i in vs) / len(vs)
        cy = sum(pts[i][1] for i in vs) / len(vs)
        cz = sum(pts[i][2] for i in vs) / len(vs)
        centroids[c] = (cx, cy, cz)
        vox[(math.floor(cx / voxel_size), math.floor(cy / voxel_size), math.floor(cz / voxel_size))].append(c)

    mid_y = sum(c[1] for c in centroids.values()) / max(1, len(centroids))
    for k in vox:
        vox[k].sort(
            key=lambda c: centroids[c][0] ** 2 + (centroids[c][1] - mid_y) ** 2 + centroids[c][2] ** 2,
            reverse=True,
        )

    selected: list[int] = []
    keys = sorted(vox.keys())
    layer = 0
    while len(selected) < max_count:
        progressed = False
        for k in keys:
            if layer < len(vox[k]):
                selected.append(vox[k][layer])
                progressed = True
                if len(selected) >= max_count:
                    break
        if not progressed:
            break
        layer += 1
    return selected


def _build_canopy_blades(
    fol_cids: list[int],
    bloom_cids: list[int],
    comp_tris: dict[int, list[tuple[int, int, int]]],
    pts: list[tuple[float, float, float]],
    srd_bytes: bytes,
    len_scale: float,
    wid_scale: float,
) -> tuple[list[tuple[float, float, float]], list[tuple[int, int, int]], list[tuple[int, int, int, int]]]:
    """Synthesize closed 6-tri slender leaf blades and 8-tri blossom octahedra preserving full silhouettes."""
    b_pts: list[tuple[float, float, float]] = []
    b_tris: list[tuple[int, int, int]] = []
    b_cols: list[tuple[int, int, int, int]] = []

    for c in fol_cids:
        vs = list({i for t in comp_tris[c] for i in t})
        vpts = [pts[i] for i in vs]
        cx = sum(p[0] for p in vpts) / len(vpts)
        cy = sum(p[1] for p in vpts) / len(vpts)
        cz = sum(p[2] for p in vpts) / len(vpts)
        cen = (cx, cy, cz)
        far = max(vpts, key=lambda p: (p[0] - cx) ** 2 + (p[1] - cy) ** 2 + (p[2] - cz) ** 2)
        u = _vec_norm(_vec_sub(far, cen))
        dist_far = math.sqrt((far[0] - cx) ** 2 + (far[1] - cy) ** 2 + (far[2] - cz) ** 2)
        half_len = max(0.12, dist_far * len_scale)

        best_perp = (0.0, 0.0, 0.0)
        best_pd = -1.0
        for p in vpts:
            d = _vec_sub(p, cen)
            perp = _vec_sub(d, _vec_mul(u, _vec_dot(d, u)))
            pd = perp[0] ** 2 + perp[1] ** 2 + perp[2] ** 2
            if pd > best_pd:
                best_pd = pd
                best_perp = perp
        fallback = (0.0, 1.0, 0.0) if abs(u[1]) < 0.9 else (1.0, 0.0, 0.0)
        v_dir = _vec_norm(best_perp) if best_pd > 1e-6 else _vec_norm(_vec_cross(u, fallback))
        half_wid = max(0.055, math.sqrt(max(best_pd, 0.002)) * wid_scale)
        w_dir = _vec_norm(_vec_cross(u, v_dir))
        thick = max(0.012, half_wid * 0.12)

        rs, gs, bs = [], [], []
        for i in vs:
            r, g, bl = srd_bytes[i * 4 : i * 4 + 3]
            if g > r + 4:
                rs.append(r)
                gs.append(g)
                bs.append(bl)
        col = (
            int(round(sum(rs) / len(rs))) if rs else 58,
            int(round(sum(gs) / len(gs))) if gs else 100,
            int(round(sum(bs) / len(bs))) if bs else 54,
            255,
        )

        bi = len(b_pts)
        p0 = _vec_add(cen, _vec_add(_vec_mul(v_dir, -half_wid), _vec_mul(w_dir, -thick * 0.5)))
        p1 = _vec_add(cen, _vec_add(_vec_mul(v_dir, half_wid), _vec_mul(w_dir, -thick * 0.5)))
        p2 = _vec_add(cen, _vec_mul(w_dir, thick))
        p3 = _vec_add(cen, _vec_mul(u, -half_len))
        p4 = _vec_add(cen, _vec_mul(u, half_len))
        b_pts.extend((p0, p1, p2, p3, p4))
        b_cols.extend((col, col, col, col, col))
        b_tris.extend((
            (bi + 0, bi + 1, bi + 4),
            (bi + 1, bi + 2, bi + 4),
            (bi + 2, bi + 0, bi + 4),
            (bi + 1, bi + 0, bi + 3),
            (bi + 2, bi + 1, bi + 3),
            (bi + 0, bi + 2, bi + 3),
        ))

    for c in bloom_cids:
        vs = list({i for t in comp_tris[c] for i in t})
        vpts = [pts[i] for i in vs]
        cx = sum(p[0] for p in vpts) / len(vpts)
        cy = sum(p[1] for p in vpts) / len(vpts)
        cz = sum(p[2] for p in vpts) / len(vpts)
        rx = max(0.05, (max(p[0] for p in vpts) - min(p[0] for p in vpts)) * 0.5 * wid_scale)
        ry = max(0.05, (max(p[1] for p in vpts) - min(p[1] for p in vpts)) * 0.5 * len_scale)
        rz = max(0.05, (max(p[2] for p in vpts) - min(p[2] for p in vpts)) * 0.5 * wid_scale)
        rs = [srd_bytes[i * 4] for i in vs]
        gs = [srd_bytes[i * 4 + 1] for i in vs]
        bs = [srd_bytes[i * 4 + 2] for i in vs]
        col = (
            int(round(sum(rs) / len(rs))),
            int(round(sum(gs) / len(gs))),
            int(round(sum(bs) / len(bs))),
            255,
        )
        bi = len(b_pts)
        b_pts.extend((
            (cx - rx, cy, cz),
            (cx + rx, cy, cz),
            (cx, cy, cz - rz),
            (cx, cy, cz + rz),
            (cx, cy + ry, cz),
            (cx, cy - ry, cz),
        ))
        b_cols.extend((col,) * 6)
        b_tris.extend((
            (bi + 0, bi + 3, bi + 4),
            (bi + 3, bi + 1, bi + 4),
            (bi + 1, bi + 2, bi + 4),
            (bi + 2, bi + 0, bi + 4),
            (bi + 3, bi + 0, bi + 5),
            (bi + 1, bi + 3, bi + 5),
            (bi + 2, bi + 1, bi + 5),
            (bi + 0, bi + 2, bi + 5),
        ))

    return b_pts, b_tris, b_cols


def _nearest_d2_lookup(verts: list[tuple[float, float, float]], cell: float = 0.25):
    """Build a fast spatial-grid distance-squared oracle against a point set."""
    inv = 1.0 / cell
    grid: dict[tuple[int, int, int], list[int]] = collections.defaultdict(list)
    for idx, (x, y, z) in enumerate(verts):
        grid[(math.floor(x * inv), math.floor(y * inv), math.floor(z * inv))].append(idx)

    def query(pt: tuple[float, float, float]) -> float:
        cx, cy, cz = math.floor(pt[0] * inv), math.floor(pt[1] * inv), math.floor(pt[2] * inv)
        best = 1e30
        for dx in (-1, 0, 1):
            for dy in (-1, 0, 1):
                for dz in (-1, 0, 1):
                    for j in grid.get((cx + dx, cy + dy, cz + dz), ()):
                        rx, ry, rz = verts[j]
                        d2 = (pt[0] - rx) ** 2 + (pt[1] - ry) ** 2 + (pt[2] - rz) ** 2
                        if d2 < best:
                            best = d2
        return best

    return query


def _attach_split_colors_and_save(
    raw_10d: Path,
    dest_10d: Path,
    wood_mesh_pts: list[tuple[float, float, float]],
    canopy_mesh_pts: list[tuple[float, float, float]],
    wood_ref_pts: list[tuple[float, float, float]],
    wood_ref_srd: bytes,
    canopy_ref_pts: list[tuple[float, float, float]],
    canopy_ref_srd: bytes,
) -> dict:
    """Attach non-bleeding wood vs canopy SurfaceReading colors and seal `.10d`."""
    hdr, secs = parse_10d(raw_10d.read_bytes())
    qmesh = decode_quantized_mesh(next(s[8] for s in secs if s[0] == 1))
    verts = qmesh["vertices"]
    qw = _nearest_d2_lookup(wood_mesh_pts)
    qc = _nearest_d2_lookup(canopy_mesh_pts)

    wood_idx: list[int] = []
    canopy_idx: list[int] = []
    for i, pt in enumerate(verts):
        if qc(pt) <= qw(pt):
            canopy_idx.append(i)
        else:
            wood_idx.append(i)

    out_srd = bytearray(8 + len(verts) * 4)
    out_srd[0:4] = b"SRD1"
    struct.pack_into("<I", out_srd, 4, len(verts))

    if wood_idx:
        srd_w = transfer_surface_reading([verts[i] for i in wood_idx], wood_ref_pts, wood_ref_srd)
        for pos, i in enumerate(wood_idx):
            out_srd[8 + i * 4 : 12 + i * 4] = srd_w[8 + pos * 4 : 12 + pos * 4]
    if canopy_idx:
        srd_c = transfer_surface_reading([verts[i] for i in canopy_idx], canopy_ref_pts, canopy_ref_srd)
        for pos, i in enumerate(canopy_idx):
            out_srd[8 + i * 4 : 12 + i * 4] = srd_c[8 + pos * 4 : 12 + pos * 4]

    new_secs = [(s[0], s[1], s[2], s[5], s[6], s[8]) for s in secs if s[0] != 12]
    new_secs.append((12, 2, 0, 0, 0, bytes(out_srd)))
    dest_10d.parent.mkdir(parents=True, exist_ok=True)
    dest_10d.write_bytes(encode_10d(hdr, new_secs))
    return inspect_10d_summary(dest_10d)


def _process_flora_species(
    slug: str,
    src_10d: Path,
    work_dir: Path,
    lods_dir: Path,
    run_id: str,
) -> tuple[dict[str, dict], Path, Path]:
    """Run the two-stage foliage-preserving flora LOD pipeline and return `(lods_meta, hmc_path, export_dir)`."""
    src_bytes = src_10d.read_bytes()
    _, orig_secs = parse_10d(src_bytes)
    orig_qmesh = decode_quantized_mesh(next(s[8] for s in orig_secs if s[0] == 1))
    orig_srd = next((s[8] for s in orig_secs if s[0] == 12), None)
    if orig_srd is None:
        raise RuntimeError(f"Missing SurfaceReading section in flora canonical asset: {src_10d}")
    srd_bytes = orig_srd[8:]
    pts = orig_qmesh["vertices"]
    tris = orig_qmesh["triangles"]

    split = _classify_flora_components(pts, tris, srd_bytes)
    comp_tris = split["comp_tris"]
    fol_cids = split["fol_cids"]
    bloom_cids = split["bloom_cids"]
    wood_cids = split["wood_cids"]

    wood_tris = [t for c in wood_cids for t in comp_tris[c]]
    canopy_tris = [t for c in (fol_cids + bloom_cids) for t in comp_tris[c]]
    wood_obj = work_dir / "wood.obj"
    canopy_obj = work_dir / "canopy.obj"
    wood_used = _write_obj(pts, wood_tris, wood_obj)
    canopy_used = _write_obj(pts, canopy_tris, canopy_obj)

    wood_ref_pts = [pts[i] for i in wood_used]
    wood_ref_srd = b"SRD1" + len(wood_ref_pts).to_bytes(4, "little") + b"".join(srd_bytes[i * 4 : i * 4 + 4] for i in wood_used)
    canopy_ref_pts = [pts[i] for i in canopy_used]
    canopy_ref_srd = b"SRD1" + len(canopy_ref_pts).to_bytes(4, "little") + b"".join(srd_bytes[i * 4 : i * 4 + 4] for i in canopy_used)

    hmc1 = SYNTH_PROD_ROOT / f"intenteco-lod-{slug}-{run_id}.hmc"
    exp1 = SYNTH_PROD_ROOT / f"intenteco-lod-{slug}-{run_id}"
    vibe1 = work_dir / f"{slug}-parts.vibe"
    vibe1.write_text(build_vibe_source(slug, True), encoding="utf-8")
    p1 = subprocess.run(
        [
            str(SYNTH_EXE),
            "produce",
            "--profile",
            "files",
            "--quality-profile",
            "production16",
            "--source",
            str(vibe1),
            "--input",
            "wood.obj",
            "obj",
            str(wood_obj),
            "--input",
            "canopy.obj",
            "obj",
            str(canopy_obj),
            "--out",
            str(hmc1),
            "--export-dir",
            str(exp1),
        ],
        capture_output=True,
        text=True,
    )
    if p1.returncode != 0:
        raise RuntimeError(f"synth.exe flora stage-1 failed for {slug} (rc={p1.returncode}):\n{p1.stderr}\n{p1.stdout}")

    def read_10d_mesh(path: Path) -> dict:
        _, secs = parse_10d(path.read_bytes())
        return decode_quantized_mesh(next(s[8] for s in secs if s[0] == 1))

    w0_qm = read_10d_mesh(exp1 / "w_lod0.10d")
    w1_qm = read_10d_mesh(exp1 / "w_lod1.10d")
    w2_qm = read_10d_mesh(exp1 / "w_lod2.10d")
    wa_qm = read_10d_mesh(exp1 / "w_abv.10d")
    f0_qm = read_10d_mesh(exp1 / "f_lod0.10d")

    def _simplify_pruned_wood(src_wood_10d: Path, tag: str, max_comps: int, target_tris: int, bisect_y: float | None = None) -> dict:
        pruned_obj = work_dir / f"{slug}-{tag}-pruned.obj"
        write_pruned_obj_from_10d(src_wood_10d, pruned_obj, max_comps)
        bisect_line = (
            f'let s = capability.invoke("hypermedia.manifold.bisect_plane", {{ mesh: s0, plane_origin: [0.0, {bisect_y:.2f}, 0.0], plane_normal: [0.0, 1.0, 0.0], clear_inner: true, clear_outer: false, fill_cap: false }});\n'
            if bisect_y is not None
            else "let s = s0;\n"
        )
        pvibe = work_dir / f"{slug}-{tag}-pruned.vibe"
        pvibe.write_text(
            'requires [capability("capability.invoke")];\n'
            'let m = capability.invoke("hypermedia.manifold.import_mesh", { input: "source.obj", format: "obj" });\n'
            f'let s0 = capability.invoke("hypermedia.manifold.simplify", {{ mesh: m, target_triangles: {target_tris}, mode: "optimize", attribute_policy: "discard" }});\n'
            + bisect_line
            + 'let r = capability.invoke("hypermedia.manifold.compile_10d_pbr", { mesh: s, topology: false, spatial_index: false });\n'
            f'let mut asset = {{ files: [ {{ key: "{tag}.10d", resource: r }} ] }};\n',
            encoding="utf-8",
        )
        hmc_p = SYNTH_PROD_ROOT / f"intenteco-lod-{slug}-{tag}-{run_id}.hmc"
        exp_p = SYNTH_PROD_ROOT / f"intenteco-lod-{slug}-{tag}-{run_id}"
        rp = subprocess.run(
            [
                str(SYNTH_EXE), "produce", "--profile", "files", "--quality-profile", "production16",
                "--source", str(pvibe), "--input", "source.obj", "obj", str(pruned_obj),
                "--out", str(hmc_p), "--export-dir", str(exp_p),
            ],
            capture_output=True, text=True,
        )
        if rp.returncode != 0:
            raise RuntimeError(f"Pruned wood pass failed for {slug} {tag}: {rp.stderr}\n{rp.stdout}")
        return read_10d_mesh(exp_p / f"{tag}.10d")

    if len(wa_qm["triangles"]) > 6500:
        wa_qm = _simplify_pruned_wood(exp1 / "w_abv.10d", "w_abv", 80, 4500, bisect_y=-0.30)
    if len(w1_qm["triangles"]) > 5500:
        w1_qm = _simplify_pruned_wood(exp1 / "w_lod1.10d", "w_lod1", 75, 3800)
    if len(w2_qm["triangles"]) > 1500:
        w2_qm = _simplify_pruned_wood(exp1 / "w_lod1.10d", "w_lod2", 38, 1050)

    def write_combined_obj(w_qm: dict, c_pts: list, c_tris: list, out_obj: Path) -> None:
        comb_pts = w_qm["vertices"] + c_pts
        off = len(w_qm["vertices"])
        lines = [f"v {x:.6f} {y:.6f} {z:.6f}\n" for x, y, z in comb_pts]
        lines.extend(f"f {a + 1} {b + 1} {c + 1}\n" for a, b, c in w_qm["triangles"])
        lines.extend(f"f {a + off + 1} {b + off + 1} {c + off + 1}\n" for a, b, c in c_tris)
        out_obj.write_text("".join(lines), encoding="utf-8")

    # LOD0: simplified wood + decimated canonical foliage/blossoms (~65k tris)
    lod0_obj = work_dir / "comb_lod0.obj"
    write_combined_obj(w0_qm, f0_qm["vertices"], f0_qm["triangles"], lod0_obj)

    # Aboveground LOD1: <= 25,000 tris total, y >= -0.30 m, keeping 100% of leaves when under budget
    abv_blade_tri_budget = max(3000, 24800 - w_abv_tris) if (w_abv_tris := len(wa_qm["triangles"])) else 19000
    abv_blooms = _stratified_canopy_sample(bloom_cids, comp_tris, pts, min(len(bloom_cids), abv_blade_tri_budget // 40))
    abv_fol_budget = max(100, (abv_blade_tri_budget - len(abv_blooms) * 8) // 6)
    abv_fols = _stratified_canopy_sample(fol_cids, comp_tris, pts, abv_fol_budget)
    abv_bpts, abv_btris, abv_bcols = _build_canopy_blades(abv_fols, abv_blooms, comp_tris, pts, srd_bytes, 1.25, 1.45)
    abv_obj = work_dir / "comb_abv.obj"
    write_combined_obj(wa_qm, abv_bpts, abv_btris, abv_obj)

    # LOD1 (full tree): <= 18,000 tris total
    lod1_blade_budget = max(3000, 17800 - len(w1_qm["triangles"]))
    lod1_blooms = _stratified_canopy_sample(bloom_cids, comp_tris, pts, min(len(bloom_cids), lod1_blade_budget // 40))
    lod1_fol_budget = max(100, (lod1_blade_budget - len(lod1_blooms) * 8) // 6)
    lod1_fols = _stratified_canopy_sample(fol_cids, comp_tris, pts, lod1_fol_budget)
    l1_bpts, l1_btris, l1_bcols = _build_canopy_blades(lod1_fols, lod1_blooms, comp_tris, pts, srd_bytes, 1.32, 1.55)
    lod1_obj = work_dir / "comb_lod1.obj"
    write_combined_obj(w1_qm, l1_bpts, l1_btris, lod1_obj)

    # LOD2 (far-field): <= 4,000 tris total with broad lush canopy blades
    lod2_blade_budget = max(600, 3900 - len(w2_qm["triangles"]))
    lod2_blooms = _stratified_canopy_sample(bloom_cids, comp_tris, pts, min(len(bloom_cids), lod2_blade_budget // 60))
    lod2_fol_budget = max(50, (lod2_blade_budget - len(lod2_blooms) * 8) // 6)
    lod2_fols = _stratified_canopy_sample(fol_cids, comp_tris, pts, lod2_fol_budget, voxel_size=2.0)
    l2_bpts, l2_btris, l2_bcols = _build_canopy_blades(lod2_fols, lod2_blooms, comp_tris, pts, srd_bytes, 1.65, 1.85)
    lod2_obj = work_dir / "comb_lod2.obj"
    write_combined_obj(w2_qm, l2_bpts, l2_btris, lod2_obj)

    hmc2 = SYNTH_PROD_ROOT / f"intenteco-lod-{slug}-comb-{run_id}.hmc"
    exp2 = SYNTH_PROD_ROOT / f"intenteco-lod-{slug}-comb-{run_id}"
    vibe2 = work_dir / f"{slug}-comb.vibe"
    vibe2.write_text(
        'requires [capability("capability.invoke")];\n'
        'let m0 = capability.invoke("hypermedia.manifold.import_mesh", { input: "lod0.obj", format: "obj" });\n'
        'let m1 = capability.invoke("hypermedia.manifold.import_mesh", { input: "lod1.obj", format: "obj" });\n'
        'let m2 = capability.invoke("hypermedia.manifold.import_mesh", { input: "lod2.obj", format: "obj" });\n'
        'let ma = capability.invoke("hypermedia.manifold.import_mesh", { input: "abv.obj", format: "obj" });\n'
        'let r0 = capability.invoke("hypermedia.manifold.compile_10d_pbr", { mesh: m0, topology: false, spatial_index: false });\n'
        'let r1 = capability.invoke("hypermedia.manifold.compile_10d_pbr", { mesh: m1, topology: false, spatial_index: false });\n'
        'let r2 = capability.invoke("hypermedia.manifold.compile_10d_pbr", { mesh: m2, topology: false, spatial_index: false });\n'
        'let ra = capability.invoke("hypermedia.manifold.compile_10d_pbr", { mesh: ma, topology: false, spatial_index: false });\n'
        'let mut asset = { files: [\n'
        f'    {{ key: "{slug}-lod0.10d", resource: r0 }},\n'
        f'    {{ key: "{slug}-lod1.10d", resource: r1 }},\n'
        f'    {{ key: "{slug}-lod2.10d", resource: r2 }},\n'
        f'    {{ key: "{slug}-aboveground-lod1.10d", resource: ra }}\n'
        ']};\n',
        encoding="utf-8",
    )
    p2 = subprocess.run(
        [
            str(SYNTH_EXE),
            "produce",
            "--profile",
            "files",
            "--quality-profile",
            "production16",
            "--source",
            str(vibe2),
            "--input",
            "lod0.obj",
            "obj",
            str(lod0_obj),
            "--input",
            "lod1.obj",
            "obj",
            str(lod1_obj),
            "--input",
            "lod2.obj",
            "obj",
            str(lod2_obj),
            "--input",
            "abv.obj",
            "obj",
            str(abv_obj),
            "--out",
            str(hmc2),
            "--export-dir",
            str(exp2),
        ],
        capture_output=True,
        text=True,
    )
    if p2.returncode != 0:
        raise RuntimeError(f"synth.exe flora stage-2 failed for {slug} (rc={p2.returncode}):\n{p2.stderr}\n{p2.stdout}")

    def blade_srd(cols: list[tuple[int, int, int, int]]) -> bytes:
        return b"SRD1" + len(cols).to_bytes(4, "little") + b"".join(bytes(c) for c in cols)

    lods_meta: dict[str, dict] = {}
    tier_configs = [
        ("lod0", f"{slug}-lod0.10d", "0-25m", w0_qm["vertices"], f0_qm["vertices"], canopy_ref_pts, canopy_ref_srd),
        ("lod1", f"{slug}-lod1.10d", "25-90m", w1_qm["vertices"], l1_bpts, l1_bpts, blade_srd(l1_bcols)),
        ("lod2", f"{slug}-lod2.10d", ">90m", w2_qm["vertices"], l2_bpts, l2_bpts, blade_srd(l2_bcols)),
        ("aboveground_lod1", f"{slug}-aboveground-lod1.10d", "25-90m (y >= -0.30m)", wa_qm["vertices"], abv_bpts, abv_bpts, blade_srd(abv_bcols)),
    ]
    for tier_key, filename, band, w_pts, c_pts, c_rpts, c_rsrd in tier_configs:
        info = _attach_split_colors_and_save(
            exp2 / filename,
            lods_dir / filename,
            w_pts,
            c_pts,
            wood_ref_pts,
            wood_ref_srd,
            c_rpts,
            c_rsrd,
        )
        info["file"] = filename
        info["distance_band_m"] = band
        info["local_url"] = f"/intenteco/{slug}/dist/lods/{filename}"
        info["remote_url"] = f"https://raw.githubusercontent.com/intenteco/{slug}/main/dist/lods/{filename}"
        lods_meta[tier_key] = info

    rhz_name = f"{slug}-rhizosphere.10d"
    rhz_info = attach_colors_and_save(exp1 / rhz_name, lods_dir / rhz_name, wood_ref_pts, wood_ref_srd)
    rhz_info["file"] = rhz_name
    rhz_info["distance_band_m"] = "xray-subterranean (y <= 0.15m)"
    rhz_info["local_url"] = f"/intenteco/{slug}/dist/lods/{rhz_name}"
    rhz_info["remote_url"] = f"https://raw.githubusercontent.com/intenteco/{slug}/main/dist/lods/{rhz_name}"
    lods_meta["rhizosphere"] = rhz_info

    return lods_meta, hmc2, exp2


def process_species(slug: str) -> dict:
    """Run the full LOD production pipeline for one species."""
    t0 = time.time()
    is_flora = slug in FLORA_SPECIES
    kingdom = "flora" if is_flora else "fauna"
    dist_dir = ECO_INTENT_ROOT / slug / "dist"
    src_10d = dist_dir / f"{slug}.10d"
    if not src_10d.is_file():
        raise FileNotFoundError(f"Missing canonical .10d: {src_10d}")

    run_id = f"{int(time.time())}-{uuid.uuid4().hex[:6]}"
    work_dir = SYNTH_PROD_ROOT / f"intenteco-lod-work-{slug}-{run_id}"
    work_dir.mkdir(parents=True, exist_ok=True)
    lods_dir = dist_dir / "lods"
    lods_dir.mkdir(parents=True, exist_ok=True)

    src_bytes = src_10d.read_bytes()
    _, orig_secs = parse_10d(src_bytes)
    orig_qmesh = decode_quantized_mesh(next(s[8] for s in orig_secs if s[0] == 1))
    orig_srd = next((s[8] for s in orig_secs if s[0] == 12), None)

    if is_flora:
        lods_meta, hmc_path, export_dir = _process_flora_species(slug, src_10d, work_dir, lods_dir, run_id)
        validate_lod_budgets(slug, is_flora, lods_meta)
    else:
        hmc_path = SYNTH_PROD_ROOT / f"intenteco-lod-{slug}-{run_id}.hmc"
        export_dir = SYNTH_PROD_ROOT / f"intenteco-lod-{slug}-{run_id}"

        obj_path = work_dir / f"{slug}.obj"
        write_obj_from_10d(src_10d, obj_path)

        vibe_path = work_dir / f"{slug}-lods.vibe"
        vibe_path.write_text(build_vibe_source(slug, is_flora), encoding="utf-8")

        cmd = [
            str(SYNTH_EXE),
            "produce",
            "--profile",
            "files",
            "--quality-profile",
            "production16",
            "--source",
            str(vibe_path),
            "--input",
            "source.obj",
            "obj",
            str(obj_path),
            "--out",
            str(hmc_path),
            "--export-dir",
            str(export_dir),
        ]
        proc = subprocess.run(cmd, capture_output=True, text=True)
        if proc.returncode != 0:
            raise RuntimeError(
                f"synth.exe produce failed for {slug} (rc={proc.returncode}):\n{proc.stderr}\n{proc.stdout}"
            )

        specs = [
            ("lod0", f"{slug}-lod0.10d", "0-25m"),
            ("lod1", f"{slug}-lod1.10d", "25-90m"),
            ("lod2", f"{slug}-lod2.10d", ">90m"),
        ]

        lods_meta = {}
        for tier_key, filename, band in specs:
            info = attach_colors_and_save(
                export_dir / filename,
                lods_dir / filename,
                orig_qmesh["vertices"],
                orig_srd,
            )
            info["file"] = filename
            info["distance_band_m"] = band
            info["local_url"] = f"/intenteco/{slug}/dist/lods/{filename}"
            info["remote_url"] = f"https://raw.githubusercontent.com/intenteco/{slug}/main/dist/lods/{filename}"
            lods_meta[tier_key] = info

        if lods_meta["lod2"]["triangles"] > 2500:
            pruned_obj = work_dir / f"{slug}-lod2-pruned.obj"
            write_pruned_obj_from_10d(lods_dir / f"{slug}-lod1.10d", pruned_obj, 140)
            pruned_vibe = work_dir / f"{slug}-lod2-pruned.vibe"
            pruned_vibe.write_text(
                f'''requires [capability("capability.invoke")];
let m = capability.invoke("hypermedia.manifold.import_mesh", {{ input: "source.obj", format: "obj" }});
let lod2_m = capability.invoke("hypermedia.manifold.simplify", {{ mesh: m, target_triangles: 2200, mode: "optimize", attribute_policy: "discard" }});
let lod2_10d = capability.invoke("hypermedia.manifold.compile_10d_pbr", {{ mesh: lod2_m, topology: false, spatial_index: false }});
let mut asset = {{ files: [ {{ key: "{slug}-lod2.10d", resource: lod2_10d }} ] }};
''',
                encoding="utf-8",
            )
            hmc2 = SYNTH_PROD_ROOT / f"intenteco-lod2-{slug}-{run_id}.hmc"
            exp2 = SYNTH_PROD_ROOT / f"intenteco-lod2-{slug}-{run_id}"
            p2 = subprocess.run(
                [
                    str(SYNTH_EXE),
                    "produce",
                    "--profile",
                    "files",
                    "--quality-profile",
                    "production16",
                    "--source",
                    str(pruned_vibe),
                    "--input",
                    "source.obj",
                    "obj",
                    str(pruned_obj),
                    "--out",
                    str(hmc2),
                    "--export-dir",
                    str(exp2),
                ],
                capture_output=True,
                text=True,
            )
            if p2.returncode != 0:
                raise RuntimeError(f"synth.exe LOD2 pruned pass failed for {slug}:\n{p2.stderr}")
            lod2_name = f"{slug}-lod2.10d"
            info2 = attach_colors_and_save(
                exp2 / lod2_name,
                lods_dir / lod2_name,
                orig_qmesh["vertices"],
                orig_srd,
            )
            info2["file"] = lod2_name
            info2["distance_band_m"] = ">90m"
            info2["local_url"] = f"/intenteco/{slug}/dist/lods/{lod2_name}"
            info2["remote_url"] = f"https://raw.githubusercontent.com/intenteco/{slug}/main/dist/lods/{lod2_name}"
            lods_meta["lod2"] = info2

        validate_lod_budgets(slug, is_flora, lods_meta)

    manifest = {
        "schema": "intenteco.lod-manifest/v1",
        "slug": slug,
        "did": f"did:q42:taxon:{slug.replace('-', '_')}",
        "kingdom": kingdom,
        "source_10d": {
            "file": f"{slug}.10d",
            "bytes": len(src_bytes),
            "sha256": sha256_bytes(src_bytes),
            "vertices": orig_qmesh["vertex_count"],
            "triangles": orig_qmesh["triangle_count"],
            "bounds_min": orig_qmesh["bounds_min"],
            "bounds_max": orig_qmesh["bounds_max"],
        },
        "synth_hmc": str(hmc_path),
        "synth_export_dir": str(export_dir),
        "lods": lods_meta,
    }
    (lods_dir / "lod-manifest.json").write_text(
        json.dumps(manifest, indent=2) + "\n", encoding="utf-8"
    )
    elapsed = round(time.time() - t0, 2)
    print(
        f"[OK] {slug:26s} ({kingdom:5s}) {elapsed:5.1f}s | "
        + " ".join(f"{k}:{v['triangles']}t/{v['bytes']//1024}KB" for k, v in lods_meta.items()),
        flush=True,
    )
    return manifest


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--species", nargs="*", help="Subset of species slugs to build")
    parser.add_argument("--skip-existing", action="store_true", help="Reuse valid existing lod-manifest.json")
    args = parser.parse_args()
    targets = args.species if args.species else list(ALL_SPECIES)
    for slug in targets:
        if slug not in ALL_SPECIES:
            raise SystemExit(f"Unknown species slug: {slug}")

    results = []
    for slug in targets:
        manifest_path = ECO_INTENT_ROOT / slug / "dist" / "lods" / "lod-manifest.json"
        if args.skip_existing and manifest_path.is_file():
            try:
                manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
                validate_lod_budgets(slug, slug in FLORA_SPECIES, manifest["lods"])
                results.append(manifest)
                print(f"[SKIP] {slug:26s} (valid existing manifest)", flush=True)
                continue
            except Exception:
                pass
        results.append(process_species(slug))

    if not args.species:
        COHORT_DIR.mkdir(parents=True, exist_ok=True)
        receipt = {
            "schema": "webizen.species-lod-pipeline-receipt/v1",
            "synth_exe": str(SYNTH_EXE),
            "species_count": len(results),
            "species": results,
        }
        (COHORT_DIR / "lod-pipeline-receipt.json").write_text(
            json.dumps(receipt, indent=2) + "\n", encoding="utf-8"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
