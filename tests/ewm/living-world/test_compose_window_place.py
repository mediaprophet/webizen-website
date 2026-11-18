"""LW-09: compose_window(river) omits camper unless river Base keeps it."""
import json
import math
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
CATALOG = ROOT / "data/ewm/first-paint.json"
PLACE_INDEX = ROOT / "data/ewm/place-index.json"
SELECTION = ROOT / "native/world-host/src/runtime/stream/selection.rs"
UNLOAD = ROOT / "native/world-host/src/runtime/stream/unload.rs"
CATALOG_TTL = ROOT / "data/ewm/catalog.ttl"
CAMPER = "vintage-windup-camper--chassis"
MAX_WINDOW = 64


def tile_xz(x, z):
    return (math.floor(x / 100.0), math.floor(z / 100.0))


def aabb_overlaps(mn, mx, x, z, radius=80.0):
    nx = min(max(x, mn[0]), mx[0])
    nz = min(max(z, mn[2]), mx[2])
    dx = x - nx
    dz = z - nz
    return dx * dx + dz * dz <= radius * radius


def live_place_base(index, first_paint, place_id):
    places = {p["id"]: p for p in index["places"]}
    place = places.get(place_id)
    if not place or not place.get("base"):
        return list(first_paint)
    if set(place["base"]) == set(first_paint):
        return list(first_paint)
    return list(place["base"])


def compose_window(catalog, index, place_id, x=0.0, z=0.0):
    first_paint = catalog["first_paint"]
    base = live_place_base(index, first_paint, place_id)
    woodland = set(base) == set(first_paint)
    ids = []
    seen = set()
    for scene_id in base:
        if len(ids) >= MAX_WINDOW:
            break
        if scene_id not in seen:
            seen.add(scene_id)
            ids.append(scene_id)
    place = next((p for p in index["places"] if p["id"] == place_id), None)
    place_enh = set(place.get("enhancement") or []) if place else set()
    for row in catalog["entries"]:
        if len(ids) >= MAX_WINDOW:
            break
        if row["role"] != "Enhancement":
            continue
        scene_id = row["scene_id"]
        if scene_id in seen:
            continue
        aabb = row["aabb"]
        tiles = [tuple(t) for t in row["tiles"]]
        if not (
            aabb_overlaps(aabb["min"], aabb["max"], x, z) or tile_xz(x, z) in tiles
        ):
            continue
        if not woodland and row.get("place_id") != place_id and scene_id not in place_enh:
            continue
        seen.add(scene_id)
        ids.append(scene_id)
    return ids


def omitted_ids(resident, nxt):
    keep = set(nxt)
    return [i for i in resident if i not in keep]


def test_place_index_river_base_omits_camper():
    catalog = json.loads(CATALOG.read_text(encoding="utf-8"))
    index = json.loads(PLACE_INDEX.read_text(encoding="utf-8"))
    ids = {p["id"] for p in index["places"]}
    assert {"field-studio", "river", "canopy", "commons"} <= ids
    studio = next(p for p in index["places"] if p["id"] == "field-studio")
    river = next(p for p in index["places"] if p["id"] == "river")
    assert studio["base"] == catalog["first_paint"]
    assert len(studio["base"]) == 24
    assert CAMPER in studio["base"]
    assert CAMPER not in river["base"]
    assert not any(i.startswith("vintage-windup-camper--") for i in river["base"])
    assert "tidal-vessel--bronze" in river["base"]
    blob = json.dumps(index)
    assert "Timeline" not in blob
    assert "INFERRED_WORK" not in blob
    assert index["streaming"]["default"] is False
    assert index["inner_ranges"] is False


def test_compose_window_river_omits_camper_that_field_studio_keeps():
    catalog = json.loads(CATALOG.read_text(encoding="utf-8"))
    index = json.loads(PLACE_INDEX.read_text(encoding="utf-8"))
    studio = compose_window(catalog, index, "field-studio")
    river = compose_window(catalog, index, "river")
    assert CAMPER in studio
    assert any(i.startswith("vintage-windup-camper--") for i in studio)
    river_keeps = CAMPER in live_place_base(index, catalog["first_paint"], "river")
    if not river_keeps:
        assert CAMPER not in river
        assert not any(i.startswith("vintage-windup-camper--") for i in river)
    assert "tidal-vessel--bronze" in river
    assert len(studio) <= MAX_WINDOW
    assert len(river) <= MAX_WINDOW
    omitted = omitted_ids(studio, river)
    assert CAMPER in omitted
    assert "habitat-terrain" not in omitted


def test_selection_source_uses_live_place_base_not_immortal_first_paint():
    src = SELECTION.read_text(encoding="utf-8")
    unload = UNLOAD.read_text(encoding="utf-8")
    ttl = CATALOG_TTL.read_text(encoding="utf-8")
    assert "pub fn compose_window" in src
    assert "live_place_base" in src
    assert "FetchRangeService" in src
    assert "fn bytes_range" not in src
    assert "First-paint stays in the window" not in src
    assert "first-paint always stays in the window" not in src
    assert "must keep streaming.default false" in src
    assert "must keep inner_ranges false" in src
    assert 'if id == "car"' not in src
    assert "omit-and-rebuild" in unload
    assert "unload_scene_parts" in unload
    assert "pub fn omitted_ids" in unload
    assert "/data/ewm/places/" in unload
    assert "<https://webizen.au/catalog/place/river>" in ttl
    assert (
        "<https://webizen.au/catalog/place/river> "
        "<https://webizen.au/relation/hasBaseAsset> "
        "<https://webizen.au/scene/asset/vintage-windup-camper--chassis>"
    ) not in ttl
    assert (
        "<https://webizen.au/catalog/place/field-studio> "
        "<https://webizen.au/relation/hasBaseAsset> "
        "<https://webizen.au/scene/asset/vintage-windup-camper--chassis>"
    ) in ttl
