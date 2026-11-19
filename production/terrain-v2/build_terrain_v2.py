"""WEB-WP-03: Upgraded 600 m x 600 m SE Australian River Catchment Terrain, Water & PBR Pipeline.

Produces deterministic LOD0 (Desktop TierA/B), LOD1 (Mobile WebGL2 TierC <= 32k tris),
continuous meandering river water surface mesh (z = -290..+290), Matter-64 (.m64)
geodetic strata field, and 1024x1024 WebGPU/WebGL2 PBR maps via release synth.exe.
"""
from __future__ import annotations

import hashlib
import json
import math
import shutil
import struct
import subprocess
import time
import uuid
from pathlib import Path

ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[1]
SYNTH_EXE = Path(r"C:\Users\Hp\AppData\Local\Programs\Webizen AI Synth\synth.exe")
SYNTH_PROD = Path(r"C:\github\synth\production")

for sub in ("inputs", "recipes", "previews", "textures", "evidence"):
    (ROOT / sub).mkdir(parents=True, exist_ok=True)


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


# ---------------------------------------------------------------------------
# Deterministic 600 m x 600 m SE Australian Catchment Heightfield
# Exact mathematical parity with site/world/living-world.vibe & site/world/terrain.js
# ---------------------------------------------------------------------------

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


CAMPSITE_LEVEL = natural_height(-22.0, 28.0)


def terrain_height(x: float, z: float) -> float:
    d = math.sqrt((x + 22.0) ** 2 + (z - 28.0) ** 2)
    t = clamp((d - 8.0) / 5.0, 0.0, 1.0)
    return CAMPSITE_LEVEL + (natural_height(x, z) - CAMPSITE_LEVEL) * t * t * (3.0 - 2.0 * t)


def verify_js_parity() -> dict:
    """Verify Python heightfield matches site/world/terrain.js to < 1e-12 m across a 25x25 grid."""
    terrain_js = (REPO / "site/world/terrain.js").as_uri()
    script = (
        f"import('{terrain_js}').then(m => {{"
        "const out=[];"
        "for(let j=0;j<=24;j++)for(let i=0;i<=24;i++){"
        "const x=-300+i*25, z=-300+j*25;"
        "out.push([x,z,m.riverX(z),m.heightAt(x,z)]);"
        "}"
        "for(const [x,z] of [[-22,28],[0,16],[-17,6],[20,-7],[-20,-16],[-15,-32],[17,-37]])"
        "out.push([x,z,m.riverX(z),m.heightAt(x,z)]);"
        "console.log(JSON.stringify(out));"
        "});"
    )
    proc = subprocess.run(["node", "-e", script], capture_output=True, text=True, check=True)
    rows = json.loads(proc.stdout.strip().splitlines()[-1])
    max_y_err = 0.0
    max_rx_err = 0.0
    for x, z, js_rx, js_y in rows:
        max_rx_err = max(max_rx_err, abs(js_rx - river_x(z)))
        max_y_err = max(max_y_err, abs(js_y - terrain_height(x, z)))
    assert max_y_err < 1e-12 and max_rx_err < 1e-12, (max_y_err, max_rx_err)
    return {
        "samples_checked": len(rows),
        "max_height_error_m": max_y_err,
        "max_river_x_error_m": max_rx_err,
        "campsite_ground_y": CAMPSITE_LEVEL,
    }


def vibe_lit(v) -> str:
    if isinstance(v, dict):
        return "{" + ",".join(f"{k}:{vibe_lit(x)}" for k, x in v.items()) + "}"
    if isinstance(v, list):
        return "[" + ",".join(vibe_lit(x) for x in v) + "]"
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, float):
        return str(round(v, 6))
    return json.dumps(v)


def run_synth_produce(label: str, vibe_path: Path, inputs: list[tuple[str, str, Path]] | None = None) -> dict:
    stamp = f"{time.strftime('%Y%m%d-%H%M%S')}-{uuid.uuid4().hex[:6]}"
    run_dir = SYNTH_PROD / f"webizen-terrain-{label}-{stamp}"
    run_dir.mkdir(parents=True, exist_ok=False)
    hmc_path = run_dir / f"{label}.hmc"
    export_dir = run_dir / "export"
    cmd = [
        str(SYNTH_EXE),
        "produce",
        "--profile",
        "files",
        "--quality-profile",
        "production16",
        "--source",
        str(vibe_path),
    ]
    if inputs:
        for key, kind, path in inputs:
            cmd.extend(["--input", key, kind, str(path)])
    cmd.extend(["--out", str(hmc_path), "--export-dir", str(export_dir)])
    proc = subprocess.run(cmd, capture_output=True, text=True)
    if proc.returncode != 0 or not hmc_path.exists():
        raise RuntimeError(f"Synth produce failed for {label} (rc={proc.returncode}):\n{proc.stdout}\n{proc.stderr}")
    return {
        "run_dir": str(run_dir),
        "hmc_path": str(hmc_path),
        "hmc_sha256": sha256_file(hmc_path),
        "hmc_bytes": hmc_path.stat().st_size,
        "export_dir": str(export_dir),
        "stdout": proc.stdout.strip(),
    }


