"""Author the 600 m soil mesh for WR-01.

The catchment smoothstep lives only in this authoring step. The bytes it writes
are the mesh. native/world-host ground::height_at samples those bytes and does
not keep a second copy of the formula.
"""
from __future__ import annotations

import hashlib
import json
import math
import struct
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[1]
ECO = Path(r"C:\github\eco-intent")
SIDE_CELLS = 240
VERTS = SIDE_CELLS + 1
CELL = 600.0 / SIDE_CELLS
ORIGIN = -300.0
SPAN = 600.0

# Material ids. Keep in lockstep with ground.rs GroundMaterial.
SILT = 1
LITTER = 2
SANDSTONE = 3
GRANITE = 4
PEBBLE = 5
WET = 6
PAD = 7

# Base sRGB. Keep in lockstep with ground.rs material_base_srgb.
PALETTE = {
    SILT: (168, 112, 64),
    LITTER: (68, 44, 24),
    SANDSTONE: (206, 184, 152),
    GRANITE: (108, 112, 118),
    PEBBLE: (146, 140, 128),
    WET: (42, 52, 40),
    PAD: (118, 82, 48),
}

CAMP = (-22.0, 28.0)
PAD_FLAT = 8.0
PAD_BLEND = 13.0


def clamp(v: float, lo: float, hi: float) -> float:
    return min(hi, max(lo, v))


def smoothstep(edge0: float, edge1: float, x: float) -> float:
    t = clamp((x - edge0) / (edge1 - edge0), 0.0, 1.0)
    return t * t * (3.0 - 2.0 * t)


def river_x(z: float) -> float:
    oz = smoothstep(65.0, 125.0, abs(z))
    return (
        math.sin(z * 0.044) * 10.0
        + math.sin(z * 0.095) * 3.0
        + oz * (math.sin(z * 0.0175) * 26.0 + math.sin(z * 0.0085 + 0.7) * 14.0)
    )


def core_noise(x: float, z: float) -> float:
    return (
        math.sin(x * 0.11 + z * 0.07) * 0.5
        + math.sin(x * 0.28 - z * 0.16) * 0.22
        + math.cos(z * 0.39 + x * 0.2) * 0.12
        + math.sin(x * 0.73 + z * 0.61) * 0.05
    )


def core_height(x: float, z: float) -> float:
    d = abs(x - river_x(z))
    bank = 1.0 - math.exp(-math.pow(d / 9.0, 2.0))
    bank_ch = 1.0 - math.exp(-math.pow(d / 6.5, 2.0))
    valley = 0.35 + bank * (2.4 + max(0.0, abs(x) - 15.0) * 0.22)
    mountain = math.pow(max(0.0, (-z - 25.0) / 65.0), 1.4) * (
        7.0 + 12.0 * math.pow(math.sin(x * 0.055 + 1.2), 2.0)
    )
    channel_gap = 1.0 - smoothstep(-34.0, -48.0, z) * (1.0 - bank_ch)
    m_masked = mountain * channel_gap
    return (
        valley
        + m_masked
        + core_noise(x, z) * bank * (1.4 + m_masked * 0.17)
        - math.exp(-d * d / 16.0) * 1.4
    )


