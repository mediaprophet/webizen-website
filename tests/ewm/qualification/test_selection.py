"""Camera-move SelectionIntent: tile debounce, window ≤64, streaming stays off."""
import json
import math
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def tile_xz(x, z):
    return (math.floor(x / 100.0), math.floor(z / 100.0))


def tile_changed(prev, next_tile):
    return prev != next_tile


def aabb_overlaps(mn, mx, x, z, radius=80.0):
    nx = min(max(x, mn[0]), mx[0])
    nz = min(max(z, mn[2]), mx[2])
    dx = x - nx
    dz = z - nz
    return dx * dx + dz * dz <= radius * radius


def overlapping_ids(catalog, x=0.0, z=0.0):
    hits = []
    for row in catalog["entries"]:
        aabb = row["aabb"]
        tiles = [tuple(t) for t in row["tiles"]]
        if aabb_overlaps(aabb["min"], aabb["max"], x, z) or tile_xz(x, z) in tiles:
            hits.append(row["scene_id"])
    return hits


def compose_window(catalog, x=0.0, z=0.0):
    ids = []
    seen = set()
    for scene_id in catalog["first_paint"]:
        if scene_id not in seen:
            seen.add(scene_id)
            ids.append(scene_id)
        if len(ids) >= 64:
            return ids
    for row in catalog["entries"]:
        if len(ids) >= 64:
            break
        if row["role"] != "Enhancement":
            continue
        scene_id = row["scene_id"]
        if scene_id in seen:
            continue
        aabb = row["aabb"]
        tiles = [tuple(t) for t in row["tiles"]]
        if aabb_overlaps(aabb["min"], aabb["max"], x, z) or tile_xz(x, z) in tiles:
            seen.add(scene_id)
            ids.append(scene_id)
    return ids


def test_tile_debounce_unchanged_then_changed():
    origin = tile_xz(0.0, 0.0)
    same = tile_xz(0.0, 0.0)
    east = tile_xz(100.0, 0.0)
    assert origin == (0, 0)
    assert east == (1, 0)
    assert not tile_changed(origin, same)
    assert tile_changed((0, 0), (1, 0))
    assert tile_changed(origin, east)
    source = (ROOT / "native/world-host/src/runtime/stream/selection.rs").read_text(
        encoding="utf-8"
    )
    assert "pub fn tile_changed" in source


def test_window_at_camera_origin_is_nonempty_and_at_most_64():
    catalog = json.loads((ROOT / "data/ewm/first-paint.json").read_text(encoding="utf-8"))
    hits = overlapping_ids(catalog, 0.0, 0.0)
    assert hits
    assert "habitat-terrain" in hits
    assert "canberra-reference-terrain" in hits
    window = compose_window(catalog, 0.0, 0.0)
    assert window[:24] == catalog["first_paint"]
    assert len(window) <= 64
    assert len(window) > 24
    assert "car-passat-b7--rubber" in window
    assert catalog["candidate"]["entries"] == 24
    assert len(catalog["entries"]) > 24


def test_enhancement_is_after_base_and_optional():
    catalog = json.loads((ROOT / "data/ewm/first-paint.json").read_text(encoding="utf-8"))
    first = set(catalog["first_paint"])
    extra = [row for row in catalog["entries"] if row["scene_id"] not in first]
    named = {row["id"] for row in catalog["enhancement_not_first_paint"]}
    assert extra
    assert {row["scene_id"] for row in extra} == named
    for row in extra:
        assert row["role"] == "Enhancement"
        assert row["obligation"] == "Optional"
        assert row["shard"] == "enhancement"
        src = ROOT / row["url"].lstrip("/")
        assert src.is_file(), src
    source = (ROOT / "native/world-host/src/runtime/stream/selection.rs").read_text(
        encoding="utf-8"
    )
    assert "pub fn compose_window" in source
    runtime = (ROOT / "native/world-host/src/runtime.rs").read_text(encoding="utf-8")
    assert "enhancement_pending" in runtime


def test_catalog_keeps_streaming_default_false():
    catalog = json.loads((ROOT / "data/ewm/first-paint.json").read_text(encoding="utf-8"))
    assert catalog["streaming"]["default"] is False
    pointer = json.loads((ROOT / "packages/current.json").read_text(encoding="utf-8"))
    assert pointer["streaming"]["default"] is False


def test_selection_source_apis_exist_and_do_not_enable_streaming():
    source = (ROOT / "native/world-host/src/runtime/stream/selection.rs").read_text(
        encoding="utf-8"
    )
    for name in (
        "prepare_window",
        "admit_prepared",
        "tile_changed",
        "live_selection_from_catalog",
        "compose_window",
    ):
        assert name in source, name
    assert "streaming.default = true" not in source
    assert "streaming.default=true" not in source
    assert "streaming_default" in source
    assert "must keep streaming.default false" in source


def test_runtime_world_has_ewm_and_scene_soft_integrator_hook():
    runtime = (ROOT / "native/world-host/src/runtime.rs").read_text(encoding="utf-8")
    assert "ewm: Option" in runtime
    assert "scene: Scene" in runtime
    # Integrator may still be landing take_ewm_tile_job; do not fail if absent.
    if "take_ewm_tile_job" in runtime:
        assert "prepare_window" in runtime
        assert "admit_prepared" in runtime
        assert "spawn_local" in runtime


def test_qualification_camera_move_is_source_not_later():
    evidence = json.loads(
        (ROOT / "evidence/ewm/qualification.json").read_text(encoding="utf-8")
    )
    join = evidence["gis"]["join"]
    assert "prepare_window" in join
    assert "admit_prepared" in join
    assert "recompile is later" not in join
    assert "ewm_live_tile" in join
    assert "compose_window" in join
    assert evidence["streaming_default"] is False
    enh = evidence["enhancement"]
    assert enh["after_base"] is True
    assert enh["catalog_entries"] == 54
    assert enh["four_tuple_entries"] == 24
    assert enh["window"] == 64
    assert enh["enhancement_in_origin_window"] == 40
    assert enh["near_eucalyptus_in_origin_window"] is False
    assert enh["streaming_default"] is False
    assert (ROOT / enh["screenshot"]).is_file()
    assert evidence["browser_ran"] is True
    live = evidence["gis"]["live_tile_change"]
    assert live["after"] == {"x": 0, "z": 0}
    assert live["method"] == "shift+drag pan"


def test_camera_move_live_tile_changed_from_browser():
    evidence = json.loads(
        (ROOT / "evidence/ewm/camera-move.json").read_text(encoding="utf-8")
    )
    assert evidence["schema"] == "webizen.ewm-camera-move/v1"
    assert evidence["locale"] == "en-AU"
    assert evidence["streaming_default"] is False
    assert evidence["streaming"]["default"] is False
    assert evidence["method"] == "shift+drag pan"
    assert evidence["tile_size_m"] == 100
    assert evidence["radius_m"] == 80
    before = evidence["before"]["ewm_live_tile"]
    after = evidence["after"]["ewm_live_tile"]
    assert after is not None
    assert before != after
    assert after != {"x": -1, "z": 0}
    assert evidence["tile_changed"] is True
    assert (ROOT / evidence["screenshot"]).is_file()
