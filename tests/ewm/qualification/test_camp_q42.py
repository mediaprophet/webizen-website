"""Camp logic macros live in Q42; Vibe authors the spatial rig."""
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
HELPER = ROOT / "adapters/qualiadb/q42_web.exe"
CAMP = ROOT / "data/ewm/camp.q42"
N3 = ROOT / "data/ewm/camp-sequence.n3"
TTL = ROOT / "data/ewm/camp-sequence.ttl"
VIBE = ROOT / "site/world/living-world.vibe"
HOST = ROOT / "native/world-host/src/runtime/macros.rs"
BOOT = ROOT / "native/world-host/src/runtime/boot.rs"
DECODER = ROOT / "native/world-host/src/runtime/scene_q42.rs"
RUNTIME = ROOT / "native/world-host/src/runtime.rs"
SCENE_TTL = ROOT / "production/lush-woodland/scene.ttl"


def test_camp_q42_volume():
    assert CAMP.is_file()
    assert N3.is_file()
    assert TTL.is_file()
    n3 = N3.read_text(encoding="utf-8")
    assert n3.startswith("@prefix")
    assert "rel:" in n3
    assert "unhitch" in n3
    assert "travelsHitched" in n3 or "travelsHitched" in TTL.read_text(encoding="utf-8")
    loaded = json.loads(
        subprocess.check_output(
            [str(HELPER), "inspect", str(CAMP)],
            encoding="utf-8",
            timeout=90,
        )
    )
    assert loaded["reader"] == "qualia_core_db::q42_volume::Q42Volume"
    assert loaded["version"] == 4
    assert loaded["verified_blocks"] is True
    assert loaded["total_quins"] > 40
    kinds = {row[2] for row in loaded["rows"] if row[1].endswith("/kind")}
    assert "https://webizen.au/camp/Sequence" in kinds
    assert "https://webizen.au/camp/Stage" in kinds
    assert "https://webizen.au/camp/Norm" in kinds
    schemas = [row[2] for row in loaded["rows"] if row[1].endswith("/schema")]
    assert any("webizen.logic-sequence/v1" in str(s) for s in schemas)
    rigs = [row[2] for row in loaded["rows"] if row[1].endswith("/rigFunction")]
    assert any("camp_rig" in str(r) for r in rigs)
    gates = [row[2] for row in loaded["rows"] if row[1].endswith("/gateGroup")]
    assert any("car" in str(g) for g in gates)
    ops = [row[2] for row in loaded["rows"] if row[1].endswith("/opcode")]
    assert '"0x11"' in ops or "0x11" in ops or any("0x11" in str(o) for o in ops)
    temporal = [row[2] for row in loaded["rows"] if row[1].endswith("/temporalOp")]
    assert any("Meets" in str(t) for t in temporal)
    assert CAMP.read_bytes()[:3] == b"Q42"


def test_host_plays_macros_not_per_scene_rust():
    macros = HOST.read_text(encoding="utf-8")
    vibe = VIBE.read_text(encoding="utf-8")
    boot = BOOT.read_text(encoding="utf-8")
    decoder = DECODER.read_text(encoding="utf-8")
    runtime = RUNTIME.read_text(encoding="utf-8")
    scene_ttl = SCENE_TTL.read_text(encoding="utf-8")
    assert "evaluate_predicate_triggers_zero_heap" in macros
    assert "evaluate_deontic_contract" in macros
    assert "pack_preset_command_quin" in macros
    assert "compile_norm_quin" in macros
    assert "evaluate_preset" in macros
    assert "webizen.logic-sequence/v1" in macros
    assert 'if id == "car"' not in macros
    assert 'evaluate(host, source, "camp_rig"' not in macros
    assert "mod camp;" not in runtime
    assert "camp.rs" not in runtime
    assert "logic_url" in boot
    assert "hasLogicVolume" in decoder
    assert "hasLogicVolume" in scene_ttl
    assert "/data/ewm/camp.q42" in scene_ttl
    assert "camp_rig" in vibe
    assert "spring_settle" in vibe
    assert "travels_hitched" in macros
    assert "travels_unhitched" in macros
    assert "unhitch_defeater" in macros
    assert "sync_parked" in macros
    assert "travels_unhitched: true" in vibe
    assert 'id: "unhitch"' in vibe


def test_camp_unhitch_facts():
    loaded = json.loads(
        subprocess.check_output(
            [str(HELPER), "inspect", str(CAMP)],
            encoding="utf-8",
            timeout=90,
        )
    )
    rows = loaded["rows"]
    ids = {row[2] for row in rows if row[1].endswith("/id")}
    assert any("unhitch" in str(i) for i in ids)
    assert any("permit-travel-when-unhitched" in str(i) for i in ids)
    preds = [row[1] for row in rows]
    assert any(p.endswith("/travelsHitched") for p in preds)
    assert any(p.endswith("/travelsUnhitched") for p in preds)
    assert any(p.endswith("/requiresUnhitch") for p in preds)
    car_unhitched = False
    chassis_unhitched = True
    for s, p, o in rows:
        if not str(p).endswith("/travelsUnhitched"):
            continue
        if str(s).endswith("/group/car") and str(o) in ("true", '"true"', True):
            car_unhitched = True
        if str(s).endswith("/group/chassis") and str(o) in ("true", '"true"', True):
            chassis_unhitched = True
        if str(s).endswith("/group/chassis") and str(o) in ("false", '"false"', False):
            chassis_unhitched = False
    assert car_unhitched
    assert chassis_unhitched is False


def test_pack_up_sides_fold_stow_then_lid():
    n3 = N3.read_text(encoding="utf-8")
    vibe = VIBE.read_text(encoding="utf-8")
    assert "group:sides" in n3
    assert "group:panels-three" in n3
    assert "group:panels-two" in n3
    assert "stage:slide-sides" in n3
    assert "stage:tuck-sides" in n3
    assert "stage:fold-panels-three" in n3
    assert "stage:fold-panels-two" in n3
    assert "stage:stow-panels-three" in n3
    assert "stage:stow-starlink" in n3
    assert 'rel:channel "fold"' in n3
    assert "vintage-windup-camper--canvas-roof" in n3
    assert "solar-array-three" in n3
    assert "solar-array-two" in n3
    sides_line = [ln for ln in n3.splitlines() if "group:sides rel:prefixes" in ln][0]
    assert "canvas-roof" in sides_line
    roof_line = [ln for ln in n3.splitlines() if "group:roof rel:prefixes" in ln][0]
    assert "camper--roof" in roof_line
    assert "canvas-roof" not in roof_line
    assert "camper--canvas," not in roof_line
    assert "camper--screen" not in roof_line
    star_dy = [ln for ln in n3.splitlines() if "group:starlink rel:deltaY" in ln][0]
    assert "-2.4" in star_dy
    assert 'id: "sides"' in vibe
    assert 'id: "panels-three"' in vibe
    assert "solar-array-three" in vibe
    macros = HOST.read_text(encoding="utf-8")
    assert "overlay_scale" in macros
    assert "fold_goals" in macros