def outer_height(x: float, z: float) -> float:
    rx = river_x(z)
    dx = x - rx
    d = abs(dx)
    bend = -math.sin(z * 0.044) * 0.45 - math.sin(z * 0.0175) * 0.55
    bend_mag = clamp(abs(bend), 0.0, 1.0)
    w_ch = 4.8 + 2.0 * math.pow(math.sin(z * 0.023 + 0.4), 2.0)
    cut_side = (dx * (-bend)) / (abs(dx) + 3.0)
    bank_scale = w_ch + 3.2 - 1.6 * cut_side
    channel_t = smoothstep(2.0, bank_scale + 4.8, d)
    bed_y = -0.50 - 0.95 * bend_mag + 0.12 * math.sin(z * 0.19)
    point_bar = max(0.0, -cut_side) * math.exp(-math.pow((d - (w_ch + 1.8)) / 3.4, 2.0)) * 0.95
    t1 = 1.95 + 0.55 * smoothstep(w_ch + 2.0, w_ch + 9.0, d)
    t2 = t1 + 2.15 * smoothstep(24.0, 35.0, d)
    flood_micro = 0.20 * math.sin(x * 0.07 + z * 0.05) + 0.10 * math.cos(x * 0.15 - z * 0.11)
    floodplain_y = t2 + flood_micro
    slope_d = clamp((d - 46.0) / 220.0, 0.0, 1.25)
    woodland_base = 21.0 * math.pow(slope_d, 1.15)
    woodland_undulation = (
        2.5 * math.sin(x * 0.021 + 0.4) * math.cos(z * 0.019 - 0.3)
        + 1.3 * math.sin(x * 0.047 - z * 0.039)
        + 0.55 * core_noise(x * 0.55, z * 0.55)
    ) * smoothstep(32.0, 75.0, d)
    n_rise = smoothstep(-95.0, -280.0, z)
    n_ridge_profile = 40.0 * math.pow(n_rise, 1.22) * (
        0.78 + 0.22 * math.sin(x * 0.018 + 1.1) * math.cos(x * 0.009)
    )
    n_terrace = n_ridge_profile + 1.9 * math.sin(n_ridge_profile * 0.55) * n_rise * smoothstep(45.0, 90.0, d)
    e_headland_x = smoothstep(115.0, 275.0, x)
    e_headland_z = smoothstep(-140.0, 40.0, z) * (1.0 - 0.35 * smoothstep(200.0, 300.0, z))
    e_headland = 30.0 * e_headland_x * e_headland_z * (0.82 + 0.18 * math.cos(z * 0.025 - 0.5))
    e_bench = e_headland + 1.5 * math.sin(e_headland * 0.6) * e_headland_x
    g_z_line = -55.0 + (x + 75.0) * 0.72
    g_dist = abs(z - g_z_line)
    g_along = smoothstep(-68.0, -120.0, x) * (1.0 - smoothstep(-245.0, -295.0, x))
    g_carve = 10.5 * g_along * math.exp(-math.pow(g_dist / 26.0, 2.0))
    g_flank = 5.0 * g_along * math.exp(-math.pow((g_dist - 42.0) / 18.0, 2.0))
    slope_y = woodland_base + woodland_undulation + n_terrace + e_bench - g_carve + g_flank
    land_y = floodplain_y + max(0.0, slope_y) * smoothstep(38.0, 72.0, d)
    return bed_y + point_bar + (land_y - bed_y - point_bar) * channel_t


def natural_height(x: float, z: float) -> float:
    r = math.sqrt(x * x + z * z)
    s_outer = smoothstep(68.0, 112.0, r)
    c = core_height(x, z)
    if s_outer <= 0.0:
        return c
    o = outer_height(x, z)
    return c + (o - c) * s_outer


CAMPSITE_LEVEL = natural_height(CAMP[0], CAMP[1])


def authored_height(x: float, z: float) -> float:
    d = math.hypot(x - CAMP[0], z - CAMP[1])
    t = clamp((d - PAD_FLAT) / (PAD_BLEND - PAD_FLAT), 0.0, 1.0)
    shaped = t * t * (3.0 - 2.0 * t)
    return CAMPSITE_LEVEL + (natural_height(x, z) - CAMPSITE_LEVEL) * shaped


def cut_side_at(x: float, z: float) -> float:
    rx = river_x(z)
    dx = x - rx
    bend = -math.sin(z * 0.044) * 0.45 - math.sin(z * 0.0175) * 0.55
    return (dx * (-bend)) / (abs(dx) + 3.0)


def vertex_xz(i: int, j: int) -> tuple[float, float]:
    return ORIGIN + i * CELL, ORIGIN + j * CELL


def slope_deg(heights: list[float], i: int, j: int) -> float:
    def h(ii: int, jj: int) -> float:
        ii = min(VERTS - 1, max(0, ii))
        jj = min(VERTS - 1, max(0, jj))
        return heights[jj * VERTS + ii]

    dhdx = (h(i + 1, j) - h(i - 1, j)) / (2.0 * CELL)
    dhdz = (h(i, j + 1) - h(i, j - 1)) / (2.0 * CELL)
    return math.degrees(math.atan(math.hypot(dhdx, dhdz)))


def in_canopy(x: float, z: float, centres: list[tuple[float, float, float]]) -> bool:
    for cx, cz, radius in centres:
        if (x - cx) * (x - cx) + (z - cz) * (z - cz) <= radius * radius:
            return True
    return False


def lerp_rgb(a: tuple[int, int, int], b: tuple[int, int, int], t: float) -> tuple[int, int, int]:
    return tuple(int(round(a[k] + (b[k] - a[k]) * t)) for k in range(3))  # type: ignore[return-value]


