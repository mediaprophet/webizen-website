"""WEB-WP-08: Assemble the 600 m x 600 m SE Australian living catchment world.

Reads:
- production/species-cohort/species-catalog.json (WEB-WP-02)
- production/terrain-v2/terrain-manifest.json (WEB-WP-03)
- C:\\github\\eco-intent\\<slug>\\ontology\\taxon.ttl (for custodial Country & trait provenance)

Places all 13 verified intenteco species in ecologically authentic zones across the
600 m x 600 m catchment using deterministic InstanceState64 seeds and exact
heightfield parity (y = terrain_height(x, z)), avoiding the campsite clearing (-22, 28)
and footbridge (z = 4).

Anti-Monolith Rule:
All species .10d, .q42, .emf, .m64, and .qac references use
`/intenteco/<species-slug>/dist/...` plus `remote_url` on GitHub (`intenteco/<slug>`).
No species binaries are copied into `webizen-website`.

Emits:
- data/ewm/places/se-australia-catchment.json
- data/ewm/places/se-australia-catchment.ttl
(Sealed .q42 is compiled via synth_compile_q42 from the emitted .ttl.)
"""
from __future__ import annotations

import hashlib
import json
import math
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[1]
TERRAIN_DIR = REPO / "production" / "terrain-v2"
ECO_INTENT_ROOT = Path(r"C:\github\eco-intent")

sys.path.insert(0, str(TERRAIN_DIR))
import build_terrain_v2 as tv2  # noqa: E402

SPECIES_CATALOG_JSON = ROOT / "species-catalog.json"
TERRAIN_MANIFEST_JSON = TERRAIN_DIR / "terrain-manifest.json"
OUT_DIR = REPO / "data" / "ewm" / "places"
OUT_JSON = OUT_DIR / "se-australia-catchment.json"
OUT_TTL = OUT_DIR / "se-australia-catchment.ttl"

# Locomotion / guild metadata keyed by species slug ONLY at offline assembly time
# so the emitted Q42 volume carries purely data-driven traits for the generic Rust runtime.
SPECIES_ECOLOGY_META = {
    "eucalyptus-camaldulensis": {
        "zone": "riparian-floodplain",
        "locomotion_class": "sessile_flora",
        "sway_hz": 0.66,
        "mycorrhizal_type": "ECM (Common Mycorrhizal Network / Hartig net sheaths)",
        "rhizosphere_signaling": "0.1-50 Hz mycelial action potentials; hydraulic lift; phenolic, glomalin & C/P/N exudate fluxes",
        "target_count": 34,
        "root_radius_m": 1.6,
    },
    "ornithorhynchus-anatinus": {
        "zone": "riparian-floodplain",
        "locomotion_class": "semi_aquatic",
        "sway_hz": 0.0,
        "mycorrhizal_type": "n/a",
        "rhizosphere_signaling": "",
        "target_count": 10,
    },
    "dicksonia-antarctica": {
        "zone": "wet-sclerophyll-gully",
        "locomotion_class": "sessile_flora",
        "sway_hz": 0.85,
        "mycorrhizal_type": "AM + Saprotrophic Bryophyte-Fungal Guild",
        "rhizosphere_signaling": "0.1-50 Hz mycelial action potentials; gully moisture buffering; glomalin & C/P/N exudate fluxes",
        "target_count": 34,
        "root_radius_m": 0.6,
    },
    "eucalyptus-regnans": {
        "zone": "wet-sclerophyll-gully",
        "locomotion_class": "sessile_flora",
        "sway_hz": 0.42,
        "mycorrhizal_type": "Wet-Sclerophyll ECM (Basidiomycota / Ascomycota)",
        "rhizosphere_signaling": "0.1-50 Hz mycelial action potentials; deep hydraulic lift; glomalin & C/P/N exudate fluxes",
        "target_count": 22,
        "root_radius_m": 1.6,
    },
    "phascolarctos-cinereus": {
        "zone": "wet-sclerophyll-gully",
        "locomotion_class": "arboreal",
        "sway_hz": 0.0,
        "mycorrhizal_type": "n/a",
        "rhizosphere_signaling": "",
        "target_count": 8,
    },
    "eucalyptus-melliodora": {
        "zone": "box-gum-woodland",
        "locomotion_class": "sessile_flora",
        "sway_hz": 0.62,
        "mycorrhizal_type": "Woodland ECM (Basidiomycota / Ascomycota)",
        "rhizosphere_signaling": "0.1-50 Hz mycelial action potentials; taproot hydraulic lift; glomalin & C/P/N exudate fluxes",
        "target_count": 40,
        "root_radius_m": 1.6,
    },
    "acacia-pycnantha": {
        "zone": "box-gum-woodland",
        "locomotion_class": "sessile_flora",
        "sway_hz": 0.92,
        "mycorrhizal_type": "AM + Rhizobium N2-fixing Root Nodules",
        "rhizosphere_signaling": "0.1-50 Hz mycelial action potentials; symbiotic N fixation; glomalin & C/P/N exudate fluxes",
        "target_count": 40,
        "root_radius_m": 0.6,
    },
    "wallabia-bicolor": {
        "zone": "box-gum-woodland",
        "locomotion_class": "saltatorial",
        "sway_hz": 0.0,
        "mycorrhizal_type": "n/a",
        "rhizosphere_signaling": "",
        "target_count": 12,
    },
    "macropus-giganteus": {
        "zone": "box-gum-woodland",
        "locomotion_class": "saltatorial",
        "sway_hz": 0.0,
        "mycorrhizal_type": "n/a",
        "rhizosphere_signaling": "",
        "target_count": 18,
    },
    "vombatus-ursinus": {
        "zone": "box-gum-woodland",
        "locomotion_class": "fossorial",
        "sway_hz": 0.0,
        "mycorrhizal_type": "n/a",
        "rhizosphere_signaling": "",
        "target_count": 10,
    },
    "dacelo-novaeguineae": {
        "zone": "box-gum-woodland",
        "locomotion_class": "avian",
        "sway_hz": 0.0,
        "mycorrhizal_type": "n/a",
        "rhizosphere_signaling": "",
        "target_count": 10,
    },
    "eucalyptus-dalrympleana": {
        "zone": "granite-ridge-headland",
        "locomotion_class": "sessile_flora",
        "sway_hz": 0.58,
        "mycorrhizal_type": "Subalpine ECM (Basidiomycota / Ascomycota)",
        "rhizosphere_signaling": "0.1-50 Hz mycelial action potentials; granite fissure lift; glomalin & C/P/N exudate fluxes",
        "target_count": 22,
        "root_radius_m": 1.6,
    },
    "banksia-integrifolia": {
        "zone": "granite-ridge-headland",
        "locomotion_class": "sessile_flora",
        "sway_hz": 0.78,
        "mycorrhizal_type": "Proteoid Cluster Rootlets (non-mycorrhizal P mining)",
        "rhizosphere_signaling": "0.1-50 Hz root bio-electric potentials; citrate/malate P-mobilizing exudate fluxes & hydraulic lift",
        "target_count": 22,
        "root_radius_m": 1.2,
    },
    "poa-labillardierei": {
        "zone": "riparian-floodplain",
        "locomotion_class": "sessile_flora",
        "sway_hz": 1.45,
        "mycorrhizal_type": "AM Tussock Grass Root Symbiosis",
        "rhizosphere_signaling": "0.1-30 Hz fibrous rhizosphere exudate & moisture buffering pulses",
        "target_count": 200,
        "root_radius_m": 0.25,
    },
    "themeda-triandra": {
        "zone": "box-gum-woodland",
        "locomotion_class": "sessile_flora",
        "sway_hz": 1.55,
        "mycorrhizal_type": "AM C4 Grassland Symbiosis",
        "rhizosphere_signaling": "0.1-30 Hz grassland C/P exudate pulses & soil aggregate binding",
        "target_count": 180,
        "root_radius_m": 0.25,
    },
    "lomandra-longifolia": {
        "zone": "riparian-floodplain",
        "locomotion_class": "sessile_flora",
        "sway_hz": 1.25,
        "mycorrhizal_type": "AM Riparian Sedgeland Symbiosis",
        "rhizosphere_signaling": "0.1-35 Hz riparian bank stabilization & exudate fluxes",
        "target_count": 120,
        "root_radius_m": 0.25,
    },
    "bursaria-spinosa": {
        "zone": "box-gum-woodland",
        "locomotion_class": "sessile_flora",
        "sway_hz": 0.98,
        "mycorrhizal_type": "AM + Woodland Understorey Shrub Guild",
        "rhizosphere_signaling": "0.1-40 Hz understorey root bio-electric & pollinator-nurse fluxes",
        "target_count": 90,
        "root_radius_m": 0.6,
    },
}

