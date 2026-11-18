"""LW-09: P1 unhitch regression. Dual travel flags, no camp.rs, no id == car."""
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
N3 = ROOT / "data/ewm/camp-sequence.n3"
TTL = ROOT / "data/ewm/camp-sequence.ttl"
VIBE = ROOT / "site/world/living-world.vibe"
MACROS = ROOT / "native/world-host/src/runtime/macros.rs"
RUNTIME = ROOT / "native/world-host/src/runtime.rs"
LOCO = ROOT / "native/world-host/src/runtime/locomotion.rs"
HOST_SRC = ROOT / "native/world-host/src"
CAR = 'if id == "car"'


def test_unhitch_facts_and_dual_travel_flags():
    n3 = N3.read_text(encoding="utf-8")
    ttl = TTL.read_text(encoding="utf-8")
    vibe = VIBE.read_text(encoding="utf-8")
    macros = MACROS.read_text(encoding="utf-8")
    assert "unhitch" in n3
    assert 'rel:id "unhitch"' in n3
    assert 'rel:label "Unhitch"' in n3
    assert "travelsHitched" in n3
    assert "travelsUnhitched" in n3
    assert "travelMessageHitched" in n3
    assert "travelMessageUnhitched" in n3
    assert "The field studio stays put" in n3
    assert "travelsHitched" in ttl
    assert "travelsUnhitched" in ttl
    assert 'id: "unhitch"' in vibe
    assert 'label: "Unhitch"' in vibe
    assert "travels_hitched: true" in vibe
    assert "travels_unhitched: true" in vibe
    assert "travels_unhitched: false" in vibe
    assert "travels_hitched" in macros
    assert "travels_unhitched" in macros
    assert "unhitch_defeater" in macros
    assert "sync_parked" in macros
    assert "group_travels" in macros


def test_unhitched_travel_skips_standing_groups_in_source():
    macros = MACROS.read_text(encoding="utf-8")
    loco = LOCO.read_text(encoding="utf-8")
    assert "fn group_travels" in macros
    assert "travels_unhitched" in macros
    assert "Unhitched groups accumulate WASD" in loco or "unhitched" in loco
    assert 'fn unhitched_steer_skips_standing_groups' in macros
    assert CAR not in macros
    assert CAR not in loco


def test_no_camp_rs_and_no_special_case_car_id():
    runtime = RUNTIME.read_text(encoding="utf-8")
    macros = MACROS.read_text(encoding="utf-8")
    loco = LOCO.read_text(encoding="utf-8")
    assert "mod camp;" not in runtime
    assert "camp.rs" not in runtime
    assert not (HOST_SRC / "camp.rs").exists()
    assert not (HOST_SRC / "runtime" / "camp.rs").exists()
    assert CAR not in macros
    assert CAR not in loco
    assert CAR not in runtime
    assert "mod camp;" not in loco