def classify(heights: list[float], centres: list[tuple[float, float, float]]) -> tuple[list[int], list[tuple[int, int, int]]]:
    mats: list[int] = []
    colours: list[tuple[int, int, int]] = []
    for j in range(VERTS):
        for i in range(VERTS):
            x, z = vertex_xz(i, j)
            y = heights[j * VERTS + i]
            d_camp = math.hypot(x - CAMP[0], z - CAMP[1])
            d_creek = abs(x - river_x(z))
            slope = slope_deg(heights, i, j)
            cut = cut_side_at(x, z)
            if y < 0.30:
                mat = WET
            elif d_creek < 3.6 and y < 1.55:
                mat = WET
            elif 2.2 <= d_creek < 11.5 and y < 3.6 and -cut > -0.05:
                mat = PEBBLE
            elif d_creek < 8.0 and y < 2.4:
                mat = WET
            elif slope >= 20.0 and z < -80.0 and y > 14.0:
                mat = GRANITE
            elif slope >= 24.0 or (slope >= 18.0 and y > 30.0):
                mat = SANDSTONE
            elif in_canopy(x, z, centres) and d_creek > 10.0 and d_camp > PAD_FLAT:
                mat = LITTER
            elif y < 6.2 and slope < 8.0 and d_creek > 7.0:
                mat = SILT
            elif d_creek < 48.0 and y < 8.0 and slope < 11.0:
                mat = SILT
            elif in_canopy(x, z, centres):
                mat = LITTER
            else:
                mat = SILT if slope < 12.0 else SANDSTONE
            soil_rgb = PALETTE[mat]
            if d_camp <= PAD_FLAT:
                mat = PAD
                rgb = PALETTE[PAD]
            elif d_camp < PAD_BLEND:
                t = smoothstep(PAD_FLAT, PAD_BLEND, d_camp)
                rgb = lerp_rgb(PALETTE[PAD], soil_rgb, t)
            else:
                rgb = soil_rgb
            mats.append(mat)
            colours.append(rgb)
    return mats, colours


def canopy_centres(heights: list[float]) -> list[tuple[float, float, float]]:
    fixed = [
        (-36.0, 36.0, 14.0),
        (-8.0, 44.0, 13.0),
        (-44.0, 18.0, 13.0),
        (4.0, 18.0, 12.0),
        (-28.0, 52.0, 12.0),
        (-2.0, 10.0, 12.0),
        (16.0, 34.0, 13.0),
        (-50.0, 30.0, 14.0),
        (-18.0, 8.0, 11.0),
        (10.0, 6.0, 11.0),
        (-32.0, 14.0, 11.0),
    ]
    centres = list(fixed)
    for j in range(8, VERTS - 8, 9):
        for i in range(8, VERTS - 8, 9):
            x, z = vertex_xz(i, j)
            y = heights[j * VERTS + i]
            d_camp = math.hypot(x - CAMP[0], z - CAMP[1])
            d_creek = abs(x - river_x(z))
            slope = slope_deg(heights, i, j)
            if d_camp < 14.0 or d_creek < 14.0 or y < 1.2 or slope > 14.0 or y > 28.0:
                continue
            jitter_x = math.sin(i * 12.9898 + j * 78.233) * 4.0
            jitter_z = math.cos(i * 4.14 + j * 9.1) * 4.0
            centres.append((x + jitter_x, z + jitter_z, 7.5))
            if len(centres) >= 90:
                return centres
    return centres


def write_bin(path: Path, heights: list[float], mats: list[int], colours: list[tuple[int, int, int]]) -> None:
    blob = bytearray()
    blob += b"SOIL"
    blob += struct.pack("<I", 1)
    blob += struct.pack("<I", VERTS)
    blob += struct.pack("<f", CELL)
    blob += struct.pack("<f", ORIGIN)
    blob += struct.pack("<f", ORIGIN)
    blob += struct.pack("<" + "f" * len(heights), *heights)
    for mat, rgb in zip(mats, colours):
        blob += struct.pack("<BBBB", mat, rgb[0], rgb[1], rgb[2])
    path.write_bytes(blob)


def png_chunk(tag: bytes, data: bytes) -> bytes:
    return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)