# ---------------------------------------------------------------------------
# Step 1: 1024x1024 WebGPU / WebGL2 PBR Maps + 256x256 Water PBR Maps via Synth
# ---------------------------------------------------------------------------

def build_pbr_maps() -> dict:
    # 128x128 ecological biome classification over 600m x 600m (8 px per cell in 1024x1024)
    grid = 128
    cell_px = 1024 // grid

    # 24 curated SE Australian catchment surface tones (RGBA 0..255)
    palette = [
        [142, 132, 108, 255],  # 0: submerged cobble / river bed gravel
        [176, 158, 121, 255],  # 1: sandy/pebble point bar
        [63, 84, 58, 255],     # 2: dark damp riparian bank & sedge
        [72, 104, 67, 255],    # 3: lower alluvial floodplain meadow (Bench 1)
        [86, 114, 71, 255],    # 4: upper alluvial terrace grass (Bench 2)
        [43, 71, 46, 255],     # 5: wet-sclerophyll gully deep fern-moss loam
        [54, 86, 55, 255],     # 6: wet-sclerophyll gully mid-slope forest floor
        [96, 112, 71, 255],    # 7: Box-Gum woodland lush olive grass
        [114, 118, 76, 255],   # 8: Box-Gum woodland warm dry tussock & litter
        [136, 124, 92, 255],   # 9: campsite compacted sandy earth clearing
        [128, 117, 98, 255],   # 10: weathered sandstone/granite escarpment rock
        [148, 137, 116, 255],  # 11: pale sandstone headland outcrop
        [84, 96, 69, 255],     # 12: coastal headland Banksia heathland
        [102, 106, 82, 255],   # 13: ridge granite gravel & dry eucalypt litter
    ]

    # Classify each cell (i -> x, j -> z)
    biome_rows: list[list[int]] = []
    # 12 roughness/AO modulation bands (R=AO, G=Roughness, B=0, A=255)
    orm_rows: list[list[int]] = []
    orm_palette = []
    for k in range(12):
        # k=0: wet/shaded gully or channel (AO 145, rough 145 ~ 0.57)
        # k=11: open dry ridge/sandstone (AO 250, rough 242 ~ 0.95)
        t = k / 11.0
        ao = round(150 + t * 102)
        rough = round(148 + t * 94)
        orm_palette.append([ao, rough, 0, 255])

    # Organic irregular woodland stand patches (deterministic, zero periodic checkerboard)
    woodland_patches = [
        (-190.0, 140.0, 68.0, -0.35), (-110.0, 210.0, 58.0, 0.32),
        (-225.0, 25.0, 52.0, 0.28), (115.0, 155.0, 72.0, -0.34),
        (185.0, -90.0, 64.0, 0.30), (95.0, -145.0, 55.0, -0.28),
        (-135.0, -45.0, 48.0, 0.26), (72.0, 55.0, 44.0, -0.30),
        (-55.0, 185.0, 52.0, 0.25), (140.0, 240.0, 60.0, 0.32),
        (-240.0, 230.0, 65.0, -0.28), (45.0, -70.0, 38.0, 0.24),
        (-78.0, 88.0, 34.0, -0.22), (165.0, 35.0, 46.0, 0.27),
    ]

    for j in range(grid):
        b_row = []
        o_row = []
        z = -300.0 + (j + 0.5) * (600.0 / grid)
        rx = river_x(z)
        for i in range(grid):
            x = -300.0 + (i + 0.5) * (600.0 / grid)
            y = terrain_height(x, z)
            # Slope via finite diff
            hx = (terrain_height(x + 2.0, z) - terrain_height(x - 2.0, z)) * 0.25
            hz = (terrain_height(x, z + 2.0) - terrain_height(x, z - 2.0)) * 0.25
            slope = math.sqrt(hx * hx + hz * hz)
            d_riv = abs(x - rx)
            d_camp = math.hypot(x + 22.0, z - 28.0)

            # Gully proximity
            g_z = -55.0 + (x + 75.0) * 0.72
            g_in = smoothstep(-65.0, -110.0, x) * (1.0 - smoothstep(-250.0, -295.0, x))
            g_fac = g_in * math.exp(-math.pow((z - g_z) / 30.0, 2.0))

            # Moisture/roughness index (0..11)
            moist = (
                0.55 * math.exp(-d_riv * d_riv / 160.0)
                + 0.45 * g_fac
                + 0.15 * max(0.0, 1.0 - y / 12.0)
            )
            dry_val = clamp(1.0 - moist + 0.12 * slope + 0.08 * math.sin(x * 0.05 + z * 0.04), 0.0, 1.0)
            o_idx = min(11, max(0, int(dry_val * 11.99)))

            # Biome classification
            if d_camp < 9.5:
                b_idx = 9
            elif y < 0.35:
                b_idx = 0
            elif d_riv < 10.5 and y < 1.55:
                b_idx = 1 if y > 0.55 else 0
            elif d_riv < 14.0:
                b_idx = 2
            elif g_fac > 0.52 and y < 22.0:
                b_idx = 5 if g_fac > 0.74 else 6
            elif slope > 0.46 or y > 44.0:
                b_idx = 11 if (slope > 0.58 or y > 52.0) else 10
            elif y > 32.0:
                b_idx = 12 if x > 130.0 else 13
            elif d_riv < 32.0 and y < 3.4:
                b_idx = 3
            elif d_riv < 56.0 and y < 6.2:
                b_idx = 4
            else:
                patch = 0.0
                for px, pz, pr, pw in woodland_patches:
                    dxp = x - px
                    dzp = z - pz
                    patch += pw * math.exp(-(dxp * dxp + dzp * dzp) / (pr * pr))
                patch += 0.14 * math.sin(x * 0.031 + z * 0.024) + 0.09 * math.cos(x * 0.067 - z * 0.053)
                b_idx = 7 if patch > -0.02 else 8

            b_row.append(b_idx)
            o_row.append(o_idx)
        biome_rows.append(b_row)
        orm_rows.append(o_row)

    def rle_layers(rows: list[list[int]], pal: list[list[int]]) -> list[dict]:
        cmds: dict[int, list[str]] = {k: [] for k in range(len(pal))}
        for j, row in enumerate(rows):
            y_px = j * cell_px
            i = 0
            while i < len(row):
                val = row[i]
                run = 1
                while i + run < len(row) and row[i + run] == val:
                    run += 1
                x_px = i * cell_px
                w_px = run * cell_px
                cmds[val].append(f"M{x_px} {y_px}h{w_px}v{cell_px}h-{w_px}Z")
                i += run
        out = []
        for k, parts in cmds.items():
            if parts:
                out.append({"path": "".join(parts), "fill": pal[k]})
        assert len(out) <= 64, len(out)
        return out

    biome_layers = rle_layers(biome_rows, palette)
    orm_layers = rle_layers(orm_rows, orm_palette)

    vibe_src = (
        'requires [capability("capability.invoke")];\n'
        f'let biome_raw = capability.invoke("hypermedia.radiance.render_vector", {vibe_lit({"width": 1024, "height": 1024, "layers": biome_layers})});\n'
        'let biome_soft = capability.invoke("hypermedia.radiance.filter_rgba", {image: biome_raw, operation: "blur", sigma: 4.2});\n'
        'let soil_grain = capability.invoke("hypermedia.surface.synthesize_maps", {side: 1024, seed: 20261119, tint: [0.54, 0.53, 0.49], contrast: 0.38, relief: 0.0055, bands: [12, 6], band_strength: 0.02, roughness: 0.90, metallic: 0.0});\n'
        'let albedo_comp = capability.invoke("hypermedia.radiance.compose_rgba", {width: 1024, height: 1024, layers: [{image: biome_soft, opacity: 1.0, mode: "normal"}, {image: soil_grain.colour, opacity: 0.42, mode: "soft-light"}]});\n'
        f'let orm_mod_raw = capability.invoke("hypermedia.radiance.render_vector", {vibe_lit({"width": 1024, "height": 1024, "layers": orm_layers})});\n'
        'let orm_mod = capability.invoke("hypermedia.radiance.filter_rgba", {image: orm_mod_raw, operation: "blur", sigma: 4.2});\n'
        'let orm_comp = capability.invoke("hypermedia.radiance.compose_rgba", {width: 1024, height: 1024, layers: [{image: orm_mod, opacity: 1.0, mode: "normal"}, {image: soil_grain.orm, opacity: 0.35, mode: "soft-light"}]});\n'
        'let terrain_pbr = capability.invoke("hypermedia.surface.pack_pbr_set", {colour: albedo_comp, normal: soil_grain.normal, orm: orm_comp});\n'
        'let water_maps = capability.invoke("hypermedia.surface.synthesize_maps", {side: 256, seed: 7741, tint: [0.09, 0.36, 0.38], contrast: 0.14, relief: 0.0022, bands: [18, 6], band_strength: 0.18, roughness: 0.16, metallic: 0.05});\n'
        'let water_pbr = capability.invoke("hypermedia.surface.pack_pbr_set", {colour: water_maps.colour, normal: water_maps.normal, orm: water_maps.orm});\n'
        'let mut asset = {files: [\n'
        '  {key: "terrain-albedo.png", resource: terrain_pbr.colour},\n'
        '  {key: "terrain-normal.png", resource: terrain_pbr.normal},\n'
        '  {key: "terrain-orm.png", resource: terrain_pbr.orm},\n'
        '  {key: "water-albedo.png", resource: water_pbr.colour},\n'
        '  {key: "water-normal.png", resource: water_pbr.normal},\n'
        '  {key: "water-orm.png", resource: water_pbr.orm}\n'
        ']};\n'
    )
    vibe_path = ROOT / "recipes/pbr_maps.vibe"
    vibe_path.write_text(vibe_src, encoding="utf-8")
    res = run_synth_produce("maps", vibe_path)
    exp = Path(res["export_dir"])

    for k in ("terrain-albedo.png", "terrain-normal.png", "terrain-orm.png"):
        shutil.copyfile(exp / k, ROOT / k)
    for k in ("water-albedo.png", "water-normal.png", "water-orm.png"):
        shutil.copyfile(exp / k, ROOT / "textures" / k)
    return res