STOP_POINTS = [
    (-22.0, 28.0, 15.0),   # campsite clearing (15 m radius clear)
    (0.0, 16.0, 6.0),      # river-view
    (-17.0, 6.0, 6.0),     # commons
    (20.0, -7.0, 6.0),     # atelier
    (-20.0, -16.0, 6.0),   # canopy
    (-15.0, -32.0, 6.0),   # observatory
    (17.0, -37.0, 6.0),    # biology
]


# ---------------------------------------------------------------------------
# Exact SplitMix64 & InstanceState64 parity with native/world-host/src/runtime/instances.rs
# ---------------------------------------------------------------------------

MASK64 = 0xFFFFFFFFFFFFFFFF


def splitmix64_next(state: int) -> tuple[int, int]:
    state = (state + 0x9E3779B97F4A7C15) & MASK64
    z = state
    z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & MASK64
    z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & MASK64
    return (z ^ (z >> 31)) & MASK64, state


def next_unit_f64(state: int) -> tuple[float, int]:
    val, state = splitmix64_next(state)
    return (val >> 11) * (1.0 / (1 << 53)), state


def next_signed_f64(state: int) -> tuple[float, int]:
    u, state = next_unit_f64(state)
    return u * 2.0 - 1.0, state


def quat_from_yaw_and_lean(yaw: float, pitch_x: float, roll_z: float) -> list[float]:
    sy, cy = math.sin(yaw * 0.5), math.cos(yaw * 0.5)
    sx, cx = math.sin(pitch_x * 0.5), math.cos(pitch_x * 0.5)
    sz, cz = math.sin(roll_z * 0.5), math.cos(roll_z * 0.5)
    x = cy * sx * cz + sy * cx * sz
    y = sy * cx * cz - cy * sx * sz
    z = cy * cx * sz - sy * sx * cz
    w = cy * cx * cz + sy * sx * sz
    norm = math.sqrt(x * x + y * y + z * z + w * w)
    if norm > 1e-12:
        q = [x / norm, y / norm, z / norm, w / norm]
        return [round(v, 8) for v in q]
    return [0.0, 0.0, 0.0, 1.0]


def terrain_slope_rad(x: float, z: float) -> float:
    """Compute local terrain slope angle in radians via central differences at +/-1.0 m."""
    dhdx = (tv2.terrain_height(x + 1.0, z) - tv2.terrain_height(x - 1.0, z)) * 0.5
    dhdz = (tv2.terrain_height(x, z + 1.0) - tv2.terrain_height(x, z - 1.0)) * 0.5
    return math.atan(math.hypot(dhdx, dhdz))


def buried_flora_y(slug: str, x: float, z: float) -> float:
    """Sink flora root collar by `sin(slope_rad) * root_radius + 0.20 m` so roots never float on slopes."""
    root_radius = float(SPECIES_ECOLOGY_META[slug].get("root_radius_m", 0.6))
    slope = terrain_slope_rad(x, z)
    return tv2.terrain_height(x, z) - (math.sin(slope) * root_radius + 0.20)


def derive_placement_from_state64(
    base_pos: list[float],
    seed: int,
    age: float,
    height_scale: float,
    site_vigor: float,
    stress: list[float],
) -> dict:
    rng = seed & MASK64
    yaw, rng = next_unit_f64(rng)
    yaw *= math.tau
    max_lean_rad = 0.06 + 0.055 * stress[0]
    lean_x, rng = next_signed_f64(rng)
    lean_z, rng = next_signed_f64(rng)
    rotation = quat_from_yaw_and_lean(yaw, lean_x * max_lean_rad, lean_z * max_lean_rad)

    stress_mean = sum(stress) / 4.0
    ontogenetic_factor = 0.35 + 0.65 * math.pow(max(0.0, min(1.0, age)), 0.75)
    stress_damping = 1.0 - 0.18 * stress_mean
    vigor_factor = 0.75 + 0.25 * site_vigor

    rjx, rng = next_signed_f64(rng)
    rjz, rng = next_signed_f64(rng)
    hj, rng = next_signed_f64(rng)

    sy = max(0.15, min(4.0, ontogenetic_factor * height_scale * vigor_factor * stress_damping * (1.0 + 0.10 * hj)))
    radial_base = max(0.15, min(4.0, ontogenetic_factor * (0.8 + 0.2 * height_scale) * vigor_factor * stress_damping))
    sx = max(0.15, min(4.0, radial_base * (1.0 + 0.12 * rjx)))
    sz = max(0.15, min(4.0, radial_base * (1.0 + 0.12 * rjz)))

    return {
        "id": f"inst-{seed:016x}",
        "translation": [round(base_pos[0], 4), round(base_pos[1], 4), round(base_pos[2], 4)],
        "rotation": rotation,
        "scale": [round(sx, 5), round(sy, 5), round(sz, 5)],
    }