def write_png(path: Path, w: int, h: int, pixels: bytes, channels: int) -> None:
    raw = bytearray()
    stride = w * channels
    for y in range(h):
        raw.append(0)
        raw.extend(pixels[y * stride : (y + 1) * stride])
    ihdr = struct.pack(">IIBBBBB", w, h, 8, 2 if channels == 3 else 6, 0, 0, 0)
    png = b"\x89PNG\r\n\x1a\n" + png_chunk(b"IHDR", ihdr) + png_chunk(b"IDAT", zlib.compress(bytes(raw), 6)) + png_chunk(b"IEND", b"")
    path.write_bytes(png)


def sample_vertex_rgb(colours: list[tuple[int, int, int]], x: float, z: float) -> tuple[int, int, int]:
    u = clamp((x - ORIGIN) / CELL, 0.0, SIDE_CELLS)
    v = clamp((z - ORIGIN) / CELL, 0.0, SIDE_CELLS)
    i = min(SIDE_CELLS - 1, int(math.floor(u)))
    j = min(SIDE_CELLS - 1, int(math.floor(v)))
    s = u - i
    t = v - j

    def rgb_at(ii: int, jj: int) -> tuple[int, int, int]:
        return colours[jj * VERTS + ii]

    if s + t <= 1.0:
        w0, w1, w2 = 1.0 - s - t, s, t
        c0, c1, c2 = rgb_at(i, j), rgb_at(i + 1, j), rgb_at(i, j + 1)
    else:
        w0, w1, w2 = 1.0 - t, s + t - 1.0, 1.0 - s
        c0, c1, c2 = rgb_at(i + 1, j), rgb_at(i + 1, j + 1), rgb_at(i, j + 1)
    out = []
    for k in range(3):
        out.append(int(round(c0[k] * w0 + c1[k] * w1 + c2[k] * w2)))
    return out[0], out[1], out[2]


def height_mesh(heights: list[float], x: float, z: float) -> float:
    u = clamp((x - ORIGIN) / CELL, 0.0, SIDE_CELLS)
    v = clamp((z - ORIGIN) / CELL, 0.0, SIDE_CELLS)
    i = min(SIDE_CELLS - 1, int(math.floor(u)))
    j = min(SIDE_CELLS - 1, int(math.floor(v)))
    s = u - i
    t = v - j

    def hv(ii: int, jj: int) -> float:
        return heights[jj * VERTS + ii]

    if s + t <= 1.0:
        return hv(i, j) * (1.0 - s - t) + hv(i + 1, j) * s + hv(i, j + 1) * t
    return hv(i + 1, j) * (1.0 - t) + hv(i + 1, j + 1) * (s + t - 1.0) + hv(i, j + 1) * (1.0 - s)


def dominant_mat(mats: list[int], x: float, z: float) -> int:
    u = clamp((x - ORIGIN) / CELL, 0.0, SIDE_CELLS)
    v = clamp((z - ORIGIN) / CELL, 0.0, SIDE_CELLS)
    i = min(SIDE_CELLS - 1, int(math.floor(u)))
    j = min(SIDE_CELLS - 1, int(math.floor(v)))
    s = u - i
    t = v - j
    if s + t <= 1.0:
        weights = [(1.0 - s - t, mats[j * VERTS + i]), (s, mats[j * VERTS + i + 1]), (t, mats[(j + 1) * VERTS + i])]
    else:
        weights = [
            (1.0 - t, mats[j * VERTS + i + 1]),
            (s + t - 1.0, mats[(j + 1) * VERTS + i + 1]),
            (1.0 - s, mats[(j + 1) * VERTS + i]),
        ]
    weights.sort(reverse=True)
    return weights[0][1]