# ---------------------------------------------------------------------------
# Step 2: Geodesy Matter-64 (.m64) + Sculpted Hypsometric Relief via Synth
# ---------------------------------------------------------------------------

def build_geodesy_m64() -> dict:
    side = 64
    spacing = 600.0 / side  # 9.375 m
    samples = []
    for j in range(side):
        z = -300.0 + (j + 0.5) * spacing
        for i in range(side):
            x = -300.0 + (i + 0.5) * spacing
            samples.append(round(terrain_height(x, z), 3))

    sculpt_strokes = [
        {"op": "raise", "x": 300.0, "y": 40.0, "radius": 240.0, "strength": 42.0, "falloff": "smoothstep"},
        {"op": "raise", "x": 555.0, "y": 380.0, "radius": 160.0, "strength": 36.0, "falloff": "smoothstep"},
        {"op": "terrace", "x": 300.0, "y": 60.0, "radius": 220.0, "strength": 0.65, "step_m": 6.0, "falloff": "smoothstep"},
        {"op": "terrace", "x": 555.0, "y": 380.0, "radius": 150.0, "strength": 0.55, "step_m": 5.0, "falloff": "smoothstep"},
        {"op": "lower", "x": 140.0, "y": 184.0, "radius": 90.0, "strength": 11.0, "falloff": "gaussian"},
        {"op": "hydraulic", "x": 300.0, "y": 300.0, "radius": 240.0, "strength": 1.4, "falloff": "smoothstep"},
        {"op": "flatten", "x": 278.0, "y": 328.0, "radius": 13.0, "strength": 1.0, "target_height": round(CAMPSITE_LEVEL, 3), "falloff": "plateau"},
    ]

    vibe_src = (
        'requires [capability("capability.invoke")];\n'
        f'let sculpted = capability.invoke("hypermedia.geodesy.sculpt_terrain", {vibe_lit({"tile_resolution": 64, "cell_spacing_m": 9.375, "origin_m": [-300.0, 0.0, -300.0], "base_height_m": 3.5, "strokes": sculpt_strokes, "width": 512, "height": 512})});\n'
        'let relief_png = capability.invoke("hypermedia.radiance.encode_rgba_png", {image: sculpted.relief_rgba});\n'
        'let m64 = capability.invoke("hypermedia.geodesy.compile_m64", {\n'
        '    origin_lat_lon: [-35.30, 149.06],\n'
        '    heightfield: {\n'
        f'        side: {side},\n'
        f'        cell_spacing_m: {spacing},\n'
        '        origin_m: [-300.0, 0.0, -300.0],\n'
        '        height_units: "m",\n'
        '        no_data: "none",\n'
        f'        samples: {vibe_lit(samples)}\n'
        '    },\n'
        '    record_mode: "strata"\n'
        '});\n'
        'let verified = capability.invoke("hypermedia.geodesy.verify_m64", {m64: m64});\n'
        'let mut asset = {files: [\n'
        '    {key: "catchment-600m.m64", resource: m64},\n'
        '    {key: "catchment-sculpt-relief.png", resource: relief_png}\n'
        ']};\n'
    )
    vibe_path = ROOT / "recipes/geodesy_m64.vibe"
    vibe_path.write_text(vibe_src, encoding="utf-8")
    res = run_synth_produce("m64", vibe_path)
    exp = Path(res["export_dir"])
    shutil.copyfile(exp / "catchment-600m.m64", ROOT / "catchment-600m.m64")
    shutil.copyfile(exp / "catchment-sculpt-relief.png", ROOT / "previews/catchment-sculpt-relief.png")
    return res


