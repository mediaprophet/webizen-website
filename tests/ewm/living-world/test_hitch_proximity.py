"""LW-09: hitch while far is refused by deontic/proximity, not a special-case id."""
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
N3 = ROOT / "data/ewm/camp-sequence.n3"
MACROS = ROOT / "native/world-host/src/runtime/macros.rs"
LOCO = ROOT / "native/world-host/src/runtime/locomotion.rs"
CAR = 'if id == "car"'


def parked_within_radius(gate, anchor, radius=4.0):
    dx = gate[0] - anchor[0]
    dz = gate[2] - anchor[2]
    return dx * dx + dz * dz <= radius * radius


def test_n3_hitch_radius_and_near_far_norms():
    n3 = N3.read_text(encoding="utf-8")
    assert "hitchRadiusM 4.0" in n3
    assert "forbid-hitch-when-far" in n3
    assert "permit-hitch-when-near" in n3
    assert 'rel:unless norm:permit-hitch-when-near' in n3
    assert 'norm:forbid-hitch-when-far rel:opcode "0x12"' in n3
    assert 'norm:permit-hitch-when-near rel:opcode "0x11"' in n3
    assert "requiresHitch true" in n3
    assert CAR not in n3


def test_player_refuses_hitch_pose_when_gate_parked_far():
    macros = MACROS.read_text(encoding="utf-8")
    loco = LOCO.read_text(encoding="utf-8")
    assert "DEFAULT_HITCH_RADIUS_M: f32 = 4.0" in macros
    assert "fn hitch_near" in macros
    assert "parked_within_radius" in macros
    assert "trigger_hitch_pose(trigger) && !self.hitch_near()" in macros
    assert "hitch_pose && !self.hitch_near()" in macros
    assert "fn hitch_when_far_is_refused_without_special_case_ids" in macros
    assert CAR not in macros
    assert "hitch_radius_m" in loco
    assert CAR not in loco
    assert parked_within_radius([0.0, 0.0, 0.0], [3.0, 0.0, 0.0])
    assert not parked_within_radius([0.0, 0.0, 0.0], [5.0, 0.0, 0.0])
    assert not parked_within_radius([20.0, 0.0, 8.0], [0.0, 0.0, 0.0])