def build_textures(heights: list[float], colours: list[tuple[int, int, int]]) -> tuple[Path, Path, Path]:
    """Nearest-vertex albedo. The host samples vertex colour; this map is the GLB texture."""
    tex = ROOT / "textures"
    tex.mkdir(parents=True, exist_ok=True)
    n = 512
    albedo = bytearray(n * n * 3)
    normal = bytearray(n * n * 3)
    orm = bytearray(n * n * 3)
    for py in range(n):
        j = min(VERTS - 1, int((py + 0.5) / n * SIDE_CELLS + 0.5))
        for px in range(n):
            i = min(VERTS - 1, int((px + 0.5) / n * SIDE_CELLS + 0.5))
            r, g, b = colours[j * VERTS + i]
            grain = ((px * 17 + py * 31) & 15) - 8
            o = (py * n + px) * 3
            albedo[o] = min(255, max(0, r + grain))
            albedo[o + 1] = min(255, max(0, g + grain))
            albedo[o + 2] = min(255, max(0, b + grain))
            i1 = min(VERTS - 1, i + 1)
            j1 = min(VERTS - 1, j + 1)
            i0 = max(0, i - 1)
            j0 = max(0, j - 1)
            dhdx = (heights[j * VERTS + i1] - heights[j * VERTS + i0]) / ((i1 - i0) * CELL or CELL)
            dhdz = (heights[j1 * VERTS + i] - heights[j0 * VERTS + i]) / ((j1 - j0) * CELL or CELL)
            nx, ny, nz = -dhdx, 1.0, -dhdz
            ln = math.sqrt(nx * nx + ny * ny + nz * nz) or 1.0
            nx, ny, nz = nx / ln, ny / ln, nz / ln
            normal[o] = int((nx * 0.5 + 0.5) * 255)
            normal[o + 1] = int((nz * 0.5 + 0.5) * 255)
            normal[o + 2] = int((ny * 0.5 + 0.5) * 255)
            steep = math.sqrt(dhdx * dhdx + dhdz * dhdz)
            y = heights[j * VERTS + i]
            orm[o] = int(clamp(230 - steep * 28, 140, 255))
            orm[o + 1] = int(clamp(200 + steep * 36 - (30 if y < 0.45 else 0), 36, 250))
            orm[o + 2] = 12 if y < 0.35 else 0
    albedo_path = tex / "ground-albedo.png"
    normal_path = tex / "ground-normal.png"
    orm_path = tex / "ground-orm.png"
    write_png(albedo_path, n, n, bytes(albedo), 3)
    write_png(normal_path, n, n, bytes(normal), 3)
    write_png(orm_path, n, n, bytes(orm), 3)
    return albedo_path, normal_path, orm_path