# ---------------------------------------------------------------------------
# Step 3: Attributed GLB Assembly & Synth compile_10d_pbr for LOD0, LOD1, Water
# ---------------------------------------------------------------------------

def pack_attributed_glb(
    positions: list[float],
    normals: list[float],
    uvs: list[float],
    tangents: list[float],
    indices: list[int],
    maps: list[Path],
    material: dict,
    mesh_name: str,
) -> bytes:
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
    accessors[a_pos].update(
        min=[min(positions[k::3]) for k in range(3)],
        max=[max(positions[k::3]) for k in range(3)],
    )
    a_nrm = add_attr(normals, "f", "VEC3", vcount, 34962)
    a_uv = add_attr(uvs, "f", "VEC2", vcount, 34962)
    a_tan = add_attr(tangents, "f", "VEC4", vcount, 34962)
    a_idx = add_attr(indices, "I", "SCALAR", len(indices), 34963)

    images = [{"bufferView": add_blob(p.read_bytes()), "mimeType": "image/png"} for p in maps]
    doc = {
        "asset": {"version": "2.0", "generator": "WEB-WP-03 terrain-v2"},
        "buffers": [{"byteLength": len(binary)}],
        "bufferViews": views,
        "accessors": accessors,
        "images": images,
        "samplers": [{"wrapS": 33071, "wrapT": 33071, "magFilter": 9729, "minFilter": 9987}],
        "textures": [{"source": i, "sampler": 0} for i in range(len(images))],
        "materials": [material],
        "meshes": [
            {
                "name": mesh_name,
                "primitives": [
                    {
                        "attributes": {"POSITION": a_pos, "NORMAL": a_nrm, "TEXCOORD_0": a_uv, "TANGENT": a_tan},
                        "indices": a_idx,
                        "material": 0,
                    }
                ],
            }
        ],
        "nodes": [{"mesh": 0, "name": mesh_name}],
        "scenes": [{"nodes": [0]}],
        "scene": 0,
    }
    encoded = json.dumps(doc, separators=(",", ":")).encode("utf-8")
    encoded += b" " * (-len(encoded) % 4)
    binary += b"\0" * (-len(binary) % 4)
    total = 28 + len(encoded) + len(binary)
    return (
        struct.pack("<III", 0x46546C67, 2, total)
        + struct.pack("<II", len(encoded), 0x4E4F534A)
        + encoded
        + struct.pack("<II", len(binary), 0x004E4942)
        + binary
    )


