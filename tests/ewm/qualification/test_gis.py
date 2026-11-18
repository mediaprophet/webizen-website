"""GIS named-place AABB / tile membership without personal coordinates."""
import json
import math
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def tile_xz(x, z):
    return (math.floor(x / 100.0), math.floor(z / 100.0))


def aabb_overlaps(mn, mx, x, z, radius=80.0):
    nx = min(max(x, mn[0]), mx[0])
    nz = min(max(z, mn[2]), mx[2])
    dx = x - nx
    dz = z - nz
    return dx * dx + dz * dz <= radius * radius


def test_place_is_public_canberra():
    place = json.loads(
        (ROOT / "data/ewm/gis/places/canberra-public-reference.json").read_text(
            encoding="utf-8"
        )
    )
    assert place["id"] == "canberra-public-reference"
    assert place["bbox_wsen"] == [149.06, -35.30, 149.08, -35.28]
    assert place["not_personal_campsite"] is True
    assert place["realm_class"] == "physical"
    assert "Timeline" not in json.dumps(place)
    assert "INFERRED_WORK" not in json.dumps(place)


def test_camera_origin_overlaps_habitat_and_dem():
    catalog = json.loads((ROOT / "data/ewm/first-paint.json").read_text(encoding="utf-8"))
    hits = []
    for row in catalog["entries"]:
        aabb = row["aabb"]
        if aabb_overlaps(aabb["min"], aabb["max"], 0.0, 0.0) or tile_xz(0.0, 0.0) in [
            tuple(t) for t in row["tiles"]
        ]:
            hits.append(row["scene_id"])
    assert "habitat-terrain" in hits
    assert "canberra-reference-terrain" in hits
    first = [row["scene_id"] for row in catalog["entries"] if row["role"] == "Base"]
    window = []
    seen = set()
    for scene_id in catalog["first_paint"]:
        if scene_id not in seen:
            seen.add(scene_id)
            window.append(scene_id)
    for scene_id in hits:
        if len(window) >= 64:
            break
        if scene_id in seen:
            continue
        seen.add(scene_id)
        window.append(scene_id)
    assert set(catalog["first_paint"]).issubset(set(first))
    assert len(window) <= 64


def test_frames_noosphere_layer():
    frames = json.loads(
        (ROOT / "data/ewm/frames/canberra-public-reference.json").read_text(
            encoding="utf-8"
        )
    )
    assert frames["realm_class"] == "physical"
    layer = frames["colocated"][0]
    assert layer["realm_class"] == "noosphere"
    assert layer["grounding"] == "EmpiricallyAnchored"
    assert layer["label"] == "infosphere"
    assert "cyber" not in json.dumps(frames)


def test_camera_move_is_implemented_not_later():
    selection = (ROOT / "native/world-host/src/runtime/stream/selection.rs").read_text(
        encoding="utf-8"
    )
    assert "prepare_window" in selection
    assert "admit_prepared" in selection
    assert "SelectionIntent" in selection
    evidence = json.loads(
        (ROOT / "evidence/ewm/qualification.json").read_text(encoding="utf-8")
    )
    join = evidence["gis"]["join"]
    assert evidence["gis"]["place_id"] == "canberra-public-reference"
    assert evidence["gis"]["bbox_wsen"] == [149.06, -35.3, 149.08, -35.28]
    assert evidence["gis"]["public_boot_calls_live_adapters"] is False
    assert "prepare_window" in join
    assert "admit_prepared" in join
    assert "recompile is later" not in join
    assert "Timeline" not in join
    assert "placeIds" not in json.dumps(evidence["gis"])