def deterministic_seed(species_slug: str, index: int) -> int:
    digest = hashlib.sha256(f"webizen:se-australia:{species_slug}:{index}".encode("utf-8")).digest()
    return int.from_bytes(digest[:8], "little")


def extract_custodial_country(slug: str) -> str:
    ttl_path = ECO_INTENT_ROOT / slug / "ontology" / "taxon.ttl"
    if not ttl_path.exists():
        return "SE Australian Country (Yuin, Ngarigo, Wiradjuri, Ngunnawal, Wurundjeri)"
    txt = ttl_path.read_text(encoding="utf-8", errors="replace")
    m = re.search(r"tk:custodial_country\s+(.+?)\s*\.", txt, re.S)
    if not m:
        return "SE Australian Country (custodial attribution in taxon.q42)"
    raw = m.group(1)
    quoted = re.findall(r'"([^"]+)"', raw)
    if quoted:
        return "; ".join(quoted[:5])
    iris = re.findall(r"country:([A-Za-z0-9_]+)", raw)
    if iris:
        formatted = []
        for token in iris[:6]:
            pretty = token.replace("_country", "").replace("_", " ").title()
            formatted.append(f"{pretty} Country")
        return ", ".join(formatted)
    return "SE Australian Country (custodial attribution in taxon.q42)"


def extract_call_summary(slug: str) -> tuple[str, float]:
    ttl_path = ECO_INTENT_ROOT / slug / "ontology" / "taxon.ttl"
    if not ttl_path.exists():
        return "", 0.0
    txt = ttl_path.read_text(encoding="utf-8", errors="replace")
    call_m = re.search(r'sosa:callName\s+"([^"]+)"', txt)
    hz_m = re.search(r'sosa:callFundamentalHz\s+"([^"]+)"', txt)
    call_name = call_m.group(1) if call_m else ""
    hz = float(hz_m.group(1)) if hz_m else 0.0
    return call_name, hz


def is_clear_of_infrastructure(x: float, z: float, radius: float = 1.5) -> bool:
    if abs(x) > 286.0 or abs(z) > 286.0:
        return False
    for sx, sz, sr in STOP_POINTS:
        if math.hypot(x - sx, z - sz) < sr + radius:
            return False
    # Footbridge at z = 4 across river
    if abs(z - 4.0) < 4.5 + radius and abs(x - tv2.river_x(4.0)) < 11.5 + radius:
        return False
    return True


def halton(index: int, base: int) -> float:
    f = 1.0
    r = 0.0
    i = index
    while i > 0:
        f /= base
        r += f * (i % base)
        i //= base
    return r


def candidate_points(offset_seed: int, count: int = 8000) -> list[tuple[float, float]]:
    pts = []
    for k in range(1, count + 1):
        u = halton(k + offset_seed, 2)
        v = halton(k + offset_seed, 3)
        x = (u * 2.0 - 1.0) * 284.0
        z = (v * 2.0 - 1.0) * 284.0
        pts.append((x, z))
    return pts


def too_close(x: float, z: float, existing: list[tuple[float, float]], min_dist: float) -> bool:
    min_sq = min_dist * min_dist
    for ex, ez in existing:
        dx = x - ex
        dz = z - ez
        if dx * dx + dz * dz < min_sq:
            return True
    return False


def _near_clearing_dist(x: float, z: float) -> float:
    return math.hypot(x + 22.0, z - 28.0)