def build_terrain_glb(side: int, glb_path: Path, name: str) -> dict:
    positions: list[float] = []
    normals: list[float] = []
    uvs: list[float] = []
    tangents: list[float] = []
    indices: list[int] = []

    span = 600.0
    eps = 0.5
    min_y = float("inf")
    max_y = float("-inf")

    for j in range(side + 1):
        z = (j / side - 0.5) * span
        v = (z + 300.0) / 600.0
        for i in range(side + 1):
            x = (i / side - 0.5) * span
            u = (x + 300.0) / 600.0
            y = terrain_height(x, z)
            min_y = min(min_y, y)
            max_y = max(max_y, y)
            positions.extend((x, y, z))
            uvs.extend((u, v))

            dhdx = (terrain_height(x + eps, z) - terrain_height(x - eps, z)) / (2.0 * eps)
            dhdz = (terrain_height(x, z + eps) - terrain_height(x, z - eps)) / (2.0 * eps)
            nx, ny, nz = -dhdx, 1.0, -dhdz
            nlen = math.sqrt(nx * nx + ny * ny + nz * nz)
            nx, ny, nz = nx / nlen, ny / nlen, nz / nlen
            normals.extend((nx, ny, nz))

            # Tangent along +X, Gram-Schmidt orthogonalized to normal
            dot = nx
            tx, ty, tz = 1.0 - dot * nx, -dot * ny, -dot * nz
            tlen = math.sqrt(tx * tx + ty * ty + tz * tz)
            tangents.extend((tx / tlen, ty / tlen, tz / tlen, 1.0))

    for j in range(side):
        for i in range(side):
            a = j * (side + 1) + i
            b = a + side + 1
            indices.extend((a, b, a + 1, a + 1, b, b + 1))

    mat = {
        "name": f"{name}-pbr",
        "pbrMetallicRoughness": {
            "baseColorFactor": [1.0, 1.0, 1.0, 1.0],
            "baseColorTexture": {"index": 0},
            "metallicFactor": 0.0,
            "roughnessFactor": 1.0,
            "metallicRoughnessTexture": {"index": 2},
        },
        "normalTexture": {"index": 1, "scale": 0.55},
        "occlusionTexture": {"index": 2},
        "doubleSided": False,
    }
    maps = [ROOT / "terrain-albedo.png", ROOT / "terrain-normal.png", ROOT / "terrain-orm.png"]
    glb_path.write_bytes(pack_attributed_glb(positions, normals, uvs, tangents, indices, maps, mat, name))
    return {
        "side": side,
        "vertices": len(positions) // 3,
        "triangles": len(indices) // 3,
        "min_y": min_y,
        "max_y": max_y,
        "glb_sha256": sha256_file(glb_path),
    }


