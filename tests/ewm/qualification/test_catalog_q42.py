"""EWM catalog is Q42 CBOR-LD. GIS place IRIs join the woodland scene assets."""
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
HELPER = ROOT / "adapters/qualiadb/q42_web.exe"
CATALOG_Q42 = ROOT / "data/ewm/catalog.q42"
CATALOG_TTL = ROOT / "data/ewm/catalog.ttl"
WORLD_Q42 = ROOT / "publication/generated/world.q42"
SCENE_Q42 = ROOT / "production/lush-woodland/scene.q42"


def test_catalog_q42_is_cbor_ld_volume():
    assert CATALOG_Q42.is_file()
    assert CATALOG_TTL.is_file()
    assert WORLD_Q42.is_file()
    assert CATALOG_Q42.read_bytes()[:3] == b"Q42"
    assert WORLD_Q42.read_bytes()[:3] == b"Q42"
    loaded = json.loads(
        subprocess.check_output(
            [str(HELPER), "inspect", str(CATALOG_Q42)],
            encoding="utf-8",
            timeout=90,
        )
    )
    assert loaded["reader"] == "qualia_core_db::q42_volume::Q42Volume"
    assert loaded["version"] == 4
    assert loaded["verified_blocks"] is True
    assert loaded["total_quins"] > 2000
    kinds = {row[2] for row in loaded["rows"] if row[1].endswith("/kind")}
    assert "https://webizen.au/catalog/Catalog" in kinds
    assert "https://webizen.au/catalog/PublicPlace" in kinds
    paints = [
        row[2]
        for row in loaded["rows"]
        if row[1].endswith("/firstPaint")
    ]
    assert len(paints) == 24
    assert all(p.startswith("https://webizen.au/scene/asset/") for p in paints)
    gis = [
        row
        for row in loaded["rows"]
        if row[1].endswith("/gisTerrain")
    ]
    assert gis
    assert gis[0][2] == "https://webizen.au/scene/asset/canberra-reference-terrain"
    streaming = [
        row[2]
        for row in loaded["rows"]
        if row[1].endswith("/streamingDefault")
    ]
    assert streaming == ["false"]
    schemas = [row[2] for row in loaded["rows"] if row[1].endswith("/schema")]
    assert schemas == ["webizen.ewm-catalog/v1"]
    assert not any(str(row[2]).startswith("[") for row in loaded["rows"])


def test_catalog_gis_joins_scene_assets():
    scene = json.loads(
        subprocess.check_output(
            [str(HELPER), "inspect", str(SCENE_Q42)],
            encoding="utf-8",
            timeout=90,
        )
    )
    catalog = json.loads(
        subprocess.check_output(
            [str(HELPER), "inspect", str(CATALOG_Q42)],
            encoding="utf-8",
            timeout=90,
        )
    )
    scene_assets = {
        row[0]
        for row in scene["rows"]
        if row[1].endswith("/kind") and row[2] == "https://webizen.au/scene/asset"
    }
    paints = {
        row[2]
        for row in catalog["rows"]
        if row[1].endswith("/firstPaint")
    }
    assert paints <= scene_assets
    terrain = next(
        row[2] for row in catalog["rows"] if row[1].endswith("/gisTerrain")
    )
    assert terrain in scene_assets


def test_host_admits_q42_volumes_not_json_catalogs():
    boot = (ROOT / "native/world-host/src/runtime/boot.rs").read_text(encoding="utf-8")
    stream = (ROOT / "native/world-host/src/runtime/stream/mod.rs").read_text(
        encoding="utf-8"
    )
    download = (ROOT / "native/world-host/src/runtime/ui/download.rs").read_text(
        encoding="utf-8"
    )
    assert "/publication/world.q42" in boot
    assert "/publication/current.json" not in boot
    assert "/data/ewm/catalog.q42" in boot
    assert "GraphDatabase.sparql" in boot
    assert "packages/current.json" not in stream
    assert "first-paint.json" not in stream
    assert "first-paint.json" not in download
    assert "/data/ewm/catalog.q42" in download
    knowledge = (ROOT / "native/world-host/src/runtime/knowledge.rs").read_text(
        encoding="utf-8"
    )
    assert "admit_q42_slice" in knowledge
    assert "GraphDatabase.sparql" in knowledge