def main() -> int:
    catalog = json.loads(SPECIES_CATALOG_JSON.read_text(encoding="utf-8"))
    terrain = json.loads(TERRAIN_MANIFEST_JSON.read_text(encoding="utf-8"))
    by_slug = {s["slug"]: s for s in catalog["species"]}
    assert len(by_slug) == 17, f"Expected 17 species, got {len(by_slug)}"

    # Separate spacing lists for canopy trees, shrubs, groundcover grasses, and fauna
    tree_positions: list[tuple[float, float]] = []
    shrub_positions: list[tuple[float, float]] = []
    grass_positions: list[tuple[float, float]] = []
    fauna_positions: list[tuple[float, float]] = []
    placed_by_slug: dict[str, list[dict]] = {slug: [] for slug in by_slug}
    species_trees_by_slug: dict[str, list[dict]] = {slug: [] for slug in by_slug}

    def add_individual(slug: str, x: float, y: float, z: float, idx: int, age: float, hscale: float, vigor: float, stress: list[float], zone: str, host_tree: str | None = None) -> None:
        seed = deterministic_seed(slug, idx)
        p = derive_placement_from_state64([x, y, z], seed, age, hscale, vigor, stress)
        qnorm = math.sqrt(sum(v * v for v in p["rotation"]))
        assert abs(qnorm - 1.0) < 1e-6, f"Non-unit quaternion for {slug}: {qnorm}"
        rec = {
            "id": p["id"],
            "species_slug": slug,
            "taxon_iri": by_slug[slug]["did"],
            "zone": zone,
            "translation": p["translation"],
            "rotation": p["rotation"],
            "scale": p["scale"],
            "instance_state_64": {
                "seed": seed,
                "ontogenetic_age_norm": round(age, 4),
                "height_scale": round(hscale, 4),
                "site_vigor": round(vigor, 4),
                "environmental_stress": [round(v, 4) for v in stress],
            },
        }
        if host_tree:
            rec["host_tree_id"] = host_tree
        placed_by_slug[slug].append(rec)
        if by_slug[slug]["kingdom"] == "flora":
            species_trees_by_slug[slug].append(rec)

    def place_clearing_or_catchment(pts: list[tuple[float, float]], min_r: float = 18.0, max_r: float = 160.0) -> list[tuple[float, float]]:
        inner = [(x, z) for (x, z) in pts if min_r <= _near_clearing_dist(x, z) <= max_r]
        outer = [(x, z) for (x, z) in pts if _near_clearing_dist(x, z) > max_r]
        return inner + outer

    # 1. Riparian Creek & Alluvial Floodplain (dist_to_river < 22 m)
    # 1a. Eucalyptus camaldulensis (River Red Gum)
    slug = "eucalyptus-camaldulensis"
    target = SPECIES_ECOLOGY_META[slug]["target_count"]
    for x, z in place_clearing_or_catchment(candidate_points(101, 14000), 18.0, 155.0):
        if len(placed_by_slug[slug]) >= target:
            break
        d_river = abs(x - tv2.river_x(z))
        if not (7.8 <= d_river < 22.0):
            continue
        if not is_clear_of_infrastructure(x, z, 3.0):
            continue
        surf_y = tv2.terrain_height(x, z)
        if surf_y < 0.70:
            continue
        if too_close(x, z, tree_positions, 11.5):
            continue
        idx = len(placed_by_slug[slug])
        u_age = 0.55 + 0.45 * halton(idx + 17, 5)
        hscale = 0.90 + 0.25 * halton(idx + 31, 7)
        vigor = 0.92 + 0.28 * (1.0 - (d_river - 7.8) / 14.2)
        stress = [0.12 + 0.10 * halton(idx + 5, 3), 0.05, 0.08, 0.10]
        add_individual(slug, x, buried_flora_y(slug, x, z), z, idx, u_age, hscale, vigor, stress, "riparian-floodplain")
        tree_positions.append((x, z))

    # 2. Shaded Wet-Sclerophyll Gully (NW/W flank x < -55, z < -25)
    # 2a. Eucalyptus regnans (Mountain Ash)
    slug = "eucalyptus-regnans"
    target = SPECIES_ECOLOGY_META[slug]["target_count"]
    for x, z in candidate_points(1409, 14000):
        if len(placed_by_slug[slug]) >= target:
            break
        if not (x < -55.0 and z < -25.0):
            continue
        d_river = abs(x - tv2.river_x(z))
        if d_river < 12.0 or not is_clear_of_infrastructure(x, z, 3.5):
            continue
        surf_y = tv2.terrain_height(x, z)
        if surf_y < 1.5 or surf_y > 24.0:
            continue
        if too_close(x, z, tree_positions, 13.5):
            continue
        idx = len(placed_by_slug[slug])
        u_age = 0.62 + 0.38 * halton(idx + 11, 5)
        hscale = 0.92 + 0.24 * halton(idx + 19, 7)
        vigor = 0.95 + 0.22 * halton(idx + 23, 3)
        stress = [0.08 + 0.06 * halton(idx + 9, 5), 0.04, 0.05, 0.06]
        add_individual(slug, x, buried_flora_y(slug, x, z), z, idx, u_age, hscale, vigor, stress, "wet-sclerophyll-gully")
        tree_positions.append((x, z))

    # 2b. Dicksonia antarctica (Soft Tree Fern)
    slug = "dicksonia-antarctica"
    target = SPECIES_ECOLOGY_META[slug]["target_count"]
    for x, z in place_clearing_or_catchment(candidate_points(2711, 16000), 18.0, 145.0):
        if len(placed_by_slug[slug]) >= target:
            break
        d_river = abs(x - tv2.river_x(z))
        in_gully = x < -55.0 and z < -25.0 and d_river >= 6.5
        in_sheltered_creek = 6.5 <= d_river <= 16.0 and _near_clearing_dist(x, z) <= 130.0
        if not (in_gully or in_sheltered_creek):
            continue
        if not is_clear_of_infrastructure(x, z, 1.5):
            continue
        surf_y = tv2.terrain_height(x, z)
        if surf_y < 0.9 or surf_y > 18.5:
            continue
        if too_close(x, z, tree_positions, 4.8):
            continue
        idx = len(placed_by_slug[slug])
        u_age = 0.50 + 0.50 * halton(idx + 13, 5)
        hscale = 0.85 + 0.30 * halton(idx + 29, 7)
        vigor = 0.95 + 0.20 * halton(idx + 41, 3)
        stress = [0.05, 0.03, 0.04, 0.05]
        add_individual(slug, x, buried_flora_y(slug, x, z), z, idx, u_age, hscale, vigor, stress, "wet-sclerophyll-gully")
        tree_positions.append((x, z))

    # 3. Elevated Northern Granite Ridge & Eastern Coastal Headland (y > 19 m)
    # 3a. Eucalyptus dalrympleana (Mountain White Gum)
    slug = "eucalyptus-dalrympleana"
    target = SPECIES_ECOLOGY_META[slug]["target_count"]
    for x, z in place_clearing_or_catchment(candidate_points(4003, 15000), 20.0, 160.0):
        if len(placed_by_slug[slug]) >= target:
            break
        surf_y = tv2.terrain_height(x, z)
        if surf_y <= 18.0 or z > 70.0:
            continue
        if not is_clear_of_infrastructure(x, z, 3.0):
            continue
        if too_close(x, z, tree_positions, 13.0):
            continue
        idx = len(placed_by_slug[slug])
        u_age = 0.52 + 0.48 * halton(idx + 7, 5)
        hscale = 0.88 + 0.24 * halton(idx + 19, 7)
        vigor = 0.85 + 0.20 * halton(idx + 3, 3)
        stress = [0.24 + 0.14 * halton(idx + 5, 3), 0.15, 0.18, 0.20]
        add_individual(slug, x, buried_flora_y(slug, x, z), z, idx, u_age, hscale, vigor, stress, "granite-ridge-headland")
        tree_positions.append((x, z))

    # 3b. Banksia integrifolia (Coast Banksia)
    slug = "banksia-integrifolia"
    target = SPECIES_ECOLOGY_META[slug]["target_count"]
    for x, z in place_clearing_or_catchment(candidate_points(5309, 15000), 20.0, 160.0):
        if len(placed_by_slug[slug]) >= target:
            break
        surf_y = tv2.terrain_height(x, z)
        if surf_y <= 15.0 or x < 35.0:
            continue
        if not is_clear_of_infrastructure(x, z, 2.5):
            continue
        if too_close(x, z, tree_positions, 11.0):
            continue
        idx = len(placed_by_slug[slug])
        u_age = 0.50 + 0.50 * halton(idx + 11, 5)
        hscale = 0.85 + 0.25 * halton(idx + 23, 7)
        vigor = 0.86 + 0.22 * halton(idx + 37, 3)
        stress = [0.28 + 0.16 * halton(idx + 13, 3), 0.16, 0.22, 0.18]
        add_individual(slug, x, buried_flora_y(slug, x, z), z, idx, u_age, hscale, vigor, stress, "granite-ridge-headland")
        tree_positions.append((x, z))

    # 4. Grassy Box-Gum Woodland & Clearing Perimeter Backdrop (18-160 m & hillsides)
    # 4a. Eucalyptus melliodora (Yellow Box)
    slug = "eucalyptus-melliodora"
    target = SPECIES_ECOLOGY_META[slug]["target_count"]
    for x, z in place_clearing_or_catchment(candidate_points(6803, 18000), 18.0, 160.0):
        if len(placed_by_slug[slug]) >= target:
            break
        d_river = abs(x - tv2.river_x(z))
        if d_river < 20.0 or (x < -60.0 and z < -30.0):
            continue
        surf_y = tv2.terrain_height(x, z)
        if surf_y < 2.2 or surf_y > 22.0:
            continue
        if not is_clear_of_infrastructure(x, z, 3.0):
            continue
        if too_close(x, z, tree_positions, 12.0):
            continue
        idx = len(placed_by_slug[slug])
        u_age = 0.58 + 0.42 * halton(idx + 17, 5)
        hscale = 0.90 + 0.22 * halton(idx + 29, 7)
        vigor = 0.90 + 0.20 * halton(idx + 11, 3)
        stress = [0.14 + 0.08 * halton(idx + 3, 5), 0.10, 0.10, 0.12]
        add_individual(slug, x, buried_flora_y(slug, x, z), z, idx, u_age, hscale, vigor, stress, "box-gum-woodland")
        tree_positions.append((x, z))

    # 4b. Acacia pycnantha (Golden Wattle)
    slug = "acacia-pycnantha"
    target = SPECIES_ECOLOGY_META[slug]["target_count"]
    for x, z in place_clearing_or_catchment(candidate_points(8111, 18000), 17.5, 155.0):
        if len(placed_by_slug[slug]) >= target:
            break
        d_river = abs(x - tv2.river_x(z))
        if d_river < 15.0 or (x < -60.0 and z < -30.0):
            continue
        surf_y = tv2.terrain_height(x, z)
        if surf_y < 2.0 or surf_y > 24.0:
            continue
        if not is_clear_of_infrastructure(x, z, 1.8):
            continue
        if too_close(x, z, tree_positions, 6.2):
            continue
        idx = len(placed_by_slug[slug])
        u_age = 0.45 + 0.55 * halton(idx + 7, 5)
        hscale = 0.85 + 0.28 * halton(idx + 19, 7)
        vigor = 0.88 + 0.22 * halton(idx + 31, 3)
        stress = [0.15, 0.12, 0.08, 0.14]
        add_individual(slug, x, buried_flora_y(slug, x, z), z, idx, u_age, hscale, vigor, stress, "box-gum-woodland")
        tree_positions.append((x, z))

    # 4c. Bursaria spinosa (Sweet Bursaria - 90 woody understorey shrubs)
    slug = "bursaria-spinosa"
    target = SPECIES_ECOLOGY_META[slug]["target_count"]
    for x, z in place_clearing_or_catchment(candidate_points(9013, 20000), 16.5, 165.0):
        if len(placed_by_slug[slug]) >= target:
            break
        d_river = abs(x - tv2.river_x(z))
        if d_river < 11.0:
            continue
        surf_y = tv2.terrain_height(x, z)
        if surf_y < 1.5 or surf_y > 24.5:
            continue
        if not is_clear_of_infrastructure(x, z, 1.4):
            continue
        if too_close(x, z, tree_positions, 2.8) or too_close(x, z, shrub_positions, 4.2):
            continue
        idx = len(placed_by_slug[slug])
        u_age = 0.45 + 0.55 * halton(idx + 5, 5)
        hscale = 0.78 + 0.38 * halton(idx + 17, 7)
        vigor = 0.88 + 0.20 * halton(idx + 23, 3)
        stress = [0.14, 0.10, 0.08, 0.12]
        add_individual(slug, x, buried_flora_y(slug, x, z), z, idx, u_age, hscale, vigor, stress, "box-gum-woodland")
        shrub_positions.append((x, z))

    # 4d. Poa labillardierei (Common Tussock-grass - 200 clustered clumps)
    slug = "poa-labillardierei"
    target = SPECIES_ECOLOGY_META[slug]["target_count"]
    for ax, az in place_clearing_or_catchment(candidate_points(9421, 18000), 16.0, 165.0):
        if len(placed_by_slug[slug]) >= target:
            break
        d_river = abs(ax - tv2.river_x(az))
        if not (6.2 <= d_river <= 48.0):
            continue
        surf_ay = tv2.terrain_height(ax, az)
        if surf_ay < 0.45 or surf_ay > 16.0:
            continue
        if not is_clear_of_infrastructure(ax, az, 1.2):
            continue
        cluster_n = 4
        for member in range(cluster_n):
            if len(placed_by_slug[slug]) >= target:
                break
            ang = member * (math.tau / cluster_n) + halton(len(placed_by_slug[slug]) + 11, 5) * 1.1
            rad = 0.9 + 1.5 * halton(len(placed_by_slug[slug]) + 29, 3)
            x = ax + math.cos(ang) * rad
            z = az + math.sin(ang) * rad
            if abs(x - tv2.river_x(z)) < 5.5 or not is_clear_of_infrastructure(x, z, 0.8):
                continue
            if too_close(x, z, tree_positions, 1.8) or too_close(x, z, grass_positions, 1.1):
                continue
            idx = len(placed_by_slug[slug])
            u_age = 0.55 + 0.45 * halton(idx + 3, 5)
            hscale = 0.78 + 0.42 * halton(idx + 13, 7)
            vigor = 0.92 + 0.22 * halton(idx + 19, 3)
            stress = [0.08, 0.05, 0.05, 0.07]
            add_individual(slug, x, buried_flora_y(slug, x, z), z, idx, u_age, hscale, vigor, stress, "riparian-floodplain")
            grass_positions.append((x, z))

    # 4e. Themeda triandra (Kangaroo Grass - 180 meadow drifts across clearing fringes & slopes)
    slug = "themeda-triandra"
    target = SPECIES_ECOLOGY_META[slug]["target_count"]
    for ax, az in place_clearing_or_catchment(candidate_points(9733, 20000), 15.8, 175.0):
        if len(placed_by_slug[slug]) >= target:
            break
        d_river = abs(ax - tv2.river_x(az))
        if d_river < 12.0:
            continue
        surf_ay = tv2.terrain_height(ax, az)
        if surf_ay < 1.8 or surf_ay > 24.0:
            continue
        if not is_clear_of_infrastructure(ax, az, 1.0):
            continue
        cluster_n = 5
        for member in range(cluster_n):
            if len(placed_by_slug[slug]) >= target:
                break
            ang = member * (math.tau / cluster_n) + halton(len(placed_by_slug[slug]) + 7, 5) * 1.2
            rad = 0.7 + 1.9 * halton(len(placed_by_slug[slug]) + 37, 3)
            x = ax + math.cos(ang) * rad
            z = az + math.sin(ang) * rad
            if abs(x - tv2.river_x(z)) < 11.0 or not is_clear_of_infrastructure(x, z, 0.7):
                continue
            if too_close(x, z, tree_positions, 1.8) or too_close(x, z, grass_positions, 1.0):
                continue
            idx = len(placed_by_slug[slug])
            u_age = 0.55 + 0.45 * halton(idx + 11, 5)
            hscale = 0.80 + 0.44 * halton(idx + 23, 7)
            vigor = 0.90 + 0.22 * halton(idx + 31, 3)
            stress = [0.10, 0.08, 0.06, 0.08]
            add_individual(slug, x, buried_flora_y(slug, x, z), z, idx, u_age, hscale, vigor, stress, "box-gum-woodland")
            grass_positions.append((x, z))

    # 4f. Lomandra longifolia (Spiny-headed Mat-rush - 120 riparian & rocky-edge tufts)
    slug = "lomandra-longifolia"
    target = SPECIES_ECOLOGY_META[slug]["target_count"]
    for ax, az in place_clearing_or_catchment(candidate_points(9973, 20000), 15.5, 175.0):
        if len(placed_by_slug[slug]) >= target:
            break
        d_river = abs(ax - tv2.river_x(az))
        surf_ay = tv2.terrain_height(ax, az)
        on_creek_edge = 5.4 <= d_river <= 16.0 and 0.4 <= surf_ay <= 8.0
        on_rocky_slope = d_river > 18.0 and terrain_slope_rad(ax, az) >= 0.18 and 3.0 <= surf_ay <= 22.0
        if not (on_creek_edge or on_rocky_slope):
            continue
        if not is_clear_of_infrastructure(ax, az, 1.0):
            continue
        cluster_n = 3
        for member in range(cluster_n):
            if len(placed_by_slug[slug]) >= target:
                break
            ang = member * (math.tau / cluster_n) + 0.5
            rad = 0.6 + 1.2 * halton(len(placed_by_slug[slug]) + 17, 3)
            x = ax + math.cos(ang) * rad
            z = az + math.sin(ang) * rad
            if abs(x - tv2.river_x(z)) < 5.0 or not is_clear_of_infrastructure(x, z, 0.7):
                continue
            if too_close(x, z, tree_positions, 1.7) or too_close(x, z, grass_positions, 1.0):
                continue
            idx = len(placed_by_slug[slug])
            u_age = 0.60 + 0.40 * halton(idx + 5, 5)
            hscale = 0.80 + 0.36 * halton(idx + 19, 7)
            vigor = 0.90 + 0.20 * halton(idx + 41, 3)
            stress = [0.12, 0.08, 0.07, 0.09]
            add_individual(slug, x, buried_flora_y(slug, x, z), z, idx, u_age, hscale, vigor, stress, "riparian-floodplain")
            grass_positions.append((x, z))

    # 5. Fauna Placements (6 species across authentic zones + visible near clearing 30-70 m)
    # 5a. Ornithorhynchus anatinus (Platypus) along creek banks (dist_to_river in 4.2..9.0 m)
    slug = "ornithorhynchus-anatinus"
    target = SPECIES_ECOLOGY_META[slug]["target_count"]
    for k in range(1, 500):
        if len(placed_by_slug[slug]) >= target:
            break
        z = -240.0 + halton(k + 911, 2) * 480.0
        side = 1.0 if (k % 2 == 0) else -1.0
        bank_offset = 4.6 + halton(k + 911, 3) * 3.8
        x = tv2.river_x(z) + side * bank_offset
        if not is_clear_of_infrastructure(x, z, 2.0):
            continue
        y = tv2.terrain_height(x, z)
        if not (0.18 <= y <= 2.3):
            continue
        if too_close(x, z, fauna_positions, 24.0):
            continue
        idx = len(placed_by_slug[slug])
        add_individual(slug, x, y, z, idx, 0.85, 1.0, 1.05, [0.02, 0.02, 0.03, 0.04], "riparian-floodplain")
        fauna_positions.append((x, z))

    # 5b. Phascolarctos cinereus (Koala) in upper forks of wet-sclerophyll / riparian eucalypts
    slug = "phascolarctos-cinereus"
    target = SPECIES_ECOLOGY_META[slug]["target_count"]
    koala_hosts = species_trees_by_slug["eucalyptus-regnans"] + species_trees_by_slug["eucalyptus-camaldulensis"]
    for i, tree in enumerate(koala_hosts):
        if len(placed_by_slug[slug]) >= target:
            break
        if i % 4 != 0:
            continue
        tx, ty, tz = tree["translation"]
        sy = tree["scale"][1]
        fork_height = 6.2 * sy
        idx = len(placed_by_slug[slug])
        add_individual(slug, tx + 0.45, ty + fork_height, tz - 0.35, idx, 0.88, 1.0, 1.0, [0.04, 0.03, 0.03, 0.05], tree["zone"], host_tree=tree["id"])

    # 5c. Dacelo novaeguineae (Laughing Kookaburra) perched on woodland limbs
    slug = "dacelo-novaeguineae"
    target = SPECIES_ECOLOGY_META[slug]["target_count"]
    perch_hosts = species_trees_by_slug["eucalyptus-melliodora"] + species_trees_by_slug["eucalyptus-camaldulensis"]
    for i, tree in enumerate(perch_hosts):
        if len(placed_by_slug[slug]) >= target:
            break
        if i % 5 != 1:
            continue
        tx, ty, tz = tree["translation"]
        sy = tree["scale"][1]
        perch_height = 5.4 * sy
        idx = len(placed_by_slug[slug])
        add_individual(slug, tx + 1.35, ty + perch_height, tz + 0.65, idx, 0.90, 1.0, 1.0, [0.03, 0.02, 0.02, 0.03], "box-gum-woodland", host_tree=tree["id"])

    # 5d. Macropus giganteus (Eastern Grey Kangaroo) mobs (first mob 30-70 m from clearing!)
    slug = "macropus-giganteus"
    target = SPECIES_ECOLOGY_META[slug]["target_count"]
    mob_anchors = []
    # Anchor 0: near clearing meadow (30-70 m from (-22, 28))
    for x, z in candidate_points(10201, 12000):
        if not (30.0 <= _near_clearing_dist(x, z) <= 70.0):
            continue
        d_river = abs(x - tv2.river_x(z))
        y = tv2.terrain_height(x, z)
        if d_river < 16.0 or not (2.0 <= y <= 19.0):
            continue
        if not is_clear_of_infrastructure(x, z, 4.0) or too_close(x, z, tree_positions, 3.5):
            continue
        mob_anchors.append((x, z))
        break
    for x, z in candidate_points(10201, 12000):
        if len(mob_anchors) >= 5:
            break
        d_river = abs(x - tv2.river_x(z))
        y = tv2.terrain_height(x, z)
        if d_river < 24.0 or (x < -60.0 and z < -30.0) or not (3.5 <= y <= 19.0):
            continue
        if not is_clear_of_infrastructure(x, z, 4.0):
            continue
        if too_close(x, z, tree_positions, 4.0) or too_close(x, z, mob_anchors, 45.0):
            continue
        mob_anchors.append((x, z))
    for m_idx, (ax, az) in enumerate(mob_anchors):
        mob_size = 4 if m_idx < 3 else 3
        for member in range(mob_size):
            if len(placed_by_slug[slug]) >= target:
                break
            ang = member * (math.tau / mob_size) + 0.4 * m_idx
            rad = 2.6 + 1.8 * halton(m_idx * 7 + member + 1, 3)
            mx = ax + math.cos(ang) * rad
            mz = az + math.sin(ang) * rad
            if not is_clear_of_infrastructure(mx, mz, 1.5):
                continue
            my = tv2.terrain_height(mx, mz)
            idx = len(placed_by_slug[slug])
            age = 0.72 + 0.28 * halton(idx + 5, 5)
            add_individual(slug, mx, my, mz, idx, age, 1.0, 1.02, [0.02, 0.05, 0.04, 0.05], "box-gum-woodland")
            fauna_positions.append((mx, mz))

    # 5e. Wallabia bicolor (Swamp Wallaby) in woodland & gully understorey (2+ near 30-70 m clearing)
    slug = "wallabia-bicolor"
    target = SPECIES_ECOLOGY_META[slug]["target_count"]
    for x, z in place_clearing_or_catchment(candidate_points(11801, 14000), 30.0, 70.0):
        if len(placed_by_slug[slug]) >= target:
            break
        d_river = abs(x - tv2.river_x(z))
        y = tv2.terrain_height(x, z)
        if d_river < 12.0 or not (2.2 <= y <= 20.0):
            continue
        if not is_clear_of_infrastructure(x, z, 2.5):
            continue
        if too_close(x, z, tree_positions, 2.5) or too_close(x, z, fauna_positions, 16.0):
            continue
        idx = len(placed_by_slug[slug])
        add_individual(slug, x, y, z, idx, 0.82 + 0.18 * halton(idx + 3, 5), 1.0, 1.0, [0.02, 0.04, 0.04, 0.05], "box-gum-woodland")
        fauna_positions.append((x, z))

    # 5f. Vombatus ursinus (Bare-Nosed Wombat) near woodland slope burrow aprons (2+ near 30-70 m clearing)
    slug = "vombatus-ursinus"
    target = SPECIES_ECOLOGY_META[slug]["target_count"]
    for x, z in place_clearing_or_catchment(candidate_points(13407, 16000), 30.0, 70.0):
        if len(placed_by_slug[slug]) >= target:
            break
        d_river = abs(x - tv2.river_x(z))
        y = tv2.terrain_height(x, z)
        if d_river < 18.0 or not (3.5 <= y <= 21.0):
            continue
        if not is_clear_of_infrastructure(x, z, 3.0):
            continue
        if too_close(x, z, tree_positions, 3.0) or too_close(x, z, fauna_positions, 22.0):
            continue
        idx = len(placed_by_slug[slug])
        add_individual(slug, x, y, z, idx, 0.85 + 0.15 * halton(idx + 9, 5), 1.0, 1.0, [0.02, 0.05, 0.05, 0.05], "box-gum-woodland")
        fauna_positions.append((x, z))

    # Verify all 17 species are placed and none hit campsite (-22, 28) or footbridge (z=4)
    counts = {slug: len(recs) for slug, recs in placed_by_slug.items()}
    for slug, cnt in counts.items():
        assert cnt > 0, f"Species {slug} has 0 placements!"
    all_placements = []
    for slug in by_slug:
        all_placements.extend(placed_by_slug[slug])

    # Build enriched species records (referencing /intenteco/<slug>/dist/... only)
    species_records = []
    for s in catalog["species"]:
        slug = s["slug"]
        meta = SPECIES_ECOLOGY_META[slug]
        custodial = extract_custodial_country(slug)
        call_name, call_hz = extract_call_summary(slug)
        rec = {
            "slug": slug,
            "did": s["did"],
            "scientific_name": s["scientific_name"],
            "common_name": s["common_name"],
            "kingdom": s["kingdom"],
            "ecological_zone": s["ecological_zone"],
            "catchment_zone": meta["zone"],
            "locomotion_class": meta["locomotion_class"],
            "natural_sway_freq_hz": meta["sway_hz"],
            "mycorrhizal_type": meta["mycorrhizal_type"],
            "rhizosphere_signaling": meta["rhizosphere_signaling"],
            "custodial_country": custodial,
            "call_name": call_name,
            "call_fundamental_hz": call_hz,
            "placement_count": counts[slug],
            "intenteco_repo": f"https://github.com/intenteco/{slug}",
            "canonical": s["canonical"],
            "lods": s["lods"],
            "sidecars": s["sidecars"],
        }
        species_records.append(rec)

    place_doc = {
        "schema": "webizen.ewm-catchment-place/v1",
        "id": "se-australia-catchment",
        "title": "SE Australia Living Catchment (600 m x 600 m)",
        "epistemic_class": "authored-artwork",
        "span_m": 600.0,
        "bounds_xz_m": [-300.0, 300.0],
        "origin_lat_lon": terrain["assets"]["m64"]["origin_lat_lon"],
        "campsite_ground_y": tv2.CAMPSITE_LEVEL,
        "terrain": {
            "lod0": terrain["assets"]["lod0"],
            "lod1": terrain["assets"]["lod1"],
            "water": terrain["assets"]["water"],
            "m64": terrain["assets"]["m64"],
            "pbr_textures": terrain["pbr_textures"],
        },
        "species_count": len(species_records),
        "total_placements": len(all_placements),
        "placement_counts": counts,
        "species": species_records,
        "placements": all_placements,
    }

    OUT_DIR.mkdir(parents=True, exist_ok=True)
    OUT_JSON.write_text(json.dumps(place_doc, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

    # Emit W3C Turtle for Q42 compilation
    ttl_str = lambda v: json.dumps(v, ensure_ascii=False)
    ttl_lines = [
        "@prefix rel:   <https://webizen.au/relation/> .",
        "@prefix cat:   <https://webizen.au/catalog/> .",
        "@prefix place: <https://webizen.au/catalog/place/> .",
        "@prefix dwc:   <http://rs.tdwg.org/dwc/terms/> .",
        "@prefix dcat:  <http://www.w3.org/ns/dcat#> .",
        "@prefix tk:    <https://localcontexts.org/ontology/tk-bc.ttl#> .",
        "@prefix inst:  <https://webizen.au/catchment/instance/> .",
        "",
        "place:se-australia-catchment",
        "    a cat:PublicPlace ;",
        '    rel:schema "webizen.ewm-catchment-place/v1" ;',
        '    rel:id "se-australia-catchment" ;',
        '    rel:title "SE Australia Living Catchment (600 m x 600 m)" ;',
        '    rel:hasSceneVolume "/data/ewm/places/se-australia-catchment.q42" ;',
        '    rel:terrainLod0Url "/production/terrain-v2/catchment-600m-lod0.10d" ;',
        f'    rel:terrainLod0Sha256 {ttl_str(terrain["assets"]["lod0"]["sha256"])} ;',
        '    rel:terrainLod1Url "/production/terrain-v2/catchment-600m-lod1.10d" ;',
        f'    rel:terrainLod1Sha256 {ttl_str(terrain["assets"]["lod1"]["sha256"])} ;',
        '    rel:waterUrl "/production/terrain-v2/catchment-600m-water.10d" ;',
        f'    rel:waterSha256 {ttl_str(terrain["assets"]["water"]["sha256"])} ;',
        '    rel:terrainM64Url "/production/terrain-v2/catchment-600m.m64" ;',
        f'    rel:terrainM64Sha256 {ttl_str(terrain["assets"]["m64"]["sha256"])} ;',
        f"    rel:speciesCount {len(species_records)} ;",
        f"    rel:totalPlacements {len(all_placements)} ;",
        "    rel:hasSpecies " + " , ".join(f"<{r['did']}>" for r in species_records) + " ;",
        "    rel:hasPlacement " + " , ".join(f"inst:{p['id']}" for p in all_placements) + " .",
        "",
    ]

    for r in species_records:
        ttl_lines.extend([
            f"<{r['did']}>",
            "    a dwc:Taxon , dcat:Dataset ;",
            f"    rel:slug {ttl_str(r['slug'])} ;",
            f"    dwc:scientificName {ttl_str(r['scientific_name'])} ;",
            f"    dwc:vernacularName {ttl_str(r['common_name'])} ;",
            f"    rel:kingdom {ttl_str(r['kingdom'])} ;",
            f"    rel:ecologicalZone {ttl_str(r['ecological_zone'])} ;",
            f"    rel:catchmentZone {ttl_str(r['catchment_zone'])} ;",
            f"    rel:locomotionClass {ttl_str(r['locomotion_class'])} ;",
            f"    rel:naturalSwayFreqHz {r['natural_sway_freq_hz']:.3f} ;",
            f"    rel:mycorrhizalType {ttl_str(r['mycorrhizal_type'])} ;",
            f"    rel:rhizosphereSignaling {ttl_str(r['rhizosphere_signaling'])} ;",
            f"    tk:custodial_country {ttl_str(r['custodial_country'])} ;",
            f"    rel:custodialCountry {ttl_str(r['custodial_country'])} ;",
            f"    rel:intentecoRepo {ttl_str(r['intenteco_repo'])} ;",
            f"    rel:canonicalUrl {ttl_str(r['canonical']['local_url'])} ;",
            f"    rel:canonicalRemoteUrl {ttl_str(r['canonical']['remote_url'])} ;",
            f"    rel:canonicalSha256 {ttl_str(r['canonical']['sha256'])} ;",
            f"    rel:sidecarTaxonQ42 {ttl_str(r['sidecars']['taxon_q42']['local_url'])} ;",
            f"    rel:sidecarFieldEmf {ttl_str(r['sidecars']['field_emf']['local_url'])} ;",
            f"    rel:placementCount {r['placement_count']} ;",
        ])
        if "call_qac" in r["sidecars"]:
            ttl_lines.extend([
                f"    rel:sidecarCallQac {ttl_str(r['sidecars']['call_qac']['local_url'])} ;",
                f"    rel:sidecarCallQacRemote {ttl_str(r['sidecars']['call_qac']['remote_url'])} ;",
                f"    rel:sidecarCallQacSha256 {ttl_str(r['sidecars']['call_qac']['sha256'])} ;",
                f"    rel:callName {ttl_str(r['call_name'])} ;",
                f"    rel:callFundamentalHz {r['call_fundamental_hz']:.2f} ;",
            ])
        if "rhizosphere" in r["lods"]:
            ttl_lines.extend([
                f"    rel:rhizosphereUrl {ttl_str(r['lods']['rhizosphere']['local_url'])} ;",
                f"    rel:abovegroundLod1Url {ttl_str(r['lods']['aboveground_lod1']['local_url'])} ;",
            ])
        ttl_lines.append(f"    rel:lod0Url {ttl_str(r['lods']['lod0']['local_url'])} .")
        ttl_lines.append("")

    for idx, p in enumerate(all_placements):
        s64 = p["instance_state_64"]
        tx, ty, tz = p["translation"]
        rx, ry, rz, rw = p["rotation"]
        sx, sy, sz = p["scale"]
        ttl_lines.extend([
            f"inst:{p['id']}",
            "    a <https://webizen.au/scene/Placement> ;",
            f"    rel:index {idx} ;",
            f"    rel:id {ttl_str(p['id'])} ;",
            f"    rel:speciesSlug {ttl_str(p['species_slug'])} ;",
            f"    rel:taxonIri <{p['taxon_iri']}> ;",
            f"    rel:zoneId {ttl_str(p['zone'])} ;",
            f"    rel:translationX {tx:.4f} ;",
            f"    rel:translationY {ty:.4f} ;",
            f"    rel:translationZ {tz:.4f} ;",
            f"    rel:rotationX {rx:.8f} ;",
            f"    rel:rotationY {ry:.8f} ;",
            f"    rel:rotationZ {rz:.8f} ;",
            f"    rel:rotationW {rw:.8f} ;",
            f"    rel:scaleX {sx:.5f} ;",
            f"    rel:scaleY {sy:.5f} ;",
            f"    rel:scaleZ {sz:.5f} ;",
            f"    rel:seed {s64['seed']} ;",
            f"    rel:ontogeneticAgeNorm {s64['ontogenetic_age_norm']:.4f} ;",
            f"    rel:heightScale {s64['height_scale']:.4f} ;",
            f"    rel:siteVigor {s64['site_vigor']:.4f} ;",
            f"    rel:stressWind {s64['environmental_stress'][0]:.4f} .",
            "",
        ])

    OUT_TTL.write_text("\n".join(ttl_lines), encoding="utf-8")
    summary = {
        "json": str(OUT_JSON),
        "ttl": str(OUT_TTL),
        "ttl_bytes": OUT_TTL.stat().st_size,
        "species_count": len(species_records),
        "total_placements": len(all_placements),
        "placement_counts": counts,
    }
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