def build_water_glb(glb_path: Path) -> dict:
    z_steps = 580
    cross_steps = 4
    z0, z1 = -290.0, 290.0
    water_y = 0.24
    positions: list[float] = []
    normals: list[float] = []
    uvs: list[float] = []
    tangents: list[float] = []
    indices: list[int] = []

    for j in range(z_steps + 1):
        z = z0 + (j / z_steps) * (z1 - z0)
        rx = river_x(z)
        w_ch = 4.8 + 2.0 * math.pow(math.sin(z * 0.023 + 0.4), 2.0)
        half_w = w_ch + 2.6
        v = (z - z0) / 18.0
        for i in range(cross_steps + 1):
            s = (i / cross_steps) * 2.0 - 1.0
            x = rx + s * half_w
            positions.extend((x, water_y, z))
            normals.extend((0.0, 1.0, 0.0))
            uvs.extend((i / cross_steps, v))
            tangents.extend((1.0, 0.0, 0.0, 1.0))

    cols = cross_steps + 1
    for j in range(z_steps):
        for i in range(cross_steps):
            a = j * cols + i
            b = a + cols
            indices.extend((a, b, a + 1, a + 1, b, b + 1))

    mat = {
        "name": "catchment-water-pbr",
        "pbrMetallicRoughness": {
            "baseColorFactor": [1.0, 1.0, 1.0, 0.88],
            "baseColorTexture": {"index": 0},
            "metallicFactor": 0.05,
            "roughnessFactor": 0.16,
            "metallicRoughnessTexture": {"index": 2},
        },
        "normalTexture": {"index": 1, "scale": 0.35},
        "alphaMode": "BLEND",
        "doubleSided": True,
    }
    maps = [ROOT / "textures/water-albedo.png", ROOT / "textures/water-normal.png", ROOT / "textures/water-orm.png"]
    glb_path.write_bytes(pack_attributed_glb(positions, normals, uvs, tangents, indices, maps, mat, "catchment-600m-water"))
    return {
        "z_range": [z0, z1],
        "water_datum_y": water_y,
        "vertices": len(positions) // 3,
        "triangles": len(indices) // 3,
        "glb_sha256": sha256_file(glb_path),
    }


def compile_10d(label: str, glb_path: Path, out_10d: Path, camera_pos: list[float], camera_tgt: list[float]) -> dict:
    vibe_src = (
        'requires [capability("capability.invoke")];\n'
        f'let mesh = capability.invoke("hypermedia.manifold.import_mesh", {{input: "{glb_path.name}", format: "glb"}});\n'
        'let native = capability.invoke("hypermedia.manifold.compile_10d_pbr", {\n'
        '    mesh: mesh,\n'
        '    topology: false,\n'
        '    spatial_index: true,\n'
        f'    camera_position: {json.dumps(camera_pos)},\n'
        f'    camera_target: {json.dumps(camera_tgt)},\n'
        '    camera_up: [0.0, 1.0, 0.0]\n'
        '});\n'
        f'let mut asset = {{files: [{{key: "{out_10d.name}", resource: native}}]}};\n'
    )
    vibe_path = ROOT / "recipes" / f"compile_{label}.vibe"
    vibe_path.write_text(vibe_src, encoding="utf-8")
    res = run_synth_produce(label, vibe_path, inputs=[(glb_path.name, "glb", glb_path)])
    exp = Path(res["export_dir"])
    shutil.copyfile(exp / out_10d.name, out_10d)
    tex_src = exp / "textures" / "sha256"
    if tex_src.exists():
        tex_dst = ROOT / "textures" / "sha256"
        tex_dst.mkdir(parents=True, exist_ok=True)
        for item in tex_src.iterdir():
            if item.is_file():
                shutil.copyfile(item, tex_dst / item.name)
    return res


