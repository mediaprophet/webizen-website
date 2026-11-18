"""Woodland scene graph is Q42 CBOR-LD, not a runtime JSON catalog."""
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
HELPER = ROOT / "adapters/qualiadb/q42_web.exe"
SCENE_Q42 = ROOT / "production/lush-woodland/scene.q42"
SCENE_TTL = ROOT / "production/lush-woodland/scene.ttl"
WATER = ROOT / "production/lush-woodland/water.bin"
CATALOG = ROOT / "data/ewm/first-paint.json"


def test_scene_q42_volume_roundtrips():
    assert SCENE_Q42.is_file()
    assert SCENE_TTL.is_file()
    assert WATER.is_file()
    assert WATER.read_bytes()[:4] == b"WTR1"
    catalog = json.loads(CATALOG.read_text(encoding="utf-8"))
    assert catalog["streaming"]["default"] is False
    loaded = json.loads(
        subprocess.check_output(
            [str(HELPER), "inspect", str(SCENE_Q42)],
            encoding="utf-8",
            timeout=90,
        )
    )
    assert loaded["reader"] == "qualia_core_db::q42_volume::Q42Volume"
    assert loaded["version"] == 4
    assert loaded["verified_blocks"] is True
    assert loaded["total_quins"] > 6000
    kinds = {row[2] for row in loaded["rows"] if row[1].endswith("/kind")}
    assert "https://webizen.au/scene/world" in kinds
    assert "https://webizen.au/scene/asset" in kinds
    def lex(term: str):
        if term[:1] in '"[{' or term in ("true", "false", "null"):
            return json.loads(term)
        try:
            return json.loads(term)
        except json.JSONDecodeError:
            return term

    ids = {
        lex(row[2])
        for row in loaded["rows"]
        if row[1].endswith("/id") and row[0].startswith("https://webizen.au/scene/asset/")
    }
    assert "vintage-windup-camper--chassis" in ids
    assert "canberra-reference-terrain" in ids
    assert len(ids) == 85
    assert "field-studio-operator-opt" in ids
    assert "field-studio-operator-desk" in ids
    assert "field-studio-operator-laptop" in ids
    assert "field-studio-operator-laptop-lid" in ids
    assert "field-studio-operator-shirt" in ids
    assert any(
        row[1].endswith("/hasLogicVolume") and "/data/ewm/camp.q42" in str(row[2])
        for row in loaded["rows"]
    )
    assert SCENE_Q42.read_bytes()[:3] == b"Q42"
    colours = [row[2] for row in loaded["rows"] if row[1].endswith("/colour")]
    assert colours == []
    assert any(row[1].endswith("/colourR") for row in loaded["rows"])
    assert any(row[1].endswith("/translationX") for row in loaded["rows"])
    assert not any(str(row[2]).startswith("[") for row in loaded["rows"])


def test_host_decodes_q42_not_woodland_json():
    boot = (ROOT / "native/world-host/src/runtime/boot.rs").read_text(encoding="utf-8")
    decoder = (ROOT / "native/world-host/src/runtime/scene_q42.rs").read_text(encoding="utf-8")
    runtime = (ROOT / "native/world-host/src/runtime.rs").read_text(encoding="utf-8")
    assert "scene_q42::from_host" in boot
    assert "never fetches scene.json" in decoder
    assert "webizen.native-world/v1" in decoder
    assert "mod scene_q42;" in runtime
    assert "admit_q42_slice" in boot
    assert "serde_json::from_value" not in decoder
    assert "colourR" in decoder
