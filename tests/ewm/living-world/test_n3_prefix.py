"""LW-09: camp N3 remains the authored dump with @prefix."""
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
N3 = ROOT / "data/ewm/camp-sequence.n3"
TTL = ROOT / "data/ewm/camp-sequence.ttl"
CAMP = ROOT / "data/ewm/camp.q42"


def test_camp_sequence_n3_has_prefix_and_living_world_facts():
    raw = N3.read_bytes()
    assert raw
    n3 = raw.decode("utf-8")
    assert n3.lstrip().startswith("@prefix")
    assert "@prefix rel:" in n3
    assert "@prefix camp:" in n3
    assert "unhitch" in n3
    assert "travelsHitched" in n3
    assert "travelsUnhitched" in n3
    assert "hitchRadiusM" in n3
    assert "group:awning" in n3
    assert "aboutGroup" in n3
    assert "Timeline" not in n3
    assert "INFERRED_WORK" not in n3
    assert "placeId" not in n3
    assert CAMP.is_file()
    assert CAMP.read_bytes()[:3] == b"Q42"
    ttl = TTL.read_text(encoding="utf-8")
    assert "hitchRadiusM" in ttl
    assert "awning" in ttl