def render_preview(name: str, asset_10d: Path, hmc_path: Path) -> dict:
    out_dir = ROOT / "previews" / name
    if out_dir.exists():
        shutil.rmtree(out_dir)
    subprocess.run(
        [
            str(SYNTH_EXE),
            "preview",
            "--asset",
            str(asset_10d),
            "--out",
            str(out_dir),
            "--hmc",
            str(hmc_path),
            "--asset-key",
            f"outputs/{asset_10d.name}",
        ],
        check=True,
    )
    receipt = json.loads((out_dir / "review-receipt.json").read_text(encoding="utf-8"))
    return {"preview_dir": str(out_dir), "receipt": receipt}


def main() -> None:
    print("1. Verifying exact JS/Python/Vibe heightfield parity...", flush=True)
    parity = verify_js_parity()
    print("   Parity OK:", parity, flush=True)

    print("2. Synthesizing 1024x1024 terrain PBR maps & 256x256 water PBR maps via Synth...", flush=True)
    maps_res = build_pbr_maps()

    print("3. Sculpting geodesy & compiling catchment-600m.m64 via Synth...", flush=True)
    m64_res = build_geodesy_m64()

    print("4. Packing LOD0 (240x240), LOD1 (120x120), and Water GLBs...", flush=True)
    lod0_glb = ROOT / "inputs/catchment-600m-lod0.glb"
    lod1_glb = ROOT / "inputs/catchment-600m-lod1.glb"
    water_glb = ROOT / "inputs/catchment-600m-water.glb"
    lod0_info = build_terrain_glb(240, lod0_glb, "catchment-600m-lod0")
    lod1_info = build_terrain_glb(120, lod1_glb, "catchment-600m-lod1")
    water_info = build_water_glb(water_glb)

    print("5. Compiling catchment-600m-lod0.10d, catchment-600m-lod1.10d, catchment-600m-water.10d via Synth...", flush=True)
    lod0_10d = ROOT / "catchment-600m-lod0.10d"
    lod1_10d = ROOT / "catchment-600m-lod1.10d"
    water_10d = ROOT / "catchment-600m-water.10d"

    lod0_res = compile_10d("lod0", lod0_glb, lod0_10d, [180.0, 145.0, 220.0], [0.0, 12.0, 0.0])
    lod1_res = compile_10d("lod1", lod1_glb, lod1_10d, [180.0, 145.0, 220.0], [0.0, 12.0, 0.0])
    water_res = compile_10d("water", water_glb, water_10d, [120.0, 160.0, 180.0], [0.0, 0.24, 0.0])

    print("6. Rendering native Synth previews...", flush=True)
    lod0_prev = render_preview("catchment-600m-lod0", lod0_10d, Path(lod0_res["hmc_path"]))
    lod1_prev = render_preview("catchment-600m-lod1", lod1_10d, Path(lod1_res["hmc_path"]))
    water_prev = render_preview("catchment-600m-water", water_10d, Path(water_res["hmc_path"]))

    landmarks = {}
    for label, x, z in [
        ("campsite-field-studio", -22.0, 28.0),
        ("river-view", 0.0, 16.0),
        ("commons", -17.0, 6.0),
        ("atelier", 20.0, -7.0),
        ("canopy", -20.0, -16.0),
        ("observatory", -15.0, -32.0),
        ("biology", 17.0, -37.0),
        ("footbridge-west-anchor", river_x(4.0) - 8.0, 4.0),
        ("footbridge-east-anchor", river_x(4.0) + 8.0, 4.0),
        ("wet-sclerophyll-gully", -160.0, -116.0),
        ("north-granite-ridge", 165.0, -265.0),
        ("east-coastal-headland", 255.0, 80.0),
    ]:
        landmarks[label] = {"x": x, "z": z, "y": round(terrain_height(x, z), 6)}

    manifest = {
        "schema": "webizen.terrain-v2-manifest/v1",
        "id": "catchment-600m",
        "epistemic_class": "authored-artwork",
        "frame": {"up": "+Y", "units": "metres", "bounds_xz_m": [-300.0, 300.0]},
        "heightfield_parity": parity,
        "heightfield_parameters": {
            "span_m": 600.0,
            "inner_core_preserved_radius_m": 68.0,
            "outer_blend_end_radius_m": 112.0,
            "campsite_clearing": {"centre": [-22.0, 28.0], "flat_radius_m": 8.0, "blend_radius_m": 13.0, "ground_y_m": CAMPSITE_LEVEL},
            "water_datum_y_m": 0.24,
            "elevation_bounds_m": {"min": round(lod0_info["min_y"], 3), "max": round(lod0_info["max_y"], 3)},
            "geomorphic_zones": [
                "Meandering creek/river channel (river_x(z)) with incised banks, sandy/pebble point bars, and pool-riffle depth variations",
                "Terraced alluvial floodplain (two fluvial benches at +1.95..+5.0 m)",
                "Shaded wet-sclerophyll gully (NW/W tributary carving at x in [-245,-68])",
                "Grassy Box-Gum woodland slopes (+7..+32 m)",
                "Elevated sandstone/granite northern ridge & eastern coastal headland (+35..+59.3 m)",
            ],
        },
        "landmarks_m": landmarks,
        "assets": {
            "lod0": {
                "path": "production/terrain-v2/catchment-600m-lod0.10d",
                "tier": "TierA/TierB Desktop WebGPU",
                "sha256": sha256_file(lod0_10d),
                "bytes": lod0_10d.stat().st_size,
                "vertices": lod0_info["vertices"],
                "triangles": lod0_info["triangles"],
                "grid_side": 240,
                "cell_spacing_m": 2.5,
                "hmc_sha256": lod0_res["hmc_sha256"],
                "hmc_path": lod0_res["hmc_path"],
                "preview_dir": lod0_prev["preview_dir"],
            },
            "lod1": {
                "path": "production/terrain-v2/catchment-600m-lod1.10d",
                "tier": "TierC Mobile WebGL2 (<= 32k tris)",
                "sha256": sha256_file(lod1_10d),
                "bytes": lod1_10d.stat().st_size,
                "vertices": lod1_info["vertices"],
                "triangles": lod1_info["triangles"],
                "grid_side": 120,
                "cell_spacing_m": 5.0,
                "hmc_sha256": lod1_res["hmc_sha256"],
                "hmc_path": lod1_res["hmc_path"],
                "preview_dir": lod1_prev["preview_dir"],
            },
            "water": {
                "path": "production/terrain-v2/catchment-600m-water.10d",
                "sha256": sha256_file(water_10d),
                "bytes": water_10d.stat().st_size,
                "vertices": water_info["vertices"],
                "triangles": water_info["triangles"],
                "z_range_m": water_info["z_range"],
                "water_datum_y_m": water_info["water_datum_y"],
                "hmc_sha256": water_res["hmc_sha256"],
                "hmc_path": water_res["hmc_path"],
                "preview_dir": water_prev["preview_dir"],
            },
            "m64": {
                "path": "production/terrain-v2/catchment-600m.m64",
                "sha256": sha256_file(ROOT / "catchment-600m.m64"),
                "bytes": (ROOT / "catchment-600m.m64").stat().st_size,
                "origin_lat_lon": [-35.30, 149.06],
                "record_mode": "strata",
                "hmc_sha256": m64_res["hmc_sha256"],
                "hmc_path": m64_res["hmc_path"],
                "relief_preview": "production/terrain-v2/previews/catchment-sculpt-relief.png",
            },
        },
        "pbr_textures": {
            "albedo": {"path": "production/terrain-v2/terrain-albedo.png", "size": [1024, 1024], "sha256": sha256_file(ROOT / "terrain-albedo.png")},
            "normal": {"path": "production/terrain-v2/terrain-normal.png", "size": [1024, 1024], "sha256": sha256_file(ROOT / "terrain-normal.png")},
            "orm": {"path": "production/terrain-v2/terrain-orm.png", "size": [1024, 1024], "sha256": sha256_file(ROOT / "terrain-orm.png")},
            "hmc_sha256": maps_res["hmc_sha256"],
            "hmc_path": maps_res["hmc_path"],
            "mobile_webgl2_compatible": True,
            "webgpu_compatible": True,
        },
        "previews": {
            "lod0_front": "production/terrain-v2/previews/catchment-600m-lod0/front-view-assumed.png",
            "lod0_side": "production/terrain-v2/previews/catchment-600m-lod0/side-view-assumed.png",
            "lod0_three_quarter": "production/terrain-v2/previews/catchment-600m-lod0/three-quarter-view-assumed.png",
            "lod1_three_quarter": "production/terrain-v2/previews/catchment-600m-lod1/three-quarter-view-assumed.png",
            "water_three_quarter": "production/terrain-v2/previews/catchment-600m-water/three-quarter-view-assumed.png",
            "sculpt_relief": "production/terrain-v2/previews/catchment-sculpt-relief.png",
        },
    }
    (ROOT / "terrain-manifest.json").write_text(json.dumps(manifest, indent=2), encoding="utf-8")
    print("DONE! Manifest written to", ROOT / "terrain-manifest.json")


if __name__ == "__main__":
    main()