def pack_glb(path: Path, heights: list[float], colours: list[tuple[int, int, int]], maps: list[Path]) -> None:
    positions: list[float] = []
    normals: list[float] = []
    uvs: list[float] = []
    tangents: list[float] = []
    indices: list[int] = []
    acc = [[0.0, 0.0, 0.0] for _ in range(VERTS * VERTS)]
    for j in range(SIDE_CELLS):
        for i in range(SIDE_CELLS):
            a = j * VERTS + i
            b = a + VERTS
            tris = ((a, b, a + 1), (a + 1, b, b + 1))
            for tri in tris:
                p = []
                for idx in tri:
                    ii, jj = idx % VERTS, idx // VERTS
                    x, z = vertex_xz(ii, jj)
                    p.append((x, heights[idx], z))
                e1 = (p[1][0] - p[0][0], p[1][1] - p[0][1], p[1][2] - p[0][2])
                e2 = (p[2][0] - p[0][0], p[2][1] - p[0][1], p[2][2] - p[0][2])
                nx = e1[1] * e2[2] - e1[2] * e2[1]
                ny = e1[2] * e2[0] - e1[0] * e2[2]
                nz = e1[0] * e2[1] - e1[1] * e2[0]
                if ny < 0.0:
                    nx, ny, nz = -nx, -ny, -nz
                for idx in tri:
                    acc[idx][0] += nx
                    acc[idx][1] += ny
                    acc[idx][2] += nz
                indices.extend(tri)
    for j in range(VERTS):
        v = j / SIDE_CELLS
        for i in range(VERTS):
            x, z = vertex_xz(i, j)
            y = heights[j * VERTS + i]
            positions.extend((x, y, z))
            uvs.extend((i / SIDE_CELLS, v))
            nx, ny, nz = acc[j * VERTS + i]
            ln = math.sqrt(nx * nx + ny * ny + nz * nz) or 1.0
            nx, ny, nz = nx / ln, ny / ln, nz / ln
            normals.extend((nx, ny, nz))
            tx, ty, tz = 1.0 - nx * nx, -nx * ny, -nx * nz
            tl = math.sqrt(tx * tx + ty * ty + tz * tz) or 1.0
            tangents.extend((tx / tl, ty / tl, tz / tl, 1.0))
    del colours

    binary = bytearray()
    views = []
    accessors = []

    def add_blob(data: bytes, target: int | None = None) -> int:
        while len(binary) % 4:
            binary.append(0)
        offset = len(binary)
        binary.extend(data)
        rec = {"buffer": 0, "byteOffset": offset, "byteLength": len(data)}
        if target is not None:
            rec["target"] = target
        views.append(rec)
        return len(views) - 1

    def add_attr(values: list[float] | list[int], fmt: str, kind: str, count: int, target: int) -> int:
        bv = add_blob(struct.pack(f"<{len(values)}{fmt}", *values), target)
        comp = 5126 if fmt == "f" else 5125
        accessors.append({"bufferView": bv, "componentType": comp, "count": count, "type": kind})
        return len(accessors) - 1

    vcount = len(positions) // 3
    a_pos = add_attr(positions, "f", "VEC3", vcount, 34962)
    accessors[a_pos].update(min=[min(positions[k::3]) for k in range(3)], max=[max(positions[k::3]) for k in range(3)])
    a_nrm = add_attr(normals, "f", "VEC3", vcount, 34962)
    a_uv = add_attr(uvs, "f", "VEC2", vcount, 34962)
    a_tan = add_attr(tangents, "f", "VEC4", vcount, 34962)
    a_idx = add_attr(indices, "I", "SCALAR", len(indices), 34963)
    images = [{"bufferView": add_blob(p.read_bytes()), "mimeType": "image/png"} for p in maps]
    mat = {
        "name": "catchment-soil",
        "pbrMetallicRoughness": {
            "baseColorFactor": [1.0, 1.0, 1.0, 1.0],
            "baseColorTexture": {"index": 0},
            "metallicFactor": 0.0,
            "roughnessFactor": 0.92,
            "metallicRoughnessTexture": {"index": 2},
        },
        "normalTexture": {"index": 1, "scale": 0.65},
        "doubleSided": False,
    }
    doc = {
        "asset": {"version": "2.0", "generator": "WR-01 catchment soil"},
        "buffers": [{"byteLength": len(binary)}],
        "bufferViews": views,
        "accessors": accessors,
        "images": images,
        "samplers": [{"wrapS": 33071, "wrapT": 33071, "magFilter": 9729, "minFilter": 9987}],
        "textures": [{"source": i, "sampler": 0} for i in range(len(images))],
        "materials": [mat],
        "meshes": [
            {
                "name": "catchment-ground",
                "primitives": [
                    {
                        "attributes": {"POSITION": a_pos, "NORMAL": a_nrm, "TEXCOORD_0": a_uv, "TANGENT": a_tan},
                        "indices": a_idx,
                        "material": 0,
                    }
                ],
            }
        ],
        "nodes": [{"mesh": 0, "name": "catchment-ground"}],
        "scenes": [{"nodes": [0]}],
        "scene": 0,
    }
    encoded = json.dumps(doc, separators=(",", ":")).encode("utf-8")
    encoded += b" " * ((-len(encoded)) % 4)
    binary += b"\0" * ((-len(binary)) % 4)
    total = 12 + 8 + len(encoded) + 8 + len(binary)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(
        struct.pack("<III", 0x46546C67, 2, total)
        + struct.pack("<II", len(encoded), 0x4E4F534A)
        + encoded
        + struct.pack("<II", len(binary), 0x004E4942)
        + binary
    )


def write_tufts(path: Path, heights: list[float], mats: list[int]) -> list[dict]:
    rows: list[tuple[str, float, float, float, bool]] = [
        ("PoaLabillardierei", -17.2, 20.4, 0.6, True),
        ("LomandraLongifolia", -14.2, 22.6, 1.4, True),
        ("ThemedaTriandra", -11.2, 20.6, 2.1, True),
    ]
    golden = math.pi * (3.0 - math.sqrt(5.0))
    species_cycle = ("ThemedaTriandra", "PoaLabillardierei", "LomandraLongifolia")
    for n in range(54):
        ang = n * golden
        rad = 13.0 + (n % 8) * 3.1
        x = CAMP[0] + math.cos(ang) * rad
        z = CAMP[1] + math.sin(ang) * rad
        if height_mesh(heights, x, z) < 0.55:
            continue
        if any((x - px) ** 2 + (z - pz) ** 2 < 2.2 ** 2 for _, px, pz, _, _ in rows):
            continue
        rows.append((species_cycle[n % 3], x, z, ang, False))
    placed_creek = 0
    for j in range(0, VERTS, 3):
        for i in range(0, VERTS, 3):
            if placed_creek >= 18:
                break
            idx = j * VERTS + i
            if mats[idx] not in (PEBBLE, WET):
                continue
            x, z = vertex_xz(i, j)
            y = heights[idx]
            if y < 0.42 or y > 2.4 or abs(z - 12.0) > 70.0 or abs(x) > 50.0:
                continue
            if any((x - px) ** 2 + (z - pz) ** 2 < 3.0 ** 2 for _, px, pz, _, _ in rows):
                continue
            rows.append(("LomandraLongifolia", x, z, 0.2 + placed_creek * 0.35, False))
            placed_creek += 1
    lines = ["["]
    payload = []
    for name, x, z, yaw, foreground in rows:
        lines.append(
            "    TuftSeed { species: Species::%s, trunk_x: %.4f, trunk_z: %.4f, yaw: %.4f, foreground: %s },"
            % (name, x, z, yaw, "true" if foreground else "false")
        )
        payload.append({"species": name, "x": x, "z": z, "yaw": yaw, "foreground": foreground, "material": dominant_mat(mats, x, z), "y": height_mesh(heights, x, z)})
    lines.append("]")
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return payload


def read_png(path: Path) -> tuple[int, int, int, list[bytearray]]:
    data = path.read_bytes()
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise SystemExit(f"not a png: {path}")
    pos = 8
    w = h = color = None
    idat = b""
    while pos + 8 <= len(data):
        n, tag = struct.unpack(">I4s", data[pos : pos + 8])
        chunk = data[pos + 8 : pos + 8 + n]
        pos += 12 + n
        if tag == b"IHDR":
            w, h, bit, color, comp, filt, inter = struct.unpack(">IIBBBBB", chunk)
            if bit != 8 or comp or filt or inter or color not in (2, 6):
                raise SystemExit(f"unsupported png {path} color={color}")
        elif tag == b"IDAT":
            idat += chunk
        elif tag == b"IEND":
            break
    assert w and h and color
    raw = zlib.decompress(idat)
    bpp = 3 if color == 2 else 4
    stride = w * bpp
    rows: list[bytearray] = []
    prev = bytearray(stride)
    i = 0
    for _y in range(h):
        f = raw[i]
        i += 1
        row = bytearray(raw[i : i + stride])
        i += stride
        if f == 1:
            for x in range(stride):
                left = row[x - bpp] if x >= bpp else 0
                row[x] = (row[x] + left) & 255
        elif f == 2:
            for x in range(stride):
                row[x] = (row[x] + prev[x]) & 255
        elif f == 3:
            for x in range(stride):
                left = row[x - bpp] if x >= bpp else 0
                row[x] = (row[x] + ((left + prev[x]) // 2)) & 255
        elif f == 4:
            for x in range(stride):
                a = row[x - bpp] if x >= bpp else 0
                b = prev[x]
                c = prev[x - bpp] if x >= bpp else 0
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                pr = a if pa <= pb and pa <= pc else (b if pb <= pc else c)
                row[x] = (row[x] + pr) & 255
        elif f != 0:
            raise SystemExit(f"bad filter {f}")
        prev = row
        rows.append(row)
    return w, h, color, rows


def crop_species_sprite(src: Path, rgba_path: Path, png_path: Path, keep_frac: float = 1.0) -> None:
    w, h, color, rows = read_png(src)
    bpp = 3 if color == 2 else 4
    rgba = bytearray(w * h * 4)
    for y in range(h):
        row = rows[y]
        for x in range(w):
            o = x * bpp
            r, g, b = row[o], row[o + 1], row[o + 2]
            spread = max(abs(r - g), abs(g - b), abs(r - b))
            if spread < 16 and 96 <= g <= 168:
                a = 0
            elif spread < 32 and 96 <= g <= 176:
                a = int((spread - 16) * 255 / 16)
            else:
                a = 255
            d = (y * w + x) * 4
            rgba[d : d + 4] = bytes((r, g, b, a))
    minx, miny, maxx, maxy = w, h, 0, 0
    for y in range(h):
        for x in range(w):
            if rgba[(y * w + x) * 4 + 3] > 24:
                minx, miny = min(minx, x), min(miny, y)
                maxx, maxy = max(maxx, x), max(maxy, y)
    if maxx <= minx:
        raise SystemExit(f"empty sprite {src}")
    # keep_frac is the fraction of the opaque span retained from the top.
    # Themeda and poa keep the blade collar. Lomandra stops above the rhizome bulb.
    span = maxy - miny
    cut = miny + max(8, int(round(span * keep_frac)))
    cut = min(maxy, cut)
    cw, ch = maxx - minx + 1, cut - miny + 1
    cropped = bytearray(cw * ch * 4)
    for y in range(ch):
        src_o = ((miny + y) * w + minx) * 4
        cropped[y * cw * 4 : (y + 1) * cw * 4] = rgba[src_o : src_o + cw * 4]
    rgba_path.parent.mkdir(parents=True, exist_ok=True)
    rgba_path.write_bytes(struct.pack("<II", cw, ch) + cropped)
    write_png(png_path, cw, ch, bytes(cropped), 4)
    print(f"sprite {png_path.name} {cw}x{ch} keep={keep_frac:.2f}", flush=True)


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    h.update(path.read_bytes())
    return h.hexdigest()


def main() -> None:
    print("authoring soil heights", flush=True)
    heights = []
    for j in range(VERTS):
        _, z = vertex_xz(0, j)
        for i in range(VERTS):
            x, _ = vertex_xz(i, j)
            heights.append(authored_height(x, z))
    centres = canopy_centres(heights)
    mats, colours = classify(heights, centres)
    counts: dict[int, int] = {}
    for m in mats:
        counts[m] = counts.get(m, 0) + 1
    names = {SILT: "silt", LITTER: "litter", SANDSTONE: "sandstone", GRANITE: "granite", PEBBLE: "pebble", WET: "wet", PAD: "pad"}
    print("material vertices", {names[k]: counts.get(k, 0) for k in names}, flush=True)
    print(f"pad level {CAMPSITE_LEVEL:.4f} height range {min(heights):.3f}..{max(heights):.3f}", flush=True)

    bin_path = ROOT / "catchment-soil.bin"
    write_bin(bin_path, heights, mats, colours)
    print("wrote", bin_path, bin_path.stat().st_size, flush=True)

    print("textures", flush=True)
    maps = build_textures(heights, colours)
    glb_path = ROOT / "inputs" / "catchment-ground.glb"
    print("glb", flush=True)
    pack_glb(glb_path, heights, colours, list(maps))
    tufts = write_tufts(ROOT / "tufts.inc.rs", heights, mats)
    print("tufts", len(tufts), "foreground", sum(1 for t in tufts if t["foreground"]), flush=True)

    sprites = ROOT / "sprites"
    crop_species_sprite(ECO / "themeda-triandra" / "previews" / "three-quarter-view-assumed.png", sprites / "themeda.rgba", sprites / "themeda.png", 0.50)
    crop_species_sprite(ECO / "poa-labillardierei" / "previews" / "three-quarter-view-assumed.png", sprites / "poa.rgba", sprites / "poa.png", 0.56)
    crop_species_sprite(ECO / "lomandra-longifolia" / "previews" / "three-quarter-view-assumed.png", sprites / "lomandra.rgba", sprites / "lomandra.png", 0.42)

    manifest = {
        "schema": "webizen.wr01-soil/v1",
        "frame": {
            "units": "metres",
            "up": "+Y",
            "origin": "basin centre",
            "bounds_xz_m": [-300.0, 300.0],
            "cell_m": CELL,
            "vertices_side": VERTS,
            "note": "Runtime height_at samples catchment-soil.bin. This script's smoothstep is not compiled into the host.",
        },
        "campsite_pad": {"centre_xz": list(CAMP), "flat_radius_m": PAD_FLAT, "blend_radius_m": PAD_BLEND, "mesh_y_m": height_mesh(heights, CAMP[0], CAMP[1])},
        "bin": {"path": "production/ground/catchment-soil.bin", "bytes": bin_path.stat().st_size, "sha256": sha256_file(bin_path)},
        "glb": {"path": "production/ground/inputs/catchment-ground.glb", "bytes": glb_path.stat().st_size, "sha256": sha256_file(glb_path)},
        "material_counts": {names[k]: counts.get(k, 0) for k in names},
        "tufts": len(tufts),
        "species_sources": [
            "C:/github/eco-intent/themeda-triandra",
            "C:/github/eco-intent/poa-labillardierei",
            "C:/github/eco-intent/lomandra-longifolia",
        ],
    }
    (ROOT / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    (ROOT / "tufts.json").write_text(json.dumps(tufts, indent=2) + "\n", encoding="utf-8")
    print("done", flush=True)


if __name__ == "__main__":
    main()
